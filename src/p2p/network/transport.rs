//! Transport layer for network communication
//!
//! Provides low-level network transport using TCP with connection pooling,
//! message framing, and error handling.

use crate::p2p::{P2PResult, P2PError, NetworkError, NodeId};
use super::{NetworkMessage, NetworkConfig};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, mpsc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use bytes::{Buf, BufMut, BytesMut};

/// Transport layer for network communication
#[derive(Debug)]
pub struct Transport {
    /// Local bind address
    bind_address: SocketAddr,
    /// TCP listener for incoming connections
    listener: Option<TcpListener>,
    /// Active connections
    connections: Arc<RwLock<HashMap<NodeId, Connection>>>,
    /// Configuration
    config: NetworkConfig,
    /// Statistics
    stats: Arc<RwLock<TransportStats>>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl Transport {
    /// Create a new transport layer
    pub async fn new(config: NetworkConfig) -> P2PResult<Self> {
        Ok(Self {
            bind_address: "127.0.0.1:0".parse().unwrap(), // Will be set when starting
            listener: None,
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(TransportStats::default())),
            shutdown_tx: None,
        })
    }

    /// Start the transport layer
    pub async fn start(&mut self) -> P2PResult<()> {
        // Bind to address
        let listener = TcpListener::bind(&self.bind_address).await
            .map_err(|e| P2PError::Network(NetworkError::BindError(e.to_string())))?;

        self.bind_address = listener.local_addr()
            .map_err(|e| P2PError::Network(NetworkError::BindError(e.to_string())))?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Start accepting connections
        let connections = Arc::clone(&self.connections);
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();

        // Spawn background task to handle incoming connections
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle shutdown signal
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    // Accept new connections
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                let connections = Arc::clone(&connections);
                                let stats = Arc::clone(&stats);
                                let config = config.clone();

                                // Spawn task to handle this connection
                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_connection(stream, addr, connections, stats, config).await {
                                        eprintln!("Connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("Failed to accept connection: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the transport layer
    pub async fn stop(&mut self) -> P2PResult<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        // Close all connections
        let mut connections = self.connections.write().await;
        for (_, connection) in connections.drain() {
            let _ = connection.close().await;
        }

        self.listener = None;
        Ok(())
    }

    /// Connect to a remote address
    pub async fn connect(&mut self, addr: SocketAddr, our_node_id: NodeId) -> P2PResult<NodeId> {
        use super::{NetworkMessage, MessageProtocol};

        let mut stream = TcpStream::connect(addr).await
            .map_err(|e| P2PError::Network(NetworkError::ConnectionFailed(e.to_string())))?;

        // Perform handshake
        let handshake = NetworkMessage::Handshake {
            node_id: our_node_id,
            protocol_version: 1,
        };

        MessageProtocol::write_message(&mut stream, &handshake).await
            .map_err(|e| P2PError::Network(NetworkError::ConnectionFailed(e.to_string())))?;

        // Wait for acknowledgment
        let ack_msg = MessageProtocol::read_message(&mut stream).await
            .map_err(|e| P2PError::Network(NetworkError::ConnectionFailed(e.to_string())))?;

        let peer_node_id = match ack_msg {
            NetworkMessage::HandshakeAck { node_id, accepted } => {
                if !accepted {
                    return Err(P2PError::Network(NetworkError::ConnectionFailed(
                        "Handshake rejected".to_string()
                    )));
                }
                node_id
            }
            _ => {
                return Err(P2PError::Network(NetworkError::ConnectionFailed(
                    "Invalid handshake response".to_string()
                )));
            }
        };

        // Create connection object
        let connection = Connection::new(stream, self.config.clone()).await?;

        // Add to connections
        self.connections.write().await.insert(peer_node_id, connection);
        self.stats.write().await.connections_established += 1;

        Ok(peer_node_id)
    }

    /// Get the bind address
    pub fn get_bind_address(&self) -> SocketAddr {
        self.bind_address
    }

    /// Get transport statistics
    pub fn get_messages_sent(&self) -> u64 {
        // This would be implemented with proper async access in real code
        0 // Placeholder
    }

    pub fn get_messages_received(&self) -> u64 {
        0 // Placeholder
    }

    pub fn get_bytes_sent(&self) -> u64 {
        0 // Placeholder
    }

    pub fn get_bytes_received(&self) -> u64 {
        0 // Placeholder
    }

    /// Handle an incoming connection
    async fn handle_connection(
        mut stream: TcpStream,
        addr: SocketAddr,
        connections: Arc<RwLock<HashMap<NodeId, Connection>>>,
        stats: Arc<RwLock<TransportStats>>,
        _config: NetworkConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use super::{NetworkMessage, MessageProtocol};

        // Perform handshake
        let handshake_msg = MessageProtocol::read_message(&mut stream).await?;

        let peer_node_id = match handshake_msg {
            NetworkMessage::Handshake { node_id, protocol_version } => {
                if protocol_version != 1 {
                    let ack = NetworkMessage::HandshakeAck {
                        node_id: NodeId::new(), // Our node ID
                        accepted: false,
                    };
                    MessageProtocol::write_message(&mut stream, &ack).await?;
                    return Err("Unsupported protocol version".into());
                }

                // Send acknowledgment
                let ack = NetworkMessage::HandshakeAck {
                    node_id: NodeId::new(), // Our node ID
                    accepted: true,
                };
                MessageProtocol::write_message(&mut stream, &ack).await?;

                node_id
            }
            _ => {
                return Err("Expected handshake message".into());
            }
        };

        // Update stats
        {
            let mut transport_stats = stats.write().await;
            transport_stats.connections_established += 1;
        }

        // Handle messages from this connection
        loop {
            match MessageProtocol::read_message(&mut stream).await {
                Ok(message) => {
                    // Update stats
                    {
                        let mut transport_stats = stats.write().await;
                        transport_stats.messages_received += 1;
                    }

                    // Process message (for now, just echo back)
                    match message {
                        NetworkMessage::Ping { timestamp } => {
                            let pong = NetworkMessage::Pong { timestamp };
                            MessageProtocol::write_message(&mut stream, &pong).await?;
                        }
                        _ => {
                            // Handle other message types
                            println!("Received message from {}: {:?}", peer_node_id, message);
                        }
                    }
                }
                Err(e) => {
                    println!("Connection error with {}: {}", peer_node_id, e);
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Network connection wrapper
#[derive(Debug, Clone)]
pub struct Connection {
    /// Connection ID
    id: uuid::Uuid,
    /// Remote address
    remote_addr: SocketAddr,
    /// Message sender
    message_tx: mpsc::Sender<NetworkMessage>,
    /// Connection statistics
    stats: Arc<RwLock<ConnectionStats>>,
}

impl Connection {
    /// Create a new connection
    pub async fn new(stream: TcpStream, config: NetworkConfig) -> P2PResult<Self> {
        let remote_addr = stream.peer_addr()
            .map_err(|e| P2PError::Network(NetworkError::ConnectionFailed(e.to_string())))?;

        let (message_tx, mut message_rx) = mpsc::channel(config.buffer_size);
        let stats = Arc::new(RwLock::new(ConnectionStats::default()));

        let connection_stats = Arc::clone(&stats);
        
        // Start message handling task
        tokio::spawn(async move {
            let (mut reader, mut writer) = stream.into_split();
            let mut read_buffer = BytesMut::with_capacity(config.max_message_size);

            loop {
                tokio::select! {
                    // Handle outgoing messages
                    message = message_rx.recv() => {
                        match message {
                            Some(msg) => {
                                if let Err(e) = Self::send_message(&mut writer, &msg).await {
                                    eprintln!("Failed to send message: {}", e);
                                    break;
                                }
                                connection_stats.write().await.messages_sent += 1;
                            }
                            None => break, // Channel closed
                        }
                    }
                    // Handle incoming messages
                    result = Self::read_message(&mut reader, &mut read_buffer, config.max_message_size) => {
                        match result {
                            Ok(Some(_msg)) => {
                                // Handle received message
                                connection_stats.write().await.messages_received += 1;
                            }
                            Ok(None) => {
                                // Connection closed
                                break;
                            }
                            Err(e) => {
                                eprintln!("Failed to read message: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            remote_addr,
            message_tx,
            stats,
        })
    }

    /// Send a message through this connection
    pub async fn send(&self, message: NetworkMessage) -> P2PResult<()> {
        self.message_tx.send(message).await
            .map_err(|e| P2PError::Network(NetworkError::SendFailed(e.to_string())))
    }

    /// Close the connection
    pub async fn close(self) -> P2PResult<()> {
        // The connection will be closed when the sender is dropped
        Ok(())
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> ConnectionStats {
        self.stats.read().await.clone()
    }

    /// Send a message over the wire
    async fn send_message(
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        message: &NetworkMessage,
    ) -> P2PResult<()> {
        // Serialize message
        let data = bincode::serialize(message)
            .map_err(|e| P2PError::Serialization(e.to_string()))?;

        // Write length prefix
        let len = data.len() as u32;
        writer.write_u32(len).await
            .map_err(|e| P2PError::Network(NetworkError::SendFailed(e.to_string())))?;

        // Write message data
        writer.write_all(&data).await
            .map_err(|e| P2PError::Network(NetworkError::SendFailed(e.to_string())))?;

        writer.flush().await
            .map_err(|e| P2PError::Network(NetworkError::SendFailed(e.to_string())))?;

        Ok(())
    }

    /// Read a message from the wire
    async fn read_message(
        reader: &mut tokio::net::tcp::OwnedReadHalf,
        buffer: &mut BytesMut,
        max_size: usize,
    ) -> P2PResult<Option<NetworkMessage>> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        match reader.read_exact(&mut len_bytes).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(None); // Connection closed
            }
            Err(e) => {
                return Err(P2PError::Network(NetworkError::ReceiveFailed(e.to_string())));
            }
        }

        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > max_size {
            return Err(P2PError::Network(NetworkError::InvalidMessage(
                format!("Message too large: {} bytes", len)
            )));
        }

        // Read message data
        buffer.clear();
        buffer.resize(len, 0);
        reader.read_exact(buffer).await
            .map_err(|e| P2PError::Network(NetworkError::ReceiveFailed(e.to_string())))?;

        // Deserialize message
        let message = bincode::deserialize(buffer)
            .map_err(|e| P2PError::Serialization(e.to_string()))?;

        Ok(Some(message))
    }
}

/// Transport layer statistics
#[derive(Debug, Default, Clone)]
pub struct TransportStats {
    /// Number of connections accepted
    pub connections_accepted: u64,
    /// Number of connections established
    pub connections_established: u64,
    /// Number of connections failed
    pub connections_failed: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Messages sent through this connection
    pub messages_sent: u64,
    /// Messages received through this connection
    pub messages_received: u64,
    /// Bytes sent through this connection
    pub bytes_sent: u64,
    /// Bytes received through this connection
    pub bytes_received: u64,
    /// Connection start time
    pub connected_at: std::time::SystemTime,
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connected_at: std::time::SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_transport_creation() {
        let config = NetworkConfig::default();
        let result = Transport::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transport_start_stop() {
        let config = NetworkConfig::default();
        let mut transport = Transport::new(config).await.unwrap();
        
        assert!(transport.start().await.is_ok());
        assert!(transport.stop().await.is_ok());
    }
}
