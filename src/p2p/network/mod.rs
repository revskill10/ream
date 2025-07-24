//! Network layer for P2P communication
//!
//! Provides session-typed network protocols, message routing, and
//! transport layer abstraction for the distributed system.

pub mod session_types;
pub mod protocol;
pub mod transport;
pub mod router;

pub use session_types::*;
pub use protocol::*;
pub use transport::*;
pub use router::*;

use crate::p2p::{P2PResult, P2PError, NetworkError, NodeId, NodeInfo};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Main network layer for P2P communication
#[derive(Debug)]
pub struct NetworkLayer {
    /// Transport layer for actual network communication
    transport: Arc<RwLock<Transport>>,
    /// Message router for distributed message routing
    router: Arc<RwLock<MessageRouter>>,
    /// Active connections to other nodes
    connections: Arc<RwLock<HashMap<NodeId, Connection>>>,
    /// Network configuration
    config: NetworkConfig,
}

impl NetworkLayer {
    /// Create a new network layer
    pub async fn new(config: NetworkConfig) -> P2PResult<Self> {
        let transport = Arc::new(RwLock::new(Transport::new(config.clone()).await?));
        let router = Arc::new(RwLock::new(MessageRouter::new()));
        let connections = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            transport,
            router,
            connections,
            config,
        })
    }

    /// Start the network layer
    pub async fn start(&self) -> P2PResult<()> {
        let mut transport = self.transport.write().await;
        transport.start().await?;
        Ok(())
    }

    /// Stop the network layer
    pub async fn stop(&self) -> P2PResult<()> {
        let mut transport = self.transport.write().await;
        transport.stop().await?;
        Ok(())
    }

    /// Get the bind address of this network layer
    pub fn get_bind_address(&self) -> SocketAddr {
        self.config.bind_address
    }

    /// Connect to a remote node
    pub async fn connect_to_node(&self, node_info: NodeInfo, our_node_id: NodeId) -> P2PResult<()> {
        let mut transport = self.transport.write().await;
        let peer_node_id = transport.connect(node_info.address, our_node_id).await?;

        // Verify the peer node ID matches what we expected
        if peer_node_id != node_info.node_id {
            return Err(P2PError::Network(NetworkError::ConnectionFailed(
                "Node ID mismatch during handshake".to_string()
            )));
        }

        Ok(())
    }

    /// Disconnect from a node
    pub async fn disconnect_from_node(&self, node_id: NodeId) -> P2PResult<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.remove(&node_id) {
            connection.close().await?;
        }
        Ok(())
    }

    /// Send a message to a specific node
    pub async fn send_message(&self, target: NodeId, message: NetworkMessage) -> P2PResult<()> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(&target) {
            connection.send(message).await?;
            Ok(())
        } else {
            Err(P2PError::Network(NetworkError::ConnectionFailed(
                format!("No connection to node {}", target)
            )))
        }
    }

    /// Broadcast a message to all connected nodes
    pub async fn broadcast_message(&self, message: NetworkMessage) -> P2PResult<()> {
        let connections = self.connections.read().await;
        let mut errors = Vec::new();

        for (node_id, connection) in connections.iter() {
            if let Err(e) = connection.send(message.clone()).await {
                errors.push((*node_id, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(P2PError::Network(NetworkError::SendFailed(
                format!("Failed to send to {} nodes", errors.len())
            )))
        }
    }

    /// Route a message through the network
    pub async fn route_message(&self, message: RoutingMessage) -> P2PResult<()> {
        let router = self.router.read().await;
        router.route_message(self, message).await
    }

    /// Get list of connected nodes
    pub async fn get_connected_nodes(&self) -> Vec<NodeId> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Check if connected to a specific node
    pub async fn is_connected_to(&self, node_id: NodeId) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(&node_id)
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> NetworkStats {
        let connections = self.connections.read().await;
        let transport = self.transport.read().await;
        
        NetworkStats {
            connected_nodes: connections.len(),
            total_messages_sent: transport.get_messages_sent(),
            total_messages_received: transport.get_messages_received(),
            total_bytes_sent: transport.get_bytes_sent(),
            total_bytes_received: transport.get_bytes_received(),
            active_connections: connections.len(),
        }
    }
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Bind address for the network layer
    pub bind_address: SocketAddr,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Connection timeout
    pub connection_timeout: std::time::Duration,
    /// Keep-alive interval
    pub keep_alive_interval: std::time::Duration,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Buffer size for message queues
    pub buffer_size: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:0".parse().unwrap(),
            max_message_size: 1024 * 1024, // 1MB
            connection_timeout: std::time::Duration::from_secs(30),
            keep_alive_interval: std::time::Duration::from_secs(60),
            max_connections: 1000,
            buffer_size: 1024,
        }
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Number of connected nodes
    pub connected_nodes: usize,
    /// Total messages sent
    pub total_messages_sent: u64,
    /// Total messages received
    pub total_messages_received: u64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Number of active connections
    pub active_connections: usize,
}

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Ping message for connectivity testing
    Ping { timestamp: u64 },
    /// Pong response to ping
    Pong { timestamp: u64 },
    /// Node discovery message
    Discovery(DiscoveryMessage),
    /// Consensus message
    Consensus(ConsensusMessage),
    /// Actor message
    Actor(ActorMessage),
    /// Cluster management message
    Cluster(ClusterMessage),
    /// Custom application message
    Custom(Vec<u8>),
    /// Connection handshake
    Handshake {
        node_id: NodeId,
        protocol_version: u32,
    },
    /// Connection acknowledgment
    HandshakeAck {
        node_id: NodeId,
        accepted: bool,
    },
}

/// Discovery message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMessage {
    /// Find node request
    FindNode { target: NodeId, requester: NodeId },
    /// Find node response
    FindNodeResponse { nodes: Vec<NodeInfo> },
    /// Join cluster request
    JoinCluster { node_info: NodeInfo },
    /// Join cluster response
    JoinClusterResponse { accepted: bool, cluster_info: Option<ClusterInfo> },
}

/// Consensus message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    /// Raft messages
    Raft(RaftMessage),
    /// PBFT messages
    PBFT(PBFTMessage),
}

/// Actor message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorMessage {
    /// Spawn actor request
    SpawnActor { actor_type: String, init_data: Vec<u8> },
    /// Actor message delivery
    DeliverMessage { target_actor: crate::p2p::ActorId, payload: Vec<u8> },
    /// Actor migration request
    MigrateActor { actor_id: crate::p2p::ActorId, target_node: NodeId },
}

/// Cluster management message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    /// Heartbeat message
    Heartbeat { node_id: NodeId, timestamp: u64 },
    /// Node failure notification
    NodeFailure { failed_node: NodeId, detector: NodeId },
    /// Membership update
    MembershipUpdate { members: Vec<NodeInfo> },
}

/// Routing message with path information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMessage {
    /// Source node
    pub source: NodeId,
    /// Destination node
    pub destination: NodeId,
    /// Message payload
    pub payload: NetworkMessage,
    /// Routing path (for debugging)
    pub path: Vec<NodeId>,
    /// Time-to-live
    pub ttl: u8,
}

// Import types that will be defined in other modules
use crate::p2p::ClusterInfo;

// Placeholder types for messages that will be defined in consensus modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftMessage {
    pub message_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PBFTMessage {
    pub message_type: String,
    pub data: Vec<u8>,
}

/// Protocol for encoding/decoding messages over TCP
pub struct MessageProtocol;

impl MessageProtocol {
    /// Encode a message for network transmission
    pub fn encode(message: &NetworkMessage) -> Result<Vec<u8>, std::io::Error> {
        let serialized = bincode::serialize(message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Prepend message length (4 bytes, big-endian)
        let mut encoded = Vec::with_capacity(4 + serialized.len());
        encoded.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&serialized);

        Ok(encoded)
    }

    /// Decode a message from network data
    pub fn decode(data: &[u8]) -> Result<NetworkMessage, std::io::Error> {
        bincode::deserialize(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Read a complete message from a TCP stream
    pub async fn read_message<R: AsyncReadExt + Unpin>(
        reader: &mut R
    ) -> Result<NetworkMessage, std::io::Error> {
        // Read message length (4 bytes)
        let mut length_bytes = [0u8; 4];
        reader.read_exact(&mut length_bytes).await?;
        let message_length = u32::from_be_bytes(length_bytes) as usize;

        // Validate message length
        if message_length > 1024 * 1024 {  // 1MB max message size
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message too large"
            ));
        }

        // Read message data
        let mut message_data = vec![0u8; message_length];
        reader.read_exact(&mut message_data).await?;

        // Decode message
        Self::decode(&message_data)
    }

    /// Write a message to a TCP stream
    pub async fn write_message<W: AsyncWriteExt + Unpin>(
        writer: &mut W,
        message: &NetworkMessage
    ) -> Result<(), std::io::Error> {
        let encoded = Self::encode(message)?;
        writer.write_all(&encoded).await?;
        writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_network_layer_creation() {
        let config = NetworkConfig::default();
        let result = NetworkLayer::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_network_layer_start_stop() {
        let config = NetworkConfig::default();
        let network = NetworkLayer::new(config).await.unwrap();
        
        assert!(network.start().await.is_ok());
        assert!(network.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_network_stats() {
        let config = NetworkConfig::default();
        let network = NetworkLayer::new(config).await.unwrap();
        
        let stats = network.get_network_stats().await;
        assert_eq!(stats.connected_nodes, 0);
        assert_eq!(stats.active_connections, 0);
    }
}
