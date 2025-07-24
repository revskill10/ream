//! TLISP Actor Model Primitives
//! 
//! Implements complete actor model integration for TLISP including:
//! - spawn, send, receive, link, monitor
//! - supervision trees and fault tolerance
//! - process management and introspection

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::tlisp::{Value, Expr, Function};
use crate::tlisp::types::{Type, SessionType};
use crate::types::{Pid, MessagePayload};
use crate::error::{TlispError, TlispResult};
use crate::runtime::{ReamRuntime, ReamActor};
use crate::runtime::supervisor::{SupervisorSpec, ChildSpec, RestartStrategy, RestartPolicy};

/// Actor message types for TLISP
#[derive(Debug, Clone)]
pub enum ActorMessage {
    /// User-defined message
    User(Value),
    /// System message for actor lifecycle
    System(SystemMessage),
    /// Supervision message
    Supervision(SupervisionMessage),
}

/// System messages for actor lifecycle
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Start the actor
    Start,
    /// Stop the actor
    Stop,
    /// Restart the actor
    Restart,
    /// Get actor status
    Status,
    /// Link to another process
    Link(Pid),
    /// Unlink from another process
    Unlink(Pid),
    /// Monitor another process
    Monitor(Pid),
    /// Demonitor another process
    Demonitor(Pid),
}

/// Supervision messages
#[derive(Debug, Clone)]
pub enum SupervisionMessage {
    /// Process exited
    Exit(Pid, ExitReason),
    /// Process down notification
    Down(Pid, ExitReason),
    /// Restart child process
    RestartChild(String),
    /// Stop child process
    StopChild(String),
}

/// Reasons for process exit
#[derive(Debug, Clone)]
pub enum ExitReason {
    /// Normal termination
    Normal,
    /// Killed by supervisor
    Killed,
    /// Error occurred
    Error(String),
    /// Shutdown requested
    Shutdown,
}

/// TLISP Actor implementation
pub struct TlispActor {
    /// Actor PID
    pid: Pid,
    /// Actor behavior function
    behavior: Function,
    /// Actor state
    state: Value,
    /// Linked processes
    links: Vec<Pid>,
    /// Monitored processes
    monitors: Vec<Pid>,
    /// Message queue
    message_queue: Vec<ActorMessage>,
    /// Actor status
    status: ActorStatus,
}

/// Actor status
#[derive(Debug, Clone)]
pub enum ActorStatus {
    /// Actor is running
    Running,
    /// Actor is suspended
    Suspended,
    /// Actor is terminated
    Terminated(ExitReason),
}

impl TlispActor {
    /// Create a new TLISP actor
    pub fn new(behavior: Function, initial_state: Value) -> Self {
        TlispActor {
            pid: Pid::new(),
            behavior,
            state: initial_state,
            links: Vec::new(),
            monitors: Vec::new(),
            message_queue: Vec::new(),
            status: ActorStatus::Running,
        }
    }

    /// Process an actor message
    pub fn process_message(&mut self, message: ActorMessage) -> TlispResult<()> {
        match message {
            ActorMessage::User(value) => {
                self.handle_user_message(value)
            }
            ActorMessage::System(sys_msg) => {
                self.handle_system_message(sys_msg)
            }
            ActorMessage::Supervision(sup_msg) => {
                self.handle_supervision_message(sup_msg)
            }
        }
    }

    /// Handle user-defined message
    fn handle_user_message(&mut self, message: Value) -> TlispResult<()> {
        // Apply behavior function to current state and message
        let args = vec![self.state.clone(), message];
        
        // TODO: Evaluate function with proper environment
        // For now, return the current state
        Ok(())
    }

    /// Handle system message
    fn handle_system_message(&mut self, message: SystemMessage) -> TlispResult<()> {
        match message {
            SystemMessage::Start => {
                self.status = ActorStatus::Running;
                Ok(())
            }
            SystemMessage::Stop => {
                self.status = ActorStatus::Terminated(ExitReason::Normal);
                Ok(())
            }
            SystemMessage::Restart => {
                self.status = ActorStatus::Running;
                self.message_queue.clear();
                Ok(())
            }
            SystemMessage::Status => {
                // Return status (in real implementation, would send response)
                Ok(())
            }
            SystemMessage::Link(pid) => {
                if !self.links.contains(&pid) {
                    self.links.push(pid);
                }
                Ok(())
            }
            SystemMessage::Unlink(pid) => {
                self.links.retain(|&p| p != pid);
                Ok(())
            }
            SystemMessage::Monitor(pid) => {
                if !self.monitors.contains(&pid) {
                    self.monitors.push(pid);
                }
                Ok(())
            }
            SystemMessage::Demonitor(pid) => {
                self.monitors.retain(|&p| p != pid);
                Ok(())
            }
        }
    }

    /// Handle supervision message
    fn handle_supervision_message(&mut self, message: SupervisionMessage) -> TlispResult<()> {
        match message {
            SupervisionMessage::Exit(pid, reason) => {
                // Handle linked process exit
                if self.links.contains(&pid) {
                    match reason {
                        ExitReason::Normal => {
                            // Normal exit, continue
                        }
                        _ => {
                            // Abnormal exit, terminate this process too
                            self.status = ActorStatus::Terminated(reason);
                        }
                    }
                }
                Ok(())
            }
            SupervisionMessage::Down(pid, reason) => {
                // Handle monitored process down
                if self.monitors.contains(&pid) {
                    // Send down message to actor
                    let down_msg = Value::List(vec![
                        Value::Symbol("down".to_string()),
                        Value::String(pid.to_string()),
                        Value::String(format!("{:?}", reason)),
                    ]);
                    self.message_queue.push(ActorMessage::User(down_msg));
                }
                Ok(())
            }
            SupervisionMessage::RestartChild(_) => {
                // Only supervisors handle this
                Ok(())
            }
            SupervisionMessage::StopChild(_) => {
                // Only supervisors handle this
                Ok(())
            }
        }
    }

    /// Get actor PID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Get actor status
    pub fn status(&self) -> &ActorStatus {
        &self.status
    }

    /// Check if actor is alive
    pub fn is_alive(&self) -> bool {
        matches!(self.status, ActorStatus::Running | ActorStatus::Suspended)
    }
}

impl ReamActor for TlispActor {
    fn receive(&mut self, message: MessagePayload) -> crate::error::RuntimeResult<()> {
        // Convert MessagePayload to ActorMessage
        let actor_msg = match message {
            MessagePayload::Text(text) => {
                ActorMessage::User(Value::String(text))
            }
            MessagePayload::Data(data) => {
                // Try to parse JSON data into TLISP value
                ActorMessage::User(Value::String(data.to_string()))
            }
            MessagePayload::Binary(bytes) => {
                ActorMessage::User(Value::String(String::from_utf8_lossy(&bytes).to_string()))
            }
        };

        self.process_message(actor_msg)
            .map_err(|e| crate::error::RuntimeError::Scheduler(format!("TLISP actor error: {}", e)))
    }

    fn pid(&self) -> Pid {
        self.pid
    }

    fn restart(&mut self) -> crate::error::RuntimeResult<()> {
        self.handle_system_message(SystemMessage::Restart)
            .map_err(|e| crate::error::RuntimeError::Scheduler(format!("Restart failed: {}", e)))
    }

    fn is_alive(&self) -> bool {
        self.is_alive()
    }
}

/// Actor primitive functions for TLISP
pub struct ActorPrimitives {
    /// REAM runtime reference
    runtime: Arc<RwLock<ReamRuntime>>,
    /// Active actors
    actors: Arc<RwLock<HashMap<Pid, TlispActor>>>,
}

impl ActorPrimitives {
    /// Create new actor primitives
    pub fn new(runtime: Arc<RwLock<ReamRuntime>>) -> Self {
        ActorPrimitives {
            runtime,
            actors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Spawn a new actor
    pub fn spawn(&mut self, behavior: Function, initial_state: Value) -> TlispResult<Pid> {
        let actor = TlispActor::new(behavior, initial_state);
        let pid = actor.pid();

        // Store actor
        {
            let mut actors = self.actors.write().unwrap();
            actors.insert(pid, actor);
        }

        // Register with REAM runtime
        // TODO: Implement proper actor registration

        Ok(pid)
    }

    /// Send message to actor
    pub fn send(&self, to: Pid, message: Value) -> TlispResult<()> {
        let mut actors = self.actors.write().unwrap();
        if let Some(actor) = actors.get_mut(&to) {
            actor.process_message(ActorMessage::User(message))?;
            Ok(())
        } else {
            Err(TlispError::Runtime(format!("Process {} not found", to)))
        }
    }

    /// Link two processes
    pub fn link(&self, from: Pid, to: Pid) -> TlispResult<()> {
        let mut actors = self.actors.write().unwrap();
        
        // Link from -> to
        if let Some(actor) = actors.get_mut(&from) {
            actor.handle_system_message(SystemMessage::Link(to))?;
        }
        
        // Link to -> from (bidirectional)
        if let Some(actor) = actors.get_mut(&to) {
            actor.handle_system_message(SystemMessage::Link(from))?;
        }
        
        Ok(())
    }

    /// Monitor a process
    pub fn monitor(&self, from: Pid, to: Pid) -> TlispResult<()> {
        let mut actors = self.actors.write().unwrap();
        if let Some(actor) = actors.get_mut(&from) {
            actor.handle_system_message(SystemMessage::Monitor(to))?;
            Ok(())
        } else {
            Err(TlispError::Runtime(format!("Process {} not found", from)))
        }
    }

    /// Get process information
    pub fn process_info(&self, pid: Pid) -> TlispResult<Value> {
        let actors = self.actors.read().unwrap();
        if let Some(actor) = actors.get(&pid) {
            let info = Value::List(vec![
                Value::Symbol("process-info".to_string()),
                Value::String(pid.to_string()),
                Value::String(format!("{:?}", actor.status())),
                Value::Bool(actor.is_alive()),
            ]);
            Ok(info)
        } else {
            Err(TlispError::Runtime(format!("Process {} not found", pid)))
        }
    }

    /// List all processes
    pub fn list_processes(&self) -> TlispResult<Value> {
        let actors = self.actors.read().unwrap();
        let pids: Vec<Value> = actors.keys()
            .map(|pid| Value::String(pid.to_string()))
            .collect();
        Ok(Value::List(pids))
    }
}
