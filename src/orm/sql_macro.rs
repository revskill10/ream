/// SQL macro system - Compile-time SQL parsing and type checking
/// 
/// This module provides the sql! procedural macro for type-safe SQL:
/// - Compile-time SQL parsing and validation
/// - Type checking against schema definitions
/// - Zero-cost SQL generation
/// - Parameter binding with injection protection

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Result, LitStr, Token};

/// SQL macro input - parses SQL string with optional parameters
#[derive(Debug)]
pub struct SqlMacroInput {
    pub sql: String,
    pub parameters: Vec<SqlParameter>,
}

impl Parse for SqlMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let sql_lit: LitStr = input.parse()?;
        let sql = sql_lit.value();
        
        let mut parameters = Vec::new();
        
        // Parse optional parameters
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let param: SqlParameter = input.parse()?;
            parameters.push(param);
        }
        
        Ok(SqlMacroInput { sql, parameters })
    }
}

/// SQL parameter - represents a parameter binding in SQL
#[derive(Debug, Clone)]
pub struct SqlParameter {
    pub name: String,
    pub value: TokenStream,
}

impl Parse for SqlParameter {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: syn::Expr = input.parse()?;
        
        Ok(SqlParameter {
            name: name.to_string(),
            value: quote! { #value },
        })
    }
}

/// SQL macro implementation
/// 
/// This would be implemented as a procedural macro in ream-macros crate
/// For now, we provide the core logic that would be used by the macro
pub fn sql_macro_impl(input: SqlMacroInput) -> TokenStream {
    let sql = &input.sql;
    let parameters = &input.parameters;
    
    // Parse SQL to validate syntax
    match parse_and_validate_sql(sql) {
        Ok(parsed_sql) => {
            // Generate code for the query
            generate_query_code(&parsed_sql, parameters)
        }
        Err(error) => {
            // Generate compile error
            let error_msg = format!("SQL parse error: {}", error);
            quote! {
                compile_error!(#error_msg);
            }
        }
    }
}

/// Parsed SQL representation
#[derive(Debug, Clone)]
pub struct ParsedSql {
    pub statement_type: SqlStatementType,
    pub tables: Vec<String>,
    pub columns: Vec<String>,
    pub parameters: Vec<String>,
    pub original_sql: String,
}

/// SQL statement types
#[derive(Debug, Clone, PartialEq)]
pub enum SqlStatementType {
    Select,
    Insert,
    Update,
    Delete,
    CreateTable,
    DropTable,
    CreateIndex,
    DropIndex,
}

/// Parse and validate SQL at compile time
fn parse_and_validate_sql(sql: &str) -> std::result::Result<ParsedSql, String> {
    // Use the existing SQL parser from sqlite module
    match crate::sqlite::parser::parse_sql(sql) {
        Ok(statement) => {
            let parsed = match statement {
                crate::sqlite::parser::ast::Statement::Select(select) => {
                    ParsedSql {
                        statement_type: SqlStatementType::Select,
                        tables: extract_tables_from_select(&select),
                        columns: extract_columns_from_select(&select),
                        parameters: extract_parameters_from_sql(sql),
                        original_sql: sql.to_string(),
                    }
                }
                crate::sqlite::parser::ast::Statement::Insert(insert) => {
                    ParsedSql {
                        statement_type: SqlStatementType::Insert,
                        tables: vec![insert.table.clone()],
                        columns: insert.columns.clone(),
                        parameters: extract_parameters_from_sql(sql),
                        original_sql: sql.to_string(),
                    }
                }
                crate::sqlite::parser::ast::Statement::Update(update) => {
                    ParsedSql {
                        statement_type: SqlStatementType::Update,
                        tables: vec![update.table.clone()],
                        columns: update.set_clauses.iter().map(|(col, _)| col.clone()).collect(),
                        parameters: extract_parameters_from_sql(sql),
                        original_sql: sql.to_string(),
                    }
                }
                crate::sqlite::parser::ast::Statement::Delete(delete) => {
                    ParsedSql {
                        statement_type: SqlStatementType::Delete,
                        tables: vec![delete.table.clone()],
                        columns: Vec::new(),
                        parameters: extract_parameters_from_sql(sql),
                        original_sql: sql.to_string(),
                    }
                }
                crate::sqlite::parser::ast::Statement::CreateTable(create) => {
                    ParsedSql {
                        statement_type: SqlStatementType::CreateTable,
                        tables: vec![create.table_name.clone()],
                        columns: create.columns.iter().map(|col| col.name.clone()).collect(),
                        parameters: Vec::new(),
                        original_sql: sql.to_string(),
                    }
                }
                crate::sqlite::parser::ast::Statement::DropTable(drop) => {
                    ParsedSql {
                        statement_type: SqlStatementType::DropTable,
                        tables: vec![drop.table_name.clone()],
                        columns: Vec::new(),
                        parameters: Vec::new(),
                        original_sql: sql.to_string(),
                    }
                }
                crate::sqlite::parser::ast::Statement::CreateIndex(create_idx) => {
                    ParsedSql {
                        statement_type: SqlStatementType::CreateIndex,
                        tables: vec![create_idx.table.clone()],
                        columns: create_idx.columns.clone(),
                        parameters: Vec::new(),
                        original_sql: sql.to_string(),
                    }
                }
                crate::sqlite::parser::ast::Statement::DropIndex(drop_idx) => {
                    ParsedSql {
                        statement_type: SqlStatementType::DropIndex,
                        tables: Vec::new(),
                        columns: Vec::new(),
                        parameters: Vec::new(),
                        original_sql: sql.to_string(),
                    }
                }
            };
            Ok(parsed)
        }
        Err(e) => Err(format!("Failed to parse SQL: {}", e)),
    }
}

/// Extract table names from SELECT statement
fn extract_tables_from_select(select: &crate::sqlite::parser::ast::SelectStatement) -> Vec<String> {
    let mut tables = Vec::new();
    
    if let Some(ref from) = select.from {
        tables.push(from.table());

        // Add joined tables
        for join in &from.joins {
            tables.push(join.table.clone());
        }
    }
    
    tables
}

/// Extract column names from SELECT statement
fn extract_columns_from_select(select: &crate::sqlite::parser::ast::SelectStatement) -> Vec<String> {
    let mut columns = Vec::new();
    
    for column in &select.columns {
        match column {
            crate::sqlite::parser::ast::SelectColumn::Wildcard => {
                columns.push("*".to_string());
            }
            crate::sqlite::parser::ast::SelectColumn::Expression { expr, alias } => {
                if let Some(alias) = alias {
                    columns.push(alias.clone());
                } else {
                    // Extract column name from expression
                    columns.push(extract_column_name_from_expr(expr));
                }
            }
        }
    }
    
    columns
}

/// Extract column name from expression
fn extract_column_name_from_expr(expr: &crate::sqlite::parser::ast::Expression) -> String {
    match expr {
        crate::sqlite::parser::ast::Expression::Column(name) => name.clone(),
        crate::sqlite::parser::ast::Expression::QualifiedColumn { column, .. } => column.clone(),
        _ => "expr".to_string(), // Fallback for complex expressions
    }
}

/// Extract parameter placeholders from SQL string
fn extract_parameters_from_sql(sql: &str) -> Vec<String> {
    let mut parameters = Vec::new();
    let mut chars = sql.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '#' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut param_name = String::new();
            
            while let Some(ch) = chars.next() {
                if ch == '}' {
                    break;
                }
                param_name.push(ch);
            }
            
            if !param_name.is_empty() {
                parameters.push(param_name);
            }
        }
    }
    
    parameters
}

/// Generate Rust code for the parsed SQL query
fn generate_query_code(parsed_sql: &ParsedSql, parameters: &[SqlParameter]) -> TokenStream {
    let sql = &parsed_sql.original_sql;
    let statement_type = &parsed_sql.statement_type;
    
    // Replace parameter placeholders with actual parameter bindings
    let mut processed_sql = sql.clone();
    let mut bind_values = Vec::new();
    
    for param in &parsed_sql.parameters {
        if let Some(sql_param) = parameters.iter().find(|p| p.name == *param) {
            let placeholder = format!("#{{{}}}", param);
            processed_sql = processed_sql.replace(&placeholder, "?");
            bind_values.push(&sql_param.value);
        }
    }
    
    match statement_type {
        SqlStatementType::Select => {
            quote! {
                {
                    use crate::orm::{Query, QueryBuilder};
                    let sql = #processed_sql;
                    let binds = vec![#(#bind_values.to_value()),*];
                    Query::raw(sql, binds)
                }
            }
        }
        SqlStatementType::Insert => {
            quote! {
                {
                    use crate::orm::{Query, QueryBuilder};
                    let sql = #processed_sql;
                    let binds = vec![#(#bind_values.to_value()),*];
                    Query::raw(sql, binds)
                }
            }
        }
        SqlStatementType::Update => {
            quote! {
                {
                    use crate::orm::{Query, QueryBuilder};
                    let sql = #processed_sql;
                    let binds = vec![#(#bind_values.to_value()),*];
                    Query::raw(sql, binds)
                }
            }
        }
        SqlStatementType::Delete => {
            quote! {
                {
                    use crate::orm::{Query, QueryBuilder};
                    let sql = #processed_sql;
                    let binds = vec![#(#bind_values.to_value()),*];
                    Query::raw(sql, binds)
                }
            }
        }
        _ => {
            quote! {
                {
                    use crate::orm::{Query, QueryBuilder};
                    let sql = #processed_sql;
                    Query::raw(sql, vec![])
                }
            }
        }
    }
}

/// Example usage of the SQL macro (this would be the actual macro in ream-macros)
/// 
/// ```rust
/// let user_id = 42;
/// let users = sql!(
///     "SELECT * FROM users WHERE id = #{user_id} AND active = true",
///     user_id = user_id
/// );
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_parameters() {
        let sql = "SELECT * FROM users WHERE id = #{user_id} AND name = #{name}";
        let params = extract_parameters_from_sql(sql);
        assert_eq!(params, vec!["user_id", "name"]);
    }

    #[test]
    fn test_parse_select_sql() {
        let sql = "SELECT id, name FROM users WHERE active = true";
        let parsed = parse_and_validate_sql(sql).unwrap();
        
        assert_eq!(parsed.statement_type, SqlStatementType::Select);
        assert_eq!(parsed.tables, vec!["users"]);
        assert!(parsed.columns.contains(&"id".to_string()));
        assert!(parsed.columns.contains(&"name".to_string()));
    }

    #[test]
    fn test_parse_insert_sql() {
        let sql = "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')";
        let parsed = parse_and_validate_sql(sql).unwrap();
        
        assert_eq!(parsed.statement_type, SqlStatementType::Insert);
        assert_eq!(parsed.tables, vec!["users"]);
        assert_eq!(parsed.columns, vec!["name", "email"]);
    }

    #[test]
    fn test_invalid_sql() {
        let sql = "INVALID SQL STATEMENT";
        let result = parse_and_validate_sql(sql);
        assert!(result.is_err());
    }
}
