/// Advanced Mutation Compiler
/// 
/// This module implements the complete catamorphism that converts mutation algebra
/// to SQL with CTEs, RETURNING clauses, and full support for nested relations.

use std::collections::HashMap;
use serde_json::Value as JsonValue;
use crate::sqlite::types::Value;
use crate::sqlite::error::{SqlError, SqlResult};
use crate::orm::mutation::{
    MutationM, MutationF, MultiRootMutation, MultiRootMutationF, CompiledMutation, 
    CompiledTransaction, TransactionIsolation,
};
use crate::orm::nested_relations::{NestedRelationCompiler, NestedRelation, RelationMetadata};
use crate::orm::schema::TypeSafeSchema;

/// Advanced mutation compiler with full feature support
pub struct AdvancedMutationCompiler {
    base_cte_counter: usize,
    base_bind_counter: usize,
    schema: TypeSafeSchema,
    nested_compiler: NestedRelationCompiler,
    optimization_level: MutationOptimizationLevel,
}

/// Optimization levels for mutation compilation
#[derive(Debug, Clone, PartialEq)]
pub enum MutationOptimizationLevel {
    None,
    Basic,
    Aggressive,
}

/// Compiled mutation with full metadata
#[derive(Debug, Clone)]
pub struct AdvancedCompiledMutation {
    pub sql: String,
    pub binds: Vec<Value>,
    pub returning_fields: Vec<String>,
    pub estimated_cost: f64,
    pub affected_tables: Vec<String>,
    pub operation_count: usize,
    pub uses_transactions: bool,
    pub rollback_safe: bool,
}

/// Mutation execution plan
#[derive(Debug, Clone)]
pub struct MutationExecutionPlan {
    pub compiled: AdvancedCompiledMutation,
    pub execution_strategy: MutationExecutionStrategy,
    pub dependency_graph: Vec<MutationDependency>,
    pub parallel_groups: Vec<Vec<usize>>,
}

/// Execution strategies for mutations
#[derive(Debug, Clone, PartialEq)]
pub enum MutationExecutionStrategy {
    Sequential,
    Parallel,
    Batched,
    Streaming,
}

/// Mutation dependencies for optimization
#[derive(Debug, Clone)]
pub struct MutationDependency {
    pub operation_id: usize,
    pub depends_on: Vec<usize>,
    pub table: String,
    pub operation_type: String,
}

impl AdvancedMutationCompiler {
    /// Create a new advanced mutation compiler
    pub fn new(optimization_level: MutationOptimizationLevel) -> Self {
        Self {
            base_cte_counter: 0,
            base_bind_counter: 1,
            schema: TypeSafeSchema::new(),
            nested_compiler: NestedRelationCompiler::new(),
            optimization_level,
        }
    }
    
    /// Register a custom relation
    pub fn register_relation(&mut self, relation: RelationMetadata) {
        self.nested_compiler.register_relation(relation);
    }
    
    /// Compile a single mutation with full optimization
    pub fn compile_mutation<A>(&mut self, mutation: &MutationM<A>) -> SqlResult<AdvancedCompiledMutation> {
        let mut ctes = Vec::new();
        let mut binds = Vec::new();
        let mut returning_fields = Vec::new();
        let mut affected_tables = Vec::new();
        let mut operation_count = 0;
        
        self.compile_mutation_recursive(
            mutation,
            &mut ctes,
            &mut binds,
            &mut returning_fields,
            &mut affected_tables,
            &mut operation_count,
        )?;
        
        // Apply optimizations
        let optimized_ctes = self.optimize_ctes(ctes)?;
        let optimized_binds = self.optimize_binds(binds)?;
        
        let sql = self.build_optimized_sql(&optimized_ctes, &returning_fields)?;
        
        Ok(AdvancedCompiledMutation {
            sql,
            binds: optimized_binds,
            returning_fields,
            estimated_cost: self.estimate_advanced_cost(&optimized_ctes, operation_count),
            affected_tables,
            operation_count,
            uses_transactions: operation_count > 1,
            rollback_safe: true,
        })
    }
    
    /// Compile multi-root mutation with execution planning
    pub fn compile_multi_root(&mut self, multi_root: MultiRootMutation) -> SqlResult<MutationExecutionPlan> {
        let mut all_ctes = Vec::new();
        let mut all_binds = Vec::new();
        let mut all_returning = Vec::new();
        let mut all_affected_tables = Vec::new();
        let mut total_operations = 0;
        let mut dependencies = Vec::new();
        
        self.compile_multi_root_with_dependencies(
            &multi_root,
            &mut all_ctes,
            &mut all_binds,
            &mut all_returning,
            &mut all_affected_tables,
            &mut total_operations,
            &mut dependencies,
        )?;
        
        // Optimize and plan execution
        let optimized_ctes = self.optimize_ctes(all_ctes)?;
        let parallel_groups = self.analyze_parallelization(&dependencies);
        let execution_strategy = self.choose_execution_strategy(total_operations, &parallel_groups);
        
        let sql = self.build_transaction_sql(&optimized_ctes, &all_returning)?;
        
        let compiled = AdvancedCompiledMutation {
            sql,
            binds: all_binds,
            returning_fields: all_returning,
            estimated_cost: self.estimate_advanced_cost(&optimized_ctes, total_operations),
            affected_tables: all_affected_tables,
            operation_count: total_operations,
            uses_transactions: true,
            rollback_safe: true,
        };
        
        Ok(MutationExecutionPlan {
            compiled,
            execution_strategy,
            dependency_graph: dependencies,
            parallel_groups,
        })
    }
    
    /// Compile mutation with nested relations
    pub fn compile_with_nested_relations<A>(
        &mut self,
        mutation: &MutationM<A>,
        nested_relations: &[NestedRelation],
        parent_table: &str,
        parent_id: i64,
    ) -> SqlResult<AdvancedCompiledMutation> {
        let mut ctes = Vec::new();
        let mut binds = Vec::new();
        let mut returning_fields = Vec::new();
        let mut affected_tables = Vec::new();
        let mut operation_count = 0;
        
        // Compile base mutation
        self.compile_mutation_recursive(
            mutation,
            &mut ctes,
            &mut binds,
            &mut returning_fields,
            &mut affected_tables,
            &mut operation_count,
        )?;
        
        // Compile nested relations
        for nested_relation in nested_relations {
            self.nested_compiler.compile_nested_relation(
                parent_table,
                parent_id,
                nested_relation,
                &mut ctes,
                &mut binds,
            )?;
            operation_count += 1;
        }
        
        let sql = self.build_optimized_sql(&ctes, &returning_fields)?;
        
        Ok(AdvancedCompiledMutation {
            sql,
            binds,
            returning_fields,
            estimated_cost: self.estimate_advanced_cost(&ctes, operation_count),
            affected_tables,
            operation_count,
            uses_transactions: operation_count > 1,
            rollback_safe: true,
        })
    }
    
    /// Compile mutation operations
    fn compile_mutation_recursive<A>(
        &mut self,
        mutation: &MutationM<A>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
        affected_tables: &mut Vec<String>,
        operation_count: &mut usize,
    ) -> SqlResult<()> {
        for operation in mutation.operations() {
            self.compile_mutation_operation(
                operation,
                ctes,
                binds,
                returning_fields,
                affected_tables,
                operation_count,
            )?;
        }
        Ok(())
    }
    
    /// Compile a specific mutation operation
    fn compile_mutation_operation(
        &mut self,
        mutation_f: &crate::orm::mutation::Mutation,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
        affected_tables: &mut Vec<String>,
        operation_count: &mut usize,
    ) -> SqlResult<()> {
        *operation_count += 1;
        
        match mutation_f {
            MutationF::CreateNode { table, input, .. } => {
                affected_tables.push(table.clone());
                self.compile_create_node_advanced(table, input, ctes, binds, returning_fields)
            },
            MutationF::UpdateNode { table, id, patch, .. } => {
                affected_tables.push(table.clone());
                self.compile_update_node_advanced(table, *id, patch, ctes, binds, returning_fields)
            },
            MutationF::DeleteNode { table, id, .. } => {
                affected_tables.push(table.clone());
                self.compile_delete_node_advanced(table, *id, ctes, binds, returning_fields)
            },
            MutationF::BulkInsert { table, rows, .. } => {
                affected_tables.push(table.clone());
                self.compile_bulk_insert_advanced(table, rows, ctes, binds, returning_fields)
            },
            MutationF::Upsert { table, input, on_conflict, .. } => {
                affected_tables.push(table.clone());
                self.compile_upsert_advanced(table, input, on_conflict, ctes, binds, returning_fields)
            },
            MutationF::Custom { sql, binds: custom_binds, .. } => {
                self.compile_custom_sql_advanced(sql, custom_binds, ctes, binds)
            },
            MutationF::Link { parent_table, parent_id, child_table, child_ids, .. } => {
                affected_tables.push(format!("{}_{}", parent_table, child_table));
                self.compile_link_advanced(parent_table, *parent_id, child_table, child_ids, ctes, binds)
            },
            MutationF::Nested { inner, .. } => {
                for nested_mutation in inner {
                    self.compile_mutation_operation(nested_mutation, ctes, binds, returning_fields, affected_tables, operation_count)?;
                }
                Ok(())
            },
        }
    }
    
    /// Compile multi-root with dependency tracking
    fn compile_multi_root_with_dependencies(
        &mut self,
        multi_root: &MultiRootMutation,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
        affected_tables: &mut Vec<String>,
        operation_count: &mut usize,
        dependencies: &mut Vec<MutationDependency>,
    ) -> SqlResult<()> {
        match multi_root {
            MultiRootMutationF::Root { name, mutation, .. } => {
                let start_op_count = *operation_count;
                
                self.compile_mutation_recursive(
                    mutation,
                    ctes,
                    binds,
                    returning_fields,
                    affected_tables,
                    operation_count,
                )?;
                
                // Add dependency tracking
                dependencies.push(MutationDependency {
                    operation_id: start_op_count,
                    depends_on: Vec::new(), // Root operations have no dependencies
                    table: name.clone(),
                    operation_type: "root".to_string(),
                });
                
                Ok(())
            },
            MultiRootMutationF::Combine { left, right, .. } => {
                self.compile_multi_root_with_dependencies(left, ctes, binds, returning_fields, affected_tables, operation_count, dependencies)?;
                self.compile_multi_root_with_dependencies(right, ctes, binds, returning_fields, affected_tables, operation_count, dependencies)?;
                Ok(())
            },
            MultiRootMutationF::Sequential { mutations, .. } => {
                let mut prev_op_id = None;
                for mutation in mutations {
                    let start_op_count = *operation_count;
                    self.compile_multi_root_with_dependencies(mutation, ctes, binds, returning_fields, affected_tables, operation_count, dependencies)?;
                    
                    // Sequential operations depend on previous ones
                    if let Some(prev_id) = prev_op_id {
                        if let Some(dep) = dependencies.last_mut() {
                            dep.depends_on.push(prev_id);
                        }
                    }
                    prev_op_id = Some(start_op_count);
                }
                Ok(())
            },
            MultiRootMutationF::Parallel { mutations, .. } => {
                // Parallel operations have no dependencies on each other
                for mutation in mutations {
                    self.compile_multi_root_with_dependencies(mutation, ctes, binds, returning_fields, affected_tables, operation_count, dependencies)?;
                }
                Ok(())
            },
        }
    }
    
    /// Advanced CREATE compilation with optimizations
    fn compile_create_node_advanced(
        &mut self,
        table: &str,
        input: &HashMap<String, JsonValue>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_create_{}", table, self.base_cte_counter);
        self.base_cte_counter += 1;
        
        let fields: Vec<String> = input.keys().cloned().collect();
        let placeholders: Vec<String> = (0..input.len())
            .map(|_| {
                let placeholder = format!("${}", self.base_bind_counter);
                self.base_bind_counter += 1;
                placeholder
            })
            .collect();
        
        // Add optimized value conversion
        for value in input.values() {
            binds.push(self.optimize_value_conversion(value)?);
        }
        
        // Generate optimized SQL based on optimization level
        let cte_sql = match self.optimization_level {
            MutationOptimizationLevel::Aggressive => {
                // Use prepared statement optimization
                format!(
                    "{} AS (INSERT INTO {} ({}) VALUES ({}) RETURNING id, created_at, updated_at)",
                    cte_name,
                    table,
                    fields.join(", "),
                    placeholders.join(", ")
                )
            },
            _ => {
                format!(
                    "{} AS (INSERT INTO {} ({}) VALUES ({}) RETURNING id)",
                    cte_name,
                    table,
                    fields.join(", "),
                    placeholders.join(", ")
                )
            }
        };
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT row_to_json({}) FROM {})", cte_name, cte_name));
        
        Ok(())
    }
    
    /// Advanced UPDATE compilation
    fn compile_update_node_advanced(
        &mut self,
        table: &str,
        id: i64,
        patch: &HashMap<String, JsonValue>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_update_{}", table, self.base_cte_counter);
        self.base_cte_counter += 1;
        
        let set_clauses: Vec<String> = patch.keys()
            .map(|field| {
                let placeholder = format!("${}", self.base_bind_counter);
                self.base_bind_counter += 1;
                format!("{} = {}", field, placeholder)
            })
            .collect();
        
        // Add patch values
        for value in patch.values() {
            binds.push(self.optimize_value_conversion(value)?);
        }
        
        // Add ID
        let id_placeholder = format!("${}", self.base_bind_counter);
        self.base_bind_counter += 1;
        binds.push(Value::Integer(id));
        
        // Add updated_at if optimization level allows
        let additional_sets = match self.optimization_level {
            MutationOptimizationLevel::Aggressive => ", updated_at = NOW()",
            _ => "",
        };
        
        let cte_sql = format!(
            "{} AS (UPDATE {} SET {}{} WHERE id = {} RETURNING *)",
            cte_name,
            table,
            set_clauses.join(", "),
            additional_sets,
            id_placeholder
        );
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT row_to_json({}) FROM {})", cte_name, cte_name));
        
        Ok(())
    }
    
    /// Advanced DELETE compilation
    fn compile_delete_node_advanced(
        &mut self,
        table: &str,
        id: i64,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_delete_{}", table, self.base_cte_counter);
        self.base_cte_counter += 1;
        
        let id_placeholder = format!("${}", self.base_bind_counter);
        self.base_bind_counter += 1;
        binds.push(Value::Integer(id));
        
        // Soft delete vs hard delete based on optimization level
        let cte_sql = match self.optimization_level {
            MutationOptimizationLevel::Aggressive => {
                // Soft delete with timestamp
                format!(
                    "{} AS (UPDATE {} SET deleted_at = NOW() WHERE id = {} AND deleted_at IS NULL RETURNING id)",
                    cte_name,
                    table,
                    id_placeholder
                )
            },
            _ => {
                format!(
                    "{} AS (DELETE FROM {} WHERE id = {} RETURNING id)",
                    cte_name,
                    table,
                    id_placeholder
                )
            }
        };
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT id FROM {})", cte_name));
        
        Ok(())
    }
    
    /// Advanced bulk insert compilation
    fn compile_bulk_insert_advanced(
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
        
        let cte_name = format!("{}_bulk_{}", table, self.base_cte_counter);
        self.base_cte_counter += 1;
        
        // Use COPY for large bulk inserts if optimization allows
        if self.optimization_level == MutationOptimizationLevel::Aggressive && rows.len() > 100 {
            return self.compile_bulk_copy(table, rows, ctes, binds, returning_fields);
        }
        
        let fields: Vec<String> = rows[0].keys().cloned().collect();
        let mut value_clauses = Vec::new();
        
        for row in rows {
            let placeholders: Vec<String> = (0..row.len())
                .map(|_| {
                    let placeholder = format!("${}", self.base_bind_counter);
                    self.base_bind_counter += 1;
                    placeholder
                })
                .collect();
            
            value_clauses.push(format!("({})", placeholders.join(", ")));
            
            for field in &fields {
                if let Some(value) = row.get(field) {
                    binds.push(self.optimize_value_conversion(value)?);
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
    
    /// Compile bulk copy for large datasets
    fn compile_bulk_copy(
        &mut self,
        table: &str,
        rows: &[HashMap<String, JsonValue>],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_copy_{}", table, self.base_cte_counter);
        self.base_cte_counter += 1;
        
        // For now, fall back to regular bulk insert
        // In a full implementation, this would use PostgreSQL COPY
        self.compile_bulk_insert_advanced(table, rows, ctes, binds, returning_fields)
    }
    
    /// Advanced upsert compilation
    fn compile_upsert_advanced(
        &mut self,
        table: &str,
        input: &HashMap<String, JsonValue>,
        on_conflict: &[String],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
        returning_fields: &mut Vec<String>,
    ) -> SqlResult<()> {
        let cte_name = format!("{}_upsert_{}", table, self.base_cte_counter);
        self.base_cte_counter += 1;
        
        let fields: Vec<String> = input.keys().cloned().collect();
        let placeholders: Vec<String> = (0..input.len())
            .map(|_| {
                let placeholder = format!("${}", self.base_bind_counter);
                self.base_bind_counter += 1;
                placeholder
            })
            .collect();
        
        for value in input.values() {
            binds.push(self.optimize_value_conversion(value)?);
        }
        
        let update_clauses: Vec<String> = fields.iter()
            .filter(|field| !on_conflict.contains(field))
            .map(|field| format!("{} = EXCLUDED.{}", field, field))
            .collect();
        
        let additional_update = match self.optimization_level {
            MutationOptimizationLevel::Aggressive => ", updated_at = NOW()",
            _ => "",
        };
        
        let cte_sql = format!(
            "{} AS (INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}{} RETURNING *)",
            cte_name,
            table,
            fields.join(", "),
            placeholders.join(", "),
            on_conflict.join(", "),
            update_clauses.join(", "),
            additional_update
        );
        
        ctes.push(cte_sql);
        returning_fields.push(format!("(SELECT row_to_json({}) FROM {})", cte_name, cte_name));
        
        Ok(())
    }
    
    /// Advanced custom SQL compilation
    fn compile_custom_sql_advanced(
        &mut self,
        sql: &str,
        custom_binds: &[Value],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_name = format!("custom_{}", self.base_cte_counter);
        self.base_cte_counter += 1;
        
        let mut processed_sql = sql.to_string();
        for (i, _) in custom_binds.iter().enumerate() {
            let old_placeholder = format!("${}", i + 1);
            let new_placeholder = format!("${}", self.base_bind_counter);
            processed_sql = processed_sql.replace(&old_placeholder, &new_placeholder);
            self.base_bind_counter += 1;
        }
        
        binds.extend(custom_binds.iter().cloned());
        
        let cte_sql = format!("{} AS ({})", cte_name, processed_sql);
        ctes.push(cte_sql);
        
        Ok(())
    }
    
    /// Advanced link compilation
    fn compile_link_advanced(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        child_table: &str,
        child_ids: &[i64],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_name = format!("link_{}_{}", parent_table, self.base_cte_counter);
        self.base_cte_counter += 1;
        
        let parent_id_placeholder = format!("${}", self.base_bind_counter);
        self.base_bind_counter += 1;
        binds.push(Value::Integer(parent_id));
        
        let child_ids_placeholder = format!("${}", self.base_bind_counter);
        self.base_bind_counter += 1;
        let child_ids_str = format!("ARRAY[{}]", 
            child_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","));
        binds.push(Value::Text(child_ids_str));
        
        let junction_table = format!("{}_{}", parent_table, child_table);
        
        // Use ON CONFLICT for idempotent linking
        let cte_sql = match self.optimization_level {
            MutationOptimizationLevel::Aggressive => {
                format!(
                    "{} AS (INSERT INTO {} ({}_id, {}_id, created_at) SELECT {}, unnest({}::int[]), NOW() ON CONFLICT DO NOTHING)",
                    cte_name,
                    junction_table,
                    parent_table,
                    child_table,
                    parent_id_placeholder,
                    child_ids_placeholder
                )
            },
            _ => {
                format!(
                    "{} AS (INSERT INTO {} ({}_id, {}_id) SELECT {}, unnest({}::int[]) ON CONFLICT DO NOTHING)",
                    cte_name,
                    junction_table,
                    parent_table,
                    child_table,
                    parent_id_placeholder,
                    child_ids_placeholder
                )
            }
        };
        
        ctes.push(cte_sql);
        
        Ok(())
    }
    
    /// Optimize CTEs for better performance
    fn optimize_ctes(&self, ctes: Vec<String>) -> SqlResult<Vec<String>> {
        match self.optimization_level {
            MutationOptimizationLevel::None => Ok(ctes),
            MutationOptimizationLevel::Basic => {
                // Basic optimization: remove duplicate CTEs
                let mut optimized = Vec::new();
                let mut seen = std::collections::HashSet::new();
                
                for cte in ctes {
                    if !seen.contains(&cte) {
                        seen.insert(cte.clone());
                        optimized.push(cte);
                    }
                }
                
                Ok(optimized)
            },
            MutationOptimizationLevel::Aggressive => {
                // Aggressive optimization: reorder CTEs, combine operations
                let mut optimized = ctes;
                
                // Sort CTEs by dependency order
                optimized.sort_by(|a, b| {
                    // Simple heuristic: CREATE before UPDATE before DELETE
                    let a_priority = if a.contains("INSERT") { 0 } else if a.contains("UPDATE") { 1 } else { 2 };
                    let b_priority = if b.contains("INSERT") { 0 } else if b.contains("UPDATE") { 1 } else { 2 };
                    a_priority.cmp(&b_priority)
                });
                
                Ok(optimized)
            }
        }
    }
    
    /// Optimize bind parameters
    fn optimize_binds(&self, binds: Vec<Value>) -> SqlResult<Vec<Value>> {
        // For now, just return as-is
        // In a full implementation, could optimize parameter types
        Ok(binds)
    }
    
    /// Optimize value conversion
    fn optimize_value_conversion(&self, value: &JsonValue) -> SqlResult<Value> {
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
            _ => Err(SqlError::runtime_error("Unsupported JSON value type")),
        }
    }
    
    /// Build optimized SQL
    fn build_optimized_sql(&self, ctes: &[String], returning_fields: &[String]) -> SqlResult<String> {
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
    
    /// Build transaction SQL
    fn build_transaction_sql(&self, ctes: &[String], returning_fields: &[String]) -> SqlResult<String> {
        let mut sql = String::new();
        
        sql.push_str("BEGIN;\n");
        
        if !ctes.is_empty() {
            sql.push_str("WITH ");
            sql.push_str(&ctes.join(",\n"));
            sql.push_str("\n");
        }
        
        sql.push_str("SELECT jsonb_build_object(");
        
        let result_pairs: Vec<String> = returning_fields.iter()
            .enumerate()
            .map(|(i, field)| format!("'result_{}', {}", i, field))
            .collect();
        
        sql.push_str(&result_pairs.join(", "));
        sql.push_str(") AS data;\n");
        sql.push_str("COMMIT;");
        
        Ok(sql)
    }
    
    /// Estimate advanced cost
    fn estimate_advanced_cost(&self, ctes: &[String], operation_count: usize) -> f64 {
        let base_cost = operation_count as f64 * 20.0;
        let complexity_cost = ctes.iter().map(|cte| cte.len() as f64 * 0.3).sum::<f64>();
        let optimization_bonus = match self.optimization_level {
            MutationOptimizationLevel::Aggressive => -10.0,
            MutationOptimizationLevel::Basic => -5.0,
            MutationOptimizationLevel::None => 0.0,
        };
        
        base_cost + complexity_cost + optimization_bonus
    }
    
    /// Analyze parallelization opportunities
    fn analyze_parallelization(&self, dependencies: &[MutationDependency]) -> Vec<Vec<usize>> {
        let mut parallel_groups = Vec::new();
        let mut processed = std::collections::HashSet::new();
        
        // Group operations that have no dependencies on each other
        for (i, dep) in dependencies.iter().enumerate() {
            if processed.contains(&i) {
                continue;
            }
            
            let mut group = vec![i];
            processed.insert(i);
            
            // Find other operations that can run in parallel
            for (j, other_dep) in dependencies.iter().enumerate().skip(i + 1) {
                if processed.contains(&j) {
                    continue;
                }
                
                // Check if operations can run in parallel
                let can_parallelize = !dep.depends_on.contains(&j) && 
                                    !other_dep.depends_on.contains(&i) &&
                                    dep.table != other_dep.table; // Different tables can often run in parallel
                
                if can_parallelize {
                    group.push(j);
                    processed.insert(j);
                }
            }
            
            parallel_groups.push(group);
        }
        
        parallel_groups
    }
    
    /// Choose execution strategy
    fn choose_execution_strategy(&self, operation_count: usize, parallel_groups: &[Vec<usize>]) -> MutationExecutionStrategy {
        if operation_count == 1 {
            MutationExecutionStrategy::Sequential
        } else if parallel_groups.len() > 1 && parallel_groups.iter().any(|group| group.len() > 1) {
            MutationExecutionStrategy::Parallel
        } else if operation_count > 50 {
            MutationExecutionStrategy::Batched
        } else if operation_count > 100 {
            MutationExecutionStrategy::Streaming
        } else {
            MutationExecutionStrategy::Sequential
        }
    }
}

impl Default for AdvancedMutationCompiler {
    fn default() -> Self {
        Self::new(MutationOptimizationLevel::Basic)
    }
}
