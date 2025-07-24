//! Consensus algorithms for distributed agreement
//!
//! Implements PBFT (Practical Byzantine Fault Tolerance) and Raft
//! consensus algorithms for achieving distributed agreement in the
//! presence of failures and network partitions.

pub mod pbft;
pub mod raft;
pub mod common;

pub use pbft::*;
pub use raft::*;
pub use common::*;

use crate::p2p::{P2PResult, P2PError, ConsensusError, NodeId, ConsensusAlgorithm};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Consensus engine that can run different consensus algorithms
#[derive(Debug)]
pub struct ConsensusEngine {
    /// Current consensus algorithm
    algorithm: ConsensusAlgorithm,
    /// PBFT consensus instance
    pbft: Option<Arc<RwLock<PBFTConsensus>>>,
    /// Raft consensus instance
    raft: Option<Arc<RwLock<RaftConsensus>>>,
    /// Consensus configuration
    config: ConsensusConfig,
    /// Consensus statistics
    stats: Arc<RwLock<ConsensusStats>>,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new(config: ConsensusConfig) -> P2PResult<Self> {
        let mut engine = Self {
            algorithm: config.algorithm.clone(),
            pbft: None,
            raft: None,
            config: config.clone(),
            stats: Arc::new(RwLock::new(ConsensusStats::default())),
        };

        // Initialize the appropriate consensus algorithm
        match &config.algorithm {
            ConsensusAlgorithm::PBFT => {
                let pbft = PBFTConsensus::new(config.pbft_config.clone())?;
                engine.pbft = Some(Arc::new(RwLock::new(pbft)));
            }
            ConsensusAlgorithm::Raft => {
                let node_id = NodeId::new();
                let cluster_members = vec![node_id]; // Single node cluster for now
                let raft = RaftConsensus::new(config.raft_config.clone(), node_id, cluster_members)?;
                engine.raft = Some(Arc::new(RwLock::new(raft)));
            }
            ConsensusAlgorithm::Custom(_) => {
                return Err(P2PError::Consensus(ConsensusError::ElectionFailed(
                    "Custom consensus algorithms not yet supported".to_string()
                )));
            }
        }

        Ok(engine)
    }

    /// Start the consensus engine
    pub async fn start(&self) -> P2PResult<()> {
        match &self.algorithm {
            ConsensusAlgorithm::PBFT => {
                if let Some(pbft) = &self.pbft {
                    let mut pbft = pbft.write().await;
                    pbft.start().await?;
                }
            }
            ConsensusAlgorithm::Raft => {
                if let Some(raft) = &self.raft {
                    let mut raft = raft.write().await;
                    raft.start().await?;
                }
            }
            ConsensusAlgorithm::Custom(_) => {
                return Err(P2PError::Consensus(ConsensusError::ElectionFailed(
                    "Custom consensus algorithms not supported".to_string()
                )));
            }
        }

        Ok(())
    }

    /// Stop the consensus engine
    pub async fn stop(&self) -> P2PResult<()> {
        match &self.algorithm {
            ConsensusAlgorithm::PBFT => {
                if let Some(pbft) = &self.pbft {
                    let mut pbft = pbft.write().await;
                    pbft.stop().await?;
                }
            }
            ConsensusAlgorithm::Raft => {
                if let Some(raft) = &self.raft {
                    let mut raft = raft.write().await;
                    raft.stop().await?;
                }
            }
            ConsensusAlgorithm::Custom(_) => {}
        }

        Ok(())
    }

    /// Join a cluster
    pub async fn join_cluster(&self) -> P2PResult<()> {
        match &self.algorithm {
            ConsensusAlgorithm::PBFT => {
                if let Some(pbft) = &self.pbft {
                    let mut pbft = pbft.write().await;
                    pbft.join_cluster().await?;
                }
            }
            ConsensusAlgorithm::Raft => {
                if let Some(raft) = &self.raft {
                    let mut raft = raft.write().await;
                    raft.join_cluster().await?;
                }
            }
            ConsensusAlgorithm::Custom(_) => {
                return Err(P2PError::Consensus(ConsensusError::ElectionFailed(
                    "Custom consensus algorithms not supported".to_string()
                )));
            }
        }

        Ok(())
    }

    /// Bootstrap consensus as the initial leader
    pub async fn bootstrap_consensus(&self) -> P2PResult<()> {
        match &self.algorithm {
            ConsensusAlgorithm::PBFT => {
                if let Some(pbft) = &self.pbft {
                    let mut pbft = pbft.write().await;
                    pbft.bootstrap().await?;
                }
            }
            ConsensusAlgorithm::Raft => {
                if let Some(raft) = &self.raft {
                    let mut raft = raft.write().await;
                    raft.bootstrap().await?;
                }
            }
            ConsensusAlgorithm::Custom(_) => {
                return Err(P2PError::Consensus(ConsensusError::ElectionFailed(
                    "Custom consensus algorithms not supported".to_string()
                )));
            }
        }

        Ok(())
    }

    /// Propose a value for consensus
    pub async fn propose(&self, value: ConsensusValue) -> P2PResult<ConsensusResult> {
        let result = match &self.algorithm {
            ConsensusAlgorithm::PBFT => {
                if let Some(pbft) = &self.pbft {
                    let pbft = pbft.read().await;
                    pbft.propose(value).await?
                } else {
                    return Err(P2PError::Consensus(ConsensusError::ProposalFailed(
                        "PBFT not initialized".to_string()
                    )));
                }
            }
            ConsensusAlgorithm::Raft => {
                if let Some(raft) = &self.raft {
                    let mut raft = raft.write().await;
                    raft.propose(value).await?
                } else {
                    return Err(P2PError::Consensus(ConsensusError::ProposalFailed(
                        "Raft not initialized".to_string()
                    )));
                }
            }
            ConsensusAlgorithm::Custom(_) => {
                return Err(P2PError::Consensus(ConsensusError::ProposalFailed(
                    "Custom consensus algorithms not supported".to_string()
                )));
            }
        };

        // Update statistics
        self.stats.write().await.proposals_submitted += 1;
        Ok(result)
    }

    /// Get current consensus state
    pub async fn get_state(&self) -> P2PResult<ConsensusState> {
        match &self.algorithm {
            ConsensusAlgorithm::PBFT => {
                if let Some(pbft) = &self.pbft {
                    let pbft = pbft.read().await;
                    Ok(pbft.get_state().await)
                } else {
                    Err(P2PError::Consensus(ConsensusError::ElectionFailed(
                        "PBFT not initialized".to_string()
                    )))
                }
            }
            ConsensusAlgorithm::Raft => {
                if let Some(raft) = &self.raft {
                    let raft = raft.read().await;
                    Ok(raft.get_state().await)
                } else {
                    Err(P2PError::Consensus(ConsensusError::ElectionFailed(
                        "Raft not initialized".to_string()
                    )))
                }
            }
            ConsensusAlgorithm::Custom(_) => {
                Err(P2PError::Consensus(ConsensusError::ElectionFailed(
                    "Custom consensus algorithms not supported".to_string()
                )))
            }
        }
    }

    /// Get consensus statistics
    pub async fn get_stats(&self) -> ConsensusStats {
        let mut stats = self.stats.read().await.clone();

        // Add algorithm-specific stats
        match &self.algorithm {
            ConsensusAlgorithm::PBFT => {
                if let Some(pbft) = &self.pbft {
                    let pbft_stats = pbft.read().await.get_stats().await;
                    stats.decisions_made = pbft_stats.decisions_made;
                    stats.view_changes = pbft_stats.view_changes;
                }
            }
            ConsensusAlgorithm::Raft => {
                if let Some(raft) = &self.raft {
                    let raft_stats = raft.read().await.get_stats().await;
                    stats.decisions_made = raft_stats.entries_committed;
                    stats.leader_elections = raft_stats.elections_held;
                }
            }
            ConsensusAlgorithm::Custom(_) => {}
        }

        stats
    }
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Consensus algorithm to use
    pub algorithm: ConsensusAlgorithm,
    /// PBFT-specific configuration
    pub pbft_config: PBFTConfig,
    /// Raft-specific configuration
    pub raft_config: RaftConfig,
    /// General consensus timeouts
    pub proposal_timeout: std::time::Duration,
    /// Maximum number of concurrent proposals
    pub max_concurrent_proposals: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            algorithm: ConsensusAlgorithm::Raft,
            pbft_config: PBFTConfig::default(),
            raft_config: RaftConfig::default(),
            proposal_timeout: std::time::Duration::from_secs(30),
            max_concurrent_proposals: 10,
        }
    }
}

/// Consensus statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    /// Number of proposals submitted
    pub proposals_submitted: u64,
    /// Number of decisions made
    pub decisions_made: u64,
    /// Number of consensus failures
    pub consensus_failures: u64,
    /// Number of view changes (PBFT)
    pub view_changes: u64,
    /// Number of leader elections (Raft)
    pub leader_elections: u64,
    /// Average consensus latency in milliseconds
    pub average_latency_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_engine_creation() {
        let config = ConsensusConfig::default();
        let result = ConsensusEngine::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_raft_consensus_engine() {
        let mut config = ConsensusConfig::default();
        config.algorithm = ConsensusAlgorithm::Raft;
        
        let engine = ConsensusEngine::new(config).unwrap();
        assert!(engine.start().await.is_ok());
        assert!(engine.bootstrap_consensus().await.is_ok());
        
        let stats = engine.get_stats().await;
        assert_eq!(stats.proposals_submitted, 0);
        
        assert!(engine.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_pbft_consensus_engine() {
        let mut config = ConsensusConfig::default();
        config.algorithm = ConsensusAlgorithm::PBFT;
        
        let engine = ConsensusEngine::new(config).unwrap();
        assert!(engine.start().await.is_ok());
        assert!(engine.bootstrap_consensus().await.is_ok());
        
        let stats = engine.get_stats().await;
        assert_eq!(stats.proposals_submitted, 0);
        
        assert!(engine.stop().await.is_ok());
    }
}
