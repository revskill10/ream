/// PostgreSQL-specific advanced SQL plugin implementation
use crate::orm::advanced_sql::*;
use crate::orm::{SqlResult, SqlError};
use crate::sqlite::types::Value;

/// PostgreSQL advanced SQL plugin
pub struct PostgresAdvancedPlugin;

impl PostgresAdvancedPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl AdvancedSqlPlugin for PostgresAdvancedPlugin {
    fn database_info(&self) -> (String, DatabaseVersion) {
        ("PostgreSQL".to_string(), DatabaseVersion::new(16, 1, 0))
    }

    fn supports_cte(&self) -> FeatureSupport {
        FeatureSupport::supported_since(DatabaseVersion::new(8, 4, 0))
    }

    fn supports_recursive_cte(&self) -> FeatureSupport {
        FeatureSupport::supported_since(DatabaseVersion::new(8, 4, 0))
    }

    fn supports_window_functions(&self) -> FeatureSupport {
        FeatureSupport::supported_since(DatabaseVersion::new(8, 4, 0))
    }

    fn supports_json(&self) -> FeatureSupport {
        FeatureSupport::supported_with_notes(
            "JSON since 9.2, JSONB since 9.4 (recommended)"
        )
    }

    fn supports_full_text_search(&self) -> FeatureSupport {
        FeatureSupport::supported_since(DatabaseVersion::new(8, 3, 0))
    }
    
    fn generate_cte_sql(&self, cte: &CteDefinition) -> SqlResult<String> {
        let mut sql = String::new();
        
        if cte.recursive {
            sql.push_str("WITH RECURSIVE ");
        } else {
            sql.push_str("WITH ");
        }
        
        sql.push_str(&cte.name);
        
        if let Some(ref columns) = cte.columns {
            sql.push_str(&format!(" ({})", columns.join(", ")));
        }
        
        sql.push_str(" AS (");
        sql.push_str(&cte.query);
        sql.push(')');
        
        Ok(sql)
    }
    
    fn generate_window_sql(&self, window: &WindowFunction) -> SqlResult<String> {
        let mut sql = String::new();
        
        // Generate the window function
        match &window.function {
            WindowFunctionType::RowNumber => sql.push_str("ROW_NUMBER()"),
            WindowFunctionType::Rank => sql.push_str("RANK()"),
            WindowFunctionType::DenseRank => sql.push_str("DENSE_RANK()"),
            WindowFunctionType::Lag { offset, default } => {
                sql.push_str(&format!("LAG(column, {}", offset));
                if let Some(def) = default {
                    sql.push_str(&format!(", {}", format_value(def)));
                }
                sql.push(')');
            },
            WindowFunctionType::Lead { offset, default } => {
                sql.push_str(&format!("LEAD(column, {})", offset));
                if let Some(def) = default {
                    sql.push_str(&format!(", {}", format_value(def)));
                }
                sql.push(')');
            },
            WindowFunctionType::FirstValue(col) => sql.push_str(&format!("FIRST_VALUE({})", col)),
            WindowFunctionType::LastValue(col) => sql.push_str(&format!("LAST_VALUE({})", col)),
            WindowFunctionType::NthValue { column, n } => {
                sql.push_str(&format!("NTH_VALUE({}, {})", column, n));
            },
            WindowFunctionType::Sum(col) => sql.push_str(&format!("SUM({})", col)),
            WindowFunctionType::Avg(col) => sql.push_str(&format!("AVG({})", col)),
            WindowFunctionType::Count(col) => sql.push_str(&format!("COUNT({})", col)),
            WindowFunctionType::Min(col) => sql.push_str(&format!("MIN({})", col)),
            WindowFunctionType::Max(col) => sql.push_str(&format!("MAX({})", col)),
        }
        
        // Add OVER clause
        sql.push_str(" OVER (");
        
        // Partition by
        if !window.partition_by.is_empty() {
            sql.push_str("PARTITION BY ");
            sql.push_str(&window.partition_by.join(", "));
        }
        
        // Order by
        if !window.order_by.is_empty() {
            if !window.partition_by.is_empty() {
                sql.push(' ');
            }
            sql.push_str("ORDER BY ");
            let order_clauses: Vec<String> = window.order_by.iter().map(|clause| {
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
        
        // Window frame
        if let Some(ref frame) = window.frame {
            sql.push(' ');
            match frame.frame_type {
                FrameType::Rows => sql.push_str("ROWS"),
                FrameType::Range => sql.push_str("RANGE"),
                FrameType::Groups => sql.push_str("GROUPS"),
            }
            
            sql.push(' ');
            if let Some(ref end) = frame.end {
                sql.push_str("BETWEEN ");
                sql.push_str(&format_frame_bound(&frame.start));
                sql.push_str(" AND ");
                sql.push_str(&format_frame_bound(end));
            } else {
                sql.push_str(&format_frame_bound(&frame.start));
            }
        }
        
        sql.push(')');
        Ok(sql)
    }
    
    fn generate_case_sql(&self, case_expr: &CaseExpression) -> SqlResult<String> {
        let mut sql = String::from("CASE");
        
        // Handle simple vs searched CASE
        match &case_expr.case_type {
            CaseType::Simple(expr) => {
                sql.push(' ');
                sql.push_str(&format_expression(expr));
            },
            CaseType::Searched => {
                // No expression after CASE for searched case
            },
        }
        
        // Add WHEN clauses
        for when_clause in &case_expr.when_clauses {
            sql.push_str(" WHEN ");
            sql.push_str(&format_expression(&when_clause.condition));
            sql.push_str(" THEN ");
            sql.push_str(&format_expression(&when_clause.result));
        }
        
        // Add ELSE clause if present
        if let Some(ref else_expr) = case_expr.else_clause {
            sql.push_str(" ELSE ");
            sql.push_str(&format_expression(else_expr));
        }
        
        sql.push_str(" END");
        Ok(sql)
    }
    
    fn generate_json_sql(&self, operation: &JsonOperation) -> SqlResult<String> {
        let sql = match operation.operation_type {
            JsonOperationType::Extract => {
                // PostgreSQL uses -> and ->> operators
                format!("column -> '$.{}'", operation.path)
            },
            JsonOperationType::Set => {
                if let Some(ref value) = operation.value {
                    format!("jsonb_set(column, '{{{}}}', {})", operation.path, format_value(value))
                } else {
                    return Err(SqlError::runtime_error("jsonb_set requires a value"));
                }
            },
            JsonOperationType::Insert => {
                if let Some(ref value) = operation.value {
                    format!("jsonb_insert(column, '{{{}}}', {})", operation.path, format_value(value))
                } else {
                    return Err(SqlError::runtime_error("jsonb_insert requires a value"));
                }
            },
            JsonOperationType::Replace => {
                // PostgreSQL doesn't have jsonb_replace, use jsonb_set with create_missing=false
                if let Some(ref value) = operation.value {
                    format!("jsonb_set(column, '{{{}}}', {}, false)", operation.path, format_value(value))
                } else {
                    return Err(SqlError::runtime_error("jsonb_set requires a value"));
                }
            },
            JsonOperationType::Remove => {
                format!("column - '{}'", operation.path)
            },
            JsonOperationType::ArrayLength => {
                format!("jsonb_array_length(column -> '{}')", operation.path)
            },
            JsonOperationType::Valid => {
                // PostgreSQL doesn't need validation for JSONB
                "true".to_string()
            },
            JsonOperationType::Type => {
                format!("jsonb_typeof(column -> '{}')", operation.path)
            },
        };
        
        Ok(sql)
    }
    
    fn generate_aggregate_sql(&self, agg: &AdvancedAggregation) -> SqlResult<String> {
        let mut sql = String::new();
        
        // Generate the aggregate function
        match &agg.function {
            AggregateFunction::Count(col) => {
                if let Some(column) = col {
                    sql.push_str(&format!("COUNT({})", column));
                } else {
                    sql.push_str("COUNT(*)");
                }
            },
            AggregateFunction::Sum(col) => sql.push_str(&format!("SUM({})", col)),
            AggregateFunction::Avg(col) => sql.push_str(&format!("AVG({})", col)),
            AggregateFunction::Min(col) => sql.push_str(&format!("MIN({})", col)),
            AggregateFunction::Max(col) => sql.push_str(&format!("MAX({})", col)),
            AggregateFunction::GroupConcat { column, separator } => {
                // PostgreSQL uses STRING_AGG
                let sep = separator.as_deref().unwrap_or(",");
                sql.push_str(&format!("STRING_AGG({}, '{}')", column, sep));
            },
            AggregateFunction::StringAgg { column, separator } => {
                sql.push_str(&format!("STRING_AGG({}, '{}')", column, separator));
            },
            AggregateFunction::ArrayAgg(col) => {
                sql.push_str(&format!("ARRAY_AGG({})", col));
            },
            AggregateFunction::JsonArrayAgg(col) => {
                sql.push_str(&format!("JSON_AGG({})", col));
            },
            AggregateFunction::JsonObjectAgg { key, value } => {
                sql.push_str(&format!("JSON_OBJECT_AGG({}, {})", key, value));
            },
            AggregateFunction::Percentile { column, percentile } => {
                sql.push_str(&format!("PERCENTILE_CONT({}) WITHIN GROUP (ORDER BY {})", percentile, column));
            },
            AggregateFunction::StdDev(col) => {
                sql.push_str(&format!("STDDEV({})", col));
            },
            AggregateFunction::Variance(col) => {
                sql.push_str(&format!("VARIANCE({})", col));
            },
        }
        
        // Add DISTINCT if specified
        if agg.distinct {
            // Insert DISTINCT after the opening parenthesis
            if let Some(pos) = sql.find('(') {
                sql.insert_str(pos + 1, "DISTINCT ");
            }
        }
        
        // Add FILTER clause if specified
        if let Some(ref filter) = agg.filter {
            sql.push_str(" FILTER (WHERE ");
            sql.push_str(&format_expression(filter));
            sql.push(')');
        }
        
        // Add OVER clause for window functions
        if let Some(ref window) = agg.over {
            sql.push_str(" OVER (");

            // Partition by
            if !window.partition_by.is_empty() {
                sql.push_str("PARTITION BY ");
                sql.push_str(&window.partition_by.join(", "));
            }

            // Order by
            if !window.order_by.is_empty() {
                if !window.partition_by.is_empty() {
                    sql.push(' ');
                }
                sql.push_str("ORDER BY ");
                let order_clauses: Vec<String> = window.order_by.iter().map(|clause| {
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

            sql.push(')');
        }
        
        Ok(sql)
    }
    
    fn generate_fts_sql(&self, fts: &FullTextSearch) -> SqlResult<String> {
        // PostgreSQL full-text search using tsvector and tsquery
        let mut sql = format!(
            "SELECT * FROM {} WHERE to_tsvector('{}', {}) @@ to_tsquery('{}')",
            fts.table,
            fts.options.language.as_deref().unwrap_or("english"),
            fts.columns.join(" || ' ' || "),
            fts.query
        );
        
        // Add ranking if specified
        if let Some(ref ranking) = fts.options.ranking {
            match ranking {
                RankingFunction::TfIdf => {
                    sql = format!(
                        "SELECT *, ts_rank(to_tsvector('{}', {}), to_tsquery('{}')) as rank FROM {} WHERE to_tsvector('{}', {}) @@ to_tsquery('{}') ORDER BY rank DESC",
                        fts.options.language.as_deref().unwrap_or("english"),
                        fts.columns.join(" || ' ' || "),
                        fts.query,
                        fts.table,
                        fts.options.language.as_deref().unwrap_or("english"),
                        fts.columns.join(" || ' ' || "),
                        fts.query
                    );
                },
                RankingFunction::Bm25 => {
                    // PostgreSQL doesn't have built-in BM25, but can be approximated
                    sql = format!(
                        "SELECT *, ts_rank_cd(to_tsvector('{}', {}), to_tsquery('{}')) as rank FROM {} WHERE to_tsvector('{}', {}) @@ to_tsquery('{}') ORDER BY rank DESC",
                        fts.options.language.as_deref().unwrap_or("english"),
                        fts.columns.join(" || ' ' || "),
                        fts.query,
                        fts.table,
                        fts.options.language.as_deref().unwrap_or("english"),
                        fts.columns.join(" || ' ' || "),
                        fts.query
                    );
                },
                RankingFunction::Simple => {
                    sql = format!(
                        "SELECT *, 1.0 as rank FROM {} WHERE to_tsvector('{}', {}) @@ to_tsquery('{}')",
                        fts.table,
                        fts.options.language.as_deref().unwrap_or("english"),
                        fts.columns.join(" || ' ' || "),
                        fts.query
                    );
                },
            }
        }
        
        Ok(sql)
    }
}

// Helper functions
fn format_value(value: &Value) -> String {
    match value {
        Value::Integer(i) => i.to_string(),
        Value::Real(r) => r.to_string(),
        Value::Text(s) => format!("'{}'", s.replace("'", "''")),
        Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Null => "NULL".to_string(),
        Value::Blob(data) => format!("'\\x{}'", hex::encode(data)),
    }
}

fn format_expression(expr: &crate::sqlite::parser::ast::Expression) -> String {
    // Simplified expression formatting
    // In a real implementation, this would be more comprehensive
    format!("{}", expr)
}

fn format_frame_bound(bound: &FrameBound) -> String {
    match bound {
        FrameBound::UnboundedPreceding => "UNBOUNDED PRECEDING".to_string(),
        FrameBound::Preceding(n) => format!("{} PRECEDING", n),
        FrameBound::CurrentRow => "CURRENT ROW".to_string(),
        FrameBound::Following(n) => format!("{} FOLLOWING", n),
        FrameBound::UnboundedFollowing => "UNBOUNDED FOLLOWING".to_string(),
    }
}
