//! Actor system implementation with coalgebraic state machines

use std::any::Any;
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use crate::types::{Pid, MessagePayload};
use crate::error::{RuntimeError, RuntimeResult};

/// Message pattern for selective receive
#[derive(Debug, Clone)]
pub enum MessagePattern {
    /// Match any message
    Any,
    /// Match specific text content
    Text(String),
    /// Match message type
    Type(MessageType),
    /// Match with custom predicate
    Custom(fn(&MessagePayload) -> bool),
}

/// Message types for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Text,
    Data,
    Bytes,
    Control,
}

/// Actor link information
#[derive(Debug, Clone)]
pub struct ActorLink {
    /// Linked actor PID
    pub linked_pid: Pid,
    /// Link type (bidirectional or monitor)
    pub link_type: LinkType,
    /// Link creation time
    pub created_at: Instant,
}

/// Type of actor link
#[derive(Debug, Clone, PartialEq)]
pub enum LinkType {
    /// Bidirectional link - both actors fail if one fails
    Bidirectional,
    /// Monitor link - only monitor receives DOWN message
    Monitor,
}

/// Actor monitor information
#[derive(Debug, Clone)]
pub struct ActorMonitor {
    /// Monitor reference
    pub monitor_ref: MonitorRef,
    /// Monitored actor PID
    pub monitored_pid: Pid,
    /// Monitor creation time
    pub created_at: Instant,
}

/// Monitor reference for tracking monitors
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonitorRef(pub u64);

impl MonitorRef {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        MonitorRef(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// System messages for actor lifecycle
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Actor termination notification
    Down { pid: Pid, reason: String },
    /// Link request from another actor
    Link { from: Pid, link_type: LinkType },
    /// Unlink request
    Unlink { from: Pid },
    /// Monitor request
    Monitor { from: Pid, monitor_ref: MonitorRef },
    /// Demonitor request
    Demonitor { monitor_ref: MonitorRef },
}

/// Actor context for macro compatibility and enhanced functionality
pub struct ActorContext {
    /// Actor PID
    pub pid: Pid,
    /// Actor links
    pub links: Arc<RwLock<HashMap<Pid, ActorLink>>>,
    /// Actor monitors
    pub monitors: Arc<RwLock<HashMap<MonitorRef, ActorMonitor>>>,
    /// Mailbox for selective receive
    pub mailbox: Arc<Mutex<VecDeque<MessagePayload>>>,
}

impl ActorContext {
    /// Create a new actor context
    pub fn new(pid: Pid) -> Self {
        Self {
            pid,
            links: Arc::new(RwLock::new(HashMap::new())),
            monitors: Arc::new(RwLock::new(HashMap::new())),
            mailbox: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Get the current actor context (placeholder)
    pub fn current() -> Self {
        Self::new(Pid::new())
    }

    /// Receive a message (placeholder)
    pub async fn receive(&self) -> RuntimeResult<MessagePayload> {
        // Placeholder implementation
        Ok(MessagePayload::Text("placeholder".to_string()))
    }

    /// Selective receive with pattern matching
    pub fn selective_receive(&self, pattern: MessagePattern, timeout: Option<Duration>) -> RuntimeResult<Option<MessagePayload>> {
        let start_time = Instant::now();

        loop {
            // Check timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() > timeout_duration {
                    return Ok(None);
                }
            }

            // Try to find a matching message
            {
                let mut mailbox = self.mailbox.lock().unwrap();
                for (index, message) in mailbox.iter().enumerate() {
                    if self.matches_pattern(message, &pattern) {
                        let matched_message = mailbox.remove(index).unwrap();
                        return Ok(Some(matched_message));
                    }
                }
            }

            // Sleep briefly before checking again
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// Check if a message matches a pattern
    pub fn matches_pattern(&self, message: &MessagePayload, pattern: &MessagePattern) -> bool {
        match pattern {
            MessagePattern::Any => true,
            MessagePattern::Text(expected) => {
                if let MessagePayload::Text(text) = message {
                    text == expected
                } else {
                    false
                }
            }
            MessagePattern::Type(msg_type) => {
                let message_type = match message {
                    MessagePayload::Text(_) => MessageType::Text,
                    MessagePayload::Data(_) => MessageType::Data,
                    MessagePayload::Bytes(_) => MessageType::Bytes,
                    MessagePayload::Control(_) => MessageType::Control,
                };
                &message_type == msg_type
            }
            MessagePattern::Custom(predicate) => predicate(message),
        }
    }

    /// Link to another actor
    pub fn link(&self, target_pid: Pid, link_type: LinkType) -> RuntimeResult<()> {
        let link = ActorLink {
            linked_pid: target_pid,
            link_type,
            created_at: Instant::now(),
        };

        let mut links = self.links.write().unwrap();
        links.insert(target_pid, link);

        // TODO: Send link message to target actor
        Ok(())
    }

    /// Unlink from another actor
    pub fn unlink(&self, target_pid: Pid) -> RuntimeResult<()> {
        let mut links = self.links.write().unwrap();
        links.remove(&target_pid);

        // TODO: Send unlink message to target actor
        Ok(())
    }

    /// Monitor another actor
    pub fn monitor(&self, target_pid: Pid) -> RuntimeResult<MonitorRef> {
        let monitor_ref = MonitorRef::new();
        let monitor = ActorMonitor {
            monitor_ref: monitor_ref.clone(),
            monitored_pid: target_pid,
            created_at: Instant::now(),
        };

        let mut monitors = self.monitors.write().unwrap();
        monitors.insert(monitor_ref.clone(), monitor);

        // TODO: Send monitor message to target actor
        Ok(monitor_ref)
    }

    /// Stop monitoring another actor
    pub fn demonitor(&self, monitor_ref: MonitorRef) -> RuntimeResult<()> {
        let mut monitors = self.monitors.write().unwrap();
        monitors.remove(&monitor_ref);

        // TODO: Send demonitor message to target actor
        Ok(())
    }

    /// Get all linked actors
    pub fn get_links(&self) -> Vec<ActorLink> {
        let links = self.links.read().unwrap();
        links.values().cloned().collect()
    }

    /// Get all monitored actors
    pub fn get_monitors(&self) -> Vec<ActorMonitor> {
        let monitors = self.monitors.read().unwrap();
        monitors.values().cloned().collect()
    }
}

/// Trait for REAM actors - coalgebraic state machines with message processing
pub trait ReamActor: Send + Sync {
    /// Process a message and potentially update state
    fn receive(&mut self, message: MessagePayload) -> RuntimeResult<()>;

    /// Get the actor's PID
    fn pid(&self) -> Pid;

    /// Restart the actor (called by supervisor)
    fn restart(&mut self) -> RuntimeResult<()>;

    /// Check if actor is alive
    fn is_alive(&self) -> bool {
        true
    }

    /// Get actor state for debugging
    fn debug_state(&self) -> Box<dyn Any + Send> {
        Box::new(())
    }

    /// Handle system messages (linking, monitoring, etc.)
    fn handle_system_message(&mut self, message: SystemMessage) -> RuntimeResult<()> {
        match message {
            SystemMessage::Down { pid, reason } => {
                // Default implementation: log the down message
                println!("Actor {} received DOWN message for {}: {}", self.pid(), pid, reason);
                Ok(())
            }
            SystemMessage::Link { from, link_type } => {
                // Default implementation: accept the link
                println!("Actor {} received link request from {} ({:?})", self.pid(), from, link_type);
                Ok(())
            }
            SystemMessage::Unlink { from } => {
                // Default implementation: accept the unlink
                println!("Actor {} received unlink request from {}", self.pid(), from);
                Ok(())
            }
            SystemMessage::Monitor { from, monitor_ref } => {
                // Default implementation: accept the monitor
                println!("Actor {} received monitor request from {} (ref: {:?})", self.pid(), from, monitor_ref);
                Ok(())
            }
            SystemMessage::Demonitor { monitor_ref } => {
                // Default implementation: accept the demonitor
                println!("Actor {} received demonitor request (ref: {:?})", self.pid(), monitor_ref);
                Ok(())
            }
        }
    }

    /// Get actor context for advanced operations
    fn get_context(&self) -> Option<&ActorContext> {
        None
    }

    /// Selective receive with pattern matching
    fn selective_receive(&self, pattern: MessagePattern, timeout: Option<Duration>) -> RuntimeResult<Option<MessagePayload>> {
        if let Some(context) = self.get_context() {
            context.selective_receive(pattern, timeout)
        } else {
            Err(RuntimeError::InvalidMessage("Actor context not available for selective receive".to_string()))
        }
    }

    /// Link to another actor
    fn link_to(&self, target_pid: Pid, link_type: LinkType) -> RuntimeResult<()> {
        if let Some(context) = self.get_context() {
            context.link(target_pid, link_type)
        } else {
            Err(RuntimeError::InvalidMessage("Actor context not available for linking".to_string()))
        }
    }

    /// Monitor another actor
    fn monitor_actor(&self, target_pid: Pid) -> RuntimeResult<MonitorRef> {
        if let Some(context) = self.get_context() {
            context.monitor(target_pid)
        } else {
            Err(RuntimeError::InvalidMessage("Actor context not available for monitoring".to_string()))
        }
    }
}

/// Generic actor implementation with behavior function
pub struct Actor<S, F>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(&mut S, MessagePayload) -> RuntimeResult<()> + Send + Sync + 'static,
{
    pid: Pid,
    state: S,
    initial_state: S,
    behavior: F,
    mailbox: Arc<Mutex<VecDeque<MessagePayload>>>,
    alive: bool,
    context: ActorContext,
}

impl<S, F> Actor<S, F>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(&mut S, MessagePayload) -> RuntimeResult<()> + Send + Sync + 'static,
{
    /// Create a new actor with initial state and behavior
    pub fn new(pid: Pid, initial_state: S, behavior: F) -> Self {
        Actor {
            pid,
            state: initial_state.clone(),
            initial_state,
            behavior,
            mailbox: Arc::new(Mutex::new(VecDeque::new())),
            alive: true,
            context: ActorContext::new(pid),
        }
    }
    
    /// Get current state (immutable reference)
    pub fn state(&self) -> &S {
        &self.state
    }
    
    /// Get mailbox size
    pub fn mailbox_size(&self) -> usize {
        self.mailbox.lock().unwrap().len()
    }
    
    /// Process all pending messages
    pub fn process_mailbox(&mut self) -> RuntimeResult<usize> {
        let mut processed = 0;
        
        while let Some(message) = {
            let mut mailbox = self.mailbox.lock().unwrap();
            mailbox.pop_front()
        } {
            self.receive(message)?;
            processed += 1;
        }
        
        Ok(processed)
    }
}

impl<S, F> ReamActor for Actor<S, F>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(&mut S, MessagePayload) -> RuntimeResult<()> + Send + Sync + 'static,
{
    fn receive(&mut self, message: MessagePayload) -> RuntimeResult<()> {
        if !self.alive {
            return Err(RuntimeError::ProcessNotFound(self.pid));
        }

        (self.behavior)(&mut self.state, message)
    }

    fn pid(&self) -> Pid {
        self.pid
    }

    fn restart(&mut self) -> RuntimeResult<()> {
        self.state = self.initial_state.clone();
        self.alive = true;

        // Clear mailbox
        self.mailbox.lock().unwrap().clear();

        // Clear context mailbox
        self.context.mailbox.lock().unwrap().clear();

        Ok(())
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn debug_state(&self) -> Box<dyn Any + Send> {
        Box::new(self.state.clone())
    }

    fn get_context(&self) -> Option<&ActorContext> {
        Some(&self.context)
    }
}

/// Simple counter actor for testing
pub struct CounterActor {
    pid: Pid,
    count: i64,
    initial_count: i64,
}

impl CounterActor {
    pub fn new(pid: Pid, initial_count: i64) -> Self {
        CounterActor {
            pid,
            count: initial_count,
            initial_count,
        }
    }
    
    pub fn count(&self) -> i64 {
        self.count
    }
}

impl ReamActor for CounterActor {
    fn receive(&mut self, message: MessagePayload) -> RuntimeResult<()> {
        match message {
            MessagePayload::Text(cmd) => {
                match cmd.as_str() {
                    "increment" => self.count += 1,
                    "decrement" => self.count -= 1,
                    "reset" => self.count = self.initial_count,
                    _ => return Err(RuntimeError::InvalidMessage(cmd)),
                }
            }
            MessagePayload::Data(data) => {
                if let Some(n) = data.as_i64() {
                    self.count += n;
                } else {
                    return Err(RuntimeError::InvalidMessage("Expected number".to_string()));
                }
            }
            _ => return Err(RuntimeError::InvalidMessage("Unsupported message type".to_string())),
        }
        
        Ok(())
    }
    
    fn pid(&self) -> Pid {
        self.pid
    }
    
    fn restart(&mut self) -> RuntimeResult<()> {
        self.count = self.initial_count;
        Ok(())
    }
}

/// Echo actor that responds with the same message
pub struct EchoActor {
    pid: Pid,
    response_count: usize,
}

impl EchoActor {
    pub fn new(pid: Pid) -> Self {
        EchoActor {
            pid,
            response_count: 0,
        }
    }
    
    pub fn response_count(&self) -> usize {
        self.response_count
    }
}

impl ReamActor for EchoActor {
    fn receive(&mut self, message: MessagePayload) -> RuntimeResult<()> {
        self.response_count += 1;
        
        // In a real implementation, we would send the message back
        // For now, just acknowledge receipt
        println!("Echo actor {} received: {:?}", self.pid, message);
        
        Ok(())
    }
    
    fn pid(&self) -> Pid {
        self.pid
    }
    
    fn restart(&mut self) -> RuntimeResult<()> {
        self.response_count = 0;
        Ok(())
    }
}

/// Actor factory for creating common actor types
pub struct ActorFactory;

impl ActorFactory {
    /// Create a counter actor
    pub fn counter(pid: Pid, initial_count: i64) -> Box<dyn ReamActor> {
        Box::new(CounterActor::new(pid, initial_count))
    }
    
    /// Create an echo actor
    pub fn echo(pid: Pid) -> Box<dyn ReamActor> {
        Box::new(EchoActor::new(pid))
    }
    
    /// Create a generic actor with behavior function
    pub fn generic<S, F>(pid: Pid, initial_state: S, behavior: F) -> Box<dyn ReamActor>
    where
        S: Clone + Send + Sync + 'static,
        F: Fn(&mut S, MessagePayload) -> RuntimeResult<()> + Send + Sync + 'static,
    {
        Box::new(Actor::new(pid, initial_state, behavior))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MessagePayload;

    #[test]
    fn test_counter_actor() {
        let pid = Pid::new();
        let mut actor = CounterActor::new(pid, 0);
        
        assert_eq!(actor.count(), 0);
        assert_eq!(actor.pid(), pid);
        
        actor.receive(MessagePayload::Text("increment".to_string())).unwrap();
        assert_eq!(actor.count(), 1);
        
        actor.receive(MessagePayload::Text("decrement".to_string())).unwrap();
        assert_eq!(actor.count(), 0);
        
        actor.restart().unwrap();
        assert_eq!(actor.count(), 0);
    }
    
    #[test]
    fn test_echo_actor() {
        let pid = Pid::new();
        let mut actor = EchoActor::new(pid);
        
        assert_eq!(actor.response_count(), 0);
        
        actor.receive(MessagePayload::Text("hello".to_string())).unwrap();
        assert_eq!(actor.response_count(), 1);
        
        actor.restart().unwrap();
        assert_eq!(actor.response_count(), 0);
    }
    
    #[test]
    fn test_generic_actor() {
        let pid = Pid::new();
        let mut actor = Actor::new(pid, 42i32, |state, msg| {
            match msg {
                MessagePayload::Data(data) => {
                    if let Some(n) = data.as_i64() {
                        *state += n as i32;
                    }
                }
                _ => {}
            }
            Ok(())
        });
        
        assert_eq!(*actor.state(), 42);
        
        actor.receive(MessagePayload::Data(serde_json::Value::Number(
            serde_json::Number::from(8)
        ))).unwrap();
        
        assert_eq!(*actor.state(), 50);
    }

    #[test]
    fn test_selective_receive() {
        let pid = Pid::new();
        let context = ActorContext::new(pid);

        // Add some messages to the mailbox
        {
            let mut mailbox = context.mailbox.lock().unwrap();
            mailbox.push_back(MessagePayload::Text("hello".to_string()));
            mailbox.push_back(MessagePayload::Data(serde_json::Value::Number(42.into())));
            mailbox.push_back(MessagePayload::Text("world".to_string()));
        }

        // Test pattern matching for specific text
        let pattern = MessagePattern::Text("world".to_string());
        let result = context.selective_receive(pattern, Some(Duration::from_millis(100))).unwrap();
        assert!(result.is_some());
        if let Some(MessagePayload::Text(text)) = result {
            assert_eq!(text, "world");
        } else {
            panic!("Expected text message");
        }

        // Test pattern matching for message type
        let pattern = MessagePattern::Type(MessageType::Data);
        let result = context.selective_receive(pattern, Some(Duration::from_millis(100))).unwrap();
        assert!(result.is_some());

        // Test timeout
        let pattern = MessagePattern::Text("nonexistent".to_string());
        let result = context.selective_receive(pattern, Some(Duration::from_millis(10))).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_actor_linking() {
        let pid1 = Pid::new();
        let pid2 = Pid::new();
        let context = ActorContext::new(pid1);

        // Test linking
        context.link(pid2, LinkType::Bidirectional).unwrap();
        let links = context.get_links();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].linked_pid, pid2);
        assert_eq!(links[0].link_type, LinkType::Bidirectional);

        // Test unlinking
        context.unlink(pid2).unwrap();
        let links = context.get_links();
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_actor_monitoring() {
        let pid1 = Pid::new();
        let pid2 = Pid::new();
        let context = ActorContext::new(pid1);

        // Test monitoring
        let monitor_ref = context.monitor(pid2).unwrap();
        let monitors = context.get_monitors();
        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].monitored_pid, pid2);
        assert_eq!(monitors[0].monitor_ref, monitor_ref);

        // Test demonitoring
        context.demonitor(monitor_ref).unwrap();
        let monitors = context.get_monitors();
        assert_eq!(monitors.len(), 0);
    }

    #[test]
    fn test_enhanced_actor() {
        let pid = Pid::new();
        let initial_state = 0i32;
        let behavior = |state: &mut i32, message: MessagePayload| -> RuntimeResult<()> {
            match message {
                MessagePayload::Text(cmd) => {
                    match cmd.as_str() {
                        "increment" => *state += 1,
                        "decrement" => *state -= 1,
                        _ => return Err(RuntimeError::InvalidMessage(cmd)),
                    }
                }
                _ => return Err(RuntimeError::InvalidMessage("Unsupported message type".to_string())),
            }
            Ok(())
        };

        let actor = Actor::new(pid, initial_state, behavior);

        // Test that actor has context
        assert!(actor.get_context().is_some());

        // Test linking through actor
        let target_pid = Pid::new();
        actor.link_to(target_pid, LinkType::Monitor).unwrap();

        // Test monitoring through actor
        let monitor_ref = actor.monitor_actor(target_pid).unwrap();
        assert!(monitor_ref.0 > 0);
    }
}
