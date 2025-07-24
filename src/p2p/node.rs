//! Main P2P node implementation
//!
//! Integrates all P2P components into a single ReamNode that can
//! participate in the distributed system.

use crate::p2p::{
    P2PResult, NodeId, NodeInfo, NodeConfig, ClusterInfo, ActorId, PlacementConstraints,
    NetworkLayer, NodeDiscovery, ConsensusEngine, DistributedActorRegistry, ClusterManager,
    MigrationManager,
};
use crate::p2p::actor::{DistributedActorRef, MigrationResult};
use crate::p2p::discovery::DiscoveryConfig;
use crate::runtime::ReamActor;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main P2P node that integrates all distributed system components
#[derive(Debug)]
pub struct ReamNode {
    /// Node configuration
    config: NodeConfig,
    /// Local node information
    node_info: NodeInfo,
    /// Network layer for communication
    network: Arc<RwLock<NetworkLayer>>,
    /// Node discovery system
    discovery: Arc<RwLock<NodeDiscovery>>,
    /// Consensus engine
    consensus: Arc<RwLock<ConsensusEngine>>,
    /// Distributed actor registry
    actor_registry: Arc<RwLock<DistributedActorRegistry>>,
    /// Cluster manager
    cluster_manager: Arc<RwLock<ClusterManager>>,
    /// Migration manager
    migration_manager: Arc<RwLock<MigrationManager>>,
    /// Node state
    state: Arc<RwLock<NodeState>>,
}

impl ReamNode {
    /// Create a new REAM node
    pub async fn new(config: NodeConfig) -> P2PResult<Self> {
        let node_info = NodeInfo::new(config.node_id, config.bind_address);

        // Create network layer
        let network_config = crate::p2p::network::NetworkConfig::default();
        let network = Arc::new(RwLock::new(NetworkLayer::new(network_config).await?));

        // Create discovery system
        let discovery_config = DiscoveryConfig::default();
        let discovery = Arc::new(RwLock::new(
            NodeDiscovery::new(node_info.clone(), discovery_config).await?
        ));

        // Create consensus engine
        let consensus_config = crate::p2p::consensus::ConsensusConfig::default();
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(consensus_config)?));

        // Create actor registry
        let actor_registry = Arc::new(RwLock::new(DistributedActorRegistry::new()));

        // Create cluster manager
        let cluster_config = crate::p2p::cluster::ClusterConfig::default();
        let cluster_manager = Arc::new(RwLock::new(
            ClusterManager::new(node_info.clone(), cluster_config).await?
        ));

        // Create migration manager
        let migration_manager = Arc::new(RwLock::new(MigrationManager::new()));

        // Initialize node state
        let state = Arc::new(RwLock::new(NodeState::Initializing));

        Ok(Self {
            config,
            node_info,
            network,
            discovery,
            consensus,
            actor_registry,
            cluster_manager,
            migration_manager,
            state,
        })
    }

    /// Start the node
    pub async fn start(&mut self) -> P2PResult<()> {
        // Update state
        *self.state.write().await = NodeState::Starting;

        // Start network layer
        {
            let network = self.network.read().await;
            network.start().await?;
        }

        // Start discovery system
        {
            let discovery = self.discovery.read().await;
            discovery.start().await?;
        }

        // Start consensus engine
        {
            let consensus = self.consensus.read().await;
            consensus.start().await?;
        }

        // Start cluster manager
        {
            let cluster_manager = self.cluster_manager.read().await;
            cluster_manager.start().await?;
        }

        // Update state
        *self.state.write().await = NodeState::Running;

        Ok(())
    }

    /// Stop the node
    pub async fn stop(&mut self) -> P2PResult<()> {
        // Update state
        *self.state.write().await = NodeState::Stopping;

        // Stop components in reverse order
        {
            let cluster_manager = self.cluster_manager.read().await;
            cluster_manager.stop().await?;
        }

        {
            let consensus = self.consensus.read().await;
            consensus.stop().await?;
        }

        {
            let discovery = self.discovery.read().await;
            discovery.stop().await?;
        }

        {
            let network = self.network.read().await;
            network.stop().await?;
        }

        // Update state
        *self.state.write().await = NodeState::Stopped;

        Ok(())
    }

    /// Create a new cluster
    pub async fn create_cluster(&mut self) -> P2PResult<()> {
        // Bootstrap consensus
        {
            let consensus = self.consensus.read().await;
            consensus.bootstrap_consensus().await?;
        }

        // Create cluster through discovery
        {
            let discovery = self.discovery.read().await;
            discovery.create_cluster().await?;
        }

        // Create cluster through cluster manager
        {
            let cluster_manager = self.cluster_manager.read().await;
            cluster_manager.create_cluster().await?;
        }

        Ok(())
    }

    /// Join an existing cluster
    pub async fn join_cluster(&mut self, bootstrap_nodes: Vec<NodeInfo>) -> P2PResult<()> {
        // Join through discovery
        {
            let discovery = self.discovery.read().await;
            discovery.join_cluster(bootstrap_nodes.clone()).await?;
        }

        // Join consensus
        {
            let consensus = self.consensus.read().await;
            consensus.join_cluster().await?;
        }

        // Join cluster through cluster manager
        {
            let cluster_manager = self.cluster_manager.read().await;
            cluster_manager.join_cluster(bootstrap_nodes).await?;
        }

        Ok(())
    }

    /// Get cluster information
    pub async fn get_cluster_info(&self) -> P2PResult<ClusterInfo> {
        let cluster_manager = self.cluster_manager.read().await;
        cluster_manager.get_cluster_info().await
    }

    /// Add a member to the cluster
    pub async fn add_cluster_member(&self, node_info: NodeInfo) -> P2PResult<()> {
        let cluster_manager = self.cluster_manager.read().await;
        cluster_manager.add_member(node_info).await
    }

    /// Get node information
    pub async fn get_node_info(&self) -> P2PResult<NodeInfo> {
        Ok(self.node_info.clone())
    }

    /// Spawn a distributed actor
    pub async fn spawn_distributed_actor<A>(
        &mut self,
        _actor: A,
        _placement_constraints: Option<PlacementConstraints>,
    ) -> P2PResult<DistributedActorRef>
    where
        A: ReamActor + Send + Sync + 'static,
    {
        let actor_registry = self.actor_registry.read().await;
        actor_registry.spawn_local_actor("generic_actor".to_string(), self.node_info.node_id).await
    }

    /// Migrate an actor to another node
    pub async fn migrate_actor(
        &mut self,
        actor_id: ActorId,
        target_node: NodeId,
    ) -> P2PResult<MigrationResult> {
        let migration_manager = self.migration_manager.read().await;
        migration_manager.migrate_actor(actor_id, target_node).await
    }

    /// Get node state
    pub async fn get_state(&self) -> NodeState {
        self.state.read().await.clone()
    }

    /// Check if node is healthy
    pub async fn is_healthy(&self) -> bool {
        matches!(*self.state.read().await, NodeState::Running)
    }
}

/// Node state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    /// Node is initializing
    Initializing,
    /// Node is starting up
    Starting,
    /// Node is running normally
    Running,
    /// Node is stopping
    Stopping,
    /// Node is stopped
    Stopped,
    /// Node has failed
    Failed(String),
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Initializing => write!(f, "Initializing"),
            NodeState::Starting => write!(f, "Starting"),
            NodeState::Running => write!(f, "Running"),
            NodeState::Stopping => write!(f, "Stopping"),
            NodeState::Stopped => write!(f, "Stopped"),
            NodeState::Failed(reason) => write!(f, "Failed: {}", reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_node_creation() {
        let config = NodeConfig::default();
        let result = ReamNode::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_node_lifecycle() {
        let config = NodeConfig::default();
        let mut node = ReamNode::new(config).await.unwrap();
        
        assert_eq!(node.get_state().await, NodeState::Initializing);
        
        assert!(node.start().await.is_ok());
        assert_eq!(node.get_state().await, NodeState::Running);
        assert!(node.is_healthy().await);
        
        assert!(node.stop().await.is_ok());
        assert_eq!(node.get_state().await, NodeState::Stopped);
    }

    #[tokio::test]
    async fn test_cluster_operations() {
        let config = NodeConfig::default();
        let mut node = ReamNode::new(config).await.unwrap();
        
        node.start().await.unwrap();
        
        // Test cluster creation
        assert!(node.create_cluster().await.is_ok());
        
        let cluster_info = node.get_cluster_info().await.unwrap();
        assert_eq!(cluster_info.member_count, 1);
        
        node.stop().await.unwrap();
    }
}
