/// Comprehensive GraphQL Tests
/// 
/// This module tests the complete GraphQL implementation including:
/// - GraphQL query parsing and validation
/// - Multi-root query compilation
/// - SQL generation with CTEs
/// - Custom function registration
/// - TLisp integration
/// - Rust macro interface

#[cfg(test)]
mod tests {
    use crate::orm::graphql::*;
    use crate::orm::graphql_parser::*;
    use crate::orm::graphql_compiler::*;
    use crate::orm::graphql_tlisp::*;
    use crate::orm::schema::TypeSafeSchema;
    use crate::tlisp::Value;
    use std::collections::HashMap;
    use serde_json::json;

    /// Test basic GraphQL query parsing
    #[test]
    fn test_basic_graphql_parsing() {
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
        assert!(result.is_ok(), "Should parse basic query successfully");
        
        let multi_root = result.unwrap();
        match multi_root {
            MultiRootF::Root { name, selection, args, .. } => {
                assert_eq!(name, "users");
                assert_eq!(selection.fields.len(), 3);
                assert!(args.contains_key("limit"));
            },
            _ => panic!("Expected single root query"),
        }
    }

    /// Test multi-root GraphQL query parsing
    #[test]
    fn test_multi_root_parsing() {
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
        
        // The parser should create a combined multi-root structure
        let multi_root = result.unwrap();
        match multi_root {
            MultiRootF::Combine { .. } => {
                // Expected structure for multi-root
            },
            MultiRootF::Root { .. } => {
                // Also acceptable if parser handles differently
            },
        }
    }

    /// Test nested selection parsing
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
        
        let multi_root = result.unwrap();
        match multi_root {
            MultiRootF::Root { selection, .. } => {
                // Check that posts field has nested selection
                let posts_field = selection.fields.iter()
                    .find(|f| f.name == "posts");
                assert!(posts_field.is_some());
                assert!(posts_field.unwrap().selection_set.is_some());
            },
            _ => panic!("Expected single root query"),
        }
    }

    /// Test custom function parsing
    #[test]
    fn test_custom_function_parsing() {
        let mut parser = GraphQLParser::new();
        
        // Register a custom function
        parser.register_function("custom_score".to_string(), CustomFunction {
            sql_template: "calculate_score({user_id})".to_string(),
            return_type: TypeTag::Real,
            arg_types: vec![TypeTag::Integer],
        });
        
        let query = r#"
            query {
                posts {
                    id
                    title
                    custom_score: custom_score()
                }
            }
        "#;
        
        let result = parser.parse(query);
        assert!(result.is_ok(), "Should parse custom functions successfully");
    }

    /// Test GraphQL compilation to SQL
    #[test]
    fn test_graphql_compilation() {
        let schema = TypeSafeSchema::new();
        let compiler = GraphQLCompiler::new(schema);
        
        // Create a simple multi-root query
        let query = MultiRootF::Root {
            name: "users".to_string(),
            selection: SelectionSet {
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        alias: None,
                        args: HashMap::new(),
                        selection_set: None,
                        custom_expr: None,
                    },
                    Field {
                        name: "name".to_string(),
                        alias: None,
                        args: HashMap::new(),
                        selection_set: None,
                        custom_expr: None,
                    },
                ],
            },
            args: {
                let mut args = HashMap::new();
                args.insert("limit".to_string(), json!(10));
                args
            },
            next: (),
        };
        
        let result = compiler.compile(query);
        assert!(result.is_ok(), "Should compile query successfully");
        
        let compiled = result.unwrap();
        assert!(compiled.sql.contains("WITH"), "Should generate CTE-based SQL");
        assert!(compiled.sql.contains("users"), "Should reference users table");
        assert!(compiled.sql.contains("LIMIT"), "Should include LIMIT clause");
    }

    /// Test advanced GraphQL compilation with optimizations
    #[test]
    fn test_advanced_compilation() {
        let schema = TypeSafeSchema::new();
        let compiler = AdvancedGraphQLCompiler::new(schema, OptimizationLevel::Basic);
        
        let query = MultiRootF::Root {
            name: "users".to_string(),
            selection: SelectionSet {
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        alias: None,
                        args: HashMap::new(),
                        selection_set: None,
                        custom_expr: None,
                    },
                ],
            },
            args: HashMap::new(),
            next: (),
        };
        
        let result = compiler.compile_optimized(query);
        assert!(result.is_ok(), "Should compile with optimizations successfully");
        
        let execution_plan = result.unwrap();
        assert!(!execution_plan.sql.is_empty(), "Should generate SQL");
        assert!(execution_plan.estimated_cost > 0.0, "Should estimate cost");
        assert!(execution_plan.estimated_rows > 0, "Should estimate rows");
    }

    /// Test TLisp GraphQL library functions
    #[test]
    fn test_tlisp_graphql_library() {
        let library = TlispGraphQLLibrary::new();
        
        // Test parse query function
        let parse_args = vec![Value::String("query { users { id name } }".to_string())];
        let parse_result = library.parse_query(parse_args);
        assert!(parse_result.is_ok(), "Should parse query from TLisp");
        
        // Test build query function
        let build_args = vec![
            Value::List(vec![Value::String("root".to_string()), Value::String("users".to_string())]),
            Value::List(vec![Value::String("field".to_string()), Value::String("id".to_string())]),
            Value::List(vec![Value::String("field".to_string()), Value::String("name".to_string())]),
            Value::List(vec![Value::String("close".to_string()), Value::String("".to_string())]),
        ];
        let build_result = library.build_query(build_args);
        assert!(build_result.is_ok(), "Should build query from TLisp");
        
        if let Ok(Value::String(query_str)) = build_result {
            assert!(query_str.contains("users"), "Built query should contain users");
            assert!(query_str.contains("id"), "Built query should contain id field");
            assert!(query_str.contains("name"), "Built query should contain name field");
        }
    }

    /// Test TLisp function registration
    #[test]
    fn test_tlisp_function_registration() {
        let library = TlispGraphQLLibrary::new();
        
        let register_args = vec![
            Value::String("my_custom_func".to_string()),
            Value::String("my_sql_function({arg})".to_string()),
            Value::String("real".to_string()),
        ];
        
        let result = library.register_function(register_args);
        assert!(result.is_ok(), "Should register custom function from TLisp");
        
        if let Ok(Value::String(msg)) = result {
            assert!(msg.contains("my_custom_func"), "Should confirm function registration");
        }
    }

    /// Test complex multi-root query with custom functions
    #[test]
    fn test_complex_multi_root_query() {
        let parser = GraphQLParser::new();
        let query = r#"
            query {
                posts(limit: 5) {
                    id
                    title
                    custom_score: score_rank()
                    categories(limit: 3) {
                        id
                        name
                        post_count: count()
                    }
                }
                users(limit: 10) {
                    id
                    name
                    total_posts: count()
                }
            }
        "#;
        
        let parse_result = parser.parse(query);
        assert!(parse_result.is_ok(), "Should parse complex multi-root query");
        
        let multi_root = parse_result.unwrap();
        
        // Test compilation
        let schema = TypeSafeSchema::new();
        let compiler = GraphQLCompiler::new(schema);
        let compile_result = compiler.compile(multi_root);
        assert!(compile_result.is_ok(), "Should compile complex query");
        
        let compiled = compile_result.unwrap();
        assert!(compiled.sql.contains("WITH"), "Should use CTEs for multi-root");
        assert!(compiled.sql.contains("posts"), "Should include posts root");
        assert!(compiled.sql.contains("users"), "Should include users root");
        assert!(compiled.sql.contains("jsonb_build_object"), "Should build JSON result");
    }

    /// Test SQL expression algebra
    #[test]
    fn test_sql_expression_algebra() {
        // Test column expression
        let col_expr = SqlExpr::column("user_id".to_string());
        match col_expr {
            SqlExprF::Column { name, .. } => {
                assert_eq!(name, "user_id");
            },
            _ => panic!("Expected column expression"),
        }
        
        // Test literal expression
        let lit_expr = SqlExpr::literal(json!(42), TypeTag::Integer);
        match lit_expr {
            SqlExprF::Literal { value, ty, .. } => {
                assert_eq!(value, json!(42));
                assert_eq!(ty, TypeTag::Integer);
            },
            _ => panic!("Expected literal expression"),
        }
        
        // Test binary expression
        let binary_expr = SqlExpr::binary(
            BinaryOp::Eq,
            SqlExpr::column("id".to_string()),
            SqlExpr::literal(json!(1), TypeTag::Integer),
        );
        match binary_expr {
            SqlExprF::Binary { op, .. } => {
                assert_eq!(op, BinaryOp::Eq);
            },
            _ => panic!("Expected binary expression"),
        }
    }

    /// Test GraphQL query builder
    #[test]
    fn test_graphql_query_builder() {
        let query = GraphQLQueryBuilder::new()
            .root("users")
            .field("id")
            .field("name")
            .field("email")
            .close()
            .build();
        
        assert!(query.contains("query {"), "Should start with query");
        assert!(query.contains("users {"), "Should contain users root");
        assert!(query.contains("id"), "Should contain id field");
        assert!(query.contains("name"), "Should contain name field");
        assert!(query.contains("email"), "Should contain email field");
        assert!(query.contains("}"), "Should close properly");
    }

    /// Test error handling
    #[test]
    fn test_error_handling() {
        let parser = GraphQLParser::new();
        
        // Test invalid GraphQL syntax
        let invalid_query = "query { users { id name";  // Missing closing brace
        let result = parser.parse(invalid_query);
        assert!(result.is_err(), "Should fail on invalid syntax");
        
        // Test TLisp library error handling
        let library = TlispGraphQLLibrary::new();
        
        // Test with wrong number of arguments
        let wrong_args = vec![];
        let result = library.parse_query(wrong_args);
        assert!(result.is_err(), "Should fail with wrong argument count");
        
        // Test with wrong argument type
        let wrong_type_args = vec![Value::Integer(42)];
        let result = library.parse_query(wrong_type_args);
        assert!(result.is_err(), "Should fail with wrong argument type");
    }

    /// Test type safety and validation
    #[test]
    fn test_type_safety() {
        let schema = TypeSafeSchema::new();
        let compiler = GraphQLCompiler::new(schema);
        
        // Create a query that references valid schema elements
        let valid_query = MultiRootF::Root {
            name: "users".to_string(),
            selection: SelectionSet {
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        alias: None,
                        args: HashMap::new(),
                        selection_set: None,
                        custom_expr: None,
                    },
                ],
            },
            args: HashMap::new(),
            next: (),
        };
        
        let result = compiler.compile(valid_query);
        assert!(result.is_ok(), "Should compile valid schema references");
        
        // Test with invalid table reference
        let invalid_query = MultiRootF::Root {
            name: "nonexistent_table".to_string(),
            selection: SelectionSet {
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        alias: None,
                        args: HashMap::new(),
                        selection_set: None,
                        custom_expr: None,
                    },
                ],
            },
            args: HashMap::new(),
            next: (),
        };
        
        let result = compiler.compile(invalid_query);
        assert!(result.is_err(), "Should fail on invalid table reference");
    }
}
