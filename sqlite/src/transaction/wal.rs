use crate::error::{SqlError, SqlResult};
use crate::transaction::SqlCommand;
use crate::types::DatabaseState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WAL entry for write-ahead logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub lsn: u64, // Log Sequence Number
    pub transaction_id: Uuid,
    pub operation: SqlCommand,
    pub checksum: u32,
    pub timestamp: std::time::SystemTime,
}

impl WalEntry {
    pub fn new(lsn: u64, transaction_id: Uuid, operation: SqlCommand) -> Self {
        let checksum = Self::calculate_checksum(&operation);
        WalEntry {
            lsn,
            transaction_id,
            operation,
            checksum,
            timestamp: std::time::SystemTime::now(),
        }
    }

    fn calculate_checksum(operation: &SqlCommand) -> u32 {
        // Simplified checksum calculation
        operation.estimated_size() as u32
    }

    pub fn verify_checksum(&self) -> bool {
        self.checksum == Self::calculate_checksum(&self.operation)
    }
}

/// WAL coalgebra for event sourcing and broadcasting
pub struct WalCoalgebra {
    entries: Vec<WalEntry>,
    observers: Vec<Box<dyn Fn(&WalEntry) + Send + Sync>>,
    current_lsn: u64,
}

impl WalCoalgebra {
    pub fn new() -> Self {
        WalCoalgebra {
            entries: Vec::new(),
            observers: Vec::new(),
            current_lsn: 0,
        }
    }

    /// Coalgebraic observation: add log entry
    pub fn append_entry(&mut self, entry: WalEntry) {
        self.entries.push(entry.clone());
        self.current_lsn = entry.lsn;

        // Broadcast to observers
        for observer in &self.observers {
            observer(&entry);
        }
    }

    /// Register observer (coalgebraic observation)
    pub fn register_observer<F>(&mut self, observer: F)
    where
        F: Fn(&WalEntry) + Send + Sync + 'static,
    {
        self.observers.push(Box::new(observer));
    }

    /// Restore from WAL (coalgebraic state restoration)
    pub fn restore_database(&self, db: &mut DatabaseState) -> SqlResult<()> {
        for entry in &self.entries {
            // Apply operation to database state
            // This is simplified - in a real implementation,
            // this would replay the actual operations
        }
        Ok(())
    }

    pub fn get_entries_since(&self, lsn: u64) -> Vec<&WalEntry> {
        self.entries.iter().filter(|entry| entry.lsn > lsn).collect()
    }

    pub fn truncate_before(&mut self, lsn: u64) {
        self.entries.retain(|entry| entry.lsn >= lsn);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for WalCoalgebra {
    fn default() -> Self {
        Self::new()
    }
}
