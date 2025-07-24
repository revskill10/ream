/// Catena-GraphQL-X: Extensible, Multi-Root, Algebraic GraphQL → SQL Compiler
/// 
/// This module implements an extensible GraphQL to SQL compiler based on algebraic
/// data types and category theory. It supports:
/// - Multi-root GraphQL queries compiled to single SQL statements with CTEs
/// - Extensible SQL expression algebra with custom functions and aggregations
/// - Type-safe integration with the existing TypeSafeSchema system
/// - Both Rust macro and TLisp library interfaces

use std::collections::HashMap;
use std::marker::PhantomData;
use serde_json::Value as JsonValue;
use crate::orm::schema::{TypeSafeSchema, Column, Table};
use crate::orm::query::{QueryBuilder, SelectQueryBuilder};
use crate::orm::QueryRow;
use crate::sqlite::types::{DataType, Value};
use crate::sqlite::error::{SqlError, SqlResult};

/// Type tags for compile-time type safety in SQL expressions
#[derive(Debug, Clone, PartialEq)]
pub enum TypeTag {
    Text,
    Integer,
    Real,
    Boolean,
    Json,
    Array(Box<TypeTag>),
}

/// Universal SQL expression algebra with open extensibility
#[derive(Debug, Clone)]
pub enum SqlExprF<A> {
    /// Column reference
    Column {
        name: String,
        table_alias: Option<String>,
        next: A,
    },
    
    /// Literal value
    Literal {
        value: JsonValue,
        ty: TypeTag,
        next: A,
    },
    
    /// Custom SQL fragment (open plug-in point)
    Custom {
        sql: String,
        binds: Vec<Value>,
        ty: TypeTag,
        next: A,
    },
    
    /// Aggregation functions
    Agg {
        func: String,  // jsonb_agg, array_agg, count, sum, etc.
        inner: Box<SqlExprF<A>>,
        filter: Option<Box<SqlExprF<A>>>,
        next: A,
    },
    
    /// Function calls
    FnCall {
        name: String,
        args: Vec<SqlExprF<A>>,
        next: A,
    },
    
    /// Binary operations
    Binary {
        op: BinaryOp,
        left: Box<SqlExprF<A>>,
        right: Box<SqlExprF<A>>,
        next: A,
    },
}

/// Binary operators for SQL expressions
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Concat,
}

/// Fixed point of SqlExprF for recursive expressions
pub type SqlExpr = SqlExprF<()>;

/// Trait for building custom SQL expressions at compile time
pub trait SqlBuilder: Send + Sync {
    /// Build SQL string and parameter bindings
    fn build(&self) -> (String, Vec<Value>);
    
    /// Get the return type of this SQL expression
    fn return_type(&self) -> TypeTag;
}

/// Multi-root GraphQL query algebra
#[derive(Debug, Clone)]
pub enum MultiRootF<A> {
    /// Single root query
    Root {
        name: String,
        selection: SelectionSet,
        args: HashMap<String, JsonValue>,
        next: A,
    },
    
    /// Combine multiple roots
    Combine {
        left: Box<MultiRootF<A>>,
        right: Box<MultiRootF<A>>,
        next: A,
    },
}

/// Fixed point for multi-root queries
pub type MultiRoot = MultiRootF<()>;

/// GraphQL selection set
#[derive(Debug, Clone)]
pub struct SelectionSet {
    pub fields: Vec<Field>,
}

/// GraphQL field
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub alias: Option<String>,
    pub args: HashMap<String, JsonValue>,
    pub selection_set: Option<SelectionSet>,
    pub custom_expr: Option<SqlExpr>,
}

/// Relational plan for a single root query
#[derive(Debug, Clone)]
pub struct RelationalPlan {
    pub root_name: String,
    pub base_table: String,
    pub columns: Vec<ColumnPlan>,
    pub where_clause: Option<SqlExpr>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub subqueries: Vec<SubqueryPlan>,
}

/// Column selection plan
#[derive(Debug, Clone)]
pub struct ColumnPlan {
    pub field_name: String,
    pub sql_expr: SqlExpr,
    pub alias: Option<String>,
}

/// Subquery plan for nested selections
#[derive(Debug, Clone)]
pub struct SubqueryPlan {
    pub field_name: String,
    pub query: RelationalPlan,
    pub join_condition: SqlExpr,
    pub aggregation: Option<String>,
}

/// Compiled SQL with CTEs for multi-root execution
#[derive(Debug, Clone)]
pub struct CompiledSql {
    pub sql: String,
    pub binds: Vec<Value>,
    pub return_type: TypeTag,
}

/// Global registry for custom SQL builders
pub struct SqlRegistry {
    builders: HashMap<String, Box<dyn SqlBuilder>>,
}

impl SqlRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }
    
    /// Register a custom SQL builder
    pub fn register<B: SqlBuilder + 'static>(&mut self, name: String, builder: B) {
        self.builders.insert(name, Box::new(builder));
    }
    
    /// Get a registered builder
    pub fn get(&self, name: &str) -> Option<&dyn SqlBuilder> {
        self.builders.get(name).map(|b| b.as_ref())
    }
}

/// Main GraphQL compiler context
pub struct GraphQLCompiler {
    pub schema: TypeSafeSchema,
    registry: SqlRegistry,
}

impl GraphQLCompiler {
    /// Create a new GraphQL compiler with the given schema
    pub fn new(schema: TypeSafeSchema) -> Self {
        Self {
            schema,
            registry: SqlRegistry::new(),
        }
    }
    
    /// Register a custom SQL builder
    pub fn register_builder<B: SqlBuilder + 'static>(&mut self, name: String, builder: B) {
        self.registry.register(name, builder);
    }
    
    /// Compile a multi-root GraphQL query to SQL
    pub fn compile(&self, query: MultiRoot) -> SqlResult<CompiledSql> {
        // Step 1: Convert multi-root query to relational plans
        let plans = self.multi_root_to_plans(query)?;
        
        // Step 2: Generate SQL with CTEs
        self.plans_to_sql(plans)
    }
    
    /// Convert multi-root query to relational plans (catamorphism 1)
    fn multi_root_to_plans(&self, query: MultiRoot) -> SqlResult<Vec<RelationalPlan>> {
        match query {
            MultiRootF::Root { name, selection, args, .. } => {
                let plan = self.selection_to_plan(name, selection, args)?;
                Ok(vec![plan])
            },
            MultiRootF::Combine { left, right, .. } => {
                let mut left_plans = self.multi_root_to_plans(*left)?;
                let mut right_plans = self.multi_root_to_plans(*right)?;
                left_plans.append(&mut right_plans);
                Ok(left_plans)
            },
        }
    }
    
    /// Convert selection set to relational plan
    fn selection_to_plan(
        &self, 
        root_name: String, 
        selection: SelectionSet, 
        args: HashMap<String, JsonValue>
    ) -> SqlResult<RelationalPlan> {
        // This is a simplified implementation - in practice would need
        // sophisticated schema introspection and query planning
        
        let base_table = self.infer_table_from_root(&root_name)?;
        let mut columns = Vec::new();
        let mut subqueries = Vec::new();
        
        for field in selection.fields {
            if let Some(nested_selection) = field.selection_set {
                // This is a nested field - create a subquery
                let subquery_plan = self.selection_to_plan(
                    field.name.clone(),
                    nested_selection,
                    field.args.clone()
                )?;
                
                let field_name = field.name.clone();
                subqueries.push(SubqueryPlan {
                    field_name: field.name,
                    query: subquery_plan,
                    join_condition: self.infer_join_condition(&base_table, &field_name)?,
                    aggregation: Some("jsonb_agg".to_string()),
                });
            } else {
                // Simple field - add to column list
                let sql_expr = if let Some(custom) = field.custom_expr {
                    custom
                } else {
                    SqlExprF::Column {
                        name: field.name.clone(),
                        table_alias: None,
                        next: (),
                    }
                };
                
                columns.push(ColumnPlan {
                    field_name: field.name,
                    sql_expr,
                    alias: field.alias,
                });
            }
        }
        
        Ok(RelationalPlan {
            root_name,
            base_table,
            columns,
            where_clause: None,
            limit: args.get("limit").and_then(|v| v.as_u64()),
            offset: args.get("offset").and_then(|v| v.as_u64()),
            subqueries,
        })
    }
    
    /// Generate SQL with CTEs from relational plans (catamorphism 2)
    pub fn plans_to_sql(&self, plans: Vec<RelationalPlan>) -> SqlResult<CompiledSql> {
        let mut cte_parts = Vec::new();
        let mut final_select_parts = Vec::new();
        let mut all_binds = Vec::new();
        
        for plan in plans {
            let (cte_sql, binds) = self.plan_to_cte_sql(&plan)?;
            cte_parts.push(format!("{} AS ({})", plan.root_name, cte_sql));
            all_binds.extend(binds);
            
            final_select_parts.push(format!(
                "'{}', (SELECT jsonb_agg(t.*) FROM {} t)",
                plan.root_name, plan.root_name
            ));
        }
        
        let sql = if cte_parts.is_empty() {
            "SELECT '{}'::jsonb AS data".to_string()
        } else {
            format!(
                "WITH {} SELECT jsonb_build_object({}) AS data",
                cte_parts.join(", "),
                final_select_parts.join(", ")
            )
        };
        
        Ok(CompiledSql {
            sql,
            binds: all_binds,
            return_type: TypeTag::Json,
        })
    }
    
    /// Convert a single relational plan to CTE SQL
    fn plan_to_cte_sql(&self, plan: &RelationalPlan) -> SqlResult<(String, Vec<Value>)> {
        let mut sql = String::from("SELECT ");
        let mut binds = Vec::new();
        
        // Add columns
        let column_parts: Vec<String> = plan.columns.iter().map(|col| {
            let expr_sql = self.sql_expr_to_string(&col.sql_expr);
            if let Some(alias) = &col.alias {
                format!("{} AS {}", expr_sql, alias)
            } else {
                format!("{} AS {}", expr_sql, col.field_name)
            }
        }).collect();
        
        sql.push_str(&column_parts.join(", "));
        
        // Add subqueries as additional columns
        for subquery in &plan.subqueries {
            let (subquery_sql, sub_binds) = self.plan_to_cte_sql(&subquery.query)?;
            binds.extend(sub_binds);
            
            sql.push_str(&format!(
                ", (SELECT coalesce({}(sub.*) FILTER (WHERE sub.id IS NOT NULL), '[]') FROM ({}) sub WHERE {}) AS {}",
                subquery.aggregation.as_ref().unwrap_or(&"jsonb_agg".to_string()),
                subquery_sql,
                self.sql_expr_to_string(&subquery.join_condition),
                subquery.field_name
            ));
        }
        
        sql.push_str(&format!(" FROM {}", plan.base_table));
        
        if let Some(where_clause) = &plan.where_clause {
            sql.push_str(&format!(" WHERE {}", self.sql_expr_to_string(where_clause)));
        }
        
        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = plan.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
        
        Ok((sql, binds))
    }
    
    /// Convert SQL expression to string
    fn sql_expr_to_string(&self, expr: &SqlExpr) -> String {
        match expr {
            SqlExprF::Column { name, table_alias, .. } => {
                if let Some(alias) = table_alias {
                    format!("{}.{}", alias, name)
                } else {
                    name.clone()
                }
            },
            SqlExprF::Literal { value, .. } => {
                // Convert JSON value to SQL literal
                match value {
                    JsonValue::String(s) => format!("'{}'", s.replace("'", "''")),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => b.to_string(),
                    JsonValue::Null => "NULL".to_string(),
                    _ => format!("'{}'", value.to_string().replace("'", "''")),
                }
            },
            SqlExprF::Custom { sql, .. } => sql.clone(),
            SqlExprF::Agg { func, inner, filter, .. } => {
                let inner_sql = self.sql_expr_to_string(inner);
                let mut result = format!("{}({})", func, inner_sql);
                if let Some(filter_expr) = filter {
                    result.push_str(&format!(" FILTER (WHERE {})", self.sql_expr_to_string(filter_expr)));
                }
                result
            },
            SqlExprF::FnCall { name, args, .. } => {
                let arg_strs: Vec<String> = args.iter().map(|arg| self.sql_expr_to_string(arg)).collect();
                format!("{}({})", name, arg_strs.join(", "))
            },
            SqlExprF::Binary { op, left, right, .. } => {
                let op_str = match op {
                    BinaryOp::Eq => "=",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Le => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "AND",
                    BinaryOp::Or => "OR",
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Concat => "||",
                };
                format!("({} {} {})", 
                    self.sql_expr_to_string(left), 
                    op_str, 
                    self.sql_expr_to_string(right)
                )
            },
        }
    }
    
    /// Infer table name from GraphQL root field name
    pub fn infer_table_from_root(&self, root_name: &str) -> SqlResult<String> {
        // Simple mapping - in practice would use schema introspection
        match root_name {
            "users" => Ok("users".to_string()),
            "posts" => Ok("posts".to_string()),
            "departments" => Ok("departments".to_string()),
            _ => Err(SqlError::runtime_error(&format!("Unknown root field: {}", root_name))),
        }
    }
    
    /// Infer join condition between tables
    pub fn infer_join_condition(&self, base_table: &str, field_name: &str) -> SqlResult<SqlExpr> {
        // Simple foreign key inference - in practice would use schema metadata
        let condition = match (base_table, field_name) {
            ("users", "posts") => "posts.user_id = users.id",
            ("posts", "categories") => "post_categories.post_id = posts.id AND post_categories.category_id = categories.id",
            _ => return Err(SqlError::runtime_error(&format!("Cannot infer join for {}.{}", base_table, field_name))),
        };

        Ok(SqlExprF::Custom {
            sql: condition.to_string(),
            binds: Vec::new(),
            ty: TypeTag::Boolean,
            next: (),
        })
    }
}

/// Builder for multi-root GraphQL queries
pub struct MultiRootBuilder {
    roots: Vec<(String, SelectionSet, HashMap<String, JsonValue>)>,
}

impl MultiRootBuilder {
    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
        }
    }

    /// Add a root query
    pub fn add_root(mut self, name: String, selection: SelectionSet, args: HashMap<String, JsonValue>) -> Self {
        self.roots.push((name, selection, args));
        self
    }

    /// Build the multi-root query
    pub fn build(self) -> MultiRoot {
        if self.roots.is_empty() {
            // Return empty root
            return MultiRootF::Root {
                name: "empty".to_string(),
                selection: SelectionSet { fields: Vec::new() },
                args: HashMap::new(),
                next: (),
            };
        }

        let mut result = None;

        for (name, selection, args) in self.roots {
            let root = MultiRootF::Root {
                name,
                selection,
                args,
                next: (),
            };

            result = Some(match result {
                None => root,
                Some(existing) => MultiRootF::Combine {
                    left: Box::new(existing),
                    right: Box::new(root),
                    next: (),
                },
            });
        }

        result.unwrap()
    }
}

/// Builder for GraphQL selection sets
pub struct SelectionSetBuilder {
    fields: Vec<Field>,
}

impl SelectionSetBuilder {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
        }
    }

    /// Add a simple field
    pub fn field(mut self, name: String) -> Self {
        self.fields.push(Field {
            name,
            alias: None,
            args: HashMap::new(),
            selection_set: None,
            custom_expr: None,
        });
        self
    }

    /// Add a field with alias
    pub fn field_as(mut self, name: String, alias: String) -> Self {
        self.fields.push(Field {
            name,
            alias: Some(alias),
            args: HashMap::new(),
            selection_set: None,
            custom_expr: None,
        });
        self
    }

    /// Add a field with arguments
    pub fn field_with_args(mut self, name: String, args: HashMap<String, JsonValue>) -> Self {
        self.fields.push(Field {
            name,
            alias: None,
            args,
            selection_set: None,
            custom_expr: None,
        });
        self
    }

    /// Add a field with nested selection
    pub fn field_with_selection(mut self, name: String, selection: SelectionSet) -> Self {
        self.fields.push(Field {
            name,
            alias: None,
            args: HashMap::new(),
            selection_set: Some(selection),
            custom_expr: None,
        });
        self
    }

    /// Add a field with custom SQL expression
    pub fn field_with_custom_expr(mut self, name: String, expr: SqlExpr) -> Self {
        self.fields.push(Field {
            name,
            alias: None,
            args: HashMap::new(),
            selection_set: None,
            custom_expr: Some(expr),
        });
        self
    }

    /// Build the selection set
    pub fn build(self) -> SelectionSet {
        SelectionSet {
            fields: self.fields,
        }
    }
}

/// Helper functions for building SQL expressions
impl SqlExpr {
    /// Create a column reference
    pub fn column(name: String) -> Self {
        SqlExprF::Column {
            name,
            table_alias: None,
            next: (),
        }
    }

    /// Create a column reference with table alias
    pub fn column_with_table(name: String, table_alias: String) -> Self {
        SqlExprF::Column {
            name,
            table_alias: Some(table_alias),
            next: (),
        }
    }

    /// Create a literal value
    pub fn literal(value: JsonValue, ty: TypeTag) -> Self {
        SqlExprF::Literal {
            value,
            ty,
            next: (),
        }
    }

    /// Create a custom SQL expression
    pub fn custom(sql: String, binds: Vec<Value>, ty: TypeTag) -> Self {
        SqlExprF::Custom {
            sql,
            binds,
            ty,
            next: (),
        }
    }

    /// Create an aggregation
    pub fn agg(func: String, inner: SqlExpr) -> Self {
        SqlExprF::Agg {
            func,
            inner: Box::new(inner),
            filter: None,
            next: (),
        }
    }

    /// Create an aggregation with filter
    pub fn agg_with_filter(func: String, inner: SqlExpr, filter: SqlExpr) -> Self {
        SqlExprF::Agg {
            func,
            inner: Box::new(inner),
            filter: Some(Box::new(filter)),
            next: (),
        }
    }

    /// Create a function call
    pub fn function(name: String, args: Vec<SqlExpr>) -> Self {
        SqlExprF::FnCall {
            name,
            args,
            next: (),
        }
    }

    /// Create a binary operation
    pub fn binary(op: BinaryOp, left: SqlExpr, right: SqlExpr) -> Self {
        SqlExprF::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
            next: (),
        }
    }
}
