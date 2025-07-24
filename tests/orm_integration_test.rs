/// Integration tests for the Ream ORM system
/// 
/// These tests verify that the ORM components work together correctly
/// and integrate properly with the actor system and TLisp.

use ream::orm::{
    OrmContext, SqliteDriver, Schema, Column, Query, QueryBuilder,
    Migration, AddTableMigration, MigrationMetadata, MigrationRunner,
    Plugin, LoggingPlugin, CachingPlugin, PluginTransformer,
    DatabaseRow, User, FromRow, ToRow,
};
use ream::sqlite::types::{DataType, Value};
use ream::tlisp::{TlispInterpreter, Value as TlispValue};
use std::time::Duration;

#[tokio::test]
async fn test_basic_orm_operations() {
    // Create SQLite driver
    let driver = SqliteDriver::new(":memory:");
    let orm = OrmContext::new(driver);
    
    // Verify ORM context creation
    assert!(orm.schema().is_empty());
    
    // Test driver metadata
    let metadata = orm.driver().metadata();
    assert_eq!(metadata.name, "SQLite");
    assert!(metadata.supports_transactions);
}

#[tokio::test]
async fn test_schema_algebra() {
    // Test schema composition using algebraic operations
    let schema = Schema::empty()
        .add_table("users", vec![
            Column::new("id", DataType::Integer).primary_key().auto_increment(),
            Column::new("name", DataType::Text).not_null(),
            Column::new("email", DataType::Text),
        ])
        .add_table("posts", vec![
            Column::new("id", DataType::Integer).primary_key().auto_increment(),
            Column::new("title", DataType::Text).not_null(),
            Column::new("user_id", DataType::Integer).not_null(),
        ])
        .add_index("idx_posts_user_id", "posts", vec!["user_id".to_string()], false);
    
    // Verify schema structure
    assert!(!schema.is_empty());
    
    let tables = schema.tables();
    assert_eq!(tables.len(), 2);
    
    let users_table = schema.find_table("users").unwrap();
    assert_eq!(users_table.name, "users");
    assert_eq!(users_table.columns.len(), 3);
    
    let posts_table = schema.find_table("posts").unwrap();
    assert_eq!(posts_table.name, "posts");
    assert_eq!(posts_table.columns.len(), 3);
    
    let indexes = schema.indexes();
    assert_eq!(indexes.len(), 1);
    assert_eq!(indexes[0].name, "idx_posts_user_id");
}

#[tokio::test]
async fn test_query_free_monad() {
    // Test query composition using monadic operations
    let query1 = Query::pure(42);
    let query2 = query1.map(|x| x * 2);
    let query3 = query2.bind(|x| Query::pure(x + 1));
    
    // Verify monadic laws (simplified test)
    match query3 {
        Query::Pure(value) => assert_eq!(value, 85), // 42 * 2 + 1
        _ => panic!("Expected pure query"),
    }
}

#[tokio::test]
async fn test_query_builders() {
    // Test SELECT query builder
    let select_query = QueryBuilder::select()
        .column("id")
        .column("name")
        .from("users")
        .limit(10)
        .build();
    
    // Verify query structure
    match select_query {
        Query::Free(_) => {}, // Expected for complex queries
        Query::Pure(_) => panic!("Expected free query"),
    }
    
    // Test INSERT query builder
    let insert_query = QueryBuilder::insert()
        .into("users")
        .columns(vec!["name".to_string(), "email".to_string()])
        .values(vec![
            Value::Text("Alice".to_string()),
            Value::Text("alice@example.com".to_string()),
        ])
        .build();
    
    match insert_query {
        Query::Free(_) => {}, // Expected
        Query::Pure(_) => panic!("Expected free query"),
    }
}

#[tokio::test]
async fn test_migration_system() {
    let schema = Schema::empty();
    
    // Create migration
    let metadata = MigrationMetadata::new(
        1,
        "add_users_table",
        "Add users table",
        "test",
    );
    
    let migration = AddTableMigration::new(
        metadata.clone(),
        "users",
        vec![
            Column::new("id", DataType::Integer).primary_key(),
            Column::new("name", DataType::Text).not_null(),
        ],
    );
    
    // Test migration metadata
    assert_eq!(migration.metadata().version, 1);
    assert_eq!(migration.metadata().name, "add_users_table");
    assert!(migration.metadata().reversible);
    
    // Test migration application
    assert!(migration.can_apply(&schema));
    let new_schema = migration.apply(&schema);
    
    // Verify schema was modified
    let tables = new_schema.tables();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name, "users");
    
    // Test migration runner
    let mut runner = MigrationRunner::new();
    let result = runner.apply_migration(Box::new(migration), schema);
    assert!(result.is_ok());
    assert!(runner.is_applied(1));
}

#[tokio::test]
async fn test_plugin_system() {
    let driver = SqliteDriver::new(":memory:");
    
    // Test logging plugin
    let logging_plugin = LoggingPlugin::new();
    let metadata = logging_plugin.metadata();
    assert_eq!(metadata.name, "logging");
    
    let enhanced_driver = logging_plugin.transform(driver);
    let driver_metadata = enhanced_driver.metadata();
    assert!(driver_metadata.name.contains("logging"));
    
    // Test caching plugin
    let caching_plugin = CachingPlugin::new(Duration::from_secs(60));
    let cached_driver = caching_plugin.transform(enhanced_driver);
    let cached_metadata = cached_driver.metadata();
    assert!(cached_metadata.name.contains("caching"));
    
    // Test metrics plugin
    let metrics_plugin = MetricsPlugin::new();
    let final_driver = metrics_plugin.transform(cached_driver);
    let final_metadata = final_driver.metadata();
    assert!(final_metadata.name.contains("metrics"));
}

#[tokio::test]
async fn test_type_system() {
    // Test basic type conversions
    let int_value = Value::Integer(42);
    let converted: i64 = i64::from_value(&int_value).unwrap();
    assert_eq!(converted, 42);
    
    let text_value = Value::Text("hello".to_string());
    let converted: String = String::from_value(&text_value).unwrap();
    assert_eq!(converted, "hello");
    
    let bool_value = Value::Boolean(true);
    let converted: bool = bool::from_value(&bool_value).unwrap();
    assert_eq!(converted, true);
    
    // Test optional types
    let null_value = Value::Null;
    let converted: Option<String> = Option::<String>::from_value(&null_value).unwrap();
    assert_eq!(converted, None);
    
    // Test User struct conversion
    let row = DatabaseRow::new(
        vec![
            "id".to_string(),
            "name".to_string(),
            "email".to_string(),
            "active".to_string(),
        ],
        vec![
            Value::Integer(1),
            Value::Text("Alice".to_string()),
            Value::Text("alice@example.com".to_string()),
            Value::Boolean(true),
        ],
    );
    
    let user = User::from_row(row).unwrap();
    assert_eq!(user.id, 1);
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, Some("alice@example.com".to_string()));
    assert_eq!(user.active, true);
    
    // Test reverse conversion
    let row_data = user.to_row().unwrap();
    assert_eq!(row_data.get("id"), Some(&Value::Integer(1)));
    assert_eq!(row_data.get("name"), Some(&Value::Text("Alice".to_string())));
}

#[tokio::test]
async fn test_tlisp_integration() {
    let mut interpreter = TlispInterpreter::new();
    
    // Register ORM functions
    interpreter.register_function("db-query", |args| {
        if args.len() != 1 {
            return Err("db-query expects 1 argument".to_string());
        }
        
        match &args[0] {
            TlispValue::String(sql) => {
                // Simulate query execution
                Ok(TlispValue::List(vec![
                    TlispValue::String("result".to_string()),
                    TlispValue::Number(42.0),
                ]))
            }
            _ => Err("db-query expects a string argument".to_string()),
        }
    });
    
    interpreter.register_function("create-user", |args| {
        if args.len() != 2 {
            return Err("create-user expects 2 arguments".to_string());
        }
        
        match (&args[0], &args[1]) {
            (TlispValue::String(_), TlispValue::String(_)) => {
                Ok(TlispValue::Number(1.0)) // Return user ID
            }
            _ => Err("create-user expects string arguments".to_string()),
        }
    });
    
    // Test TLisp code execution
    let code = r#"
        (create-user "Alice" "alice@example.com")
    "#;
    
    let result = interpreter.eval(code);
    assert!(result.is_ok());
    
    match result.unwrap() {
        TlispValue::Number(id) => assert_eq!(id, 1.0),
        _ => panic!("Expected number result"),
    }
    
    // Test database query from TLisp
    let query_code = r#"
        (db-query "SELECT * FROM users")
    "#;
    
    let result = interpreter.eval(query_code);
    assert!(result.is_ok());
    
    match result.unwrap() {
        TlispValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], TlispValue::String("result".to_string()));
            assert_eq!(items[1], TlispValue::Number(42.0));
        }
        _ => panic!("Expected list result"),
    }
}

#[tokio::test]
async fn test_driver_health_check() {
    let driver = SqliteDriver::new(":memory:");
    let health = driver.health_check().await;
    assert!(health.is_ok());
    assert!(health.unwrap());
}

#[tokio::test]
async fn test_orm_context_integration() {
    let driver = SqliteDriver::new(":memory:");
    let mut orm = OrmContext::new(driver);
    
    // Test schema setting
    let schema = Schema::empty()
        .add_table("test", vec![
            Column::new("id", DataType::Integer).primary_key(),
        ]);
    
    orm.set_schema(schema.clone());
    assert_eq!(orm.schema().tables().len(), 1);
    
    // Test migration through ORM context
    let migration_metadata = MigrationMetadata::new(
        1,
        "test_migration",
        "Test migration",
        "test",
    );
    
    let migration = AddTableMigration::new(
        migration_metadata,
        "another_table",
        vec![Column::new("id", DataType::Integer)],
    );
    
    let result = orm.migrate(migration).await;
    assert!(result.is_ok());
}
