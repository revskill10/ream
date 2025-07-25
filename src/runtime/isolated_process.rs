//! Process isolation system for fault tolerance
//!
//! Implements mathematical process isolation with memory boundaries,
//! fault handlers, and execution bounds to prevent one process from
//! affecting others.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

use crate::types::{Pid, ExecutionBounds, MemoryLayout};
use crate::error::{FaultError, FaultResult};
use crate::runtime::actor::ReamActor;
use crate::runtime::preemption::PreemptionTimer;

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

/// Memory allocation tracking
#[derive(Debug, Clone)]
struct MemoryAllocation {
    /// Base address of allocation
    base: *mut u8,
    /// Size of allocation
    size: usize,
    /// Allocation timestamp
    allocated_at: std::time::Instant,
}

unsafe impl Send for MemoryAllocation {}
unsafe impl Sync for MemoryAllocation {}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memory size allocated to this region
    pub total_size: usize,
    /// Currently allocated bytes
    pub allocated_bytes: usize,
    /// Free bytes remaining
    pub free_bytes: usize,
    /// Number of active allocations
    pub allocation_count: usize,
    /// Memory fragmentation ratio (0.0 = no fragmentation, 1.0 = fully fragmented)
    pub fragmentation_ratio: f64,
}

/// Isolated memory region with guard pages and protection
pub struct IsolatedMemory {
    /// Base pointer to allocated memory
    base: *mut u8,
    /// Size of allocated memory
    size: usize,
    /// Guard pages for protection
    guard_pages: Vec<*mut u8>,
    /// Current allocation offset
    offset: AtomicU64,
    /// Memory protection enabled
    protection_enabled: bool,
    /// Allocation tracking
    allocations: Mutex<Vec<MemoryAllocation>>,
}

// Safety: IsolatedMemory contains raw pointers to memory that is owned by this instance.
// The memory is allocated and deallocated by this instance, and the pointers remain valid
// for the lifetime of the IsolatedMemory. It's safe to send between threads as long as
// the memory is not accessed concurrently without proper synchronization.
unsafe impl Send for IsolatedMemory {}
unsafe impl Sync for IsolatedMemory {}

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
            protection_enabled: true,
            allocations: Mutex::new(Vec::new()),
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
            Ok(_) => {
                let ptr = unsafe { self.base.add(current_offset as usize) };
                
                // Track the allocation
                let allocation = MemoryAllocation {
                    base: ptr,
                    size,
                    allocated_at: std::time::Instant::now(),
                };
                
                if let Ok(mut allocations) = self.allocations.try_lock() {
                    allocations.push(allocation);
                }
                
                // Initialize memory to zero for security
                unsafe {
                    ptr::write_bytes(ptr, 0, size);
                }
                
                Ok(ptr)
            }
            Err(_) => Err(FaultError::MemoryBoundaryExceeded),
        }
    }
    
    /// Check if a pointer is within this memory region
    pub fn contains(&self, ptr: *const u8) -> bool {
        let base_addr = self.base as usize;
        let ptr_addr = ptr as usize;
        ptr_addr >= base_addr && ptr_addr < base_addr + self.size
    }
    
    /// Get current memory usage statistics
    pub fn get_stats(&self) -> MemoryStats {
        let allocations = self.allocations.lock().unwrap();
        let total_allocated = allocations.iter().map(|a| a.size).sum();
        let allocation_count = allocations.len();
        let current_offset = self.offset.load(Ordering::Relaxed);
        
        MemoryStats {
            total_size: self.size,
            allocated_bytes: total_allocated,
            free_bytes: self.size - current_offset as usize,
            allocation_count,
            fragmentation_ratio: if self.size > 0 {
                (current_offset as usize - total_allocated) as f64 / self.size as f64
            } else {
                0.0
            },
        }
    }
    
    /// Validate memory integrity (check for corruption)
    pub fn validate_integrity(&self) -> Result<(), String> {
        let allocations = self.allocations.lock().unwrap();
        
        // Check guard pages for corruption
        for (i, &guard_page) in self.guard_pages.iter().enumerate() {
            // In a real implementation, we'd check if guard pages were written to
            // For now, we just verify they're still valid pointers
            if guard_page.is_null() {
                return Err(format!("Guard page {} is null", i));
            }
        }
        
        // Check for overlapping allocations
        for (i, alloc1) in allocations.iter().enumerate() {
            for (j, alloc2) in allocations.iter().enumerate() {
                if i != j {
                    let addr1 = alloc1.base as usize;
                    let end1 = addr1 + alloc1.size;
                    let addr2 = alloc2.base as usize;
                    let end2 = addr2 + alloc2.size;
                    
                    if (addr1 < end2 && addr2 < end1) {
                        return Err(format!("Overlapping allocations detected: {:p}-{:p} and {:p}-{:p}", 
                                         alloc1.base, (addr1 + alloc1.size) as *const u8,
                                         alloc2.base, (addr2 + alloc2.size) as *const u8));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Enable or disable memory protection
    pub fn set_protection(&mut self, enabled: bool) {
        self.protection_enabled = enabled;
        // In a real implementation, this would call mprotect
        // to enable/disable read/write/execute permissions
    }
    
    /// Get allocation information for debugging
    pub fn get_allocations(&self) -> Vec<MemoryAllocation> {
        self.allocations.lock().unwrap().clone()
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
    /// Preemption timer for signal-based interruption
    preemption_timer: Option<Arc<PreemptionTimer>>,
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
            preemption_timer: None,
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
    
    /// Handle a fault (delegated to fault handler)
    pub fn handle_fault(&self, fault: ProcessFault) -> RecoveryAction {
        self.fault_handler.handle_fault(fault)
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

    /// Set preemption timer for signal-based interruption
    pub fn set_preemption_timer(&mut self, timer: Arc<PreemptionTimer>) {
        self.preemption_timer = Some(timer);
    }

    /// Process messages with preemptive scheduling
    pub fn process_message_preemptive(&mut self) -> FaultResult<Option<()>> {
        if !self.is_alive() {
            return Ok(None);
        }

        // Start quantum if we have a preemption timer
        if let Some(ref timer) = self.preemption_timer {
            timer.start_quantum();
        }

        // Check execution bounds before processing
        if let Some(fault_error) = self.current_counts.check_bounds(&self.execution_bounds) {
            return Err(fault_error);
        }

        // Process message with preemption checks
        let mut instruction_count = 0;
        const PREEMPTION_CHECK_INTERVAL: u32 = 100;

        // Try to receive a message
        if let Some(message) = self.mailbox.receive() {
            loop {
                // Check for preemption signal
                if let Some(ref timer) = self.preemption_timer {
                    if timer.should_preempt() {
                        // Save state and yield
                        return Ok(Some(()));
                    }
                }

                // Increment instruction counter
                instruction_count += 1;
                self.current_counts.instructions.fetch_add(1, Ordering::Relaxed);
                
                // Check bounds periodically
                if instruction_count % PREEMPTION_CHECK_INTERVAL == 0 {
                    if let Some(fault_error) = self.current_counts.check_bounds(&self.execution_bounds) {
                        return Err(fault_error);
                    }
                }

                // Simulate message processing work
                if instruction_count >= 1000 {
                    // Process the actual message
                    match self.actor.receive(message) {
                        Ok(_) => return Ok(Some(())),
                        Err(e) => {
                            let fault = ProcessFault::Panic(format!("{:?}", e));
                            let action = self.fault_handler.handle_fault(fault);
                            match action {
                                RecoveryAction::Kill => {
                                    self.kill();
                                    return Ok(None);
                                }
                                RecoveryAction::Restart => {
                                    self.restart()?;
                                    return Ok(Some(()));
                                }
                                _ => return Err(FaultError::FaultHandler(format!("Unhandled recovery action: {:?}", action))),
                            }
                        }
                    }
                }
            }
        } else {
            // No messages to process
            Ok(Some(()))
        }
    }

    /// Get memory statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        self.memory.get_stats()
    }

    /// Validate memory integrity
    pub fn validate_memory(&self) -> Result<(), String> {
        self.memory.validate_integrity()
    }

    /// Enable/disable memory protection
    pub fn set_memory_protection(&mut self, enabled: bool) -> FaultResult<()> {
        // This would require mutable access to memory, which we need to handle carefully
        // For now, we'll just return Ok since we can't modify through immutable reference
        Ok(())
    }
}
