use crate::error::{SqlError, SqlResult};
use crate::schema::Schema;
use serde::{Deserialize, Serialize};

/// Index definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub metadata: IndexMetadata,
}

/// Index metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub created_at: std::time::SystemTime,
    pub size_estimate: u64,
    pub selectivity: f64,
}

impl Index {
    pub fn new(name: String, table_name: String, columns: Vec<String>, unique: bool) -> Self {
        Index {
            name,
            table_name,
            columns,
            unique,
            metadata: IndexMetadata {
                created_at: std::time::SystemTime::now(),
                size_estimate: 0,
                selectivity: 1.0,
            },
        }
    }

    pub fn validate(&self, schema: &Schema) -> SqlResult<()> {
        if self.name.is_empty() {
            return Err(SqlError::schema_error("Index name cannot be empty"));
        }

        if self.columns.is_empty() {
            return Err(SqlError::schema_error("Index must have at least one column"));
        }

        // Verify table exists
        if !schema.has_table(&self.table_name) {
            return Err(SqlError::table_not_found(&self.table_name));
        }

        Ok(())
    }

    pub fn estimated_size(&self) -> usize {
        self.name.len() + self.table_name.len() + self.columns.iter().map(|c| c.len()).sum::<usize>()
    }
}
