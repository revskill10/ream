//! Core types for the P2P distributed system
//!
//! This module defines the fundamental data structures used throughout
//! the P2P system, maintaining mathematical rigor and type safety.

use std::collections::{HashMap, BTreeMap};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a node in the P2P network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Generate a new random node ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }
    
    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
    
    /// Calculate XOR distance to another node (for DHT)
    pub fn distance_to(&self, other: &NodeId) -> u128 {
        let self_bytes = self.0.as_u128();
        let other_bytes = other.0.as_u128();
        self_bytes ^ other_bytes
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an actor in the distributed system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub Uuid);

impl ActorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ActorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about a node in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub node_id: NodeId,
    /// Network address
    pub address: SocketAddr,
    /// Node capabilities
    pub capabilities: NodeCapabilities,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Node version
    pub version: String,
    /// Public key for cryptographic operations
    pub public_key: Vec<u8>,
}

impl NodeInfo {
    pub fn new(node_id: NodeId, address: SocketAddr) -> Self {
        Self {
            node_id,
            address,
            capabilities: NodeCapabilities::default(),
            last_seen: SystemTime::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            public_key: vec![], // TODO: Generate actual public key
        }
    }
    
    /// Check if node is considered alive
    pub fn is_alive(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed().unwrap_or(Duration::MAX) < timeout
    }
    
    /// Update last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now();
    }
}

/// Node capabilities and features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Maximum number of actors this node can host
    pub max_actors: usize,
    /// Available memory in bytes
    pub available_memory: u64,
    /// CPU cores available
    pub cpu_cores: usize,
    /// Supported consensus algorithms
    pub consensus_algorithms: Vec<ConsensusAlgorithm>,
    /// Node type (gateway, worker, storage, etc.)
    pub node_type: NodeType,
    /// Custom capabilities
    pub custom: HashMap<String, String>,
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            max_actors: 10000,
            available_memory: 1024 * 1024 * 1024, // 1GB
            cpu_cores: num_cpus::get(),
            consensus_algorithms: vec![ConsensusAlgorithm::Raft, ConsensusAlgorithm::PBFT],
            node_type: NodeType::Worker,
            custom: HashMap::new(),
        }
    }
}

/// Types of nodes in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    /// Gateway node (handles external connections)
    Gateway,
    /// Worker node (runs actors)
    Worker,
    /// Storage node (persistent data)
    Storage,
    /// Coordinator node (cluster management)
    Coordinator,
    /// Custom node type
    Custom(String),
}

/// Supported consensus algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusAlgorithm {
    /// Raft consensus
    Raft,
    /// Practical Byzantine Fault Tolerance
    PBFT,
    /// Custom consensus algorithm
    Custom(String),
}

/// Cluster information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// Cluster identifier
    pub cluster_id: Uuid,
    /// Number of members in cluster
    pub member_count: usize,
    /// List of cluster members
    pub members: Vec<NodeInfo>,
    /// Current cluster leader (if applicable)
    pub leader: Option<NodeId>,
    /// Cluster health status
    pub health: ClusterHealth,
    /// Cluster formation timestamp
    pub formed_at: SystemTime,
}

/// Cluster health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClusterHealth {
    /// All nodes healthy
    Healthy,
    /// Some nodes degraded but cluster operational
    Degraded,
    /// Cluster partitioned
    Partitioned,
    /// Cluster unhealthy
    Unhealthy,
}

/// Placement constraints for actor spawning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementConstraints {
    /// Preferred node type
    pub node_type: Option<NodeType>,
    /// Specific node ID
    pub node_id: Option<NodeId>,
    /// Minimum memory requirement
    pub min_memory: Option<u64>,
    /// Minimum CPU cores
    pub min_cpu_cores: Option<usize>,
    /// Custom constraints
    pub custom: HashMap<String, String>,
}

impl PlacementConstraints {
    pub fn node(node_id: NodeId) -> Self {
        Self {
            node_type: None,
            node_id: Some(node_id),
            min_memory: None,
            min_cpu_cores: None,
            custom: HashMap::new(),
        }
    }
    
    pub fn node_type(node_type: NodeType) -> Self {
        Self {
            node_type: Some(node_type),
            node_id: None,
            min_memory: None,
            min_cpu_cores: None,
            custom: HashMap::new(),
        }
    }
}

impl Default for PlacementConstraints {
    fn default() -> Self {
        Self {
            node_type: None,
            node_id: None,
            min_memory: None,
            min_cpu_cores: None,
            custom: HashMap::new(),
        }
    }
}

/// Configuration for P2P node
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Node identifier
    pub node_id: NodeId,
    /// Bind address for network communication
    pub bind_address: SocketAddr,
    /// Node capabilities
    pub capabilities: NodeCapabilities,
    /// Bootstrap nodes for joining cluster
    pub bootstrap_nodes: Vec<NodeInfo>,
    /// Network timeouts
    pub network_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: NodeId::new(),
            bind_address: "127.0.0.1:0".parse().unwrap(),
            capabilities: NodeCapabilities::default(),
            bootstrap_nodes: vec![],
            network_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(5),
        }
    }
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Maximum message size
    pub max_message_size: usize,
    /// Connection pool size
    pub connection_pool_size: usize,
    /// Message timeout
    pub message_timeout: Duration,
    /// Retry attempts
    pub retry_attempts: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_message_size: 1024 * 1024, // 1MB
            connection_pool_size: 100,
            message_timeout: Duration::from_secs(10),
            retry_attempts: 3,
        }
    }
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Consensus algorithm to use
    pub algorithm: ConsensusAlgorithm,
    /// Election timeout
    pub election_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Maximum log entries per batch
    pub max_log_batch_size: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            algorithm: ConsensusAlgorithm::Raft,
            election_timeout: Duration::from_millis(150),
            heartbeat_interval: Duration::from_millis(50),
            max_log_batch_size: 100,
        }
    }
}

/// Cluster configuration
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Gossip interval
    pub gossip_interval: Duration,
    /// Failure detection timeout
    pub failure_timeout: Duration,
    /// Maximum cluster size
    pub max_cluster_size: usize,
    /// Minimum cluster size for operation
    pub min_cluster_size: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            gossip_interval: Duration::from_secs(1),
            failure_timeout: Duration::from_secs(30),
            max_cluster_size: 1000,
            min_cluster_size: 1,
        }
    }
}
