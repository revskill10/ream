//! Session types for type-safe network protocols
//!
//! Implements session types to ensure protocol correctness and prevent
//! communication errors at compile time.

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::fmt;
use crate::p2p::{P2PResult, P2PError, NetworkError};

/// Session type for protocol specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    /// End of session
    End,
    /// Send a message of type T, then continue with session S
    Send { message_type: String, next: Box<SessionType> },
    /// Receive a message of type T, then continue with session S
    Receive { message_type: String, next: Box<SessionType> },
    /// Choice between multiple branches
    Choice { branches: Vec<(String, SessionType)> },
    /// Offer multiple branches to the other party
    Offer { branches: Vec<(String, SessionType)> },
    /// Recursive session type
    Recursive { name: String, body: Box<SessionType> },
    /// Variable reference for recursion
    Variable { name: String },
}

impl SessionType {
    /// Create a simple send session
    pub fn send(message_type: &str, next: SessionType) -> Self {
        SessionType::Send {
            message_type: message_type.to_string(),
            next: Box::new(next),
        }
    }

    /// Create a simple receive session
    pub fn receive(message_type: &str, next: SessionType) -> Self {
        SessionType::Receive {
            message_type: message_type.to_string(),
            next: Box::new(next),
        }
    }

    /// Create a choice session
    pub fn choice(branches: Vec<(&str, SessionType)>) -> Self {
        SessionType::Choice {
            branches: branches.into_iter()
                .map(|(name, session)| (name.to_string(), session))
                .collect(),
        }
    }

    /// Create an offer session
    pub fn offer(branches: Vec<(&str, SessionType)>) -> Self {
        SessionType::Offer {
            branches: branches.into_iter()
                .map(|(name, session)| (name.to_string(), session))
                .collect(),
        }
    }

    /// Create a recursive session
    pub fn recursive(name: &str, body: SessionType) -> Self {
        SessionType::Recursive {
            name: name.to_string(),
            body: Box::new(body),
        }
    }

    /// Create a variable reference
    pub fn variable(name: &str) -> Self {
        SessionType::Variable {
            name: name.to_string(),
        }
    }

    /// Check if session is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, SessionType::End)
    }

    /// Get the dual session type (for the other party)
    pub fn dual(&self) -> SessionType {
        match self {
            SessionType::End => SessionType::End,
            SessionType::Send { message_type, next } => {
                SessionType::Receive {
                    message_type: message_type.clone(),
                    next: Box::new(next.dual()),
                }
            }
            SessionType::Receive { message_type, next } => {
                SessionType::Send {
                    message_type: message_type.clone(),
                    next: Box::new(next.dual()),
                }
            }
            SessionType::Choice { branches } => {
                SessionType::Offer {
                    branches: branches.iter()
                        .map(|(name, session)| (name.clone(), session.dual()))
                        .collect(),
                }
            }
            SessionType::Offer { branches } => {
                SessionType::Choice {
                    branches: branches.iter()
                        .map(|(name, session)| (name.clone(), session.dual()))
                        .collect(),
                }
            }
            SessionType::Recursive { name, body } => {
                SessionType::Recursive {
                    name: name.clone(),
                    body: Box::new(body.dual()),
                }
            }
            SessionType::Variable { name } => {
                SessionType::Variable {
                    name: name.clone(),
                }
            }
        }
    }

    /// Validate that a message conforms to the current session state
    pub fn validate_send(&self, message_type: &str) -> P2PResult<SessionType> {
        match self {
            SessionType::Send { message_type: expected, next } => {
                if message_type == expected {
                    Ok((**next).clone())
                } else {
                    Err(P2PError::Network(NetworkError::SessionTypeViolation(
                        format!("Expected to send {}, but tried to send {}", expected, message_type)
                    )))
                }
            }
            _ => Err(P2PError::Network(NetworkError::SessionTypeViolation(
                format!("Cannot send {} in current session state", message_type)
            )))
        }
    }

    /// Validate that a received message conforms to the current session state
    pub fn validate_receive(&self, message_type: &str) -> P2PResult<SessionType> {
        match self {
            SessionType::Receive { message_type: expected, next } => {
                if message_type == expected {
                    Ok((**next).clone())
                } else {
                    Err(P2PError::Network(NetworkError::SessionTypeViolation(
                        format!("Expected to receive {}, but received {}", expected, message_type)
                    )))
                }
            }
            _ => Err(P2PError::Network(NetworkError::SessionTypeViolation(
                format!("Cannot receive {} in current session state", message_type)
            )))
        }
    }

    /// Select a branch in a choice
    pub fn select_branch(&self, branch_name: &str) -> P2PResult<SessionType> {
        match self {
            SessionType::Choice { branches } => {
                for (name, session) in branches {
                    if name == branch_name {
                        return Ok(session.clone());
                    }
                }
                Err(P2PError::Network(NetworkError::SessionTypeViolation(
                    format!("Branch {} not found in choice", branch_name)
                )))
            }
            _ => Err(P2PError::Network(NetworkError::SessionTypeViolation(
                "Cannot select branch in non-choice session".to_string()
            )))
        }
    }

    /// Offer a branch
    pub fn offer_branch(&self, branch_name: &str) -> P2PResult<SessionType> {
        match self {
            SessionType::Offer { branches } => {
                for (name, session) in branches {
                    if name == branch_name {
                        return Ok(session.clone());
                    }
                }
                Err(P2PError::Network(NetworkError::SessionTypeViolation(
                    format!("Branch {} not found in offer", branch_name)
                )))
            }
            _ => Err(P2PError::Network(NetworkError::SessionTypeViolation(
                "Cannot offer branch in non-offer session".to_string()
            )))
        }
    }
}

impl fmt::Display for SessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionType::End => write!(f, "end"),
            SessionType::Send { message_type, next } => {
                write!(f, "!{}.{}", message_type, next)
            }
            SessionType::Receive { message_type, next } => {
                write!(f, "?{}.{}", message_type, next)
            }
            SessionType::Choice { branches } => {
                write!(f, "+")?;
                for (i, (name, session)) in branches.iter().enumerate() {
                    if i > 0 { write!(f, " | ")?; }
                    write!(f, "{}:{}", name, session)?;
                }
                Ok(())
            }
            SessionType::Offer { branches } => {
                write!(f, "&")?;
                for (i, (name, session)) in branches.iter().enumerate() {
                    if i > 0 { write!(f, " | ")?; }
                    write!(f, "{}:{}", name, session)?;
                }
                Ok(())
            }
            SessionType::Recursive { name, body } => {
                write!(f, "μ{}.{}", name, body)
            }
            SessionType::Variable { name } => {
                write!(f, "{}", name)
            }
        }
    }
}

/// Session channel for type-safe communication
#[derive(Debug)]
pub struct SessionChannel<S> {
    /// Current session type
    session_type: SessionType,
    /// Phantom data for compile-time session tracking
    _phantom: PhantomData<S>,
}

impl<S> SessionChannel<S> {
    /// Create a new session channel
    pub fn new(session_type: SessionType) -> Self {
        Self {
            session_type,
            _phantom: PhantomData,
        }
    }

    /// Get current session type
    pub fn session_type(&self) -> &SessionType {
        &self.session_type
    }

    /// Check if session is complete
    pub fn is_complete(&self) -> bool {
        self.session_type.is_complete()
    }

    /// Transition to next session state after sending
    pub fn after_send(&mut self, message_type: &str) -> P2PResult<()> {
        let next_session = self.session_type.validate_send(message_type)?;
        self.session_type = next_session;
        Ok(())
    }

    /// Transition to next session state after receiving
    pub fn after_receive(&mut self, message_type: &str) -> P2PResult<()> {
        let next_session = self.session_type.validate_receive(message_type)?;
        self.session_type = next_session;
        Ok(())
    }

    /// Select a branch in choice
    pub fn select_branch(&mut self, branch_name: &str) -> P2PResult<()> {
        let next_session = self.session_type.select_branch(branch_name)?;
        self.session_type = next_session;
        Ok(())
    }

    /// Offer a branch
    pub fn offer_branch(&mut self, branch_name: &str) -> P2PResult<()> {
        let next_session = self.session_type.offer_branch(branch_name)?;
        self.session_type = next_session;
        Ok(())
    }
}

/// Predefined session types for common protocols

/// Node discovery protocol
pub fn node_discovery_session() -> SessionType {
    SessionType::send("NodeDiscoveryRequest",
        SessionType::receive("NodeDiscoveryResponse",
            SessionType::choice(vec![
                ("join_cluster", SessionType::send("JoinClusterRequest",
                    SessionType::receive("JoinClusterResponse",
                        SessionType::End))),
                ("query_nodes", SessionType::send("QueryNodesRequest",
                    SessionType::receive("QueryNodesResponse",
                        SessionType::End))),
            ])
        )
    )
}

/// Actor migration protocol
pub fn actor_migration_session() -> SessionType {
    SessionType::send("MigrationRequest",
        SessionType::receive("MigrationAck",
            SessionType::send("ActorState",
                SessionType::receive("StateAck",
                    SessionType::send("MigrationComplete",
                        SessionType::End)))))
}

/// Consensus protocol
pub fn consensus_session() -> SessionType {
    SessionType::choice(vec![
        ("propose", SessionType::send("ProposalMessage",
            SessionType::receive("VoteMessage",
                SessionType::End))),
        ("vote", SessionType::receive("ProposalMessage",
            SessionType::send("VoteMessage",
                SessionType::End))),
    ])
}

/// Heartbeat protocol
pub fn heartbeat_session() -> SessionType {
    SessionType::recursive("heartbeat_loop",
        SessionType::choice(vec![
            ("continue", SessionType::send("Heartbeat",
                SessionType::receive("HeartbeatAck",
                    SessionType::variable("heartbeat_loop")))),
            ("stop", SessionType::End),
        ])
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_type_creation() {
        let session = SessionType::send("Hello",
            SessionType::receive("World",
                SessionType::End));
        
        assert!(!session.is_complete());
    }

    #[test]
    fn test_session_type_dual() {
        let session = SessionType::send("Hello",
            SessionType::receive("World",
                SessionType::End));
        
        let dual = session.dual();
        
        match dual {
            SessionType::Receive { message_type, .. } => {
                assert_eq!(message_type, "Hello");
            }
            _ => panic!("Expected receive session"),
        }
    }

    #[test]
    fn test_session_validation() {
        let session = SessionType::send("Hello", SessionType::End);
        let result = session.validate_send("Hello");
        assert!(result.is_ok());
        
        let result = session.validate_send("World");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_channel() {
        let session = SessionType::send("Hello", SessionType::End);
        let mut channel = SessionChannel::<()>::new(session);
        
        assert!(!channel.is_complete());
        assert!(channel.after_send("Hello").is_ok());
        assert!(channel.is_complete());
    }

    #[test]
    fn test_predefined_sessions() {
        let discovery = node_discovery_session();
        assert!(!discovery.is_complete());
        
        let migration = actor_migration_session();
        assert!(!migration.is_complete());
        
        let consensus = consensus_session();
        assert!(!consensus.is_complete());
    }
}
