/// GraphQL Composable Query and Mutation System
/// 
/// This module implements a composable system for GraphQL queries and mutations
/// that follows the GraphQL specification, allowing queries to be returned from
/// mutations and complex compositions.

use std::collections::HashMap;
use serde_json::{Value as JsonValue, json};
use crate::orm::schema::TypeSafeSchema;
use crate::orm::mutation::{MutationM, Mutation};
use crate::orm::graphql::{MultiRoot, Field, SelectionSet};
use crate::orm::{MultiRootF};
use crate::sqlite::types::Value;
use crate::sqlite::error::{SqlResult, SqlError};

/// Composable GraphQL operation that can be either a query or mutation
#[derive(Debug, Clone)]
pub enum GraphQLOperation {
    Query(ComposableQuery),
    Mutation(ComposableMutation),
    Subscription(ComposableSubscription),
}

/// Composable query that can be nested and combined
#[derive(Debug, Clone)]
pub struct ComposableQuery {
    pub selection_set: SelectionSet,
    pub variables: HashMap<String, JsonValue>,
    pub fragments: HashMap<String, Fragment>,
    pub directives: Vec<Directive>,
}

/// Composable mutation that can return queries
#[derive(Debug, Clone)]
pub struct ComposableMutation {
    pub operations: Vec<MutationOperation>,
    pub return_query: Option<ComposableQuery>,
    pub variables: HashMap<String, JsonValue>,
    pub directives: Vec<Directive>,
}

/// Subscription for real-time updates (GraphQL spec compliant)
#[derive(Debug, Clone)]
pub struct ComposableSubscription {
    pub selection_set: SelectionSet,
    pub variables: HashMap<String, JsonValue>,
    pub directives: Vec<Directive>,
}

/// Individual mutation operation
#[derive(Debug, Clone)]
pub struct MutationOperation {
    pub operation_type: MutationOperationType,
    pub input: HashMap<String, JsonValue>,
    pub where_clause: Option<HashMap<String, JsonValue>>,
    pub return_fields: Vec<String>,
}

/// Types of mutation operations according to GraphQL best practices
#[derive(Debug, Clone)]
pub enum MutationOperationType {
    Create {
        table: String,
        input_variable: Option<String>, // e.g., "$input" for create(input: $input)
    },
    Update {
        table: String,
        input_variable: Option<String>, // e.g., "$input" for update(input: $input)
        where_variable: Option<String>, // e.g., "$where" for update(where: $where)
    },
    Delete {
        table: String,
        where_variable: Option<String>, // e.g., "$where" for delete(where: $where)
    },
    Upsert {
        table: String,
        conflict_fields: Vec<String>,
        input_variable: Option<String>, // e.g., "$input" for upsert(input: $input)
    },
    Connect {
        parent_table: String,
        child_table: String,
        parent_variable: Option<String>, // e.g., "$parentId"
        child_variable: Option<String>,  // e.g., "$childId"
    },
    Disconnect {
        parent_table: String,
        child_table: String,
        parent_variable: Option<String>, // e.g., "$parentId"
        child_variable: Option<String>,  // e.g., "$childId"
    },
    Custom {
        sql_template: String,
        variables: HashMap<String, String>, // Variable mappings for custom SQL
    },
}

/// GraphQL fragment for reusable selection sets
#[derive(Debug, Clone)]
pub struct Fragment {
    pub name: String,
    pub type_condition: String,
    pub selection_set: SelectionSet,
    pub directives: Vec<Directive>,
}

/// GraphQL directive (e.g., @include, @skip, @deprecated)
#[derive(Debug, Clone)]
pub struct Directive {
    pub name: String,
    pub arguments: HashMap<String, JsonValue>,
}

/// Result of executing a composable operation
#[derive(Debug, Clone)]
pub struct ComposableResult {
    pub data: Option<JsonValue>,
    pub errors: Vec<GraphQLError>,
    pub extensions: HashMap<String, JsonValue>,
}

/// GraphQL error following the spec
#[derive(Debug, Clone)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Vec<SourceLocation>,
    pub path: Vec<PathSegment>,
    pub extensions: HashMap<String, JsonValue>,
}

/// Source location in GraphQL document
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
}

/// Path segment for error reporting
#[derive(Debug, Clone)]
pub enum PathSegment {
    Field(String),
    Index(usize),
}

/// Composable GraphQL executor
pub struct ComposableExecutor {
    schema: TypeSafeSchema,
    mutation_compiler: crate::orm::mutation_compiler::AdvancedMutationCompiler,
    query_compiler: crate::orm::graphql_compiler::AdvancedGraphQLCompiler,
}

impl ComposableExecutor {
    /// Create a new composable executor
    pub fn new(schema: TypeSafeSchema) -> Self {
        let mutation_compiler = crate::orm::mutation_compiler::AdvancedMutationCompiler::new(
            crate::orm::mutation_compiler::MutationOptimizationLevel::Aggressive
        );
        let query_compiler = crate::orm::graphql_compiler::AdvancedGraphQLCompiler::new(
            schema.clone(),
            crate::orm::graphql_compiler::OptimizationLevel::Aggressive
        );
        
        Self {
            schema,
            mutation_compiler,
            query_compiler,
        }
    }
    
    /// Execute a composable operation
    pub async fn execute(&mut self, operation: GraphQLOperation) -> SqlResult<ComposableResult> {
        match operation {
            GraphQLOperation::Query(query) => self.execute_query(query).await,
            GraphQLOperation::Mutation(mutation) => self.execute_mutation(mutation).await,
            GraphQLOperation::Subscription(subscription) => self.execute_subscription(subscription).await,
        }
    }
    
    /// Execute a composable query
    async fn execute_query(&mut self, query: ComposableQuery) -> SqlResult<ComposableResult> {
        // Apply directives
        let processed_selection = self.apply_directives(&query.selection_set, &query.directives, &query.variables)?;
        
        // Resolve fragments
        let resolved_selection = self.resolve_fragments(processed_selection, &query.fragments)?;
        
        // Convert to MultiRoot for compilation
        let multi_root = self.selection_set_to_multi_root(resolved_selection)?;
        
        // Compile and execute (simplified for now)
        let _compiled_sql = multi_root; // Would use actual compiler in real implementation
        let result = serde_json::json!({"data": "query_result"});
        
        Ok(ComposableResult {
            data: Some(result),
            errors: Vec::new(),
            extensions: HashMap::new(),
        })
    }
    
    /// Execute a composable mutation
    async fn execute_mutation(&mut self, mutation: ComposableMutation) -> SqlResult<ComposableResult> {
        let mut mutation_results = Vec::new();
        
        // Execute each mutation operation (simplified for now)
        for operation in &mutation.operations {
            let _mutation_m = self.operation_to_mutation_m(operation)?;
            // Simplified execution - would use actual compiler in real implementation
            let result = serde_json::json!({"id": 1, "affected_rows": 1});
            mutation_results.push(result);
        }
        
        // If there's a return query, execute it with the mutation results
        let final_result = if let Some(return_query) = mutation.return_query {
            // Inject mutation results into query variables
            let mut enhanced_query = return_query;
            for (i, result) in mutation_results.iter().enumerate() {
                enhanced_query.variables.insert(
                    format!("mutation_result_{}", i),
                    result.clone()
                );
            }
            
            self.execute_query(enhanced_query).await?
        } else {
            ComposableResult {
                data: Some(JsonValue::Array(mutation_results)),
                errors: Vec::new(),
                extensions: HashMap::new(),
            }
        };
        
        Ok(final_result)
    }
    
    /// Execute a subscription (placeholder for real-time functionality)
    async fn execute_subscription(&mut self, _subscription: ComposableSubscription) -> SqlResult<ComposableResult> {
        // Subscriptions would require a real-time event system
        // For now, return a placeholder
        Ok(ComposableResult {
            data: Some(JsonValue::Object(serde_json::Map::new())),
            errors: vec![GraphQLError {
                message: "Subscriptions not yet implemented".to_string(),
                locations: Vec::new(),
                path: Vec::new(),
                extensions: HashMap::new(),
            }],
            extensions: HashMap::new(),
        })
    }
    
    /// Apply GraphQL directives to selection set
    fn apply_directives(
        &self,
        selection_set: &SelectionSet,
        directives: &[Directive],
        variables: &HashMap<String, JsonValue>,
    ) -> SqlResult<SelectionSet> {
        let mut processed_fields = Vec::new();
        
        for field in &selection_set.fields {
            // For now, include all fields since directive processing needs proper Field structure
            // TODO: Implement proper directive support when Field structure supports it
            processed_fields.push(field.clone());
        }
        
        Ok(SelectionSet {
            fields: processed_fields,
        })
    }
    
    /// Evaluate a condition for directive processing
    fn evaluate_condition(&self, condition: &JsonValue, variables: &HashMap<String, JsonValue>) -> SqlResult<bool> {
        match condition {
            JsonValue::Bool(b) => Ok(*b),
            JsonValue::String(var_name) if var_name.starts_with('$') => {
                let var_name = &var_name[1..];
                variables.get(var_name)
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| SqlError::runtime_error(format!("Variable ${} not found or not boolean", var_name)))
            },
            _ => Err(SqlError::runtime_error("Invalid condition in directive".to_string())),
        }
    }
    
    /// Resolve fragments in selection set
    fn resolve_fragments(&self, selection_set: SelectionSet, fragments: &HashMap<String, Fragment>) -> SqlResult<SelectionSet> {
        let mut resolved_fields = Vec::new();
        
        for field in selection_set.fields {
            // Handle fragment spreads
            if field.name.starts_with("...") {
                let fragment_name = &field.name[3..];
                if let Some(fragment) = fragments.get(fragment_name) {
                    // Add fragment fields
                    for fragment_field in &fragment.selection_set.fields {
                        resolved_fields.push(fragment_field.clone());
                    }
                } else {
                    return Err(SqlError::runtime_error(format!("Fragment {} not found", fragment_name)));
                }
            } else {
                resolved_fields.push(field);
            }
        }
        
        Ok(SelectionSet {
            fields: resolved_fields,
        })
    }
    
    /// Convert selection set to MultiRoot for compilation
    fn selection_set_to_multi_root(&self, selection_set: SelectionSet) -> SqlResult<MultiRoot> {
        // This would convert the selection set to the internal MultiRoot format
        // For now, create a simple conversion
        let roots: Vec<MultiRootF<()>> = selection_set.fields.into_iter().map(|field| {
            MultiRootF::Root {
                name: field.name,
                selection: field.selection_set.unwrap_or_else(|| SelectionSet { fields: Vec::new() }),
                args: field.args,
                next: (),
            }
        }).collect();

        // Return the first root or create an empty one
        Ok(roots.into_iter().next().unwrap_or_else(|| MultiRootF::Root {
            name: "empty".to_string(),
            selection: SelectionSet { fields: Vec::new() },
            args: HashMap::new(),
            next: (),
        }))
    }
    
    /// Convert mutation operation to MutationM
    fn operation_to_mutation_m(&self, operation: &MutationOperation) -> SqlResult<MutationM<JsonValue>> {
        // For now, create a simplified mutation that returns JsonValue
        // In a real implementation, we would properly convert between types
        let mutation_data = match &operation.operation_type {
            MutationOperationType::Create { table, input_variable } => {
                json!({
                    "type": "create",
                    "table": table,
                    "input_variable": input_variable,
                    "input": operation.input
                })
            },
            MutationOperationType::Update { table, input_variable, where_variable } => {
                json!({
                    "type": "update",
                    "table": table,
                    "input_variable": input_variable,
                    "where_variable": where_variable,
                    "input": operation.input,
                    "where": operation.where_clause
                })
            },
            MutationOperationType::Delete { table, where_variable } => {
                json!({
                    "type": "delete",
                    "table": table,
                    "where_variable": where_variable,
                    "where": operation.where_clause
                })
            },
            MutationOperationType::Upsert { table, conflict_fields, input_variable } => {
                json!({
                    "type": "upsert",
                    "table": table,
                    "input_variable": input_variable,
                    "input": operation.input,
                    "conflict_fields": conflict_fields
                })
            },
            MutationOperationType::Connect { parent_table, child_table, parent_variable, child_variable } => {
                json!({
                    "type": "connect",
                    "parent_table": parent_table,
                    "child_table": child_table,
                    "parent_variable": parent_variable,
                    "child_variable": child_variable,
                    "input": operation.input
                })
            },
            MutationOperationType::Disconnect { parent_table, child_table, parent_variable, child_variable } => {
                json!({
                    "type": "disconnect",
                    "parent_table": parent_table,
                    "child_table": child_table,
                    "parent_variable": parent_variable,
                    "child_variable": child_variable,
                    "input": operation.input
                })
            },
            MutationOperationType::Custom { sql_template, variables } => {
                json!({
                    "type": "custom",
                    "sql": sql_template,
                    "variables": variables,
                    "input": operation.input
                })
            },
        };

        // Create a simplified mutation wrapper
        // In a real implementation, this would properly convert to the correct MutationM type
        let _custom_mutation: MutationM<()> = MutationM::<()>::custom(
            format!("-- GraphQL Composable Operation: {}", mutation_data),
            vec![Value::Text(mutation_data.to_string())]
        );

        // Convert to JsonValue type by using the custom mutation constructor
        let custom_mutation = MutationM::<()>::custom(
            format!("-- GraphQL Composable Operation: {}", mutation_data),
            vec![Value::Text(mutation_data.to_string())]
        );

        // Transform to JsonValue mutation by creating a new one with the data
        Ok(MutationM::pure(mutation_data))
    }
    
    /// Convert JSON value to SQL value
    fn json_to_sql_value(&self, json: &JsonValue) -> SqlResult<Value> {
        match json {
            JsonValue::String(s) => Ok(Value::Text(s.clone())),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::Real(f))
                } else {
                    Err(SqlError::runtime_error("Invalid number format".to_string()))
                }
            },
            JsonValue::Bool(b) => Ok(Value::Boolean(*b)),
            JsonValue::Null => Ok(Value::Null),
            _ => Err(SqlError::runtime_error("Unsupported JSON type for SQL conversion".to_string())),
        }
    }
}

/// Builder for composable queries
pub struct ComposableQueryBuilder {
    selection_set: SelectionSet,
    variables: HashMap<String, JsonValue>,
    fragments: HashMap<String, Fragment>,
    directives: Vec<Directive>,
}

impl ComposableQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            selection_set: SelectionSet { fields: Vec::new() },
            variables: HashMap::new(),
            fragments: HashMap::new(),
            directives: Vec::new(),
        }
    }

    /// Add a field to the selection set
    pub fn field(mut self, name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(name.into(), self)
    }

    /// Add a variable with its value
    pub fn variable(mut self, name: impl Into<String>, value: JsonValue) -> Self {
        self.variables.insert(name.into(), value);
        self
    }

    /// Add multiple variables at once
    pub fn variables(mut self, variables: HashMap<String, JsonValue>) -> Self {
        self.variables.extend(variables);
        self
    }

    /// Add a variable reference in a field argument (e.g., field(id: $userId))
    pub fn field_with_variable(mut self, field_name: impl Into<String>, arg_name: impl Into<String>, variable_name: impl Into<String>) -> FieldBuilder {
        let mut field_builder = FieldBuilder::new(field_name.into(), self);
        field_builder.field.args.insert(
            arg_name.into(),
            json!(format!("${}", variable_name.into()))
        );
        field_builder
    }

    /// Add a fragment
    pub fn fragment(mut self, fragment: Fragment) -> Self {
        self.fragments.insert(fragment.name.clone(), fragment);
        self
    }

    /// Add a directive
    pub fn directive(mut self, directive: Directive) -> Self {
        self.directives.push(directive);
        self
    }

    /// Build the composable query
    pub fn build(self) -> ComposableQuery {
        ComposableQuery {
            selection_set: self.selection_set,
            variables: self.variables,
            fragments: self.fragments,
            directives: self.directives,
        }
    }
}

/// Builder for fields in a query
pub struct FieldBuilder {
    field: Field,
    parent: ComposableQueryBuilder,
}

impl FieldBuilder {
    fn new(name: String, parent: ComposableQueryBuilder) -> Self {
        Self {
            field: Field {
                name,
                alias: None,
                args: HashMap::new(),
                selection_set: None,
                custom_expr: None,
            },
            parent,
        }
    }

    /// Set field alias
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.field.alias = Some(alias.into());
        self
    }

    /// Add argument to field
    pub fn argument(mut self, name: impl Into<String>, value: JsonValue) -> Self {
        self.field.args.insert(name.into(), value);
        self
    }

    /// Add directive to field (simplified - Field doesn't support directives yet)
    pub fn directive(self, _directive: Directive) -> Self {
        // TODO: Add directive support when Field structure supports it
        self
    }

    /// Add nested selection set
    pub fn selection(mut self, selection_set: SelectionSet) -> Self {
        self.field.selection_set = Some(selection_set);
        self
    }

    /// Finish building this field and return to parent
    pub fn end_field(mut self) -> ComposableQueryBuilder {
        self.parent.selection_set.fields.push(self.field);
        self.parent
    }
}

/// Builder for composable mutations
pub struct ComposableMutationBuilder {
    operations: Vec<MutationOperation>,
    return_query: Option<ComposableQuery>,
    variables: HashMap<String, JsonValue>,
    directives: Vec<Directive>,
}

impl ComposableMutationBuilder {
    /// Create a new mutation builder
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            return_query: None,
            variables: HashMap::new(),
            directives: Vec::new(),
        }
    }

    /// Add a create operation
    pub fn create(mut self, table: impl Into<String>) -> MutationOperationBuilder {
        MutationOperationBuilder::new(
            MutationOperationType::Create {
                table: table.into(),
                input_variable: None,
            },
            self,
        )
    }

    /// Add an update operation
    pub fn update(mut self, table: impl Into<String>) -> MutationOperationBuilder {
        MutationOperationBuilder::new(
            MutationOperationType::Update {
                table: table.into(),
                input_variable: None,
                where_variable: None,
            },
            self,
        )
    }

    /// Add a delete operation
    pub fn delete(mut self, table: impl Into<String>) -> MutationOperationBuilder {
        MutationOperationBuilder::new(
            MutationOperationType::Delete {
                table: table.into(),
                where_variable: None,
            },
            self,
        )
    }

    /// Add an upsert operation
    pub fn upsert(mut self, table: impl Into<String>, conflict_fields: Vec<String>) -> MutationOperationBuilder {
        MutationOperationBuilder::new(
            MutationOperationType::Upsert {
                table: table.into(),
                conflict_fields,
                input_variable: None,
            },
            self,
        )
    }

    /// Set a return query to execute after mutations
    pub fn return_query(mut self, query: ComposableQuery) -> Self {
        self.return_query = Some(query);
        self
    }

    /// Add a variable with its value for use in mutations
    pub fn variable(mut self, name: impl Into<String>, value: JsonValue) -> Self {
        self.variables.insert(name.into(), value);
        self
    }

    /// Add multiple variables at once
    pub fn variables(mut self, variables: HashMap<String, JsonValue>) -> Self {
        self.variables.extend(variables);
        self
    }

    /// Create a mutation with variable-based input
    pub fn create_with_variables(mut self, table: impl Into<String>, input_variable: impl Into<String>) -> MutationOperationBuilder {
        MutationOperationBuilder::new(
            MutationOperationType::Create {
                table: table.into(),
                input_variable: Some(input_variable.into()),
            },
            self,
        )
    }

    /// Update a mutation with variable-based input and where clause
    pub fn update_with_variables(mut self, table: impl Into<String>, input_variable: impl Into<String>, where_variable: Option<String>) -> MutationOperationBuilder {
        MutationOperationBuilder::new(
            MutationOperationType::Update {
                table: table.into(),
                input_variable: Some(input_variable.into()),
                where_variable,
            },
            self,
        )
    }

    /// Delete with variable-based where clause
    pub fn delete_with_variables(mut self, table: impl Into<String>, where_variable: impl Into<String>) -> MutationOperationBuilder {
        MutationOperationBuilder::new(
            MutationOperationType::Delete {
                table: table.into(),
                where_variable: Some(where_variable.into()),
            },
            self,
        )
    }



    /// Build the composable mutation
    pub fn build(self) -> ComposableMutation {
        ComposableMutation {
            operations: self.operations,
            return_query: self.return_query,
            variables: self.variables,
            directives: self.directives,
        }
    }
}

/// Builder for individual mutation operations
pub struct MutationOperationBuilder {
    operation_type: MutationOperationType,
    input: HashMap<String, JsonValue>,
    where_clause: Option<HashMap<String, JsonValue>>,
    return_fields: Vec<String>,
    parent: ComposableMutationBuilder,
}

impl MutationOperationBuilder {
    fn new(operation_type: MutationOperationType, parent: ComposableMutationBuilder) -> Self {
        Self {
            operation_type,
            input: HashMap::new(),
            where_clause: None,
            return_fields: Vec::new(),
            parent,
        }
    }

    /// Set input data
    pub fn input(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        self.input.insert(key.into(), value);
        self
    }

    /// Set where clause
    pub fn where_clause(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        if self.where_clause.is_none() {
            self.where_clause = Some(HashMap::new());
        }
        self.where_clause.as_mut().unwrap().insert(key.into(), value);
        self
    }

    /// Add return field
    pub fn return_field(mut self, field: impl Into<String>) -> Self {
        self.return_fields.push(field.into());
        self
    }

    /// Start building a nested return query fragment
    pub fn return_query_fragment(mut self) -> ReturnQueryBuilder {
        ReturnQueryBuilder::new(self)
    }

    /// Finish building this operation and return to parent
    pub fn end_operation(mut self) -> ComposableMutationBuilder {
        let operation = MutationOperation {
            operation_type: self.operation_type,
            input: self.input,
            where_clause: self.where_clause,
            return_fields: self.return_fields,
        };
        self.parent.operations.push(operation);
        self.parent
    }
}

/// Builder for return query fragments in mutations
pub struct ReturnQueryBuilder {
    parent: MutationOperationBuilder,
    selection_set: SelectionSet,
}

impl ReturnQueryBuilder {
    fn new(parent: MutationOperationBuilder) -> Self {
        Self {
            parent,
            selection_set: SelectionSet { fields: Vec::new() },
        }
    }

    /// Add a field to the return query fragment
    pub fn field(mut self, name: impl Into<String>) -> ReturnFieldBuilder {
        ReturnFieldBuilder::new(name.into(), self)
    }

    /// Add a simple field without nesting
    pub fn simple_field(mut self, name: impl Into<String>) -> Self {
        self.selection_set.fields.push(Field {
            name: name.into(),
            alias: None,
            args: HashMap::new(),
            selection_set: None,
            custom_expr: None,
        });
        self
    }

    /// Finish building the return query and return to parent
    pub fn end_return_query(mut self) -> MutationOperationBuilder {
        // Convert the selection set to return fields (simplified)
        for field in self.selection_set.fields {
            self.parent.return_fields.push(field.name);
        }
        self.parent
    }
}

/// Builder for fields in return query fragments
pub struct ReturnFieldBuilder {
    field: Field,
    parent: ReturnQueryBuilder,
}

impl ReturnFieldBuilder {
    fn new(name: String, parent: ReturnQueryBuilder) -> Self {
        Self {
            field: Field {
                name,
                alias: None,
                args: HashMap::new(),
                selection_set: None,
                custom_expr: None,
            },
            parent,
        }
    }

    /// Add an argument to this field
    pub fn arg(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        self.field.args.insert(key.into(), value);
        self
    }

    /// Add a variable reference as an argument (e.g., id: $userId)
    pub fn variable_arg(mut self, key: impl Into<String>, variable_name: impl Into<String>) -> Self {
        self.field.args.insert(key.into(), json!(format!("${}", variable_name.into())));
        self
    }

    /// Add multiple arguments at once
    pub fn args(mut self, args: HashMap<String, JsonValue>) -> Self {
        self.field.args.extend(args);
        self
    }

    /// Set an alias for this field
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.field.alias = Some(alias.into());
        self
    }

    /// Start building nested fields
    pub fn nested_field(mut self, name: impl Into<String>) -> ReturnFieldBuilder {
        if self.field.selection_set.is_none() {
            self.field.selection_set = Some(SelectionSet { fields: Vec::new() });
        }

        let nested_field = Field {
            name: name.into(),
            alias: None,
            args: HashMap::new(),
            selection_set: None,
            custom_expr: None,
        };

        self.field.selection_set.as_mut().unwrap().fields.push(nested_field);
        self
    }

    /// Finish building this field and return to parent
    pub fn end_field(mut self) -> ReturnQueryBuilder {
        self.parent.selection_set.fields.push(self.field);
        self.parent
    }
}

impl Default for ComposableQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ComposableMutationBuilder {
    fn default() -> Self {
        Self::new()
    }
}
