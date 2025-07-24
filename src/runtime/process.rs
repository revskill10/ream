//! Process management and execution

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use crate::types::{Pid, Priority, ProcessState, ProcessInfo};
use crate::error::RuntimeResult;
use crate::runtime::actor::ReamActor;
use crate::runtime::message::Mailbox;

/// Process execution context
pub struct Process {
    /// Process ID
    pid: Pid,
    
    /// Actor implementation
    actor: Box<dyn ReamActor>,
    
    /// Process priority
    priority: Priority,
    
    /// Current state
    state: ProcessState,
    
    /// Process mailbox
    mailbox: Arc<RwLock<Mailbox>>,
    
    /// Process statistics
    stats: ProcessStats,
    
    /// Creation time
    created_at: Instant,
    
    /// Parent process (if any)
    parent: Option<Pid>,
    
    /// Linked processes
    links: Vec<Pid>,
    
    /// Monitored processes
    monitors: Vec<Pid>,
}

#[derive(Debug, Default, Clone)]
struct ProcessStats {
    messages_processed: u64,
    cpu_time: Duration,
    memory_usage: usize,
    restarts: u32,
    last_activity: Option<Instant>,
}

impl Process {
    /// Create a new process
    pub fn new(pid: Pid, actor: Box<dyn ReamActor>, priority: Priority) -> Self {
        Process {
            pid,
            actor,
            priority,
            state: ProcessState::Running,
            mailbox: Arc::new(RwLock::new(Mailbox::new())),
            stats: ProcessStats::default(),
            created_at: Instant::now(),
            parent: None,
            links: Vec::new(),
            monitors: Vec::new(),
        }
    }
    
    /// Get process ID
    pub fn pid(&self) -> Pid {
        self.pid
    }
    
    /// Get process state
    pub fn state(&self) -> ProcessState {
        self.state
    }
    
    /// Get process priority
    pub fn priority(&self) -> Priority {
        self.priority
    }
    
    /// Set process priority
    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }
    
    /// Get process mailbox
    pub fn mailbox(&self) -> Arc<RwLock<Mailbox>> {
        Arc::clone(&self.mailbox)
    }
    
    /// Execute a quantum of work
    pub fn run_quantum(&mut self) -> RuntimeResult<usize> {
        if self.state != ProcessState::Running {
            return Ok(0);
        }
        
        let start = Instant::now();
        let mut messages_processed = 0;
        
        // Process messages from mailbox
        {
            let mut mailbox = self.mailbox.write().unwrap();
            while let Some(message) = mailbox.receive() {
                self.actor.receive(message)?;
                messages_processed += 1;
                
                // Limit quantum to prevent starvation
                if messages_processed >= 10 {
                    break;
                }
            }
        }
        
        let quantum_time = start.elapsed();
        self.stats.cpu_time += quantum_time;
        self.stats.messages_processed += messages_processed;
        self.stats.last_activity = Some(Instant::now());
        
        Ok(messages_processed as usize)
    }
    
    /// Suspend the process
    pub fn suspend(&mut self) -> RuntimeResult<()> {
        if self.state == ProcessState::Running {
            self.state = ProcessState::Suspended;
        }
        Ok(())
    }
    
    /// Resume the process
    pub fn resume(&mut self) -> RuntimeResult<()> {
        if self.state == ProcessState::Suspended {
            self.state = ProcessState::Running;
        }
        Ok(())
    }
    
    /// Terminate the process
    pub fn terminate(&mut self) -> RuntimeResult<()> {
        self.state = ProcessState::Terminated;
        self.mailbox.write().unwrap().clear();
        Ok(())
    }
    
    /// Restart the process
    pub fn restart(&mut self) -> RuntimeResult<()> {
        self.actor.restart()?;
        self.state = ProcessState::Running;
        self.mailbox.write().unwrap().clear();
        self.stats.restarts += 1;
        Ok(())
    }
    
    /// Link to another process
    pub fn link(&mut self, other: Pid) {
        if !self.links.contains(&other) {
            self.links.push(other);
        }
    }
    
    /// Unlink from another process
    pub fn unlink(&mut self, other: Pid) {
        self.links.retain(|&pid| pid != other);
    }
    
    /// Monitor another process
    pub fn monitor(&mut self, other: Pid) {
        if !self.monitors.contains(&other) {
            self.monitors.push(other);
        }
    }
    
    /// Stop monitoring another process
    pub fn demonitor(&mut self, other: Pid) {
        self.monitors.retain(|&pid| pid != other);
    }
    
    /// Set parent process
    pub fn set_parent(&mut self, parent: Pid) {
        self.parent = Some(parent);
    }
    
    /// Get process information
    pub fn info(&self) -> ProcessInfo {
        ProcessInfo {
            pid: self.pid,
            state: self.state,
            priority: self.priority,
            parent: self.parent,
            links: self.links.clone(),
            monitors: self.monitors.clone(),
            message_queue_len: self.mailbox.read().unwrap().len(),
            memory_usage: self.stats.memory_usage,
            cpu_time: self.stats.cpu_time.as_micros() as u64,
        }
    }
    
    /// Check if process is alive
    pub fn is_alive(&self) -> bool {
        self.state != ProcessState::Terminated && self.actor.is_alive()
    }
    
    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    /// Get statistics
    pub fn stats(&self) -> &ProcessStats {
        &self.stats
    }
}

/// Process handle for external management
pub struct ProcessHandle {
    /// Inner process
    process: Arc<RwLock<Process>>,
}

impl ProcessHandle {
    /// Create a new process handle
    pub fn new(process: Process) -> Self {
        ProcessHandle {
            process: Arc::new(RwLock::new(process)),
        }
    }
    
    /// Get process ID
    pub fn pid(&self) -> Pid {
        self.process.read().unwrap().pid()
    }
    
    /// Get process state
    pub fn state(&self) -> ProcessState {
        self.process.read().unwrap().state()
    }
    
    /// Check if process is running
    pub fn is_running(&self) -> bool {
        self.process.read().unwrap().state() == ProcessState::Running
    }
    
    /// Check if process is alive
    pub fn is_alive(&self) -> bool {
        self.process.read().unwrap().is_alive()
    }
    
    /// Run a quantum of work
    pub fn run_quantum(&self) -> RuntimeResult<usize> {
        self.process.write().unwrap().run_quantum()
    }
    
    /// Suspend the process
    pub fn suspend(&self) -> RuntimeResult<()> {
        self.process.write().unwrap().suspend()
    }
    
    /// Resume the process
    pub fn resume(&self) -> RuntimeResult<()> {
        self.process.write().unwrap().resume()
    }
    
    /// Terminate the process
    pub fn terminate(&self) -> RuntimeResult<()> {
        self.process.write().unwrap().terminate()
    }
    
    /// Restart the process
    pub fn restart(&self) -> RuntimeResult<()> {
        self.process.write().unwrap().restart()
    }
    
    /// Get process information
    pub fn info(&self) -> ProcessInfo {
        self.process.read().unwrap().info()
    }
    
    /// Get process mailbox
    pub fn mailbox(&self) -> Arc<RwLock<Mailbox>> {
        self.process.read().unwrap().mailbox()
    }
    
    /// Link to another process
    pub fn link(&self, other: Pid) {
        self.process.write().unwrap().link(other);
    }
    
    /// Unlink from another process
    pub fn unlink(&self, other: Pid) {
        self.process.write().unwrap().unlink(other);
    }
    
    /// Monitor another process
    pub fn monitor(&self, other: Pid) {
        self.process.write().unwrap().monitor(other);
    }
    
    /// Stop monitoring another process
    pub fn demonitor(&self, other: Pid) {
        self.process.write().unwrap().demonitor(other);
    }
    
    /// Set parent process
    pub fn set_parent(&self, parent: Pid) {
        self.process.write().unwrap().set_parent(parent);
    }
    
    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.process.read().unwrap().uptime()
    }
}

impl Clone for ProcessHandle {
    fn clone(&self) -> Self {
        ProcessHandle {
            process: Arc::clone(&self.process),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::actor::CounterActor;
    use crate::types::MessagePayload;

    #[test]
    fn test_process_creation() {
        let pid = Pid::new();
        let actor = CounterActor::new(pid, 0);
        let process = Process::new(pid, Box::new(actor), Priority::Normal);
        
        assert_eq!(process.pid(), pid);
        assert_eq!(process.state(), ProcessState::Running);
        assert_eq!(process.priority(), Priority::Normal);
    }
    
    #[test]
    fn test_process_handle() {
        let pid = Pid::new();
        let actor = CounterActor::new(pid, 0);
        let process = Process::new(pid, Box::new(actor), Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        assert_eq!(handle.pid(), pid);
        assert!(handle.is_running());
        assert!(handle.is_alive());
    }
    
    #[test]
    fn test_process_lifecycle() {
        let pid = Pid::new();
        let actor = CounterActor::new(pid, 0);
        let process = Process::new(pid, Box::new(actor), Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        // Suspend
        handle.suspend().unwrap();
        assert_eq!(handle.state(), ProcessState::Suspended);
        
        // Resume
        handle.resume().unwrap();
        assert_eq!(handle.state(), ProcessState::Running);
        
        // Terminate
        handle.terminate().unwrap();
        assert_eq!(handle.state(), ProcessState::Terminated);
        assert!(!handle.is_alive());
    }
    
    #[test]
    fn test_process_messaging() {
        let pid = Pid::new();
        let actor = CounterActor::new(pid, 0);
        let process = Process::new(pid, Box::new(actor), Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        // Send a message
        {
            let mailbox = handle.mailbox();
            let mut mb = mailbox.write().unwrap();
            mb.send(MessagePayload::Text("increment".to_string()));
        }
        
        // Process the message
        let processed = handle.run_quantum().unwrap();
        assert_eq!(processed, 1);
    }
}
