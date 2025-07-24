pub mod planner;
pub mod executor;
pub mod plan;
pub mod result;

pub use planner::{QueryPlanner, QueryPlannerCoalgebra};
pub use executor::QueryExecutor;
pub use plan::*;
pub use result::QueryResult;

use crate::error::{SqlError, SqlResult};
use crate::parser::ast::Statement;
use crate::types::{Row, Statistics};

/// Query processing pipeline
#[derive(Debug)]
pub struct QueryProcessor {
    planner: QueryPlanner,
    executor: QueryExecutor,
}

impl QueryProcessor {
    pub fn new() -> Self {
        QueryProcessor {
            planner: QueryPlanner::new(),
            executor: QueryExecutor::new(),
        }
    }

    /// Process a SQL statement end-to-end
    pub async fn process_statement(&self, statement: Statement) -> SqlResult<QueryResult> {
        // Plan the query
        let plan = self.planner.plan_statement(statement).await?;
        
        // Execute the plan
        self.executor.execute_plan(plan).await
    }

    /// Optimize and execute a query plan
    pub async fn optimize_and_execute(&self, plan: QueryPlan) -> SqlResult<QueryResult> {
        // Optimize the plan
        let optimized_plan = self.planner.optimize_plan(plan).await?;
        
        // Execute the optimized plan
        self.executor.execute_plan(optimized_plan).await
    }

    /// Get query statistics
    pub fn get_statistics(&self) -> &Statistics {
        self.planner.get_statistics()
    }

    /// Update statistics
    pub fn update_statistics(&mut self, stats: Statistics) {
        self.planner.update_statistics(stats);
    }
}

impl Default for QueryProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;
    use crate::types::Value;

    #[tokio::test]
    async fn test_query_processor_select() {
        let processor = QueryProcessor::new();
        
        let statement = Statement::Select(SelectStatement {
            columns: vec![SelectColumn::Wildcard],
            from: Some(FromClause {
                table: "users".to_string(),
                alias: None,
                joins: vec![],
            }),
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
        });
        
        let result = processor.process_statement(statement).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_processor_insert() {
        let processor = QueryProcessor::new();
        
        let statement = Statement::Insert(InsertStatement {
            table: "users".to_string(),
            columns: Some(vec!["name".to_string(), "age".to_string()]),
            values: vec![vec![
                Expression::Literal(Value::Text("Alice".to_string())),
                Expression::Literal(Value::Integer(30)),
            ]],
        });
        
        let result = processor.process_statement(statement).await;
        assert!(result.is_ok());
    }
}
