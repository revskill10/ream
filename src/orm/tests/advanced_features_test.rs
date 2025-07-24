/// Comprehensive tests for advanced SQL features and database extensibility
#[cfg(test)]
mod tests {
    use crate::orm::advanced_sql::*;
    use crate::orm::sqlite_advanced::SqliteAdvancedPlugin;
    use crate::orm::postgres_advanced::PostgresAdvancedPlugin;
    use crate::orm::sqlserver_advanced::SqlServerAdvancedPlugin;
    use crate::orm::feature_compatibility::*;
    use crate::orm::advanced_query_builder::*;
    use crate::orm::schema::*;
    use crate::orm::query::{QueryBuilder, SelectQueryBuilder};
    use crate::orm::sql_composable::{SqlComposable, TableAlias};
    use crate::orm::schema::{Table, AliasedTable, TypeSafeSchema};
    use crate::sqlite::types::{DataType, Value};
    use serde_json::json;
    use crate::sqlite::parser::ast::Expression;

    #[test]
    fn test_database_version_comparison() {
        let v1 = DatabaseVersion::new(3, 8, 3);
        let v2 = DatabaseVersion::new(3, 25, 0);
        let v3 = DatabaseVersion::new(4, 0, 0);
        
        assert!(v2.is_at_least(&v1));
        assert!(!v1.is_at_least(&v2));
        assert!(v3.is_at_least(&v2));
        assert!(v3.is_at_least(&v1));
        
        // Test with build numbers
        let v4 = DatabaseVersion::with_build(3, 8, 3, 100);
        let v5 = DatabaseVersion::with_build(3, 8, 3, 200);
        
        assert!(v5.is_at_least(&v4));
        assert!(!v4.is_at_least(&v5));
    }

    #[test]
    fn test_sqlite_feature_support() {
        let plugin = SqliteAdvancedPlugin::new();
        let (name, version) = plugin.database_info();
        
        assert_eq!(name, "SQLite");
        assert!(version.is_at_least(&DatabaseVersion::new(3, 0, 0)));
        
        // Test CTE support
        let cte_support = plugin.supports_cte();
        assert!(cte_support.supported);
        assert!(cte_support.minimum_version.is_some());
        
        // Test window functions
        let window_support = plugin.supports_window_functions();
        assert!(window_support.supported);
        
        // Test JSON support
        let json_support = plugin.supports_json();
        assert!(json_support.supported);
        assert!(json_support.notes.is_some());
    }

    #[test]
    fn test_postgres_feature_support() {
        let plugin = PostgresAdvancedPlugin::new();
        let (name, version) = plugin.database_info();
        
        assert_eq!(name, "PostgreSQL");
        
        // PostgreSQL has excellent support for most features
        assert!(plugin.supports_cte().supported);
        assert!(plugin.supports_recursive_cte().supported);
        assert!(plugin.supports_window_functions().supported);
        assert!(plugin.supports_json().supported);
        assert!(plugin.supports_full_text_search().supported);
    }

    #[test]
    fn test_sqlserver_version_specific_features() {
        // Test SQL Server 2008 (limited features)
        let old_plugin = SqlServerAdvancedPlugin::with_version(DatabaseVersion::new(2008, 10, 0));
        
        assert!(old_plugin.supports_cte().supported);
        assert!(old_plugin.supports_recursive_cte().supported);
        
        let window_support = old_plugin.supports_window_functions();
        assert!(window_support.supported); // Limited support
        assert!(window_support.notes.is_some());
        
        let json_support = old_plugin.supports_json();
        assert!(!json_support.supported); // No JSON in 2008
        
        // Test SQL Server 2016 (JSON support added)
        let new_plugin = SqlServerAdvancedPlugin::with_version(DatabaseVersion::new(2016, 13, 0));
        
        let json_support_2016 = new_plugin.supports_json();
        assert!(json_support_2016.supported);
        
        // Test SQL Server 2022 (latest features)
        let latest_plugin = SqlServerAdvancedPlugin::with_version(DatabaseVersion::new(2022, 16, 0));
        
        assert!(latest_plugin.supports_cte().supported);
        assert!(latest_plugin.supports_window_functions().supported);
        assert!(latest_plugin.supports_json().supported);
    }

    #[test]
    fn test_feature_compatibility_checker() {
        let mut checker = FeatureCompatibilityChecker::new();
        
        checker.register_plugin(
            "SQLite".to_string(),
            Box::new(SqliteAdvancedPlugin::new())
        );
        
        checker.register_plugin(
            "PostgreSQL".to_string(),
            Box::new(PostgresAdvancedPlugin::new())
        );
        
        checker.register_plugin(
            "SQL Server 2019".to_string(),
            Box::new(SqlServerAdvancedPlugin::with_version(DatabaseVersion::new(2019, 15, 0)))
        );
        
        // Test compatibility report
        let report = checker.check_compatibility("SQLite").unwrap();
        assert_eq!(report.database, "SQLite");
        assert!(report.overall_score > 0.0);
        assert!(!report.features.is_empty());
        
        // Test database comparison
        let databases = vec![
            "SQLite".to_string(),
            "PostgreSQL".to_string(),
            "SQL Server 2019".to_string(),
        ];
        
        let reports = checker.compare_databases(&databases).unwrap();
        assert_eq!(reports.len(), 3);
        
        // Test feature matrix
        let matrix = checker.get_feature_matrix().unwrap();
        assert!(!matrix.features.is_empty());
        
        // Test database recommendation
        let required_features = vec![
            "CTE".to_string(),
            "Window Functions".to_string(),
        ];
        
        let recommendation = checker.recommend_database(&required_features).unwrap();
        assert_eq!(recommendation.required_features.len(), 2);
        assert!(!recommendation.database_scores.is_empty());
        assert!(recommendation.recommended.is_some());
    }

    #[test]
    fn test_advanced_query_builder() {
        let plugin = Box::new(SqliteAdvancedPlugin::new());

        // Create type-safe column definitions
        let department_col = Column::new("department", DataType::Text).with_table_name("users");
        let salary_col = Column::new("salary", DataType::Real).with_table_name("users");

        // Create type-safe table reference for the CTE
        let test_cte_table = Table::new("test_cte");

        // Build the base query using the SQL builder with type-safe columns
        let base_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column("*")
            .from_table_ref(&test_cte_table);

        let query = AdvancedQueryBuilder::new()
            .with_plugin(plugin)
            .with_cte("test_cte", "SELECT * FROM users")
            .with_row_number(
                "row_num",
                vec![department_col.qualified_name()],
                vec![order_by_column(&salary_col, OrderDirection::Desc)]
            )
            .base_query(base_query)
            .build();

        assert!(query.is_ok());
        let sql = query.unwrap();

        // Verify the complete SQL structure (updated for qualified column names)
        let expected_sql = "WITH test_cte AS (SELECT * FROM users) SELECT *, ROW_NUMBER() OVER (PARTITION BY users.department ORDER BY users.salary DESC) AS row_num FROM test_cte";
        assert_eq!(sql, expected_sql, "Generated SQL should match exactly");

        // Verify individual components are present (updated for qualified column names)
        let expected_parts = vec![
            "WITH test_cte AS (SELECT * FROM users)",
            "ROW_NUMBER() OVER (PARTITION BY users.department ORDER BY users.salary DESC)",
            "AS row_num",
            "FROM test_cte"
        ];

        for part in expected_parts {
            assert!(sql.contains(part),
                "Generated SQL should contain '{}'\nActual SQL: {}", part, sql);
        }

        // Verify SQL structure order (CTE should come before main query)
        let with_pos = sql.find("WITH").expect("Should contain WITH clause");
        let select_pos = sql.find("SELECT *").expect("Should contain main SELECT");
        assert!(with_pos < select_pos, "WITH clause should come before main SELECT");

        println!("Generated SQL:\n{}", sql);
    }

    #[test]
    fn test_cte_generation() {
        let plugin = SqliteAdvancedPlugin::new();

        // Test recursive CTE
        let recursive_cte = CteDefinition {
            name: "recursive_tree".to_string(),
            columns: Some(vec!["id".to_string(), "parent_id".to_string(), "level".to_string()]),
            query: "SELECT id, parent_id, 1 as level FROM nodes WHERE parent_id IS NULL".to_string(),
            recursive: true,
        };

        let sql = plugin.generate_cte_sql(&recursive_cte).unwrap();
        let expected_sql = "WITH RECURSIVE recursive_tree (id, parent_id, level) AS (SELECT id, parent_id, 1 as level FROM nodes WHERE parent_id IS NULL)";
        assert_eq!(sql, expected_sql, "Recursive CTE SQL should match exactly");

        // Test non-recursive CTE without columns
        let simple_cte = CteDefinition {
            name: "user_summary".to_string(),
            columns: None,
            query: "SELECT department, COUNT(*) as count FROM users GROUP BY department".to_string(),
            recursive: false,
        };

        let sql = plugin.generate_cte_sql(&simple_cte).unwrap();
        let expected_sql = "WITH user_summary AS (SELECT department, COUNT(*) as count FROM users GROUP BY department)";
        assert_eq!(sql, expected_sql, "Simple CTE SQL should match exactly");

        println!("Recursive CTE SQL: {}", plugin.generate_cte_sql(&recursive_cte).unwrap());
        println!("Simple CTE SQL: {}", plugin.generate_cte_sql(&simple_cte).unwrap());
    }

    #[test]
    fn test_window_function_generation() {
        let plugin = PostgresAdvancedPlugin::new();

        // Test LAG function with frame
        let lag_window = WindowFunction {
            function: WindowFunctionType::Lag {
                offset: 1,
                default: Some(Value::Integer(0))
            },
            partition_by: vec!["department".to_string()],
            order_by: vec![order_by_with_nulls(
                "hire_date",
                OrderDirection::Asc,
                NullsOrder::Last
            )],
            frame: Some(WindowFrame {
                frame_type: FrameType::Rows,
                start: FrameBound::Preceding(1),
                end: Some(FrameBound::CurrentRow),
            }),
        };

        let sql = plugin.generate_window_sql(&lag_window).unwrap();
        let expected_sql = "LAG(column, 1, 0) OVER (PARTITION BY department ORDER BY hire_date ASC NULLS LAST ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)";
        assert_eq!(sql, expected_sql, "LAG window function SQL should match exactly");

        // Test ROW_NUMBER without frame
        let row_number_window = WindowFunction {
            function: WindowFunctionType::RowNumber,
            partition_by: vec!["region".to_string(), "department".to_string()],
            order_by: vec![
                order_by("salary", OrderDirection::Desc),
                order_by("hire_date", OrderDirection::Asc)
            ],
            frame: None,
        };

        let sql = plugin.generate_window_sql(&row_number_window).unwrap();
        let expected_sql = "ROW_NUMBER() OVER (PARTITION BY region, department ORDER BY salary DESC, hire_date ASC)";
        assert_eq!(sql, expected_sql, "ROW_NUMBER window function SQL should match exactly");

        // Test SUM with RANGE frame
        let sum_window = WindowFunction {
            function: WindowFunctionType::Sum("amount".to_string()),
            partition_by: vec![],
            order_by: vec![order_by("date", OrderDirection::Asc)],
            frame: Some(WindowFrame {
                frame_type: FrameType::Range,
                start: FrameBound::UnboundedPreceding,
                end: Some(FrameBound::CurrentRow),
            }),
        };

        let sql = plugin.generate_window_sql(&sum_window).unwrap();
        let expected_sql = "SUM(amount) OVER (ORDER BY date ASC RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)";
        assert_eq!(sql, expected_sql, "SUM window function SQL should match exactly");

        println!("LAG SQL: {}", plugin.generate_window_sql(&lag_window).unwrap());
        println!("ROW_NUMBER SQL: {}", plugin.generate_window_sql(&row_number_window).unwrap());
        println!("SUM SQL: {}", plugin.generate_window_sql(&sum_window).unwrap());
    }

    #[test]
    fn test_json_operations() {
        let postgres_plugin = PostgresAdvancedPlugin::new();
        let sqlite_plugin = SqliteAdvancedPlugin::new();

        // Test PostgreSQL JSON operations
        let extract_op = JsonOperation {
            operation_type: JsonOperationType::Extract,
            path: "user.name".to_string(),
            value: None,
        };

        let postgres_sql = postgres_plugin.generate_json_sql(&extract_op).unwrap();
        let expected_postgres_sql = "column -> '$.user.name'";
        assert_eq!(postgres_sql, expected_postgres_sql, "PostgreSQL JSON extract should match exactly");

        let set_op = JsonOperation {
            operation_type: JsonOperationType::Set,
            path: "user.age".to_string(),
            value: Some(Value::Integer(25)),
        };

        let postgres_sql = postgres_plugin.generate_json_sql(&set_op).unwrap();
        let expected_postgres_sql = "jsonb_set(column, '{user.age}', 25)";
        assert_eq!(postgres_sql, expected_postgres_sql, "PostgreSQL JSON set should match exactly");

        let remove_op = JsonOperation {
            operation_type: JsonOperationType::Remove,
            path: "user.temp_field".to_string(),
            value: None,
        };

        let postgres_sql = postgres_plugin.generate_json_sql(&remove_op).unwrap();
        let expected_postgres_sql = "column - 'user.temp_field'";
        assert_eq!(postgres_sql, expected_postgres_sql, "PostgreSQL JSON remove should match exactly");

        // Test SQLite JSON operations
        let sqlite_sql = sqlite_plugin.generate_json_sql(&extract_op).unwrap();
        let expected_sqlite_sql = "JSON_EXTRACT(column, '$.user.name')";
        assert_eq!(sqlite_sql, expected_sqlite_sql, "SQLite JSON extract should match exactly");

        let sqlite_sql = sqlite_plugin.generate_json_sql(&set_op).unwrap();
        let expected_sqlite_sql = "JSON_SET(column, '$.user.age', 25)";
        assert_eq!(sqlite_sql, expected_sqlite_sql, "SQLite JSON set should match exactly");

        println!("PostgreSQL JSON Extract: {}", postgres_plugin.generate_json_sql(&extract_op).unwrap());
        println!("PostgreSQL JSON Set: {}", postgres_plugin.generate_json_sql(&set_op).unwrap());
        println!("SQLite JSON Extract: {}", sqlite_plugin.generate_json_sql(&extract_op).unwrap());
        println!("SQLite JSON Set: {}", sqlite_plugin.generate_json_sql(&set_op).unwrap());
    }

    #[test]
    fn test_case_expression_generation() {
        let plugin = SqlServerAdvancedPlugin::new();

        // Test searched CASE expression
        let searched_case = CaseExpression {
            case_type: CaseType::Searched,
            when_clauses: vec![
                WhenClause {
                    condition: Expression::Column("salary".to_string()),
                    result: Expression::Literal(Value::Text("High".to_string())),
                },
                WhenClause {
                    condition: Expression::Column("department".to_string()),
                    result: Expression::Literal(Value::Text("Medium".to_string())),
                },
            ],
            else_clause: Some(Expression::Literal(Value::Text("Low".to_string()))),
        };

        let sql = plugin.generate_case_sql(&searched_case).unwrap();
        let expected_sql = "CASE WHEN salary THEN 'High' WHEN department THEN 'Medium' ELSE 'Low' END";
        assert_eq!(sql, expected_sql, "Searched CASE expression should match exactly");

        // Test simple CASE expression
        let simple_case = CaseExpression {
            case_type: CaseType::Simple(Expression::Column("status".to_string())),
            when_clauses: vec![
                WhenClause {
                    condition: Expression::Literal(Value::Integer(1)),
                    result: Expression::Literal(Value::Text("Active".to_string())),
                },
                WhenClause {
                    condition: Expression::Literal(Value::Integer(0)),
                    result: Expression::Literal(Value::Text("Inactive".to_string())),
                },
            ],
            else_clause: Some(Expression::Literal(Value::Text("Unknown".to_string()))),
        };

        let sql = plugin.generate_case_sql(&simple_case).unwrap();
        let expected_sql = "CASE status WHEN 1 THEN 'Active' WHEN 0 THEN 'Inactive' ELSE 'Unknown' END";
        assert_eq!(sql, expected_sql, "Simple CASE expression should match exactly");

        // Test CASE without ELSE clause
        let case_no_else = CaseExpression {
            case_type: CaseType::Searched,
            when_clauses: vec![
                WhenClause {
                    condition: Expression::Column("priority".to_string()),
                    result: Expression::Literal(Value::Text("Urgent".to_string())),
                },
            ],
            else_clause: None,
        };

        let sql = plugin.generate_case_sql(&case_no_else).unwrap();
        let expected_sql = "CASE WHEN priority THEN 'Urgent' END";
        assert_eq!(sql, expected_sql, "CASE without ELSE should match exactly");

        println!("Searched CASE: {}", plugin.generate_case_sql(&searched_case).unwrap());
        println!("Simple CASE: {}", plugin.generate_case_sql(&simple_case).unwrap());
        println!("CASE without ELSE: {}", plugin.generate_case_sql(&case_no_else).unwrap());
    }

    #[test]
    fn test_full_text_search_generation() {
        let sqlserver_plugin = SqlServerAdvancedPlugin::new();
        let postgres_plugin = PostgresAdvancedPlugin::new();
        let sqlite_plugin = SqliteAdvancedPlugin::new();

        // Test SQL Server FTS
        let fts = FullTextSearch {
            table: "articles".to_string(),
            columns: vec!["title".to_string(), "content".to_string()],
            query: "machine learning".to_string(),
            options: FtsOptions {
                tokenizer: None,
                language: Some("english".to_string()),
                stemming: true,
                stop_words: true,
                ranking: None,
            },
        };

        let sqlserver_sql = sqlserver_plugin.generate_fts_sql(&fts).unwrap();
        let expected_sqlserver_sql = "SELECT * FROM articles WHERE CONTAINS((title, content) , 'machine learning')";
        assert_eq!(sqlserver_sql, expected_sqlserver_sql, "SQL Server FTS should match exactly");

        // Test SQL Server FTS with ranking
        let fts_with_ranking = FullTextSearch {
            table: "articles".to_string(),
            columns: vec!["title".to_string(), "content".to_string()],
            query: "machine learning".to_string(),
            options: FtsOptions {
                tokenizer: None,
                language: Some("english".to_string()),
                stemming: true,
                stop_words: true,
                ranking: Some(RankingFunction::Simple),
            },
        };

        let sqlserver_sql = sqlserver_plugin.generate_fts_sql(&fts_with_ranking).unwrap();
        let expected_sqlserver_sql = "SELECT *, ft.RANK as rank FROM articles INNER JOIN CONTAINSTABLE(articles, (title, content), 'machine learning') AS ft ON articles.id = ft.[KEY] ORDER BY rank DESC";
        assert_eq!(sqlserver_sql, expected_sqlserver_sql, "SQL Server FTS with ranking should match exactly");

        // Test PostgreSQL FTS
        let postgres_sql = postgres_plugin.generate_fts_sql(&fts).unwrap();
        let expected_postgres_sql = "SELECT * FROM articles WHERE to_tsvector('english', title || ' ' || content) @@ to_tsquery('machine learning')";
        assert_eq!(postgres_sql, expected_postgres_sql, "PostgreSQL FTS should match exactly");

        // Test SQLite FTS
        let sqlite_sql = sqlite_plugin.generate_fts_sql(&fts).unwrap();
        let expected_sqlite_sql = "SELECT * FROM articles WHERE articles MATCH ?";
        assert_eq!(sqlite_sql, expected_sqlite_sql, "SQLite FTS should match exactly");

        println!("SQL Server FTS: {}", sqlserver_plugin.generate_fts_sql(&fts).unwrap());
        println!("SQL Server FTS with ranking: {}", sqlserver_plugin.generate_fts_sql(&fts_with_ranking).unwrap());
        println!("PostgreSQL FTS: {}", postgres_plugin.generate_fts_sql(&fts).unwrap());
        println!("SQLite FTS: {}", sqlite_plugin.generate_fts_sql(&fts).unwrap());
    }

    #[test]
    fn test_version_aware_error_handling() {
        // Test that older SQL Server versions properly reject unsupported features
        let old_plugin = SqlServerAdvancedPlugin::with_version(DatabaseVersion::new(2008, 10, 0));
        
        let window = WindowFunction {
            function: WindowFunctionType::Lag { offset: 1, default: None },
            partition_by: vec![],
            order_by: vec![],
            frame: None,
        };
        
        // LAG function should fail on SQL Server 2008
        let result = old_plugin.generate_window_sql(&window);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SQL Server 2012"));
        
        // JSON operations should fail on pre-2016 versions
        let json_op = JsonOperation {
            operation_type: JsonOperationType::Extract,
            path: "$.name".to_string(),
            value: None,
        };
        
        let result = old_plugin.generate_json_sql(&json_op);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SQL Server 2016"));
    }

    #[test]
    fn test_aggregation_functions() {
        let postgres_plugin = PostgresAdvancedPlugin::new();
        let sqlite_plugin = SqliteAdvancedPlugin::new();
        let sqlserver_plugin = SqlServerAdvancedPlugin::new();

        // Test GROUP_CONCAT/STRING_AGG
        let group_concat_agg = AdvancedAggregation {
            function: AggregateFunction::GroupConcat {
                column: "name".to_string(),
                separator: Some(", ".to_string()),
            },
            distinct: true,
            filter: None,
            over: None,
        };

        let postgres_sql = postgres_plugin.generate_aggregate_sql(&group_concat_agg).unwrap();
        let expected_postgres_sql = "STRING_AGG(DISTINCT name, ', ')";
        assert_eq!(postgres_sql, expected_postgres_sql, "PostgreSQL STRING_AGG should match exactly");

        let sqlite_sql = sqlite_plugin.generate_aggregate_sql(&group_concat_agg).unwrap();
        let expected_sqlite_sql = "GROUP_CONCAT(DISTINCT name, ', ')";
        assert_eq!(sqlite_sql, expected_sqlite_sql, "SQLite GROUP_CONCAT should match exactly");

        // Test JSON aggregation
        let json_array_agg = AdvancedAggregation {
            function: AggregateFunction::JsonArrayAgg("tags".to_string()),
            distinct: false,
            filter: None,
            over: None,
        };

        let postgres_sql = postgres_plugin.generate_aggregate_sql(&json_array_agg).unwrap();
        let expected_postgres_sql = "JSON_AGG(tags)";
        assert_eq!(postgres_sql, expected_postgres_sql, "PostgreSQL JSON_AGG should match exactly");

        let sqlite_sql = sqlite_plugin.generate_aggregate_sql(&json_array_agg).unwrap();
        let expected_sqlite_sql = "JSON_GROUP_ARRAY(tags)";
        assert_eq!(sqlite_sql, expected_sqlite_sql, "SQLite JSON_GROUP_ARRAY should match exactly");

        // Test window aggregate
        let window_sum = AdvancedAggregation {
            function: AggregateFunction::Sum("amount".to_string()),
            distinct: false,
            filter: None,
            over: Some(WindowFunction {
                function: WindowFunctionType::Sum("amount".to_string()),
                partition_by: vec!["category".to_string()],
                order_by: vec![order_by("date", OrderDirection::Asc)],
                frame: None,
            }),
        };

        let postgres_sql = postgres_plugin.generate_aggregate_sql(&window_sum).unwrap();
        // This should contain both the SUM function and the OVER clause
        assert!(postgres_sql.contains("SUM(amount)"), "Should contain SUM function");
        assert!(postgres_sql.contains("OVER"), "Should contain OVER clause");
        assert!(postgres_sql.contains("PARTITION BY category"), "Should contain PARTITION BY");
        assert!(postgres_sql.contains("ORDER BY date ASC"), "Should contain ORDER BY");

        println!("PostgreSQL STRING_AGG: {}", postgres_plugin.generate_aggregate_sql(&group_concat_agg).unwrap());
        println!("SQLite GROUP_CONCAT: {}", sqlite_plugin.generate_aggregate_sql(&group_concat_agg).unwrap());
        println!("PostgreSQL JSON_AGG: {}", postgres_plugin.generate_aggregate_sql(&json_array_agg).unwrap());
        println!("SQLite JSON_GROUP_ARRAY: {}", sqlite_plugin.generate_aggregate_sql(&json_array_agg).unwrap());
        println!("PostgreSQL Window SUM: {}", postgres_plugin.generate_aggregate_sql(&window_sum).unwrap());
    }

    #[test]
    fn test_feature_support_struct() {
        let supported = FeatureSupport::supported();
        assert!(supported.supported);
        assert!(supported.minimum_version.is_none());

        let version_specific = FeatureSupport::supported_since(DatabaseVersion::new(3, 8, 0));
        assert!(version_specific.supported);
        assert!(version_specific.minimum_version.is_some());

        let not_supported = FeatureSupport::not_supported_with_notes("Feature not implemented");
        assert!(!not_supported.supported);
        assert!(not_supported.notes.is_some());
    }

    #[test]
    fn test_complex_query_generation() {
        let plugin = Box::new(PostgresAdvancedPlugin::new());

        // Create type-safe column definitions
        let level_col = Column::new("level", DataType::Integer).with_table_name("employee_hierarchy");
        let name_col = Column::new("name", DataType::Text).with_table_name("employee_hierarchy");
        let preferences_col = Column::new("preferences", DataType::Text).with_table_name("employees");

        // Build a complex query with multiple advanced features using type-safe columns
        let complex_query = AdvancedQueryBuilder::new()
            .with_plugin(plugin)
            .with_recursive_cte(
                "employee_hierarchy",
                vec!["id".to_string(), "name".to_string(), "manager_id".to_string(), "level".to_string()],
                "SELECT id, name, manager_id, 1 as level FROM employees WHERE manager_id IS NULL
                 UNION ALL
                 SELECT e.id, e.name, e.manager_id, eh.level + 1
                 FROM employees e
                 JOIN employee_hierarchy eh ON e.manager_id = eh.id"
            )
            .with_cte(
                "department_stats",
                "SELECT department, COUNT(*) as emp_count, AVG(salary) as avg_salary FROM employees GROUP BY department"
            )
            .with_row_number(
                "row_num",
                vec![level_col.qualified_name()],
                vec![order_by_column(&name_col, OrderDirection::Asc)]
            )
            .with_json_extract("preferences", "theme")
            .base_query({
                // Create type-safe table references with aliases
                let employee_hierarchy_table = Table::new("employee_hierarchy").as_alias("eh");

                // Build complex query using SQL builder with type-safe columns
                QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
                    .column("eh.*")
                    .column("ds.emp_count")
                    .column("ds.avg_salary")
                    .aliased_column(&preferences_col.as_alias("preferences"))
                    .from_aliased_table_ref(&employee_hierarchy_table)
            })
            .build();

        assert!(complex_query.is_ok(), "Complex query should build successfully");
        let sql = complex_query.unwrap();

        // Verify all components are present (updated for qualified column names)
        let expected_components = vec![
            "WITH RECURSIVE employee_hierarchy (id, name, manager_id, level) AS",
            "department_stats AS (SELECT department, COUNT(*) as emp_count",
            "ROW_NUMBER() OVER (PARTITION BY employee_hierarchy.level ORDER BY employee_hierarchy.name ASC)",
            "column -> '$.theme' AS preferences",
            "AS row_num",
            "eh.*, ds.emp_count, ds.avg_salary",
        ];

        for component in expected_components {
            assert!(sql.contains(component),
                "Generated SQL should contain '{}'\nActual SQL:\n{}", component, sql);
        }

        // Verify structure order
        let with_pos = sql.find("WITH RECURSIVE").expect("Should contain WITH RECURSIVE");
        let main_select_pos = sql.find("eh.*, ds.emp_count, ds.avg_salary").expect("Should contain main SELECT");
        assert!(with_pos < main_select_pos, "CTEs should come before main SELECT");

        println!("Complex Query SQL:\n{}", sql);

        // Verify the SQL is properly formatted (no syntax errors in structure)
        assert!(!sql.contains(",,"), "Should not contain double commas");

        // Count the number of CTEs
        let cte_count = sql.matches(" AS (").count();
        assert_eq!(cte_count, 2, "Should have exactly 2 CTEs");
    }

    #[test]
    fn test_database_specific_sql_differences() {
        // Test that different databases generate different SQL for the same operations
        let sqlite_plugin = Box::new(SqliteAdvancedPlugin::new());
        let postgres_plugin = Box::new(PostgresAdvancedPlugin::new());
        let sqlserver_plugin = Box::new(SqlServerAdvancedPlugin::new());

        // Create type-safe table reference
        let users_table = Table::new("users");

        // Create the same query for all databases using SQL builder
        let base_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .from_table_ref(&users_table);

        let sqlite_query = AdvancedQueryBuilder::new()
            .with_plugin(sqlite_plugin)
            .with_json_extract("settings", "theme")
            .base_query(base_query.clone())
            .build()
            .unwrap();

        let postgres_query = AdvancedQueryBuilder::new()
            .with_plugin(postgres_plugin)
            .with_json_extract("settings", "theme")
            .base_query(base_query.clone())
            .build()
            .unwrap();

        let sqlserver_query = AdvancedQueryBuilder::new()
            .with_plugin(sqlserver_plugin)
            .with_json_extract("settings", "theme")
            .base_query(base_query)
            .build()
            .unwrap();

        // Verify each database generates different JSON syntax
        assert!(sqlite_query.contains("JSON_EXTRACT(column, '$.theme')"), "SQLite should use JSON_EXTRACT");
        assert!(postgres_query.contains("column -> '$.theme'"), "PostgreSQL should use -> operator");
        assert!(sqlserver_query.contains("JSON_VALUE(column, '$.theme')"), "SQL Server should use JSON_VALUE");

        // Verify they're all different
        assert_ne!(sqlite_query, postgres_query, "SQLite and PostgreSQL should generate different SQL");
        assert_ne!(postgres_query, sqlserver_query, "PostgreSQL and SQL Server should generate different SQL");
        assert_ne!(sqlite_query, sqlserver_query, "SQLite and SQL Server should generate different SQL");

        println!("SQLite JSON Query: {}", sqlite_query);
        println!("PostgreSQL JSON Query: {}", postgres_query);
        println!("SQL Server JSON Query: {}", sqlserver_query);
    }

    #[test]
    fn test_schema_integrated_type_safe_queries() {
        // Test type-safe column references without complex schema integration
        // This demonstrates the concept of eliminating literal strings

        // Create mock column objects with table context
        let user_salary = Column::new("salary", DataType::Real).with_table_name("users");
        let user_department_id = Column::new("department_id", DataType::Integer).with_table_name("users");
        let user_hire_date = Column::new("hire_date", DataType::Text).with_table_name("users");

        // Test 1: Type-safe window functions (using existing methods)
        let plugin = Box::new(PostgresAdvancedPlugin::new());

        // Create type-safe table reference
        let users_table = Table::new("users");

        // Build the base query using the SQL builder
        let base_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .from_table_ref(&users_table);

        let query = AdvancedQueryBuilder::new()
            .with_plugin(plugin)
            .with_row_number(
                "salary_rank",
                vec![user_department_id.qualified_name()],
                vec![order_by_column(&user_salary, OrderDirection::Desc)]
            )
            .with_lag(
                "prev_salary",
                1,
                Some(Value::Real(0.0)),
                vec![user_department_id.qualified_name()],
                vec![order_by_column(&user_hire_date, OrderDirection::Asc)]
            )
            .base_query(base_query)
            .build();

        assert!(query.is_ok());
        let sql = query.unwrap();

        // Verify type-safe column references are used
        assert!(sql.contains("users.department_id"), "Should use qualified column name");
        assert!(sql.contains("ROW_NUMBER() OVER"), "Should contain window function");
        assert!(sql.contains("LAG(column, 1"), "Should contain LAG function");

        println!("Type-safe Window Function SQL:\n{}\n", sql);
    }

    #[test]
    fn test_schema_integrated_json_operations() {
        // Test type-safe JSON operations with schema-aware columns

        // Create mock columns with JSON schema information
        let profile_column = Column::new("profile", DataType::Text)
            .with_table_name("users")
            .with_json_schema(json!({
                "type": "object",
                "properties": {
                    "theme": {"type": "string"},
                    "notifications": {"type": "boolean"},
                    "settings": {
                        "type": "object",
                        "properties": {
                            "language": {"type": "string"},
                            "timezone": {"type": "string"}
                        }
                    }
                }
            }));

        let metadata_column = Column::new("metadata", DataType::Text)
            .with_table_name("users")
            .with_json_schema(json!({
                "type": "object",
                "properties": {
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "score": {"type": "number"}
                }
            }));

        // Test type-safe JSON operations
        let sqlite_plugin = Box::new(SqliteAdvancedPlugin::new());
        let postgres_plugin = Box::new(PostgresAdvancedPlugin::new());

        // Create type-safe table and column references
        let users_table = Table::new("users");
        let id_col = Column::new("id", DataType::Integer).with_table_name("users");

        // Build the base query using the SQL builder
        let base_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&id_col)
            .from_table_ref(&users_table);

        // SQLite JSON extraction with schema validation
        let sqlite_query = AdvancedQueryBuilder::new()
            .with_plugin(sqlite_plugin)
            .with_json_extract_typed("user_theme", &profile_column, "theme")
            .with_json_extract_typed("user_language", &profile_column, "settings.language")
            .with_json_array_agg_typed("all_tags", &metadata_column, "tags")
            .base_query(base_query.clone())
            .build();

        assert!(sqlite_query.is_ok());
        let sqlite_sql = sqlite_query.unwrap();

        // Verify JSON operations are generated correctly
        assert!(sqlite_sql.contains("theme"), "Should contain theme extraction");
        assert!(sqlite_sql.contains("language"), "Should contain language extraction");
        assert!(sqlite_sql.contains("tags"), "Should contain tags aggregation");

        // PostgreSQL JSON operations
        let postgres_query = AdvancedQueryBuilder::new()
            .with_plugin(postgres_plugin)
            .with_json_extract_typed("user_theme", &profile_column, "theme")
            .with_json_set_typed("updated_score", &metadata_column, "score", Value::Real(95.5))
            .base_query(base_query)
            .build();

        assert!(postgres_query.is_ok());
        let postgres_sql = postgres_query.unwrap();

        // Verify PostgreSQL-specific JSON syntax is generated
        assert!(postgres_sql.contains("theme"), "Should contain theme extraction");
        assert!(postgres_sql.contains("score"), "Should contain score update");

        println!("SQLite JSON SQL:\n{}\n", sqlite_sql);
        println!("PostgreSQL JSON SQL:\n{}\n", postgres_sql);
    }

    #[test]
    fn test_schema_integrated_cte_with_type_safety() {
        // Test type-safe CTE generation with column references

        // Create mock columns for departments table
        let dept_id = Column::new("id", DataType::Integer).with_table_name("departments");
        let dept_name = Column::new("name", DataType::Text).with_table_name("departments");
        let dept_parent_id = Column::new("parent_id", DataType::Integer).with_table_name("departments");

        // Create columns for the CTE result
        let level_col = Column::new("level", DataType::Integer).with_table_name("dept_hierarchy");
        let path_col = Column::new("path", DataType::Text).with_table_name("dept_hierarchy");

        // Create type-safe table references with aliases
        let departments_table = Table::new("departments");
        let dept_alias = departments_table.as_alias("d");
        let dept_hierarchy_alias = Table::new("dept_hierarchy").as_alias("dh");

        // Create aliased column references for the recursive part
        let dept_id_aliased = Column::new("id", DataType::Integer).with_table_name("d");
        let dept_name_aliased = Column::new("name", DataType::Text).with_table_name("d");
        let dept_parent_id_aliased = Column::new("parent_id", DataType::Integer).with_table_name("d");
        let dh_level = Column::new("level", DataType::Integer).with_table_name("dh");
        let dh_path = Column::new("path", DataType::Text).with_table_name("dh");

        // Create type-safe recursive CTE for organizational hierarchy
        let plugin = Box::new(SqliteAdvancedPlugin::new());

        // Build the recursive CTE using the type-safe query builder
        let base_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&dept_id)
            .column_ref(&dept_name)
            .column_ref(&dept_parent_id)
            .column_expr_as("1", "level")
            .column_ref_as(&dept_name, "path")
            .from_table_ref(&departments_table);

        // Create type-safe expressions - NO string literals!
        let level_plus_one = dh_level.add(1);  // Type-safe arithmetic from schema property
        let path_concat_name = dh_path.concat("/").concat_column(&dept_name_aliased);  // Type-safe concatenation

        let recursive_part = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&dept_id_aliased)
            .column_ref(&dept_name_aliased)
            .column_ref(&dept_parent_id_aliased)
            .column_type_safe_expr(&level_plus_one)  // Type-safe expression
            .column_type_safe_expr(&path_concat_name)  // Type-safe expression
            .from_aliased_table_ref(&dept_alias);

        // Note: union_all and with_recursive_cte are not yet implemented in the type-safe interface
        // For now, we'll build the CTE SQL manually using type-safe components
        let base_sql = base_query.to_sql();
        let recursive_sql = recursive_part.to_sql();
        let cte_sql = format!("{} UNION ALL {}", base_sql, recursive_sql);

        let dept_hierarchy_cte = CteDefinition::new_with_columns(
            "dept_hierarchy",
            &[&dept_id, &dept_name, &dept_parent_id, &level_col, &path_col],
            cte_sql,
            true,
        );

        // Create type-safe table reference for the CTE result
        let dept_hierarchy_table = Table::new("dept_hierarchy");

        // Build the base query using the SQL builder with type-safe methods
        let final_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .from_table_ref(&dept_hierarchy_table)
            .order_by_column(&level_col, crate::orm::query::OrderDirection::Asc)
            .order_by_column(&path_col, crate::orm::query::OrderDirection::Asc);

        let query = AdvancedQueryBuilder::new()
            .with_plugin(plugin)
            .with_cte_definition(dept_hierarchy_cte)
            .with_row_number(
                "dept_rank",
                vec![level_col.qualified_name()],
                vec![order_by_column(&path_col, OrderDirection::Asc)]
            )
            .base_query(final_query)
            .build();

        assert!(query.is_ok());
        let sql = query.unwrap();

        // Verify schema-aware CTE generation
        assert!(sql.contains("WITH RECURSIVE dept_hierarchy"));
        assert!(sql.contains("departments.id"));
        assert!(sql.contains("departments.name"));
        assert!(sql.contains("departments.parent_id"));
        assert!(sql.contains("ROW_NUMBER() OVER"));

        println!("Schema-aware Recursive CTE SQL:\n{}\n", sql);
    }

    #[test]
    fn test_type_safe_query_composition_and_reuse() {
        use crate::orm::query::{QueryBuilder, OrderDirection};
        use crate::orm::schema::Column;
        use crate::sqlite::types::DataType;

        // Define reusable schema components
        let users_table = Table::new("users");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let user_email = Column::new("email", DataType::Text).with_table_name("users");

        // 1. Build a reusable base user query
        let base_user_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .column_ref(&user_email)
            .from_table_ref(&users_table);

        // Create type-safe references for the CTE
        let user_base_table = Table::new("user_base").as_alias("u");
        let u_id = Column::new("id", DataType::Integer).with_table_name("u");
        let u_name = Column::new("name", DataType::Text).with_table_name("u");
        let u_email = Column::new("email", DataType::Text).with_table_name("u");

        // 2. Test simple query composition with CTE
        let query_with_cte = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&u_id)
            .column_ref(&u_name)
            .column_ref(&u_email)
            .from_aliased_table_ref(&user_base_table)
            .with_cte("user_base", base_user_query.clone())
            .order_by_column(&u_name, OrderDirection::Asc);

        // 3. Generate SQL and verify composition
        let sql = query_with_cte.to_sql();
        println!("Type-safe Composed Query SQL:\n{}\n", sql);

        // Verify the composed query contains all expected elements
        assert!(sql.contains("WITH user_base AS"));
        assert!(sql.contains(&user_id.qualified_name()));
        assert!(sql.contains(&user_name.qualified_name()));
        assert!(sql.contains(&user_email.qualified_name()));
        assert!(sql.contains("ORDER BY u.name ASC"));

        // 4. Test UNION composition for combining different data sources
        let active_users = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .column_expr_as("'active'", "status")
            .from_table_ref(&users_table);

        let inactive_users = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .column_expr_as("'inactive'", "status")
            .from_table_ref(&users_table);

        let all_users_with_status = active_users
            .union_all(inactive_users)
            .order_by(&user_name.qualified_name(), OrderDirection::Asc);

        let union_sql = all_users_with_status.to_sql();
        println!("UNION Query SQL:\n{}\n", union_sql);

        // Verify UNION composition
        assert!(union_sql.contains("UNION ALL"));
        assert!(union_sql.contains("'active' AS status"));
        assert!(union_sql.contains("'inactive' AS status"));
        assert!(union_sql.contains(&format!("ORDER BY {}", user_name.qualified_name())));

        // 5. Test query reuse with different modifications
        let limited_users = base_user_query
            .clone()
            .limit(10)
            .order_by(&user_name.qualified_name(), OrderDirection::Desc);

        let limited_sql = limited_users.to_sql();
        println!("Reused Query with Limit:\n{}\n", limited_sql);

        // Verify reuse works correctly
        assert!(limited_sql.contains("LIMIT 10"));
        assert!(limited_sql.contains(&format!("ORDER BY {} DESC", user_name.qualified_name())));
        assert!(limited_sql.contains(&user_id.qualified_name()));
    }

    #[test]
    fn test_type_safe_column_interface() {
        // Test the new type-safe column interface

        // Create type-safe table and column definitions
        let users_table = Table::new("users");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let user_email = Column::new("email", DataType::Text).with_table_name("users");

        // Test 1: Basic column references
        let query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .column_ref_as(&user_email, "email_address")
            .from_table_ref(&users_table);

        let sql = query.to_sql();
        assert!(sql.contains("users.id"), "Should contain qualified column name");
        assert!(sql.contains("users.name"), "Should contain qualified column name");
        assert!(sql.contains("users.email AS email_address"), "Should contain aliased column");

        // Test 2: Multiple column references at once
        let query2 = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_refs(&[&user_id, &user_name, &user_email])
            .from_table_ref(&users_table);

        let sql2 = query2.to_sql();
        assert!(sql2.contains("users.id"), "Should contain all qualified column names");
        assert!(sql2.contains("users.name"), "Should contain all qualified column names");
        assert!(sql2.contains("users.email"), "Should contain all qualified column names");

        // Test 3: Aliased columns using AliasedColumn type
        let aliased_name = user_name.as_alias("full_name");
        let aliased_email = user_email.as_alias("contact_email");

        let query3 = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .aliased_column(&aliased_name)
            .aliased_column(&aliased_email)
            .from_table_ref(&users_table);

        let sql3 = query3.to_sql();
        assert!(sql3.contains("users.id"), "Should contain unaliased column");
        assert!(sql3.contains("users.name AS full_name"), "Should contain aliased column");
        assert!(sql3.contains("users.email AS contact_email"), "Should contain aliased column");

        // Test 4: Multiple aliased columns at once
        let query4 = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .aliased_columns(&[&aliased_name, &aliased_email])
            .from_table_ref(&users_table);

        let sql4 = query4.to_sql();
        assert!(sql4.contains("users.name AS full_name"), "Should contain all aliased columns");
        assert!(sql4.contains("users.email AS contact_email"), "Should contain all aliased columns");

        // Test 5: Mixed column types (backward compatibility)
        let query5 = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column("created_at")  // String literal for backward compatibility
            .column_expr_as("COUNT(*)", "total_count")
            .from_table_ref(&users_table);

        let sql5 = query5.to_sql();
        assert!(sql5.contains("users.id"), "Should contain type-safe column");
        assert!(sql5.contains("created_at"), "Should contain string literal column");
        assert!(sql5.contains("COUNT(*) AS total_count"), "Should contain expression with alias");

        println!("Type-safe Column Interface Tests:");
        println!("Basic columns: {}", sql);
        println!("Multiple columns: {}", sql2);
        println!("Aliased columns: {}", sql3);
        println!("Multiple aliased: {}", sql4);
        println!("Mixed types: {}", sql5);
    }

    #[test]
    fn test_comprehensive_type_safe_interface() {
        // Test comprehensive type-safe interface across all query builder features

        // Create the type-safe schema - all tables and columns from data model schema
        let schema = TypeSafeSchema::new();

        // Access tables from schema properties - NO string literals!
        let users_table = &schema.tables.users.table;
        let departments_table = &schema.tables.departments.table;

        // Access columns from schema properties - NO string literals!
        let user_id = &schema.tables.users.id;
        let user_name = &schema.tables.users.name;
        let user_email = &schema.tables.users.email;
        let user_created_at = &schema.tables.users.created_at;

        let dept_id = &schema.tables.departments.id;
        let dept_name = &schema.tables.departments.name;

        // Test 1: Basic type-safe query with columns, from, and order by
        let basic_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref_as(&user_name, "full_name")
            .column_ref(&user_email)
            .from_table_ref(&users_table)
            .order_by_column(&user_created_at, crate::orm::query::OrderDirection::Desc);

        let basic_sql = basic_query.to_sql();
        assert!(basic_sql.contains("users.id"), "Should contain qualified column");
        assert!(basic_sql.contains("users.name AS full_name"), "Should contain aliased column");
        assert!(basic_sql.contains("FROM users"), "Should contain type-safe table reference");
        assert!(basic_sql.contains("ORDER BY users.created_at DESC"), "Should contain type-safe order by");

        // Test 1b: Type-safe table with alias
        let aliased_users = users_table.as_alias("u");
        let aliased_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .from_aliased_table(&aliased_users);

        let aliased_sql = aliased_query.to_sql();
        assert!(aliased_sql.contains("FROM users AS u"), "Should contain aliased table reference");

        // Test 2: Type-safe CTE creation
        let cte_columns = vec![user_id, user_name, dept_name];
        let cte_def = CteDefinition::new_with_columns(
            "user_dept_cte",
            &cte_columns,
            "SELECT u.id, u.name, d.name FROM users u JOIN departments d ON u.dept_id = d.id",
            false,
        );

        assert_eq!(cte_def.name, "user_dept_cte");
        assert!(cte_def.columns.as_ref().unwrap().contains(&"id".to_string()));
        assert!(cte_def.columns.as_ref().unwrap().contains(&"name".to_string()));
        assert!(!cte_def.recursive);

        // Test 3: Advanced query builder with type-safe methods
        let plugin = Box::new(SqliteAdvancedPlugin::new());
        let advanced_query = AdvancedQueryBuilder::new()
            .with_plugin(plugin)
            .column_ref(&user_id)
            .column_ref_as(&user_name, "user_name")
            .from_table_name_as("users", "u")
            .order_by_column(&user_id, crate::orm::query::OrderDirection::Asc);

        let advanced_sql = advanced_query.to_sql();
        assert!(advanced_sql.contains("users.id"), "Advanced query should contain qualified columns");
        assert!(advanced_sql.contains("users.name AS user_name"), "Advanced query should contain aliases");

        // Test 4: Mixed type-safe and string methods (backward compatibility)
        let mixed_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)  // Type-safe
            .column("COUNT(*) as total")  // String literal
            .column_expr_as("MAX(created_at)", "latest")  // Expression
            .from_table_ref(&users_table)  // Type-safe table
            .order_by_column(&user_id, crate::orm::query::OrderDirection::Asc);  // Type-safe

        let mixed_sql = mixed_query.to_sql();
        assert!(mixed_sql.contains("users.id"), "Should contain type-safe column");
        assert!(mixed_sql.contains("COUNT(*) as total"), "Should contain string literal");
        assert!(mixed_sql.contains("MAX(created_at) AS latest"), "Should contain expression with alias");
        assert!(mixed_sql.contains("ORDER BY users.id ASC"), "Should contain type-safe order by");

        // Test 5: Aliased columns
        let aliased_name = user_name.as_alias("display_name");
        let aliased_email = user_email.as_alias("contact_email");

        let aliased_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .aliased_column(&aliased_name)
            .aliased_column(&aliased_email)
            .from_table_ref(&users_table);

        let aliased_sql = aliased_query.to_sql();
        assert!(aliased_sql.contains("users.name AS display_name"), "Should contain aliased column");
        assert!(aliased_sql.contains("users.email AS contact_email"), "Should contain aliased column");

        // Test 6: Type-safe table operations
        let schema_table = Table::new("users").with_schema("public");
        let schema_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .from_table_ref(&schema_table);

        let schema_sql = schema_query.to_sql();
        assert!(schema_sql.contains("FROM public.users"), "Should contain schema-qualified table");

        // Test 7: Backward compatibility with string table names
        let compat_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .from_table_name("users")  // String-based method
            .from_table_name_as("departments", "d");  // String-based with alias

        let compat_sql = compat_query.to_sql();
        assert!(compat_sql.contains("FROM departments AS d"), "Should support string-based table methods");

        println!("Comprehensive Type-Safe Interface Tests:");
        println!("Basic query: {}", basic_sql);
        println!("Aliased table query: {}", aliased_sql);
        println!("CTE definition: {:?}", cte_def);
        println!("Advanced query: {}", advanced_sql);
        println!("Mixed query: {}", mixed_sql);
        println!("Aliased columns query: {}", aliased_sql);
        println!("Schema table query: {}", schema_sql);
        println!("Backward compatibility query: {}", compat_sql);
    }

    // Helper functions for tests
    fn order_by(column: &str, direction: OrderDirection) -> OrderByClause {
        OrderByClause {
            column: column.to_string(),
            direction,
            nulls: None,
        }
    }

    fn order_by_with_nulls(column: &str, direction: OrderDirection, nulls: NullsOrder) -> OrderByClause {
        OrderByClause {
            column: column.to_string(),
            direction,
            nulls: Some(nulls),
        }
    }

    fn order_by_column(column: &Column, direction: OrderDirection) -> OrderByClause {
        OrderByClause {
            column: column.qualified_name(),
            direction,
            nulls: None,
        }
    }
}
