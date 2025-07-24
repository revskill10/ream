/// GraphQL Query Parser
/// 
/// This module implements parsing of GraphQL queries into the internal
/// algebraic representation used by the Catena-GraphQL-X compiler.

use std::collections::HashMap;
use serde_json::Value as JsonValue;
use graphql_parser::query::{
    Document, Definition, OperationDefinition, Query, SelectionSet, Selection, Field,
    Value as GraphQLValue, Type as GraphQLType,
};
use graphql_parser::parse_query;
use crate::orm::graphql::{
    MultiRoot, MultiRootF, SelectionSet as InternalSelectionSet, Field as InternalField,
    SqlExpr, SqlExprF, TypeTag, BinaryOp,
};
use crate::sqlite::error::{SqlError, SqlResult};

/// GraphQL query parser that converts GraphQL syntax to internal algebra
pub struct GraphQLParser {
    /// Registry of custom functions and their SQL mappings
    custom_functions: HashMap<String, CustomFunction>,
}

/// Custom function definition for GraphQL to SQL mapping
#[derive(Debug, Clone)]
pub struct CustomFunction {
    pub sql_template: String,
    pub return_type: TypeTag,
    pub arg_types: Vec<TypeTag>,
}

impl GraphQLParser {
    /// Create a new GraphQL parser
    pub fn new() -> Self {
        let mut parser = Self {
            custom_functions: HashMap::new(),
        };
        
        // Register built-in custom functions
        parser.register_builtin_functions();
        parser
    }
    
    /// Register a custom function
    pub fn register_function(&mut self, name: String, func: CustomFunction) {
        self.custom_functions.insert(name, func);
    }
    
    /// Parse a GraphQL query string into the internal multi-root representation
    pub fn parse(&self, query_str: &str) -> SqlResult<MultiRoot> {
        let document = parse_query(query_str)
            .map_err(|e| SqlError::runtime_error(&format!("GraphQL parse error: {}", e)))?;
        
        self.convert_document(document)
    }
    
    /// Convert a GraphQL document to multi-root query
    fn convert_document(&self, document: Document<'_, String>) -> SqlResult<MultiRoot> {
        let mut roots = Vec::new();
        
        for definition in document.definitions {
            match definition {
                Definition::Operation(OperationDefinition::Query(query)) => {
                    let converted_roots = self.convert_query(query)?;
                    roots.extend(converted_roots);
                },
                Definition::Operation(OperationDefinition::Mutation(_)) => {
                    return Err(SqlError::runtime_error("Mutations not yet supported"));
                },
                Definition::Operation(OperationDefinition::Subscription(_)) => {
                    return Err(SqlError::runtime_error("Subscriptions not yet supported"));
                },
                Definition::Operation(OperationDefinition::SelectionSet(_)) => {
                    return Err(SqlError::runtime_error("Selection sets not yet supported"));
                },
                Definition::Fragment(_) => {
                    return Err(SqlError::runtime_error("Fragments not yet supported"));
                },
            }
        }
        
        if roots.is_empty() {
            return Ok(MultiRootF::Root {
                name: "empty".to_string(),
                selection: InternalSelectionSet { fields: Vec::new() },
                args: HashMap::new(),
                next: (),
            });
        }
        
        // Combine all roots into a single multi-root query
        let mut roots_iter = roots.into_iter();
        let mut result = roots_iter.next().unwrap();
        for root in roots_iter {
            result = MultiRootF::Combine {
                left: Box::new(result),
                right: Box::new(root),
                next: (),
            };
        }
        
        Ok(result)
    }
    
    /// Convert a GraphQL query to list of root queries
    fn convert_query(&self, query: Query<'_, String>) -> SqlResult<Vec<MultiRoot>> {
        let mut roots = Vec::new();
        
        for selection in query.selection_set.items {
            match selection {
                Selection::Field(field) => {
                    let root = self.convert_field_to_root(field)?;
                    roots.push(root);
                },
                Selection::InlineFragment(_) => {
                    return Err(SqlError::runtime_error("Inline fragments not yet supported"));
                },
                Selection::FragmentSpread(_) => {
                    return Err(SqlError::runtime_error("Fragment spreads not yet supported"));
                },
            }
        }
        
        Ok(roots)
    }
    
    /// Convert a top-level GraphQL field to a root query
    fn convert_field_to_root(&self, field: Field<'_, String>) -> SqlResult<MultiRoot> {
        let args = self.convert_arguments(&field.arguments)?;
        let selection = self.convert_selection_set(&field.selection_set)?;
        
        Ok(MultiRootF::Root {
            name: field.name.to_string(),
            selection,
            args,
            next: (),
        })
    }
    
    /// Convert GraphQL selection set to internal representation
    fn convert_selection_set(&self, selection_set: &SelectionSet<'_, String>) -> SqlResult<InternalSelectionSet> {
        let mut fields = Vec::new();
        
        for selection in &selection_set.items {
            match selection {
                Selection::Field(field) => {
                    let internal_field = self.convert_field(field)?;
                    fields.push(internal_field);
                },
                Selection::InlineFragment(_) => {
                    return Err(SqlError::runtime_error("Inline fragments not yet supported"));
                },
                Selection::FragmentSpread(_) => {
                    return Err(SqlError::runtime_error("Fragment spreads not yet supported"));
                },
            }
        }
        
        Ok(InternalSelectionSet { fields })
    }
    
    /// Convert a GraphQL field to internal representation
    fn convert_field(&self, field: &Field<'_, String>) -> SqlResult<InternalField> {
        let args = self.convert_arguments(&field.arguments)?;
        let selection_set = if field.selection_set.items.is_empty() {
            None
        } else {
            Some(self.convert_selection_set(&field.selection_set)?)
        };
        
        // Check if this is a custom function call
        let custom_expr = if let Some(custom_func) = self.custom_functions.get(&field.name) {
            Some(self.build_custom_function_expr(&field.name, &args, custom_func)?)
        } else {
            None
        };
        
        Ok(InternalField {
            name: field.name.to_string(),
            alias: field.alias.as_ref().map(|s| s.to_string()),
            args,
            selection_set,
            custom_expr,
        })
    }
    
    /// Convert GraphQL arguments to JSON values
    fn convert_arguments(&self, arguments: &[(String, GraphQLValue<'_, String>)]) -> SqlResult<HashMap<String, JsonValue>> {
        let mut args = HashMap::new();
        
        for (name, value) in arguments {
            let json_value = self.convert_graphql_value(value)?;
            args.insert(name.clone(), json_value);
        }
        
        Ok(args)
    }
    
    /// Convert GraphQL value to JSON value
    fn convert_graphql_value(&self, value: &GraphQLValue<'_, String>) -> SqlResult<JsonValue> {
        match value {
            GraphQLValue::Variable(_) => {
                Err(SqlError::runtime_error("Variables not yet supported"))
            },
            GraphQLValue::Int(i) => {
                Ok(JsonValue::Number(serde_json::Number::from(i.as_i64().unwrap_or(0))))
            },
            GraphQLValue::Float(f) => {
                Ok(JsonValue::Number(serde_json::Number::from_f64(*f)
                    .ok_or_else(|| SqlError::runtime_error("Invalid float value"))?))
            },
            GraphQLValue::String(s) => {
                Ok(JsonValue::String(s.clone()))
            },
            GraphQLValue::Boolean(b) => {
                Ok(JsonValue::Bool(*b))
            },
            GraphQLValue::Null => {
                Ok(JsonValue::Null)
            },
            GraphQLValue::Enum(e) => {
                Ok(JsonValue::String(e.clone()))
            },
            GraphQLValue::List(items) => {
                let mut json_items = Vec::new();
                for item in items {
                    json_items.push(self.convert_graphql_value(item)?);
                }
                Ok(JsonValue::Array(json_items))
            },
            GraphQLValue::Object(obj) => {
                let mut json_obj = serde_json::Map::new();
                for (key, val) in obj {
                    json_obj.insert(key.clone(), self.convert_graphql_value(val)?);
                }
                Ok(JsonValue::Object(json_obj))
            },
        }
    }
    
    /// Build a custom function SQL expression
    fn build_custom_function_expr(
        &self,
        _func_name: &str,
        args: &HashMap<String, JsonValue>,
        custom_func: &CustomFunction,
    ) -> SqlResult<SqlExpr> {
        // Simple template substitution - in practice would be more sophisticated
        let mut sql = custom_func.sql_template.clone();
        
        // Replace argument placeholders
        for (arg_name, arg_value) in args {
            let placeholder = format!("{{{}}}", arg_name);
            let sql_value = match arg_value {
                JsonValue::String(s) => format!("'{}'", s.replace("'", "''")),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Null => "NULL".to_string(),
                _ => return Err(SqlError::runtime_error(&format!("Unsupported argument type for {}", arg_name))),
            };
            sql = sql.replace(&placeholder, &sql_value);
        }
        
        Ok(SqlExprF::Custom {
            sql,
            binds: Vec::new(),
            ty: custom_func.return_type.clone(),
            next: (),
        })
    }
    
    /// Register built-in custom functions
    fn register_builtin_functions(&mut self) {
        // score_rank() function
        self.register_function("score_rank".to_string(), CustomFunction {
            sql_template: "score_rank()".to_string(),
            return_type: TypeTag::Real,
            arg_types: Vec::new(),
        });
        
        // count(posts) aggregation
        self.register_function("count".to_string(), CustomFunction {
            sql_template: "count(*)".to_string(),
            return_type: TypeTag::Integer,
            arg_types: Vec::new(),
        });
        
        // jsonb_agg aggregation
        self.register_function("jsonb_agg".to_string(), CustomFunction {
            sql_template: "jsonb_agg({column})".to_string(),
            return_type: TypeTag::Json,
            arg_types: vec![TypeTag::Json],
        });
        
        // array_agg aggregation
        self.register_function("array_agg".to_string(), CustomFunction {
            sql_template: "array_agg({column})".to_string(),
            return_type: TypeTag::Array(Box::new(TypeTag::Json)),
            arg_types: vec![TypeTag::Json],
        });
        
        // sum aggregation
        self.register_function("sum".to_string(), CustomFunction {
            sql_template: "sum({column})".to_string(),
            return_type: TypeTag::Real,
            arg_types: vec![TypeTag::Real],
        });
        
        // avg aggregation
        self.register_function("avg".to_string(), CustomFunction {
            sql_template: "avg({column})".to_string(),
            return_type: TypeTag::Real,
            arg_types: vec![TypeTag::Real],
        });
        
        // max aggregation
        self.register_function("max".to_string(), CustomFunction {
            sql_template: "max({column})".to_string(),
            return_type: TypeTag::Json, // Generic type
            arg_types: vec![TypeTag::Json],
        });
        
        // min aggregation
        self.register_function("min".to_string(), CustomFunction {
            sql_template: "min({column})".to_string(),
            return_type: TypeTag::Json, // Generic type
            arg_types: vec![TypeTag::Json],
        });
    }
}

impl Default for GraphQLParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_query_parsing() {
        let parser = GraphQLParser::new();
        let query = r#"
            query {
                users(limit: 10) {
                    id
                    name
                    email
                }
            }
        "#;
        
        let result = parser.parse(query);
        assert!(result.is_ok(), "Should parse simple query successfully");
    }
    
    #[test]
    fn test_multi_root_query_parsing() {
        let parser = GraphQLParser::new();
        let query = r#"
            query {
                users(limit: 5) {
                    id
                    name
                }
                posts(limit: 10) {
                    id
                    title
                    content
                }
            }
        "#;
        
        let result = parser.parse(query);
        assert!(result.is_ok(), "Should parse multi-root query successfully");
    }
    
    #[test]
    fn test_nested_selection_parsing() {
        let parser = GraphQLParser::new();
        let query = r#"
            query {
                users {
                    id
                    name
                    posts {
                        id
                        title
                        categories {
                            id
                            name
                        }
                    }
                }
            }
        "#;
        
        let result = parser.parse(query);
        assert!(result.is_ok(), "Should parse nested selections successfully");
    }
    
    #[test]
    fn test_custom_function_parsing() {
        let parser = GraphQLParser::new();
        let query = r#"
            query {
                posts {
                    id
                    title
                    custom_score: score_rank()
                    total_comments: count()
                }
            }
        "#;
        
        let result = parser.parse(query);
        assert!(result.is_ok(), "Should parse custom functions successfully");
    }
}
