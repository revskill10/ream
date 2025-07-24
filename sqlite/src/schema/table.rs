use crate::error::{SqlError, SqlResult};
use crate::types::{DataType, Value};
use serde::{Deserialize, Serialize};

/// Table definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub metadata: TableMetadata,
}

/// Column definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<Value>,
    pub auto_increment: bool,
    pub primary_key: bool,
    pub unique: bool,
}

/// Table metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableMetadata {
    pub created_at: std::time::SystemTime,
    pub modified_at: std::time::SystemTime,
    pub row_count: u64,
    pub size_estimate: u64,
}

impl Table {
    pub fn new(name: String, columns: Vec<Column>) -> Self {
        let now = std::time::SystemTime::now();
        Table {
            name,
            columns,
            metadata: TableMetadata {
                created_at: now,
                modified_at: now,
                row_count: 0,
                size_estimate: 0,
            },
        }
    }

    pub fn has_column(&self, name: &str) -> bool {
        self.columns.iter().any(|col| col.name == name)
    }

    pub fn get_column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|col| col.name == name)
    }

    pub fn validate(&self) -> SqlResult<()> {
        if self.name.is_empty() {
            return Err(SqlError::schema_error("Table name cannot be empty"));
        }

        if self.columns.is_empty() {
            return Err(SqlError::schema_error("Table must have at least one column"));
        }

        // Check for duplicate column names
        let mut column_names = std::collections::HashSet::new();
        for column in &self.columns {
            if !column_names.insert(&column.name) {
                return Err(SqlError::schema_error(format!(
                    "Duplicate column name: {}",
                    column.name
                )));
            }
        }

        Ok(())
    }

    pub fn estimated_size(&self) -> usize {
        self.name.len() + self.columns.iter().map(|c| c.estimated_size()).sum::<usize>()
    }
}

impl Column {
    pub fn new(name: String, data_type: DataType, nullable: bool) -> Self {
        Column {
            name,
            data_type,
            nullable,
            default_value: None,
            auto_increment: false,
            primary_key: false,
            unique: false,
        }
    }

    pub fn estimated_size(&self) -> usize {
        self.name.len() + 32 // Rough estimate
    }
}
