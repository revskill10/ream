//! REAM P2P Distributed System
//!
//! A mathematically-grounded peer-to-peer distributed system that maintains
//! category-theoretic properties while enabling transparent remote actor
//! communication, fault-tolerant consensus, and dynamic cluster formation.

pub mod types;
pub mod error;
pub mod network;
pub mod discovery;
pub mod consensus;
pub mod actor;
pub mod cluster;
pub mod node;

// Re-export core types and functions
pub use types::*;
pub use error::*;
pub use node::ReamNode;

// Re-export network components
pub use network::{NetworkLayer, SessionType, NetworkProtocol};

// Re-export discovery components
pub use discovery::{ReamDHT, GossipProtocol, NodeDiscovery};

// Re-export consensus components
pub use consensus::{ConsensusEngine, PBFTConsensus, RaftConsensus};

// Re-export actor components
pub use actor::{DistributedActor, DistributedActorRegistry, MigrationManager, DistributedActorRef, MigrationResult};

// Re-export cluster components
pub use cluster::{ClusterManager, FailureDetector, ClusterMetrics};

use std::sync::Arc;
use tokio::sync::RwLock;

/// P2P system configuration
#[derive(Debug, Clone)]
pub struct P2PConfig {
    /// Node configuration
    pub node_config: NodeConfig,
    /// Network configuration
    pub network_config: NetworkConfig,
    /// Consensus configuration
    pub consensus_config: ConsensusConfig,
    /// Cluster configuration
    pub cluster_config: ClusterConfig,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            node_config: NodeConfig::default(),
            network_config: NetworkConfig::default(),
            consensus_config: ConsensusConfig::default(),
            cluster_config: ClusterConfig::default(),
        }
    }
}

/// Initialize a P2P system
pub async fn initialize_p2p_system(config: P2PConfig) -> P2PResult<Arc<RwLock<ReamNode>>> {
    let mut node = ReamNode::new(config.node_config).await?;
    node.start().await?;
    Ok(Arc::new(RwLock::new(node)))
}

/// Join an existing P2P cluster
pub async fn join_cluster(
    node: Arc<RwLock<ReamNode>>,
    bootstrap_nodes: Vec<NodeInfo>,
) -> P2PResult<()> {
    let mut node = node.write().await;
    node.join_cluster(bootstrap_nodes).await
}

/// Create a new P2P cluster
pub async fn create_cluster(node: Arc<RwLock<ReamNode>>) -> P2PResult<()> {
    let mut node = node.write().await;
    node.create_cluster().await
}

/// Get cluster information
pub async fn get_cluster_info(node: Arc<RwLock<ReamNode>>) -> P2PResult<ClusterInfo> {
    let node = node.read().await;
    node.get_cluster_info().await
}

/// Add a member to the cluster
pub async fn add_cluster_member(node: Arc<RwLock<ReamNode>>, member_info: NodeInfo) -> P2PResult<()> {
    let node = node.read().await;
    node.add_cluster_member(member_info).await
}

/// Spawn a distributed actor
pub async fn spawn_distributed_actor<A>(
    node: Arc<RwLock<ReamNode>>,
    actor: A,
    placement_constraints: Option<PlacementConstraints>,
) -> P2PResult<DistributedActorRef>
where
    A: crate::runtime::ReamActor + Send + Sync + 'static,
{
    let mut node = node.write().await;
    node.spawn_distributed_actor(actor, placement_constraints).await
}

/// Migrate an actor to a different node
pub async fn migrate_actor(
    node: Arc<RwLock<ReamNode>>,
    actor_id: ActorId,
    target_node: NodeId,
) -> P2PResult<MigrationResult> {
    let mut node = node.write().await;
    node.migrate_actor(actor_id, target_node).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_p2p_system_initialization() {
        let config = P2PConfig::default();
        let result = initialize_p2p_system(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_creation() {
        let config = P2PConfig::default();
        let node = initialize_p2p_system(config).await.unwrap();
        
        let result = create_cluster(node.clone()).await;
        assert!(result.is_ok());
        
        let cluster_info = get_cluster_info(node).await.unwrap();
        assert_eq!(cluster_info.member_count, 1);
    }

    #[tokio::test]
    async fn test_cluster_joining() {
        // Create bootstrap node
        let config1 = P2PConfig::default();
        let bootstrap_node = initialize_p2p_system(config1).await.unwrap();
        create_cluster(bootstrap_node.clone()).await.unwrap();
        
        // Get bootstrap node info
        let bootstrap_info = {
            let node = bootstrap_node.read().await;
            node.get_node_info().await.unwrap()
        };
        
        // Create joining node
        let config2 = P2PConfig::default();
        let joining_node = initialize_p2p_system(config2).await.unwrap();
        
        // Join cluster
        let result = join_cluster(joining_node.clone(), vec![bootstrap_info.clone()]).await;
        assert!(result.is_ok());

        // Simulate the bootstrap node being notified of the new member
        let joining_node_info = {
            let node = joining_node.read().await;
            node.get_node_info().await.unwrap()
        };
        add_cluster_member(bootstrap_node.clone(), joining_node_info).await.unwrap();

        // Verify cluster has 2 members
        let cluster_info = timeout(
            Duration::from_secs(5),
            get_cluster_info(bootstrap_node)
        ).await.unwrap().unwrap();

        assert_eq!(cluster_info.member_count, 2);
    }
}
