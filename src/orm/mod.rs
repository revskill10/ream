/// Catena - A Categorical, Type-Safe, Macro-Driven ORM for Rust
/// 
/// This module implements a mathematically-grounded ORM based on category theory:
/// - Objects are database schemas
/// - Morphisms are type-safe, side-effectful queries  
/// - Composition is monadic query chaining
/// - Identity is the no-op migration
///
/// Architecture:
/// - Schema: Initial algebra of tables (μ F)
/// - Query<A>: Free monad over SQL algebra
/// - Driver: Coalgebraic interface for database operations
/// - Migration: Catamorphism over schema transformations
/// - Plugin: Natural transformations Driver → Driver

pub mod algebra;
pub mod coalgebra;
pub mod driver;
pub mod migration;
pub mod plugin;
pub mod query;
pub mod schema;
pub mod sql_macro;
pub mod types;
pub mod advanced_sql;
pub mod sqlite_advanced;
pub mod postgres_advanced;
pub mod sqlserver_advanced;
pub mod advanced_query_builder;
pub mod feature_compatibility;
pub mod sql_composable;
pub mod examples;
pub mod graphql;
pub mod graphql_parser;
pub mod graphql_compiler;
pub mod graphql_macro;
pub mod graphql_tlisp;
pub mod graphql_composable;
pub mod mutation;
pub mod nested_relations;
pub mod mutation_compiler;

#[cfg(test)]
pub mod tests;

// Re-export core types and traits
pub use algebra::*;
pub use coalgebra::*;
pub use driver::*;
pub use migration::*;
pub use plugin::*;
pub use query::*;
pub use schema::*;
pub use types::*;
pub use advanced_sql::*;
pub use advanced_query_builder::*;
pub use feature_compatibility::*;
pub use sql_composable::*;
pub use graphql::*;
pub use graphql_parser::*;
pub use graphql_compiler::*;
pub use graphql_macro::*;
pub use graphql_tlisp::*;
pub use graphql_composable::*;
pub use mutation::*;
pub use nested_relations::*;
pub use mutation_compiler::*;

// Re-export SQL parsing from sqlite module
pub use crate::sqlite::parser::{ast, parse_sql};
pub use crate::sqlite::types::{DataType, Value};
pub use crate::sqlite::error::{SqlError, SqlResult};

/// Main ORM context for managing database operations
pub struct OrmContext<D: Driver> {
    driver: D,
    schema: Schema,
}

impl<D: Driver> OrmContext<D> {
    /// Create a new ORM context with the given driver
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            schema: Schema::empty(),
        }
    }

    /// Get a reference to the driver
    pub fn driver(&self) -> &D {
        &self.driver
    }

    /// Get a mutable reference to the driver
    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    /// Get the current schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Update the schema
    pub fn set_schema(&mut self, schema: Schema) {
        self.schema = schema;
    }

    /// Execute a query using the driver
    pub async fn execute<A>(&self, query: Query<A>) -> SqlResult<A>
    where
        A: Send + 'static,
        D::Row: crate::orm::types::Row,
    {
        query.execute(&self.driver).await
    }

    /// Run migrations to update the schema
    pub async fn migrate(&mut self, migration: impl Migration) -> SqlResult<()> {
        let new_schema = migration.apply(&self.schema);
        self.driver.migrate(&new_schema).await?;
        self.schema = new_schema;
        Ok(())
    }
}

/// Convenience type alias for ORM context with SQLite driver
pub type SqliteOrm = OrmContext<SqliteDriver>;

/// Convenience type alias for ORM context with PostgreSQL driver  
pub type PostgresOrm = OrmContext<PostgresDriver>;

/// Result type for ORM operations
pub type OrmResult<T> = Result<T, OrmError>;

/// ORM-specific error types
#[derive(Debug, thiserror::Error)]
pub enum OrmError {
    #[error("SQL error: {0}")]
    Sql(#[from] SqlError),
    
    #[error("Schema error: {message}")]
    Schema { message: String },
    
    #[error("Migration error: {message}")]
    Migration { message: String },
    
    #[error("Type error: {message}")]
    Type { message: String },
    
    #[error("Driver error: {message}")]
    Driver { message: String },
}

impl OrmError {
    pub fn schema_error(message: impl Into<String>) -> Self {
        Self::Schema { message: message.into() }
    }
    
    pub fn migration_error(message: impl Into<String>) -> Self {
        Self::Migration { message: message.into() }
    }
    
    pub fn type_error(message: impl Into<String>) -> Self {
        Self::Type { message: message.into() }
    }
    
    pub fn driver_error(message: impl Into<String>) -> Self {
        Self::Driver { message: message.into() }
    }
}


