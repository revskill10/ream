//! Message passing system with monoidal composition

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crossbeam_channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use crate::types::{Pid, Message, MessagePayload};
use crate::error::{RuntimeError, RuntimeResult};

/// Type alias for actor messages (for macro compatibility)
pub type ActorMessage = MessagePayload;

/// Trait for message types (for macro compatibility)
pub trait MessageTrait {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Result<Self, RuntimeError> where Self: Sized;
}

/// Message router for inter-process communication
pub struct MessageRouter {
    /// Process mailboxes
    mailboxes: Arc<DashMap<Pid, Arc<RwLock<Mailbox>>>>,
    
    /// Message delivery channel
    delivery_tx: Sender<Message>,
    delivery_rx: Receiver<Message>,
    
    /// Router statistics
    stats: Arc<RwLock<RouterStats>>,
    
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug, Default, Clone)]
struct RouterStats {
    messages_sent: u64,
    messages_delivered: u64,
    messages_dropped: u64,
    total_delivery_time: std::time::Duration,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        let (delivery_tx, delivery_rx) = unbounded();
        
        MessageRouter {
            mailboxes: Arc::new(DashMap::new()),
            delivery_tx,
            delivery_rx,
            stats: Arc::new(RwLock::new(RouterStats::default())),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Start the message router
    pub fn start(&self) -> RuntimeResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        let delivery_rx = self.delivery_rx.clone();
        let mailboxes = Arc::clone(&self.mailboxes);
        let stats = Arc::clone(&self.stats);
        let running = Arc::clone(&self.running);
        
        std::thread::spawn(move || {
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                match delivery_rx.try_recv() {
                    Ok(message) => {
                        let start = std::time::Instant::now();
                        
                        if let Some(mailbox) = mailboxes.get(&message.to) {
                            if let Ok(mut mb) = mailbox.write() {
                                mb.send(message.payload);
                                
                                let mut s = stats.write().unwrap();
                                s.messages_delivered += 1;
                                s.total_delivery_time += start.elapsed();
                            }
                        } else {
                            // Process not found, drop message
                            let mut s = stats.write().unwrap();
                            s.messages_dropped += 1;
                        }
                    }
                    Err(_) => {
                        // No messages, sleep briefly
                        std::thread::sleep(std::time::Duration::from_micros(100));
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the message router
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Register a process mailbox
    pub fn register_process(&self, pid: Pid) -> Arc<RwLock<Mailbox>> {
        let mailbox = Arc::new(RwLock::new(Mailbox::new()));
        self.mailboxes.insert(pid, Arc::clone(&mailbox));
        mailbox
    }
    
    /// Unregister a process mailbox
    pub fn unregister_process(&self, pid: Pid) {
        self.mailboxes.remove(&pid);
    }
    
    /// Send a message to a process
    pub fn send_message(&self, to: Pid, payload: MessagePayload) -> RuntimeResult<()> {
        let message = Message {
            from: Pid::new(), // TODO: Get actual sender PID from context
            to,
            payload,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        
        self.delivery_tx.send(message).map_err(|_| {
            RuntimeError::InvalidMessage("Failed to queue message".to_string())
        })?;
        
        let mut stats = self.stats.write().unwrap();
        stats.messages_sent += 1;
        
        Ok(())
    }
    
    /// Get router statistics
    pub fn stats(&self) -> RouterStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Get mailbox for a process
    pub fn get_mailbox(&self, pid: Pid) -> Option<Arc<RwLock<Mailbox>>> {
        self.mailboxes.get(&pid).map(|entry| Arc::clone(entry.value()))
    }
}

/// Process mailbox with coalgebraic message observation
#[derive(Debug)]
pub struct Mailbox {
    /// Message queue
    messages: VecDeque<MessagePayload>,
    
    /// Maximum queue size
    max_size: usize,
    
    /// Mailbox statistics
    stats: MailboxStats,
}

#[derive(Debug, Default, Clone)]
struct MailboxStats {
    messages_received: u64,
    messages_processed: u64,
    queue_overflows: u64,
    max_queue_size: usize,
}

impl Mailbox {
    /// Create a new mailbox
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }
    
    /// Create a new mailbox with specific capacity
    pub fn with_capacity(max_size: usize) -> Self {
        Mailbox {
            messages: VecDeque::new(),
            max_size,
            stats: MailboxStats::default(),
        }
    }
    
    /// Send a message to this mailbox
    pub fn send(&mut self, message: MessagePayload) {
        if self.messages.len() >= self.max_size {
            // Drop oldest message to make room
            self.messages.pop_front();
            self.stats.queue_overflows += 1;
        }
        
        self.messages.push_back(message);
        self.stats.messages_received += 1;
        
        if self.messages.len() > self.stats.max_queue_size {
            self.stats.max_queue_size = self.messages.len();
        }
    }
    
    /// Receive a message from this mailbox
    pub fn receive(&mut self) -> Option<MessagePayload> {
        if let Some(message) = self.messages.pop_front() {
            self.stats.messages_processed += 1;
            Some(message)
        } else {
            None
        }
    }
    
    /// Peek at the next message without removing it
    pub fn peek(&self) -> Option<&MessagePayload> {
        self.messages.front()
    }
    
    /// Get number of messages in queue
    pub fn len(&self) -> usize {
        self.messages.len()
    }
    
    /// Check if mailbox is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
    
    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }
    
    /// Get mailbox statistics
    pub fn stats(&self) -> &MailboxStats {
        &self.stats
    }
    
    /// Filter messages by predicate
    pub fn filter<F>(&mut self, predicate: F) -> Vec<MessagePayload>
    where
        F: Fn(&MessagePayload) -> bool,
    {
        let mut filtered = Vec::new();
        let mut remaining = VecDeque::new();
        
        while let Some(message) = self.messages.pop_front() {
            if predicate(&message) {
                filtered.push(message);
            } else {
                remaining.push_back(message);
            }
        }
        
        self.messages = remaining;
        filtered
    }
    
    /// Map over all messages
    pub fn map<F, T>(&self, f: F) -> Vec<T>
    where
        F: Fn(&MessagePayload) -> T,
    {
        self.messages.iter().map(f).collect()
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Message trait for composable message types
pub trait MessageCompose: Clone + Send + Sync {
    type Identity: MessageCompose;
    
    /// Combine two messages
    fn combine(self, other: Self) -> Self;
    
    /// Identity element for combination
    fn identity() -> Self::Identity;
}

impl MessageCompose for MessagePayload {
    type Identity = MessagePayload;
    
    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (MessagePayload::Text(a), MessagePayload::Text(b)) => {
                MessagePayload::Text(format!("{}{}", a, b))
            }
            (MessagePayload::Bytes(mut a), MessagePayload::Bytes(b)) => {
                a.extend(b);
                MessagePayload::Bytes(a)
            }
            (a, _) => a, // Left-biased combination
        }
    }
    
    fn identity() -> Self::Identity {
        MessagePayload::Text(String::new())
    }
}

/// Channel abstraction for typed message passing
pub struct Channel<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Channel<T> {
    /// Create a new channel
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Channel { sender, receiver }
    }
    
    /// Send a message
    pub fn send(&self, message: T) -> Result<(), T> {
        self.sender.send(message).map_err(|e| e.into_inner())
    }
    
    /// Receive a message
    pub fn recv(&self) -> Result<T, crossbeam_channel::RecvError> {
        self.receiver.recv()
    }
    
    /// Try to receive a message without blocking
    pub fn try_recv(&self) -> Result<T, crossbeam_channel::TryRecvError> {
        self.receiver.try_recv()
    }
    
    /// Get sender handle
    pub fn sender(&self) -> Sender<T> {
        self.sender.clone()
    }
    
    /// Get receiver handle
    pub fn receiver(&self) -> Receiver<T> {
        self.receiver.clone()
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailbox() {
        let mut mailbox = Mailbox::new();
        
        assert!(mailbox.is_empty());
        assert_eq!(mailbox.len(), 0);
        
        mailbox.send(MessagePayload::Text("hello".to_string()));
        assert_eq!(mailbox.len(), 1);
        
        let message = mailbox.receive();
        assert!(message.is_some());
        assert!(mailbox.is_empty());
    }
    
    #[test]
    fn test_message_compose() {
        let msg1 = MessagePayload::Text("hello".to_string());
        let msg2 = MessagePayload::Text(" world".to_string());
        
        let combined = msg1.combine(msg2);
        match combined {
            MessagePayload::Text(text) => assert_eq!(text, "hello world"),
            _ => panic!("Expected text message"),
        }
    }
    
    #[test]
    fn test_channel() {
        let channel = Channel::new();
        
        channel.send(42).unwrap();
        let received = channel.recv().unwrap();
        assert_eq!(received, 42);
    }
    
    #[test]
    fn test_message_router() {
        let router = MessageRouter::new();
        let pid = Pid::new();
        
        let mailbox = router.register_process(pid);
        router.start().unwrap();
        
        router.send_message(pid, MessagePayload::Text("test".to_string())).unwrap();
        
        // Give router time to deliver
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let message = mailbox.write().unwrap().receive();
        assert!(message.is_some());
        
        router.stop();
    }
}
