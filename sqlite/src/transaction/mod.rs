pub mod manager;
pub mod wal;
pub mod command;

pub use manager::{TransactionManager, SqlTransaction};
pub use wal::{WalCoalgebra, WalEntry};
pub use command::SqlCommand;

use crate::error::{SqlError, SqlResult};
use crate::types::DatabaseState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Transaction state following algebraic patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionState {
    Active,
    Committed,
    Aborted,
    Preparing, // For two-phase commit
}

/// Transaction isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// Transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub state: TransactionState,
    pub isolation_level: IsolationLevel,
    pub commands: Vec<SqlCommand>,
    pub checkpoint: Option<DatabaseState>,
    pub start_time: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(isolation_level: IsolationLevel) -> Self {
        let now = std::time::SystemTime::now();
        Transaction {
            id: Uuid::new_v4(),
            state: TransactionState::Active,
            isolation_level,
            commands: Vec::new(),
            checkpoint: None,
            start_time: now,
            last_activity: now,
        }
    }

    /// Add a command to the transaction
    pub fn add_command(&mut self, command: SqlCommand) {
        self.commands.push(command);
        self.last_activity = std::time::SystemTime::now();
    }

    /// Create a checkpoint (memento pattern)
    pub fn create_checkpoint(&mut self, state: DatabaseState) {
        self.checkpoint = Some(state);
    }

    /// Get checkpoint state
    pub fn get_checkpoint(&self) -> Option<&DatabaseState> {
        self.checkpoint.as_ref()
    }

    /// Check if transaction is active
    pub fn is_active(&self) -> bool {
        self.state == TransactionState::Active
    }

    /// Check if transaction is committed
    pub fn is_committed(&self) -> bool {
        self.state == TransactionState::Committed
    }

    /// Check if transaction is aborted
    pub fn is_aborted(&self) -> bool {
        self.state == TransactionState::Aborted
    }

    /// Get transaction duration
    pub fn duration(&self) -> std::time::Duration {
        self.last_activity
            .duration_since(self.start_time)
            .unwrap_or_default()
    }

    /// Check if transaction has timed out
    pub fn is_timed_out(&self, timeout: std::time::Duration) -> bool {
        self.duration() > timeout
    }

    /// Mark transaction as committed
    pub fn commit(&mut self) {
        self.state = TransactionState::Committed;
        self.last_activity = std::time::SystemTime::now();
    }

    /// Mark transaction as aborted
    pub fn abort(&mut self) {
        self.state = TransactionState::Aborted;
        self.last_activity = std::time::SystemTime::now();
    }

    /// Get command count
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Check if transaction is read-only
    pub fn is_read_only(&self) -> bool {
        self.commands.iter().all(|cmd| cmd.is_read_only())
    }

    /// Get all table names affected by this transaction
    pub fn affected_tables(&self) -> Vec<String> {
        let mut tables = Vec::new();
        for command in &self.commands {
            if let Some(table) = command.affected_table() {
                if !tables.contains(&table) {
                    tables.push(table);
                }
            }
        }
        tables
    }
}

/// Transaction statistics
#[derive(Debug, Clone, Default)]
pub struct TransactionStats {
    pub total_transactions: u64,
    pub active_transactions: u64,
    pub committed_transactions: u64,
    pub aborted_transactions: u64,
    pub average_duration: std::time::Duration,
    pub longest_transaction: std::time::Duration,
}

impl TransactionStats {
    /// Update statistics with a completed transaction
    pub fn update_with_transaction(&mut self, transaction: &Transaction) {
        self.total_transactions += 1;
        
        match transaction.state {
            TransactionState::Committed => self.committed_transactions += 1,
            TransactionState::Aborted => self.aborted_transactions += 1,
            TransactionState::Active => self.active_transactions += 1,
            _ => {}
        }

        let duration = transaction.duration();
        if duration > self.longest_transaction {
            self.longest_transaction = duration;
        }

        // Update average duration (simplified)
        if self.total_transactions > 0 {
            let total_duration = self.average_duration * (self.total_transactions - 1) as u32 + duration;
            self.average_duration = total_duration / self.total_transactions as u32;
        }
    }

    /// Get commit rate
    pub fn commit_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            self.committed_transactions as f64 / self.total_transactions as f64
        }
    }

    /// Get abort rate
    pub fn abort_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            self.aborted_transactions as f64 / self.total_transactions as f64
        }
    }
}

/// Transaction configuration
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub default_isolation_level: IsolationLevel,
    pub transaction_timeout: std::time::Duration,
    pub max_active_transactions: usize,
    pub enable_wal: bool,
    pub wal_sync_mode: WalSyncMode,
    pub checkpoint_interval: std::time::Duration,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        TransactionConfig {
            default_isolation_level: IsolationLevel::ReadCommitted,
            transaction_timeout: std::time::Duration::from_secs(300), // 5 minutes
            max_active_transactions: 1000,
            enable_wal: true,
            wal_sync_mode: WalSyncMode::Normal,
            checkpoint_interval: std::time::Duration::from_secs(60), // 1 minute
        }
    }
}

/// WAL synchronization modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalSyncMode {
    /// No synchronization (fastest, least safe)
    Off,
    /// Normal synchronization
    Normal,
    /// Full synchronization (slowest, safest)
    Full,
}

/// Lock types for concurrency control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Shared,
    Exclusive,
    IntentShared,
    IntentExclusive,
    SharedIntentExclusive,
}

/// Lock information
#[derive(Debug, Clone)]
pub struct Lock {
    pub transaction_id: Uuid,
    pub resource: String,
    pub lock_type: LockType,
    pub acquired_at: std::time::SystemTime,
}

impl Lock {
    pub fn new(transaction_id: Uuid, resource: String, lock_type: LockType) -> Self {
        Lock {
            transaction_id,
            resource,
            lock_type,
            acquired_at: std::time::SystemTime::now(),
        }
    }

    /// Check if this lock conflicts with another lock type
    pub fn conflicts_with(&self, other_type: LockType) -> bool {
        use LockType::*;
        match (self.lock_type, other_type) {
            (Exclusive, _) | (_, Exclusive) => true,
            (Shared, Shared) => false,
            (IntentShared, IntentShared) => false,
            (IntentShared, Shared) => false,
            (Shared, IntentShared) => false,
            _ => true,
        }
    }

    /// Get lock duration
    pub fn duration(&self) -> std::time::Duration {
        std::time::SystemTime::now()
            .duration_since(self.acquired_at)
            .unwrap_or_default()
    }
}

/// Deadlock detection result
#[derive(Debug, Clone)]
pub enum DeadlockResult {
    NoDeadlock,
    DeadlockDetected {
        cycle: Vec<Uuid>, // Transaction IDs in the cycle
        victim: Uuid,     // Transaction to abort
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new(IsolationLevel::ReadCommitted);
        
        assert!(tx.is_active());
        assert!(!tx.is_committed());
        assert!(!tx.is_aborted());
        assert_eq!(tx.isolation_level, IsolationLevel::ReadCommitted);
        assert_eq!(tx.command_count(), 0);
        assert!(tx.is_read_only());
    }

    #[test]
    fn test_transaction_state_transitions() {
        let mut tx = Transaction::new(IsolationLevel::ReadCommitted);
        
        // Start active
        assert!(tx.is_active());
        
        // Commit
        tx.commit();
        assert!(tx.is_committed());
        assert!(!tx.is_active());
        
        // Create new transaction and abort
        let mut tx2 = Transaction::new(IsolationLevel::ReadCommitted);
        tx2.abort();
        assert!(tx2.is_aborted());
        assert!(!tx2.is_active());
    }

    #[test]
    fn test_lock_conflicts() {
        let tx_id = Uuid::new_v4();
        let shared_lock = Lock::new(tx_id, "table1".to_string(), LockType::Shared);
        let exclusive_lock = Lock::new(tx_id, "table1".to_string(), LockType::Exclusive);
        
        // Shared locks don't conflict with each other
        assert!(!shared_lock.conflicts_with(LockType::Shared));
        
        // Exclusive locks conflict with everything
        assert!(exclusive_lock.conflicts_with(LockType::Shared));
        assert!(exclusive_lock.conflicts_with(LockType::Exclusive));
        
        // Shared conflicts with exclusive
        assert!(shared_lock.conflicts_with(LockType::Exclusive));
    }

    #[test]
    fn test_transaction_stats() {
        let mut stats = TransactionStats::default();
        
        let mut tx1 = Transaction::new(IsolationLevel::ReadCommitted);
        tx1.commit();
        stats.update_with_transaction(&tx1);
        
        let mut tx2 = Transaction::new(IsolationLevel::ReadCommitted);
        tx2.abort();
        stats.update_with_transaction(&tx2);
        
        assert_eq!(stats.total_transactions, 2);
        assert_eq!(stats.committed_transactions, 1);
        assert_eq!(stats.aborted_transactions, 1);
        assert_eq!(stats.commit_rate(), 0.5);
        assert_eq!(stats.abort_rate(), 0.5);
    }
}
