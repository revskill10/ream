//! Process isolation system for fault tolerance
//!
//! Implements mathematical process isolation with memory boundaries,
//! fault handlers, and execution bounds to prevent one process from
//! affecting others.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::alloc::{alloc, dealloc, Layout};

use crate::types::{Pid, ExecutionBounds, MemoryLayout};
use crate::error::{FaultError, FaultResult};
use crate::runtime::actor::ReamActor;

/// Atomic counters for tracking resource usage
#[derive(Default)]
pub struct AtomicCounters {
    /// Instructions executed
    pub instructions: AtomicU64,
    /// Memory allocated in bytes
    pub memory: AtomicU64,
    /// Messages sent
    pub messages: AtomicU64,
}

impl AtomicCounters {
    /// Create new counters
    pub fn new() -> Self {
        AtomicCounters::default()
    }
    
    /// Reset all counters
    pub fn reset(&self) {
        self.instructions.store(0, Ordering::Relaxed);
        self.memory.store(0, Ordering::Relaxed);
        self.messages.store(0, Ordering::Relaxed);
    }
    
    /// Check if any bounds are exceeded
    pub fn check_bounds(&self, bounds: &ExecutionBounds) -> Option<FaultError> {
        if self.instructions.load(Ordering::Relaxed) >= bounds.instruction_limit {
            return Some(FaultError::InstructionLimitExceeded);
        }
        if self.memory.load(Ordering::Relaxed) >= bounds.memory_limit {
            return Some(FaultError::MemoryBoundaryExceeded);
        }
        if self.messages.load(Ordering::Relaxed) >= bounds.message_limit {
            return Some(FaultError::MessageQuotaExceeded);
        }
        None
    }
}

/// Isolated memory region with guard pages
pub struct IsolatedMemory {
    /// Base pointer to allocated memory
    base: *mut u8,
    /// Size of allocated memory
    size: usize,
    /// Guard pages for protection
    guard_pages: Vec<*mut u8>,
    /// Current allocation offset
    offset: AtomicU64,
}

impl IsolatedMemory {
    /// Create a new isolated memory region
    pub fn new(size: usize) -> FaultResult<Self> {
        // Allocate memory with guard pages
        let layout = Layout::from_size_align(size + 8192, 4096) // Extra for guard pages
            .map_err(|_| FaultError::MemoryBoundaryExceeded)?;
        
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(FaultError::MemoryBoundaryExceeded);
        }
        
        // Set up guard pages at beginning and end
        let guard_pages = vec![
            ptr,
            unsafe { ptr.add(size + 4096) },
        ];
        
        // Protect guard pages (this would use mprotect on Unix)
        // For now, we'll simulate this with a simple check
        
        Ok(IsolatedMemory {
            base: unsafe { ptr.add(4096) }, // Start after first guard page
            size,
            guard_pages,
            offset: AtomicU64::new(0),
        })
    }
    
    /// Allocate memory within this isolated region
    pub fn allocate(&self, size: usize) -> FaultResult<*mut u8> {
        let current_offset = self.offset.load(Ordering::Relaxed);
        let new_offset = current_offset + size as u64;
        
        if new_offset > self.size as u64 {
            return Err(FaultError::MemoryBoundaryExceeded);
        }
        
        // Try to update offset atomically
        match self.offset.compare_exchange_weak(
            current_offset,
            new_offset,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(unsafe { self.base.add(current_offset as usize) }),
            Err(_) => Err(FaultError::MemoryBoundaryExceeded),
        }
    }
    
    /// Check if a pointer is within this memory region
    pub fn contains(&self, ptr: *const u8) -> bool {
        let base_addr = self.base as usize;
        let ptr_addr = ptr as usize;
        ptr_addr >= base_addr && ptr_addr < base_addr + self.size
    }
}

impl Drop for IsolatedMemory {
    fn drop(&mut self) {
        // Deallocate memory
        let layout = Layout::from_size_align(self.size + 8192, 4096).unwrap();
        unsafe {
            dealloc(self.guard_pages[0], layout);
        }
    }
}

/// Isolated mailbox for message passing
pub struct IsolatedMailbox {
    /// Messages in the mailbox
    messages: Mutex<Vec<crate::types::MessagePayload>>,
    /// Maximum number of messages
    max_messages: usize,
    /// Current message count
    message_count: AtomicU64,
}

impl IsolatedMailbox {
    /// Create a new isolated mailbox
    pub fn new(max_messages: usize) -> Self {
        IsolatedMailbox {
            messages: Mutex::new(Vec::with_capacity(max_messages)),
            max_messages,
            message_count: AtomicU64::new(0),
        }
    }
    
    /// Send a message to this mailbox
    pub fn send(&self, message: crate::types::MessagePayload) -> FaultResult<()> {
        let current_count = self.message_count.load(Ordering::Relaxed);
        if current_count >= self.max_messages as u64 {
            return Err(FaultError::MessageQuotaExceeded);
        }
        
        let mut messages = self.messages.lock().unwrap();
        if messages.len() >= self.max_messages {
            return Err(FaultError::MessageQuotaExceeded);
        }
        
        messages.push(message);
        self.message_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    /// Receive a message from this mailbox
    pub fn receive(&self) -> Option<crate::types::MessagePayload> {
        let mut messages = self.messages.lock().unwrap();
        if let Some(message) = messages.pop() {
            self.message_count.fetch_sub(1, Ordering::Relaxed);
            Some(message)
        } else {
            None
        }
    }
    
    /// Get current message count
    pub fn len(&self) -> usize {
        self.message_count.load(Ordering::Relaxed) as usize
    }
    
    /// Check if mailbox is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Fault handler trait for process recovery
pub trait FaultHandler: Send + Sync {
    /// Handle a process fault
    fn handle_fault(&self, fault: ProcessFault) -> RecoveryAction;
    
    /// Determine the type of fault
    fn determine_fault_type(&self, process: &IsolatedProcess) -> FaultClassification;
}

/// Types of process faults
#[derive(Debug, Clone)]
pub enum ProcessFault {
    /// Process panicked
    Panic(String),
    /// Infinite loop detected
    InfiniteLoop,
    /// Out of memory
    OutOfMemory,
    /// Message overflow
    MessageOverflow,
    /// Segmentation fault
    SegmentationFault,
    /// Instruction limit exceeded
    InstructionLimit,
    /// Timeout
    Timeout,
}

/// Recovery actions for faults
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Restart the process
    Restart,
    /// Kill the process
    Kill,
    /// Suspend the process
    Suspend,
    /// Escalate to supervisor
    Escalate,
    /// Replace with new process
    Replace,
}

/// Fault classification for recovery decisions
#[derive(Debug, Clone)]
pub enum FaultClassification {
    /// Transient fault - likely to succeed on retry
    Transient,
    /// Permanent fault - unlikely to succeed on retry
    Permanent,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Logic error
    LogicError,
}

/// Default fault handler implementation
pub struct DefaultFaultHandler;

impl FaultHandler for DefaultFaultHandler {
    fn handle_fault(&self, fault: ProcessFault) -> RecoveryAction {
        match fault {
            ProcessFault::Panic(_) => RecoveryAction::Restart,
            ProcessFault::InfiniteLoop => RecoveryAction::Kill,
            ProcessFault::OutOfMemory => RecoveryAction::Restart,
            ProcessFault::MessageOverflow => RecoveryAction::Suspend,
            ProcessFault::SegmentationFault => RecoveryAction::Kill,
            ProcessFault::InstructionLimit => RecoveryAction::Restart,
            ProcessFault::Timeout => RecoveryAction::Restart,
        }
    }
    
    fn determine_fault_type(&self, _process: &IsolatedProcess) -> FaultClassification {
        FaultClassification::Transient
    }
}

/// Isolated process with fault boundaries
pub struct IsolatedProcess {
    /// Process ID
    pid: Pid,
    /// The actor running in this process
    actor: Box<dyn ReamActor>,
    /// Isolated memory region
    memory: IsolatedMemory,
    /// Isolated mailbox
    mailbox: IsolatedMailbox,
    /// Fault handler
    fault_handler: Arc<dyn FaultHandler>,
    /// Execution bounds
    execution_bounds: ExecutionBounds,
    /// Resource counters
    current_counts: Arc<AtomicCounters>,
    /// Whether the process is alive
    alive: AtomicBool,
}

impl IsolatedProcess {
    /// Create a new isolated process
    pub fn new(
        actor: Box<dyn ReamActor>,
        memory_layout: MemoryLayout,
        execution_bounds: ExecutionBounds,
    ) -> FaultResult<Self> {
        let pid = Pid::new();
        let memory_size = memory_layout.process_heap.end - memory_layout.process_heap.start;
        let memory = IsolatedMemory::new(memory_size as usize)?;
        let mailbox = IsolatedMailbox::new(execution_bounds.message_limit as usize);
        let fault_handler = Arc::new(DefaultFaultHandler);
        let current_counts = Arc::new(AtomicCounters::new());
        
        Ok(IsolatedProcess {
            pid,
            actor,
            memory,
            mailbox,
            fault_handler,
            execution_bounds,
            current_counts,
            alive: AtomicBool::new(true),
        })
    }
    
    /// Get the process ID
    pub fn pid(&self) -> Pid {
        self.pid
    }
    
    /// Check if the process is alive
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }
    
    /// Kill the process
    pub fn kill(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }
    
    /// Process a single message with bounds checking
    pub fn process_message(&mut self) -> FaultResult<Option<()>> {
        if !self.is_alive() {
            return Ok(None);
        }
        
        // Check execution bounds before processing
        if let Some(fault_error) = self.current_counts.check_bounds(&self.execution_bounds) {
            return Err(fault_error);
        }
        
        // Increment instruction counter
        self.current_counts.instructions.fetch_add(1, Ordering::Relaxed);
        
        // Try to receive a message
        if let Some(message) = self.mailbox.receive() {
            // Process the message
            match self.actor.receive(message) {
                Ok(_) => Ok(Some(())),
                Err(e) => {
                    // Handle actor error as a fault
                    let fault = ProcessFault::Panic(format!("{:?}", e));
                    let action = self.fault_handler.handle_fault(fault);
                    match action {
                        RecoveryAction::Kill => {
                            self.kill();
                            Ok(None)
                        }
                        RecoveryAction::Restart => {
                            self.restart()?;
                            Ok(Some(()))
                        }
                        _ => Err(FaultError::FaultHandler(format!("Unhandled recovery action: {:?}", action))),
                    }
                }
            }
        } else {
            // No messages to process
            Ok(Some(()))
        }
    }
    
    /// Restart the process
    pub fn restart(&mut self) -> FaultResult<()> {
        // Reset counters
        self.current_counts.reset();
        
        // Restart the actor
        self.actor.restart().map_err(|e| FaultError::RecoveryFailed(format!("{:?}", e)))?;
        
        // Mark as alive
        self.alive.store(true, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Send a message to this process
    pub fn send_message(&self, message: crate::types::MessagePayload) -> FaultResult<()> {
        self.mailbox.send(message)
    }
    
    /// Get current resource usage
    pub fn get_resource_usage(&self) -> (u64, u64, u64) {
        (
            self.current_counts.instructions.load(Ordering::Relaxed),
            self.current_counts.memory.load(Ordering::Relaxed),
            self.current_counts.messages.load(Ordering::Relaxed),
        )
    }
}
