/// SQL Composable trait for unified SQL composition across query builders
/// 
/// This trait provides a common interface for SQL composition that both
/// QueryBuilder and AdvancedQueryBuilder can implement, ensuring consistent
/// behavior and enabling type-safe column references throughout the ORM system.

use crate::orm::schema::{Column, AliasedColumn, Table, AliasedTable, TypeSafeExpression};
use crate::sqlite::types::Value;
use crate::sqlite::error::SqlResult;
use crate::orm::query::OrderDirection;

/// Trait for SQL composition that enables type-safe column references
/// and consistent query building across different query builder types
pub trait SqlComposable {
    /// The type returned by this composable (e.g., SelectQueryBuilder)
    type Output;
    
    /// Add a column reference using a Column type for type safety
    fn column_ref(self, column: &Column) -> Self::Output;
    
    /// Add a column reference with an alias using Column type
    fn column_ref_as(self, column: &Column, alias: &str) -> Self::Output;
    
    /// Add multiple column references at once
    fn column_refs(self, columns: &[&Column]) -> Self::Output;
    
    /// Add a raw column expression (for backward compatibility)
    fn column_expr(self, expression: &str) -> Self::Output;
    
    /// Add a raw column expression with alias
    fn column_expr_as(self, expression: &str, alias: &str) -> Self::Output;

    /// Add a type-safe expression (eliminates string literals)
    fn column_type_safe_expr(self, expression: &TypeSafeExpression) -> Self::Output;

    /// Add a type-safe expression with an alias (eliminates string literals)
    fn column_type_safe_expr_as(self, expression: &TypeSafeExpression, alias: &str) -> Self::Output;

    /// Set the FROM clause using a string table name (for backward compatibility)
    fn from_table_name(self, table_name: &str) -> Self::Output;

    /// Set the FROM clause with an alias using string names (for backward compatibility)
    fn from_table_name_as(self, table_name: &str, alias: &str) -> Self::Output;

    /// Set the FROM clause using a type-safe Table reference
    fn from_table(self, table: &Table) -> Self::Output;

    /// Set the FROM clause using a type-safe AliasedTable
    fn from_aliased_table(self, aliased_table: &AliasedTable) -> Self::Output;

    /// Add an ORDER BY clause using a type-safe Column
    fn order_by_column(self, column: &Column, direction: crate::orm::query::OrderDirection) -> Self::Output;

    /// Convert to SQL string
    fn to_sql(&self) -> String;

    /// Build and validate the query
    fn build(self) -> SqlResult<Self::Output>;
}

/// Trait for advanced SQL composition features
pub trait AdvancedSqlComposable: SqlComposable {
    /// Add a window function column with type-safe alias
    fn window_function_as(self, alias: &str, function: &str, partition_by: &[&Column], order_by: &[&Column]) -> Self::Output;

    /// Add a JSON extraction column with type-safe alias
    fn json_extract_as(self, alias: &str, column: &Column, path: &str) -> Self::Output;
    
    /// Add a case expression column
    fn case_expression(self, alias: &str, when_clauses: &[(&str, &str)], else_clause: Option<&str>) -> Self::Output;
    
    /// Add an aggregation function column
    fn aggregate_function(self, alias: &str, function: &str, column: &Column) -> Self::Output;

    /// Add an aliased column using AliasedColumn type
    fn aliased_column(self, aliased_col: &AliasedColumn) -> Self::Output;

    /// Add multiple aliased columns at once
    fn aliased_columns(self, aliased_cols: &[&AliasedColumn]) -> Self::Output;
}

/// Helper trait for column aliasing
pub trait ColumnAlias {
    /// Create an aliased column reference
    fn aliased(&self, alias: &str) -> AliasedColumn;
}

impl ColumnAlias for Column {
    fn aliased(&self, alias: &str) -> AliasedColumn {
        AliasedColumn {
            column: self.clone(),
            alias: alias.to_string(),
        }
    }
}

/// Helper trait for table aliasing
pub trait TableAlias {
    /// Create an aliased table reference
    fn aliased(&self, alias: &str) -> AliasedTable;
}

impl TableAlias for Table {
    fn aliased(&self, alias: &str) -> AliasedTable {
        AliasedTable {
            table: self.clone(),
            alias: alias.to_string(),
        }
    }
}



/// Helper functions for creating type-safe column references
pub mod column_helpers {
    use super::*;
    
    /// Create a column reference from a Column
    pub fn col_ref(column: &Column) -> String {
        column.qualified_name()
    }
    
    /// Create an aliased column reference
    pub fn col_ref_as(column: &Column, alias: &str) -> String {
        format!("{} AS {}", column.qualified_name(), alias)
    }
    
    /// Create multiple column references
    pub fn col_refs(columns: &[&Column]) -> Vec<String> {
        columns.iter().map(|col| col.qualified_name()).collect()
    }
    
    /// Create column references with aliases
    pub fn col_refs_with_aliases(columns: &[(&Column, &str)]) -> Vec<String> {
        columns.iter().map(|(col, alias)| {
            format!("{} AS {}", col.qualified_name(), alias)
        }).collect()
    }
}

/// Helper functions for creating type-safe table references
pub mod table_helpers {
    use super::*;

    /// Create a table reference from a Table
    pub fn table_ref(table: &Table) -> String {
        table.qualified_name()
    }

    /// Create an aliased table reference
    pub fn table_ref_as(table: &Table, alias: &str) -> String {
        format!("{} AS {}", table.qualified_name(), alias)
    }

    /// Create a table reference from an AliasedTable
    pub fn aliased_table_ref(aliased_table: &AliasedTable) -> String {
        aliased_table.to_sql()
    }

    /// Create multiple table references
    pub fn table_refs(tables: &[&Table]) -> Vec<String> {
        tables.iter().map(|table| table.qualified_name()).collect()
    }

    /// Create table references with aliases
    pub fn table_refs_with_aliases(tables: &[(&Table, &str)]) -> Vec<String> {
        tables.iter().map(|(table, alias)| {
            format!("{} AS {}", table.qualified_name(), alias)
        }).collect()
    }
}

/// Validation helpers for type-safe column operations
pub mod validation {
    use super::*;
    
    /// Validate that a column has a table name for qualified references
    pub fn validate_qualified_column(column: &Column) -> SqlResult<()> {
        if column.table_name.is_none() {
            return Err(crate::sqlite::error::SqlError::runtime_error(
                &format!("Column '{}' must have a table name for qualified references", column.name)
            ));
        }
        Ok(())
    }
    
    /// Validate that columns are from compatible tables for joins
    pub fn validate_join_columns(left: &Column, right: &Column) -> SqlResult<()> {
        validate_qualified_column(left)?;
        validate_qualified_column(right)?;
        
        if left.table_name == right.table_name {
            return Err(crate::sqlite::error::SqlError::runtime_error(
                "Join columns cannot be from the same table"
            ));
        }
        
        Ok(())
    }
    
    /// Validate that a column supports JSON operations
    pub fn validate_json_column(column: &Column) -> SqlResult<()> {
        use crate::sqlite::types::DataType;

        match column.data_type {
            DataType::Text | DataType::Blob => Ok(()),
            _ => Err(crate::sqlite::error::SqlError::runtime_error(
                &format!("Column '{}' with type '{:?}' does not support JSON operations",
                    column.name, column.data_type)
            ))
        }
    }

    /// Validate that a table name is valid
    pub fn validate_table_name(table: &Table) -> SqlResult<()> {
        if table.name.is_empty() {
            return Err(crate::sqlite::error::SqlError::runtime_error(
                "Table name cannot be empty"
            ));
        }

        // Basic validation - table name should not contain invalid characters
        if table.name.contains(';') || table.name.contains('\'') || table.name.contains('"') {
            return Err(crate::sqlite::error::SqlError::runtime_error(
                &format!("Table name '{}' contains invalid characters", table.name)
            ));
        }

        Ok(())
    }

    /// Validate that an aliased table has valid names
    pub fn validate_aliased_table(aliased_table: &AliasedTable) -> SqlResult<()> {
        validate_table_name(&aliased_table.table)?;

        if aliased_table.alias.is_empty() {
            return Err(crate::sqlite::error::SqlError::runtime_error(
                "Table alias cannot be empty"
            ));
        }

        if aliased_table.alias.contains(';') || aliased_table.alias.contains('\'') || aliased_table.alias.contains('"') {
            return Err(crate::sqlite::error::SqlError::runtime_error(
                &format!("Table alias '{}' contains invalid characters", aliased_table.alias)
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::types::DataType;
    
    #[test]
    fn test_column_alias() {
        let column = Column::new("name", DataType::Text)
            .with_table_name("users");
        
        let aliased = column.aliased("user_name");
        assert_eq!(aliased.alias, "user_name");
        assert_eq!(aliased.to_sql(), "users.name AS user_name");
    }
    
    #[test]
    fn test_column_helpers() {
        let column = Column::new("id", DataType::Integer)
            .with_table_name("users");
        
        assert_eq!(column_helpers::col_ref(&column), "users.id");
        assert_eq!(column_helpers::col_ref_as(&column, "user_id"), "users.id AS user_id");
        
        let columns = vec![&column];
        let refs = column_helpers::col_refs(&columns);
        assert_eq!(refs, vec!["users.id"]);
    }
    
    #[test]
    fn test_validation() {
        let column_without_table = Column::new("name", DataType::Text);
        assert!(validation::validate_qualified_column(&column_without_table).is_err());
        
        let column_with_table = Column::new("name", DataType::Text)
            .with_table_name("users");
        assert!(validation::validate_qualified_column(&column_with_table).is_ok());
    }
    
    #[test]
    fn test_json_validation() {
        let text_column = Column::new("data", DataType::Text);
        assert!(validation::validate_json_column(&text_column).is_ok());

        let int_column = Column::new("id", DataType::Integer);
        assert!(validation::validate_json_column(&int_column).is_err());
    }

    #[test]
    fn test_table_alias() {
        let table = Table::new("users");
        let aliased = table.aliased("u");

        assert_eq!(aliased.alias, "u");
        assert_eq!(aliased.to_sql(), "users AS u");
    }

    #[test]
    fn test_table_helpers() {
        let table = Table::new("users").with_schema("public");

        assert_eq!(table_helpers::table_ref(&table), "public.users");
        assert_eq!(table_helpers::table_ref_as(&table, "u"), "public.users AS u");

        let tables = vec![&table];
        let refs = table_helpers::table_refs(&tables);
        assert_eq!(refs, vec!["public.users"]);
    }

    #[test]
    fn test_table_validation() {
        let valid_table = Table::new("users");
        assert!(validation::validate_table_name(&valid_table).is_ok());

        let invalid_table = Table::new("users; DROP TABLE users;");
        assert!(validation::validate_table_name(&invalid_table).is_err());

        let aliased_table = valid_table.as_alias("u");
        assert!(validation::validate_aliased_table(&aliased_table).is_ok());

        let invalid_aliased = valid_table.as_alias("u; DROP TABLE users;");
        assert!(validation::validate_aliased_table(&invalid_aliased).is_err());
    }
}
