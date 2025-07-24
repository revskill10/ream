/// Catena-Mutate-X: Multi-root, nested, relation-aware mutations
/// 
/// This module implements an algebraic approach to GraphQL mutations that
/// compiles multiple mutation roots into a single SQL transaction with CTEs,
/// providing atomic rollback safety and input coalescing.

use std::collections::HashMap;
use serde_json::Value as JsonValue;
use crate::sqlite::types::{Value, DataType};
use crate::sqlite::error::{SqlError, SqlResult};
use crate::orm::schema::{TypeSafeSchema, Column, Table};

/// Core mutation algebra - initial algebra over effectful operations
#[derive(Debug, Clone)]
pub enum MutationF<A> {
    /// Create a new node in a table
    CreateNode {
        table: String,
        input: HashMap<String, JsonValue>,
        next: A,
    },
    
    /// Update an existing node
    UpdateNode {
        table: String,
        id: i64,
        patch: HashMap<String, JsonValue>,
        next: A,
    },
    
    /// Delete a node
    DeleteNode {
        table: String,
        id: i64,
        next: A,
    },
    
    /// Link parent and child nodes (many-to-many relationships)
    Link {
        parent_table: String,
        parent_id: i64,
        child_table: String,
        child_ids: Vec<i64>,
        next: A,
    },
    
    /// Nested mutations (composition)
    Nested {
        inner: Vec<MutationF<A>>,
        next: A,
    },
    
    /// Custom SQL operation (extensibility hook)
    Custom {
        sql: String,
        binds: Vec<Value>,
        next: A,
    },
    
    /// Bulk insert operation
    BulkInsert {
        table: String,
        rows: Vec<HashMap<String, JsonValue>>,
        next: A,
    },
    
    /// Upsert operation (insert or update)
    Upsert {
        table: String,
        input: HashMap<String, JsonValue>,
        on_conflict: Vec<String>, // Columns for conflict resolution
        next: A,
    },
}

/// Fixed point of MutationF for recursive mutations
pub type Mutation = MutationF<()>;

/// Free monad over mutation algebra
#[derive(Debug, Clone)]
pub enum Free<F, A> {
    Pure(A),
    Free(Box<F>),
}

/// Mutation monad - simplified for practical use
#[derive(Debug, Clone)]
pub struct MutationM<A> {
    operations: Vec<Mutation>,
    result: A,
}

impl<A> MutationM<A> {
    /// Monad return - lift a value into the mutation monad
    pub fn pure(a: A) -> Self {
        MutationM {
            operations: Vec::new(),
            result: a,
        }
    }

    /// Create a create node mutation
    pub fn create_node(table: String, input: HashMap<String, JsonValue>) -> MutationM<i64> {
        MutationM {
            operations: vec![MutationF::CreateNode {
                table,
                input,
                next: (),
            }],
            result: 0, // Placeholder ID
        }
    }

    /// Create an update node mutation
    pub fn update_node(table: String, id: i64, patch: HashMap<String, JsonValue>) -> MutationM<i64> {
        MutationM {
            operations: vec![MutationF::UpdateNode {
                table,
                id,
                patch,
                next: (),
            }],
            result: id,
        }
    }

    /// Create a delete node mutation
    pub fn delete_node(table: String, id: i64) -> MutationM<i64> {
        MutationM {
            operations: vec![MutationF::DeleteNode {
                table,
                id,
                next: (),
            }],
            result: id,
        }
    }

    /// Create a link mutation
    pub fn link_nodes(
        parent_table: String,
        parent_id: i64,
        child_table: String,
        child_ids: Vec<i64>,
    ) -> MutationM<()> {
        MutationM {
            operations: vec![MutationF::Link {
                parent_table,
                parent_id,
                child_table,
                child_ids,
                next: (),
            }],
            result: (),
        }
    }

    /// Create a custom SQL mutation
    pub fn custom(sql: String, binds: Vec<Value>) -> MutationM<()> {
        MutationM {
            operations: vec![MutationF::Custom {
                sql,
                binds,
                next: (),
            }],
            result: (),
        }
    }

    /// Create a bulk insert mutation
    pub fn bulk_insert(table: String, rows: Vec<HashMap<String, JsonValue>>) -> MutationM<Vec<i64>> {
        MutationM {
            operations: vec![MutationF::BulkInsert {
                table,
                rows,
                next: (),
            }],
            result: Vec::new(),
        }
    }

    /// Create an upsert mutation
    pub fn upsert(
        table: String,
        input: HashMap<String, JsonValue>,
        on_conflict: Vec<String>,
    ) -> MutationM<i64> {
        MutationM {
            operations: vec![MutationF::Upsert {
                table,
                input,
                on_conflict,
                next: (),
            }],
            result: 0,
        }
    }

    /// Monadic bind - sequence mutations
    pub fn and_then<B, F>(mut self, f: F) -> MutationM<B>
    where
        F: FnOnce(A) -> MutationM<B>,
    {
        let next_mutation = f(self.result);
        self.operations.extend(next_mutation.operations);
        MutationM {
            operations: self.operations,
            result: next_mutation.result,
        }
    }

    /// Compile mutation to SQL with CTEs
    pub fn to_sql(&self) -> SqlResult<CompiledMutation> {
        let mut compiler = MutationCompiler::new();
        compiler.compile_mutation(self)
    }

    /// Get the operations for compilation
    pub fn operations(&self) -> &[Mutation] {
        &self.operations
    }
}

/// Compiled mutation result
#[derive(Debug, Clone)]
pub struct CompiledMutation {
    pub sql: String,
    pub binds: Vec<Value>,
    pub returning_fields: Vec<String>,
    pub estimated_cost: f64,
}

/// Mutation compiler that converts mutation algebra to SQL
pub struct MutationCompiler {
    cte_counter: usize,
    bind_counter: usize,
    schema: TypeSafeSchema,
}

impl MutationCompiler {
    /// Create a new mutation compiler
    pub fn new() -> Self {
        Self {
            cte_counter: 0,
            bind_counter: 1, // SQL parameters start at $1
            schema: TypeSafeSchema::new(),
        }
    }
    
    /// Compile a mutation to SQL
    pub fn compile_mutation<A>(&mut self, mutation: &MutationM<A>) -> SqlResult<CompiledMutation> {
        let mut ctes = Vec::new();
        let mut binds = Vec::new();
        let mut returning_fields = Vec::new();

        for operation in mutation.operations() {
            self.compile_mutation_operation(operation, &mut ctes, &mut binds, &mut returning_fields)?;
        }

        let sql = self.build_final_sql(&ctes, &returning_fields)?;

        Ok(CompiledMutation {
            sql,
            binds,
            returning_fields,
            estimated_cost: self.estimate_cost(&ctes),
        })
    }
    
    /// Compile a single mutation operation
    fn compile_mutation_operation(
        &mut self,
        mutation: &Mutation,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        self.compile_mutation_f(mutation, ctes, binds, returning_fields)
    }
    
    /// Compile a specific mutation operation
    fn compile_mutation_f(
        &mut self,
        mutation_f: &Mutation,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        match mutation_f {
            MutationF::CreateNode { table, input, .. } => {
                self.compile_create_node(table, input, ctes, binds, returning_fields)
            },
            MutationF::UpdateNode { table, id, patch, .. } => {
                self.compile_update_node(table, *id, patch, ctes, binds, returning_fields)
            },
            MutationF::DeleteNode { table, id, .. } => {
                self.compile_delete_node(table, *id, ctes, binds, returning_fields)
            },
            MutationF::Link { parent_table, parent_id, child_table, child_ids, .. } => {
                self.compile_link_nodes(parent_table, *parent_id, child_table, child_ids, ctes, binds)
            },
            MutationF::Custom { sql, binds: custom_binds, .. } => {
                self.compile_custom_sql(sql, custom_binds, ctes, binds)
            },
            MutationF::BulkInsert { table, rows, .. } => {
                self.compile_bulk_insert(table, rows, ctes, binds, returning_fields)
            },
            MutationF::Upsert { table, input, on_conflict, .. } => {
                self.compile_upsert(table, input, on_conflict, ctes, binds, returning_fields)
            },
            MutationF::Nested { inner, .. } => {
                for nested_mutation in inner {
                    self.compile_mutation_f(nested_mutation, ctes, binds, returning_fields)?;
                }
                Ok(())
            },
        }
    }
    
    /// Compile CREATE operation
    fn compile_create_node(
        &mut self,
        table: &str,
        input: &HashMap<String, JsonValue>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_in_{}", table, self.cte_counter);
        self.cte_counter += 1;
        
        let fields: Vec<String> = input.keys().cloned().collect();
        let placeholders: Vec<String> = (0..input.len())
            .map(|_| {
                let placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                placeholder
            })
            .collect();
        
        // Add values to binds
        for value in input.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        let cte_sql = format!(
            "{} AS (INSERT INTO {} ({}) VALUES ({}) RETURNING id)",
            cte_name,
            table,
            fields.join(", "),
            placeholders.join(", ")
        );
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT id FROM {})", cte_name));
        
        Ok(())
    }
    
    /// Compile UPDATE operation
    fn compile_update_node(
        &mut self,
        table: &str,
        id: i64,
        patch: &HashMap<String, JsonValue>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_upd_{}", table, self.cte_counter);
        self.cte_counter += 1;
        
        let set_clauses: Vec<String> = patch.keys()
            .map(|field| {
                let placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                format!("{} = {}", field, placeholder)
            })
            .collect();
        
        // Add patch values to binds
        for value in patch.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        // Add ID to binds
        let id_placeholder = format!("${}", self.bind_counter);
        self.bind_counter += 1;
        binds.push(Value::Integer(id));
        
        let cte_sql = format!(
            "{} AS (UPDATE {} SET {} WHERE id = {} RETURNING id)",
            cte_name,
            table,
            set_clauses.join(", "),
            id_placeholder
        );
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT id FROM {})", cte_name));
        
        Ok(())
    }
    
    /// Compile DELETE operation
    fn compile_delete_node(
        &mut self,
        table: &str,
        id: i64,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_del_{}", table, self.cte_counter);
        self.cte_counter += 1;
        
        let id_placeholder = format!("${}", self.bind_counter);
        self.bind_counter += 1;
        binds.push(Value::Integer(id));
        
        let cte_sql = format!(
            "{} AS (DELETE FROM {} WHERE id = {} RETURNING id)",
            cte_name,
            table,
            id_placeholder
        );
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT id FROM {})", cte_name));
        
        Ok(())
    }
    
    /// Compile LINK operation
    fn compile_link_nodes(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        child_table: &str,
        child_ids: &[i64],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_{}_link_{}", parent_table, child_table, self.cte_counter);
        self.cte_counter += 1;
        
        let parent_id_placeholder = format!("${}", self.bind_counter);
        self.bind_counter += 1;
        binds.push(Value::Integer(parent_id));
        
        let child_ids_placeholder = format!("${}", self.bind_counter);
        self.bind_counter += 1;
        
        // Convert child IDs to SQL array format
        let child_ids_str = format!("ARRAY[{}]", 
            child_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","));
        binds.push(Value::Text(child_ids_str));
        
        let junction_table = format!("{}_{}", parent_table, child_table);
        let cte_sql = format!(
            "{} AS (INSERT INTO {} ({}_id, {}_id) SELECT {}, unnest({}::int[]))",
            cte_name,
            junction_table,
            parent_table,
            child_table,
            parent_id_placeholder,
            child_ids_placeholder
        );
        
        ctes.push(cte_sql);
        
        Ok(())
    }
    
    /// Compile custom SQL
    fn compile_custom_sql(
        &mut self,
        sql: &str,
        custom_binds: &[Value],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_name = format!("custom_{}", self.cte_counter);
        self.cte_counter += 1;
        
        // Replace placeholders in custom SQL
        let mut processed_sql = sql.to_string();
        for (i, _) in custom_binds.iter().enumerate() {
            let old_placeholder = format!("${}", i + 1);
            let new_placeholder = format!("${}", self.bind_counter);
            processed_sql = processed_sql.replace(&old_placeholder, &new_placeholder);
            self.bind_counter += 1;
        }
        
        binds.extend(custom_binds.iter().cloned());
        
        let cte_sql = format!("{} AS ({})", cte_name, processed_sql);
        ctes.push(cte_sql);
        
        Ok(())
    }
    
    /// Compile bulk insert
    fn compile_bulk_insert(
        &mut self,
        table: &str,
        rows: &[HashMap<String, JsonValue>],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        if rows.is_empty() {
            return Ok(());
        }
        
        let cte_name = format!("{}_bulk_{}", table, self.cte_counter);
        self.cte_counter += 1;
        
        let fields: Vec<String> = rows[0].keys().cloned().collect();
        let mut value_clauses = Vec::new();
        
        for row in rows {
            let placeholders: Vec<String> = (0..row.len())
                .map(|_| {
                    let placeholder = format!("${}", self.bind_counter);
                    self.bind_counter += 1;
                    placeholder
                })
                .collect();
            
            value_clauses.push(format!("({})", placeholders.join(", ")));
            
            // Add row values to binds in field order
            for field in &fields {
                if let Some(value) = row.get(field) {
                    binds.push(self.json_value_to_sql_value(value)?);
                } else {
                    binds.push(Value::Null);
                }
            }
        }
        
        let cte_sql = format!(
            "{} AS (INSERT INTO {} ({}) VALUES {} RETURNING id)",
            cte_name,
            table,
            fields.join(", "),
            value_clauses.join(", ")
        );
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT array_agg(id) FROM {})", cte_name));
        
        Ok(())
    }
    
    /// Compile upsert operation
    fn compile_upsert(
        &mut self,
        table: &str,
        input: &HashMap<String, JsonValue>,
        on_conflict: &[String],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_upsert_{}", table, self.cte_counter);
        self.cte_counter += 1;
        
        let fields: Vec<String> = input.keys().cloned().collect();
        let placeholders: Vec<String> = (0..input.len())
            .map(|_| {
                let placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                placeholder
            })
            .collect();
        
        // Add values to binds
        for value in input.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        // Build UPDATE SET clause for conflict resolution
        let update_clauses: Vec<String> = fields.iter()
            .filter(|field| !on_conflict.contains(field))
            .map(|field| format!("{} = EXCLUDED.{}", field, field))
            .collect();
        
        let cte_sql = format!(
            "{} AS (INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {} RETURNING id)",
            cte_name,
            table,
            fields.join(", "),
            placeholders.join(", "),
            on_conflict.join(", "),
            update_clauses.join(", ")
        );
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT id FROM {})", cte_name));
        
        Ok(())
    }
    
    /// Build final SQL with CTEs and result selection
    fn build_final_sql(&self, ctes: &[String], returning_fields: &[String]) -> SqlResult<String> {
        if ctes.is_empty() {
            return Ok("SELECT '{}'::jsonb AS data".to_string());
        }
        
        let mut sql = String::new();
        sql.push_str("WITH ");
        sql.push_str(&ctes.join(",\n"));
        sql.push_str("\nSELECT jsonb_build_object(");
        
        let result_pairs: Vec<String> = returning_fields.iter()
            .enumerate()
            .map(|(i, field)| format!("'result_{}', {}", i, field))
            .collect();
        
        sql.push_str(&result_pairs.join(", "));
        sql.push_str(") AS data");
        
        Ok(sql)
    }
    
    /// Convert JSON value to SQL value
    fn json_value_to_sql_value(&self, value: &JsonValue) -> SqlResult<Value> {
        match value {
            JsonValue::String(s) => Ok(Value::Text(s.clone())),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::Real(f))
                } else {
                    Err(SqlError::runtime_error("Invalid number format"))
                }
            },
            JsonValue::Bool(b) => Ok(Value::Boolean(*b)),
            JsonValue::Null => Ok(Value::Null),
            _ => Err(SqlError::runtime_error("Unsupported JSON value type for SQL conversion")),
        }
    }
    
    /// Estimate cost of mutation operations
    fn estimate_cost(&self, ctes: &[String]) -> f64 {
        // Simple cost estimation based on number of operations
        ctes.len() as f64 * 10.0 + 
        ctes.iter().map(|cte| cte.len() as f64 * 0.1).sum::<f64>()
    }
}

impl Default for MutationCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-root mutation algebra for combining multiple mutations
#[derive(Debug, Clone)]
pub enum MultiRootMutationF<A> {
    /// Single mutation root
    Root {
        name: String,
        mutation: MutationM<i64>,
        next: A,
    },

    /// Combine multiple mutation roots
    Combine {
        left: Box<MultiRootMutationF<A>>,
        right: Box<MultiRootMutationF<A>>,
        next: A,
    },

    /// Sequential execution (enforces order)
    Sequential {
        mutations: Vec<MultiRootMutationF<A>>,
        next: A,
    },

    /// Parallel execution (can be optimized)
    Parallel {
        mutations: Vec<MultiRootMutationF<A>>,
        next: A,
    },
}

/// Fixed point for multi-root mutations
pub type MultiRootMutation = MultiRootMutationF<()>;

/// Multi-root mutation compiler
pub struct MultiRootMutationCompiler {
    base_compiler: MutationCompiler,
    transaction_isolation: TransactionIsolation,
}

/// Transaction isolation levels
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionIsolation {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl MultiRootMutationCompiler {
    /// Create a new multi-root mutation compiler
    pub fn new(isolation: TransactionIsolation) -> Self {
        Self {
            base_compiler: MutationCompiler::new(),
            transaction_isolation: isolation,
        }
    }

    /// Compile multi-root mutation to SQL transaction
    pub fn compile(&mut self, multi_root: MultiRootMutation) -> SqlResult<CompiledTransaction> {
        let mut transaction_ctes = Vec::new();
        let mut transaction_binds = Vec::new();
        let mut root_results = Vec::new();

        self.compile_multi_root_recursive(
            &multi_root,
            &mut transaction_ctes,
            &mut transaction_binds,
            &mut root_results,
        )?;

        let sql = self.build_transaction_sql(&transaction_ctes, &root_results)?;

        Ok(CompiledTransaction {
            sql,
            binds: transaction_binds,
            isolation: self.transaction_isolation.clone(),
            root_results,
            estimated_cost: self.estimate_transaction_cost(&transaction_ctes),
        })
    }

    /// Recursively compile multi-root mutations
    fn compile_multi_root_recursive(
        &mut self,
        multi_root: &MultiRootMutation,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        root_results: &mut Vec<String>,
    ) -> SqlResult<()> {
        match multi_root {
            MultiRootMutationF::Root { name, mutation, .. } => {
                let compiled = self.base_compiler.compile_mutation(mutation)?;

                // Extract CTEs from the compiled mutation
                if compiled.sql.starts_with("WITH") {
                    let parts: Vec<&str> = compiled.sql.splitn(2, "SELECT").collect();
                    if parts.len() == 2 {
                        let cte_part = parts[0].trim_start_matches("WITH").trim();
                        ctes.push(cte_part.to_string());
                    }
                }

                binds.extend(compiled.binds);
                root_results.push(format!("'{}', ({})", name, compiled.returning_fields.join(", ")));

                Ok(())
            },
            MultiRootMutationF::Combine { left, right, .. } => {
                self.compile_multi_root_recursive(left, ctes, binds, root_results)?;
                self.compile_multi_root_recursive(right, ctes, binds, root_results)?;
                Ok(())
            },
            MultiRootMutationF::Sequential { mutations, .. } => {
                for mutation in mutations {
                    self.compile_multi_root_recursive(mutation, ctes, binds, root_results)?;
                }
                Ok(())
            },
            MultiRootMutationF::Parallel { mutations, .. } => {
                // For now, treat parallel the same as sequential
                // In a full implementation, we'd optimize for parallel execution
                for mutation in mutations {
                    self.compile_multi_root_recursive(mutation, ctes, binds, root_results)?;
                }
                Ok(())
            },
        }
    }

    /// Build transaction SQL with proper isolation
    fn build_transaction_sql(&self, ctes: &[String], root_results: &[String]) -> SqlResult<String> {
        let mut sql = String::new();

        // Start transaction with isolation level
        sql.push_str("BEGIN TRANSACTION ISOLATION LEVEL ");
        sql.push_str(match self.transaction_isolation {
            TransactionIsolation::ReadUncommitted => "READ UNCOMMITTED",
            TransactionIsolation::ReadCommitted => "READ COMMITTED",
            TransactionIsolation::RepeatableRead => "REPEATABLE READ",
            TransactionIsolation::Serializable => "SERIALIZABLE",
        });
        sql.push_str(";\n");

        // Add CTEs and main query
        if !ctes.is_empty() {
            sql.push_str("WITH ");
            sql.push_str(&ctes.join(",\n"));
            sql.push_str("\n");
        }

        sql.push_str("SELECT jsonb_build_object(");
        sql.push_str(&root_results.join(", "));
        sql.push_str(") AS data;\n");

        // Commit transaction
        sql.push_str("COMMIT;");

        Ok(sql)
    }

    /// Estimate transaction cost
    fn estimate_transaction_cost(&self, ctes: &[String]) -> f64 {
        let base_cost = ctes.len() as f64 * 15.0; // Higher cost for transactions
        let complexity_cost = ctes.iter().map(|cte| cte.len() as f64 * 0.2).sum::<f64>();
        let transaction_overhead = 50.0; // Fixed transaction overhead

        base_cost + complexity_cost + transaction_overhead
    }
}

/// Compiled transaction result
#[derive(Debug, Clone)]
pub struct CompiledTransaction {
    pub sql: String,
    pub binds: Vec<Value>,
    pub isolation: TransactionIsolation,
    pub root_results: Vec<String>,
    pub estimated_cost: f64,
}

/// Builder for multi-root mutations
pub struct MultiRootMutationBuilder {
    roots: Vec<(String, MutationM<i64>)>,
    execution_mode: ExecutionMode,
}

/// Execution modes for multi-root mutations
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionMode {
    Sequential,
    Parallel,
    Optimized, // Let compiler decide
}

impl MultiRootMutationBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
            execution_mode: ExecutionMode::Optimized,
        }
    }

    /// Add a mutation root
    pub fn add_root(mut self, name: String, mutation: MutationM<i64>) -> Self {
        self.roots.push((name, mutation));
        self
    }

    /// Set execution mode
    pub fn execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Build the multi-root mutation
    pub fn build(self) -> MultiRootMutation {
        if self.roots.is_empty() {
            return MultiRootMutationF::Root {
                name: "empty".to_string(),
                mutation: MutationM::pure(0),
                next: (),
            };
        }

        let mut result = None;

        for (name, mutation) in self.roots {
            let root = MultiRootMutationF::Root {
                name,
                mutation,
                next: (),
            };

            result = Some(match result {
                None => root,
                Some(existing) => match self.execution_mode {
                    ExecutionMode::Sequential => MultiRootMutationF::Sequential {
                        mutations: vec![existing, root],
                        next: (),
                    },
                    ExecutionMode::Parallel => MultiRootMutationF::Parallel {
                        mutations: vec![existing, root],
                        next: (),
                    },
                    ExecutionMode::Optimized => MultiRootMutationF::Combine {
                        left: Box::new(existing),
                        right: Box::new(root),
                        next: (),
                    },
                },
            });
        }

        result.unwrap()
    }
}

impl Default for MultiRootMutationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MultiRootMutationCompiler {
    fn default() -> Self {
        Self::new(TransactionIsolation::ReadCommitted)
    }
}
