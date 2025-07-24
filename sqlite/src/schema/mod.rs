pub mod registry;
pub mod table;
pub mod index;
pub mod constraint;

pub use registry::SchemaRegistry;
pub use table::{Table, TableMetadata};
pub use index::{Index, IndexMetadata};
pub use constraint::{Constraint, ConstraintType};

use crate::error::{SqlError, SqlResult};
use crate::types::{DataType, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database schema following algebraic construction patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub tables: HashMap<String, Table>,
    pub indexes: HashMap<String, Index>,
    pub constraints: Vec<Constraint>,
    pub version: u64,
    pub created_at: std::time::SystemTime,
    pub modified_at: std::time::SystemTime,
}

impl Schema {
    /// Create an empty schema (monoid identity)
    pub fn empty() -> Self {
        let now = std::time::SystemTime::now();
        Schema {
            tables: HashMap::new(),
            indexes: HashMap::new(),
            constraints: Vec::new(),
            version: 1,
            created_at: now,
            modified_at: now,
        }
    }

    /// Combine two schemas (monoid operation)
    pub fn combine(mut self, other: Self) -> Self {
        self.tables.extend(other.tables);
        self.indexes.extend(other.indexes);
        self.constraints.extend(other.constraints);
        self.version = std::cmp::max(self.version, other.version) + 1;
        self.modified_at = std::time::SystemTime::now();
        self
    }

    /// Add a table to the schema
    pub fn add_table(&mut self, table: Table) -> SqlResult<()> {
        if self.tables.contains_key(&table.name) {
            return Err(SqlError::schema_error(format!(
                "Table '{}' already exists",
                table.name
            )));
        }

        self.tables.insert(table.name.clone(), table);
        self.version += 1;
        self.modified_at = std::time::SystemTime::now();
        Ok(())
    }

    /// Remove a table from the schema
    pub fn remove_table(&mut self, table_name: &str) -> SqlResult<Table> {
        let table = self.tables.remove(table_name).ok_or_else(|| {
            SqlError::table_not_found(table_name)
        })?;

        // Remove associated indexes
        self.indexes.retain(|_, index| index.table_name != table_name);

        // Remove associated constraints
        self.constraints.retain(|constraint| {
            !constraint.affected_tables().contains(&table_name.to_string())
        });

        self.version += 1;
        self.modified_at = std::time::SystemTime::now();
        Ok(table)
    }

    /// Add an index to the schema
    pub fn add_index(&mut self, index: Index) -> SqlResult<()> {
        if self.indexes.contains_key(&index.name) {
            return Err(SqlError::schema_error(format!(
                "Index '{}' already exists",
                index.name
            )));
        }

        // Verify the table exists
        if !self.tables.contains_key(&index.table_name) {
            return Err(SqlError::table_not_found(&index.table_name));
        }

        // Verify columns exist in the table
        let table = &self.tables[&index.table_name];
        for column in &index.columns {
            if !table.has_column(column) {
                return Err(SqlError::column_not_found(column));
            }
        }

        self.indexes.insert(index.name.clone(), index);
        self.version += 1;
        self.modified_at = std::time::SystemTime::now();
        Ok(())
    }

    /// Remove an index from the schema
    pub fn remove_index(&mut self, index_name: &str) -> SqlResult<Index> {
        let index = self.indexes.remove(index_name).ok_or_else(|| {
            SqlError::index_not_found(index_name)
        })?;

        self.version += 1;
        self.modified_at = std::time::SystemTime::now();
        Ok(index)
    }

    /// Add a constraint to the schema
    pub fn add_constraint(&mut self, constraint: Constraint) -> SqlResult<()> {
        // Validate constraint
        constraint.validate(self)?;

        self.constraints.push(constraint);
        self.version += 1;
        self.modified_at = std::time::SystemTime::now();
        Ok(())
    }

    /// Get a table by name
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    /// Get a mutable table by name
    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut Table> {
        self.tables.get_mut(name)
    }

    /// Get an index by name
    pub fn get_index(&self, name: &str) -> Option<&Index> {
        self.indexes.get(name)
    }

    /// Get all indexes for a table
    pub fn get_table_indexes(&self, table_name: &str) -> Vec<&Index> {
        self.indexes
            .values()
            .filter(|index| index.table_name == table_name)
            .collect()
    }

    /// Get all constraints for a table
    pub fn get_table_constraints(&self, table_name: &str) -> Vec<&Constraint> {
        self.constraints
            .iter()
            .filter(|constraint| {
                constraint.affected_tables().contains(&table_name.to_string())
            })
            .collect()
    }

    /// Check if a table exists
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    /// Check if an index exists
    pub fn has_index(&self, name: &str) -> bool {
        self.indexes.contains_key(name)
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    /// Get all index names
    pub fn index_names(&self) -> Vec<String> {
        self.indexes.keys().cloned().collect()
    }

    /// Validate the entire schema
    pub fn validate(&self) -> SqlResult<()> {
        // Validate all tables
        for table in self.tables.values() {
            table.validate()?;
        }

        // Validate all indexes
        for index in self.indexes.values() {
            index.validate(self)?;
        }

        // Validate all constraints
        for constraint in &self.constraints {
            constraint.validate(self)?;
        }

        Ok(())
    }

    /// Get schema statistics
    pub fn statistics(&self) -> SchemaStatistics {
        SchemaStatistics {
            table_count: self.tables.len(),
            index_count: self.indexes.len(),
            constraint_count: self.constraints.len(),
            total_columns: self.tables.values().map(|t| t.columns.len()).sum(),
            version: self.version,
            size_estimate: self.estimated_size(),
        }
    }

    /// Estimate schema size in bytes
    pub fn estimated_size(&self) -> usize {
        let tables_size: usize = self.tables.values().map(|t| t.estimated_size()).sum();
        let indexes_size: usize = self.indexes.values().map(|i| i.estimated_size()).sum();
        let constraints_size: usize = self.constraints.iter().map(|c| c.estimated_size()).sum();
        
        tables_size + indexes_size + constraints_size
    }

    /// Create a diff between this schema and another
    pub fn diff(&self, other: &Schema) -> SchemaDiff {
        let mut diff = SchemaDiff::new();

        // Find added tables
        for (name, table) in &other.tables {
            if !self.tables.contains_key(name) {
                diff.added_tables.push(table.clone());
            }
        }

        // Find removed tables
        for (name, table) in &self.tables {
            if !other.tables.contains_key(name) {
                diff.removed_tables.push(table.clone());
            }
        }

        // Find modified tables
        for (name, other_table) in &other.tables {
            if let Some(self_table) = self.tables.get(name) {
                if self_table != other_table {
                    diff.modified_tables.push((self_table.clone(), other_table.clone()));
                }
            }
        }

        // Similar logic for indexes and constraints...
        diff
    }

    /// Apply a schema migration
    pub fn apply_migration(&mut self, migration: SchemaMigration) -> SqlResult<()> {
        for operation in migration.operations {
            match operation {
                MigrationOperation::CreateTable(table) => {
                    self.add_table(table)?;
                }
                MigrationOperation::DropTable(name) => {
                    self.remove_table(&name)?;
                }
                MigrationOperation::CreateIndex(index) => {
                    self.add_index(index)?;
                }
                MigrationOperation::DropIndex(name) => {
                    self.remove_index(&name)?;
                }
                MigrationOperation::AddConstraint(constraint) => {
                    self.add_constraint(constraint)?;
                }
            }
        }
        Ok(())
    }
}

/// Schema statistics
#[derive(Debug, Clone)]
pub struct SchemaStatistics {
    pub table_count: usize,
    pub index_count: usize,
    pub constraint_count: usize,
    pub total_columns: usize,
    pub version: u64,
    pub size_estimate: usize,
}

/// Schema difference
#[derive(Debug, Clone)]
pub struct SchemaDiff {
    pub added_tables: Vec<Table>,
    pub removed_tables: Vec<Table>,
    pub modified_tables: Vec<(Table, Table)>, // (old, new)
    pub added_indexes: Vec<Index>,
    pub removed_indexes: Vec<Index>,
    pub added_constraints: Vec<Constraint>,
    pub removed_constraints: Vec<Constraint>,
}

impl SchemaDiff {
    pub fn new() -> Self {
        SchemaDiff {
            added_tables: Vec::new(),
            removed_tables: Vec::new(),
            modified_tables: Vec::new(),
            added_indexes: Vec::new(),
            removed_indexes: Vec::new(),
            added_constraints: Vec::new(),
            removed_constraints: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.added_tables.is_empty()
            && self.removed_tables.is_empty()
            && self.modified_tables.is_empty()
            && self.added_indexes.is_empty()
            && self.removed_indexes.is_empty()
            && self.added_constraints.is_empty()
            && self.removed_constraints.is_empty()
    }
}

/// Schema migration
#[derive(Debug, Clone)]
pub struct SchemaMigration {
    pub version: u64,
    pub operations: Vec<MigrationOperation>,
    pub description: String,
}

/// Migration operations
#[derive(Debug, Clone)]
pub enum MigrationOperation {
    CreateTable(Table),
    DropTable(String),
    CreateIndex(Index),
    DropIndex(String),
    AddConstraint(Constraint),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::table::Column;

    #[test]
    fn test_empty_schema() {
        let schema = Schema::empty();
        assert_eq!(schema.tables.len(), 0);
        assert_eq!(schema.indexes.len(), 0);
        assert_eq!(schema.constraints.len(), 0);
        assert_eq!(schema.version, 1);
    }

    #[test]
    fn test_add_table() {
        let mut schema = Schema::empty();
        let table = Table::new(
            "users".to_string(),
            vec![
                Column::new("id".to_string(), DataType::Integer, false),
                Column::new("name".to_string(), DataType::Text, false),
            ],
        );

        schema.add_table(table).unwrap();
        assert_eq!(schema.tables.len(), 1);
        assert!(schema.has_table("users"));
        assert_eq!(schema.version, 2);
    }

    #[test]
    fn test_schema_combine() {
        let mut schema1 = Schema::empty();
        let table1 = Table::new(
            "users".to_string(),
            vec![Column::new("id".to_string(), DataType::Integer, false)],
        );
        schema1.add_table(table1).unwrap();

        let mut schema2 = Schema::empty();
        let table2 = Table::new(
            "orders".to_string(),
            vec![Column::new("id".to_string(), DataType::Integer, false)],
        );
        schema2.add_table(table2).unwrap();

        let combined = schema1.combine(schema2);
        assert_eq!(combined.tables.len(), 2);
        assert!(combined.has_table("users"));
        assert!(combined.has_table("orders"));
    }
}
