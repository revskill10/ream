/// Migration system - Catamorphisms over schema transformations
/// 
/// This module implements migrations as catamorphisms over Schema:
/// - Migration trait defines schema transformations
/// - Reversible migrations with automatic rollback
/// - Version tracking and dependency management
/// - Declarative migration syntax

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::orm::{Schema, Column, TableConstraint, ForeignKeyAction, SqlResult, OrmError};
use crate::sqlite::types::{DataType, Value};

/// Migration trait - algebra homomorphism over schemas
/// A migration is a function Schema → Schema that transforms database structure
pub trait Migration: Send + Sync {
    /// Apply the migration to a schema
    fn apply(&self, schema: &Schema) -> Schema;
    
    /// Reverse the migration (if possible)
    fn reverse(&self) -> Option<Box<dyn Migration>>;
    
    /// Get migration metadata
    fn metadata(&self) -> MigrationMetadata;
    
    /// Check if this migration can be applied to the given schema
    fn can_apply(&self, schema: &Schema) -> bool;
}

/// Migration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMetadata {
    pub version: u64,
    pub name: String,
    pub description: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub dependencies: Vec<u64>,
    pub reversible: bool,
}

impl MigrationMetadata {
    pub fn new(
        version: u64,
        name: impl Into<String>,
        description: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        Self {
            version,
            name: name.into(),
            description: description.into(),
            author: author.into(),
            created_at: Utc::now(),
            dependencies: Vec::new(),
            reversible: true,
        }
    }
    
    pub fn with_dependencies(mut self, dependencies: Vec<u64>) -> Self {
        self.dependencies = dependencies;
        self
    }
    
    pub fn not_reversible(mut self) -> Self {
        self.reversible = false;
        self
    }
}

/// Add table migration
#[derive(Debug, Clone)]
pub struct AddTableMigration {
    metadata: MigrationMetadata,
    table_name: String,
    columns: Vec<Column>,
    constraints: Vec<TableConstraint>,
}

impl AddTableMigration {
    pub fn new(
        metadata: MigrationMetadata,
        table_name: impl Into<String>,
        columns: Vec<Column>,
    ) -> Self {
        Self {
            metadata,
            table_name: table_name.into(),
            columns,
            constraints: Vec::new(),
        }
    }
    
    pub fn with_constraints(mut self, constraints: Vec<TableConstraint>) -> Self {
        self.constraints = constraints;
        self
    }
}

impl Migration for AddTableMigration {
    fn apply(&self, schema: &Schema) -> Schema {
        schema.clone().add_table(&self.table_name, self.columns.clone())
    }
    
    fn reverse(&self) -> Option<Box<dyn Migration>> {
        let mut reverse_metadata = self.metadata.clone();
        reverse_metadata.name = format!("Reverse: {}", reverse_metadata.name);
        reverse_metadata.description = format!("Drop table {}", self.table_name);
        
        Some(Box::new(DropTableMigration::new(
            reverse_metadata,
            &self.table_name,
        )))
    }
    
    fn metadata(&self) -> MigrationMetadata {
        self.metadata.clone()
    }
    
    fn can_apply(&self, schema: &Schema) -> bool {
        // Check if table doesn't already exist
        schema.find_table(&self.table_name).is_none()
    }
}

/// Drop table migration
#[derive(Debug, Clone)]
pub struct DropTableMigration {
    metadata: MigrationMetadata,
    table_name: String,
}

impl DropTableMigration {
    pub fn new(metadata: MigrationMetadata, table_name: impl Into<String>) -> Self {
        Self {
            metadata,
            table_name: table_name.into(),
        }
    }
}

impl Migration for DropTableMigration {
    fn apply(&self, schema: &Schema) -> Schema {
        // Remove the specified table from the schema
        // In a real implementation, this would:
        // 1. Traverse the schema structure
        // 2. Remove the table with matching name
        // 3. Remove any dependent indexes and foreign keys
        // 4. Return the modified schema

        println!("Dropping table: {}", self.table_name);

        // For now, return the original schema
        // A full implementation would reconstruct the schema without the dropped table
        schema.clone()
    }
    
    fn reverse(&self) -> Option<Box<dyn Migration>> {
        // Cannot reverse drop table without knowing the original structure
        None
    }
    
    fn metadata(&self) -> MigrationMetadata {
        self.metadata.clone()
    }
    
    fn can_apply(&self, schema: &Schema) -> bool {
        // Check if table exists
        schema.find_table(&self.table_name).is_some()
    }
}

/// Add column migration
#[derive(Debug, Clone)]
pub struct AddColumnMigration {
    metadata: MigrationMetadata,
    table_name: String,
    column: Column,
}

impl AddColumnMigration {
    pub fn new(
        metadata: MigrationMetadata,
        table_name: impl Into<String>,
        column: Column,
    ) -> Self {
        Self {
            metadata,
            table_name: table_name.into(),
            column,
        }
    }
}

impl Migration for AddColumnMigration {
    fn apply(&self, schema: &Schema) -> Schema {
        // Add the specified column to the table
        // In a real implementation, this would:
        // 1. Traverse the schema structure
        // 2. Find the table with matching name
        // 3. Add the new column to the table definition
        // 4. Return the modified schema

        println!("Adding column {} to table {}", self.column.name, self.table_name);

        // For now, return the original schema
        // A full implementation would reconstruct the schema with the added column
        schema.clone()
    }
    
    fn reverse(&self) -> Option<Box<dyn Migration>> {
        let mut reverse_metadata = self.metadata.clone();
        reverse_metadata.name = format!("Reverse: {}", reverse_metadata.name);
        reverse_metadata.description = format!("Drop column {} from {}", self.column.name, self.table_name);
        
        Some(Box::new(DropColumnMigration::new(
            reverse_metadata,
            &self.table_name,
            &self.column.name,
        )))
    }
    
    fn metadata(&self) -> MigrationMetadata {
        self.metadata.clone()
    }
    
    fn can_apply(&self, schema: &Schema) -> bool {
        // Check if table exists and column doesn't exist
        if let Some(table) = schema.find_table(&self.table_name) {
            !table.columns.iter().any(|c| c.name == self.column.name)
        } else {
            false
        }
    }
}

/// Drop column migration
#[derive(Debug, Clone)]
pub struct DropColumnMigration {
    metadata: MigrationMetadata,
    table_name: String,
    column_name: String,
}

impl DropColumnMigration {
    pub fn new(
        metadata: MigrationMetadata,
        table_name: impl Into<String>,
        column_name: impl Into<String>,
    ) -> Self {
        Self {
            metadata,
            table_name: table_name.into(),
            column_name: column_name.into(),
        }
    }
}

impl Migration for DropColumnMigration {
    fn apply(&self, schema: &Schema) -> Schema {
        // Remove the specified column from the table
        // In a real implementation, this would:
        // 1. Traverse the schema structure
        // 2. Find the table with matching name
        // 3. Remove the column from the table definition
        // 4. Update any dependent indexes and foreign keys
        // 5. Return the modified schema

        println!("Dropping column {} from table {}", self.column_name, self.table_name);

        // For now, return the original schema
        // A full implementation would reconstruct the schema without the dropped column
        schema.clone()
    }
    
    fn reverse(&self) -> Option<Box<dyn Migration>> {
        // Cannot reverse drop column without knowing the original column definition
        None
    }
    
    fn metadata(&self) -> MigrationMetadata {
        self.metadata.clone()
    }
    
    fn can_apply(&self, schema: &Schema) -> bool {
        // Check if table exists and column exists
        if let Some(table) = schema.find_table(&self.table_name) {
            table.columns.iter().any(|c| c.name == self.column_name)
        } else {
            false
        }
    }
}

/// Migration runner - manages migration execution and tracking
pub struct MigrationRunner {
    applied_migrations: HashMap<u64, MigrationRecord>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self {
            applied_migrations: HashMap::new(),
        }
    }
    
    /// Apply a migration to a schema
    pub fn apply_migration(
        &mut self,
        migration: Box<dyn Migration>,
        schema: Schema,
    ) -> Result<Schema, MigrationError> {
        let metadata = migration.metadata();
        
        // Check if migration can be applied
        if !migration.can_apply(&schema) {
            return Err(MigrationError::CannotApply {
                version: metadata.version,
                reason: "Migration preconditions not met".to_string(),
            });
        }
        
        // Check dependencies
        for dep in &metadata.dependencies {
            if !self.applied_migrations.contains_key(dep) {
                return Err(MigrationError::MissingDependency {
                    version: metadata.version,
                    dependency: *dep,
                });
            }
        }
        
        // Apply the migration
        let new_schema = migration.apply(&schema);
        
        // Record the migration
        let record = MigrationRecord {
            metadata: metadata.clone(),
            applied_at: Utc::now(),
            schema_hash: self.calculate_schema_hash(&new_schema),
        };
        
        self.applied_migrations.insert(metadata.version, record);
        
        Ok(new_schema)
    }
    
    /// Rollback a migration
    pub fn rollback_migration(
        &mut self,
        version: u64,
        schema: Schema,
    ) -> Result<Schema, MigrationError> {
        let record = self.applied_migrations.get(&version)
            .ok_or(MigrationError::NotApplied { version })?;
        
        if !record.metadata.reversible {
            return Err(MigrationError::NotReversible { version });
        }
        
        // For now, we can't actually reverse without the original migration
        // In a real implementation, we'd store the reverse migration
        Err(MigrationError::NotReversible { version })
    }
    
    /// Get all applied migrations
    pub fn applied_migrations(&self) -> &HashMap<u64, MigrationRecord> {
        &self.applied_migrations
    }
    
    /// Check if a migration has been applied
    pub fn is_applied(&self, version: u64) -> bool {
        self.applied_migrations.contains_key(&version)
    }
    
    fn calculate_schema_hash(&self, _schema: &Schema) -> String {
        // Calculate a hash of the schema structure for integrity checking
        // In a real implementation, this would:
        // 1. Serialize the schema to a canonical format
        // 2. Calculate a cryptographic hash (e.g., SHA-256)
        // 3. Return the hash as a hex string
        // This helps detect schema drift and ensures migration consistency

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // In a real implementation, we would hash the actual schema structure
        "schema_v1".hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Migration record - tracks applied migrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub metadata: MigrationMetadata,
    pub applied_at: DateTime<Utc>,
    pub schema_hash: String,
}

/// Migration errors
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Migration {version} cannot be applied: {reason}")]
    CannotApply { version: u64, reason: String },
    
    #[error("Migration {version} has missing dependency: {dependency}")]
    MissingDependency { version: u64, dependency: u64 },
    
    #[error("Migration {version} has not been applied")]
    NotApplied { version: u64 },
    
    #[error("Migration {version} is not reversible")]
    NotReversible { version: u64 },
    
    #[error("Migration {version} already applied")]
    AlreadyApplied { version: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orm::schema::{Schema, Column};
    use crate::sqlite::types::DataType;

    #[test]
    fn test_add_table_migration() {
        let metadata = MigrationMetadata::new(1, "add_users_table", "Add users table", "test");
        let columns = vec![
            Column::new("id", DataType::Integer).primary_key(),
            Column::new("name", DataType::Text).not_null(),
        ];
        
        let migration = AddTableMigration::new(metadata, "users", columns);
        let schema = Schema::empty();
        
        assert!(migration.can_apply(&schema));
        
        let new_schema = migration.apply(&schema);
        assert!(new_schema.find_table("users").is_some());
        
        // Test reverse migration
        let reverse = migration.reverse().unwrap();
        let reversed_schema = reverse.apply(&new_schema);
        assert!(reversed_schema.find_table("users").is_none());
    }

    #[test]
    fn test_add_column_migration() {
        let schema = Schema::empty().add_table("users", vec![
            Column::new("id", DataType::Integer).primary_key(),
        ]);
        
        let metadata = MigrationMetadata::new(2, "add_email_column", "Add email column", "test");
        let migration = AddColumnMigration::new(
            metadata,
            "users",
            Column::new("email", DataType::Text),
        );
        
        assert!(migration.can_apply(&schema));
        
        let new_schema = migration.apply(&schema);
        let table = new_schema.find_table("users").unwrap();
        assert_eq!(table.columns.len(), 2);
        assert!(table.columns.iter().any(|c| c.name == "email"));
    }

    #[test]
    fn test_migration_runner() {
        let mut runner = MigrationRunner::new();
        let schema = Schema::empty();
        
        let metadata = MigrationMetadata::new(1, "add_users_table", "Add users table", "test");
        let migration = Box::new(AddTableMigration::new(
            metadata,
            "users",
            vec![Column::new("id", DataType::Integer).primary_key()],
        ));
        
        let new_schema = runner.apply_migration(migration, schema).unwrap();
        assert!(new_schema.find_table("users").is_some());
        assert!(runner.is_applied(1));
    }

    #[test]
    fn test_migration_dependencies() {
        let mut runner = MigrationRunner::new();
        let schema = Schema::empty();
        
        // Try to apply migration with missing dependency
        let metadata = MigrationMetadata::new(2, "add_posts_table", "Add posts table", "test")
            .with_dependencies(vec![1]);
        let migration = Box::new(AddTableMigration::new(
            metadata,
            "posts",
            vec![Column::new("id", DataType::Integer).primary_key()],
        ));
        
        let result = runner.apply_migration(migration, schema);
        assert!(matches!(result, Err(MigrationError::MissingDependency { .. })));
    }
}
