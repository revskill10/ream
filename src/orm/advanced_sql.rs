/// Advanced SQL patterns and driver plugins
/// 
/// This module provides support for advanced SQL features like:
/// - Common Table Expressions (CTEs)
/// - Recursive queries
/// - Window functions
/// - CASE expressions
/// - Nested relations
/// - Aggregations
/// - JSON operations
/// - Full-text search

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::sqlite::types::Value;
use crate::sqlite::parser::ast::Expression;
use crate::orm::{SqlResult, Driver};
use crate::orm::schema::Column;

/// Common Table Expression (CTE) definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CteDefinition {
    pub name: String,
    pub columns: Option<Vec<String>>,
    pub query: String,
    pub recursive: bool,
}

impl CteDefinition {
    /// Create a new CTE definition with type-safe columns
    pub fn new_with_columns(
        name: impl Into<String>,
        columns: &[&Column],
        query: impl Into<String>,
        recursive: bool,
    ) -> Self {
        Self {
            name: name.into(),
            columns: Some(columns.iter().map(|col| col.name.clone()).collect()),
            query: query.into(),
            recursive,
        }
    }

    /// Create a new CTE definition with qualified column names
    pub fn new_with_qualified_columns(
        name: impl Into<String>,
        columns: &[&Column],
        query: impl Into<String>,
        recursive: bool,
    ) -> Self {
        Self {
            name: name.into(),
            columns: Some(columns.iter().map(|col| col.qualified_name()).collect()),
            query: query.into(),
            recursive,
        }
    }

    /// Create a new CTE definition with string columns (for backward compatibility)
    pub fn new_with_string_columns(
        name: impl Into<String>,
        columns: Vec<String>,
        query: impl Into<String>,
        recursive: bool,
    ) -> Self {
        Self {
            name: name.into(),
            columns: Some(columns),
            query: query.into(),
            recursive,
        }
    }

    /// Create a new CTE definition without explicit columns
    pub fn new(
        name: impl Into<String>,
        query: impl Into<String>,
        recursive: bool,
    ) -> Self {
        Self {
            name: name.into(),
            columns: None,
            query: query.into(),
            recursive,
        }
    }
}

/// Window function specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowFunction {
    pub function: WindowFunctionType,
    pub partition_by: Vec<String>,
    pub order_by: Vec<OrderByClause>,
    pub frame: Option<WindowFrame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowFunctionType {
    RowNumber,
    Rank,
    DenseRank,
    Lag { offset: i32, default: Option<Value> },
    Lead { offset: i32, default: Option<Value> },
    FirstValue(String),
    LastValue(String),
    NthValue { column: String, n: i32 },
    Sum(String),
    Avg(String),
    Count(String),
    Min(String),
    Max(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByClause {
    pub column: String,
    pub direction: OrderDirection,
    pub nulls: Option<NullsOrder>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NullsOrder {
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowFrame {
    pub frame_type: FrameType,
    pub start: FrameBound,
    pub end: Option<FrameBound>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FrameType {
    Rows,
    Range,
    Groups,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(i32),
    CurrentRow,
    Following(i32),
    UnboundedFollowing,
}

/// CASE expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseExpression {
    pub case_type: CaseType,
    pub when_clauses: Vec<WhenClause>,
    pub else_clause: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CaseType {
    Simple(Expression), // CASE expr WHEN ...
    Searched,           // CASE WHEN condition ...
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhenClause {
    pub condition: Expression,
    pub result: Expression,
}

/// JSON operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonOperation {
    pub operation_type: JsonOperationType,
    pub path: String,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsonOperationType {
    Extract,        // JSON_EXTRACT
    Set,           // JSON_SET
    Insert,        // JSON_INSERT
    Replace,       // JSON_REPLACE
    Remove,        // JSON_REMOVE
    ArrayLength,   // JSON_ARRAY_LENGTH
    Valid,         // JSON_VALID
    Type,          // JSON_TYPE
}

/// Aggregation with advanced features
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedAggregation {
    pub function: AggregateFunction,
    pub distinct: bool,
    pub filter: Option<Expression>,
    pub over: Option<WindowFunction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count(Option<String>),
    Sum(String),
    Avg(String),
    Min(String),
    Max(String),
    GroupConcat { column: String, separator: Option<String> },
    StringAgg { column: String, separator: String },
    ArrayAgg(String),
    JsonArrayAgg(String),
    JsonObjectAgg { key: String, value: String },
    Percentile { column: String, percentile: f64 },
    StdDev(String),
    Variance(String),
}

/// Full-text search configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullTextSearch {
    pub table: String,
    pub columns: Vec<String>,
    pub query: String,
    pub options: FtsOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FtsOptions {
    pub tokenizer: Option<String>,
    pub language: Option<String>,
    pub stemming: bool,
    pub stop_words: bool,
    pub ranking: Option<RankingFunction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RankingFunction {
    Bm25,
    TfIdf,
    Simple,
}

/// Advanced SQL builder trait
pub trait AdvancedSqlBuilder {
    /// Build a CTE query
    fn with_cte(&mut self, cte: CteDefinition) -> &mut Self;
    
    /// Add window function
    fn window(&mut self, alias: String, window: WindowFunction) -> &mut Self;
    
    /// Add CASE expression
    fn case_when(&mut self, case_expr: CaseExpression) -> &mut Self;
    
    /// Add JSON operation
    fn json_op(&mut self, column: String, operation: JsonOperation) -> &mut Self;
    
    /// Add advanced aggregation
    fn aggregate(&mut self, alias: String, agg: AdvancedAggregation) -> &mut Self;
    
    /// Add full-text search
    fn full_text_search(&mut self, fts: FullTextSearch) -> &mut Self;
    
    /// Build the final SQL
    fn build_advanced_sql(&self) -> String;
}

/// Database version information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: Option<u32>,
}

impl DatabaseVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch, build: None }
    }

    pub fn with_build(major: u32, minor: u32, patch: u32, build: u32) -> Self {
        Self { major, minor, patch, build: Some(build) }
    }

    pub fn is_at_least(&self, other: &DatabaseVersion) -> bool {
        if self.major != other.major {
            return self.major >= other.major;
        }
        if self.minor != other.minor {
            return self.minor >= other.minor;
        }
        if self.patch != other.patch {
            return self.patch >= other.patch;
        }
        match (self.build, other.build) {
            (Some(a), Some(b)) => a >= b,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
        }
    }
}

impl std::fmt::Display for DatabaseVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(build) = self.build {
            write!(f, "{}.{}.{}.{}", self.major, self.minor, self.patch, build)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

/// Feature support information with version requirements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureSupport {
    pub supported: bool,
    pub minimum_version: Option<DatabaseVersion>,
    pub notes: Option<String>,
}

impl FeatureSupport {
    pub fn supported() -> Self {
        Self { supported: true, minimum_version: None, notes: None }
    }

    pub fn supported_since(version: DatabaseVersion) -> Self {
        Self { supported: true, minimum_version: Some(version), notes: None }
    }

    pub fn supported_with_notes(notes: impl Into<String>) -> Self {
        Self { supported: true, minimum_version: None, notes: Some(notes.into()) }
    }

    pub fn not_supported() -> Self {
        Self { supported: false, minimum_version: None, notes: None }
    }

    pub fn not_supported_with_notes(notes: impl Into<String>) -> Self {
        Self { supported: false, minimum_version: None, notes: Some(notes.into()) }
    }
}

/// Driver plugin trait for advanced SQL features
pub trait AdvancedSqlPlugin {
    /// Get the database name and version
    fn database_info(&self) -> (String, DatabaseVersion);

    /// Check if the driver supports CTEs
    fn supports_cte(&self) -> FeatureSupport;

    /// Check if the driver supports recursive CTEs
    fn supports_recursive_cte(&self) -> FeatureSupport;

    /// Check if the driver supports window functions
    fn supports_window_functions(&self) -> FeatureSupport;

    /// Check if the driver supports JSON operations
    fn supports_json(&self) -> FeatureSupport;

    /// Check if the driver supports full-text search
    fn supports_full_text_search(&self) -> FeatureSupport;
    
    /// Generate CTE SQL for this driver
    fn generate_cte_sql(&self, cte: &CteDefinition) -> SqlResult<String>;
    
    /// Generate window function SQL for this driver
    fn generate_window_sql(&self, window: &WindowFunction) -> SqlResult<String>;
    
    /// Generate CASE expression SQL for this driver
    fn generate_case_sql(&self, case_expr: &CaseExpression) -> SqlResult<String>;
    
    /// Generate JSON operation SQL for this driver
    fn generate_json_sql(&self, operation: &JsonOperation) -> SqlResult<String>;
    
    /// Generate aggregation SQL for this driver
    fn generate_aggregate_sql(&self, agg: &AdvancedAggregation) -> SqlResult<String>;
    
    /// Generate full-text search SQL for this driver
    fn generate_fts_sql(&self, fts: &FullTextSearch) -> SqlResult<String>;
}

/// Registry for advanced SQL plugins
pub struct AdvancedSqlRegistry {
    plugins: HashMap<String, Box<dyn AdvancedSqlPlugin + Send + Sync>>,
}

impl AdvancedSqlRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    pub fn register_plugin(&mut self, driver_name: String, plugin: Box<dyn AdvancedSqlPlugin + Send + Sync>) {
        self.plugins.insert(driver_name, plugin);
    }
    
    pub fn get_plugin(&self, driver_name: &str) -> Option<&(dyn AdvancedSqlPlugin + Send + Sync)> {
        self.plugins.get(driver_name).map(|p| p.as_ref())
    }
}

/// Nested relation support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NestedRelation {
    pub parent_table: String,
    pub child_table: String,
    pub relation_type: RelationType,
    pub foreign_key: String,
    pub parent_key: String,
    pub nested_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationType {
    OneToOne,
    OneToMany,
    ManyToMany { junction_table: String },
}

/// Query optimizer hints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryHints {
    pub use_index: Option<String>,
    pub force_index: Option<String>,
    pub ignore_index: Option<String>,
    pub parallel_degree: Option<u32>,
    pub timeout: Option<u32>,
}
