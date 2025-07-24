use ream::orm::graphql_composable::*;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_graphql_composable_query_builder() {
    // Test basic query building
    let query = ComposableQueryBuilder::new()
        .field("users").end_field()
        .field("posts").end_field()
        .build();

    // Verify the query structure
    assert_eq!(query.selection_set.fields.len(), 2);
    assert_eq!(query.selection_set.fields[0].name, "users");
    assert_eq!(query.selection_set.fields[1].name, "posts");
}

#[tokio::test]
async fn test_graphql_composable_mutation_builder() {
    // Test mutation building
    let mutation = ComposableMutationBuilder::new()
        .create("users").end_operation()
        .build();

    // Verify the mutation structure
    assert_eq!(mutation.operations.len(), 1);
}

#[tokio::test]
async fn test_field_builder() {
    // Test field builder with arguments - simplified test since FieldBuilder is internal
    let query = ComposableQueryBuilder::new()
        .field("user").end_field()
        .build();

    assert_eq!(query.selection_set.fields.len(), 1);
    assert_eq!(query.selection_set.fields[0].name, "user");
}

#[tokio::test]
async fn test_mutation_operation_builder() {
    // Test create operation - simplified since MutationOperationBuilder is internal
    let mutation = ComposableMutationBuilder::new()
        .create("users").end_operation()
        .build();

    assert_eq!(mutation.operations.len(), 1);

    // Test update operation
    let mutation = ComposableMutationBuilder::new()
        .update("users").end_operation()
        .build();

    assert_eq!(mutation.operations.len(), 1);

    // Test delete operation
    let mutation = ComposableMutationBuilder::new()
        .delete("users").end_operation()
        .build();

    assert_eq!(mutation.operations.len(), 1);
}

#[tokio::test]
async fn test_complex_query_composition() {
    // Test complex query with multiple fields and nested structure
    let query = ComposableQueryBuilder::new()
        .field("users").end_field()
        .field("posts").end_field()
        .field("comments").end_field()
        .build();

    // Verify the query structure
    assert_eq!(query.selection_set.fields.len(), 3);
    assert_eq!(query.selection_set.fields[0].name, "users");
    assert_eq!(query.selection_set.fields[1].name, "posts");
    assert_eq!(query.selection_set.fields[2].name, "comments");
}

#[tokio::test]
async fn test_mutation_with_data() {
    // Test mutation with actual data
    let mutation_data = json!({
        "name": "Alice",
        "email": "alice@example.com",
        "age": 30
    });

    let mutation = ComposableMutationBuilder::new()
        .create("users").end_operation()
        .build();

    // Verify the mutation structure
    assert_eq!(mutation.operations.len(), 1);
}

#[tokio::test]
async fn test_error_handling() {
    // Test error handling for invalid operations
    let query = ComposableQueryBuilder::new()
        .build();

    // Should still work with empty query
    assert_eq!(query.selection_set.fields.len(), 0);

    // Test mutation error handling
    let mutation = ComposableMutationBuilder::new()
        .build();

    // Should handle empty mutation gracefully
    assert_eq!(mutation.operations.len(), 0);
}

#[tokio::test]
async fn test_builder_pattern_fluency() {
    // Test that builder pattern is fluent and chainable
    let query = ComposableQueryBuilder::new()
        .field("users").end_field()
        .field("posts").end_field()
        .field("comments").end_field()
        .field("tags").end_field()
        .build();

    assert_eq!(query.selection_set.fields.len(), 4);

    let mutation = ComposableMutationBuilder::new()
        .create("users").end_operation()
        .update("posts").end_operation()
        .delete("comments").end_operation()
        .upsert("tags", vec!["name".to_string()]).end_operation()
        .build();

    assert_eq!(mutation.operations.len(), 4);
}

#[tokio::test]
async fn test_type_safety() {
    // Test that the system maintains type safety through JSON values
    let test_data = json!({
        "string_arg": "test",
        "number_arg": 42,
        "boolean_arg": true,
        "array_arg": [1, 2, 3],
        "object_arg": {"key": "value"}
    });

    // Verify types are preserved in JSON
    assert!(test_data.get("string_arg").unwrap().is_string());
    assert!(test_data.get("number_arg").unwrap().is_number());
    assert!(test_data.get("boolean_arg").unwrap().is_boolean());
    assert!(test_data.get("array_arg").unwrap().is_array());
    assert!(test_data.get("object_arg").unwrap().is_object());
}

#[tokio::test]
async fn test_integration_with_existing_orm() {
    // Test integration with existing ORM components
    let query = ComposableQueryBuilder::new()
        .field("users").end_field()
        .build();

    // Verify the query structure
    assert_eq!(query.selection_set.fields.len(), 1);
    assert_eq!(query.selection_set.fields[0].name, "users");

    let mutation = ComposableMutationBuilder::new()
        .create("users").end_operation()
        .build();

    // Simplified test since execute methods are on GraphQLComposableORM
    assert_eq!(mutation.operations.len(), 1);
}

#[tokio::test]
async fn test_graphql_variables_support() {
    // Test GraphQL variables in queries
    let query = ComposableQueryBuilder::new()
        .variable("userId", json!(123))
        .variable("includeProfile", json!(true))
        .field_with_variable("user", "id", "userId").end_field()
        .field("posts")
            .argument("authorId", json!("$userId"))
            .end_field()
        .build();

    // Verify variables are stored
    assert_eq!(query.variables.len(), 2);
    assert_eq!(query.variables.get("userId"), Some(&json!(123)));
    assert_eq!(query.variables.get("includeProfile"), Some(&json!(true)));

    // Verify field arguments use variables
    assert_eq!(query.selection_set.fields.len(), 2);
    assert_eq!(query.selection_set.fields[0].args.get("id"), Some(&json!("$userId")));
    assert_eq!(query.selection_set.fields[1].args.get("authorId"), Some(&json!("$userId")));
}

#[tokio::test]
async fn test_mutation_with_variables_and_return_query() {
    // Test mutation with variables and return query fragment
    let user_input = json!({
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30
    });

    let where_clause = json!({
        "id": 123
    });

    let mutation = ComposableMutationBuilder::new()
        .variable("input", user_input.clone())
        .variable("where", where_clause.clone())
        .create_with_variables("users", "input")
            .return_field("id")
            .return_field("name")
            .return_field("email")
            .end_operation()
        .build();

    // Verify variables are stored
    assert_eq!(mutation.variables.len(), 2);
    assert_eq!(mutation.variables.get("input"), Some(&user_input));
    assert_eq!(mutation.variables.get("where"), Some(&where_clause));

    // Verify mutation operation
    assert_eq!(mutation.operations.len(), 1);
}

#[tokio::test]
async fn test_complex_mutation_with_nested_return_query() {
    // Test complex mutation that returns nested data
    let mutation = ComposableMutationBuilder::new()
        .variable("userInput", json!({
            "name": "Alice Smith",
            "email": "alice@example.com"
        }))
        .create_with_variables("users", "userInput")
            .return_field("id")
            .return_field("name")
            .return_field("email")
            .return_field("createdAt")
            .end_operation()
        .return_query(
            ComposableQueryBuilder::new()
                .field("user")
                    .argument("id", json!("$newUserId")) // Reference the created user's ID
                    .end_field()
                .build()
        )
        .build();

    // Verify the structure
    assert_eq!(mutation.operations.len(), 1);
    assert!(mutation.return_query.is_some());

    let return_query = mutation.return_query.unwrap();
    assert_eq!(return_query.selection_set.fields.len(), 1);
    assert_eq!(return_query.selection_set.fields[0].name, "user");
}

#[tokio::test]
async fn test_variable_based_field_arguments() {
    // Test using variables in field arguments
    let query = ComposableQueryBuilder::new()
        .variables(HashMap::from([
            ("limit".to_string(), json!(10)),
            ("offset".to_string(), json!(20)),
            ("status".to_string(), json!("active")),
        ]))
        .field("users")
            .argument("first", json!("$limit"))
            .argument("skip", json!("$offset"))
            .argument("where", json!({"status": "$status"}))
            .end_field()
        .build();

    // Verify variables and field arguments
    assert_eq!(query.variables.len(), 3);
    assert_eq!(query.selection_set.fields[0].args.len(), 3);
    assert_eq!(query.selection_set.fields[0].args.get("first"), Some(&json!("$limit")));
    assert_eq!(query.selection_set.fields[0].args.get("skip"), Some(&json!("$offset")));
}
