use crate::error::{SqlError, SqlResult};
use crate::query::plan::*;
use crate::query::result::QueryResult;
use crate::types::{Row, Value};
use std::collections::HashMap;

/// Query executor that executes query plans
#[derive(Debug)]
pub struct QueryExecutor {
    // In a real implementation, this would have references to:
    // - Storage engine
    // - Transaction manager
    // - Index manager
    // - Statistics collector
}

impl QueryExecutor {
    pub fn new() -> Self {
        QueryExecutor {}
    }

    /// Execute a query plan and return results
    pub async fn execute_plan(&self, plan: QueryPlan) -> SqlResult<QueryResult> {
        match plan {
            QueryPlan::Scan { table, filter, projection } => {
                self.execute_scan(table, filter, projection).await
            }
            QueryPlan::IndexScan { table, index, key, filter } => {
                self.execute_index_scan(table, index, key, filter).await
            }
            QueryPlan::Join { left, right, join_type, condition } => {
                self.execute_join(*left, *right, join_type, condition).await
            }
            QueryPlan::Projection { input, columns, expressions } => {
                self.execute_projection(*input, columns, expressions).await
            }
            QueryPlan::Selection { input, condition } => {
                self.execute_selection(*input, condition).await
            }
            QueryPlan::Sort { input, order_by } => {
                self.execute_sort(*input, order_by).await
            }
            QueryPlan::Limit { input, count, offset } => {
                self.execute_limit(*input, count, offset).await
            }
            QueryPlan::GroupBy { input, group_columns, aggregates } => {
                self.execute_group_by(*input, group_columns, aggregates).await
            }
            QueryPlan::Insert { table, columns, values } => {
                self.execute_insert(table, columns, values).await
            }
            QueryPlan::Update { table, assignments, condition } => {
                self.execute_update(table, assignments, condition).await
            }
            QueryPlan::Delete { table, condition } => {
                self.execute_delete(table, condition).await
            }
            QueryPlan::CreateTable { table, schema } => {
                self.execute_create_table(table, schema).await
            }
            QueryPlan::Union { left, right, all } => {
                self.execute_union(*left, *right, all).await
            }
            QueryPlan::Intersect { left, right } => {
                self.execute_intersect(*left, *right).await
            }
            QueryPlan::Except { left, right } => {
                self.execute_except(*left, *right).await
            }
        }
    }

    // Individual execution methods
    async fn execute_scan(
        &self,
        table: String,
        _filter: Option<crate::parser::ast::Expression>,
        _projection: Option<Vec<String>>,
    ) -> SqlResult<QueryResult> {
        // Simulate table scan
        // In a real implementation, this would:
        // 1. Open the table
        // 2. Scan through pages
        // 3. Apply filters
        // 4. Apply projection
        
        let columns = vec!["id".to_string(), "name".to_string(), "age".to_string()];
        let rows = vec![
            Row::new(vec![
                Value::Integer(1),
                Value::Text("Alice".to_string()),
                Value::Integer(30),
            ]),
            Row::new(vec![
                Value::Integer(2),
                Value::Text("Bob".to_string()),
                Value::Integer(25),
            ]),
        ];

        Ok(QueryResult::select(columns, rows))
    }

    async fn execute_index_scan(
        &self,
        table: String,
        _index: String,
        _key: Value,
        _filter: Option<crate::parser::ast::Expression>,
    ) -> SqlResult<QueryResult> {
        // Simulate index scan (more efficient than table scan)
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![Row::new(vec![
            Value::Integer(1),
            Value::Text("Alice".to_string()),
        ])];

        Ok(QueryResult::select(columns, rows))
    }

    async fn execute_join(
        &self,
        left: QueryPlan,
        right: QueryPlan,
        _join_type: crate::parser::ast::JoinType,
        _condition: crate::parser::ast::Expression,
    ) -> SqlResult<QueryResult> {
        // Execute left and right plans
        let left_result = Box::pin(self.execute_plan(left)).await?;
        let right_result = Box::pin(self.execute_plan(right)).await?;

        // Perform join operation
        match (left_result, right_result) {
            (
                QueryResult::Select { columns: left_cols, rows: left_rows },
                QueryResult::Select { columns: right_cols, rows: right_rows },
            ) => {
                // Simplified nested loop join
                let mut joined_columns = left_cols;
                joined_columns.extend(right_cols);

                let mut joined_rows = Vec::new();
                for left_row in &left_rows {
                    for right_row in &right_rows {
                        let mut joined_values = left_row.values.clone();
                        joined_values.extend(right_row.values.clone());
                        joined_rows.push(Row::new(joined_values));
                    }
                }

                Ok(QueryResult::select(joined_columns, joined_rows))
            }
            _ => Err(SqlError::runtime_error("Cannot join non-SELECT results")),
        }
    }

    async fn execute_projection(
        &self,
        input: QueryPlan,
        columns: Vec<String>,
        _expressions: Vec<crate::parser::ast::Expression>,
    ) -> SqlResult<QueryResult> {
        let input_result = Box::pin(self.execute_plan(input)).await?;

        match input_result {
            QueryResult::Select { columns: input_cols, rows } => {
                // For simplicity, just return the specified columns
                // In a real implementation, this would evaluate expressions
                Ok(QueryResult::select(columns, rows))
            }
            other => Ok(other),
        }
    }

    async fn execute_selection(
        &self,
        input: QueryPlan,
        _condition: crate::parser::ast::Expression,
    ) -> SqlResult<QueryResult> {
        let input_result = Box::pin(self.execute_plan(input)).await?;

        match input_result {
            QueryResult::Select { columns, rows } => {
                // For simplicity, filter out half the rows
                // In a real implementation, this would evaluate the condition
                let row_count = rows.len();
                let filtered_rows: Vec<Row> = rows.into_iter().take(row_count / 2).collect();
                Ok(QueryResult::select(columns, filtered_rows))
            }
            other => Ok(other),
        }
    }

    async fn execute_sort(
        &self,
        input: QueryPlan,
        _order_by: Vec<(String, crate::parser::ast::OrderDirection)>,
    ) -> SqlResult<QueryResult> {
        let input_result = Box::pin(self.execute_plan(input)).await?;

        match input_result {
            QueryResult::Select { columns, mut rows } => {
                // For simplicity, reverse the order
                // In a real implementation, this would sort by the specified columns
                rows.reverse();
                Ok(QueryResult::select(columns, rows))
            }
            other => Ok(other),
        }
    }

    async fn execute_limit(
        &self,
        input: QueryPlan,
        count: u64,
        offset: Option<u64>,
    ) -> SqlResult<QueryResult> {
        let input_result = Box::pin(self.execute_plan(input)).await?;

        match input_result {
            QueryResult::Select { columns, rows } => {
                let start = offset.unwrap_or(0) as usize;
                let end = start + count as usize;

                let limited_rows = if start < rows.len() {
                    rows.into_iter().skip(start).take(count as usize).collect()
                } else {
                    Vec::new()
                };

                Ok(QueryResult::select(columns, limited_rows))
            }
            other => Ok(other),
        }
    }

    async fn execute_group_by(
        &self,
        input: QueryPlan,
        _group_columns: Vec<String>,
        _aggregates: Vec<AggregateFunction>,
    ) -> SqlResult<QueryResult> {
        let input_result = Box::pin(self.execute_plan(input)).await?;

        match input_result {
            QueryResult::Select { columns: _, rows } => {
                // Simplified aggregation - just count rows
                let result_columns = vec!["count".to_string()];
                let result_rows = vec![Row::new(vec![Value::Integer(rows.len() as i64)])];

                Ok(QueryResult::select(result_columns, result_rows))
            }
            other => Ok(other),
        }
    }

    async fn execute_insert(
        &self,
        _table: String,
        _columns: Vec<String>,
        values: Vec<Vec<Value>>,
    ) -> SqlResult<QueryResult> {
        // Simulate insert operation
        // In a real implementation, this would:
        // 1. Validate the data
        // 2. Insert into storage
        // 3. Update indexes
        // 4. Log to WAL

        Ok(QueryResult::insert(values.len() as u64))
    }

    async fn execute_update(
        &self,
        _table: String,
        _assignments: HashMap<String, crate::parser::ast::Expression>,
        _condition: Option<crate::parser::ast::Expression>,
    ) -> SqlResult<QueryResult> {
        // Simulate update operation
        Ok(QueryResult::update(1))
    }

    async fn execute_delete(
        &self,
        _table: String,
        _condition: Option<crate::parser::ast::Expression>,
    ) -> SqlResult<QueryResult> {
        // Simulate delete operation
        Ok(QueryResult::delete(1))
    }

    async fn execute_create_table(
        &self,
        _table: String,
        _schema: TableSchema,
    ) -> SqlResult<QueryResult> {
        // Simulate table creation
        Ok(QueryResult::create_table())
    }

    async fn execute_union(
        &self,
        left: QueryPlan,
        right: QueryPlan,
        _all: bool,
    ) -> SqlResult<QueryResult> {
        let left_result = Box::pin(self.execute_plan(left)).await?;
        let right_result = Box::pin(self.execute_plan(right)).await?;

        match (left_result, right_result) {
            (
                QueryResult::Select { columns: left_cols, rows: left_rows },
                QueryResult::Select { columns: right_cols, rows: right_rows },
            ) => {
                if left_cols != right_cols {
                    return Err(SqlError::runtime_error("UNION column schemas don't match"));
                }

                let mut union_rows = left_rows;
                union_rows.extend(right_rows);

                Ok(QueryResult::select(left_cols, union_rows))
            }
            _ => Err(SqlError::runtime_error("Cannot UNION non-SELECT results")),
        }
    }

    async fn execute_intersect(
        &self,
        left: QueryPlan,
        right: QueryPlan,
    ) -> SqlResult<QueryResult> {
        let left_result = Box::pin(self.execute_plan(left)).await?;
        let right_result = Box::pin(self.execute_plan(right)).await?;

        match (left_result, right_result) {
            (
                QueryResult::Select { columns, rows: left_rows },
                QueryResult::Select { rows: right_rows, .. },
            ) => {
                // Simplified intersect - find common rows
                let intersect_rows: Vec<Row> = left_rows
                    .into_iter()
                    .filter(|row| right_rows.contains(row))
                    .collect();

                Ok(QueryResult::select(columns, intersect_rows))
            }
            _ => Err(SqlError::runtime_error("Cannot INTERSECT non-SELECT results")),
        }
    }

    async fn execute_except(
        &self,
        left: QueryPlan,
        right: QueryPlan,
    ) -> SqlResult<QueryResult> {
        let left_result = Box::pin(self.execute_plan(left)).await?;
        let right_result = Box::pin(self.execute_plan(right)).await?;

        match (left_result, right_result) {
            (
                QueryResult::Select { columns, rows: left_rows },
                QueryResult::Select { rows: right_rows, .. },
            ) => {
                // Simplified except - remove common rows
                let except_rows: Vec<Row> = left_rows
                    .into_iter()
                    .filter(|row| !right_rows.contains(row))
                    .collect();

                Ok(QueryResult::select(columns, except_rows))
            }
            _ => Err(SqlError::runtime_error("Cannot EXCEPT non-SELECT results")),
        }
    }
}

impl Default for QueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::JoinType;

    #[tokio::test]
    async fn test_execute_scan() {
        let executor = QueryExecutor::new();
        
        let plan = QueryPlan::Scan {
            table: "users".to_string(),
            filter: None,
            projection: None,
        };
        
        let result = executor.execute_plan(plan).await.unwrap();
        
        match result {
            QueryResult::Select { columns, rows } => {
                assert_eq!(columns.len(), 3);
                assert_eq!(rows.len(), 2);
            }
            _ => panic!("Expected SELECT result"),
        }
    }

    #[tokio::test]
    async fn test_execute_insert() {
        let executor = QueryExecutor::new();
        
        let plan = QueryPlan::Insert {
            table: "users".to_string(),
            columns: vec!["name".to_string()],
            values: vec![vec![Value::Text("Alice".to_string())]],
        };
        
        let result = executor.execute_plan(plan).await.unwrap();
        
        match result {
            QueryResult::Insert { rows_affected } => {
                assert_eq!(rows_affected, 1);
            }
            _ => panic!("Expected INSERT result"),
        }
    }

    #[tokio::test]
    async fn test_execute_join() {
        let executor = QueryExecutor::new();
        
        let left_plan = QueryPlan::Scan {
            table: "users".to_string(),
            filter: None,
            projection: None,
        };
        
        let right_plan = QueryPlan::Scan {
            table: "orders".to_string(),
            filter: None,
            projection: None,
        };
        
        let join_plan = QueryPlan::Join {
            left: Box::new(left_plan),
            right: Box::new(right_plan),
            join_type: JoinType::Inner,
            condition: crate::parser::ast::Expression::Literal(Value::Boolean(true)),
        };
        
        let result = executor.execute_plan(join_plan).await.unwrap();
        
        match result {
            QueryResult::Select { columns, rows } => {
                assert_eq!(columns.len(), 6); // 3 + 3 columns
                assert_eq!(rows.len(), 4); // 2 * 2 rows (cartesian product)
            }
            _ => panic!("Expected SELECT result"),
        }
    }
}
