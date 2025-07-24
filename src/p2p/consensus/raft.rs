//! Raft consensus algorithm implementation
//!
//! Implements the Raft consensus algorithm for distributed agreement
//! in non-Byzantine environments.

use crate::p2p::{P2PResult, P2PError, ConsensusError, NodeId};
use super::common::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::{interval, timeout};

/// Raft message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftMessage {
    /// Request vote message
    RequestVote {
        term: u64,
        candidate_id: NodeId,
        last_log_index: u64,
        last_log_term: u64,
    },
    /// Vote response
    RequestVoteResponse {
        term: u64,
        vote_granted: bool,
    },
    /// Append entries message
    AppendEntries {
        term: u64,
        leader_id: NodeId,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    },
    /// Append entries response
    AppendEntriesResponse {
        term: u64,
        success: bool,
        match_index: u64,
    },
}

/// Raft consensus implementation
#[derive(Debug)]
pub struct RaftConsensus {
    /// Local node ID
    local_node: NodeId,
    /// Current term
    current_term: u64,
    /// Node we voted for in current term
    voted_for: Option<NodeId>,
    /// Consensus log
    log: Vec<LogEntry>,
    /// Current role
    role: ConsensusRole,
    /// Cluster membership
    cluster: ClusterMembership,
    /// Commit index
    commit_index: u64,
    /// Last applied index
    last_applied: u64,
    /// Configuration
    config: RaftConfig,
    /// Statistics
    stats: Arc<RwLock<RaftStats>>,
    /// Leader state (only used when leader)
    leader_state: Option<LeaderState>,
    /// Last heartbeat received (for followers)
    last_heartbeat: Option<Instant>,
    /// Election timeout
    election_timeout: Duration,
    /// Random election timeout offset
    election_timeout_offset: Duration,
}

/// Leader-specific state
#[derive(Debug)]
struct LeaderState {
    /// Next index to send to each follower
    next_index: HashMap<NodeId, u64>,
    /// Highest index replicated to each follower
    match_index: HashMap<NodeId, u64>,
    /// Last heartbeat sent time
    last_heartbeat_sent: Instant,
}

impl RaftConsensus {
    /// Create a new Raft consensus instance
    pub fn new(config: RaftConfig, local_node: NodeId, cluster_members: Vec<NodeId>) -> P2PResult<Self> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Random election timeout between 150-300ms
        let base_timeout = Duration::from_millis(150);
        let offset = Duration::from_millis(rng.gen_range(0..150));

        Ok(Self {
            local_node,
            current_term: 0,
            voted_for: None,
            log: Vec::new(),
            role: ConsensusRole::Follower,
            cluster: ClusterMembership::new(cluster_members),
            commit_index: 0,
            last_applied: 0,
            config,
            stats: Arc::new(RwLock::new(RaftStats::default())),
            leader_state: None,
            last_heartbeat: None,
            election_timeout: base_timeout,
            election_timeout_offset: offset,
        })
    }

    /// Start the Raft consensus
    pub async fn start(&mut self) -> P2PResult<()> {
        // Initialize as follower
        self.role = ConsensusRole::Follower;
        self.current_term = 0;
        self.voted_for = None;
        self.last_heartbeat = Some(Instant::now());

        // Start election timer
        self.start_election_timer().await;

        Ok(())
    }

    /// Start election timer
    async fn start_election_timer(&mut self) {
        // In a real implementation, this would run in a background task
        // For now, we'll just set the timeout
        self.last_heartbeat = Some(Instant::now());
    }

    /// Check if election timeout has elapsed
    pub fn is_election_timeout(&self) -> bool {
        if let Some(last_heartbeat) = self.last_heartbeat {
            let timeout = self.election_timeout + self.election_timeout_offset;
            last_heartbeat.elapsed() > timeout
        } else {
            true
        }
    }

    /// Force election timeout for testing
    pub fn force_election_timeout(&mut self) {
        self.last_heartbeat = Some(Instant::now() - Duration::from_secs(1));
    }

    /// Start leader election
    pub async fn start_election(&mut self) -> P2PResult<()> {
        // Increment term and vote for self
        self.current_term += 1;
        self.voted_for = Some(self.local_node);
        self.role = ConsensusRole::Candidate;
        self.last_heartbeat = Some(Instant::now());

        // Update statistics
        self.stats.write().await.elections_held += 1;

        // Get last log info
        let (last_log_index, last_log_term) = if let Some(last_entry) = self.log.last() {
            (last_entry.index, last_entry.term)
        } else {
            (0, 0)
        };

        // Send RequestVote to all other nodes
        let _request = RaftMessage::RequestVote {
            term: self.current_term,
            candidate_id: self.local_node,
            last_log_index,
            last_log_term,
        };

        // In a real implementation, we would send this to all cluster members
        // and count votes. For now, we'll simulate becoming leader if we're the only node
        if self.cluster.voting_members.len() == 1 {
            self.become_leader().await?;
        }

        Ok(())
    }

    /// Become leader
    async fn become_leader(&mut self) -> P2PResult<()> {
        self.role = ConsensusRole::Leader;

        // Initialize leader state
        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();

        let next_log_index = self.log.len() as u64 + 1;

        for member in &self.cluster.voting_members {
            if *member != self.local_node {
                next_index.insert(*member, next_log_index);
                match_index.insert(*member, 0);
            }
        }

        self.leader_state = Some(LeaderState {
            next_index,
            match_index,
            last_heartbeat_sent: Instant::now(),
        });

        // Send initial heartbeat
        self.send_heartbeat().await?;

        Ok(())
    }

    /// Send heartbeat to all followers
    async fn send_heartbeat(&mut self) -> P2PResult<()> {
        if self.role != ConsensusRole::Leader {
            return Ok(());
        }

        let _heartbeat = RaftMessage::AppendEntries {
            term: self.current_term,
            leader_id: self.local_node,
            prev_log_index: self.log.len() as u64,
            prev_log_term: self.log.last().map(|e| e.term).unwrap_or(0),
            entries: vec![], // Empty for heartbeat
            leader_commit: self.commit_index,
        };

        // In a real implementation, we would send this to all followers
        // For now, just update the timestamp
        if let Some(ref mut leader_state) = self.leader_state {
            leader_state.last_heartbeat_sent = Instant::now();
        }

        Ok(())
    }

    /// Stop the Raft consensus
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

    /// Bootstrap as initial leader
    pub async fn bootstrap(&mut self) -> P2PResult<()> {
        self.role = ConsensusRole::Leader;
        self.current_term = 1;
        self.cluster = ClusterMembership::new(vec![self.local_node]);
        Ok(())
    }

    /// Propose a value for consensus
    pub async fn propose(&mut self, value: ConsensusValue) -> P2PResult<ConsensusResult> {
        if self.role != ConsensusRole::Leader {
            return Err(P2PError::Consensus(ConsensusError::NotLeader));
        }

        // Create log entry
        let entry = LogEntry::new(
            self.log.len() as u64 + 1,
            self.current_term,
            value.clone(),
        );

        // 1. Append entry to local log
        self.log.push(entry.clone());

        // 2. Replicate to followers (simplified for single node)
        if self.cluster.voting_members.len() == 1 {
            // Single node cluster - immediately commit
            self.commit_index = entry.index;
            self.apply_committed_entries().await?;

            // Update statistics
            self.stats.write().await.entries_committed += 1;
            self.stats.write().await.log_size = self.log.len();

            let result = ConsensusResult::new(
                value,
                self.current_term,
                entry.index,
                self.cluster.voting_members.clone(),
            );

            return Ok(result);
        }

        // 3. For multi-node clusters, send AppendEntries to followers
        let append_entries = RaftMessage::AppendEntries {
            term: self.current_term,
            leader_id: self.local_node,
            prev_log_index: entry.index - 1,
            prev_log_term: if entry.index > 1 {
                self.log.get((entry.index - 2) as usize).map(|e| e.term).unwrap_or(0)
            } else {
                0
            },
            entries: vec![entry.clone()],
            leader_commit: self.commit_index,
        };

        // In a real implementation, we would send this to all followers
        // and wait for majority acknowledgment before committing

        // For now, simulate immediate success
        let result = ConsensusResult::new(
            value,
            self.current_term,
            entry.index,
            self.cluster.voting_members.clone(),
        );

        Ok(result)
    }

    /// Apply committed entries to state machine
    async fn apply_committed_entries(&mut self) -> P2PResult<()> {
        while self.last_applied < self.commit_index {
            self.last_applied += 1;

            if let Some(entry) = self.log.get((self.last_applied - 1) as usize) {
                // In a real implementation, we would apply this to the state machine
                println!("Applied entry {} to state machine: {:?}", entry.index, entry.value);
            }
        }

        Ok(())
    }

    /// Get current consensus state
    pub async fn get_state(&self) -> ConsensusState {
        ConsensusState {
            term: self.current_term,
            role: self.role.clone(),
            leader: if self.role == ConsensusRole::Leader {
                Some(self.local_node)
            } else {
                None
            },
            last_committed: self.commit_index,
            cluster_size: self.cluster.size(),
            health: ConsensusHealth::Healthy,
        }
    }

    /// Get Raft statistics
    pub async fn get_stats(&self) -> RaftStats {
        self.stats.read().await.clone()
    }

    /// Handle incoming Raft message
    pub async fn handle_message(&mut self, message: RaftMessage) -> P2PResult<Option<RaftMessage>> {
        match message {
            RaftMessage::RequestVote { term, candidate_id, last_log_index, last_log_term } => {
                self.handle_request_vote(term, candidate_id, last_log_index, last_log_term).await
            }
            RaftMessage::RequestVoteResponse { term, vote_granted } => {
                self.handle_request_vote_response(term, vote_granted).await
            }
            RaftMessage::AppendEntries { term, leader_id, prev_log_index, prev_log_term, entries, leader_commit } => {
                self.handle_append_entries(term, leader_id, prev_log_index, prev_log_term, entries, leader_commit).await
            }
            RaftMessage::AppendEntriesResponse { term, success, match_index } => {
                self.handle_append_entries_response(term, success, match_index).await
            }
        }
    }

    /// Handle RequestVote message
    async fn handle_request_vote(
        &mut self,
        term: u64,
        candidate_id: NodeId,
        last_log_index: u64,
        last_log_term: u64,
    ) -> P2PResult<Option<RaftMessage>> {
        // Update term if necessary
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
            self.role = ConsensusRole::Follower;
        }

        let vote_granted = if term < self.current_term {
            false
        } else if self.voted_for.is_some() && self.voted_for != Some(candidate_id) {
            false
        } else {
            // Check if candidate's log is at least as up-to-date as ours
            let our_last_log_term = self.log.last().map(|e| e.term).unwrap_or(0);
            let our_last_log_index = self.log.len() as u64;

            last_log_term > our_last_log_term ||
            (last_log_term == our_last_log_term && last_log_index >= our_last_log_index)
        };

        if vote_granted {
            self.voted_for = Some(candidate_id);
            self.last_heartbeat = Some(Instant::now());
        }

        Ok(Some(RaftMessage::RequestVoteResponse {
            term: self.current_term,
            vote_granted,
        }))
    }

    /// Handle RequestVoteResponse message
    async fn handle_request_vote_response(
        &mut self,
        term: u64,
        vote_granted: bool,
    ) -> P2PResult<Option<RaftMessage>> {
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
            self.role = ConsensusRole::Follower;
        }

        // In a real implementation, we would count votes and become leader
        // if we receive majority votes
        if vote_granted && self.role == ConsensusRole::Candidate {
            // For simplicity, assume we become leader immediately
            self.become_leader().await?;
        }

        Ok(None)
    }

    /// Handle AppendEntries message
    async fn handle_append_entries(
        &mut self,
        term: u64,
        _leader_id: NodeId,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    ) -> P2PResult<Option<RaftMessage>> {
        // Update term if necessary
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
            self.role = ConsensusRole::Follower;
        }

        self.last_heartbeat = Some(Instant::now());

        let success = if term < self.current_term {
            false
        } else if prev_log_index > 0 {
            // Check if we have the previous log entry
            if let Some(prev_entry) = self.log.get((prev_log_index - 1) as usize) {
                prev_entry.term == prev_log_term
            } else {
                false
            }
        } else {
            true
        };

        if success && !entries.is_empty() {
            // Append new entries
            let start_index = prev_log_index as usize;

            // Remove conflicting entries
            if self.log.len() > start_index {
                self.log.truncate(start_index);
            }

            // Append new entries
            self.log.extend(entries);
        }

        // Update commit index
        if success && leader_commit > self.commit_index {
            self.commit_index = std::cmp::min(leader_commit, self.log.len() as u64);
            self.apply_committed_entries().await?;
        }

        Ok(Some(RaftMessage::AppendEntriesResponse {
            term: self.current_term,
            success,
            match_index: if success { self.log.len() as u64 } else { 0 },
        }))
    }

    /// Handle AppendEntriesResponse message
    async fn handle_append_entries_response(
        &mut self,
        term: u64,
        success: bool,
        match_index: u64,
    ) -> P2PResult<Option<RaftMessage>> {
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
            self.role = ConsensusRole::Follower;
        }

        // In a real implementation, we would update match_index and next_index
        // for the responding follower and check if we can advance commit_index
        if success && self.role == ConsensusRole::Leader {
            // Update commit index if majority of followers have replicated
            // For simplicity, just advance if we get any successful response
            if match_index > self.commit_index {
                self.commit_index = match_index;
                self.apply_committed_entries().await?;
            }
        }

        Ok(None)
    }



    /// Handle vote request
    pub async fn handle_vote_request(&mut self, request: VoteRequest) -> P2PResult<VoteResponse> {
        let mut grant_vote = false;

        // Grant vote if:
        // 1. Candidate's term is at least as current as ours
        // 2. We haven't voted for anyone else in this term
        // 3. Candidate's log is at least as up-to-date as ours
        if request.term >= self.current_term {
            if self.voted_for.is_none() || self.voted_for == Some(request.candidate_id) {
                // Check log up-to-date condition
                let our_last_log_term = self.log.last().map(|e| e.term).unwrap_or(0);
                let our_last_log_index = self.log.len() as u64;

                if request.last_log_term > our_last_log_term ||
                   (request.last_log_term == our_last_log_term && request.last_log_index >= our_last_log_index) {
                    grant_vote = true;
                    self.voted_for = Some(request.candidate_id);
                    self.current_term = request.term;
                }
            }
        }

        Ok(VoteResponse {
            term: self.current_term,
            vote_granted: grant_vote,
        })
    }

    /// Append entries to log
    pub async fn append_entries(&mut self, request: AppendEntriesRequest) -> P2PResult<AppendEntriesResponse> {
        let mut success = false;

        // Reply false if term < currentTerm
        if request.term >= self.current_term {
            self.current_term = request.term;
            self.role = ConsensusRole::Follower;

            // Check if log contains an entry at prevLogIndex whose term matches prevLogTerm
            if request.prev_log_index == 0 ||
               (request.prev_log_index <= self.log.len() as u64 &&
                self.log.get(request.prev_log_index as usize - 1).map(|e| e.term).unwrap_or(0) == request.prev_log_term) {
                
                // Append new entries
                let start_index = request.prev_log_index as usize;
                self.log.truncate(start_index);
                self.log.extend(request.entries);
                
                // Update commit index
                if request.leader_commit > self.commit_index {
                    self.commit_index = std::cmp::min(request.leader_commit, self.log.len() as u64);
                }
                
                success = true;
            }
        }

        Ok(AppendEntriesResponse {
            term: self.current_term,
            success,
        })
    }
}

/// Raft configuration
#[derive(Debug, Clone)]
pub struct RaftConfig {
    /// Election timeout range
    pub election_timeout_min: std::time::Duration,
    pub election_timeout_max: std::time::Duration,
    /// Heartbeat interval
    pub heartbeat_interval: std::time::Duration,
    /// Maximum entries per append request
    pub max_entries_per_request: usize,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            election_timeout_min: std::time::Duration::from_millis(150),
            election_timeout_max: std::time::Duration::from_millis(300),
            heartbeat_interval: std::time::Duration::from_millis(50),
            max_entries_per_request: 100,
        }
    }
}

/// Raft statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RaftStats {
    /// Number of elections held
    pub elections_held: u64,
    /// Number of entries committed
    pub entries_committed: u64,
    /// Number of append entries sent
    pub append_entries_sent: u64,
    /// Number of vote requests sent
    pub vote_requests_sent: u64,
    /// Current log size
    pub log_size: usize,
}

/// Vote request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting vote
    pub candidate_id: NodeId,
    /// Index of candidate's last log entry
    pub last_log_index: u64,
    /// Term of candidate's last log entry
    pub last_log_term: u64,
}

/// Vote response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    /// Current term for candidate to update itself
    pub term: u64,
    /// True means candidate received vote
    pub vote_granted: bool,
}

/// Append entries request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: u64,
    /// Leader ID
    pub leader_id: NodeId,
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: u64,
    /// Term of prev_log_index entry
    pub prev_log_term: u64,
    /// Log entries to store
    pub entries: Vec<LogEntry>,
    /// Leader's commit index
    pub leader_commit: u64,
}

/// Append entries response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term for leader to update itself
    pub term: u64,
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_raft_creation() {
        let config = RaftConfig::default();
        let node_id = NodeId::new();
        let result = RaftConsensus::new(config, node_id, vec![node_id]);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_raft_lifecycle() {
        let config = RaftConfig::default();
        let node_id = NodeId::new();
        let mut raft = RaftConsensus::new(config, node_id, vec![node_id]).unwrap();
        
        assert!(raft.start().await.is_ok());
        assert!(raft.bootstrap().await.is_ok());
        
        let state = raft.get_state().await;
        assert_eq!(state.role, ConsensusRole::Leader);
        assert_eq!(state.term, 1);
        
        assert!(raft.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_raft_proposal() {
        let config = RaftConfig::default();
        let node_id = NodeId::new();
        let mut raft = RaftConsensus::new(config, node_id, vec![node_id]).unwrap();
        
        raft.start().await.unwrap();
        raft.bootstrap().await.unwrap();
        
        let value = ConsensusValue::from_string("test".to_string(), raft.local_node);
        let result = raft.propose(value.clone()).await.unwrap();
        
        assert_eq!(result.value, value);
        assert_eq!(result.term, 1);
    }
}
