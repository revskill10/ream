/// Nested Relations for Mutations
/// 
/// This module implements support for nested relation operations within mutations,
/// including connect, create, and link operations that maintain referential integrity.

use std::collections::HashMap;
use serde_json::Value as JsonValue;
use crate::sqlite::types::Value;
use crate::sqlite::error::{SqlError, SqlResult};
use crate::orm::mutation::{MutationM, MutationF};

/// Nested relation operations
#[derive(Debug, Clone)]
pub enum NestedRelationF<A> {
    /// Connect existing nodes
    Connect {
        relation_field: String,
        ids: Vec<i64>,
        next: A,
    },
    
    /// Create new nodes and connect them
    Create {
        relation_field: String,
        inputs: Vec<HashMap<String, JsonValue>>,
        next: A,
    },
    
    /// Create or connect (upsert relation)
    CreateOrConnect {
        relation_field: String,
        where_clause: HashMap<String, JsonValue>,
        create_input: HashMap<String, JsonValue>,
        next: A,
    },
    
    /// Disconnect existing relations
    Disconnect {
        relation_field: String,
        ids: Vec<i64>,
        next: A,
    },
    
    /// Set relations (replace all existing)
    Set {
        relation_field: String,
        ids: Vec<i64>,
        next: A,
    },
    
    /// Delete connected nodes
    Delete {
        relation_field: String,
        where_clause: HashMap<String, JsonValue>,
        next: A,
    },
    
    /// Update connected nodes
    Update {
        relation_field: String,
        where_clause: HashMap<String, JsonValue>,
        data: HashMap<String, JsonValue>,
        next: A,
    },
    
    /// Nested operations within a relation
    Nested {
        relation_field: String,
        operations: Vec<NestedRelationF<A>>,
        next: A,
    },
}

/// Fixed point for nested relations
pub type NestedRelation = NestedRelationF<()>;

/// Relation metadata for compilation
#[derive(Debug, Clone)]
pub struct RelationMetadata {
    pub name: String,
    pub relation_type: RelationType,
    pub foreign_key: String,
    pub target_table: String,
    pub junction_table: Option<String>,
}

/// Types of relations
#[derive(Debug, Clone, PartialEq)]
pub enum RelationType {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// Nested relation compiler
pub struct NestedRelationCompiler {
    relations: HashMap<String, RelationMetadata>,
    cte_counter: usize,
    bind_counter: usize,
}

impl NestedRelationCompiler {
    /// Create a new nested relation compiler
    pub fn new() -> Self {
        let mut compiler = Self {
            relations: HashMap::new(),
            cte_counter: 0,
            bind_counter: 1,
        };
        
        // Register default relations
        compiler.register_default_relations();
        compiler
    }
    
    /// Register a relation
    pub fn register_relation(&mut self, relation: RelationMetadata) {
        self.relations.insert(relation.name.clone(), relation);
    }
    
    /// Compile nested relation operations
    pub fn compile_nested_relation(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        nested_relation: &NestedRelation,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        match nested_relation {
            NestedRelationF::Connect { relation_field, ids, .. } => {
                self.compile_connect(parent_table, parent_id, relation_field, ids, ctes, binds)
            },
            NestedRelationF::Create { relation_field, inputs, .. } => {
                self.compile_create(parent_table, parent_id, relation_field, inputs, ctes, binds)
            },
            NestedRelationF::CreateOrConnect { relation_field, where_clause, create_input, .. } => {
                self.compile_create_or_connect(
                    parent_table, parent_id, relation_field, where_clause, create_input, ctes, binds
                )
            },
            NestedRelationF::Disconnect { relation_field, ids, .. } => {
                self.compile_disconnect(parent_table, parent_id, relation_field, ids, ctes, binds)
            },
            NestedRelationF::Set { relation_field, ids, .. } => {
                self.compile_set(parent_table, parent_id, relation_field, ids, ctes, binds)
            },
            NestedRelationF::Delete { relation_field, where_clause, .. } => {
                self.compile_delete(parent_table, parent_id, relation_field, where_clause, ctes, binds)
            },
            NestedRelationF::Update { relation_field, where_clause, data, .. } => {
                self.compile_update(parent_table, parent_id, relation_field, where_clause, data, ctes, binds)
            },
            NestedRelationF::Nested { relation_field, operations, .. } => {
                self.compile_nested_operations(parent_table, parent_id, relation_field, operations, ctes, binds)
            },
        }
    }
    
    /// Compile connect operation
    fn compile_connect(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        relation_field: &str,
        ids: &[i64],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_counter = self.cte_counter;
        self.cte_counter += 1;
        let relation = self.get_relation(relation_field)?.clone();
        let cte_name = format!("connect_{}_{}", relation_field, cte_counter);
        
        match relation.relation_type {
            RelationType::OneToMany | RelationType::ManyToOne => {
                // Update foreign key in target table
                let ids_placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                let parent_id_placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                
                binds.push(Value::Text(format!("ARRAY[{}]", 
                    ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","))));
                binds.push(Value::Integer(parent_id));
                
                let cte_sql = format!(
                    "{} AS (UPDATE {} SET {} = {} WHERE id = ANY({}::int[]))",
                    cte_name,
                    relation.target_table,
                    relation.foreign_key,
                    parent_id_placeholder,
                    ids_placeholder
                );
                
                ctes.push(cte_sql);
            },
            RelationType::ManyToMany => {
                // Insert into junction table
                if let Some(junction_table) = &relation.junction_table {
                    let parent_id_placeholder = format!("${}", self.bind_counter);
                    self.bind_counter += 1;
                    let ids_placeholder = format!("${}", self.bind_counter);
                    self.bind_counter += 1;
                    
                    binds.push(Value::Integer(parent_id));
                    binds.push(Value::Text(format!("ARRAY[{}]", 
                        ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","))));
                    
                    let cte_sql = format!(
                        "{} AS (INSERT INTO {} ({}_id, {}_id) SELECT {}, unnest({}::int[]))",
                        cte_name,
                        junction_table,
                        parent_table,
                        relation.target_table,
                        parent_id_placeholder,
                        ids_placeholder
                    );
                    
                    ctes.push(cte_sql);
                } else {
                    return Err(SqlError::runtime_error("Many-to-many relation requires junction table"));
                }
            },
            RelationType::OneToOne => {
                // Update foreign key (similar to OneToMany but with uniqueness constraint)
                let id_placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                let parent_id_placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                
                if ids.len() != 1 {
                    return Err(SqlError::runtime_error("One-to-one relation can only connect to one record"));
                }
                
                binds.push(Value::Integer(ids[0]));
                binds.push(Value::Integer(parent_id));
                
                let cte_sql = format!(
                    "{} AS (UPDATE {} SET {} = {} WHERE id = {})",
                    cte_name,
                    relation.target_table,
                    relation.foreign_key,
                    parent_id_placeholder,
                    id_placeholder
                );
                
                ctes.push(cte_sql);
            },
        }
        
        Ok(())
    }
    
    /// Compile create operation
    fn compile_create(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        relation_field: &str,
        inputs: &[HashMap<String, JsonValue>],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_counter = self.cte_counter;
        self.cte_counter += 1;
        let relation = self.get_relation(relation_field)?.clone();
        let cte_name = format!("create_{}_{}", relation_field, cte_counter);
        
        if inputs.is_empty() {
            return Ok(());
        }
        
        // Create records in target table
        let fields: Vec<String> = inputs[0].keys().cloned().collect();
        let mut value_clauses = Vec::new();
        
        for input in inputs {
            let mut placeholders = Vec::new();
            
            // Add foreign key if it's a direct relation
            if matches!(relation.relation_type, RelationType::OneToMany | RelationType::ManyToOne) {
                placeholders.push(format!("${}", self.bind_counter));
                self.bind_counter += 1;
                binds.push(Value::Integer(parent_id));
            }
            
            // Add input values
            for field in &fields {
                placeholders.push(format!("${}", self.bind_counter));
                self.bind_counter += 1;
                
                if let Some(value) = input.get(field) {
                    binds.push(self.json_value_to_sql_value(value)?);
                } else {
                    binds.push(Value::Null);
                }
            }
            
            value_clauses.push(format!("({})", placeholders.join(", ")));
        }
        
        let mut insert_fields = fields.clone();
        if matches!(relation.relation_type, RelationType::OneToMany | RelationType::ManyToOne) {
            insert_fields.insert(0, relation.foreign_key.clone());
        }
        
        let cte_sql = format!(
            "{} AS (INSERT INTO {} ({}) VALUES {} RETURNING id)",
            cte_name,
            relation.target_table,
            insert_fields.join(", "),
            value_clauses.join(", ")
        );
        
        ctes.push(cte_sql);
        
        // For many-to-many, also create junction table entries
        if relation.relation_type == RelationType::ManyToMany {
            if let Some(junction_table) = &relation.junction_table {
                let junction_cte_name = format!("junction_{}_{}", relation_field, self.cte_counter);
                self.cte_counter += 1;
                
                let parent_id_placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                binds.push(Value::Integer(parent_id));
                
                let junction_sql = format!(
                    "{} AS (INSERT INTO {} ({}_id, {}_id) SELECT {}, id FROM {})",
                    junction_cte_name,
                    junction_table,
                    parent_table,
                    relation.target_table,
                    parent_id_placeholder,
                    cte_name
                );
                
                ctes.push(junction_sql);
            }
        }
        
        Ok(())
    }
    
    /// Compile create or connect operation
    fn compile_create_or_connect(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        relation_field: &str,
        where_clause: &HashMap<String, JsonValue>,
        create_input: &HashMap<String, JsonValue>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_name = format!("create_or_connect_{}_{}", relation_field, self.cte_counter);
        self.cte_counter += 1;
        
        // Build WHERE clause for finding existing record
        let where_conditions: Vec<String> = where_clause.iter()
            .map(|(field, _)| {
                let placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                format!("{} = {}", field, placeholder)
            })
            .collect();
        
        // Add WHERE clause values to binds
        for value in where_clause.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        let relation = self.get_relation(relation_field)?.clone();

        // Create INSERT ... ON CONFLICT for upsert behavior
        let create_fields: Vec<String> = create_input.keys().cloned().collect();
        let mut bind_counter = self.bind_counter;
        let create_placeholders: Vec<String> = (0..create_input.len())
            .map(|_| {
                let placeholder = format!("${}", bind_counter);
                bind_counter += 1;
                placeholder
            })
            .collect();
        self.bind_counter = bind_counter;
        
        // Add create input values to binds
        for value in create_input.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        let conflict_fields: Vec<String> = where_clause.keys().cloned().collect();
        
        let cte_sql = format!(
            "{} AS (INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO NOTHING RETURNING id)",
            cte_name,
            relation.target_table,
            create_fields.join(", "),
            create_placeholders.join(", "),
            conflict_fields.join(", ")
        );
        
        ctes.push(cte_sql);
        
        // Then connect the found/created record
        let cte_counter = self.cte_counter;
        self.cte_counter += 1;
        let connect_cte_name = format!("connect_found_{}_{}", relation_field, cte_counter);
        
        let parent_id_placeholder = format!("${}", self.bind_counter);
        self.bind_counter += 1;
        binds.push(Value::Integer(parent_id));
        
        let connect_sql = match relation.relation_type {
            RelationType::ManyToMany => {
                if let Some(junction_table) = &relation.junction_table {
                    format!(
                        "{} AS (INSERT INTO {} ({}_id, {}_id) SELECT {}, id FROM {} UNION SELECT {}, id FROM {} WHERE {})",
                        connect_cte_name,
                        junction_table,
                        parent_table,
                        relation.target_table,
                        parent_id_placeholder,
                        cte_name,
                        parent_id_placeholder,
                        relation.target_table,
                        where_conditions.join(" AND ")
                    )
                } else {
                    return Err(SqlError::runtime_error("Many-to-many relation requires junction table"));
                }
            },
            _ => {
                format!(
                    "{} AS (UPDATE {} SET {} = {} WHERE id IN (SELECT id FROM {} UNION SELECT id FROM {} WHERE {}))",
                    connect_cte_name,
                    relation.target_table,
                    relation.foreign_key,
                    parent_id_placeholder,
                    cte_name,
                    relation.target_table,
                    where_conditions.join(" AND ")
                )
            },
        };
        
        ctes.push(connect_sql);
        
        Ok(())
    }
    
    /// Compile disconnect operation
    fn compile_disconnect(
        &mut self,
        _parent_table: &str,
        _parent_id: i64,
        relation_field: &str,
        ids: &[i64],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_counter = self.cte_counter;
        self.cte_counter += 1;
        let relation = self.get_relation(relation_field)?.clone();
        let cte_name = format!("disconnect_{}_{}", relation_field, cte_counter);
        
        match relation.relation_type {
            RelationType::OneToMany | RelationType::ManyToOne | RelationType::OneToOne => {
                // Set foreign key to NULL
                let ids_placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                
                binds.push(Value::Text(format!("ARRAY[{}]", 
                    ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","))));
                
                let cte_sql = format!(
                    "{} AS (UPDATE {} SET {} = NULL WHERE id = ANY({}::int[]))",
                    cte_name,
                    relation.target_table,
                    relation.foreign_key,
                    ids_placeholder
                );
                
                ctes.push(cte_sql);
            },
            RelationType::ManyToMany => {
                // Delete from junction table
                if let Some(junction_table) = &relation.junction_table {
                    let ids_placeholder = format!("${}", self.bind_counter);
                    self.bind_counter += 1;
                    
                    binds.push(Value::Text(format!("ARRAY[{}]", 
                        ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","))));
                    
                    let cte_sql = format!(
                        "{} AS (DELETE FROM {} WHERE {}_id = ANY({}::int[]))",
                        cte_name,
                        junction_table,
                        relation.target_table,
                        ids_placeholder
                    );
                    
                    ctes.push(cte_sql);
                } else {
                    return Err(SqlError::runtime_error("Many-to-many relation requires junction table"));
                }
            },
        }
        
        Ok(())
    }
    
    /// Compile set operation (replace all relations)
    fn compile_set(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        relation_field: &str,
        ids: &[i64],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        // First disconnect all existing relations
        self.compile_disconnect_all(parent_table, parent_id, relation_field, ctes, binds)?;
        
        // Then connect the new ones
        if !ids.is_empty() {
            self.compile_connect(parent_table, parent_id, relation_field, ids, ctes, binds)?;
        }
        
        Ok(())
    }
    
    /// Compile delete operation
    fn compile_delete(
        &mut self,
        _parent_table: &str,
        _parent_id: i64,
        relation_field: &str,
        where_clause: &HashMap<String, JsonValue>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let cte_counter = self.cte_counter;
        self.cte_counter += 1;
        let relation = self.get_relation(relation_field)?.clone();
        let cte_name = format!("delete_{}_{}", relation_field, cte_counter);
        
        let mut bind_counter = self.bind_counter;
        let where_conditions: Vec<String> = where_clause.iter()
            .map(|(field, _)| {
                let placeholder = format!("${}", bind_counter);
                bind_counter += 1;
                format!("{} = {}", field, placeholder)
            })
            .collect();
        self.bind_counter = bind_counter;
        
        for value in where_clause.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        let cte_sql = format!(
            "{} AS (DELETE FROM {} WHERE {})",
            cte_name,
            relation.target_table,
            where_conditions.join(" AND ")
        );
        
        ctes.push(cte_sql);
        
        Ok(())
    }
    
    /// Compile update operation
    fn compile_update(
        &mut self,
        _parent_table: &str,
        _parent_id: i64,
        relation_field: &str,
        where_clause: &HashMap<String, JsonValue>,
        data: &HashMap<String, JsonValue>,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let relation = self.get_relation(relation_field)?.clone();
        let cte_name = format!("update_{}_{}", relation_field, self.cte_counter);
        self.cte_counter += 1;
        
        let set_clauses: Vec<String> = data.iter()
            .map(|(field, _)| {
                let placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                format!("{} = {}", field, placeholder)
            })
            .collect();
        
        let where_conditions: Vec<String> = where_clause.iter()
            .map(|(field, _)| {
                let placeholder = format!("${}", self.bind_counter);
                self.bind_counter += 1;
                format!("{} = {}", field, placeholder)
            })
            .collect();
        
        // Add SET values to binds
        for value in data.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        // Add WHERE values to binds
        for value in where_clause.values() {
            binds.push(self.json_value_to_sql_value(value)?);
        }
        
        let cte_sql = format!(
            "{} AS (UPDATE {} SET {} WHERE {})",
            cte_name,
            relation.target_table,
            set_clauses.join(", "),
            where_conditions.join(" AND ")
        );
        
        ctes.push(cte_sql);
        
        Ok(())
    }
    
    /// Compile nested operations
    fn compile_nested_operations(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        relation_field: &str,
        operations: &[NestedRelationF<()>],
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        for operation in operations {
            self.compile_nested_relation(parent_table, parent_id, operation, ctes, binds)?;
        }
        Ok(())
    }
    
    /// Disconnect all existing relations
    fn compile_disconnect_all(
        &mut self,
        parent_table: &str,
        parent_id: i64,
        relation_field: &str,
        ctes: &mut Vec<String>,
        binds: &mut Vec<Value>,
    ) -> SqlResult<()> {
        let relation = self.get_relation(relation_field)?.clone();
        let cte_name = format!("disconnect_all_{}_{}", relation_field, self.cte_counter);
        self.cte_counter += 1;
        
        let parent_id_placeholder = format!("${}", self.bind_counter);
        self.bind_counter += 1;
        binds.push(Value::Integer(parent_id));
        
        match relation.relation_type {
            RelationType::OneToMany | RelationType::ManyToOne | RelationType::OneToOne => {
                let cte_sql = format!(
                    "{} AS (UPDATE {} SET {} = NULL WHERE {} = {})",
                    cte_name,
                    relation.target_table,
                    relation.foreign_key,
                    relation.foreign_key,
                    parent_id_placeholder
                );
                
                ctes.push(cte_sql);
            },
            RelationType::ManyToMany => {
                if let Some(junction_table) = &relation.junction_table {
                    let cte_sql = format!(
                        "{} AS (DELETE FROM {} WHERE {}_id = {})",
                        cte_name,
                        junction_table,
                        parent_table,
                        parent_id_placeholder
                    );
                    
                    ctes.push(cte_sql);
                } else {
                    return Err(SqlError::runtime_error("Many-to-many relation requires junction table"));
                }
            },
        }
        
        Ok(())
    }
    
    /// Get relation metadata
    fn get_relation(&self, relation_field: &str) -> SqlResult<&RelationMetadata> {
        self.relations.get(relation_field)
            .ok_or_else(|| SqlError::runtime_error(&format!("Unknown relation: {}", relation_field)))
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
    
    /// Register default relations
    fn register_default_relations(&mut self) {
        // User -> Posts (one-to-many)
        self.register_relation(RelationMetadata {
            name: "posts".to_string(),
            relation_type: RelationType::OneToMany,
            foreign_key: "user_id".to_string(),
            target_table: "posts".to_string(),
            junction_table: None,
        });
        
        // Post -> Categories (many-to-many)
        self.register_relation(RelationMetadata {
            name: "categories".to_string(),
            relation_type: RelationType::ManyToMany,
            foreign_key: "".to_string(), // Not used for many-to-many
            target_table: "categories".to_string(),
            junction_table: Some("post_categories".to_string()),
        });
        
        // User -> Profile (one-to-one)
        self.register_relation(RelationMetadata {
            name: "profile".to_string(),
            relation_type: RelationType::OneToOne,
            foreign_key: "user_id".to_string(),
            target_table: "profiles".to_string(),
            junction_table: None,
        });
    }
}

impl Default for NestedRelationCompiler {
    fn default() -> Self {
        Self::new()
    }
}
