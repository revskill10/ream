//! STM (Software Transactional Memory) mailbox implementation
//!
//! Implements zero-copy message passing with transactional memory
//! for concurrent actor communication with conflict resolution.

use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};

use crate::types::{Pid, Versioned};
use crate::error::{StmError, StmResult};

/// Transaction context for STM operations
#[derive(Clone)]
pub struct TxContext {
    /// Transaction ID
    pub tx_id: u64,
    /// Read set - variables read in this transaction
    pub read_set: HashMap<u64, u64>, // variable_id -> version
    /// Write set - variables written in this transaction
    pub write_set: HashMap<u64, Vec<u8>>, // variable_id -> new_value
    /// Transaction start time
    pub start_time: Instant,
    /// Transaction timeout
    pub timeout: Duration,
}

impl TxContext {
    /// Create a new transaction context
    pub fn new(tx_id: u64, timeout: Duration) -> Self {
        TxContext {
            tx_id,
            read_set: HashMap::new(),
            write_set: HashMap::new(),
            start_time: Instant::now(),
            timeout,
        }
    }
    
    /// Check if transaction has timed out
    pub fn is_timed_out(&self) -> bool {
        self.start_time.elapsed() > self.timeout
    }
    
    /// Record a read operation
    pub fn record_read(&mut self, var_id: u64, version: u64) {
        self.read_set.insert(var_id, version);
    }
    
    /// Record a write operation
    pub fn record_write(&mut self, var_id: u64, value: Vec<u8>) {
        self.write_set.insert(var_id, value);
    }
}

/// Transaction computation
pub struct Tx<T> {
    /// The computation to execute
    computation: Box<dyn Fn(&mut TxContext) -> StmResult<T> + Send + Sync>,
}

impl<T> Tx<T> {
    /// Create a new transaction
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut TxContext) -> StmResult<T> + Send + Sync + 'static,
    {
        Tx {
            computation: Box::new(f),
        }
    }
    
    /// Run the transaction
    pub fn run(&self, ctx: &mut TxContext) -> StmResult<T> {
        (self.computation)(ctx)
    }
}

/// Transactional mailbox with versioned message log
pub struct TMailbox<M> {
    /// Message log with versions
    log: RwLock<VecDeque<Versioned<M>>>,
    /// Current tail version
    tail: AtomicU64,
    /// Maximum log size before compaction
    max_log_size: usize,
    /// Compaction threshold
    compaction_threshold: usize,
}

impl<M: Clone + Send + Sync> TMailbox<M> {
    /// Create a new transactional mailbox
    pub fn new(max_log_size: usize) -> Self {
        TMailbox {
            log: RwLock::new(VecDeque::with_capacity(max_log_size)),
            tail: AtomicU64::new(0),
            max_log_size,
            compaction_threshold: max_log_size / 2,
        }
    }
    
    /// Read all messages (non-transactional for now)
    pub fn read_all(&self) -> Vec<Versioned<M>> {
        let log = self.log.read().unwrap();
        log.iter().cloned().collect()
    }

    /// Append a message (non-transactional for now)
    pub fn append(&self, msg: M) -> StmResult<u64> {
        let version = self.tail.fetch_add(1, Ordering::Relaxed);
        let versioned_msg = Versioned::new(version, msg);

        {
            let mut log = self.log.write().unwrap();
            log.push_back(versioned_msg);

            // Check if compaction is needed
            if log.len() > self.max_log_size {
                return Err(StmError::ResourceExhausted);
            }
        }

        Ok(version)
    }

    /// Read messages from a specific version
    pub fn read_from_version(&self, from_version: u64) -> Vec<Versioned<M>> {
        let log = self.log.read().unwrap();
        log.iter()
            .filter(|v| v.version >= from_version)
            .cloned()
            .collect()
    }
    
    /// Get the current tail version
    pub fn current_version(&self) -> u64 {
        self.tail.load(Ordering::Relaxed)
    }
    
    /// Compact the log by removing old messages
    pub fn compact(&self, keep_versions: u64) -> StmResult<usize> {
        let mut log = self.log.write().unwrap();
        let current_version = self.tail.load(Ordering::Relaxed);
        let cutoff_version = current_version.saturating_sub(keep_versions);
        
        let original_len = log.len();
        log.retain(|v| v.version >= cutoff_version);
        let removed = original_len - log.len();
        
        Ok(removed)
    }
}

/// STM engine for managing transactions
pub struct StmEngine {
    /// Global transaction counter
    tx_counter: AtomicU64,
    /// Active transactions
    active_transactions: RwLock<HashMap<u64, Arc<Mutex<TxContext>>>>,
    /// Mailboxes for each process
    mailboxes: RwLock<HashMap<Pid, Arc<TMailbox<Vec<u8>>>>>,
    /// Default transaction timeout
    default_timeout: Duration,
    /// Maximum retry attempts
    max_retries: u32,
}

impl StmEngine {
    /// Create a new STM engine
    pub fn new() -> Self {
        StmEngine {
            tx_counter: AtomicU64::new(1),
            active_transactions: RwLock::new(HashMap::new()),
            mailboxes: RwLock::new(HashMap::new()),
            default_timeout: Duration::from_secs(5),
            max_retries: 10,
        }
    }
    
    /// Create a mailbox for a process
    pub fn create_mailbox(&self, pid: Pid) -> StmResult<()> {
        let mut mailboxes = self.mailboxes.write().unwrap();
        if mailboxes.contains_key(&pid) {
            return Err(StmError::InvalidState("Mailbox already exists".to_string()));
        }
        
        let mailbox = Arc::new(TMailbox::new(10000)); // 10K message limit
        mailboxes.insert(pid, mailbox);
        Ok(())
    }
    
    /// Remove a mailbox for a process
    pub fn remove_mailbox(&self, pid: Pid) -> StmResult<()> {
        let mut mailboxes = self.mailboxes.write().unwrap();
        mailboxes.remove(&pid);
        Ok(())
    }
    
    /// Execute a transaction with automatic retry
    pub fn execute_transaction<T>(&self, tx: Tx<T>) -> StmResult<T>
    where
        T: Clone + Send + Sync,
    {
        let mut retries = 0;
        
        loop {
            let tx_id = self.tx_counter.fetch_add(1, Ordering::Relaxed);
            let mut ctx = TxContext::new(tx_id, self.default_timeout);
            
            // Register transaction
            {
                let mut active = self.active_transactions.write().unwrap();
                active.insert(tx_id, Arc::new(Mutex::new(ctx.clone())));
            }
            
            // Execute transaction
            let result = tx.run(&mut ctx);
            
            // Unregister transaction
            {
                let mut active = self.active_transactions.write().unwrap();
                active.remove(&tx_id);
            }
            
            match result {
                Ok(value) => {
                    // Transaction succeeded, validate and commit
                    if self.validate_transaction(&ctx)? {
                        return Ok(value);
                    } else {
                        // Validation failed, retry
                        retries += 1;
                        if retries >= self.max_retries {
                            return Err(StmError::RetryLimitExceeded);
                        }
                        continue;
                    }
                }
                Err(StmError::Conflict) => {
                    // Conflict detected, retry
                    retries += 1;
                    if retries >= self.max_retries {
                        return Err(StmError::RetryLimitExceeded);
                    }
                    continue;
                }
                Err(e) => {
                    // Other error, don't retry
                    return Err(e);
                }
            }
        }
    }
    
    /// Validate a transaction for conflicts
    fn validate_transaction(&self, ctx: &TxContext) -> StmResult<bool> {
        // Check if transaction has timed out
        if ctx.is_timed_out() {
            return Err(StmError::Timeout);
        }
        
        // For now, we'll implement a simple validation
        // In a full implementation, this would check for conflicts
        // with other concurrent transactions
        Ok(true)
    }
    
    /// Send a message using STM
    pub fn stm_send_message(&self, _from: Pid, to: Pid, message: Vec<u8>) -> StmResult<u64> {
        let mailboxes = self.mailboxes.read().unwrap();
        let mailbox = mailboxes.get(&to)
            .ok_or_else(|| StmError::InvalidState("Target mailbox not found".to_string()))?;

        mailbox.append(message)
    }

    /// Receive messages using STM
    pub fn stm_receive_messages(&self, pid: Pid, from_version: u64) -> StmResult<Vec<Versioned<Vec<u8>>>> {
        let mailboxes = self.mailboxes.read().unwrap();
        let mailbox = mailboxes.get(&pid)
            .ok_or_else(|| StmError::InvalidState("Mailbox not found".to_string()))?;

        Ok(mailbox.read_from_version(from_version))
    }
    
    /// Compact mailbox logs
    pub fn compact_mailbox(&self, pid: Pid, keep_versions: u64) -> StmResult<usize> {
        let mailboxes = self.mailboxes.read().unwrap();
        let mailbox = mailboxes.get(&pid)
            .ok_or_else(|| StmError::InvalidState("Mailbox not found".to_string()))?;
        
        mailbox.compact(keep_versions)
    }
    
    /// Get statistics about the STM engine
    pub fn get_stats(&self) -> StmStats {
        let active_count = self.active_transactions.read().unwrap().len();
        let mailbox_count = self.mailboxes.read().unwrap().len();
        
        StmStats {
            active_transactions: active_count,
            total_mailboxes: mailbox_count,
            next_tx_id: self.tx_counter.load(Ordering::Relaxed),
        }
    }
}

impl Default for StmEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// STM engine statistics
#[derive(Debug, Clone)]
pub struct StmStats {
    /// Number of active transactions
    pub active_transactions: usize,
    /// Total number of mailboxes
    pub total_mailboxes: usize,
    /// Next transaction ID
    pub next_tx_id: u64,
}

/// Compaction engine for managing memory usage
pub struct CompactionEngine {
    /// Compaction threshold (number of versions to keep)
    threshold: u64,
    /// Compaction strategy
    strategy: CompactionStrategy,
}

/// Compaction strategies
#[derive(Debug, Clone)]
pub enum CompactionStrategy {
    /// Keep last N versions
    LastN(u64),
    /// Keep versions within time window
    TimeWindow(Duration),
    /// Keep only reachable versions
    Reachability,
}

impl CompactionEngine {
    /// Create a new compaction engine
    pub fn new(strategy: CompactionStrategy) -> Self {
        let threshold = match &strategy {
            CompactionStrategy::LastN(n) => *n,
            CompactionStrategy::TimeWindow(_) => 1000, // Default
            CompactionStrategy::Reachability => 100,   // Default
        };
        
        CompactionEngine {
            threshold,
            strategy,
        }
    }
    
    /// Run compaction on a mailbox
    pub fn compact_mailbox<M: Clone + Send + Sync>(&self, mailbox: &TMailbox<M>) -> StmResult<usize> {
        match &self.strategy {
            CompactionStrategy::LastN(n) => mailbox.compact(*n),
            CompactionStrategy::TimeWindow(_) => {
                // For time-based compaction, we'd need timestamps
                // For now, use threshold
                mailbox.compact(self.threshold)
            }
            CompactionStrategy::Reachability => {
                // For reachability-based compaction, we'd analyze references
                // For now, use threshold
                mailbox.compact(self.threshold)
            }
        }
    }
}
