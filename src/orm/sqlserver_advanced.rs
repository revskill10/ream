/// SQL Server-specific advanced SQL plugin implementation
use crate::orm::advanced_sql::*;
use crate::orm::{SqlResult, SqlError};
use crate::sqlite::types::Value;

/// SQL Server advanced SQL plugin
pub struct SqlServerAdvancedPlugin {
    version: DatabaseVersion,
}

impl SqlServerAdvancedPlugin {
    pub fn new() -> Self {
        Self {
            version: DatabaseVersion::new(2022, 16, 0), // SQL Server 2022
        }
    }
    
    pub fn with_version(version: DatabaseVersion) -> Self {
        Self { version }
    }
}

impl AdvancedSqlPlugin for SqlServerAdvancedPlugin {
    fn database_info(&self) -> (String, DatabaseVersion) {
        ("SQL Server".to_string(), self.version.clone())
    }
    
    fn supports_cte(&self) -> FeatureSupport {
        FeatureSupport::supported_since(DatabaseVersion::new(2005, 9, 0))
    }
    
    fn supports_recursive_cte(&self) -> FeatureSupport {
        FeatureSupport::supported_since(DatabaseVersion::new(2005, 9, 0))
    }
    
    fn supports_window_functions(&self) -> FeatureSupport {
        if self.version.is_at_least(&DatabaseVersion::new(2012, 11, 0)) {
            FeatureSupport::supported_with_notes(
                "Full window function support since SQL Server 2012. Basic ROW_NUMBER() since 2005."
            )
        } else if self.version.is_at_least(&DatabaseVersion::new(2005, 9, 0)) {
            FeatureSupport::supported_with_notes(
                "Limited window function support (ROW_NUMBER, RANK, DENSE_RANK only)"
            )
        } else {
            FeatureSupport::not_supported()
        }
    }
    
    fn supports_json(&self) -> FeatureSupport {
        if self.version.is_at_least(&DatabaseVersion::new(2016, 13, 0)) {
            FeatureSupport::supported_since(DatabaseVersion::new(2016, 13, 0))
        } else {
            FeatureSupport::not_supported_with_notes(
                "JSON support requires SQL Server 2016 or later"
            )
        }
    }
    
    fn supports_full_text_search(&self) -> FeatureSupport {
        FeatureSupport::supported_since(DatabaseVersion::new(2000, 8, 0))
    }
    
    fn generate_cte_sql(&self, cte: &CteDefinition) -> SqlResult<String> {
        let mut sql = String::new();
        
        if cte.recursive {
            sql.push_str("WITH ");
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
                if !self.version.is_at_least(&DatabaseVersion::new(2012, 11, 0)) {
                    return Err(SqlError::runtime_error("LAG function requires SQL Server 2012 or later"));
                }
                sql.push_str(&format!("LAG(column, {})", offset));
                if let Some(def) = default {
                    sql.push_str(&format!(", {}", format_value(def)));
                }
                sql.push(')');
            },
            WindowFunctionType::Lead { offset, default } => {
                if !self.version.is_at_least(&DatabaseVersion::new(2012, 11, 0)) {
                    return Err(SqlError::runtime_error("LEAD function requires SQL Server 2012 or later"));
                }
                sql.push_str(&format!("LEAD(column, {})", offset));
                if let Some(def) = default {
                    sql.push_str(&format!(", {}", format_value(def)));
                }
                sql.push(')');
            },
            WindowFunctionType::FirstValue(col) => {
                if !self.version.is_at_least(&DatabaseVersion::new(2012, 11, 0)) {
                    return Err(SqlError::runtime_error("FIRST_VALUE function requires SQL Server 2012 or later"));
                }
                sql.push_str(&format!("FIRST_VALUE({})", col));
            },
            WindowFunctionType::LastValue(col) => {
                if !self.version.is_at_least(&DatabaseVersion::new(2012, 11, 0)) {
                    return Err(SqlError::runtime_error("LAST_VALUE function requires SQL Server 2012 or later"));
                }
                sql.push_str(&format!("LAST_VALUE({})", col));
            },
            WindowFunctionType::NthValue { column, n } => {
                // SQL Server doesn't have NTH_VALUE, simulate with LAG/LEAD
                return Err(SqlError::runtime_error("NTH_VALUE not directly supported in SQL Server"));
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
                // SQL Server doesn't support NULLS FIRST/LAST in all versions
                order_sql
            }).collect();
            sql.push_str(&order_clauses.join(", "));
        }
        
        // Window frame (SQL Server 2012+)
        if let Some(ref frame) = window.frame {
            if !self.version.is_at_least(&DatabaseVersion::new(2012, 11, 0)) {
                return Err(SqlError::runtime_error("Window frames require SQL Server 2012 or later"));
            }
            
            sql.push(' ');
            match frame.frame_type {
                FrameType::Rows => sql.push_str("ROWS"),
                FrameType::Range => sql.push_str("RANGE"),
                FrameType::Groups => {
                    return Err(SqlError::runtime_error("GROUPS frame type not supported in SQL Server"));
                },
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
        if !self.version.is_at_least(&DatabaseVersion::new(2016, 13, 0)) {
            return Err(SqlError::runtime_error("JSON operations require SQL Server 2016 or later"));
        }
        
        let sql = match operation.operation_type {
            JsonOperationType::Extract => {
                format!("JSON_VALUE(column, '$.{}')", operation.path)
            },
            JsonOperationType::Set => {
                if let Some(ref value) = operation.value {
                    format!("JSON_MODIFY(column, '$.{}', {})", operation.path, format_value(value))
                } else {
                    return Err(SqlError::runtime_error("JSON_MODIFY requires a value"));
                }
            },
            JsonOperationType::Insert => {
                // SQL Server doesn't have JSON_INSERT, use JSON_MODIFY with append
                if let Some(ref value) = operation.value {
                    format!("JSON_MODIFY(column, 'append $.{}', {})", operation.path, format_value(value))
                } else {
                    return Err(SqlError::runtime_error("JSON_MODIFY requires a value"));
                }
            },
            JsonOperationType::Replace => {
                // SQL Server doesn't distinguish between set and replace
                if let Some(ref value) = operation.value {
                    format!("JSON_MODIFY(column, '$.{}', {})", operation.path, format_value(value))
                } else {
                    return Err(SqlError::runtime_error("JSON_MODIFY requires a value"));
                }
            },
            JsonOperationType::Remove => {
                format!("JSON_MODIFY(column, '$.{}', NULL)", operation.path)
            },
            JsonOperationType::ArrayLength => {
                // SQL Server doesn't have direct array length function
                return Err(SqlError::runtime_error("JSON array length not directly supported"));
            },
            JsonOperationType::Valid => {
                "ISJSON(column)".to_string()
            },
            JsonOperationType::Type => {
                // SQL Server doesn't have JSON_TYPE function
                return Err(SqlError::runtime_error("JSON type detection not directly supported"));
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
                // SQL Server uses STRING_AGG (2017+) or FOR XML PATH (older versions)
                if self.version.is_at_least(&DatabaseVersion::new(2017, 14, 0)) {
                    let sep = separator.as_deref().unwrap_or(",");
                    sql.push_str(&format!("STRING_AGG({}, '{}')", column, sep));
                } else {
                    // Fallback to STUFF + FOR XML PATH for older versions
                    return Err(SqlError::runtime_error("STRING_AGG requires SQL Server 2017 or later"));
                }
            },
            AggregateFunction::StringAgg { column, separator } => {
                if self.version.is_at_least(&DatabaseVersion::new(2017, 14, 0)) {
                    sql.push_str(&format!("STRING_AGG({}, '{}')", column, separator));
                } else {
                    return Err(SqlError::runtime_error("STRING_AGG requires SQL Server 2017 or later"));
                }
            },
            AggregateFunction::ArrayAgg(col) => {
                // SQL Server doesn't have ARRAY_AGG, but can simulate with JSON
                if self.version.is_at_least(&DatabaseVersion::new(2016, 13, 0)) {
                    sql.push_str(&format!("(SELECT {} FROM table FOR JSON AUTO)", col));
                } else {
                    return Err(SqlError::runtime_error("Array aggregation requires SQL Server 2016 or later"));
                }
            },
            AggregateFunction::JsonArrayAgg(col) => {
                if self.version.is_at_least(&DatabaseVersion::new(2016, 13, 0)) {
                    sql.push_str(&format!("(SELECT {} FROM table FOR JSON AUTO)", col));
                } else {
                    return Err(SqlError::runtime_error("JSON aggregation requires SQL Server 2016 or later"));
                }
            },
            AggregateFunction::JsonObjectAgg { key, value } => {
                if self.version.is_at_least(&DatabaseVersion::new(2016, 13, 0)) {
                    sql.push_str(&format!("(SELECT {} as [key], {} as [value] FROM table FOR JSON AUTO)", key, value));
                } else {
                    return Err(SqlError::runtime_error("JSON aggregation requires SQL Server 2016 or later"));
                }
            },
            AggregateFunction::Percentile { column, percentile } => {
                sql.push_str(&format!("PERCENTILE_CONT({}) WITHIN GROUP (ORDER BY {})", percentile, column));
            },
            AggregateFunction::StdDev(col) => {
                sql.push_str(&format!("STDEV({})", col));
            },
            AggregateFunction::Variance(col) => {
                sql.push_str(&format!("VAR({})", col));
            },
        }
        
        // Add DISTINCT if specified
        if agg.distinct {
            // Insert DISTINCT after the opening parenthesis
            if let Some(pos) = sql.find('(') {
                sql.insert_str(pos + 1, "DISTINCT ");
            }
        }
        
        // SQL Server doesn't support FILTER clause, would need to be handled differently
        if agg.filter.is_some() {
            return Err(SqlError::runtime_error("FILTER clause not supported in SQL Server"));
        }
        
        // Add OVER clause for window functions
        if let Some(ref window) = agg.over {
            sql.push(' ');
            sql.push_str(&self.generate_window_sql(window)?);
        }
        
        Ok(sql)
    }
    
    fn generate_fts_sql(&self, fts: &FullTextSearch) -> SqlResult<String> {
        // SQL Server full-text search using CONTAINS or FREETEXT
        let mut sql = format!(
            "SELECT * FROM {} WHERE CONTAINS(({}) , '{}')",
            fts.table,
            fts.columns.join(", "),
            fts.query
        );
        
        // Add ranking if specified
        if let Some(ref ranking) = fts.options.ranking {
            match ranking {
                RankingFunction::Simple => {
                    sql = format!(
                        "SELECT *, ft.RANK as rank FROM {} INNER JOIN CONTAINSTABLE({}, ({}), '{}') AS ft ON {}.id = ft.[KEY] ORDER BY rank DESC",
                        fts.table,
                        fts.table, fts.columns.join(", "), fts.query,
                        fts.table
                    );
                },
                RankingFunction::TfIdf | RankingFunction::Bm25 => {
                    // SQL Server uses its own ranking algorithm
                    sql = format!(
                        "SELECT *, ft.RANK as rank FROM {} INNER JOIN CONTAINSTABLE({}, ({}), '{}') AS ft ON {}.id = ft.[KEY] ORDER BY rank DESC",
                        fts.table,
                        fts.table, fts.columns.join(", "), fts.query,
                        fts.table
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
        Value::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
        Value::Null => "NULL".to_string(),
        Value::Blob(_) => "0x<blob>".to_string(),
    }
}

fn format_expression(expr: &crate::sqlite::parser::ast::Expression) -> String {
    // Simplified expression formatting
    // In a real implementation, this would be more comprehensive
    match expr {
        crate::sqlite::parser::ast::Expression::Literal(value) => {
            format_value(value)
        },
        crate::sqlite::parser::ast::Expression::Column(name) => {
            name.clone()
        },
        _ => format!("{}", expr)
    }
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
