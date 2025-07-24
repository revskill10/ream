/// Tests for GraphQL Composable Query and Mutation System
/// 
/// This module tests the composable GraphQL system that allows:
/// - Queries and mutations to be composed together
/// - Mutations that return queries
/// - Complex nested operations
/// - GraphQL spec compliance

use crate::orm::graphql_composable::*;
use crate::orm::schema::TypeSafeSchema;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schema() -> TypeSafeSchema {
        // Create a test schema for our composable tests
        TypeSafeSchema::new()
    }

    #[test]
    fn test_composable_query_builder() {
        // Test building a complex query with the builder pattern
        let query = ComposableQueryBuilder::new()
            .variable("userId", json!(123))
            .field("user")
                .argument("id", json!("$userId"))
                .selection(crate::orm::graphql::SelectionSet {
                    fields: vec![
                        crate::orm::graphql::Field {
                            name: "id".to_string(),
                            alias: None,
                            arguments: HashMap::new(),
                            directives: Vec::new(),
                            selection_set: None,
                        },
                        crate::orm::graphql::Field {
                            name: "name".to_string(),
                            alias: None,
                            arguments: HashMap::new(),
                            directives: Vec::new(),
                            selection_set: None,
                        },
                    ],
                })
                .end_field()
            .build();

        assert_eq!(query.variables.get("userId"), Some(&json!(123)));
        assert_eq!(query.selection_set.fields.len(), 1);
        assert_eq!(query.selection_set.fields[0].name, "user");
    }

    #[test]
    fn test_composable_mutation_builder() {
        // Test building a mutation that returns a query
        let return_query = ComposableQueryBuilder::new()
            .field("user")
                .argument("id", json!("$mutation_result_0"))
                .selection(crate::orm::graphql::SelectionSet {
                    fields: vec![
                        crate::orm::graphql::Field {
                            name: "id".to_string(),
                            alias: None,
                            arguments: HashMap::new(),
                            directives: Vec::new(),
                            selection_set: None,
                        },
                        crate::orm::graphql::Field {
                            name: "name".to_string(),
                            alias: None,
                            arguments: HashMap::new(),
                            directives: Vec::new(),
                            selection_set: None,
                        },
                    ],
                })
                .end_field()
            .build();

        let mutation = ComposableMutationBuilder::new()
            .create("users")
                .input("name", json!("John Doe"))
                .input("email", json!("john@example.com"))
                .return_field("id")
                .return_field("name")
                .end_operation()
            .return_query(return_query)
            .build();

        assert_eq!(mutation.operations.len(), 1);
        assert!(mutation.return_query.is_some());
        
        match &mutation.operations[0].operation_type {
            MutationOperationType::Create { table } => {
                assert_eq!(table, "users");
            },
            _ => panic!("Expected Create operation"),
        }
    }

    #[test]
    fn test_mutation_with_multiple_operations() {
        // Test a mutation with multiple operations
        let mutation = ComposableMutationBuilder::new()
            .create("users")
                .input("name", json!("Alice"))
                .input("email", json!("alice@example.com"))
                .end_operation()
            .create("posts")
                .input("title", json!("My First Post"))
                .input("content", json!("Hello World!"))
                .input("user_id", json!("$mutation_result_0"))
                .end_operation()
            .update("users")
                .where_clause("id", json!("$mutation_result_0"))
                .input("post_count", json!(1))
                .end_operation()
            .build();

        assert_eq!(mutation.operations.len(), 3);
        
        // Verify operation types
        match &mutation.operations[0].operation_type {
            MutationOperationType::Create { table } => assert_eq!(table, "users"),
            _ => panic!("Expected Create operation"),
        }
        
        match &mutation.operations[1].operation_type {
            MutationOperationType::Create { table } => assert_eq!(table, "posts"),
            _ => panic!("Expected Create operation"),
        }
        
        match &mutation.operations[2].operation_type {
            MutationOperationType::Update { table } => assert_eq!(table, "users"),
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_upsert_operation() {
        // Test upsert operation with conflict resolution
        let mutation = ComposableMutationBuilder::new()
            .upsert("users", vec!["email".to_string()])
                .input("name", json!("Bob"))
                .input("email", json!("bob@example.com"))
                .input("age", json!(30))
                .end_operation()
            .build();

        assert_eq!(mutation.operations.len(), 1);
        
        match &mutation.operations[0].operation_type {
            MutationOperationType::Upsert { table, conflict_fields } => {
                assert_eq!(table, "users");
                assert_eq!(conflict_fields, &vec!["email".to_string()]);
            },
            _ => panic!("Expected Upsert operation"),
        }
    }

    #[test]
    fn test_connect_disconnect_operations() {
        // Test relationship operations
        let mutation = ComposableMutationBuilder::new()
            .create("posts")
                .input("title", json!("New Post"))
                .input("content", json!("Content here"))
                .end_operation()
            // Connect the new post to existing tags
            .build();

        // For now, just test that we can build the mutation
        assert_eq!(mutation.operations.len(), 1);
    }

    #[test]
    fn test_graphql_directives() {
        // Test GraphQL directives like @include and @skip
        let include_directive = Directive {
            name: "include".to_string(),
            arguments: {
                let mut args = HashMap::new();
                args.insert("if".to_string(), json!(true));
                args
            },
        };

        let skip_directive = Directive {
            name: "skip".to_string(),
            arguments: {
                let mut args = HashMap::new();
                args.insert("if".to_string(), json!(false));
                args
            },
        };

        let query = ComposableQueryBuilder::new()
            .variable("includeEmail", json!(true))
            .variable("skipPhone", json!(false))
            .field("user")
                .argument("id", json!(123))
                .directive(include_directive)
                .directive(skip_directive)
                .end_field()
            .build();

        assert_eq!(query.selection_set.fields[0].directives.len(), 2);
        assert_eq!(query.selection_set.fields[0].directives[0].name, "include");
        assert_eq!(query.selection_set.fields[0].directives[1].name, "skip");
    }

    #[test]
    fn test_fragments() {
        // Test GraphQL fragments for reusable selection sets
        let user_fragment = Fragment {
            name: "UserInfo".to_string(),
            type_condition: "User".to_string(),
            selection_set: crate::orm::graphql::SelectionSet {
                fields: vec![
                    crate::orm::graphql::Field {
                        name: "id".to_string(),
                        alias: None,
                        arguments: HashMap::new(),
                        directives: Vec::new(),
                        selection_set: None,
                    },
                    crate::orm::graphql::Field {
                        name: "name".to_string(),
                        alias: None,
                        arguments: HashMap::new(),
                        directives: Vec::new(),
                        selection_set: None,
                    },
                    crate::orm::graphql::Field {
                        name: "email".to_string(),
                        alias: None,
                        arguments: HashMap::new(),
                        directives: Vec::new(),
                        selection_set: None,
                    },
                ],
            },
            directives: Vec::new(),
        };

        let query = ComposableQueryBuilder::new()
            .fragment(user_fragment)
            .field("user")
                .argument("id", json!(123))
                .end_field()
            .build();

        assert_eq!(query.fragments.len(), 1);
        assert!(query.fragments.contains_key("UserInfo"));
    }

    #[test]
    fn test_complex_nested_mutation_query() {
        // Test a complex scenario: create user, create posts, then query the result
        let return_query = ComposableQueryBuilder::new()
            .field("user")
                .argument("id", json!("$mutation_result_0"))
                .selection(crate::orm::graphql::SelectionSet {
                    fields: vec![
                        crate::orm::graphql::Field {
                            name: "id".to_string(),
                            alias: None,
                            arguments: HashMap::new(),
                            directives: Vec::new(),
                            selection_set: None,
                        },
                        crate::orm::graphql::Field {
                            name: "name".to_string(),
                            alias: None,
                            arguments: HashMap::new(),
                            directives: Vec::new(),
                            selection_set: None,
                        },
                        crate::orm::graphql::Field {
                            name: "posts".to_string(),
                            alias: None,
                            arguments: HashMap::new(),
                            directives: Vec::new(),
                            selection_set: Some(crate::orm::graphql::SelectionSet {
                                fields: vec![
                                    crate::orm::graphql::Field {
                                        name: "id".to_string(),
                                        alias: None,
                                        arguments: HashMap::new(),
                                        directives: Vec::new(),
                                        selection_set: None,
                                    },
                                    crate::orm::graphql::Field {
                                        name: "title".to_string(),
                                        alias: None,
                                        arguments: HashMap::new(),
                                        directives: Vec::new(),
                                        selection_set: None,
                                    },
                                ],
                            }),
                        },
                    ],
                })
                .end_field()
            .build();

        let mutation = ComposableMutationBuilder::new()
            .create("users")
                .input("name", json!("Jane Doe"))
                .input("email", json!("jane@example.com"))
                .return_field("id")
                .end_operation()
            .create("posts")
                .input("title", json!("First Post"))
                .input("content", json!("Hello from Jane!"))
                .input("user_id", json!("$mutation_result_0"))
                .return_field("id")
                .end_operation()
            .create("posts")
                .input("title", json!("Second Post"))
                .input("content", json!("Another post from Jane!"))
                .input("user_id", json!("$mutation_result_0"))
                .return_field("id")
                .end_operation()
            .return_query(return_query)
            .build();

        assert_eq!(mutation.operations.len(), 3);
        assert!(mutation.return_query.is_some());
        
        // Verify that the return query references the mutation result
        let return_query = mutation.return_query.unwrap();
        assert_eq!(return_query.selection_set.fields[0].arguments.get("id"), Some(&json!("$mutation_result_0")));
    }

    #[test]
    fn test_operation_composition() {
        // Test that operations can be composed together
        let operation = GraphQLOperation::Mutation(
            ComposableMutationBuilder::new()
                .create("users")
                    .input("name", json!("Test User"))
                    .end_operation()
                .build()
        );

        match operation {
            GraphQLOperation::Mutation(mutation) => {
                assert_eq!(mutation.operations.len(), 1);
            },
            _ => panic!("Expected mutation operation"),
        }
    }
}
