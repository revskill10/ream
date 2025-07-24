/// Advanced query builder with support for complex SQL patterns
use std::collections::HashMap;
use crate::orm::advanced_sql::*;
use crate::orm::{SqlResult, SqlError};
use crate::orm::query::SelectQueryBuilder;
use crate::orm::schema::{Column, AliasedColumn, Table, AliasedTable, TypeSafeExpression};
use crate::orm::sql_composable::{SqlComposable, AdvancedSqlComposable};
use crate::sqlite::parser::ast::Expression;

/// Advanced query builder that supports complex SQL patterns
pub struct AdvancedQueryBuilder {
    ctes: Vec<CteDefinition>,
    windows: HashMap<String, WindowFunction>,
    case_expressions: HashMap<String, CaseExpression>,
    json_operations: HashMap<String, JsonOperation>,
    aggregations: HashMap<String, AdvancedAggregation>,
    full_text_searches: Vec<FullTextSearch>,
    nested_relations: Vec<NestedRelation>,
    query_hints: Option<QueryHints>,
    base_query: Option<SelectQueryBuilder>,
    plugin: Option<Box<dyn AdvancedSqlPlugin + Send + Sync>>,
}

impl AdvancedQueryBuilder {
    pub fn new() -> Self {
        Self {
            ctes: Vec::new(),
            windows: HashMap::new(),
            case_expressions: HashMap::new(),
            json_operations: HashMap::new(),
            aggregations: HashMap::new(),
            full_text_searches: Vec::new(),
            nested_relations: Vec::new(),
            query_hints: None,
            base_query: None,
            plugin: None,
        }
    }
    
    /// Set the database plugin for advanced SQL generation
    pub fn with_plugin(mut self, plugin: Box<dyn AdvancedSqlPlugin + Send + Sync>) -> Self {
        self.plugin = Some(plugin);
        self
    }
    
    /// Set the base query
    pub fn base_query(mut self, query: SelectQueryBuilder) -> Self {
        self.base_query = Some(query);
        self
    }
    
    /// Add a Common Table Expression
    pub fn with_cte(mut self, name: impl Into<String>, query: impl Into<String>) -> Self {
        self.ctes.push(CteDefinition {
            name: name.into(),
            columns: None,
            query: query.into(),
            recursive: false,
        });
        self
    }
    
    /// Add a recursive CTE
    pub fn with_recursive_cte(
        mut self, 
        name: impl Into<String>, 
        columns: Vec<String>,
        query: impl Into<String>
    ) -> Self {
        self.ctes.push(CteDefinition {
            name: name.into(),
            columns: Some(columns),
            query: query.into(),
            recursive: true,
        });
        self
    }
    
    /// Add a window function
    pub fn with_window(mut self, alias: impl Into<String>, window: WindowFunction) -> Self {
        self.windows.insert(alias.into(), window);
        self
    }
    
    /// Add a ROW_NUMBER() window function
    pub fn with_row_number(
        mut self, 
        alias: impl Into<String>,
        partition_by: Vec<String>,
        order_by: Vec<OrderByClause>
    ) -> Self {
        let window = WindowFunction {
            function: WindowFunctionType::RowNumber,
            partition_by,
            order_by,
            frame: None,
        };
        self.windows.insert(alias.into(), window);
        self
    }
    
    /// Add a RANK() window function
    pub fn with_rank(
        mut self,
        alias: impl Into<String>,
        partition_by: Vec<String>,
        order_by: Vec<OrderByClause>
    ) -> Self {
        let window = WindowFunction {
            function: WindowFunctionType::Rank,
            partition_by,
            order_by,
            frame: None,
        };
        self.windows.insert(alias.into(), window);
        self
    }
    
    /// Add a LAG window function
    pub fn with_lag(
        mut self,
        alias: impl Into<String>,
        offset: i32,
        default: Option<crate::sqlite::types::Value>,
        partition_by: Vec<String>,
        order_by: Vec<OrderByClause>
    ) -> Self {
        let window = WindowFunction {
            function: WindowFunctionType::Lag { offset, default },
            partition_by,
            order_by,
            frame: None,
        };
        self.windows.insert(alias.into(), window);
        self
    }
    
    /// Add a CASE expression
    pub fn with_case(mut self, alias: impl Into<String>, case_expr: CaseExpression) -> Self {
        self.case_expressions.insert(alias.into(), case_expr);
        self
    }
    
    /// Add a simple CASE expression
    pub fn with_simple_case(
        mut self,
        alias: impl Into<String>,
        expr: Expression,
        when_clauses: Vec<WhenClause>,
        else_clause: Option<Expression>
    ) -> Self {
        let case_expr = CaseExpression {
            case_type: CaseType::Simple(expr),
            when_clauses,
            else_clause,
        };
        self.case_expressions.insert(alias.into(), case_expr);
        self
    }
    
    /// Add a searched CASE expression
    pub fn with_searched_case(
        mut self,
        alias: impl Into<String>,
        when_clauses: Vec<WhenClause>,
        else_clause: Option<Expression>
    ) -> Self {
        let case_expr = CaseExpression {
            case_type: CaseType::Searched,
            when_clauses,
            else_clause,
        };
        self.case_expressions.insert(alias.into(), case_expr);
        self
    }
    
    /// Add a JSON operation
    pub fn with_json_extract(mut self, alias: impl Into<String>, path: impl Into<String>) -> Self {
        let json_op = JsonOperation {
            operation_type: JsonOperationType::Extract,
            path: path.into(),
            value: None,
        };
        self.json_operations.insert(alias.into(), json_op);
        self
    }
    
    /// Add a JSON set operation
    pub fn with_json_set(
        mut self,
        alias: impl Into<String>,
        path: impl Into<String>,
        value: crate::sqlite::types::Value
    ) -> Self {
        let json_op = JsonOperation {
            operation_type: JsonOperationType::Set,
            path: path.into(),
            value: Some(value),
        };
        self.json_operations.insert(alias.into(), json_op);
        self
    }

    /// Add a type-safe JSON extract operation using schema column
    pub fn with_json_extract_typed(mut self, alias: impl Into<String>, column: &crate::orm::schema::Column, path: impl Into<String>) -> Self {
        let path_str = path.into();

        // Validate JSON path against schema if available
        if column.has_json_schema() && !column.validate_json_path(&path_str) {
            // In a real implementation, this would return a Result
            // For now, we'll just proceed with a warning
            eprintln!("Warning: JSON path '{}' may not be valid for column '{}'", path_str, column.name);
        }

        let json_op = JsonOperation {
            operation_type: JsonOperationType::Extract,
            path: format!("$.{}", path_str),
            value: None,
        };
        self.json_operations.insert(alias.into(), json_op);
        self
    }

    /// Add a type-safe JSON set operation using schema column
    pub fn with_json_set_typed(mut self, alias: impl Into<String>, column: &crate::orm::schema::Column, path: impl Into<String>, value: crate::sqlite::types::Value) -> Self {
        let path_str = path.into();

        // Validate JSON path against schema if available
        if column.has_json_schema() && !column.validate_json_path(&path_str) {
            eprintln!("Warning: JSON path '{}' may not be valid for column '{}'", path_str, column.name);
        }

        let json_op = JsonOperation {
            operation_type: JsonOperationType::Set,
            path: format!("$.{}", path_str),
            value: Some(value),
        };
        self.json_operations.insert(alias.into(), json_op);
        self
    }

    /// Add a type-safe JSON array aggregation using schema column
    pub fn with_json_array_agg_typed(mut self, alias: impl Into<String>, column: &crate::orm::schema::Column, path: impl Into<String>) -> Self {
        let path_str = path.into();

        // For array aggregation, we'll use the column directly
        let json_op = JsonOperation {
            operation_type: JsonOperationType::Extract,
            path: format!("$.{}", path_str),
            value: None,
        };
        self.json_operations.insert(alias.into(), json_op);
        self
    }

    /// Add a LAG window function with type-safe column references
    pub fn with_lag_function(
        mut self,
        alias: impl Into<String>,
        column: &crate::orm::schema::Column,
        offset: i32,
        default_value: Option<crate::sqlite::types::Value>,
        partition_by: Vec<String>,
        order_by: Vec<OrderByClause>,
    ) -> Self {
        let window_func = WindowFunction {
            function: WindowFunctionType::Lag {
                offset,
                default: default_value,
            },
            partition_by,
            order_by,
            frame: None,
        };
        self.windows.insert(alias.into(), window_func);
        self
    }

    /// Add a CTE definition directly
    pub fn with_cte_definition(mut self, cte: CteDefinition) -> Self {
        self.ctes.push(cte);
        self
    }
    
    /// Add an advanced aggregation
    pub fn with_aggregation(mut self, alias: impl Into<String>, agg: AdvancedAggregation) -> Self {
        self.aggregations.insert(alias.into(), agg);
        self
    }
    
    /// Add a GROUP_CONCAT aggregation
    pub fn with_group_concat(
        mut self,
        alias: impl Into<String>,
        column: impl Into<String>,
        separator: Option<String>,
        distinct: bool
    ) -> Self {
        let agg = AdvancedAggregation {
            function: AggregateFunction::GroupConcat {
                column: column.into(),
                separator,
            },
            distinct,
            filter: None,
            over: None,
        };
        self.aggregations.insert(alias.into(), agg);
        self
    }
    
    /// Add a JSON array aggregation
    pub fn with_json_array_agg(
        mut self,
        alias: impl Into<String>,
        column: impl Into<String>,
        distinct: bool
    ) -> Self {
        let agg = AdvancedAggregation {
            function: AggregateFunction::JsonArrayAgg(column.into()),
            distinct,
            filter: None,
            over: None,
        };
        self.aggregations.insert(alias.into(), agg);
        self
    }
    
    /// Add full-text search
    pub fn with_full_text_search(mut self, fts: FullTextSearch) -> Self {
        self.full_text_searches.push(fts);
        self
    }
    
    /// Add a nested relation
    pub fn with_nested_relation(mut self, relation: NestedRelation) -> Self {
        self.nested_relations.push(relation);
        self
    }
    
    /// Add query hints
    pub fn with_hints(mut self, hints: QueryHints) -> Self {
        self.query_hints = Some(hints);
        self
    }
    
    /// Build the final SQL query
    pub fn build(self) -> SqlResult<String> {
        let plugin = self.plugin.as_ref()
            .ok_or_else(|| SqlError::runtime_error("No database plugin configured"))?;

        // Get the base query or create a default one
        let mut base_query = self.base_query
            .unwrap_or_else(|| {
                use crate::orm::query::QueryBuilder;
                QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            });

        // For now, we'll handle CTEs by generating them separately and prepending to the final SQL
        // This maintains compatibility with the existing CTE system while using the type-safe base query
        let mut cte_sql_parts = Vec::new();

        if !self.ctes.is_empty() {
            let mut has_recursive = false;

            for cte in &self.ctes {
                let cte_sql = plugin.generate_cte_sql(cte)?;
                if cte.recursive && !has_recursive {
                    cte_sql_parts.push(cte_sql);
                    has_recursive = true;
                } else if cte.recursive {
                    let without_with = cte_sql.strip_prefix("WITH RECURSIVE ").unwrap_or(&cte_sql);
                    cte_sql_parts.push(without_with.to_string());
                } else if !has_recursive {
                    cte_sql_parts.push(cte_sql);
                } else {
                    let without_with = cte_sql.strip_prefix("WITH ").unwrap_or(&cte_sql);
                    cte_sql_parts.push(without_with.to_string());
                }
            }
        }

        // Add advanced features as columns to the base query

        // Add window functions
        for (alias, window) in &self.windows {
            let window_sql = plugin.generate_window_sql(window)?;
            base_query = base_query.column(&format!("{} AS {}", window_sql, alias));
        }

        // Add CASE expressions
        for (alias, case_expr) in &self.case_expressions {
            let case_sql = plugin.generate_case_sql(case_expr)?;
            base_query = base_query.column(&format!("{} AS {}", case_sql, alias));
        }

        // Add JSON operations
        for (alias, json_op) in &self.json_operations {
            let json_sql = plugin.generate_json_sql(json_op)?;
            base_query = base_query.column(&format!("{} AS {}", json_sql, alias));
        }

        // Add aggregations
        for (alias, agg) in &self.aggregations {
            let agg_sql = plugin.generate_aggregate_sql(agg)?;
            base_query = base_query.column(&format!("{} AS {}", agg_sql, alias));
        }

        // Generate the final SQL by combining CTEs with the enhanced base query
        let mut final_sql = String::new();

        // Add CTEs first if they exist
        if !cte_sql_parts.is_empty() {
            final_sql.push_str(&cte_sql_parts.join(", "));
            final_sql.push(' ');
        }

        // Add the main query
        final_sql.push_str(&base_query.to_sql());

        // Add full-text search conditions as comments for now
        // In a real implementation, these would be integrated into the WHERE clause
        if !self.full_text_searches.is_empty() {
            for fts in &self.full_text_searches {
                let fts_sql = plugin.generate_fts_sql(fts)?;
                final_sql.push_str(&format!(" -- FTS: {}", fts_sql));
            }
        }

        Ok(final_sql)
    }
}

impl AdvancedSqlBuilder for AdvancedQueryBuilder {
    fn with_cte(&mut self, cte: CteDefinition) -> &mut Self {
        self.ctes.push(cte);
        self
    }
    
    fn window(&mut self, alias: String, window: WindowFunction) -> &mut Self {
        self.windows.insert(alias, window);
        self
    }
    
    fn case_when(&mut self, case_expr: CaseExpression) -> &mut Self {
        // Generate a unique alias for the case expression
        let alias = format!("case_{}", self.case_expressions.len());
        self.case_expressions.insert(alias, case_expr);
        self
    }
    
    fn json_op(&mut self, column: String, operation: JsonOperation) -> &mut Self {
        self.json_operations.insert(column, operation);
        self
    }
    
    fn aggregate(&mut self, alias: String, agg: AdvancedAggregation) -> &mut Self {
        self.aggregations.insert(alias, agg);
        self
    }
    
    fn full_text_search(&mut self, fts: FullTextSearch) -> &mut Self {
        self.full_text_searches.push(fts);
        self
    }
    
    fn build_advanced_sql(&self) -> String {
        // This is a simplified implementation
        // In practice, you'd want to clone self and call build()
        "-- Advanced SQL query would be built here".to_string()
    }
}

/// Implementation of SqlComposable trait for AdvancedQueryBuilder
impl SqlComposable for AdvancedQueryBuilder {
    type Output = Self;

    fn column_ref(mut self, column: &Column) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().column_ref(column);
        } else {
            // Create a new base query if none exists
            self.base_query = Some(SelectQueryBuilder::new().column_ref(column));
        }
        self
    }

    fn column_ref_as(mut self, column: &Column, alias: &str) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().column_ref_as(column, alias);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().column_ref_as(column, alias));
        }
        self
    }

    fn column_refs(mut self, columns: &[&Column]) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().column_refs(columns);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().column_refs(columns));
        }
        self
    }

    fn column_expr(mut self, expression: &str) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().column(expression);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().column(expression));
        }
        self
    }

    fn column_expr_as(mut self, expression: &str, alias: &str) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().column_expr_as(expression, alias);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().column_expr_as(expression, alias));
        }
        self
    }

    fn column_type_safe_expr(mut self, expression: &TypeSafeExpression) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().column_type_safe_expr(expression);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().column_type_safe_expr(expression));
        }
        self
    }

    fn column_type_safe_expr_as(mut self, expression: &TypeSafeExpression, alias: &str) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().column_type_safe_expr_as(expression, alias);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().column_type_safe_expr_as(expression, alias));
        }
        self
    }

    fn from_table_name(mut self, table_name: &str) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().from_table(table_name);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().from_table(table_name));
        }
        self
    }

    fn from_table_name_as(mut self, table_name: &str, alias: &str) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().from_table_as(table_name, alias);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().from_table_as(table_name, alias));
        }
        self
    }

    fn from_table(mut self, table: &Table) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().from_table_ref(table);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().from_table_ref(table));
        }
        self
    }

    fn from_aliased_table(mut self, aliased_table: &AliasedTable) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().from_aliased_table_ref(aliased_table);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().from_aliased_table_ref(aliased_table));
        }
        self
    }

    fn order_by_column(mut self, column: &Column, direction: crate::orm::query::OrderDirection) -> Self::Output {
        if let Some(ref mut base_query) = self.base_query {
            *base_query = base_query.clone().order_by_column(column, direction);
        } else {
            self.base_query = Some(SelectQueryBuilder::new().order_by_column(column, direction));
        }
        self
    }

    fn to_sql(&self) -> String {
        // Simple SQL generation without requiring a plugin
        // This is a basic implementation for the trait requirement
        if let Some(ref base_query) = self.base_query {
            base_query.to_sql()
        } else {
            "SELECT * FROM dual".to_string() // Default query
        }
    }

    fn build(self) -> SqlResult<Self::Output> {
        Ok(self)
    }
}

impl AdvancedQueryBuilder {
    /// Build SQL string (separate from trait method)
    pub fn build_sql(self) -> SqlResult<String> {
        let plugin = self.plugin.as_ref()
            .ok_or_else(|| SqlError::runtime_error("No database plugin configured"))?;

        // Get the base query or create a default one
        let mut base_query = self.base_query
            .unwrap_or_else(|| {
                use crate::orm::query::QueryBuilder;
                QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            });

        // For now, we'll handle CTEs by generating them separately and prepending to the final SQL
        // This maintains compatibility with the existing CTE system while using the type-safe base query
        let mut cte_sql_parts = Vec::new();

        // Generate CTE SQL
        for cte in &self.ctes {
            let cte_sql = plugin.generate_cte_sql(cte)?;
            cte_sql_parts.push(cte_sql);
        }

        // Generate window function SQL and add to base query
        for (alias, window) in &self.windows {
            let window_sql = plugin.generate_window_sql(window)?;
            base_query = base_query.column(&format!("{} AS {}", window_sql, alias));
        }

        // Generate JSON operation SQL and add to base query
        for (column, json_op) in &self.json_operations {
            let json_sql = plugin.generate_json_sql(json_op)?;
            base_query = base_query.column(&format!("{} AS {}", json_sql, column));
        }

        // Generate case expression SQL and add to base query
        for (alias, case_expr) in &self.case_expressions {
            let case_sql = plugin.generate_case_sql(case_expr)?;
            base_query = base_query.column(&format!("{} AS {}", case_sql, alias));
        }

        // Generate aggregation SQL and add to base query
        for (alias, agg) in &self.aggregations {
            let agg_sql = plugin.generate_aggregate_sql(agg)?;
            base_query = base_query.column(&format!("{} AS {}", agg_sql, alias));
        }

        // Get the base SQL
        let mut final_sql = String::new();

        // Add CTEs if any
        if !cte_sql_parts.is_empty() {
            final_sql.push_str(&cte_sql_parts.join(", "));
            final_sql.push(' ');
        }

        // Add the main query
        final_sql.push_str(&base_query.to_sql());

        Ok(final_sql)
    }
}

/// Helper function to create order by clauses
pub fn order_by(column: impl Into<String>, direction: OrderDirection) -> OrderByClause {
    OrderByClause {
        column: column.into(),
        direction,
        nulls: None,
    }
}

/// Helper function to create order by clauses with nulls handling
pub fn order_by_with_nulls(
    column: impl Into<String>, 
    direction: OrderDirection,
    nulls: NullsOrder
) -> OrderByClause {
    OrderByClause {
        column: column.into(),
        direction,
        nulls: Some(nulls),
    }
}

/// Implementation of AdvancedSqlComposable trait for AdvancedQueryBuilder
impl AdvancedSqlComposable for AdvancedQueryBuilder {
    fn window_function_as(self, alias: &str, function: &str, partition_by: &[&Column], order_by: &[&Column]) -> Self::Output {
        // Create a window function using the existing infrastructure
        let partition_cols: Vec<String> = partition_by.iter().map(|col| col.qualified_name()).collect();
        let order_cols: Vec<String> = order_by.iter().map(|col| col.qualified_name()).collect();

        // Use the existing with_window method with a constructed WindowFunction
        let window_func = WindowFunction {
            function: WindowFunctionType::RowNumber, // Fixed field name
            partition_by: partition_cols,
            order_by: order_cols.into_iter().map(|col| OrderByClause {
                column: col,
                direction: OrderDirection::Asc,
                nulls: None,
            }).collect(),
            frame: None, // Added missing field
        };

        self.with_window(alias, window_func)
    }

    fn json_extract_as(self, alias: &str, column: &Column, path: &str) -> Self::Output {
        // Use the existing with_json_extract_typed method
        self.with_json_extract_typed(alias, column, path)
    }

    fn case_expression(self, alias: &str, when_clauses: &[(&str, &str)], else_clause: Option<&str>) -> Self::Output {
        // Use the existing with_case method
        let when_exprs: Vec<WhenClause> = when_clauses.iter().map(|(condition, result)| {
            WhenClause {
                condition: Expression::Column(condition.to_string()),
                result: Expression::Column(result.to_string()),
            }
        }).collect();

        let else_expr = else_clause.map(|clause| Expression::Column(clause.to_string()));

        // Create a CaseExpression struct as expected by with_case
        let case_expr = CaseExpression {
            case_type: CaseType::Simple(Expression::Column("1".to_string())), // Simple case with dummy expression
            when_clauses: when_exprs,
            else_clause: else_expr,
        };

        self.with_case(alias, case_expr)
    }

    fn aggregate_function(self, alias: &str, function: &str, column: &Column) -> Self::Output {
        // Use the existing aggregate method with AdvancedAggregation
        let agg_func = match function.to_uppercase().as_str() {
            "COUNT" => AggregateFunction::Count(Some(column.qualified_name())),
            "SUM" => AggregateFunction::Sum(column.qualified_name()),
            "AVG" => AggregateFunction::Avg(column.qualified_name()),
            "MIN" => AggregateFunction::Min(column.qualified_name()),
            "MAX" => AggregateFunction::Max(column.qualified_name()),
            _ => AggregateFunction::Count(Some(column.qualified_name())), // Default fallback
        };

        let advanced_agg = AdvancedAggregation {
            function: agg_func,
            filter: None,
            distinct: false,
            over: None,
        };

        let mut result = self;
        result.aggregate(alias.to_string(), advanced_agg);
        result
    }

    fn aliased_column(self, aliased_col: &AliasedColumn) -> Self::Output {
        // Add the aliased column using column_ref_as
        self.column_ref_as(&aliased_col.column, &aliased_col.alias)
    }

    fn aliased_columns(self, aliased_cols: &[&AliasedColumn]) -> Self::Output {
        // Add multiple aliased columns
        let mut result = self;
        for aliased_col in aliased_cols {
            result = result.column_ref_as(&aliased_col.column, &aliased_col.alias);
        }
        result
    }
}

/// Helper function to create WHEN clauses for CASE expressions
pub fn when_clause(condition: Expression, result: Expression) -> WhenClause {
    WhenClause { condition, result }
}
