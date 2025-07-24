pub mod ast;
pub mod lexer;
pub mod sql_parser;

pub use ast::*;
pub use sql_parser::SqlParser;

use crate::error::{SqlError, SqlResult};

/// Parse SQL statement into AST
pub fn parse_sql(input: &str) -> SqlResult<Statement> {
    let parser = SqlParser::new();
    parser.parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let sql = "SELECT * FROM users";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        
        if let Ok(Statement::Select(select)) = result {
            assert_eq!(select.columns.len(), 1);
            assert!(matches!(select.columns[0], SelectColumn::Wildcard));
            assert_eq!(select.from.as_ref().unwrap().table, "users");
        } else {
            panic!("Expected SELECT statement");
        }
    }
    
    #[test]
    fn test_parse_create_table() {
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        
        if let Ok(Statement::CreateTable(create)) = result {
            assert_eq!(create.table_name, "users");
            assert_eq!(create.columns.len(), 2);
        } else {
            panic!("Expected CREATE TABLE statement");
        }
    }
    
    #[test]
    fn test_parse_insert() {
        let sql = "INSERT INTO users (name, age) VALUES ('Alice', 30)";
        let result = parse_sql(sql);
        assert!(result.is_ok());
        
        if let Ok(Statement::Insert(insert)) = result {
            assert_eq!(insert.table, "users");
            assert_eq!(insert.columns.as_ref().unwrap().len(), 2);
        } else {
            panic!("Expected INSERT statement");
        }
    }
}
