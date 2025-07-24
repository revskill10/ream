//! Session Types for TLISP
//! 
//! Implements session types for protocol verification in actor communication.
//! Session types ensure that communication protocols are followed correctly
//! at compile time, preventing protocol violations and deadlocks.

use std::collections::HashMap;
use crate::tlisp::{Value, Expr};
use crate::tlisp::types::{Type, SessionType};
use crate::types::Pid;
use crate::error::{TlispError, TlispResult, TypeError};

/// Session type checker
pub struct SessionTypeChecker {
    /// Active sessions
    sessions: HashMap<Pid, SessionState>,
    /// Session type definitions
    session_types: HashMap<String, SessionType>,
}

/// State of an active session
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Current session type
    current_type: SessionType,
    /// Session history for debugging
    history: Vec<SessionAction>,
    /// Whether session is complete
    complete: bool,
}

/// Actions performed in a session
#[derive(Debug, Clone)]
pub enum SessionAction {
    /// Sent a message of given type
    Send(Type),
    /// Received a message of given type
    Receive(Type),
    /// Made a choice in a choice session
    Choose(String),
    /// Offered a choice in an offer session
    Offer(String),
}

/// Session type operations
impl SessionTypeChecker {
    /// Create a new session type checker
    pub fn new() -> Self {
        SessionTypeChecker {
            sessions: HashMap::new(),
            session_types: HashMap::new(),
        }
    }

    /// Define a new session type
    pub fn define_session_type(&mut self, name: String, session_type: SessionType) {
        self.session_types.insert(name, session_type);
    }

    /// Start a new session
    pub fn start_session(&mut self, pid: Pid, session_type: SessionType) -> TlispResult<()> {
        let state = SessionState {
            current_type: session_type,
            history: Vec::new(),
            complete: false,
        };
        self.sessions.insert(pid, state);
        Ok(())
    }

    /// Check if a send operation is valid
    pub fn check_send(&mut self, pid: Pid, message_type: &Type) -> TlispResult<()> {
        let session = self.sessions.get_mut(&pid)
            .ok_or_else(|| TlispError::Type(TypeError::SessionNotFound(pid)))?;

        match &session.current_type {
            SessionType::Send(expected_type, continuation) => {
                // Check if message type matches expected type
                if self.types_compatible(message_type, expected_type)? {
                    // Update session state
                    session.current_type = (**continuation).clone();
                    session.history.push(SessionAction::Send(message_type.clone()));
                    
                    // Check if session is complete
                    if matches!(session.current_type, SessionType::End) {
                        session.complete = true;
                    }
                    
                    Ok(())
                } else {
                    Err(TlispError::Type(TypeError::SessionTypeMismatch {
                        expected: (**expected_type).clone(),
                        actual: message_type.clone(),
                    }))
                }
            }
            SessionType::Receive(_, _) => {
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Expected receive, got send".to_string()
                )))
            }
            SessionType::Choose(choices) => {
                // For choice, we need to know which choice was made
                // This would be handled by a separate choose operation
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Must make choice before sending".to_string()
                )))
            }
            SessionType::Offer(choices) => {
                // For offer, we need to wait for the other party to choose
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Cannot send during offer, must wait for choice".to_string()
                )))
            }
            SessionType::End => {
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Session already ended".to_string()
                )))
            }
            SessionType::Recursive(_, _) => {
                // Handle recursive sessions
                // TODO: Implement proper recursive session handling
                Ok(())
            }
        }
    }

    /// Check if a receive operation is valid
    pub fn check_receive(&mut self, pid: Pid, message_type: &Type) -> TlispResult<()> {
        let session = self.sessions.get_mut(&pid)
            .ok_or_else(|| TlispError::Type(TypeError::SessionNotFound(pid)))?;

        match &session.current_type {
            SessionType::Receive(expected_type, continuation) => {
                // Check if message type matches expected type
                if self.types_compatible(message_type, expected_type)? {
                    // Update session state
                    session.current_type = (**continuation).clone();
                    session.history.push(SessionAction::Receive(message_type.clone()));
                    
                    // Check if session is complete
                    if matches!(session.current_type, SessionType::End) {
                        session.complete = true;
                    }
                    
                    Ok(())
                } else {
                    Err(TlispError::Type(TypeError::SessionTypeMismatch {
                        expected: (**expected_type).clone(),
                        actual: message_type.clone(),
                    }))
                }
            }
            SessionType::Send(_, _) => {
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Expected send, got receive".to_string()
                )))
            }
            SessionType::Choose(_) => {
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Must make choice before receiving".to_string()
                )))
            }
            SessionType::Offer(choices) => {
                // For offer, we need to determine which choice was made
                // This would typically be done by examining the message content
                // For now, just accept any valid choice
                for (choice_name, choice_type) in choices {
                    if let SessionType::Receive(recv_type, continuation) = choice_type {
                        if self.types_compatible(message_type, recv_type)? {
                            session.current_type = (**continuation).clone();
                            session.history.push(SessionAction::Offer(choice_name.clone()));
                            session.history.push(SessionAction::Receive(message_type.clone()));
                            return Ok(());
                        }
                    }
                }
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "No matching choice in offer".to_string()
                )))
            }
            SessionType::End => {
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Session already ended".to_string()
                )))
            }
            SessionType::Recursive(_, _) => {
                // Handle recursive sessions
                // TODO: Implement proper recursive session handling
                Ok(())
            }
        }
    }

    /// Make a choice in a choice session
    pub fn make_choice(&mut self, pid: Pid, choice: &str) -> TlispResult<()> {
        let session = self.sessions.get_mut(&pid)
            .ok_or_else(|| TlispError::Type(TypeError::SessionNotFound(pid)))?;

        match &session.current_type {
            SessionType::Choose(choices) => {
                if let Some(choice_type) = choices.get(choice) {
                    session.current_type = choice_type.clone();
                    session.history.push(SessionAction::Choose(choice.to_string()));
                    Ok(())
                } else {
                    Err(TlispError::Type(TypeError::SessionProtocolViolation(
                        format!("Invalid choice: {}", choice)
                    )))
                }
            }
            _ => {
                Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Not in a choice session".to_string()
                )))
            }
        }
    }

    /// Check if session is complete
    pub fn is_session_complete(&self, pid: Pid) -> bool {
        self.sessions.get(&pid)
            .map(|session| session.complete)
            .unwrap_or(false)
    }

    /// Get current session type
    pub fn get_current_session_type(&self, pid: Pid) -> Option<&SessionType> {
        self.sessions.get(&pid).map(|session| &session.current_type)
    }

    /// End a session
    pub fn end_session(&mut self, pid: Pid) -> TlispResult<()> {
        if let Some(session) = self.sessions.get(&pid) {
            if !session.complete {
                return Err(TlispError::Type(TypeError::SessionProtocolViolation(
                    "Session not complete".to_string()
                )));
            }
        }
        self.sessions.remove(&pid);
        Ok(())
    }

    /// Check if two types are compatible
    fn types_compatible(&self, actual: &Type, expected: &Type) -> TlispResult<bool> {
        // Simple type compatibility check
        // In a full implementation, this would handle subtyping, etc.
        Ok(actual == expected)
    }

    /// Get session history for debugging
    pub fn get_session_history(&self, pid: Pid) -> Option<&Vec<SessionAction>> {
        self.sessions.get(&pid).map(|session| &session.history)
    }
}

/// Session type builder for creating complex session types
pub struct SessionTypeBuilder {
    current: SessionType,
}

impl SessionTypeBuilder {
    /// Create a new session type builder
    pub fn new() -> Self {
        SessionTypeBuilder {
            current: SessionType::End,
        }
    }

    /// Add a send operation
    pub fn send(mut self, message_type: Type) -> Self {
        self.current = SessionType::Send(
            Box::new(message_type),
            Box::new(self.current),
        );
        self
    }

    /// Add a receive operation
    pub fn receive(mut self, message_type: Type) -> Self {
        self.current = SessionType::Receive(
            Box::new(message_type),
            Box::new(self.current),
        );
        self
    }

    /// Add a choice
    pub fn choose(mut self, choices: HashMap<String, SessionType>) -> Self {
        self.current = SessionType::Choose(choices);
        self
    }

    /// Add an offer
    pub fn offer(mut self, choices: HashMap<String, SessionType>) -> Self {
        self.current = SessionType::Offer(choices);
        self
    }

    /// Make the session recursive
    pub fn recursive(mut self, var: String) -> Self {
        self.current = SessionType::Recursive(var, Box::new(self.current));
        self
    }

    /// Build the session type
    pub fn build(self) -> SessionType {
        self.current
    }
}

/// Session type utilities
pub struct SessionTypeUtils;

impl SessionTypeUtils {
    /// Create a simple request-response session type
    pub fn request_response(request_type: Type, response_type: Type) -> SessionType {
        SessionTypeBuilder::new()
            .send(request_type)
            .receive(response_type)
            .build()
    }

    /// Create a bidirectional communication session type
    pub fn bidirectional(type1: Type, type2: Type) -> SessionType {
        SessionTypeBuilder::new()
            .send(type1.clone())
            .receive(type2.clone())
            .send(type2)
            .receive(type1)
            .build()
    }

    /// Create a streaming session type
    pub fn streaming(item_type: Type) -> SessionType {
        let mut choices = HashMap::new();
        choices.insert("data".to_string(), 
            SessionType::Receive(
                Box::new(item_type),
                Box::new(SessionType::Recursive("stream".to_string(), Box::new(SessionType::End)))
            )
        );
        choices.insert("end".to_string(), SessionType::End);
        
        SessionType::Recursive("stream".to_string(), Box::new(SessionType::Offer(choices)))
    }
}

impl Default for SessionTypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SessionTypeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
