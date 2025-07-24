use crate::parser::ast::{Expression, JoinType, OrderDirection};
use crate::types::{Row, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query execution plan following composite pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryPlan {
    /// Table scan operation
    Scan {
        table: String,
        filter: Option<Expression>,
        projection: Option<Vec<String>>,
    },
    /// Index scan operation
    IndexScan {
        table: String,
        index: String,
        key: Value,
        filter: Option<Expression>,
    },
    /// Join operation
    Join {
        left: Box<QueryPlan>,
        right: Box<QueryPlan>,
        join_type: JoinType,
        condition: Expression,
    },
    /// Projection operation
    Projection {
        input: Box<QueryPlan>,
        columns: Vec<String>,
        expressions: Vec<Expression>,
    },
    /// Selection (filter) operation
    Selection {
        input: Box<QueryPlan>,
        condition: Expression,
    },
    /// Sort operation
    Sort {
        input: Box<QueryPlan>,
        order_by: Vec<(String, OrderDirection)>,
    },
    /// Limit operation
    Limit {
        input: Box<QueryPlan>,
        count: u64,
        offset: Option<u64>,
    },
    /// Group by operation
    GroupBy {
        input: Box<QueryPlan>,
        group_columns: Vec<String>,
        aggregates: Vec<AggregateFunction>,
    },
    /// Insert operation
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<Value>>,
    },
    /// Update operation
    Update {
        table: String,
        assignments: HashMap<String, Expression>,
        condition: Option<Expression>,
    },
    /// Delete operation
    Delete {
        table: String,
        condition: Option<Expression>,
    },
    /// Create table operation
    CreateTable {
        table: String,
        schema: TableSchema,
    },
    /// Union operation
    Union {
        left: Box<QueryPlan>,
        right: Box<QueryPlan>,
        all: bool,
    },
    /// Intersect operation
    Intersect {
        left: Box<QueryPlan>,
        right: Box<QueryPlan>,
    },
    /// Except operation
    Except {
        left: Box<QueryPlan>,
        right: Box<QueryPlan>,
    },
}

/// Aggregate function types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count { column: Option<String> },
    Sum { column: String },
    Avg { column: String },
    Min { column: String },
    Max { column: String },
}

/// Table schema for CREATE TABLE operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub columns: Vec<ColumnSchema>,
    pub constraints: Vec<TableConstraint>,
}

/// Column schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: crate::types::DataType,
    pub nullable: bool,
    pub default: Option<Value>,
    pub primary_key: bool,
    pub unique: bool,
}

/// Table constraint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TableConstraint {
    PrimaryKey(Vec<String>),
    Unique(Vec<String>),
    ForeignKey {
        columns: Vec<String>,
        foreign_table: String,
        foreign_columns: Vec<String>,
    },
    Check(Expression),
}

impl QueryPlan {
    /// Get estimated cost of executing this plan
    pub fn estimated_cost(&self) -> f64 {
        match self {
            QueryPlan::Scan { .. } => 100.0,
            QueryPlan::IndexScan { .. } => 10.0,
            QueryPlan::Join { left, right, .. } => {
                left.estimated_cost() * right.estimated_cost() * 0.1
            }
            QueryPlan::Projection { input, .. } => input.estimated_cost() * 1.1,
            QueryPlan::Selection { input, .. } => input.estimated_cost() * 1.2,
            QueryPlan::Sort { input, .. } => input.estimated_cost() * 2.0,
            QueryPlan::Limit { input, .. } => input.estimated_cost() * 0.5,
            QueryPlan::GroupBy { input, .. } => input.estimated_cost() * 1.5,
            QueryPlan::Insert { values, .. } => values.len() as f64 * 5.0,
            QueryPlan::Update { .. } => 50.0,
            QueryPlan::Delete { .. } => 30.0,
            QueryPlan::CreateTable { .. } => 20.0,
            QueryPlan::Union { left, right, .. } => left.estimated_cost() + right.estimated_cost(),
            QueryPlan::Intersect { left, right } => left.estimated_cost() + right.estimated_cost(),
            QueryPlan::Except { left, right } => left.estimated_cost() + right.estimated_cost(),
        }
    }

    /// Get estimated number of output rows
    pub fn estimated_rows(&self) -> u64 {
        match self {
            QueryPlan::Scan { .. } => 1000, // Default estimate
            QueryPlan::IndexScan { .. } => 100,
            QueryPlan::Join { left, right, .. } => {
                (left.estimated_rows() * right.estimated_rows()) / 10
            }
            QueryPlan::Projection { input, .. } => input.estimated_rows(),
            QueryPlan::Selection { input, .. } => input.estimated_rows() / 3,
            QueryPlan::Sort { input, .. } => input.estimated_rows(),
            QueryPlan::Limit { count, .. } => *count,
            QueryPlan::GroupBy { input, .. } => input.estimated_rows() / 10,
            QueryPlan::Insert { values, .. } => values.len() as u64,
            QueryPlan::Update { .. } => 1,
            QueryPlan::Delete { .. } => 1,
            QueryPlan::CreateTable { .. } => 0,
            QueryPlan::Union { left, right, .. } => left.estimated_rows() + right.estimated_rows(),
            QueryPlan::Intersect { left, right } => {
                std::cmp::min(left.estimated_rows(), right.estimated_rows())
            }
            QueryPlan::Except { left, .. } => left.estimated_rows() / 2,
        }
    }

    /// Get all table names referenced in this plan
    pub fn referenced_tables(&self) -> Vec<String> {
        let mut tables = Vec::new();
        self.collect_tables(&mut tables);
        tables.sort();
        tables.dedup();
        tables
    }

    /// Check if this plan is read-only
    pub fn is_read_only(&self) -> bool {
        match self {
            QueryPlan::Scan { .. }
            | QueryPlan::IndexScan { .. }
            | QueryPlan::Join { .. }
            | QueryPlan::Projection { .. }
            | QueryPlan::Selection { .. }
            | QueryPlan::Sort { .. }
            | QueryPlan::Limit { .. }
            | QueryPlan::GroupBy { .. }
            | QueryPlan::Union { .. }
            | QueryPlan::Intersect { .. }
            | QueryPlan::Except { .. } => true,
            QueryPlan::Insert { .. }
            | QueryPlan::Update { .. }
            | QueryPlan::Delete { .. }
            | QueryPlan::CreateTable { .. } => false,
        }
    }

    /// Check if this plan modifies data
    pub fn is_write_operation(&self) -> bool {
        !self.is_read_only()
    }

    /// Get the output schema (column names and types)
    pub fn output_schema(&self) -> Vec<(String, crate::types::DataType)> {
        match self {
            QueryPlan::Scan { table, projection, .. } => {
                if let Some(columns) = projection {
                    columns
                        .iter()
                        .map(|col| (col.clone(), crate::types::DataType::Text))
                        .collect()
                } else {
                    // Return all columns (would need schema lookup in real implementation)
                    vec![("*".to_string(), crate::types::DataType::Text)]
                }
            }
            QueryPlan::Projection { columns, .. } => columns
                .iter()
                .map(|col| (col.clone(), crate::types::DataType::Text))
                .collect(),
            QueryPlan::Join { left, right, .. } => {
                let mut schema = left.output_schema();
                schema.extend(right.output_schema());
                schema
            }
            _ => vec![("result".to_string(), crate::types::DataType::Integer)],
        }
    }

    /// Transform plan using a function (functor operation)
    pub fn map<F>(self, f: F) -> QueryPlan
    where
        F: Fn(QueryPlan) -> QueryPlan + Clone,
    {
        match self {
            QueryPlan::Join { left, right, join_type, condition } => {
                let new_left = Box::new(f((*left).clone()));
                let new_right = Box::new(f((*right).clone()));
                QueryPlan::Join {
                    left: new_left,
                    right: new_right,
                    join_type,
                    condition,
                }
            }
            QueryPlan::Projection { input, columns, expressions } => {
                let new_input = Box::new(f((*input).clone()));
                QueryPlan::Projection {
                    input: new_input,
                    columns,
                    expressions,
                }
            }
            QueryPlan::Selection { input, condition } => {
                let new_input = Box::new(f((*input).clone()));
                QueryPlan::Selection {
                    input: new_input,
                    condition,
                }
            }
            QueryPlan::Sort { input, order_by } => {
                let new_input = Box::new(f((*input).clone()));
                QueryPlan::Sort {
                    input: new_input,
                    order_by,
                }
            }
            QueryPlan::Limit { input, count, offset } => {
                let new_input = Box::new(f((*input).clone()));
                QueryPlan::Limit {
                    input: new_input,
                    count,
                    offset,
                }
            }
            QueryPlan::GroupBy { input, group_columns, aggregates } => {
                let new_input = Box::new(f((*input).clone()));
                QueryPlan::GroupBy {
                    input: new_input,
                    group_columns,
                    aggregates,
                }
            }
            QueryPlan::Union { left, right, all } => {
                let new_left = Box::new(f((*left).clone()));
                let new_right = Box::new(f((*right).clone()));
                QueryPlan::Union {
                    left: new_left,
                    right: new_right,
                    all,
                }
            }
            QueryPlan::Intersect { left, right } => {
                let new_left = Box::new(f((*left).clone()));
                let new_right = Box::new(f((*right).clone()));
                QueryPlan::Intersect {
                    left: new_left,
                    right: new_right,
                }
            }
            QueryPlan::Except { left, right } => {
                let new_left = Box::new(f((*left).clone()));
                let new_right = Box::new(f((*right).clone()));
                QueryPlan::Except {
                    left: new_left,
                    right: new_right,
                }
            }
            // Leaf nodes remain unchanged
            other => other,
        }
    }

    /// Compose this plan with another (monadic composition)
    pub fn compose_with(self, other: QueryPlan) -> QueryPlan {
        match other {
            QueryPlan::Projection { columns, expressions, .. } => QueryPlan::Projection {
                input: Box::new(self),
                columns,
                expressions,
            },
            QueryPlan::Selection { condition, .. } => QueryPlan::Selection {
                input: Box::new(self),
                condition,
            },
            QueryPlan::Sort { order_by, .. } => QueryPlan::Sort {
                input: Box::new(self),
                order_by,
            },
            QueryPlan::Limit { count, offset, .. } => QueryPlan::Limit {
                input: Box::new(self),
                count,
                offset,
            },
            _ => other, // For non-composable plans, return the other plan
        }
    }

    // Private helper method
    fn collect_tables(&self, tables: &mut Vec<String>) {
        match self {
            QueryPlan::Scan { table, .. } | QueryPlan::IndexScan { table, .. } => {
                tables.push(table.clone());
            }
            QueryPlan::Join { left, right, .. } => {
                left.collect_tables(tables);
                right.collect_tables(tables);
            }
            QueryPlan::Projection { input, .. }
            | QueryPlan::Selection { input, .. }
            | QueryPlan::Sort { input, .. }
            | QueryPlan::Limit { input, .. }
            | QueryPlan::GroupBy { input, .. } => {
                input.collect_tables(tables);
            }
            QueryPlan::Insert { table, .. }
            | QueryPlan::Update { table, .. }
            | QueryPlan::Delete { table, .. }
            | QueryPlan::CreateTable { table, .. } => {
                tables.push(table.clone());
            }
            QueryPlan::Union { left, right, .. }
            | QueryPlan::Intersect { left, right }
            | QueryPlan::Except { left, right } => {
                left.collect_tables(tables);
                right.collect_tables(tables);
            }
        }
    }
}
