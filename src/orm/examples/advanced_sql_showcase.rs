/// Comprehensive example showcasing advanced SQL features and database extensibility
use crate::orm::advanced_sql::*;
use crate::orm::sqlite_advanced::SqliteAdvancedPlugin;
use crate::orm::postgres_advanced::PostgresAdvancedPlugin;
use crate::orm::QueryBuilder;
use crate::orm::sqlserver_advanced::SqlServerAdvancedPlugin;
use crate::orm::feature_compatibility::*;
use crate::orm::advanced_query_builder::*;
use crate::orm::{SqlResult, SqlError};
use crate::sqlite::types::Value;
use crate::sqlite::parser::ast::Expression;

/// Demonstrate advanced SQL features across different databases
pub async fn showcase_advanced_sql_features() -> SqlResult<()> {
    println!("🚀 Advanced SQL Features Showcase");
    println!("==================================\n");
    
    // Initialize feature compatibility checker
    let mut checker = FeatureCompatibilityChecker::new();
    
    // Register database plugins with different versions
    checker.register_plugin(
        "SQLite".to_string(),
        Box::new(SqliteAdvancedPlugin::new())
    );
    
    checker.register_plugin(
        "PostgreSQL".to_string(),
        Box::new(PostgresAdvancedPlugin::new())
    );
    
    // Register different SQL Server versions to show version-aware features
    checker.register_plugin(
        "SQL Server 2019".to_string(),
        Box::new(SqlServerAdvancedPlugin::with_version(DatabaseVersion::new(2019, 15, 0)))
    );
    
    checker.register_plugin(
        "SQL Server 2022".to_string(),
        Box::new(SqlServerAdvancedPlugin::with_version(DatabaseVersion::new(2022, 16, 0)))
    );
    
    // Check compatibility for each database
    println!("📊 Database Feature Compatibility Analysis");
    println!("==========================================");
    
    let databases = vec![
        "SQLite".to_string(),
        "PostgreSQL".to_string(),
        "SQL Server 2019".to_string(),
        "SQL Server 2022".to_string(),
    ];
    
    let reports = checker.compare_databases(&databases)?;
    
    for report in &reports {
        print_compatibility_report(report);
    }
    
    // Show feature matrix
    println!("\n📋 Feature Support Matrix");
    println!("=========================");
    let matrix = checker.get_feature_matrix()?;
    print_feature_matrix(&matrix);
    
    // Demonstrate database recommendation
    println!("\n🎯 Database Recommendation Engine");
    println!("=================================");
    
    let required_features = vec![
        "CTE".to_string(),
        "Window Functions".to_string(),
        "JSON Operations".to_string(),
    ];
    
    let recommendation = checker.recommend_database(&required_features)?;
    print_database_recommendation(&recommendation);
    
    // Demonstrate advanced query building
    println!("\n🔧 Advanced Query Builder Examples");
    println!("==================================");
    
    demonstrate_advanced_queries().await?;
    
    Ok(())
}

fn print_compatibility_report(report: &CompatibilityReport) {
    println!("\n🗄️  {} {}", report.database, report.version);
    println!("   Overall Compatibility Score: {:.1}%", report.overall_score);
    
    for (feature_name, feature) in &report.features {
        let status = if feature.current_version_compatible {
            "✅"
        } else if feature.supported {
            "⚠️ "
        } else {
            "❌"
        };
        
        println!("   {} {}", status, feature_name);
        
        if let Some(ref min_version) = feature.minimum_version {
            if !feature.current_version_compatible {
                println!("      Requires: {} or later", min_version);
            }
        }
        
        if let Some(ref notes) = feature.notes {
            println!("      Note: {}", notes);
        }
        
        if !feature.alternatives.is_empty() {
            println!("      Alternatives: {}", feature.alternatives.join(", "));
        }
    }
    
    if !report.recommendations.is_empty() {
        println!("   💡 Recommendations:");
        for rec in &report.recommendations {
            println!("      • {}", rec);
        }
    }
}

fn print_feature_matrix(matrix: &FeatureMatrix) {
    for (feature_name, databases) in &matrix.features {
        println!("\n📌 {}", feature_name);
        for (db_name, info) in databases {
            let status = if info.support.supported {
                if let Some(ref min_version) = info.support.minimum_version {
                    format!("✅ (since {})", min_version)
                } else {
                    "✅".to_string()
                }
            } else {
                "❌".to_string()
            };
            println!("   {} {}: {}", status, db_name, info.version);
        }
    }
}

fn print_database_recommendation(recommendation: &DatabaseRecommendation) {
    println!("Required Features: {}", recommendation.required_features.join(", "));
    println!("\nDatabase Scores:");
    
    let mut sorted_scores = recommendation.database_scores.clone();
    sorted_scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    
    for score in &sorted_scores {
        println!("  {} {}: {:.1}% ({}/{} features)",
            score.database,
            score.version,
            score.score * 100.0,
            score.supported_features.len(),
            recommendation.required_features.len()
        );
        
        if !score.unsupported_features.is_empty() {
            println!("    Missing: {}", score.unsupported_features.join(", "));
        }
    }
    
    if let Some(ref recommended) = recommendation.recommended {
        println!("\n🏆 Recommended: {} {} ({:.1}% compatibility)",
            recommended.database,
            recommended.version,
            recommended.score * 100.0
        );
    }
}

async fn demonstrate_advanced_queries() -> SqlResult<()> {
    // Example 1: Complex CTE with window functions
    println!("\n1️⃣  Recursive CTE with Window Functions");
    
    let sqlite_plugin = Box::new(SqliteAdvancedPlugin::new());
    
    let query = AdvancedQueryBuilder::new()
        .with_plugin(sqlite_plugin)
        .with_recursive_cte(
            "employee_hierarchy",
            vec!["id".to_string(), "name".to_string(), "manager_id".to_string(), "level".to_string()],
            "SELECT id, name, manager_id, 1 as level FROM employees WHERE manager_id IS NULL
             UNION ALL
             SELECT e.id, e.name, e.manager_id, eh.level + 1
             FROM employees e
             JOIN employee_hierarchy eh ON e.manager_id = eh.id"
        )
        .with_row_number(
            "row_num",
            vec!["level".to_string()],
            vec![order_by("name", OrderDirection::Asc)]
        )
        .base_query(
            QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
                .column("*")
                .column("row_num")
                .from("employee_hierarchy")
        )
        .build()?;
    
    println!("Generated SQL:\n{}\n", query);
    
    // Example 2: JSON operations with aggregations
    println!("2️⃣  JSON Operations with Advanced Aggregations");
    
    let postgres_plugin = Box::new(PostgresAdvancedPlugin::new());
    
    let query2 = AdvancedQueryBuilder::new()
        .with_plugin(postgres_plugin)
        .with_json_extract("user_preferences", "$.preferences.theme")
        .with_json_array_agg("tags_array", "tags", true)
        .base_query(
            QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
                .column("user_id")
                .column("user_preferences")
                .column("tags_array")
                .from("users")
        )
        .build()?;
    
    println!("Generated SQL:\n{}\n", query2);
    
    // Example 3: Full-text search with ranking
    println!("3️⃣  Full-Text Search with Custom Ranking");
    
    let sqlserver_plugin = Box::new(SqlServerAdvancedPlugin::new());
    
    let fts = FullTextSearch {
        table: "articles".to_string(),
        columns: vec!["title".to_string(), "content".to_string()],
        query: "machine learning AI".to_string(),
        options: FtsOptions {
            tokenizer: None,
            language: Some("english".to_string()),
            stemming: true,
            stop_words: true,
            ranking: Some(RankingFunction::Simple),
        },
    };
    
    let fts_sql = sqlserver_plugin.generate_fts_sql(&fts)?;
    println!("Generated SQL:\n{}\n", fts_sql);
    
    // Example 4: Complex CASE expression with window functions
    println!("4️⃣  Complex CASE Expression with Window Functions");
    
    let case_expr = CaseExpression {
        case_type: CaseType::Searched,
        when_clauses: vec![
            WhenClause {
                condition: Expression::Column("salary".to_string()),
                result: Expression::Literal(Value::Text("High".to_string())),
            },
            WhenClause {
                condition: Expression::Column("salary".to_string()),
                result: Expression::Literal(Value::Text("Medium".to_string())),
            },
        ],
        else_clause: Some(Expression::Literal(Value::Text("Low".to_string()))),
    };
    
    let case_sql = sqlserver_plugin.generate_case_sql(&case_expr)?;
    println!("Generated CASE expression:\n{}\n", case_sql);
    
    Ok(())
}

/// Demonstrate version-specific feature handling
pub async fn demonstrate_version_compatibility() -> SqlResult<()> {
    println!("🔄 Version-Specific Feature Compatibility");
    println!("=========================================\n");
    
    // Test with different SQL Server versions
    let versions = vec![
        ("SQL Server 2008", DatabaseVersion::new(2008, 10, 0)),
        ("SQL Server 2012", DatabaseVersion::new(2012, 11, 0)),
        ("SQL Server 2016", DatabaseVersion::new(2016, 13, 0)),
        ("SQL Server 2019", DatabaseVersion::new(2019, 15, 0)),
        ("SQL Server 2022", DatabaseVersion::new(2022, 16, 0)),
    ];
    
    for (name, version) in versions {
        println!("📅 Testing {}", name);
        let plugin = SqlServerAdvancedPlugin::with_version(version);
        
        // Test window functions
        let window_support = plugin.supports_window_functions();
        println!("  Window Functions: {}", 
            if window_support.supported { "✅" } else { "❌" });
        
        // Test JSON operations
        let json_support = plugin.supports_json();
        println!("  JSON Operations: {}", 
            if json_support.supported { "✅" } else { "❌" });
        
        if let Some(ref notes) = json_support.notes {
            println!("    Note: {}", notes);
        }
        
        println!();
    }
    
    Ok(())
}

/// Example of creating a custom database plugin
pub struct CustomDatabasePlugin {
    name: String,
    version: DatabaseVersion,
}

impl CustomDatabasePlugin {
    pub fn new(name: String, version: DatabaseVersion) -> Self {
        Self { name, version }
    }
}

impl AdvancedSqlPlugin for CustomDatabasePlugin {
    fn database_info(&self) -> (String, DatabaseVersion) {
        (self.name.clone(), self.version.clone())
    }
    
    fn supports_cte(&self) -> FeatureSupport {
        FeatureSupport::supported_with_notes("Custom implementation of CTEs")
    }
    
    fn supports_recursive_cte(&self) -> FeatureSupport {
        FeatureSupport::not_supported_with_notes("Recursive CTEs not implemented yet")
    }
    
    fn supports_window_functions(&self) -> FeatureSupport {
        FeatureSupport::supported()
    }
    
    fn supports_json(&self) -> FeatureSupport {
        FeatureSupport::supported_with_notes("Custom JSON implementation")
    }
    
    fn supports_full_text_search(&self) -> FeatureSupport {
        FeatureSupport::not_supported()
    }
    
    fn generate_cte_sql(&self, cte: &CteDefinition) -> SqlResult<String> {
        Ok(format!("/* Custom CTE: {} */", cte.name))
    }
    
    fn generate_window_sql(&self, _window: &WindowFunction) -> SqlResult<String> {
        Ok("/* Custom window function */".to_string())
    }
    
    fn generate_case_sql(&self, _case_expr: &CaseExpression) -> SqlResult<String> {
        Ok("/* Custom CASE expression */".to_string())
    }
    
    fn generate_json_sql(&self, _operation: &JsonOperation) -> SqlResult<String> {
        Ok("/* Custom JSON operation */".to_string())
    }
    
    fn generate_aggregate_sql(&self, _agg: &AdvancedAggregation) -> SqlResult<String> {
        Ok("/* Custom aggregation */".to_string())
    }
    
    fn generate_fts_sql(&self, _fts: &FullTextSearch) -> SqlResult<String> {
        Err(SqlError::runtime_error("Full-text search not supported"))
    }
}
