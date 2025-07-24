/// SQLite module - Re-exports from the sqlite directory
///
/// This module provides access to the SQLite components that are reused
/// by the ORM system, including the parser, AST, types, and error handling.

// Import the types and error modules first
#[path = "../sqlite/src/types.rs"]
pub mod types;

#[path = "../sqlite/src/error.rs"]
pub mod error;

// Import parser components individually to avoid circular dependencies
pub mod parser {
    use super::{types, error};
    use serde::{Deserialize, Serialize};

    // Re-export types for parser modules
    pub use super::types::{Value, DataType};
    pub use super::error::{SqlError, SqlResult};

    // Simplified AST for ORM use
    pub mod ast {
        use super::{Value, DataType};
        use serde::{Deserialize, Serialize};

        /// SQL statement AST
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum Statement {
            Select(SelectStatement),
            Insert(InsertStatement),
            Update(UpdateStatement),
            Delete(DeleteStatement),
            CreateTable(CreateTableStatement),
            DropTable(DropTableStatement),
            CreateIndex(CreateIndexStatement),
            DropIndex(DropIndexStatement),
        }

        /// SELECT statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct SelectStatement {
            pub columns: Vec<SelectColumn>,
            pub from: Option<FromClause>,
            pub where_clause: Option<Expression>,
            pub group_by: Option<Vec<Expression>>,
            pub having: Option<Expression>,
            pub order_by: Option<Vec<OrderByClause>>,
            pub limit: Option<LimitClause>,
        }

        impl std::fmt::Display for SelectStatement {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "SELECT ")?;

                if self.columns.is_empty() {
                    write!(f, "*")?;
                } else {
                    let column_strs: Vec<String> = self.columns.iter().map(|c| c.to_string()).collect();
                    write!(f, "{}", column_strs.join(", "))?;
                }

                if let Some(ref from) = self.from {
                    write!(f, " FROM {}", from)?;
                }

                if let Some(ref where_clause) = self.where_clause {
                    write!(f, " WHERE {}", where_clause)?;
                }

                if let Some(ref group_by) = self.group_by {
                    write!(f, " GROUP BY ")?;
                    let group_strs: Vec<String> = group_by.iter().map(|e| e.to_string()).collect();
                    write!(f, "{}", group_strs.join(", "))?;
                }

                if let Some(ref having) = self.having {
                    write!(f, " HAVING {}", having)?;
                }

                if let Some(ref order_by) = self.order_by {
                    write!(f, " ORDER BY ")?;
                    let order_strs: Vec<String> = order_by.iter().map(|o| o.to_string()).collect();
                    write!(f, "{}", order_strs.join(", "))?;
                }

                if let Some(ref limit) = self.limit {
                    write!(f, " LIMIT {}", limit.limit)?;
                    if let Some(offset) = limit.offset {
                        write!(f, " OFFSET {}", offset)?;
                    }
                }

                Ok(())
            }
        }

        /// SELECT column
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum SelectColumn {
            Wildcard,
            Expression { expr: Expression, alias: Option<String> },
        }

        impl std::fmt::Display for SelectColumn {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    SelectColumn::Wildcard => write!(f, "*"),
                    SelectColumn::Expression { expr, alias } => {
                        if let Some(alias) = alias {
                            write!(f, "{} AS {}", expr, alias)
                        } else {
                            write!(f, "{}", expr)
                        }
                    }
                }
            }
        }

        /// FROM clause
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct FromClause {
            pub source: FromSource,
            pub alias: Option<String>,
            pub joins: Vec<JoinClause>,
        }

        /// Source for FROM clause - can be table or subquery
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum FromSource {
            Table(String),
            Subquery(Box<SelectStatement>),
        }

        impl FromClause {
            /// Create a FROM clause from a table name
            pub fn from_table(table: String) -> Self {
                Self {
                    source: FromSource::Table(table),
                    alias: None,
                    joins: Vec::new(),
                }
            }

            /// Create a FROM clause from a subquery
            pub fn from_subquery(subquery: SelectStatement) -> Self {
                Self {
                    source: FromSource::Subquery(Box::new(subquery)),
                    alias: None,
                    joins: Vec::new(),
                }
            }

            /// Add an alias to the FROM clause
            pub fn with_alias(mut self, alias: String) -> Self {
                self.alias = Some(alias);
                self
            }

            /// Get the table name for backward compatibility
            pub fn table(&self) -> String {
                match &self.source {
                    FromSource::Table(name) => name.clone(),
                    FromSource::Subquery(_) => "subquery".to_string(),
                }
            }
        }

        impl std::fmt::Display for FromClause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match &self.source {
                    FromSource::Table(table) => {
                        if let Some(alias) = &self.alias {
                            write!(f, "{} AS {}", table, alias)?;
                        } else {
                            write!(f, "{}", table)?;
                        }
                    },
                    FromSource::Subquery(subquery) => {
                        if let Some(alias) = &self.alias {
                            write!(f, "({}) AS {}", subquery, alias)?;
                        } else {
                            write!(f, "({})", subquery)?;
                        }
                    }
                }

                for join in &self.joins {
                    write!(f, " {}", join)?;
                }

                Ok(())
            }
        }

        /// JOIN clause
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct JoinClause {
            pub join_type: JoinType,
            pub table: String,
            pub alias: Option<String>,
            pub on: Option<Expression>,
        }

        impl std::fmt::Display for JoinClause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} JOIN {}", self.join_type, self.table)?;
                if let Some(alias) = &self.alias {
                    write!(f, " AS {}", alias)?;
                }
                if let Some(on) = &self.on {
                    write!(f, " ON {}", on)?;
                }
                Ok(())
            }
        }

        /// JOIN type
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum JoinType {
            Inner,
            Left,
            Right,
            Full,
        }

        impl std::fmt::Display for JoinType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    JoinType::Inner => write!(f, "INNER"),
                    JoinType::Left => write!(f, "LEFT"),
                    JoinType::Right => write!(f, "RIGHT"),
                    JoinType::Full => write!(f, "FULL"),
                }
            }
        }

        /// Expression
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum Expression {
            Literal(Value),
            Column(String),
            QualifiedColumn { table: String, column: String },
            Binary { left: Box<Expression>, op: BinaryOp, right: Box<Expression> },
            Function { name: String, args: Vec<Expression> },
            IsNull(Box<Expression>),
            IsNotNull(Box<Expression>),
            In { expr: Box<Expression>, values: Vec<Expression> },
            Between { expr: Box<Expression>, low: Box<Expression>, high: Box<Expression> },
            /// Subquery expression - can be used in SELECT, WHERE, FROM, etc.
            Subquery(Box<SelectStatement>),
            /// EXISTS subquery
            Exists(Box<SelectStatement>),
            /// NOT EXISTS subquery
            NotExists(Box<SelectStatement>),
            /// IN subquery
            InSubquery { expr: Box<Expression>, subquery: Box<SelectStatement> },
            /// NOT IN subquery
            NotInSubquery { expr: Box<Expression>, subquery: Box<SelectStatement> },
            /// Scalar subquery (returns single value)
            ScalarSubquery(Box<SelectStatement>),
            /// CASE expression
            Case {
                expr: Option<Box<Expression>>,
                when_clauses: Vec<WhenClause>,
                else_clause: Option<Box<Expression>>,
            },
        }

        /// WHEN clause for CASE expressions
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct WhenClause {
            pub condition: Expression,
            pub result: Expression,
        }

        impl std::fmt::Display for Expression {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Expression::Literal(value) => write!(f, "{}", value),
                    Expression::Column(name) => write!(f, "{}", name),
                    Expression::QualifiedColumn { table, column } => write!(f, "{}.{}", table, column),
                    Expression::Binary { left, op, right } => write!(f, "({} {} {})", left, op, right),
                    Expression::Function { name, args } => {
                        write!(f, "{}(", name)?;
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 { write!(f, ", ")?; }
                            write!(f, "{}", arg)?;
                        }
                        write!(f, ")")
                    },
                    Expression::IsNull(expr) => write!(f, "{} IS NULL", expr),
                    Expression::IsNotNull(expr) => write!(f, "{} IS NOT NULL", expr),
                    Expression::In { expr, values } => {
                        write!(f, "{} IN (", expr)?;
                        for (i, value) in values.iter().enumerate() {
                            if i > 0 { write!(f, ", ")?; }
                            write!(f, "{}", value)?;
                        }
                        write!(f, ")")
                    },
                    Expression::Between { expr, low, high } => {
                        write!(f, "{} BETWEEN {} AND {}", expr, low, high)
                    },
                    Expression::Subquery(stmt) => write!(f, "({})", stmt),
                    Expression::Exists(stmt) => write!(f, "EXISTS ({})", stmt),
                    Expression::NotExists(stmt) => write!(f, "NOT EXISTS ({})", stmt),
                    Expression::InSubquery { expr, subquery } => write!(f, "{} IN ({})", expr, subquery),
                    Expression::NotInSubquery { expr, subquery } => write!(f, "{} NOT IN ({})", expr, subquery),
                    Expression::ScalarSubquery(stmt) => write!(f, "({})", stmt),
                    Expression::Case { expr, when_clauses, else_clause } => {
                        write!(f, "CASE")?;
                        if let Some(case_expr) = expr {
                            write!(f, " {}", case_expr)?;
                        }
                        for when_clause in when_clauses {
                            write!(f, " WHEN {} THEN {}", when_clause.condition, when_clause.result)?;
                        }
                        if let Some(else_expr) = else_clause {
                            write!(f, " ELSE {}", else_expr)?;
                        }
                        write!(f, " END")
                    },
                }
            }
        }

        /// Binary operator
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum BinaryOp {
            Eq, Ne, Lt, Le, Gt, Ge,
            And, Or,
            Add, Sub, Mul, Div, Mod,
            Like, NotLike,
        }

        impl std::fmt::Display for BinaryOp {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    BinaryOp::Eq => write!(f, "="),
                    BinaryOp::Ne => write!(f, "!="),
                    BinaryOp::Lt => write!(f, "<"),
                    BinaryOp::Le => write!(f, "<="),
                    BinaryOp::Gt => write!(f, ">"),
                    BinaryOp::Ge => write!(f, ">="),
                    BinaryOp::And => write!(f, "AND"),
                    BinaryOp::Or => write!(f, "OR"),
                    BinaryOp::Add => write!(f, "+"),
                    BinaryOp::Sub => write!(f, "-"),
                    BinaryOp::Mul => write!(f, "*"),
                    BinaryOp::Div => write!(f, "/"),
                    BinaryOp::Mod => write!(f, "%"),
                    BinaryOp::Like => write!(f, "LIKE"),
                    BinaryOp::NotLike => write!(f, "NOT LIKE"),
                }
            }
        }

        /// ORDER BY clause
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct OrderByClause {
            pub expr: Expression,
            pub direction: OrderDirection,
        }

        /// Order direction
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum OrderDirection {
            Asc,
            Desc,
        }

        impl std::fmt::Display for OrderByClause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} {}", self.expr, match self.direction {
                    OrderDirection::Asc => "ASC",
                    OrderDirection::Desc => "DESC",
                })
            }
        }

        /// LIMIT clause
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct LimitClause {
            pub limit: u64,
            pub offset: Option<u64>,
        }

        /// INSERT statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct InsertStatement {
            pub table: String,
            pub columns: Vec<String>,
            pub values: Vec<Vec<Value>>,
            pub on_conflict: Option<String>,
        }

        /// UPDATE statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct UpdateStatement {
            pub table: String,
            pub set_clauses: Vec<(String, Value)>,
            pub where_clause: Option<Expression>,
        }

        /// DELETE statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct DeleteStatement {
            pub table: String,
            pub where_clause: Option<Expression>,
        }

        /// CREATE TABLE statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct CreateTableStatement {
            pub table_name: String,
            pub columns: Vec<ColumnDefinition>,
            pub constraints: Vec<TableConstraint>,
        }

        /// Column definition
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct ColumnDefinition {
            pub name: String,
            pub data_type: DataType,
            pub constraints: Vec<ColumnConstraint>,
        }

        /// Column constraint
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum ColumnConstraint {
            NotNull,
            PrimaryKey,
            Unique,
            Default(Value),
            ForeignKey { table: String, column: String },
        }

        /// Table constraint
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum TableConstraint {
            PrimaryKey(Vec<String>),
            Unique(Vec<String>),
            ForeignKey { columns: Vec<String>, ref_table: String, ref_columns: Vec<String> },
            Check(Expression),
        }

        /// DROP TABLE statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct DropTableStatement {
            pub table_name: String,
            pub if_exists: bool,
        }

        /// CREATE INDEX statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct CreateIndexStatement {
            pub index_name: String,
            pub table: String,
            pub columns: Vec<String>,
            pub unique: bool,
        }

        /// DROP INDEX statement
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct DropIndexStatement {
            pub index_name: String,
            pub if_exists: bool,
        }
    }

    // Re-export main items
    pub use ast::*;

    // Create a wrapper function that uses the real parser
    pub fn parse_sql(input: &str) -> SqlResult<Statement> {

        // For now, implement a simple INSERT parser
        if input.trim().to_uppercase().starts_with("INSERT") {
            parse_insert_statement(input.trim())
        } else if input.trim().to_uppercase().starts_with("SELECT") {
            parse_select_statement(input.trim())
        } else {
            Err(SqlError::parse_error(format!("Unsupported SQL statement: {}", input)))
        }
    }

    fn parse_insert_statement(input: &str) -> SqlResult<Statement> {
        // Simple INSERT parser
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() < 4 || parts[0].to_uppercase() != "INSERT" || parts[1].to_uppercase() != "INTO" {
            return Err(SqlError::parse_error("Invalid INSERT syntax".to_string()));
        }

        let table = parts[2].to_string();

        // Extract columns from parentheses
        let columns_start = input.find('(').ok_or_else(|| SqlError::parse_error("Missing column list".to_string()))?;
        let columns_end = input.find(')').ok_or_else(|| SqlError::parse_error("Missing closing parenthesis".to_string()))?;
        let columns_str = &input[columns_start + 1..columns_end];
        let columns: Vec<String> = columns_str.split(',').map(|s| s.trim().to_string()).collect();

        // For now, create a simple INSERT statement
        Ok(Statement::Insert(ast::InsertStatement {
            table,
            columns,
            values: vec![], // We'll parse values later if needed
            on_conflict: None,
        }))
    }

    fn parse_select_statement(input: &str) -> SqlResult<Statement> {
        // Simple SELECT parser
        let input = input.trim();

        // Find SELECT keyword
        if !input.to_uppercase().starts_with("SELECT") {
            return Err(SqlError::parse_error("Invalid SELECT syntax".to_string()));
        }

        // Extract columns part (between SELECT and FROM)
        let from_pos = input.to_uppercase().find(" FROM ").ok_or_else(|| SqlError::parse_error("Missing FROM clause".to_string()))?;
        let columns_str = &input[6..from_pos].trim(); // Skip "SELECT"

        // Parse columns
        let columns = if *columns_str == "*" {
            vec![ast::SelectColumn::Wildcard]
        } else {
            columns_str.split(',')
                .map(|col| ast::SelectColumn::Expression {
                    expr: ast::Expression::Column(col.trim().to_string()),
                    alias: None
                })
                .collect()
        };

        // Extract table name (after FROM)
        let after_from = &input[from_pos + 6..].trim(); // Skip " FROM "
        let table_end = after_from.find(' ').unwrap_or(after_from.len());
        let table = after_from[..table_end].to_string();

        Ok(Statement::Select(ast::SelectStatement {
            columns,
            from: Some(ast::FromClause {
                source: ast::FromSource::Table(table),
                alias: None,
                joins: vec![],
            }),
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
        }))
    }
}

// Re-export convenience items
pub use self::parser::parse_sql;
pub use self::types::{Value, DataType};
pub use self::error::{SqlError, SqlResult};
