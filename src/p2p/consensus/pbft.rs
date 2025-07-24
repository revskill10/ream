//! PBFT (Practical Byzantine Fault Tolerance) consensus algorithm
//!
//! Implements the PBFT consensus algorithm for Byzantine fault tolerance
//! in distributed systems.

use crate::p2p::{P2PResult, P2PError, ConsensusError, NodeId};
use super::common::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// PBFT consensus implementation
#[derive(Debug)]
pub struct PBFTConsensus {
    /// Local node ID
    local_node: NodeId,
    /// Current view number
    view: u64,
    /// Current sequence number
    sequence: u64,
    /// Current role
    role: ConsensusRole,
    /// Cluster membership
    cluster: ClusterMembership,
    /// Message log for PBFT phases
    message_log: HashMap<u64, PBFTMessageLog>,
    /// Configuration
    config: PBFTConfig,
    /// Statistics
    stats: Arc<RwLock<PBFTStats>>,
}

impl PBFTConsensus {
    /// Create a new PBFT consensus instance
    pub fn new(config: PBFTConfig) -> P2PResult<Self> {
        Ok(Self {
            local_node: NodeId::new(),
            view: 0,
            sequence: 0,
            role: ConsensusRole::Follower,
            cluster: ClusterMembership::new(vec![]),
            message_log: HashMap::new(),
            config,
            stats: Arc::new(RwLock::new(PBFTStats::default())),
        })
    }

    /// Start the PBFT consensus
    pub async fn start(&mut self) -> P2PResult<()> {
        // Initialize as backup
        self.role = ConsensusRole::Follower;
        self.view = 0;
        self.sequence = 0;
        Ok(())
    }

    /// Stop the PBFT consensus
    pub async fn stop(&mut self) -> P2PResult<()> {
        // Clean shutdown
        Ok(())
    }

    /// Join a cluster
    pub async fn join_cluster(&mut self) -> P2PResult<()> {
        // Add self to cluster
        self.cluster.add_member(self.local_node, true);
        Ok(())
    }

    /// Bootstrap as initial primary
    pub async fn bootstrap(&mut self) -> P2PResult<()> {
        self.role = ConsensusRole::Leader;
        self.view = 0;
        self.cluster = ClusterMembership::new(vec![self.local_node]);
        Ok(())
    }

    /// Propose a value for consensus
    pub async fn propose(&self, value: ConsensusValue) -> P2PResult<ConsensusResult> {
        if self.role != ConsensusRole::Leader {
            return Err(P2PError::Consensus(ConsensusError::NotLeader));
        }

        // Check if cluster can tolerate Byzantine faults
        let cluster_size = self.cluster.size();
        if cluster_size < 4 {
            return Err(P2PError::Consensus(ConsensusError::InsufficientReplicas));
        }

        // In a real implementation, we would:
        // 1. Send pre-prepare message to all backups
        // 2. Wait for prepare messages from 2f backups
        // 3. Send commit message
        // 4. Wait for commit messages from 2f backups
        // 5. Execute the operation

        // For now, return a simple result
        let result = ConsensusResult::new(
            value,
            self.view,
            self.sequence + 1,
            self.cluster.voting_members.clone(),
        );

        Ok(result)
    }

    /// Get current consensus state
    pub async fn get_state(&self) -> ConsensusState {
        ConsensusState {
            term: self.view,
            role: self.role.clone(),
            leader: if self.role == ConsensusRole::Leader {
                Some(self.local_node)
            } else {
                self.get_primary()
            },
            last_committed: self.sequence,
            cluster_size: self.cluster.size(),
            health: if self.can_make_progress() {
                ConsensusHealth::Healthy
            } else {
                ConsensusHealth::Degraded
            },
        }
    }

    /// Get PBFT statistics
    pub async fn get_stats(&self) -> PBFTStats {
        self.stats.read().await.clone()
    }

    /// Handle pre-prepare message
    pub async fn handle_pre_prepare(&mut self, message: PrePrepareMessage) -> P2PResult<()> {
        // Validate message
        if message.view != self.view {
            return Err(P2PError::Consensus(ConsensusError::InvalidTerm));
        }

        if message.sequence <= self.sequence {
            return Err(P2PError::Consensus(ConsensusError::InvalidSequence));
        }

        // Check if we're in the right state to accept this message
        if self.role == ConsensusRole::Leader {
            return Err(P2PError::Consensus(ConsensusError::ByzantineBehavior(
                "Received pre-prepare as primary".to_string()
            )));
        }

        // Store message and send prepare
        let log_entry = self.message_log.entry(message.sequence).or_insert_with(PBFTMessageLog::new);
        log_entry.pre_prepare = Some(message.clone());

        // Send prepare message to all replicas
        let prepare_msg = PrepareMessage {
            view: self.view,
            sequence: message.sequence,
            digest: message.digest.clone(),
            replica_id: self.local_node,
        };

        // In real implementation, would send to all replicas
        log_entry.prepares.push(prepare_msg);

        Ok(())
    }

    /// Handle prepare message
    pub async fn handle_prepare(&mut self, message: PrepareMessage) -> P2PResult<()> {
        if message.view != self.view {
            return Err(P2PError::Consensus(ConsensusError::InvalidTerm));
        }

        let log_entry = self.message_log.entry(message.sequence).or_insert_with(PBFTMessageLog::new);
        log_entry.prepares.push(message.clone());

        // Check if we have enough prepare messages (2f)
        let required_prepares = 2 * byzantine_threshold(self.cluster.size());
        if log_entry.prepares.len() >= required_prepares {
            // Send commit message
            let commit_msg = CommitMessage {
                view: self.view,
                sequence: message.sequence,
                digest: message.digest,
                replica_id: self.local_node,
            };

            log_entry.commits.push(commit_msg);
        }

        Ok(())
    }

    /// Handle commit message
    pub async fn handle_commit(&mut self, message: CommitMessage) -> P2PResult<()> {
        if message.view != self.view {
            return Err(P2PError::Consensus(ConsensusError::InvalidTerm));
        }

        let sequence = message.sequence;
        let log_entry = self.message_log.entry(message.sequence).or_insert_with(PBFTMessageLog::new);
        log_entry.commits.push(message);

        // Check if we have enough commit messages (2f + 1)
        let required_commits = 2 * byzantine_threshold(self.cluster.size()) + 1;
        if log_entry.commits.len() >= required_commits {
            // Execute the operation
            self.sequence = std::cmp::max(self.sequence, sequence);
            self.stats.write().await.decisions_made += 1;
        }

        Ok(())
    }

    /// Start view change
    pub async fn start_view_change(&mut self) -> P2PResult<()> {
        self.view += 1;
        self.role = ConsensusRole::Candidate;

        // In real implementation, would send view-change messages
        // and wait for new-view message from new primary

        self.stats.write().await.view_changes += 1;
        Ok(())
    }

    /// Get the primary node for current view
    fn get_primary(&self) -> Option<NodeId> {
        if self.cluster.voting_members.is_empty() {
            return None;
        }

        let primary_index = (self.view as usize) % self.cluster.voting_members.len();
        self.cluster.voting_members.get(primary_index).copied()
    }

    /// Check if the system can make progress
    fn can_make_progress(&self) -> bool {
        let cluster_size = self.cluster.size();
        let byzantine_threshold = byzantine_threshold(cluster_size);
        
        // Need at least 3f + 1 nodes for Byzantine fault tolerance
        cluster_size >= 3 * byzantine_threshold + 1
    }
}

/// PBFT configuration
#[derive(Debug, Clone)]
pub struct PBFTConfig {
    /// Timeout for consensus rounds
    pub consensus_timeout: std::time::Duration,
    /// View change timeout
    pub view_change_timeout: std::time::Duration,
    /// Maximum message size
    pub max_message_size: usize,
}

impl Default for PBFTConfig {
    fn default() -> Self {
        Self {
            consensus_timeout: std::time::Duration::from_secs(10),
            view_change_timeout: std::time::Duration::from_secs(20),
            max_message_size: 1024 * 1024, // 1MB
        }
    }
}

/// PBFT statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PBFTStats {
    /// Number of decisions made
    pub decisions_made: u64,
    /// Number of view changes
    pub view_changes: u64,
    /// Number of pre-prepare messages sent
    pub pre_prepares_sent: u64,
    /// Number of prepare messages sent
    pub prepares_sent: u64,
    /// Number of commit messages sent
    pub commits_sent: u64,
}

/// Message log for PBFT consensus phases
#[derive(Debug, Default)]
pub struct PBFTMessageLog {
    /// Pre-prepare message
    pub pre_prepare: Option<PrePrepareMessage>,
    /// Prepare messages
    pub prepares: Vec<PrepareMessage>,
    /// Commit messages
    pub commits: Vec<CommitMessage>,
}

impl PBFTMessageLog {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Pre-prepare message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePrepareMessage {
    /// View number
    pub view: u64,
    /// Sequence number
    pub sequence: u64,
    /// Message digest
    pub digest: String,
    /// Request being proposed
    pub request: ConsensusValue,
    /// Primary replica ID
    pub primary_id: NodeId,
}

/// Prepare message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareMessage {
    /// View number
    pub view: u64,
    /// Sequence number
    pub sequence: u64,
    /// Message digest
    pub digest: String,
    /// Backup replica ID
    pub replica_id: NodeId,
}

/// Commit message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMessage {
    /// View number
    pub view: u64,
    /// Sequence number
    pub sequence: u64,
    /// Message digest
    pub digest: String,
    /// Replica ID
    pub replica_id: NodeId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pbft_creation() {
        let config = PBFTConfig::default();
        let result = PBFTConsensus::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pbft_lifecycle() {
        let config = PBFTConfig::default();
        let mut pbft = PBFTConsensus::new(config).unwrap();
        
        assert!(pbft.start().await.is_ok());
        assert!(pbft.bootstrap().await.is_ok());
        
        let state = pbft.get_state().await;
        assert_eq!(state.role, ConsensusRole::Leader);
        assert_eq!(state.term, 0);
        
        assert!(pbft.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_pbft_byzantine_threshold() {
        assert_eq!(byzantine_threshold(1), 0);
        assert_eq!(byzantine_threshold(4), 1);
        assert_eq!(byzantine_threshold(7), 2);
        assert_eq!(byzantine_threshold(10), 3);
    }
}
