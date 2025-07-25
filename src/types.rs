//! Core types and data structures for REAM

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::ops::Range;

use serde::{Deserialize, Serialize};

/// Process identifier - unique across the runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pid(pub u64);

impl Pid {
    /// Generate a new unique PID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Pid(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Get the raw PID value
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Parse a PID from a string
    pub fn from_string(s: &str) -> Result<Self, std::num::ParseIntError> {
        // Handle both "#123" and "123" formats
        let s = s.strip_prefix('#').unwrap_or(s);
        s.parse::<u64>().map(Pid)
    }

    /// Create a PID from a raw value
    pub fn from_raw(raw: u64) -> Self {
        Pid(raw)
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Process priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// High priority - system processes
    High = 0,
    /// Normal priority - user processes
    Normal = 1,
    /// Low priority - background tasks
    Low = 2,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Effect grades for tracking side effects in bytecode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EffectGrade {
    /// Pure computation - no side effects
    Pure,
    /// Memory reads
    Read,
    /// Memory writes
    Write,
    /// Memory operations (allocation/deallocation)
    Memory,
    /// Message sends
    Send,
    /// Process creation
    Spawn,
    /// External I/O
    IO,
}

impl EffectGrade {
    /// Combine two effect grades, taking the maximum
    pub fn combine(self, other: Self) -> Self {
        self.max(other)
    }
}

impl Default for EffectGrade {
    fn default() -> Self {
        EffectGrade::Pure
    }
}

/// Restart strategies for supervision trees
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartStrategy {
    /// Restart only the failed child
    OneForOne,
    /// Restart all children when one fails
    OneForAll,
    /// Restart the failed child and all children started after it
    RestForOne,
}

impl Default for RestartStrategy {
    fn default() -> Self {
        RestartStrategy::OneForOne
    }
}

/// Message envelope for inter-process communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Sender process ID
    pub from: Pid,
    /// Recipient process ID
    pub to: Pid,
    /// Message payload
    pub payload: MessagePayload,
    /// Message timestamp
    pub timestamp: u64,
}

/// Message payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Raw bytes
    Bytes(Vec<u8>),
    /// Text message
    Text(String),
    /// Structured data
    Data(serde_json::Value),
    /// System control message
    Control(ControlMessage),
}

/// System control messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    /// Terminate process
    Terminate,
    /// Suspend process
    Suspend,
    /// Resume process
    Resume,
    /// Link to another process
    Link(Pid),
    /// Monitor another process
    Monitor(Pid),
    /// Process exit notification
    Exit { pid: Pid, reason: String },
}

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    /// Process is running
    Running,
    /// Process is waiting for messages
    Waiting,
    /// Process is suspended
    Suspended,
    /// Process has terminated
    Terminated,
}

/// Process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: Pid,
    /// Current state
    pub state: ProcessState,
    /// Priority level
    pub priority: Priority,
    /// Parent process (if any)
    pub parent: Option<Pid>,
    /// Linked processes
    pub links: Vec<Pid>,
    /// Monitored processes
    pub monitors: Vec<Pid>,
    /// Message queue size
    pub message_queue_len: usize,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// CPU time used (microseconds)
    pub cpu_time: u64,
}

/// Runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    /// Total number of processes
    pub process_count: usize,
    /// Number of running processes
    pub running_processes: usize,
    /// Total memory usage
    pub memory_usage: usize,
    /// Messages sent per second
    pub message_rate: f64,
    /// Scheduler utilization
    pub scheduler_utilization: f64,
    /// GC collections performed
    pub gc_collections: u64,
}

/// Configuration for REAM runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReamConfig {
    /// Maximum number of processes
    pub max_processes: usize,
    /// Scheduler quantum in microseconds
    pub scheduler_quantum: u64,
    /// Maximum message queue size per process
    pub max_message_queue_size: usize,
    /// GC threshold in bytes
    pub gc_threshold: usize,
    /// Enable JIT compilation
    pub enable_jit: bool,
    /// JIT optimization level
    pub jit_opt_level: u8,
}

impl Default for ReamConfig {
    fn default() -> Self {
        ReamConfig {
            max_processes: 1_000_000,
            scheduler_quantum: 1000, // 1ms
            max_message_queue_size: 10_000,
            gc_threshold: 64 * 1024 * 1024, // 64MB
            enable_jit: true,
            jit_opt_level: 2,
        }
    }
}

// ============================================================================
// PRODUCTION-GRADE TYPES FOR FAULT TOLERANCE, STM, AND WASM
// ============================================================================

/// Isolation levels for process fault boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Single actor isolation
    Process,
    /// Actor pool isolation
    Pool,
    /// System-wide isolation
    System,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        IsolationLevel::Process
    }
}

/// Fault boundary definition for process isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultBoundary {
    /// Memory range allocated to this fault domain
    pub memory_range: Range<u64>,
    /// Maximum instructions before forced yield
    pub instruction_limit: u64,
    /// Maximum messages in mailbox
    pub message_quota: u32,
    /// Isolation level for this boundary
    pub isolation_level: IsolationLevel,
}

impl Default for FaultBoundary {
    fn default() -> Self {
        FaultBoundary {
            memory_range: 0..10_485_760, // 10MB default
            instruction_limit: 1_000_000,
            message_quota: 1000,
            isolation_level: IsolationLevel::Process,
        }
    }
}

/// Execution bounds for preventing infinite loops and resource exhaustion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExecutionBounds {
    /// Maximum instructions before termination
    pub instruction_limit: u64,
    /// Maximum memory usage in bytes
    pub memory_limit: u64,
    /// Maximum messages that can be sent
    pub message_limit: u64,
}

impl Default for ExecutionBounds {
    fn default() -> Self {
        ExecutionBounds {
            instruction_limit: 1_000_000,
            memory_limit: 10 * 1024 * 1024, // 10MB
            message_limit: 1000,
        }
    }
}

/// STM (Software Transactional Memory) effect grades
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StmGrade {
    /// Read-only transactions
    ReadOnly,
    /// Read-write transactions
    ReadWrite,
    /// Append-only transactions (for logs)
    AppendOnly,
}

impl Default for StmGrade {
    fn default() -> Self {
        StmGrade::ReadOnly
    }
}

/// Versioned data for STM transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versioned<T> {
    /// Version number for conflict detection
    pub version: u64,
    /// The actual payload
    pub payload: T,
}

impl<T> Versioned<T> {
    /// Create a new versioned value
    pub fn new(version: u64, payload: T) -> Self {
        Versioned { version, payload }
    }
}

/// WebAssembly module representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    /// Compiled WASM bytecode
    pub bytecode: Vec<u8>,
    /// Exported functions and their indices
    pub exports: HashMap<String, u32>,
    /// Imported functions and their indices
    pub imports: HashMap<String, u32>,
    /// Module metadata
    pub metadata: WasmMetadata,
}

/// WebAssembly module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmMetadata {
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Source language
    pub source_language: String,
    /// Compilation timestamp
    pub compiled_at: u64,
}

/// Memory layout for WASM modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLayout {
    /// Process heap range
    pub process_heap: Range<u32>,
    /// Mailbox memory range
    pub mailbox: Range<u32>,
    /// Stack memory range
    pub stack: Range<u32>,
    /// Global variables range
    pub globals: Range<u32>,
}

impl MemoryLayout {
    /// Create a new memory layout with specified heap and stack sizes
    pub fn new(heap_size: u32, stack_size: u32) -> Self {
        let heap_start = 0;
        let heap_end = heap_size;
        let stack_start = heap_end;
        let stack_end = stack_start + stack_size;
        let mailbox_start = stack_end;
        let mailbox_end = mailbox_start + 64 * 1024; // 64KB for mailbox
        let globals_start = mailbox_end;
        let globals_end = globals_start + 4 * 1024; // 4KB for globals

        MemoryLayout {
            process_heap: heap_start..heap_end,
            stack: stack_start..stack_end,
            mailbox: mailbox_start..mailbox_end,
            globals: globals_start..globals_end,
        }
    }
}
