/// TLisp GraphQL Interface
/// 
/// This module provides TLisp functions for GraphQL query building and execution
/// that can be called from TLisp programs, enabling seamless integration between
/// the GraphQL system and TLisp runtime.

use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value as JsonValue;
use crate::tlisp::Value;
use crate::error::{TlispError, TlispResult};
use crate::orm::graphql::{
    GraphQLCompiler, MultiRoot, MultiRootF, SelectionSet, Field, SqlExpr, SqlExprF, TypeTag,
};
use crate::orm::graphql_parser::{GraphQLParser, CustomFunction};
use crate::orm::graphql_compiler::{AdvancedGraphQLCompiler, OptimizationLevel};
use crate::orm::schema::TypeSafeSchema;
use crate::sqlite::error::SqlResult;

/// TLisp GraphQL runtime context
pub struct TlispGraphQLContext {
    parser: GraphQLParser,
    compiler: AdvancedGraphQLCompiler,
    schema: TypeSafeSchema,
}

impl TlispGraphQLContext {
    /// Create a new TLisp GraphQL context
    pub fn new() -> Self {
        let schema = TypeSafeSchema::new();
        let parser = GraphQLParser::new();
        let compiler = AdvancedGraphQLCompiler::new(schema.clone(), OptimizationLevel::Basic);
        
        Self {
            parser,
            compiler,
            schema,
        }
    }
    
    /// Register a custom function for GraphQL queries
    pub fn register_function(&mut self, name: String, func: CustomFunction) {
        self.parser.register_function(name, func);
    }
}

/// TLisp GraphQL library functions
pub struct TlispGraphQLLibrary {
    context: Arc<std::sync::Mutex<TlispGraphQLContext>>,
}

impl TlispGraphQLLibrary {
    /// Create a new TLisp GraphQL library
    pub fn new() -> Self {
        Self {
            context: Arc::new(std::sync::Mutex::new(TlispGraphQLContext::new())),
        }
    }
    
    /// Parse a GraphQL query from TLisp
    pub fn parse_query(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("parse-graphql-query expects 1 argument".to_string()));
        }

        let query_str = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Query must be a string".to_string())),
        };

        let context = self.context.lock().map_err(|_| TlispError::Runtime("Failed to lock context".to_string()))?;
        let multi_root = context.parser.parse(&query_str)
            .map_err(|e| TlispError::Runtime(format!("GraphQL parse error: {}", e)))?;
        
        // Convert MultiRoot to TLisp value
        self.multi_root_to_tlisp_value(&multi_root)
    }
    
    /// Compile a GraphQL query to SQL
    pub fn compile_query(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("compile-graphql-query expects 1 argument".to_string()));
        }

        let query_str = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Query must be a string".to_string())),
        };

        let context = self.context.lock().map_err(|_| TlispError::Runtime("Failed to lock context".to_string()))?;

        // Parse the query
        let multi_root = context.parser.parse(&query_str)
            .map_err(|e| TlispError::Runtime(format!("GraphQL parse error: {}", e)))?;

        // Compile to execution plan
        let execution_plan = context.compiler.compile_optimized(multi_root)
            .map_err(|e| TlispError::Runtime(format!("GraphQL compile error: {}", e)))?;
        
        // Return compilation result as TLisp value
        Ok(Value::List(vec![
            Value::String("compiled-query".to_string()),
            Value::String(execution_plan.sql),
            Value::List(execution_plan.binds.into_iter().map(|b| self.sql_value_to_tlisp_value(&b)).collect()),
            Value::Float(execution_plan.estimated_cost),
            Value::Int(execution_plan.estimated_rows as i64),
        ]))
    }
    
    /// Build a GraphQL query using TLisp syntax
    pub fn build_query(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.is_empty() {
            return Err(TlispError::Runtime("build-graphql-query expects at least 1 argument".to_string()));
        }
        
        let mut builder = GraphQLQueryBuilder::new();
        
        for arg in args {
            match arg {
                Value::List(ref items) if items.len() >= 2 => {
                    match (&items[0], &items[1]) {
                        (Value::String(cmd), Value::String(name)) => {
                            match cmd.as_str() {
                                "root" => {
                                    builder = builder.root(name);
                                },
                                "field" => {
                                    builder = builder.field(name);
                                },
                                "close" => {
                                    builder = builder.close();
                                },
                                _ => return Err(TlispError::Runtime(format!("Unknown command: {}", cmd))),
                            }
                        },
                        _ => return Err(TlispError::Runtime("Invalid command format".to_string())),
                    }
                },
                _ => return Err(TlispError::Runtime("Commands must be lists".to_string())),
            }
        }
        
        Ok(Value::String(builder.build()))
    }
    
    /// Execute a GraphQL query (simplified implementation)
    pub fn execute_query(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("execute-graphql-query expects 1 argument".to_string()));
        }

        let query_str = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Query must be a string".to_string())),
        };
        
        // For now, return a mock result
        // In a full implementation, this would execute the compiled SQL
        Ok(Value::List(vec![
            Value::String("result".to_string()),
            Value::String(format!("Executed query: {}", query_str)),
            Value::List(vec![
                Value::String("data".to_string()),
                Value::List(vec![]),
            ]),
        ]))
    }
    
    /// Register a custom GraphQL function from TLisp
    pub fn register_function(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime("register-graphql-function expects 3 arguments: name, sql-template, return-type".to_string()));
        }

        let name = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Function name must be a string".to_string())),
        };

        let sql_template = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("SQL template must be a string".to_string())),
        };

        let return_type = match &args[2] {
            Value::String(s) => self.parse_type_tag(s)?,
            _ => return Err(TlispError::Runtime("Return type must be a string".to_string())),
        };
        
        let custom_func = CustomFunction {
            sql_template,
            return_type,
            arg_types: Vec::new(), // Simplified
        };
        
        let mut context = self.context.lock().map_err(|_| TlispError::Runtime("Failed to lock context".to_string()))?;
        context.register_function(name.clone(), custom_func);
        
        Ok(Value::String(format!("Registered function: {}", name)))
    }
    
    /// Create a GraphQL schema from TLisp definition
    pub fn create_schema(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("create-graphql-schema expects 1 argument".to_string()));
        }
        
        // For now, return a success message
        // In a full implementation, this would create a schema from TLisp definition
        Ok(Value::String("Schema created successfully".to_string()))
    }
    
    /// Get all available TLisp GraphQL functions
    pub fn get_functions() -> HashMap<String, fn(&TlispGraphQLLibrary, Vec<Value>) -> TlispResult<Value>> {
        let mut functions = HashMap::new();
        
        functions.insert("parse-graphql-query".to_string(), TlispGraphQLLibrary::parse_query as fn(&TlispGraphQLLibrary, Vec<Value>) -> TlispResult<Value>);
        functions.insert("compile-graphql-query".to_string(), TlispGraphQLLibrary::compile_query as fn(&TlispGraphQLLibrary, Vec<Value>) -> TlispResult<Value>);
        functions.insert("build-graphql-query".to_string(), TlispGraphQLLibrary::build_query as fn(&TlispGraphQLLibrary, Vec<Value>) -> TlispResult<Value>);
        functions.insert("execute-graphql-query".to_string(), TlispGraphQLLibrary::execute_query as fn(&TlispGraphQLLibrary, Vec<Value>) -> TlispResult<Value>);
        functions.insert("register-graphql-function".to_string(), TlispGraphQLLibrary::register_function as fn(&TlispGraphQLLibrary, Vec<Value>) -> TlispResult<Value>);
        functions.insert("create-graphql-schema".to_string(), TlispGraphQLLibrary::create_schema as fn(&TlispGraphQLLibrary, Vec<Value>) -> TlispResult<Value>);
        
        functions
    }
    
    /// Convert MultiRoot to TLisp value
    fn multi_root_to_tlisp_value(&self, multi_root: &MultiRoot) -> TlispResult<Value> {
        match multi_root {
            MultiRootF::Root { name, selection, args, .. } => {
                Ok(Value::List(vec![
                    Value::String("root".to_string()),
                    Value::String(name.clone()),
                    self.selection_set_to_tlisp_value(selection)?,
                    self.args_to_tlisp_value(args)?,
                ]))
            },
            MultiRootF::Combine { left, right, .. } => {
                Ok(Value::List(vec![
                    Value::String("combine".to_string()),
                    self.multi_root_to_tlisp_value(left)?,
                    self.multi_root_to_tlisp_value(right)?,
                ]))
            },
        }
    }
    
    /// Convert SelectionSet to TLisp value
    fn selection_set_to_tlisp_value(&self, selection: &SelectionSet) -> TlispResult<Value> {
        let fields: Result<Vec<Value>, TlispError> = selection.fields.iter()
            .map(|field| self.field_to_tlisp_value(field))
            .collect();
        
        Ok(Value::List(vec![
            Value::String("selection".to_string()),
            Value::List(fields?),
        ]))
    }
    
    /// Convert Field to TLisp value
    fn field_to_tlisp_value(&self, field: &Field) -> TlispResult<Value> {
        let mut result = vec![
            Value::String("field".to_string()),
            Value::String(field.name.clone()),
        ];
        
        if let Some(alias) = &field.alias {
            result.push(Value::String(alias.clone()));
        }
        
        Ok(Value::List(result))
    }
    
    /// Convert arguments to TLisp value
    fn args_to_tlisp_value(&self, args: &HashMap<String, JsonValue>) -> TlispResult<Value> {
        let arg_pairs: Vec<Value> = args.iter().map(|(k, v)| {
            Value::List(vec![
                Value::String(k.clone()),
                self.json_value_to_tlisp_value(v),
            ])
        }).collect();
        
        Ok(Value::List(arg_pairs))
    }
    
    /// Convert JSON value to TLisp value
    fn json_value_to_tlisp_value(&self, value: &JsonValue) -> Value {
        match value {
            JsonValue::String(s) => Value::String(s.clone()),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            },
            JsonValue::Bool(b) => Value::Bool(*b),
            JsonValue::Null => Value::Null,
            JsonValue::Array(arr) => {
                Value::List(arr.iter().map(|v| self.json_value_to_tlisp_value(v)).collect())
            },
            JsonValue::Object(obj) => {
                let pairs: Vec<Value> = obj.iter().map(|(k, v)| {
                    Value::List(vec![
                        Value::String(k.clone()),
                        self.json_value_to_tlisp_value(v),
                    ])
                }).collect();
                Value::List(pairs)
            },
        }
    }
    
    /// Convert SQL value to TLisp value
    fn sql_value_to_tlisp_value(&self, value: &crate::sqlite::types::Value) -> Value {
        match value {
            crate::sqlite::types::Value::Text(s) => Value::String(s.clone()),
            crate::sqlite::types::Value::Integer(i) => Value::Int(*i),
            crate::sqlite::types::Value::Real(f) => Value::Float(*f),
            crate::sqlite::types::Value::Boolean(b) => Value::Bool(*b),
            crate::sqlite::types::Value::Null => Value::Null,
            crate::sqlite::types::Value::Blob(_) => Value::String("blob".to_string()),
        }
    }
    
    /// Parse a type tag from string
    fn parse_type_tag(&self, type_str: &str) -> TlispResult<TypeTag> {
        match type_str {
            "text" => Ok(TypeTag::Text),
            "integer" => Ok(TypeTag::Integer),
            "real" => Ok(TypeTag::Real),
            "boolean" => Ok(TypeTag::Boolean),
            "json" => Ok(TypeTag::Json),
            _ => Err(TlispError::Runtime(format!("Unknown type: {}", type_str))),
        }
    }
}

impl Default for TlispGraphQLLibrary {
    fn default() -> Self {
        Self::new()
    }
}

/// GraphQL query builder for TLisp
pub struct GraphQLQueryBuilder {
    query_parts: Vec<String>,
}

impl GraphQLQueryBuilder {
    pub fn new() -> Self {
        Self {
            query_parts: vec!["query {".to_string()],
        }
    }
    
    pub fn root(mut self, name: &str) -> Self {
        self.query_parts.push(format!("  {} {{", name));
        self
    }
    
    pub fn field(mut self, name: &str) -> Self {
        self.query_parts.push(format!("    {}", name));
        self
    }
    
    pub fn close(mut self) -> Self {
        self.query_parts.push("  }".to_string());
        self
    }
    
    pub fn build(mut self) -> String {
        self.query_parts.push("}".to_string());
        self.query_parts.join("\n")
    }
}

impl Default for GraphQLQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tlisp_graphql_library_creation() {
        let library = TlispGraphQLLibrary::new();
        assert!(!library.context.lock().unwrap().schema.tables.users.id.name.is_empty());
    }
    
    #[test]
    fn test_parse_query_function() {
        let library = TlispGraphQLLibrary::new();
        let args = vec![Value::String("query { users { id name } }".to_string())];
        let result = library.parse_query(args);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_build_query_function() {
        let library = TlispGraphQLLibrary::new();
        let args = vec![
            Value::List(vec![Value::String("root".to_string()), Value::String("users".to_string())]),
            Value::List(vec![Value::String("field".to_string()), Value::String("id".to_string())]),
            Value::List(vec![Value::String("field".to_string()), Value::String("name".to_string())]),
            Value::List(vec![Value::String("close".to_string()), Value::String("".to_string())]),
        ];
        let result = library.build_query(args);
        assert!(result.is_ok());
    }
}
