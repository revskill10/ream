//! Categorical SQLite - A mathematical SQLite implementation
//! 
//! This crate implements a SQLite-compatible database engine using category theory
//! and algebraic patterns for maximum correctness and compositionality.

pub mod types;
pub mod error;
pub mod parser;
pub mod btree;
pub mod page_cache;
pub mod query;
pub mod transaction;
pub mod schema;
pub mod engine;

pub use engine::CategoricalSQLite;
pub use error::{SqlError, SqlResult};
pub use types::{Value, Row, DataType};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_engine_creation() {
        let config = engine::DatabaseConfig::default();
        let db = CategoricalSQLite::new(config);
        
        // Test basic state query
        let state = db.get_database_state().await;
        assert!(state.schemas.is_empty());
        assert!(state.tables.is_empty());
    }
}
