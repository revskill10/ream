//! Integration tests for P2P distributed system
//!
//! Comprehensive tests for the P2P distributed system including
//! cluster formation, consensus, actor migration, and fault tolerance.

use ream::p2p::*;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_single_node_cluster_creation() {
    let config = P2PConfig::default();
    let node = initialize_p2p_system(config).await.unwrap();
    
    // Create cluster
    let result = create_cluster(node.clone()).await;
    assert!(result.is_ok());
    
    // Verify cluster info
    let cluster_info = get_cluster_info(node.clone()).await.unwrap();
    assert_eq!(cluster_info.member_count, 1);
    assert_eq!(cluster_info.health, ClusterHealth::Healthy);
}

#[tokio::test]
async fn test_two_node_cluster_formation() {
    // Create first node (bootstrap)
    let config1 = P2PConfig::default();
    let node1 = initialize_p2p_system(config1).await.unwrap();
    create_cluster(node1.clone()).await.unwrap();
    
    // Get bootstrap node info
    let bootstrap_info = {
        let node = node1.read().await;
        node.get_node_info().await.unwrap()
    };
    
    // Create second node
    let config2 = P2PConfig::default();
    let node2 = initialize_p2p_system(config2).await.unwrap();
    
    // Join cluster
    let result = join_cluster(node2.clone(), vec![bootstrap_info]).await;
    assert!(result.is_ok());
    
    // Verify both nodes see the cluster
    let cluster_info1 = timeout(
        Duration::from_secs(5),
        get_cluster_info(node1)
    ).await.unwrap().unwrap();
    
    let cluster_info2 = timeout(
        Duration::from_secs(5),
        get_cluster_info(node2)
    ).await.unwrap().unwrap();
    
    assert_eq!(cluster_info1.member_count, 2);
    assert_eq!(cluster_info2.member_count, 2);
    assert_eq!(cluster_info1.cluster_id, cluster_info2.cluster_id);
}

#[tokio::test]
async fn test_distributed_actor_spawning() {
    // Create cluster
    let config = P2PConfig::default();
    let node = initialize_p2p_system(config).await.unwrap();
    create_cluster(node.clone()).await.unwrap();
    
    // Spawn distributed actor
    let actor_ref = spawn_distributed_actor(
        node.clone(),
        TestActor::default(),
        None,
    ).await.unwrap();
    
    assert!(!actor_ref.actor_id.to_string().is_empty());
    assert!(!actor_ref.actor_type.is_empty());
}

#[tokio::test]
async fn test_actor_migration() {
    // Create two-node cluster
    let config1 = P2PConfig::default();
    let node1 = initialize_p2p_system(config1).await.unwrap();
    create_cluster(node1.clone()).await.unwrap();
    
    let bootstrap_info = {
        let node = node1.read().await;
        node.get_node_info().await.unwrap()
    };
    
    let config2 = P2PConfig::default();
    let node2 = initialize_p2p_system(config2).await.unwrap();
    join_cluster(node2.clone(), vec![bootstrap_info]).await.unwrap();
    
    // Spawn actor on node1
    let actor_ref = spawn_distributed_actor(
        node1.clone(),
        TestActor::default(),
        None,
    ).await.unwrap();
    
    // Get node2 ID
    let target_node = {
        let node = node2.read().await;
        node.get_node_info().await.unwrap().node_id
    };
    
    // Migrate actor to node2
    let migration_result = migrate_actor(
        node1.clone(),
        actor_ref.actor_id,
        target_node,
    ).await.unwrap();
    
    assert!(migration_result.success);
    assert_eq!(migration_result.target_node, target_node);
}

#[tokio::test]
async fn test_consensus_operations() {
    // Create cluster with consensus
    let mut config = P2PConfig::default();
    config.consensus_config.algorithm = ConsensusAlgorithm::Raft;
    
    let node = initialize_p2p_system(config).await.unwrap();
    create_cluster(node.clone()).await.unwrap();
    
    // Test consensus state
    let consensus_state = {
        let node = node.read().await;
        let consensus = node.consensus.read().await;
        consensus.get_state().await.unwrap()
    };
    
    assert_eq!(consensus_state.role, ConsensusRole::Leader);
    assert_eq!(consensus_state.cluster_size, 1);
}

#[tokio::test]
async fn test_network_layer_functionality() {
    let config = NetworkConfig::default();
    let network = NetworkLayer::new(config).await.unwrap();
    
    // Test network startup
    assert!(network.start().await.is_ok());
    
    // Test network stats
    let stats = network.get_network_stats().await;
    assert_eq!(stats.connected_nodes, 0);
    assert_eq!(stats.active_connections, 0);
    
    // Test network shutdown
    assert!(network.stop().await.is_ok());
}

#[tokio::test]
async fn test_dht_operations() {
    let node_info = NodeInfo::new(
        NodeId::new(),
        "127.0.0.1:8080".parse().unwrap(),
    );
    let config = DHTConfig::default();
    
    let mut dht = ReamDHT::new(node_info, config);
    
    // Test DHT lifecycle
    assert!(dht.start().await.is_ok());
    assert!(dht.initialize_network().await.is_ok());
    
    // Test node operations
    let test_node = NodeInfo::new(
        NodeId::new(),
        "127.0.0.1:8081".parse().unwrap(),
    );
    assert!(dht.add_node(test_node.clone()).await.is_ok());
    
    let known_nodes = dht.get_known_nodes().await;
    assert!(!known_nodes.is_empty());
    
    assert!(dht.stop().await.is_ok());
}

#[tokio::test]
async fn test_cluster_management() {
    let node_info = NodeInfo::new(
        NodeId::new(),
        "127.0.0.1:8080".parse().unwrap(),
    );
    let config = ClusterConfig::default();
    
    let manager = ClusterManager::new(node_info.clone(), config).await.unwrap();
    
    // Test cluster manager lifecycle
    assert!(manager.start().await.is_ok());
    
    // Test cluster creation
    let cluster_info = manager.create_cluster().await.unwrap();
    assert_eq!(cluster_info.member_count, 1);
    
    // Test health status
    let health = manager.get_health_status().await;
    assert_eq!(health, ClusterHealth::Healthy);
    
    assert!(manager.stop().await.is_ok());
}

#[tokio::test]
async fn test_session_types() {
    // Test session type creation
    let session = SessionType::send("Hello",
        SessionType::receive("World",
            SessionType::End));
    
    assert!(!session.is_complete());
    
    // Test session validation
    let result = session.validate_send("Hello");
    assert!(result.is_ok());
    
    let next_session = result.unwrap();
    let result = next_session.validate_receive("World");
    assert!(result.is_ok());
    
    let final_session = result.unwrap();
    assert!(final_session.is_complete());
}

#[tokio::test]
async fn test_consensus_algorithms() {
    // Test Raft consensus
    let raft_config = RaftConfig::default();
    let mut raft = RaftConsensus::new(raft_config).unwrap();
    
    assert!(raft.start().await.is_ok());
    assert!(raft.bootstrap().await.is_ok());
    
    let state = raft.get_state().await;
    assert_eq!(state.role, ConsensusRole::Leader);
    
    assert!(raft.stop().await.is_ok());
    
    // Test PBFT consensus
    let pbft_config = PBFTConfig::default();
    let mut pbft = PBFTConsensus::new(pbft_config).unwrap();
    
    assert!(pbft.start().await.is_ok());
    assert!(pbft.bootstrap().await.is_ok());
    
    let state = pbft.get_state().await;
    assert_eq!(state.role, ConsensusRole::Leader);
    
    assert!(pbft.stop().await.is_ok());
}

#[tokio::test]
async fn test_failure_detection() {
    let config = FailureDetectionConfig::default();
    let mut detector = FailureDetector::new(config);
    
    assert!(detector.start().await.is_ok());
    
    let node_id = NodeId::new();
    
    // Initially node should be considered alive
    assert!(detector.is_alive(node_id).await);
    
    // Report failure
    assert!(detector.report_failure(node_id).await.is_ok());
    assert!(!detector.is_alive(node_id).await);
    
    // Update heartbeat should revive node
    detector.update_heartbeat(node_id).await;
    assert!(detector.is_alive(node_id).await);
    
    assert!(detector.stop().await.is_ok());
}

// Test actor for distributed actor tests
#[derive(Default)]
struct TestActor {
    state: String,
}

impl ream::runtime::ReamActor for TestActor {
    fn handle_message(&mut self, _message: ream::runtime::Message) -> ream::runtime::ActorResult<()> {
        Ok(())
    }
    
    fn pre_start(&mut self) -> ream::runtime::ActorResult<()> {
        self.state = "started".to_string();
        Ok(())
    }
    
    fn post_stop(&mut self) -> ream::runtime::ActorResult<()> {
        self.state = "stopped".to_string();
        Ok(())
    }
}

#[tokio::test]
async fn test_p2p_system_integration() {
    // Test complete P2P system integration
    let config = P2PConfig::default();
    let node = initialize_p2p_system(config).await.unwrap();
    
    // Create cluster
    create_cluster(node.clone()).await.unwrap();
    
    // Spawn actor
    let actor_ref = spawn_distributed_actor(
        node.clone(),
        TestActor::default(),
        None,
    ).await.unwrap();
    
    // Get cluster info
    let cluster_info = get_cluster_info(node.clone()).await.unwrap();
    assert_eq!(cluster_info.member_count, 1);
    assert_eq!(cluster_info.health, ClusterHealth::Healthy);
    
    // Verify actor was spawned
    assert!(!actor_ref.actor_id.to_string().is_empty());
}

#[tokio::test]
async fn test_byzantine_fault_tolerance() {
    // Test Byzantine threshold calculations
    assert_eq!(byzantine_threshold(1), 0);
    assert_eq!(byzantine_threshold(4), 1);
    assert_eq!(byzantine_threshold(7), 2);
    assert_eq!(byzantine_threshold(10), 3);
    
    // Test fault tolerance checks
    assert!(can_tolerate_byzantine_faults(4, 1));
    assert!(!can_tolerate_byzantine_faults(4, 2));
    assert!(can_tolerate_byzantine_faults(7, 2));
    assert!(!can_tolerate_byzantine_faults(7, 3));
}

#[tokio::test]
async fn test_majority_consensus() {
    // Test majority threshold calculations
    assert_eq!(majority_threshold(1), 1);
    assert_eq!(majority_threshold(3), 2);
    assert_eq!(majority_threshold(5), 3);
    assert_eq!(majority_threshold(7), 4);
    
    // Test majority checks
    assert!(has_majority(3, 2));
    assert!(!has_majority(3, 1));
    assert!(has_majority(5, 3));
    assert!(!has_majority(5, 2));
}
