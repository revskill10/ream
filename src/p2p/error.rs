//! Error types for the P2P distributed system
//!
//! Comprehensive error handling for all P2P operations with detailed
//! error information for debugging and recovery.

use std::fmt;
use std::error::Error;
use std::io;
use std::net::AddrParseError;
use serde::{Deserialize, Serialize};
use tokio::time::error::Elapsed;

/// Result type for P2P operations
pub type P2PResult<T> = Result<T, P2PError>;

/// Main error type for P2P operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PError {
    /// Network-related errors
    Network(NetworkError),
    /// Consensus-related errors
    Consensus(ConsensusError),
    /// Actor-related errors
    Actor(ActorError),
    /// Cluster management errors
    Cluster(ClusterError),
    /// Discovery errors
    Discovery(DiscoveryError),
    /// Migration errors
    Migration(MigrationError),
    /// Configuration errors
    Configuration(String),
    /// Serialization errors
    Serialization(String),
    /// Timeout errors
    Timeout(String),
    /// Generic errors
    Generic(String),
}

impl fmt::Display for P2PError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            P2PError::Network(e) => write!(f, "Network error: {}", e),
            P2PError::Consensus(e) => write!(f, "Consensus error: {}", e),
            P2PError::Actor(e) => write!(f, "Actor error: {}", e),
            P2PError::Cluster(e) => write!(f, "Cluster error: {}", e),
            P2PError::Discovery(e) => write!(f, "Discovery error: {}", e),
            P2PError::Migration(e) => write!(f, "Migration error: {}", e),
            P2PError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            P2PError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            P2PError::Timeout(msg) => write!(f, "Timeout error: {}", msg),
            P2PError::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl Error for P2PError {}

/// Network-related errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkError {
    /// Connection failed
    ConnectionFailed(String),
    /// Connection lost
    ConnectionLost(String),
    /// Message send failed
    SendFailed(String),
    /// Message receive failed
    ReceiveFailed(String),
    /// Invalid message format
    InvalidMessage(String),
    /// Session type violation
    SessionTypeViolation(String),
    /// Protocol error
    ProtocolError(String),
    /// Address parsing error
    AddressParse(String),
    /// Binding error
    BindError(String),
    /// Routing error
    RoutingError(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            NetworkError::ConnectionLost(msg) => write!(f, "Connection lost: {}", msg),
            NetworkError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            NetworkError::ReceiveFailed(msg) => write!(f, "Receive failed: {}", msg),
            NetworkError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            NetworkError::SessionTypeViolation(msg) => write!(f, "Session type violation: {}", msg),
            NetworkError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            NetworkError::AddressParse(msg) => write!(f, "Address parse error: {}", msg),
            NetworkError::BindError(msg) => write!(f, "Bind error: {}", msg),
            NetworkError::RoutingError(msg) => write!(f, "Routing error: {}", msg),
        }
    }
}

/// Consensus-related errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusError {
    /// Not the leader
    NotLeader,
    /// No leader elected
    NoLeader,
    /// Election failed
    ElectionFailed(String),
    /// Proposal failed
    ProposalFailed(String),
    /// Commit failed
    CommitFailed(String),
    /// Log inconsistency
    LogInconsistency(String),
    /// Byzantine behavior detected
    ByzantineBehavior(String),
    /// Insufficient replicas
    InsufficientReplicas,
    /// Consensus timeout
    ConsensusTimeout,
    /// Invalid term
    InvalidTerm,
    /// Invalid sequence number
    InvalidSequence,
}

impl fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusError::NotLeader => write!(f, "Not the leader"),
            ConsensusError::NoLeader => write!(f, "No leader elected"),
            ConsensusError::ElectionFailed(msg) => write!(f, "Election failed: {}", msg),
            ConsensusError::ProposalFailed(msg) => write!(f, "Proposal failed: {}", msg),
            ConsensusError::CommitFailed(msg) => write!(f, "Commit failed: {}", msg),
            ConsensusError::LogInconsistency(msg) => write!(f, "Log inconsistency: {}", msg),
            ConsensusError::ByzantineBehavior(msg) => write!(f, "Byzantine behavior: {}", msg),
            ConsensusError::InsufficientReplicas => write!(f, "Insufficient replicas"),
            ConsensusError::ConsensusTimeout => write!(f, "Consensus timeout"),
            ConsensusError::InvalidTerm => write!(f, "Invalid term"),
            ConsensusError::InvalidSequence => write!(f, "Invalid sequence number"),
        }
    }
}

/// Actor-related errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorError {
    /// Actor not found
    ActorNotFound(String),
    /// Actor spawn failed
    SpawnFailed(String),
    /// Actor message send failed
    MessageSendFailed(String),
    /// Actor state serialization failed
    StateSerialization(String),
    /// Actor state deserialization failed
    StateDeserialization(String),
    /// Invalid actor reference
    InvalidActorRef(String),
    /// Actor supervision failed
    SupervisionFailed(String),
    /// Actor restart failed
    RestartFailed(String),
}

impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActorError::ActorNotFound(msg) => write!(f, "Actor not found: {}", msg),
            ActorError::SpawnFailed(msg) => write!(f, "Spawn failed: {}", msg),
            ActorError::MessageSendFailed(msg) => write!(f, "Message send failed: {}", msg),
            ActorError::StateSerialization(msg) => write!(f, "State serialization failed: {}", msg),
            ActorError::StateDeserialization(msg) => write!(f, "State deserialization failed: {}", msg),
            ActorError::InvalidActorRef(msg) => write!(f, "Invalid actor reference: {}", msg),
            ActorError::SupervisionFailed(msg) => write!(f, "Supervision failed: {}", msg),
            ActorError::RestartFailed(msg) => write!(f, "Restart failed: {}", msg),
        }
    }
}

/// Cluster management errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterError {
    /// Failed to join cluster
    JoinFailed(String),
    /// Failed to leave cluster
    LeaveFailed(String),
    /// Node not found in cluster
    NodeNotFound(String),
    /// Cluster formation failed
    FormationFailed(String),
    /// Cluster split detected
    ClusterSplit,
    /// Insufficient nodes
    InsufficientNodes,
    /// Membership update failed
    MembershipUpdateFailed(String),
    /// Health check failed
    HealthCheckFailed(String),
}

impl fmt::Display for ClusterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterError::JoinFailed(msg) => write!(f, "Join failed: {}", msg),
            ClusterError::LeaveFailed(msg) => write!(f, "Leave failed: {}", msg),
            ClusterError::NodeNotFound(msg) => write!(f, "Node not found: {}", msg),
            ClusterError::FormationFailed(msg) => write!(f, "Formation failed: {}", msg),
            ClusterError::ClusterSplit => write!(f, "Cluster split detected"),
            ClusterError::InsufficientNodes => write!(f, "Insufficient nodes"),
            ClusterError::MembershipUpdateFailed(msg) => write!(f, "Membership update failed: {}", msg),
            ClusterError::HealthCheckFailed(msg) => write!(f, "Health check failed: {}", msg),
        }
    }
}

/// Discovery errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryError {
    /// DHT operation failed
    DHTFailed(String),
    /// Bootstrap failed
    BootstrapFailed(String),
    /// Gossip failed
    GossipFailed(String),
    /// Node lookup failed
    NodeLookupFailed(String),
    /// Routing table update failed
    RoutingTableUpdateFailed(String),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoveryError::DHTFailed(msg) => write!(f, "DHT failed: {}", msg),
            DiscoveryError::BootstrapFailed(msg) => write!(f, "Bootstrap failed: {}", msg),
            DiscoveryError::GossipFailed(msg) => write!(f, "Gossip failed: {}", msg),
            DiscoveryError::NodeLookupFailed(msg) => write!(f, "Node lookup failed: {}", msg),
            DiscoveryError::RoutingTableUpdateFailed(msg) => write!(f, "Routing table update failed: {}", msg),
        }
    }
}

/// Migration errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationError {
    /// Migration preparation failed
    PreparationFailed(String),
    /// Migration transfer failed
    TransferFailed(String),
    /// Migration completion failed
    CompletionFailed(String),
    /// Migration rollback failed
    RollbackFailed(String),
    /// Invalid migration state
    InvalidState(String),
    /// Migration timeout
    MigrationTimeout,
    /// Target node unavailable
    TargetUnavailable(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::PreparationFailed(msg) => write!(f, "Preparation failed: {}", msg),
            MigrationError::TransferFailed(msg) => write!(f, "Transfer failed: {}", msg),
            MigrationError::CompletionFailed(msg) => write!(f, "Completion failed: {}", msg),
            MigrationError::RollbackFailed(msg) => write!(f, "Rollback failed: {}", msg),
            MigrationError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            MigrationError::MigrationTimeout => write!(f, "Migration timeout"),
            MigrationError::TargetUnavailable(msg) => write!(f, "Target unavailable: {}", msg),
        }
    }
}

// Conversion implementations for common error types
impl From<io::Error> for P2PError {
    fn from(err: io::Error) -> Self {
        P2PError::Network(NetworkError::ConnectionFailed(err.to_string()))
    }
}

impl From<AddrParseError> for P2PError {
    fn from(err: AddrParseError) -> Self {
        P2PError::Network(NetworkError::AddressParse(err.to_string()))
    }
}

impl From<Elapsed> for P2PError {
    fn from(err: Elapsed) -> Self {
        P2PError::Timeout(err.to_string())
    }
}

impl From<serde_json::Error> for P2PError {
    fn from(err: serde_json::Error) -> Self {
        P2PError::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for P2PError {
    fn from(err: bincode::Error) -> Self {
        P2PError::Serialization(err.to_string())
    }
}
