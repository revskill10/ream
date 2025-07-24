use crate::error::{SqlError, SqlResult};
use crate::parser::ast::Expression;
use crate::schema::Schema;
use serde::{Deserialize, Serialize};

/// Constraint definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub constraint_type: ConstraintType,
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    PrimaryKey {
        table: String,
        columns: Vec<String>,
    },
    ForeignKey {
        table: String,
        columns: Vec<String>,
        foreign_table: String,
        foreign_columns: Vec<String>,
    },
    Unique {
        table: String,
        columns: Vec<String>,
    },
    Check {
        table: String,
        expression: Expression,
    },
    NotNull {
        table: String,
        column: String,
    },
}

impl Constraint {
    pub fn validate(&self, schema: &Schema) -> SqlResult<()> {
        match &self.constraint_type {
            ConstraintType::PrimaryKey { table, columns } => {
                if !schema.has_table(table) {
                    return Err(SqlError::table_not_found(table));
                }
                
                let table_def = schema.get_table(table).unwrap();
                for column in columns {
                    if !table_def.has_column(column) {
                        return Err(SqlError::column_not_found(column));
                    }
                }
            }
            ConstraintType::ForeignKey { table, columns, foreign_table, foreign_columns } => {
                if !schema.has_table(table) {
                    return Err(SqlError::table_not_found(table));
                }
                if !schema.has_table(foreign_table) {
                    return Err(SqlError::table_not_found(foreign_table));
                }
                
                if columns.len() != foreign_columns.len() {
                    return Err(SqlError::schema_error("Foreign key column count mismatch"));
                }
            }
            ConstraintType::Unique { table, columns } => {
                if !schema.has_table(table) {
                    return Err(SqlError::table_not_found(table));
                }
                
                let table_def = schema.get_table(table).unwrap();
                for column in columns {
                    if !table_def.has_column(column) {
                        return Err(SqlError::column_not_found(column));
                    }
                }
            }
            ConstraintType::Check { table, .. } => {
                if !schema.has_table(table) {
                    return Err(SqlError::table_not_found(table));
                }
            }
            ConstraintType::NotNull { table, column } => {
                if !schema.has_table(table) {
                    return Err(SqlError::table_not_found(table));
                }
                
                let table_def = schema.get_table(table).unwrap();
                if !table_def.has_column(column) {
                    return Err(SqlError::column_not_found(column));
                }
            }
        }
        
        Ok(())
    }

    pub fn affected_tables(&self) -> Vec<String> {
        match &self.constraint_type {
            ConstraintType::PrimaryKey { table, .. } => vec![table.clone()],
            ConstraintType::ForeignKey { table, foreign_table, .. } => {
                vec![table.clone(), foreign_table.clone()]
            }
            ConstraintType::Unique { table, .. } => vec![table.clone()],
            ConstraintType::Check { table, .. } => vec![table.clone()],
            ConstraintType::NotNull { table, .. } => vec![table.clone()],
        }
    }

    pub fn estimated_size(&self) -> usize {
        self.name.len() + 64 // Rough estimate
    }
}
