/// Query Free Monad - Composable database queries
/// 
/// This module implements queries as a free monad over SQL algebra:
/// - QueryF<A> is the base functor for SQL operations
/// - Query<A> is the free monad Free<QueryF, A>
/// - Monadic composition allows chaining queries
/// - Interpretation converts queries to SQL and executes them

use std::marker::PhantomData;
use serde::{Deserialize, Serialize};
use crate::sqlite::parser::ast::{Expression, SelectStatement, InsertStatement, UpdateStatement, DeleteStatement};
use crate::sqlite::parser::{SelectColumn, FromClause};
use crate::sqlite::types::{Value, DataType};
use crate::orm::{Driver, SqlResult};
use crate::orm::types::Row;
use crate::orm::schema::{Column, AliasedColumn, Table, AliasedTable, TypeSafeExpression};
use crate::orm::sql_composable::{SqlComposable, AdvancedSqlComposable};


/// Query functor - base functor for SQL operations
#[derive(Debug, Clone)]
pub enum QueryF<A> {
    /// SELECT query
    Select {
        statement: SelectStatement,
        next: A,
    },
    /// INSERT query
    Insert {
        statement: InsertStatement,
        next: A,
    },
    /// UPDATE query
    Update {
        statement: UpdateStatement,
        next: A,
    },
    /// DELETE query
    Delete {
        statement: DeleteStatement,
        next: A,
    },
    /// Raw SQL query with parameter bindings
    Raw {
        sql: String,
        binds: Vec<Value>,
        next: A,
    },
    /// Transaction boundary
    Transaction {
        queries: Vec<Query<()>>,
        next: A,
    },
}

/// Free monad over QueryF
#[derive(Debug, Clone)]
pub enum Query<A> {
    /// Pure value - return without database operation
    Pure(A),
    /// Free operation - database operation followed by continuation
    Free(Box<QueryF<Query<A>>>),
}

impl<A> Query<A> {
    /// Create a pure query that returns a value without database operations
    pub fn pure(value: A) -> Self {
        Query::Pure(value)
    }

    /// Create a SELECT query
    pub fn select(statement: SelectStatement) -> Query<Vec<QueryRow>> {
        Query::Free(Box::new(QueryF::Select {
            statement,
            next: Query::Pure(Vec::new()),
        }))
    }

    /// Create an INSERT query
    pub fn insert(statement: InsertStatement) -> Query<u64> {
        Query::Free(Box::new(QueryF::Insert {
            statement,
            next: Query::Pure(0),
        }))
    }

    /// Create an UPDATE query
    pub fn update(statement: UpdateStatement) -> Query<u64> {
        Query::Free(Box::new(QueryF::Update {
            statement,
            next: Query::Pure(0),
        }))
    }

    /// Create a DELETE query
    pub fn delete(statement: DeleteStatement) -> Query<u64> {
        Query::Free(Box::new(QueryF::Delete {
            statement,
            next: Query::Pure(0),
        }))
    }

    /// Create a raw SQL query
    pub fn raw(sql: impl Into<String>, binds: Vec<Value>) -> Query<Vec<QueryRow>> {
        Query::Free(Box::new(QueryF::Raw {
            sql: sql.into(),
            binds,
            next: Query::Pure(Vec::new()),
        }))
    }

    /// Create a transaction containing multiple queries
    pub fn transaction(queries: Vec<Query<()>>) -> Query<()> {
        Query::Free(Box::new(QueryF::Transaction {
            queries,
            next: Query::Pure(()),
        }))
    }

    /// Monadic bind operation
    /// This is the key operation that allows composing queries
    pub fn bind<B, F>(self, f: F) -> Query<B>
    where
        F: FnOnce(A) -> Query<B>,
    {
        match self {
            Query::Pure(a) => f(a),
            Query::Free(query_f) => {
                let mapped_f = match *query_f {
                    QueryF::Select { statement, next } => {
                        QueryF::Select {
                            statement,
                            next: next.bind(f),
                        }
                    }
                    QueryF::Insert { statement, next } => {
                        QueryF::Insert {
                            statement,
                            next: next.bind(f),
                        }
                    }
                    QueryF::Update { statement, next } => {
                        QueryF::Update {
                            statement,
                            next: next.bind(f),
                        }
                    }
                    QueryF::Delete { statement, next } => {
                        QueryF::Delete {
                            statement,
                            next: next.bind(f),
                        }
                    }
                    QueryF::Raw { sql, binds, next } => {
                        QueryF::Raw {
                            sql,
                            binds,
                            next: next.bind(f),
                        }
                    }
                    QueryF::Transaction { queries, next } => {
                        QueryF::Transaction {
                            queries,
                            next: next.bind(f),
                        }
                    }
                };
                Query::Free(Box::new(mapped_f))
            }
        }
    }

    /// Map operation (functor)
    pub fn map<B, F>(self, f: F) -> Query<B>
    where
        F: FnOnce(A) -> B,
    {
        self.bind(|a| Query::pure(f(a)))
    }

    /// Execute the query using the given driver
    pub async fn execute<D: Driver>(self, driver: &D) -> SqlResult<A>
    where
        D::Row: Row,
    {
        self.interpret(driver).await
    }

    /// Interpret the query by executing it against a driver
    async fn interpret<D: Driver>(self, driver: &D) -> SqlResult<A>
    where
        D::Row: Row,
    {
        match self {
            Query::Pure(a) => Ok(a),
            Query::Free(query_f) => {
                match *query_f {
                    QueryF::Select { statement, next } => {
                        let sql = statement.to_sql();
                        let rows = driver.observe(&sql, &[]).await?;

                        // Convert driver-specific rows to QueryRow
                        let query_rows: Vec<QueryRow> = rows.into_iter().map(|row| {
                            QueryRow {
                                columns: Row::columns(&row).to_vec(),
                                values: Row::values(&row).to_vec(),
                            }
                        }).collect();

                        // For now, we ignore the converted rows and continue with next
                        // In a real implementation, we would pass the results to the continuation
                        next.interpret(driver).await
                    }
                    QueryF::Insert { statement, next } => {
                        let sql = statement.to_sql();
                        let _result = driver.observe(&sql, &[]).await?;

                        // Extract affected row count
                        // In a real implementation, we would get the actual row count from the driver
                        // For now, we simulate 1 affected row for INSERT operations
                        let _affected_rows = 1u64;

                        next.interpret(driver).await
                    }
                    QueryF::Update { statement, next } => {
                        let sql = statement.to_sql();
                        let _result = driver.observe(&sql, &[]).await?;

                        // Extract affected row count
                        // In a real implementation, we would get the actual row count from the driver
                        // For now, we simulate 1 affected row for UPDATE operations
                        let _affected_rows = 1u64;

                        next.interpret(driver).await
                    }
                    QueryF::Delete { statement, next } => {
                        let sql = statement.to_sql();
                        let _result = driver.observe(&sql, &[]).await?;

                        // Extract affected row count
                        // In a real implementation, we would get the actual row count from the driver
                        // For now, we simulate 1 affected row for DELETE operations
                        let _affected_rows = 1u64;

                        next.interpret(driver).await
                    }
                    QueryF::Raw { sql, binds, next } => {
                        let result = driver.observe(&sql, &binds).await?;

                        // Convert result to appropriate type
                        // In a real implementation, we would:
                        // 1. Determine the expected return type from the query
                        // 2. Convert the driver-specific result to that type
                        // 3. Pass the converted result to the continuation

                        let _converted_result: Vec<QueryRow> = result.into_iter().map(|row| {
                            QueryRow {
                                columns: Row::columns(&row).to_vec(),
                                values: Row::values(&row).to_vec(),
                            }
                        }).collect();

                        next.interpret(driver).await
                    }
                    QueryF::Transaction { queries, next } => {
                        // Implement transaction handling
                        // In a real implementation, this would:
                        // 1. Begin a database transaction
                        // 2. Execute all queries within the transaction
                        // 3. Commit if all succeed, rollback if any fail
                        // 4. Continue with the next operation

                        let mut transaction = driver.begin_transaction().await?;

                        for query in queries {
                            // Execute each query within the transaction
                            // For now, we just interpret them normally
                            query.interpret(driver).await?;
                        }

                        // Commit the transaction
                        transaction.commit().await?;

                        next.interpret(driver).await
                    }
                }
            }
        }
    }
}

/// Query result row - generic representation of database row
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRow {
    pub columns: Vec<String>,
    pub values: Vec<Value>,
}

impl QueryRow {
    pub fn new(columns: Vec<String>, values: Vec<Value>) -> Self {
        Self { columns, values }
    }

    pub fn get(&self, column: &str) -> Option<&Value> {
        self.columns
            .iter()
            .position(|c| c == column)
            .and_then(|i| self.values.get(i))
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }
}

/// Query builder for constructing type-safe queries
pub struct QueryBuilder<T> {
    _phantom: PhantomData<T>,
}

impl<T> QueryBuilder<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Build a SELECT query
    pub fn select() -> SelectQueryBuilder {
        SelectQueryBuilder::new()
    }

    /// Build an INSERT query
    pub fn insert() -> InsertQueryBuilder {
        InsertQueryBuilder::new()
    }

    /// Build an UPDATE query
    pub fn update() -> UpdateQueryBuilder {
        UpdateQueryBuilder::new()
    }

    /// Build a DELETE query
    pub fn delete() -> DeleteQueryBuilder {
        DeleteQueryBuilder::new()
    }
}

/// Builder for SELECT queries
#[derive(Debug, Clone)]
pub struct SelectQueryBuilder {
    columns: Vec<SelectColumn>,
    from: Option<FromClause>,
    where_clause: Option<Expression>,
    limit: Option<u64>,
    offset: Option<u64>,
    union_queries: Vec<(UnionType, SelectQueryBuilder)>,
    ctes: Vec<CteBuilder>,
    order_by: Vec<OrderByClause>,
}

/// Union type for UNION operations
#[derive(Debug, Clone)]
pub enum UnionType {
    Union,
    UnionAll,
}

/// CTE (Common Table Expression) builder
#[derive(Debug, Clone)]
pub struct CteBuilder {
    pub name: String,
    pub columns: Option<Vec<String>>,
    pub query: SelectQueryBuilder,
    pub recursive: bool,
}

/// Order by clause for queries
#[derive(Debug, Clone)]
pub struct OrderByClause {
    pub column: String,
    pub direction: OrderDirection,
    pub nulls: Option<NullsOrder>,
}

#[derive(Debug, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub enum NullsOrder {
    First,
    Last,
}

impl SelectQueryBuilder {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            from: None,
            where_clause: None,
            limit: None,
            offset: None,
            union_queries: Vec::new(),
            ctes: Vec::new(),
            order_by: Vec::new(),
        }
    }

    pub fn column(mut self, column: impl Into<String>) -> Self {
        self.columns.push(SelectColumn::Expression {
            expr: Expression::Column(column.into()),
            alias: None
        });
        self
    }

    /// Add a column using a Column type for type safety
    pub fn column_ref(mut self, column: &Column) -> Self {
        self.columns.push(SelectColumn::Expression {
            expr: Expression::Column(column.qualified_name()),
            alias: None
        });
        self
    }

    /// Add a column with an alias using Column type
    pub fn column_ref_as(mut self, column: &Column, alias: &str) -> Self {
        self.columns.push(SelectColumn::Expression {
            expr: Expression::Column(column.qualified_name()),
            alias: Some(alias.to_string())
        });
        self
    }

    /// Add multiple column references at once
    pub fn column_refs(mut self, columns: &[&Column]) -> Self {
        for column in columns {
            self.columns.push(SelectColumn::Expression {
                expr: Expression::Column(column.qualified_name()),
                alias: None
            });
        }
        self
    }

    /// Add an aliased column using AliasedColumn type
    pub fn aliased_column(mut self, aliased_col: &AliasedColumn) -> Self {
        self.columns.push(SelectColumn::Expression {
            expr: Expression::Column(aliased_col.qualified_name()),
            alias: Some(aliased_col.alias_name().to_string())
        });
        self
    }

    /// Add multiple aliased columns at once
    pub fn aliased_columns(mut self, aliased_cols: &[&AliasedColumn]) -> Self {
        for aliased_col in aliased_cols {
            self.columns.push(SelectColumn::Expression {
                expr: Expression::Column(aliased_col.qualified_name()),
                alias: Some(aliased_col.alias_name().to_string())
            });
        }
        self
    }

    /// Add a scalar subquery as a column
    pub fn column_subquery(mut self, subquery: SelectQueryBuilder, alias: &str) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.columns.push(SelectColumn::Expression {
            expr: Expression::ScalarSubquery(Box::new(subquery_stmt)),
            alias: Some(alias.to_string())
        });
        self
    }

    /// Add a scalar subquery as a column without alias
    pub fn column_subquery_no_alias(mut self, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.columns.push(SelectColumn::Expression {
            expr: Expression::ScalarSubquery(Box::new(subquery_stmt)),
            alias: None
        });
        self
    }

    /// Add an expression column with alias
    pub fn column_expr_as(mut self, expr: &str, alias: &str) -> Self {
        self.columns.push(SelectColumn::Expression {
            expr: Expression::Column(expr.to_string()),
            alias: Some(alias.to_string())
        });
        self
    }

    pub fn columns(mut self, columns: Vec<String>) -> Self {
        let select_columns = columns.into_iter().map(|col| {
            SelectColumn::Expression {
                expr: Expression::Column(col),
                alias: None
            }
        }).collect::<Vec<_>>();
        self.columns.extend(select_columns);
        self
    }

    pub fn from(mut self, table: impl Into<String>) -> Self {
        self.from = Some(FromClause {
            source: crate::sqlite::parser::ast::FromSource::Table(table.into()),
            alias: None,
            joins: Vec::new(),
        });
        self
    }

    /// Set the FROM clause using a type-safe table reference
    pub fn from_table(mut self, table_name: &str) -> Self {
        self.from = Some(FromClause {
            source: crate::sqlite::parser::ast::FromSource::Table(table_name.to_string()),
            alias: None,
            joins: Vec::new(),
        });
        self
    }

    /// Set the FROM clause with an alias using a type-safe table reference
    pub fn from_table_as(mut self, table_name: &str, alias: &str) -> Self {
        self.from = Some(FromClause {
            source: crate::sqlite::parser::ast::FromSource::Table(table_name.to_string()),
            alias: Some(alias.to_string()),
            joins: Vec::new(),
        });
        self
    }

    /// Set the FROM clause using a type-safe Table reference
    pub fn from_table_ref(mut self, table: &Table) -> Self {
        self.from = Some(FromClause {
            source: crate::sqlite::parser::ast::FromSource::Table(table.qualified_name()),
            alias: None,
            joins: Vec::new(),
        });
        self
    }

    /// Set the FROM clause using a type-safe AliasedTable
    pub fn from_aliased_table_ref(mut self, aliased_table: &AliasedTable) -> Self {
        self.from = Some(FromClause {
            source: crate::sqlite::parser::ast::FromSource::Table(aliased_table.qualified_name()),
            alias: Some(aliased_table.alias_name().to_string()),
            joins: Vec::new(),
        });
        self
    }

    /// Set the FROM clause using a subquery
    pub fn from_subquery(mut self, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.from = Some(FromClause {
            source: crate::sqlite::parser::ast::FromSource::Subquery(Box::new(subquery_stmt)),
            alias: None,
            joins: Vec::new(),
        });
        self
    }

    /// Set the FROM clause using a subquery with alias
    pub fn from_subquery_as(mut self, subquery: SelectQueryBuilder, alias: &str) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.from = Some(FromClause {
            source: crate::sqlite::parser::ast::FromSource::Subquery(Box::new(subquery_stmt)),
            alias: Some(alias.to_string()),
            joins: Vec::new(),
        });
        self
    }

    pub fn where_clause(mut self, condition: Expression) -> Self {
        self.where_clause = Some(condition);
        self
    }

    /// Add WHERE EXISTS subquery
    pub fn where_exists(mut self, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.where_clause = Some(Expression::Exists(Box::new(subquery_stmt)));
        self
    }

    /// Add WHERE NOT EXISTS subquery
    pub fn where_not_exists(mut self, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.where_clause = Some(Expression::NotExists(Box::new(subquery_stmt)));
        self
    }

    /// Add WHERE column IN (subquery)
    pub fn where_in_subquery(mut self, column: &str, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.where_clause = Some(Expression::InSubquery {
            expr: Box::new(Expression::Column(column.to_string())),
            subquery: Box::new(subquery_stmt),
        });
        self
    }

    /// Add WHERE column NOT IN (subquery)
    pub fn where_not_in_subquery(mut self, column: &str, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.where_clause = Some(Expression::NotInSubquery {
            expr: Box::new(Expression::Column(column.to_string())),
            subquery: Box::new(subquery_stmt),
        });
        self
    }

    /// Add WHERE column IN (subquery) using type-safe column
    pub fn where_column_in_subquery(mut self, column: &Column, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.where_clause = Some(Expression::InSubquery {
            expr: Box::new(Expression::Column(column.qualified_name())),
            subquery: Box::new(subquery_stmt),
        });
        self
    }

    /// Add WHERE column NOT IN (subquery) using type-safe column
    pub fn where_column_not_in_subquery(mut self, column: &Column, subquery: SelectQueryBuilder) -> Self {
        let subquery_stmt = subquery.build_select_statement();
        self.where_clause = Some(Expression::NotInSubquery {
            expr: Box::new(Expression::Column(column.qualified_name())),
            subquery: Box::new(subquery_stmt),
        });
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add a UNION query
    pub fn union(mut self, query: SelectQueryBuilder) -> Self {
        self.union_queries.push((UnionType::Union, query));
        self
    }

    /// Add a UNION ALL query
    pub fn union_all(mut self, query: SelectQueryBuilder) -> Self {
        self.union_queries.push((UnionType::UnionAll, query));
        self
    }

    /// Add a CTE (Common Table Expression)
    pub fn with_cte(mut self, name: impl Into<String>, query: SelectQueryBuilder) -> Self {
        self.ctes.push(CteBuilder {
            name: name.into(),
            columns: None,
            query,
            recursive: false,
        });
        self
    }

    /// Add a recursive CTE
    pub fn with_recursive_cte(
        mut self,
        name: impl Into<String>,
        columns: Vec<String>,
        query: SelectQueryBuilder,
    ) -> Self {
        self.ctes.push(CteBuilder {
            name: name.into(),
            columns: Some(columns),
            query,
            recursive: true,
        });
        self
    }

    /// Add an ORDER BY clause
    pub fn order_by(mut self, column: impl Into<String>, direction: OrderDirection) -> Self {
        self.order_by.push(OrderByClause {
            column: column.into(),
            direction,
            nulls: None,
        });
        self
    }

    /// Add an ORDER BY clause using a type-safe Column
    pub fn order_by_column(mut self, column: &Column, direction: OrderDirection) -> Self {
        self.order_by.push(OrderByClause {
            column: column.qualified_name(),
            direction,
            nulls: None,
        });
        self
    }

    /// Add an ORDER BY clause with NULLS handling
    pub fn order_by_with_nulls(
        mut self,
        column: impl Into<String>,
        direction: OrderDirection,
        nulls: NullsOrder,
    ) -> Self {
        self.order_by.push(OrderByClause {
            column: column.into(),
            direction,
            nulls: Some(nulls),
        });
        self
    }

    /// Add an ORDER BY clause with NULLS handling using a type-safe Column
    pub fn order_by_column_with_nulls(
        mut self,
        column: &Column,
        direction: OrderDirection,
        nulls: NullsOrder,
    ) -> Self {
        self.order_by.push(OrderByClause {
            column: column.qualified_name(),
            direction,
            nulls: Some(nulls),
        });
        self
    }

    pub fn build(self) -> Query<Vec<QueryRow>> {
        let statement = self.build_select_statement();
        Query::<Vec<QueryRow>>::select(statement)
    }

    /// Build a SelectStatement from the builder state
    pub fn build_select_statement(self) -> SelectStatement {
        SelectStatement {
            columns: if self.columns.is_empty() { vec![SelectColumn::Wildcard] } else { self.columns },
            from: self.from,
            where_clause: self.where_clause,
            group_by: None,
            having: None,
            order_by: Some(self.order_by.into_iter().map(|o| crate::sqlite::parser::ast::OrderByClause {
                expr: Expression::Column(o.column),
                direction: match o.direction {
                    OrderDirection::Asc => crate::sqlite::parser::ast::OrderDirection::Asc,
                    OrderDirection::Desc => crate::sqlite::parser::ast::OrderDirection::Desc,
                }
            }).collect()).filter(|v: &Vec<_>| !v.is_empty()),
            limit: self.limit.map(|l| crate::sqlite::parser::ast::LimitClause {
                limit: l,
                offset: self.offset,
            }),
        }
    }

    /// Build the SQL string representation
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();

        // Add CTEs first
        if !self.ctes.is_empty() {
            let mut cte_parts = Vec::new();
            let mut has_recursive = false;

            for cte in &self.ctes {
                if cte.recursive && !has_recursive {
                    sql.push_str("WITH RECURSIVE ");
                    has_recursive = true;
                } else if !has_recursive {
                    sql.push_str("WITH ");
                }

                cte_parts.push(self.build_cte_sql(cte));
            }

            sql.push_str(&cte_parts.join(", "));
            sql.push(' ');
        }

        // Build main SELECT
        sql.push_str(&self.build_select_sql());

        // Add UNION queries
        for (union_type, union_query) in &self.union_queries {
            match union_type {
                UnionType::Union => sql.push_str(" UNION "),
                UnionType::UnionAll => sql.push_str(" UNION ALL "),
            }
            sql.push_str(&union_query.build_select_sql());
        }

        // Add ORDER BY
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_clauses: Vec<String> = self.order_by.iter().map(|clause| {
                let mut order_sql = clause.column.clone();
                match clause.direction {
                    OrderDirection::Asc => order_sql.push_str(" ASC"),
                    OrderDirection::Desc => order_sql.push_str(" DESC"),
                }
                if let Some(ref nulls) = clause.nulls {
                    match nulls {
                        NullsOrder::First => order_sql.push_str(" NULLS FIRST"),
                        NullsOrder::Last => order_sql.push_str(" NULLS LAST"),
                    }
                }
                order_sql
            }).collect();
            sql.push_str(&order_clauses.join(", "));
        }

        sql
    }

    fn build_cte_sql(&self, cte: &CteBuilder) -> String {
        let mut sql = String::new();

        sql.push_str(&cte.name);

        if let Some(ref columns) = cte.columns {
            sql.push_str(" (");
            sql.push_str(&columns.join(", "));
            sql.push(')');
        }

        sql.push_str(" AS (");
        sql.push_str(&cte.query.build_select_sql());
        sql.push(')');

        sql
    }

    fn build_select_sql(&self) -> String {
        let mut sql = String::new();

        sql.push_str("SELECT ");

        if self.columns.is_empty() {
            sql.push('*');
        } else {
            let column_strs: Vec<String> = self.columns.iter().map(|col| {
                match col {
                    SelectColumn::Wildcard => "*".to_string(),
                    SelectColumn::Expression { expr, alias } => {
                        let expr_str = self.expression_to_sql(expr);
                        if let Some(alias) = alias {
                            format!("{} AS {}", expr_str, alias)
                        } else {
                            expr_str
                        }
                    }
                }
            }).collect();
            sql.push_str(&column_strs.join(", "));
        }

        if let Some(ref from) = self.from {
            sql.push_str(" FROM ");
            match &from.source {
                crate::sqlite::parser::ast::FromSource::Table(table) => {
                    sql.push_str(table);
                },
                crate::sqlite::parser::ast::FromSource::Subquery(subquery) => {
                    sql.push('(');
                    sql.push_str(&subquery.to_string());
                    sql.push(')');
                }
            }
            if let Some(ref alias) = from.alias {
                sql.push_str(" AS ");
                sql.push_str(alias);
            }
        }

        if let Some(ref where_clause) = self.where_clause {
            sql.push_str(" WHERE ");
            sql.push_str(&self.expression_to_sql(where_clause));
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }

    fn expression_to_sql(&self, expr: &Expression) -> String {
        match expr {
            Expression::Column(name) => name.clone(),
            Expression::Literal(value) => format!("{:?}", value), // Simplified
            Expression::Binary { left, op: _, right } => {
                format!("{} /* op */ {}",
                    self.expression_to_sql(left),
                    self.expression_to_sql(right)
                )
            }
            Expression::Subquery(stmt) => format!("({})", stmt),
            Expression::Exists(stmt) => format!("EXISTS ({})", stmt),
            Expression::NotExists(stmt) => format!("NOT EXISTS ({})", stmt),
            Expression::InSubquery { expr, subquery } => {
                format!("{} IN ({})", self.expression_to_sql(expr), subquery)
            }
            Expression::NotInSubquery { expr, subquery } => {
                format!("{} NOT IN ({})", self.expression_to_sql(expr), subquery)
            }
            Expression::ScalarSubquery(stmt) => format!("({})", stmt),
            _ => "/* complex expression */".to_string(), // Simplified for now
        }
    }
}

/// Implementation of SqlComposable trait for SelectQueryBuilder
impl SqlComposable for SelectQueryBuilder {
    type Output = Self;

    fn column_ref(self, column: &Column) -> Self::Output {
        self.column_ref(column)
    }

    fn column_ref_as(self, column: &Column, alias: &str) -> Self::Output {
        self.column_ref_as(column, alias)
    }

    fn column_refs(self, columns: &[&Column]) -> Self::Output {
        self.column_refs(columns)
    }

    fn column_expr(self, expression: &str) -> Self::Output {
        self.column(expression)
    }

    fn column_expr_as(self, expression: &str, alias: &str) -> Self::Output {
        let mut result = self.column(expression);
        // Update the last added column to have the alias
        if let Some(last_column) = result.columns.last_mut() {
            if let SelectColumn::Expression { alias: ref mut col_alias, .. } = last_column {
                *col_alias = Some(alias.to_string());
            }
        }
        result
    }

    fn column_type_safe_expr(self, expression: &TypeSafeExpression) -> Self::Output {
        self.column(expression.to_sql())
    }

    fn column_type_safe_expr_as(self, expression: &TypeSafeExpression, alias: &str) -> Self::Output {
        let mut result = self.column(expression.to_sql());
        // Update the last added column to have the alias
        if let Some(last_column) = result.columns.last_mut() {
            if let SelectColumn::Expression { alias: ref mut col_alias, .. } = last_column {
                *col_alias = Some(alias.to_string());
            }
        }
        result
    }

    fn from_table_name(self, table_name: &str) -> Self::Output {
        self.from_table(table_name)
    }

    fn from_table_name_as(self, table_name: &str, alias: &str) -> Self::Output {
        self.from_table_as(table_name, alias)
    }

    fn from_table(self, table: &Table) -> Self::Output {
        self.from_table_ref(table)
    }

    fn from_aliased_table(self, aliased_table: &AliasedTable) -> Self::Output {
        self.from_aliased_table_ref(aliased_table)
    }

    fn order_by_column(self, column: &Column, direction: crate::orm::query::OrderDirection) -> Self::Output {
        self.order_by_column(column, direction)
    }

    fn to_sql(&self) -> String {
        self.to_sql()
    }

    fn build(self) -> SqlResult<Self::Output> {
        Ok(self)
    }
}

/// Implementation of AdvancedSqlComposable trait for SelectQueryBuilder
impl AdvancedSqlComposable for SelectQueryBuilder {
    fn window_function_as(self, alias: &str, function: &str, partition_by: &[&Column], order_by: &[&Column]) -> Self::Output {
        // This is a simplified implementation - in a real system, this would integrate with the advanced query builder
        let partition_cols: Vec<String> = partition_by.iter().map(|col| col.qualified_name()).collect();
        let order_cols: Vec<String> = order_by.iter().map(|col| col.qualified_name()).collect();

        let window_expr = format!("{}() OVER (PARTITION BY {} ORDER BY {})",
            function,
            partition_cols.join(", "),
            order_cols.join(", ")
        );

        let mut result = self;
        result.columns.push(SelectColumn::Expression {
            expr: Expression::Column(window_expr),
            alias: Some(alias.to_string())
        });
        result
    }

    fn json_extract_as(self, alias: &str, column: &Column, path: &str) -> Self::Output {
        let json_expr = format!("JSON_EXTRACT({}, '$.{}')", column.qualified_name(), path);
        let mut result = self;
        result.columns.push(SelectColumn::Expression {
            expr: Expression::Column(json_expr),
            alias: Some(alias.to_string())
        });
        result
    }

    fn case_expression(self, alias: &str, when_clauses: &[(&str, &str)], else_clause: Option<&str>) -> Self::Output {
        let mut case_expr = "CASE".to_string();
        for (condition, result) in when_clauses {
            case_expr.push_str(&format!(" WHEN {} THEN {}", condition, result));
        }
        if let Some(else_result) = else_clause {
            case_expr.push_str(&format!(" ELSE {}", else_result));
        }
        case_expr.push_str(" END");

        let mut result = self;
        result.columns.push(SelectColumn::Expression {
            expr: Expression::Column(case_expr),
            alias: Some(alias.to_string())
        });
        result
    }

    fn aggregate_function(self, alias: &str, function: &str, column: &Column) -> Self::Output {
        let agg_expr = format!("{}({})", function, column.qualified_name());
        let mut result = self;
        result.columns.push(SelectColumn::Expression {
            expr: Expression::Column(agg_expr),
            alias: Some(alias.to_string())
        });
        result
    }

    fn aliased_column(self, aliased_col: &AliasedColumn) -> Self::Output {
        self.aliased_column(aliased_col)
    }

    fn aliased_columns(self, aliased_cols: &[&AliasedColumn]) -> Self::Output {
        self.aliased_columns(aliased_cols)
    }
}

/// Builder for INSERT queries
pub struct InsertQueryBuilder {
    table: Option<String>,
    columns: Vec<String>,
    values: Vec<Vec<Value>>,
}

impl InsertQueryBuilder {
    pub fn new() -> Self {
        Self {
            table: None,
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn into(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }

    pub fn columns(mut self, columns: Vec<String>) -> Self {
        self.columns = columns;
        self
    }

    pub fn values(mut self, values: Vec<Value>) -> Self {
        self.values.push(values);
        self
    }

    pub fn build(self) -> Query<u64> {
        // Build InsertStatement from builder state
        // In a real implementation, this would:
        // 1. Validate that table and columns are set
        // 2. Ensure values match column count
        // 3. Handle conflict resolution strategies
        // 4. Construct complete INSERT statement

        let statement = InsertStatement {
            table: self.table.unwrap_or_else(|| "unknown_table".to_string()),
            columns: self.columns,
            values: self.values,
            on_conflict: None,
        };
        Query::<u64>::insert(statement)
    }
}

/// Builder for UPDATE queries
pub struct UpdateQueryBuilder {
    table: Option<String>,
    set_clauses: Vec<(String, Value)>,
    where_clause: Option<Expression>,
}

impl UpdateQueryBuilder {
    pub fn new() -> Self {
        Self {
            table: None,
            set_clauses: Vec::new(),
            where_clause: None,
        }
    }

    pub fn table(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }

    pub fn set(mut self, column: impl Into<String>, value: Value) -> Self {
        self.set_clauses.push((column.into(), value));
        self
    }

    pub fn where_clause(mut self, condition: Expression) -> Self {
        self.where_clause = Some(condition);
        self
    }

    pub fn build(self) -> Query<u64> {
        // Build UpdateStatement from builder state
        // In a real implementation, this would:
        // 1. Validate that table and set clauses are provided
        // 2. Ensure WHERE clause is present to prevent accidental full table updates
        // 3. Validate column names and value types
        // 4. Construct complete UPDATE statement

        let statement = UpdateStatement {
            table: self.table.unwrap_or_else(|| "unknown_table".to_string()),
            set_clauses: self.set_clauses,
            where_clause: self.where_clause,
        };
        Query::<u64>::update(statement)
    }
}

/// Builder for DELETE queries
pub struct DeleteQueryBuilder {
    table: Option<String>,
    where_clause: Option<Expression>,
}

impl DeleteQueryBuilder {
    pub fn new() -> Self {
        Self {
            table: None,
            where_clause: None,
        }
    }

    pub fn from(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }

    pub fn where_clause(mut self, condition: Expression) -> Self {
        self.where_clause = Some(condition);
        self
    }

    pub fn build(self) -> Query<u64> {
        // Build DeleteStatement from builder state
        // In a real implementation, this would:
        // 1. Validate that table is provided
        // 2. Strongly recommend WHERE clause to prevent accidental full table deletion
        // 3. Validate column names in WHERE clause
        // 4. Construct complete DELETE statement

        let statement = DeleteStatement {
            table: self.table.unwrap_or_else(|| "unknown_table".to_string()),
            where_clause: self.where_clause,
        };
        Query::<u64>::delete(statement)
    }
}

/// Extension trait for converting AST to SQL
trait ToSql {
    fn to_sql(&self) -> String;
}

impl ToSql for SelectStatement {
    fn to_sql(&self) -> String {
        // Generate proper SQL for SELECT statement
        // In a real implementation, this would:
        // 1. Handle column list properly (including expressions)
        // 2. Generate proper FROM clause with joins
        // 3. Handle WHERE, GROUP BY, HAVING, ORDER BY clauses
        // 4. Apply proper SQL escaping and quoting

        let mut sql = String::from("SELECT ");

        // Columns
        if self.columns.is_empty() {
            sql.push_str("*");
        } else {
            let column_strs: Vec<String> = self.columns.iter().map(|c| c.to_string()).collect();
            sql.push_str(&column_strs.join(", "));
        }

        // FROM clause
        if let Some(ref table) = self.from {
            sql.push_str(&format!(" FROM {}", table));
        }

        // WHERE clause (simplified)
        if self.where_clause.is_some() {
            sql.push_str(" WHERE <condition>");
        }

        // LIMIT clause
        if let Some(ref limit_clause) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit_clause.limit));
            if let Some(offset) = limit_clause.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }
        }

        sql
    }
}

impl ToSql for InsertStatement {
    fn to_sql(&self) -> String {
        // Generate proper SQL for INSERT statement
        // In a real implementation, this would:
        // 1. Handle multiple value rows
        // 2. Generate proper value placeholders for prepared statements
        // 3. Handle ON CONFLICT clauses
        // 4. Apply proper SQL escaping and quoting

        let mut sql = format!("INSERT INTO {}", self.table);

        // Columns
        if !self.columns.is_empty() {
            sql.push_str(&format!(" ({})", self.columns.join(", ")));
        }

        // Values
        sql.push_str(" VALUES ");
        if self.values.is_empty() {
            sql.push_str("()");
        } else {
            let value_rows: Vec<String> = self.values.iter().map(|row| {
                let value_strs: Vec<String> = row.iter().map(|v| {
                    match v {
                        Value::Integer(i) => i.to_string(),
                        Value::Real(r) => r.to_string(),
                        Value::Text(s) => format!("'{}'", s.replace("'", "''")),
                        Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                        Value::Null => "NULL".to_string(),
                        Value::Blob(_) => "X'<blob>'".to_string(), // Hex representation placeholder
                    }
                }).collect();
                format!("({})", value_strs.join(", "))
            }).collect();
            sql.push_str(&value_rows.join(", "));
        }

        sql
    }
}

impl ToSql for UpdateStatement {
    fn to_sql(&self) -> String {
        // Generate proper SQL for UPDATE statement
        // In a real implementation, this would:
        // 1. Generate proper SET clauses with value placeholders
        // 2. Handle complex WHERE conditions
        // 3. Apply proper SQL escaping and quoting
        // 4. Support JOIN clauses for complex updates

        let mut sql = format!("UPDATE {}", self.table);

        // SET clauses
        if !self.set_clauses.is_empty() {
            let set_strs: Vec<String> = self.set_clauses.iter().map(|(column, value)| {
                let value_str = match value {
                    Value::Integer(i) => i.to_string(),
                    Value::Real(r) => r.to_string(),
                    Value::Text(s) => format!("'{}'", s.replace("'", "''")),
                    Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                    Value::Null => "NULL".to_string(),
                    Value::Blob(_) => "X'<blob>'".to_string(), // Hex representation placeholder
                };
                format!("{} = {}", column, value_str)
            }).collect();
            sql.push_str(&format!(" SET {}", set_strs.join(", ")));
        }

        // WHERE clause (simplified)
        if self.where_clause.is_some() {
            sql.push_str(" WHERE <condition>");
        }

        sql
    }
}

impl ToSql for DeleteStatement {
    fn to_sql(&self) -> String {
        // Generate proper SQL for DELETE statement
        // In a real implementation, this would:
        // 1. Handle complex WHERE conditions
        // 2. Support JOIN clauses for complex deletes
        // 3. Apply proper SQL escaping and quoting
        // 4. Add safety checks for DELETE without WHERE

        let mut sql = format!("DELETE FROM {}", self.table);

        // WHERE clause (simplified)
        if self.where_clause.is_some() {
            sql.push_str(" WHERE <condition>");
        } else {
            // In a real implementation, we might warn about DELETE without WHERE
            // or require explicit confirmation for such operations
        }

        sql
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_query() {
        let query = Query::pure(42);
        match query {
            Query::Pure(value) => assert_eq!(value, 42),
            _ => panic!("Expected pure query"),
        }
    }

    #[test]
    fn test_query_bind() {
        let query = Query::pure(5)
            .bind(|x| Query::pure(x * 2))
            .bind(|x| Query::pure(x + 1));
        
        match query {
            Query::Pure(value) => assert_eq!(value, 11),
            _ => panic!("Expected pure query after bind"),
        }
    }

    #[test]
    fn test_query_map() {
        let query = Query::pure(5).map(|x| x * 2);
        
        match query {
            Query::Pure(value) => assert_eq!(value, 10),
            _ => panic!("Expected pure query after map"),
        }
    }

    #[test]
    fn test_select_builder() {
        let query = QueryBuilder::<Vec<QueryRow>>::select()
            .column("id")
            .column("name")
            .from("users")
            .limit(10)
            .build();
        
        // Query should be constructed without errors
        match query {
            Query::Free(_) => {}, // Expected
            Query::Pure(_) => panic!("Expected free query"),
        }
    }
}
