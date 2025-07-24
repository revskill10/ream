/// ORM type system - Type-safe database operations
/// 
/// This module provides type-safe abstractions for database operations:
/// - Row types for query results
/// - Type conversions between Rust and SQL types
/// - Serialization/deserialization support
/// - Type-safe query builders

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::fmt;
use crate::sqlite::types::{Value, DataType};

/// Generic row trait for database results
pub trait Row: Send + Sync + Clone {
    /// Get a value by column name
    fn get(&self, column: &str) -> Option<&Value>;
    
    /// Get a value by column index
    fn get_by_index(&self, index: usize) -> Option<&Value>;
    
    /// Get all column names
    fn columns(&self) -> &[String];
    
    /// Get all values
    fn values(&self) -> &[Value];
    
    /// Convert to a typed struct
    fn into_typed<T: FromRow>(self) -> Result<T, TypeConversionError>;
}

/// Trait for converting from database rows to Rust types
pub trait FromRow: Sized {
    /// Convert from a database row
    fn from_row<R: Row>(row: R) -> Result<Self, TypeConversionError>;
}

/// Trait for converting from Rust types to database values
pub trait ToRow {
    /// Convert to database values
    fn to_row(&self) -> Result<HashMap<String, Value>, TypeConversionError>;
}

/// Type conversion errors
#[derive(Debug, thiserror::Error)]
pub enum TypeConversionError {
    #[error("Missing column: {column}")]
    MissingColumn { column: String },
    
    #[error("Type mismatch for column {column}: expected {expected}, got {actual}")]
    TypeMismatch {
        column: String,
        expected: String,
        actual: String,
    },
    
    #[error("Null value for non-nullable column: {column}")]
    NullValue { column: String },
    
    #[error("Serialization error: {message}")]
    Serialization { message: String },
    
    #[error("Deserialization error: {message}")]
    Deserialization { message: String },
}

/// Generic database row implementation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseRow {
    pub columns: Vec<String>,
    pub values: Vec<Value>,
}

impl DatabaseRow {
    pub fn new(columns: Vec<String>, values: Vec<Value>) -> Self {
        Self { columns, values }
    }
    
    pub fn from_pairs(pairs: Vec<(String, Value)>) -> Self {
        let (columns, values) = pairs.into_iter().unzip();
        Self { columns, values }
    }
}

impl Row for DatabaseRow {
    fn get(&self, column: &str) -> Option<&Value> {
        self.columns
            .iter()
            .position(|c| c == column)
            .and_then(|i| self.values.get(i))
    }
    
    fn get_by_index(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }
    
    fn columns(&self) -> &[String] {
        &self.columns
    }
    
    fn values(&self) -> &[Value] {
        &self.values
    }
    
    fn into_typed<T: FromRow>(self) -> Result<T, TypeConversionError> {
        T::from_row(self)
    }
}

/// Macro for deriving FromRow and ToRow traits
/// This would be implemented as a procedural macro in a real system
pub trait Queryable: FromRow + ToRow + Send + Sync + Clone {}

/// Helper trait for extracting typed values from database values
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self, TypeConversionError>;
}

/// Helper trait for converting Rust values to database values
pub trait ToValue {
    fn to_value(&self) -> Value;
}

// Implementations for common Rust types

impl FromValue for i64 {
    fn from_value(value: &Value) -> Result<Self, TypeConversionError> {
        match value {
            Value::Integer(i) => Ok(*i),
            Value::Null => Err(TypeConversionError::NullValue { 
                column: "unknown".to_string() 
            }),
            _ => Err(TypeConversionError::TypeMismatch {
                column: "unknown".to_string(),
                expected: "Integer".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::Integer(*self)
    }
}

impl FromValue for f64 {
    fn from_value(value: &Value) -> Result<Self, TypeConversionError> {
        match value {
            Value::Real(f) => Ok(*f),
            Value::Integer(i) => Ok(*i as f64),
            Value::Null => Err(TypeConversionError::NullValue { 
                column: "unknown".to_string() 
            }),
            _ => Err(TypeConversionError::TypeMismatch {
                column: "unknown".to_string(),
                expected: "Real".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::Real(*self)
    }
}

impl FromValue for String {
    fn from_value(value: &Value) -> Result<Self, TypeConversionError> {
        match value {
            Value::Text(s) => Ok(s.clone()),
            Value::Null => Err(TypeConversionError::NullValue { 
                column: "unknown".to_string() 
            }),
            _ => Err(TypeConversionError::TypeMismatch {
                column: "unknown".to_string(),
                expected: "Text".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::Text(self.clone())
    }
}

impl ToValue for &str {
    fn to_value(&self) -> Value {
        Value::Text(self.to_string())
    }
}

impl FromValue for bool {
    fn from_value(value: &Value) -> Result<Self, TypeConversionError> {
        match value {
            Value::Boolean(b) => Ok(*b),
            Value::Integer(i) => Ok(*i != 0),
            Value::Null => Err(TypeConversionError::NullValue { 
                column: "unknown".to_string() 
            }),
            _ => Err(TypeConversionError::TypeMismatch {
                column: "unknown".to_string(),
                expected: "Boolean".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::Boolean(*self)
    }
}

impl FromValue for Vec<u8> {
    fn from_value(value: &Value) -> Result<Self, TypeConversionError> {
        match value {
            Value::Blob(b) => Ok(b.clone()),
            Value::Null => Err(TypeConversionError::NullValue { 
                column: "unknown".to_string() 
            }),
            _ => Err(TypeConversionError::TypeMismatch {
                column: "unknown".to_string(),
                expected: "Blob".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

impl ToValue for Vec<u8> {
    fn to_value(&self) -> Value {
        Value::Blob(self.clone())
    }
}

// Optional types
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: &Value) -> Result<Self, TypeConversionError> {
        match value {
            Value::Null => Ok(None),
            _ => T::from_value(value).map(Some),
        }
    }
}

impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(v) => v.to_value(),
            None => Value::Null,
        }
    }
}

/// Helper function to extract a typed value from a row
pub fn get_typed<T: FromValue, R: Row>(row: &R, column: &str) -> Result<T, TypeConversionError> {
    let value = row.get(column)
        .ok_or_else(|| TypeConversionError::MissingColumn { 
            column: column.to_string() 
        })?;
    
    T::from_value(value).map_err(|mut e| {
        // Update error with correct column name
        match &mut e {
            TypeConversionError::TypeMismatch { column: col, .. } => *col = column.to_string(),
            TypeConversionError::NullValue { column: col } => *col = column.to_string(),
            _ => {}
        }
        e
    })
}

/// Helper function to extract an optional typed value from a row
pub fn get_optional<T: FromValue, R: Row>(row: &R, column: &str) -> Result<Option<T>, TypeConversionError> {
    match row.get(column) {
        Some(Value::Null) => Ok(None),
        Some(value) => T::from_value(value).map(Some).map_err(|mut e| {
            match &mut e {
                TypeConversionError::TypeMismatch { column: col, .. } => *col = column.to_string(),
                TypeConversionError::NullValue { column: col } => *col = column.to_string(),
                _ => {}
            }
            e
        }),
        None => Ok(None),
    }
}

/// Example user struct demonstrating manual FromRow implementation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub active: bool,
}

impl FromRow for User {
    fn from_row<R: Row>(row: R) -> Result<Self, TypeConversionError> {
        Ok(User {
            id: get_typed(&row, "id")?,
            name: get_typed(&row, "name")?,
            email: get_optional(&row, "email")?,
            active: get_typed(&row, "active")?,
        })
    }
}

impl ToRow for User {
    fn to_row(&self) -> Result<HashMap<String, Value>, TypeConversionError> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), self.id.to_value());
        map.insert("name".to_string(), self.name.to_value());
        map.insert("email".to_string(), self.email.to_value());
        map.insert("active".to_string(), self.active.to_value());
        Ok(map)
    }
}

impl Queryable for User {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_row() {
        let row = DatabaseRow::new(
            vec!["id".to_string(), "name".to_string()],
            vec![Value::Integer(1), Value::Text("Alice".to_string())],
        );
        
        assert_eq!(row.get("id"), Some(&Value::Integer(1)));
        assert_eq!(row.get("name"), Some(&Value::Text("Alice".to_string())));
        assert_eq!(row.get("missing"), None);
        
        assert_eq!(row.get_by_index(0), Some(&Value::Integer(1)));
        assert_eq!(row.get_by_index(1), Some(&Value::Text("Alice".to_string())));
        assert_eq!(row.get_by_index(2), None);
    }

    #[test]
    fn test_type_conversions() {
        let int_value = Value::Integer(42);
        let result: Result<i64, _> = i64::from_value(&int_value);
        assert_eq!(result.unwrap(), 42);
        
        let text_value = Value::Text("hello".to_string());
        let result: Result<String, _> = String::from_value(&text_value);
        assert_eq!(result.unwrap(), "hello");
        
        let bool_value = Value::Boolean(true);
        let result: Result<bool, _> = bool::from_value(&bool_value);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_optional_types() {
        let null_value = Value::Null;
        let result: Result<Option<String>, _> = Option::<String>::from_value(&null_value);
        assert_eq!(result.unwrap(), None);
        
        let text_value = Value::Text("hello".to_string());
        let result: Result<Option<String>, _> = Option::<String>::from_value(&text_value);
        assert_eq!(result.unwrap(), Some("hello".to_string()));
    }

    #[test]
    fn test_user_from_row() {
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
    }

    #[test]
    fn test_user_to_row() {
        let user = User {
            id: 1,
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            active: true,
        };
        
        let row_data = user.to_row().unwrap();
        assert_eq!(row_data.get("id"), Some(&Value::Integer(1)));
        assert_eq!(row_data.get("name"), Some(&Value::Text("Alice".to_string())));
        assert_eq!(row_data.get("email"), Some(&Value::Text("alice@example.com".to_string())));
        assert_eq!(row_data.get("active"), Some(&Value::Boolean(true)));
    }
}
