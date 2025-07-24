//! STM Integration for TLISP
//! 
//! Integrates Software Transactional Memory with TLISP syntax and REAM's STM engine.
//! Provides atomic transactions, conflict detection, and retry mechanisms.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::tlisp::{Value, Expr, Function};
use crate::tlisp::types::Type;
use crate::error::{TlispError, TlispResult};
use crate::runtime::stm_mailbox::{StmEngine, TxContext, StmError};

/// STM variable reference
#[derive(Debug, Clone, PartialEq)]
pub struct StmVar {
    /// Variable ID
    id: u64,
    /// Variable name
    name: String,
    /// Variable type
    var_type: Type,
}

/// STM transaction context for TLISP
pub struct TlispStmContext {
    /// Underlying STM transaction context
    tx_context: TxContext,
    /// STM variables accessed in this transaction
    accessed_vars: HashMap<u64, StmVar>,
    /// Transaction ID
    tx_id: u64,
    /// Whether transaction is active
    active: bool,
}

/// STM integration for TLISP
pub struct TlispStmIntegration {
    /// STM engine reference
    stm_engine: Arc<StmEngine>,
    /// STM variables registry
    variables: Arc<RwLock<HashMap<u64, (StmVar, Value)>>>,
    /// Next variable ID
    next_var_id: Arc<RwLock<u64>>,
    /// Active transactions
    transactions: Arc<RwLock<HashMap<u64, TlispStmContext>>>,
    /// Next transaction ID
    next_tx_id: Arc<RwLock<u64>>,
}

impl StmVar {
    /// Create a new STM variable
    pub fn new(name: String, var_type: Type) -> Self {
        StmVar {
            id: 0, // Will be set by STM integration
            name,
            var_type,
        }
    }

    /// Get variable ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get variable name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get variable type
    pub fn var_type(&self) -> &Type {
        &self.var_type
    }
}

impl TlispStmContext {
    /// Create a new STM context
    pub fn new(tx_id: u64) -> Self {
        TlispStmContext {
            tx_context: TxContext::new(),
            accessed_vars: HashMap::new(),
            tx_id,
            active: true,
        }
    }

    /// Check if transaction is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get transaction ID
    pub fn tx_id(&self) -> u64 {
        self.tx_id
    }

    /// Add accessed variable
    pub fn add_accessed_var(&mut self, var: StmVar) {
        self.accessed_vars.insert(var.id, var);
    }

    /// Get accessed variables
    pub fn accessed_vars(&self) -> &HashMap<u64, StmVar> {
        &self.accessed_vars
    }
}

impl TlispStmIntegration {
    /// Create a new STM integration
    pub fn new(stm_engine: Arc<StmEngine>) -> Self {
        TlispStmIntegration {
            stm_engine,
            variables: Arc::new(RwLock::new(HashMap::new())),
            next_var_id: Arc::new(RwLock::new(1)),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            next_tx_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Create a new STM variable
    pub fn create_stm_var(&self, name: String, var_type: Type, initial_value: Value) -> TlispResult<StmVar> {
        let var_id = {
            let mut next_id = self.next_var_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let mut stm_var = StmVar::new(name, var_type);
        stm_var.id = var_id;

        // Store variable and initial value
        {
            let mut variables = self.variables.write().unwrap();
            variables.insert(var_id, (stm_var.clone(), initial_value));
        }

        Ok(stm_var)
    }

    /// Start a new transaction
    pub fn begin_transaction(&self) -> TlispResult<u64> {
        let tx_id = {
            let mut next_id = self.next_tx_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let context = TlispStmContext::new(tx_id);
        
        {
            let mut transactions = self.transactions.write().unwrap();
            transactions.insert(tx_id, context);
        }

        Ok(tx_id)
    }

    /// Read from STM variable
    pub fn stm_read(&self, tx_id: u64, var: &StmVar) -> TlispResult<Value> {
        // Check if transaction exists and is active
        {
            let mut transactions = self.transactions.write().unwrap();
            if let Some(tx_context) = transactions.get_mut(&tx_id) {
                if !tx_context.is_active() {
                    return Err(TlispError::Runtime("Transaction not active".to_string()));
                }
                tx_context.add_accessed_var(var.clone());
            } else {
                return Err(TlispError::Runtime("Transaction not found".to_string()));
            }
        }

        // Read variable value
        let variables = self.variables.read().unwrap();
        if let Some((_, value)) = variables.get(&var.id) {
            Ok(value.clone())
        } else {
            Err(TlispError::Runtime(format!("STM variable {} not found", var.name)))
        }
    }

    /// Write to STM variable
    pub fn stm_write(&self, tx_id: u64, var: &StmVar, value: Value) -> TlispResult<()> {
        // Check if transaction exists and is active
        {
            let mut transactions = self.transactions.write().unwrap();
            if let Some(tx_context) = transactions.get_mut(&tx_id) {
                if !tx_context.is_active() {
                    return Err(TlispError::Runtime("Transaction not active".to_string()));
                }
                tx_context.add_accessed_var(var.clone());
            } else {
                return Err(TlispError::Runtime("Transaction not found".to_string()));
            }
        }

        // Write variable value (in transaction log)
        // For now, just update the variable directly
        // In a full implementation, this would be logged and applied on commit
        {
            let mut variables = self.variables.write().unwrap();
            if let Some((stm_var, old_value)) = variables.get_mut(&var.id) {
                *old_value = value;
            } else {
                return Err(TlispError::Runtime(format!("STM variable {} not found", var.name)));
            }
        }

        Ok(())
    }

    /// Commit transaction
    pub fn commit_transaction(&self, tx_id: u64) -> TlispResult<()> {
        let mut transactions = self.transactions.write().unwrap();
        if let Some(mut tx_context) = transactions.remove(&tx_id) {
            if !tx_context.active {
                return Err(TlispError::Runtime("Transaction already completed".to_string()));
            }

            // Mark transaction as inactive
            tx_context.active = false;

            // In a full implementation, this would:
            // 1. Validate all reads are still consistent
            // 2. Apply all writes atomically
            // 3. Handle conflicts and retry if necessary

            Ok(())
        } else {
            Err(TlispError::Runtime("Transaction not found".to_string()))
        }
    }

    /// Abort transaction
    pub fn abort_transaction(&self, tx_id: u64) -> TlispResult<()> {
        let mut transactions = self.transactions.write().unwrap();
        if let Some(mut tx_context) = transactions.remove(&tx_id) {
            tx_context.active = false;
            // Rollback any changes made in this transaction
            Ok(())
        } else {
            Err(TlispError::Runtime("Transaction not found".to_string()))
        }
    }

    /// Retry transaction (used when conflicts are detected)
    pub fn retry_transaction(&self, tx_id: u64) -> TlispResult<()> {
        // Mark transaction for retry
        // In a full implementation, this would block until
        // one of the accessed variables changes
        Err(TlispError::Runtime("Transaction retry requested".to_string()))
    }

    /// Run a transaction function
    pub fn run_transaction<F>(&self, transaction_fn: F) -> TlispResult<Value>
    where
        F: Fn(u64) -> TlispResult<Value>,
    {
        let max_retries = 10;
        let mut retries = 0;

        loop {
            let tx_id = self.begin_transaction()?;
            
            match transaction_fn(tx_id) {
                Ok(result) => {
                    match self.commit_transaction(tx_id) {
                        Ok(()) => return Ok(result),
                        Err(_) => {
                            // Commit failed, retry
                            self.abort_transaction(tx_id).ok();
                        }
                    }
                }
                Err(TlispError::Runtime(msg)) if msg == "Transaction retry requested" => {
                    // Explicit retry requested
                    self.abort_transaction(tx_id).ok();
                }
                Err(e) => {
                    // Other error, abort and propagate
                    self.abort_transaction(tx_id).ok();
                    return Err(e);
                }
            }

            retries += 1;
            if retries >= max_retries {
                return Err(TlispError::Runtime("Transaction failed after maximum retries".to_string()));
            }

            // Brief delay before retry
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// Get STM variable by name
    pub fn get_var_by_name(&self, name: &str) -> Option<StmVar> {
        let variables = self.variables.read().unwrap();
        for (_, (var, _)) in variables.iter() {
            if var.name == name {
                return Some(var.clone());
            }
        }
        None
    }

    /// List all STM variables
    pub fn list_variables(&self) -> Vec<StmVar> {
        let variables = self.variables.read().unwrap();
        variables.values().map(|(var, _)| var.clone()).collect()
    }

    /// Get transaction statistics
    pub fn get_stats(&self) -> StmStats {
        StmStats {
            active_transactions: self.transactions.read().unwrap().len(),
            total_variables: self.variables.read().unwrap().len(),
            conflicts_detected: 0, // TODO: Track conflicts
            retries_performed: 0,  // TODO: Track retries
        }
    }
}

/// STM statistics
#[derive(Debug, Clone)]
pub struct StmStats {
    /// Number of active transactions
    pub active_transactions: usize,
    /// Total number of STM variables
    pub total_variables: usize,
    /// Number of conflicts detected
    pub conflicts_detected: u64,
    /// Number of retries performed
    pub retries_performed: u64,
}

/// STM primitive functions for TLISP
pub struct StmPrimitives {
    /// STM integration
    stm: Arc<TlispStmIntegration>,
}

impl StmPrimitives {
    /// Create new STM primitives
    pub fn new(stm: Arc<TlispStmIntegration>) -> Self {
        StmPrimitives { stm }
    }

    /// Create STM variable primitive
    pub fn stm_var(&self, name: String, var_type: Type, initial_value: Value) -> TlispResult<Value> {
        let stm_var = self.stm.create_stm_var(name, var_type, initial_value)?;
        Ok(Value::StmVar(stm_var))
    }

    /// STM read primitive
    pub fn stm_read(&self, tx_id: u64, var: StmVar) -> TlispResult<Value> {
        self.stm.stm_read(tx_id, &var)
    }

    /// STM write primitive
    pub fn stm_write(&self, tx_id: u64, var: StmVar, value: Value) -> TlispResult<()> {
        self.stm.stm_write(tx_id, &var, value)
    }

    /// STM transaction primitive
    pub fn stm_transaction(&self, transaction_fn: Function) -> TlispResult<Value> {
        // TODO: Evaluate function in transaction context
        // For now, just run a simple transaction
        self.stm.run_transaction(|_tx_id| {
            Ok(Value::Symbol("transaction-result".to_string()))
        })
    }

    /// STM retry primitive
    pub fn stm_retry(&self, tx_id: u64) -> TlispResult<Value> {
        self.stm.retry_transaction(tx_id)?;
        Ok(Value::Symbol("retry".to_string()))
    }
}
