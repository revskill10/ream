use crate::error::{SqlError, SqlResult};
use crate::parser::ast::*;
use crate::query::plan::*;
use crate::types::{Statistics, Value};
use std::collections::HashMap;

/// Query planner following coalgebraic optimization patterns
#[derive(Debug)]
pub struct QueryPlanner {
    coalgebra: QueryPlannerCoalgebra,
}

impl QueryPlanner {
    pub fn new() -> Self {
        QueryPlanner {
            coalgebra: QueryPlannerCoalgebra::new(),
        }
    }

    /// Plan a SQL statement into an execution plan
    pub async fn plan_statement(&self, statement: Statement) -> SqlResult<QueryPlan> {
        match statement {
            Statement::Select(select) => self.plan_select(select).await,
            Statement::Insert(insert) => self.plan_insert(insert).await,
            Statement::Update(update) => self.plan_update(update).await,
            Statement::Delete(delete) => self.plan_delete(delete).await,
            Statement::CreateTable(create) => self.plan_create_table(create).await,
            Statement::DropTable(drop) => self.plan_drop_table(drop).await,
            Statement::CreateIndex(create) => self.plan_create_index(create).await,
            Statement::DropIndex(drop) => self.plan_drop_index(drop).await,
        }
    }

    /// Optimize a query plan using coalgebraic strategies
    pub async fn optimize_plan(&self, plan: QueryPlan) -> SqlResult<QueryPlan> {
        self.coalgebra.optimize_plan(plan).await
    }

    /// Get query statistics
    pub fn get_statistics(&self) -> &Statistics {
        &self.coalgebra.statistics
    }

    /// Update statistics
    pub fn update_statistics(&mut self, stats: Statistics) {
        self.coalgebra.statistics = stats;
    }

    // Private planning methods
    async fn plan_select(&self, select: SelectStatement) -> SqlResult<QueryPlan> {
        let mut plan = if let Some(from) = select.from {
            // Start with table scan or index scan
            let base_plan = if self.should_use_index(&from.table, &select.where_clause) {
                QueryPlan::IndexScan {
                    table: from.table.clone(),
                    index: "primary".to_string(), // Simplified
                    key: Value::Null,
                    filter: select.where_clause.clone(),
                }
            } else {
                QueryPlan::Scan {
                    table: from.table.clone(),
                    filter: select.where_clause.clone(),
                    projection: None,
                }
            };

            // Add joins
            let mut current_plan = base_plan;
            for join in from.joins {
                current_plan = QueryPlan::Join {
                    left: Box::new(current_plan),
                    right: Box::new(QueryPlan::Scan {
                        table: join.table,
                        filter: None,
                        projection: None,
                    }),
                    join_type: join.join_type,
                    condition: join.condition,
                };
            }

            current_plan
        } else {
            // SELECT without FROM (e.g., SELECT 1)
            return Err(SqlError::parse_error("SELECT without FROM not supported"));
        };

        // Add projection
        if !matches!(select.columns.as_slice(), [SelectColumn::Wildcard]) {
            let (columns, expressions) = self.extract_projection_info(&select.columns)?;
            plan = QueryPlan::Projection {
                input: Box::new(plan),
                columns,
                expressions,
            };
        }

        // Add GROUP BY
        if let Some(group_columns) = select.group_by {
            plan = QueryPlan::GroupBy {
                input: Box::new(plan),
                group_columns: group_columns.into_iter().map(|_| "col".to_string()).collect(),
                aggregates: vec![], // Simplified
            };
        }

        // Add HAVING
        if let Some(having) = select.having {
            plan = QueryPlan::Selection {
                input: Box::new(plan),
                condition: having,
            };
        }

        // Add ORDER BY
        if let Some(order_by) = select.order_by {
            let order_spec: Vec<(String, OrderDirection)> = order_by
                .into_iter()
                .map(|clause| ("col".to_string(), clause.direction))
                .collect();
            plan = QueryPlan::Sort {
                input: Box::new(plan),
                order_by: order_spec,
            };
        }

        // Add LIMIT
        if let Some(limit) = select.limit {
            plan = QueryPlan::Limit {
                input: Box::new(plan),
                count: limit.count,
                offset: limit.offset,
            };
        }

        Ok(plan)
    }

    async fn plan_insert(&self, insert: InsertStatement) -> SqlResult<QueryPlan> {
        let values: Vec<Vec<Value>> = insert
            .values
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|expr| self.evaluate_literal_expression(expr))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(QueryPlan::Insert {
            table: insert.table,
            columns: insert.columns.unwrap_or_default(),
            values,
        })
    }

    async fn plan_update(&self, update: UpdateStatement) -> SqlResult<QueryPlan> {
        let assignments: HashMap<String, Expression> = update
            .assignments
            .into_iter()
            .map(|assignment| (assignment.column, assignment.value))
            .collect();

        Ok(QueryPlan::Update {
            table: update.table,
            assignments,
            condition: update.where_clause,
        })
    }

    async fn plan_delete(&self, delete: DeleteStatement) -> SqlResult<QueryPlan> {
        Ok(QueryPlan::Delete {
            table: delete.table,
            condition: delete.where_clause,
        })
    }

    async fn plan_create_table(&self, create: CreateTableStatement) -> SqlResult<QueryPlan> {
        let schema = TableSchema {
            columns: create
                .columns
                .into_iter()
                .map(|col| ColumnSchema {
                    name: col.name,
                    data_type: col.data_type,
                    nullable: !col.constraints.contains(&ColumnConstraint::NotNull),
                    default: None,
                    primary_key: col.constraints.contains(&ColumnConstraint::PrimaryKey),
                    unique: col.constraints.contains(&ColumnConstraint::Unique),
                })
                .collect(),
            constraints: vec![], // Simplified
        };

        Ok(QueryPlan::CreateTable {
            table: create.table_name,
            schema,
        })
    }

    async fn plan_drop_table(&self, _drop: DropTableStatement) -> SqlResult<QueryPlan> {
        // Simplified implementation
        Err(SqlError::runtime_error("DROP TABLE not implemented"))
    }

    async fn plan_create_index(&self, _create: CreateIndexStatement) -> SqlResult<QueryPlan> {
        // Simplified implementation
        Err(SqlError::runtime_error("CREATE INDEX not implemented"))
    }

    async fn plan_drop_index(&self, _drop: DropIndexStatement) -> SqlResult<QueryPlan> {
        // Simplified implementation
        Err(SqlError::runtime_error("DROP INDEX not implemented"))
    }

    // Helper methods
    fn should_use_index(&self, _table: &str, _where_clause: &Option<Expression>) -> bool {
        // Simplified index selection logic
        false
    }

    fn extract_projection_info(
        &self,
        columns: &[SelectColumn],
    ) -> SqlResult<(Vec<String>, Vec<Expression>)> {
        let mut column_names = Vec::new();
        let mut expressions = Vec::new();

        for column in columns {
            match column {
                SelectColumn::Wildcard => {
                    return Err(SqlError::runtime_error("Wildcard in projection extraction"));
                }
                SelectColumn::Expression { expr, alias } => {
                    let name = alias
                        .clone()
                        .unwrap_or_else(|| format!("col_{}", column_names.len()));
                    column_names.push(name);
                    expressions.push(expr.clone());
                }
            }
        }

        Ok((column_names, expressions))
    }

    fn evaluate_literal_expression(&self, expr: Expression) -> SqlResult<Value> {
        match expr {
            Expression::Literal(value) => Ok(value),
            _ => Err(SqlError::runtime_error(
                "Only literal expressions supported in INSERT",
            )),
        }
    }
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Query planner coalgebra for optimization strategies
#[derive(Debug)]
pub struct QueryPlannerCoalgebra {
    pub statistics: Statistics,
    cost_function: fn(&QueryPlan, &Statistics) -> f64,
}

impl QueryPlannerCoalgebra {
    pub fn new() -> Self {
        QueryPlannerCoalgebra {
            statistics: Statistics::empty(),
            cost_function: Self::default_cost_function,
        }
    }

    /// Coalgebraic optimization: evolve plan based on strategy
    pub async fn optimize_plan(&self, plan: QueryPlan) -> SqlResult<QueryPlan> {
        let cost = (self.cost_function)(&plan, &self.statistics);

        match plan {
            QueryPlan::Scan { table, filter, projection } => {
                // Check if index scan is better
                if let Some(index_cost) = self.check_index_scan(&table, filter.as_ref()) {
                    if index_cost < cost {
                        return Ok(QueryPlan::IndexScan {
                            table,
                            index: "idx_primary".to_string(),
                            key: Value::Null,
                            filter,
                        });
                    }
                }
                Ok(QueryPlan::Scan { table, filter, projection })
            }
            QueryPlan::Join { left, right, join_type, condition } => {
                // Choose optimal join strategy
                let optimized_left = Box::new(Box::pin(self.optimize_plan(*left)).await?);
                let optimized_right = Box::new(Box::pin(self.optimize_plan(*right)).await?);

                let _strategy = self.choose_join_strategy(
                    &optimized_left,
                    &optimized_right,
                    &condition,
                    join_type,
                );

                Ok(QueryPlan::Join {
                    left: optimized_left,
                    right: optimized_right,
                    join_type,
                    condition,
                })
            }
            QueryPlan::Projection { input, columns, expressions } => {
                let optimized_input = Box::new(Box::pin(self.optimize_plan(*input)).await?);
                Ok(QueryPlan::Projection {
                    input: optimized_input,
                    columns,
                    expressions,
                })
            }
            QueryPlan::Selection { input, condition } => {
                let optimized_input = Box::new(Box::pin(self.optimize_plan(*input)).await?);
                Ok(QueryPlan::Selection {
                    input: optimized_input,
                    condition,
                })
            }
            QueryPlan::Sort { input, order_by } => {
                let optimized_input = Box::new(Box::pin(self.optimize_plan(*input)).await?);
                Ok(QueryPlan::Sort {
                    input: optimized_input,
                    order_by,
                })
            }
            QueryPlan::Limit { input, count, offset } => {
                let optimized_input = Box::new(Box::pin(self.optimize_plan(*input)).await?);
                Ok(QueryPlan::Limit {
                    input: optimized_input,
                    count,
                    offset,
                })
            }
            other => Ok(other),
        }
    }

    fn default_cost_function(plan: &QueryPlan, _statistics: &Statistics) -> f64 {
        plan.estimated_cost()
    }

    fn check_index_scan(&self, _table: &str, _filter: Option<&Expression>) -> Option<f64> {
        // Simplified index cost estimation
        Some(50.0)
    }

    fn choose_join_strategy(
        &self,
        _left: &QueryPlan,
        _right: &QueryPlan,
        _condition: &Expression,
        _join_type: JoinType,
    ) -> JoinStrategy {
        // Simplified join strategy selection
        JoinStrategy::NestedLoop
    }
}

impl Default for QueryPlannerCoalgebra {
    fn default() -> Self {
        Self::new()
    }
}

/// Join strategy options
#[derive(Debug, Clone, Copy)]
pub enum JoinStrategy {
    NestedLoop,
    HashJoin,
    MergeJoin,
    IndexJoin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plan_simple_select() {
        let planner = QueryPlanner::new();
        
        let select = SelectStatement {
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
        };
        
        let plan = planner.plan_select(select).await.unwrap();
        
        match plan {
            QueryPlan::Scan { table, .. } => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected scan plan"),
        }
    }

    #[tokio::test]
    async fn test_plan_insert() {
        let planner = QueryPlanner::new();
        
        let insert = InsertStatement {
            table: "users".to_string(),
            columns: Some(vec!["name".to_string()]),
            values: vec![vec![Expression::Literal(Value::Text("Alice".to_string()))]],
        };
        
        let plan = planner.plan_insert(insert).await.unwrap();
        
        match plan {
            QueryPlan::Insert { table, values, .. } => {
                assert_eq!(table, "users");
                assert_eq!(values.len(), 1);
            }
            _ => panic!("Expected insert plan"),
        }
    }
}
