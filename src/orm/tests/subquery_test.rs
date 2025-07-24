/// Comprehensive tests for subquery support and composable query building
#[cfg(test)]
mod tests {
    use crate::orm::query::{QueryBuilder, OrderDirection};
    use crate::orm::schema::{Column, Table};
    use crate::sqlite::types::DataType;
    use crate::sqlite::parser::ast::{Expression, BinaryOp};

    #[test]
    fn test_subquery_in_from_clause() {
        // Create type-safe schema components
        let users_table = Table::new("users");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let user_department_id = Column::new("department_id", DataType::Integer).with_table_name("users");

        // Create a subquery for active users
        let active_users_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .column_ref(&user_department_id)
            .from_table_ref(&users_table)
            .where_clause(Expression::Binary {
                left: Box::new(Expression::Column("status".to_string())),
                op: crate::sqlite::parser::ast::BinaryOp::Eq,
                right: Box::new(Expression::Literal(crate::sqlite::types::Value::Text("active".to_string()))),
            });

        // Use the subquery in FROM clause
        let main_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column("*")
            .from_subquery_as(active_users_subquery, "active_users")
            .order_by("name", OrderDirection::Asc);

        let sql = main_query.to_sql();
        println!("Subquery in FROM clause SQL:\n{}\n", sql);

        // Verify the SQL contains subquery structure
        assert!(sql.contains("FROM ("), "Should contain subquery in FROM");
        assert!(sql.contains(") AS active_users"), "Should contain subquery alias");
        assert!(sql.contains("users.id"), "Should contain qualified column names");
        assert!(sql.contains("ORDER BY name ASC"), "Should contain ORDER BY");
    }

    #[test]
    fn test_exists_subquery() {
        // Create schema components
        let users_table = Table::new("users");
        let orders_table = Table::new("orders");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let order_user_id = Column::new("user_id", DataType::Integer).with_table_name("orders");

        // Create EXISTS subquery to find users with orders
        let has_orders_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column("1")
            .from_table_ref(&orders_table)
            .where_clause(Expression::Binary {
                left: Box::new(Expression::Column(order_user_id.qualified_name())),
                op: crate::sqlite::parser::ast::BinaryOp::Eq,
                right: Box::new(Expression::Column(user_id.qualified_name())),
            });

        // Main query using EXISTS
        let main_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .from_table_ref(&users_table)
            .where_exists(has_orders_subquery);

        let sql = main_query.to_sql();
        println!("EXISTS subquery SQL:\n{}\n", sql);

        // Verify EXISTS structure
        assert!(sql.contains("WHERE EXISTS ("), "Should contain EXISTS clause");
        assert!(sql.contains("FROM orders"), "Should contain orders table in subquery");
        assert!(sql.contains("users.id"), "Should contain qualified column names");
    }

    #[test]
    fn test_not_exists_subquery() {
        // Create schema components
        let users_table = Table::new("users");
        let orders_table = Table::new("orders");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let order_user_id = Column::new("user_id", DataType::Integer).with_table_name("orders");

        // Create NOT EXISTS subquery to find users without orders
        let no_orders_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column("1")
            .from_table_ref(&orders_table)
            .where_clause(Expression::Binary {
                left: Box::new(Expression::Column(order_user_id.qualified_name())),
                op: crate::sqlite::parser::ast::BinaryOp::Eq,
                right: Box::new(Expression::Column(user_id.qualified_name())),
            });

        // Main query using NOT EXISTS
        let main_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .from_table_ref(&users_table)
            .where_not_exists(no_orders_subquery);

        let sql = main_query.to_sql();
        println!("NOT EXISTS subquery SQL:\n{}\n", sql);

        // Verify NOT EXISTS structure
        assert!(sql.contains("WHERE NOT EXISTS ("), "Should contain NOT EXISTS clause");
        assert!(sql.contains("FROM orders"), "Should contain orders table in subquery");
    }

    #[test]
    fn test_in_subquery() {
        // Create schema components
        let users_table = Table::new("users");
        let departments_table = Table::new("departments");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let user_department_id = Column::new("department_id", DataType::Integer).with_table_name("users");
        let dept_id = Column::new("id", DataType::Integer).with_table_name("departments");

        // Create subquery for active department IDs
        let active_dept_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&dept_id)
            .from_table_ref(&departments_table)
            .where_clause(Expression::Binary {
                left: Box::new(Expression::Column("status".to_string())),
                op: crate::sqlite::parser::ast::BinaryOp::Eq,
                right: Box::new(Expression::Literal(crate::sqlite::types::Value::Text("active".to_string()))),
            });

        // Main query using IN subquery
        let main_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .from_table_ref(&users_table)
            .where_column_in_subquery(&user_department_id, active_dept_subquery);

        let sql = main_query.to_sql();
        println!("IN subquery SQL:\n{}\n", sql);

        // Verify IN subquery structure
        assert!(sql.contains("users.department_id IN ("), "Should contain IN subquery");
        assert!(sql.contains("FROM departments"), "Should contain departments table in subquery");
    }

    #[test]
    fn test_not_in_subquery() {
        // Create schema components
        let users_table = Table::new("users");
        let blocked_users_table = Table::new("blocked_users");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let blocked_user_id = Column::new("user_id", DataType::Integer).with_table_name("blocked_users");

        // Create subquery for blocked user IDs
        let blocked_users_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&blocked_user_id)
            .from_table_ref(&blocked_users_table);

        // Main query using NOT IN subquery
        let main_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .from_table_ref(&users_table)
            .where_column_not_in_subquery(&user_id, blocked_users_subquery);

        let sql = main_query.to_sql();
        println!("NOT IN subquery SQL:\n{}\n", sql);

        // Verify NOT IN subquery structure
        assert!(sql.contains("users.id NOT IN ("), "Should contain NOT IN subquery");
        assert!(sql.contains("FROM blocked_users"), "Should contain blocked_users table in subquery");
    }

    #[test]
    fn test_scalar_subquery_in_select() {
        // Create schema components
        let users_table = Table::new("users");
        let orders_table = Table::new("orders");
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let order_user_id = Column::new("user_id", DataType::Integer).with_table_name("orders");
        let order_total = Column::new("total", DataType::Real).with_table_name("orders");

        // Create scalar subquery for total order amount
        let total_orders_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column("SUM(orders.total)")
            .from_table_ref(&orders_table)
            .where_clause(Expression::Binary {
                left: Box::new(Expression::Column(order_user_id.qualified_name())),
                op: crate::sqlite::parser::ast::BinaryOp::Eq,
                right: Box::new(Expression::Column(user_id.qualified_name())),
            });

        // Main query with scalar subquery in SELECT
        let main_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .column_subquery(total_orders_subquery, "total_orders")
            .from_table_ref(&users_table);

        let sql = main_query.to_sql();
        println!("Scalar subquery in SELECT SQL:\n{}\n", sql);

        // Verify scalar subquery structure
        assert!(sql.contains("(SELECT SUM(orders.total)"), "Should contain scalar subquery");
        assert!(sql.contains(") AS total_orders"), "Should contain subquery alias");
        assert!(sql.contains("FROM orders"), "Should contain orders table in subquery");
    }

    #[test]
    fn test_complex_nested_subqueries() {
        // Create schema components
        let users_table = Table::new("users");
        let orders_table = Table::new("orders");
        let order_items_table = Table::new("order_items");
        let products_table = Table::new("products");
        
        let user_id = Column::new("id", DataType::Integer).with_table_name("users");
        let user_name = Column::new("name", DataType::Text).with_table_name("users");
        let order_id = Column::new("id", DataType::Integer).with_table_name("orders");
        let order_user_id = Column::new("user_id", DataType::Integer).with_table_name("orders");
        let item_order_id = Column::new("order_id", DataType::Integer).with_table_name("order_items");
        let item_product_id = Column::new("product_id", DataType::Integer).with_table_name("order_items");
        let product_id = Column::new("id", DataType::Integer).with_table_name("products");

        // Innermost subquery: expensive products
        let expensive_products_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&product_id)
            .from_table_ref(&products_table)
            .where_clause(Expression::Binary {
                left: Box::new(Expression::Column("price".to_string())),
                op: crate::sqlite::parser::ast::BinaryOp::Gt,
                right: Box::new(Expression::Literal(crate::sqlite::types::Value::Real(100.0))),
            });

        // Middle subquery: orders with expensive products
        let expensive_orders_subquery = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&item_order_id)
            .from_table_ref(&order_items_table)
            .where_column_in_subquery(&item_product_id, expensive_products_subquery);

        // Outer query: users who bought expensive products
        let main_query = QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
            .column_ref(&user_id)
            .column_ref(&user_name)
            .from_table_ref(&users_table)
            .where_exists(
                QueryBuilder::<Vec<crate::orm::QueryRow>>::select()
                    .column("1")
                    .from_table_ref(&orders_table)
                    .where_clause(Expression::Binary {
                        left: Box::new(Expression::Column(order_user_id.qualified_name())),
                        op: crate::sqlite::parser::ast::BinaryOp::Eq,
                        right: Box::new(Expression::Column(user_id.qualified_name())),
                    })
                    .where_column_in_subquery(&order_id, expensive_orders_subquery)
            );

        let sql = main_query.to_sql();
        println!("Complex nested subqueries SQL:\n{}\n", sql);

        // Verify nested structure
        assert!(sql.contains("WHERE EXISTS ("), "Should contain EXISTS clause");
        assert!(sql.contains("orders.id IN ("), "Should contain IN subquery");
        assert!(sql.contains("order_items.product_id IN ("), "Should contain nested IN subquery");
        assert!(sql.contains("FROM products"), "Should contain innermost table");
        assert!(sql.contains("price"), "Should contain innermost condition");
    }
}
