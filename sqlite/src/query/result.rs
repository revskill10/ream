use crate::types::{Row, Value};
use serde::{Deserialize, Serialize};

/// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResult {
    /// SELECT query result
    Select {
        columns: Vec<String>,
        rows: Vec<Row>,
    },
    /// INSERT query result
    Insert {
        rows_affected: u64,
    },
    /// UPDATE query result
    Update {
        rows_affected: u64,
    },
    /// DELETE query result
    Delete {
        rows_affected: u64,
    },
    /// CREATE TABLE query result
    CreateTable,
    /// DROP TABLE query result
    DropTable,
    /// CREATE INDEX query result
    CreateIndex,
    /// DROP INDEX query result
    DropIndex,
}

impl QueryResult {
    /// Create a SELECT result
    pub fn select(columns: Vec<String>, rows: Vec<Row>) -> Self {
        QueryResult::Select { columns, rows }
    }

    /// Create an INSERT result
    pub fn insert(rows_affected: u64) -> Self {
        QueryResult::Insert { rows_affected }
    }

    /// Create an UPDATE result
    pub fn update(rows_affected: u64) -> Self {
        QueryResult::Update { rows_affected }
    }

    /// Create a DELETE result
    pub fn delete(rows_affected: u64) -> Self {
        QueryResult::Delete { rows_affected }
    }

    /// Create a CREATE TABLE result
    pub fn create_table() -> Self {
        QueryResult::CreateTable
    }

    /// Create a DROP TABLE result
    pub fn drop_table() -> Self {
        QueryResult::DropTable
    }

    /// Create a CREATE INDEX result
    pub fn create_index() -> Self {
        QueryResult::CreateIndex
    }

    /// Create a DROP INDEX result
    pub fn drop_index() -> Self {
        QueryResult::DropIndex
    }

    /// Get the number of rows affected (for DML operations)
    pub fn rows_affected(&self) -> Option<u64> {
        match self {
            QueryResult::Insert { rows_affected }
            | QueryResult::Update { rows_affected }
            | QueryResult::Delete { rows_affected } => Some(*rows_affected),
            _ => None,
        }
    }

    /// Get the result rows (for SELECT operations)
    pub fn rows(&self) -> Option<&Vec<Row>> {
        match self {
            QueryResult::Select { rows, .. } => Some(rows),
            _ => None,
        }
    }

    /// Get the column names (for SELECT operations)
    pub fn columns(&self) -> Option<&Vec<String>> {
        match self {
            QueryResult::Select { columns, .. } => Some(columns),
            _ => None,
        }
    }

    /// Check if this is a SELECT result
    pub fn is_select(&self) -> bool {
        matches!(self, QueryResult::Select { .. })
    }

    /// Check if this is a DML (Data Manipulation Language) result
    pub fn is_dml(&self) -> bool {
        matches!(
            self,
            QueryResult::Insert { .. } | QueryResult::Update { .. } | QueryResult::Delete { .. }
        )
    }

    /// Check if this is a DDL (Data Definition Language) result
    pub fn is_ddl(&self) -> bool {
        matches!(
            self,
            QueryResult::CreateTable
                | QueryResult::DropTable
                | QueryResult::CreateIndex
                | QueryResult::DropIndex
        )
    }

    /// Get the number of result rows (for SELECT operations)
    pub fn row_count(&self) -> Option<usize> {
        match self {
            QueryResult::Select { rows, .. } => Some(rows.len()),
            _ => None,
        }
    }

    /// Check if the result is empty (no rows for SELECT, no affected rows for DML)
    pub fn is_empty(&self) -> bool {
        match self {
            QueryResult::Select { rows, .. } => rows.is_empty(),
            QueryResult::Insert { rows_affected }
            | QueryResult::Update { rows_affected }
            | QueryResult::Delete { rows_affected } => *rows_affected == 0,
            _ => false,
        }
    }

    /// Convert to a formatted string representation
    pub fn to_formatted_string(&self) -> String {
        match self {
            QueryResult::Select { columns, rows } => {
                if rows.is_empty() {
                    "No rows returned".to_string()
                } else {
                    let mut result = String::new();
                    
                    // Header
                    result.push_str(&columns.join(" | "));
                    result.push('\n');
                    
                    // Separator
                    let separator = columns
                        .iter()
                        .map(|col| "-".repeat(col.len()))
                        .collect::<Vec<_>>()
                        .join("-+-");
                    result.push_str(&separator);
                    result.push('\n');
                    
                    // Rows
                    for row in rows {
                        let row_str = row
                            .values
                            .iter()
                            .map(|v| format!("{}", v))
                            .collect::<Vec<_>>()
                            .join(" | ");
                        result.push_str(&row_str);
                        result.push('\n');
                    }
                    
                    result.push_str(&format!("\n{} row(s) returned", rows.len()));
                    result
                }
            }
            QueryResult::Insert { rows_affected } => {
                format!("{} row(s) inserted", rows_affected)
            }
            QueryResult::Update { rows_affected } => {
                format!("{} row(s) updated", rows_affected)
            }
            QueryResult::Delete { rows_affected } => {
                format!("{} row(s) deleted", rows_affected)
            }
            QueryResult::CreateTable => "Table created successfully".to_string(),
            QueryResult::DropTable => "Table dropped successfully".to_string(),
            QueryResult::CreateIndex => "Index created successfully".to_string(),
            QueryResult::DropIndex => "Index dropped successfully".to_string(),
        }
    }

    /// Convert SELECT result to CSV format
    pub fn to_csv(&self) -> Option<String> {
        match self {
            QueryResult::Select { columns, rows } => {
                let mut csv = String::new();
                
                // Header
                csv.push_str(&columns.join(","));
                csv.push('\n');
                
                // Rows
                for row in rows {
                    let row_csv = row
                        .values
                        .iter()
                        .map(|v| match v {
                            Value::Text(s) => format!("\"{}\"", s.replace("\"", "\"\"")),
                            other => format!("{}", other),
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    csv.push_str(&row_csv);
                    csv.push('\n');
                }
                
                Some(csv)
            }
            _ => None,
        }
    }

    /// Convert SELECT result to JSON format
    pub fn to_json(&self) -> Option<String> {
        match self {
            QueryResult::Select { columns, rows } => {
                let json_rows: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|row| {
                        let mut obj = serde_json::Map::new();
                        for (i, column) in columns.iter().enumerate() {
                            if let Some(value) = row.values.get(i) {
                                let json_value = match value {
                                    Value::Null => serde_json::Value::Null,
                                    Value::Integer(i) => serde_json::Value::Number((*i).into()),
                                    Value::Real(f) => serde_json::Value::Number(
                                        serde_json::Number::from_f64(*f).unwrap_or_else(|| 0.into())
                                    ),
                                    Value::Text(s) => serde_json::Value::String(s.clone()),
                                    Value::Boolean(b) => serde_json::Value::Bool(*b),
                                    Value::Blob(_) => serde_json::Value::String("[BLOB]".to_string()),
                                };
                                obj.insert(column.clone(), json_value);
                            }
                        }
                        serde_json::Value::Object(obj)
                    })
                    .collect();
                
                serde_json::to_string_pretty(&json_rows).ok()
            }
            _ => None,
        }
    }

    /// Merge two SELECT results (union operation)
    pub fn merge_select(self, other: QueryResult) -> Result<QueryResult, String> {
        match (self, other) {
            (
                QueryResult::Select { columns: cols1, rows: rows1 },
                QueryResult::Select { columns: cols2, rows: rows2 },
            ) => {
                if cols1 != cols2 {
                    return Err("Column schemas do not match".to_string());
                }
                
                let mut merged_rows = rows1;
                merged_rows.extend(rows2);
                
                Ok(QueryResult::Select {
                    columns: cols1,
                    rows: merged_rows,
                })
            }
            _ => Err("Can only merge SELECT results".to_string()),
        }
    }

    /// Filter SELECT result rows based on a predicate
    pub fn filter_rows<F>(self, predicate: F) -> QueryResult
    where
        F: Fn(&Row) -> bool,
    {
        match self {
            QueryResult::Select { columns, rows } => {
                let filtered_rows: Vec<Row> = rows.into_iter().filter(predicate).collect();
                QueryResult::Select {
                    columns,
                    rows: filtered_rows,
                }
            }
            other => other,
        }
    }

    /// Transform SELECT result rows
    pub fn map_rows<F>(self, transform: F) -> QueryResult
    where
        F: Fn(Row) -> Row,
    {
        match self {
            QueryResult::Select { columns, rows } => {
                let transformed_rows: Vec<Row> = rows.into_iter().map(transform).collect();
                QueryResult::Select {
                    columns,
                    rows: transformed_rows,
                }
            }
            other => other,
        }
    }

    /// Take only the first N rows from a SELECT result
    pub fn take(self, n: usize) -> QueryResult {
        match self {
            QueryResult::Select { columns, mut rows } => {
                rows.truncate(n);
                QueryResult::Select { columns, rows }
            }
            other => other,
        }
    }

    /// Skip the first N rows from a SELECT result
    pub fn skip(self, n: usize) -> QueryResult {
        match self {
            QueryResult::Select { columns, rows } => {
                let remaining_rows = if n < rows.len() {
                    rows.into_iter().skip(n).collect()
                } else {
                    Vec::new()
                };
                QueryResult::Select {
                    columns,
                    rows: remaining_rows,
                }
            }
            other => other,
        }
    }
}

impl std::fmt::Display for QueryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_formatted_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    #[test]
    fn test_select_result() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Bob".to_string())]),
        ];
        
        let result = QueryResult::select(columns.clone(), rows.clone());
        
        assert!(result.is_select());
        assert!(!result.is_dml());
        assert!(!result.is_ddl());
        assert_eq!(result.row_count(), Some(2));
        assert_eq!(result.columns(), Some(&columns));
        assert_eq!(result.rows(), Some(&rows));
    }

    #[test]
    fn test_insert_result() {
        let result = QueryResult::insert(5);
        
        assert!(!result.is_select());
        assert!(result.is_dml());
        assert!(!result.is_ddl());
        assert_eq!(result.rows_affected(), Some(5));
    }

    #[test]
    fn test_result_formatting() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
        ];
        
        let result = QueryResult::select(columns, rows);
        let formatted = result.to_formatted_string();
        
        assert!(formatted.contains("id | name"));
        assert!(formatted.contains("1 | Alice"));
        assert!(formatted.contains("1 row(s) returned"));
    }

    #[test]
    fn test_csv_conversion() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Bob".to_string())]),
        ];
        
        let result = QueryResult::select(columns, rows);
        let csv = result.to_csv().unwrap();
        
        assert!(csv.contains("id,name"));
        assert!(csv.contains("1,\"Alice\""));
        assert!(csv.contains("2,\"Bob\""));
    }

    #[test]
    fn test_result_filtering() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Bob".to_string())]),
            Row::new(vec![Value::Integer(3), Value::Text("Charlie".to_string())]),
        ];
        
        let result = QueryResult::select(columns, rows);
        let filtered = result.filter_rows(|row| {
            if let Some(Value::Integer(id)) = row.values.get(0) {
                *id > 1
            } else {
                false
            }
        });
        
        assert_eq!(filtered.row_count(), Some(2));
    }
}
