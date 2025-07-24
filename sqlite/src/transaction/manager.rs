use crate::error::{SqlError, SqlResult};
use crate::transaction::{Transaction, TransactionConfig, TransactionState, TransactionStats, IsolationLevel};
use std::collections::HashMap;
use uuid::Uuid;

/// Transaction manager
#[derive(Debug)]
pub struct TransactionManager {
    active_transactions: HashMap<Uuid, Transaction>,
    config: TransactionConfig,
    stats: TransactionStats,
}

impl TransactionManager {
    pub fn new(config: TransactionConfig) -> Self {
        TransactionManager {
            active_transactions: HashMap::new(),
            config,
            stats: TransactionStats::default(),
        }
    }

    pub async fn begin_transaction(&mut self) -> SqlResult<Transaction> {
        if self.active_transactions.len() >= self.config.max_active_transactions {
            return Err(SqlError::transaction_error("Too many active transactions"));
        }

        let transaction = Transaction::new(self.config.default_isolation_level);
        let id = transaction.id;
        
        self.active_transactions.insert(id, transaction.clone());
        self.stats.active_transactions += 1;
        
        Ok(transaction)
    }

    pub async fn commit_transaction(&mut self, id: Uuid) -> SqlResult<()> {
        if let Some(mut transaction) = self.active_transactions.remove(&id) {
            transaction.commit();
            self.stats.update_with_transaction(&transaction);
            self.stats.active_transactions = self.stats.active_transactions.saturating_sub(1);
            Ok(())
        } else {
            Err(SqlError::transaction_error("Transaction not found"))
        }
    }

    pub async fn rollback_transaction(&mut self, id: Uuid) -> SqlResult<()> {
        if let Some(mut transaction) = self.active_transactions.remove(&id) {
            transaction.abort();
            self.stats.update_with_transaction(&transaction);
            self.stats.active_transactions = self.stats.active_transactions.saturating_sub(1);
            Ok(())
        } else {
            Err(SqlError::transaction_error("Transaction not found"))
        }
    }

    pub async fn cleanup_completed_transactions(&mut self) -> SqlResult<()> {
        // Remove timed out transactions
        let timeout = self.config.transaction_timeout;
        let timed_out: Vec<Uuid> = self.active_transactions
            .iter()
            .filter(|(_, tx)| tx.is_timed_out(timeout))
            .map(|(id, _)| *id)
            .collect();

        for id in timed_out {
            self.rollback_transaction(id).await?;
        }

        Ok(())
    }

    pub async fn commit_all_active_transactions(&mut self) -> SqlResult<()> {
        let active_ids: Vec<Uuid> = self.active_transactions.keys().cloned().collect();
        for id in active_ids {
            self.commit_transaction(id).await?;
        }
        Ok(())
    }

    pub fn get_statistics(&self) -> TransactionStats {
        self.stats.clone()
    }
}

/// SQL Transaction following free monad pattern
#[derive(Debug, Clone)]
pub struct SqlTransaction {
    pub id: Uuid,
    pub commands: Vec<crate::transaction::SqlCommand>,
    pub checkpoint: Option<crate::types::DatabaseState>,
}

impl SqlTransaction {
    pub fn pure() -> Self {
        SqlTransaction {
            id: Uuid::new_v4(),
            commands: Vec::new(),
            checkpoint: None,
        }
    }

    pub fn then<F>(self, f: F) -> Self 
    where 
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    pub fn add_command(mut self, command: crate::transaction::SqlCommand) -> Self {
        self.commands.push(command);
        self
    }
}
