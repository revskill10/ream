use thiserror::Error;

pub type SqlResult<T> = Result<T, SqlError>;

#[derive(Error, Debug, Clone)]
pub enum SqlError {
    #[error("Parse error: {message}")]
    ParseError { message: String },
    
    #[error("Type error: {message}")]
    TypeError { message: String },
    
    #[error("Runtime error: {message}")]
    RuntimeError { message: String },
    
    #[error("IO error: {message}")]
    IoError { message: String },
    
    #[error("Transaction error: {message}")]
    TransactionError { message: String },

    #[error("Connection error: {message}")]
    ConnectionError { message: String },
    
    #[error("Schema error: {message}")]
    SchemaError { message: String },
    
    #[error("Constraint violation: {message}")]
    ConstraintViolation { message: String },
    
    #[error("Table not found: {table}")]
    TableNotFound { table: String },
    
    #[error("Column not found: {column}")]
    ColumnNotFound { column: String },
    
    #[error("Index not found: {index}")]
    IndexNotFound { index: String },
    
    #[error("Duplicate key: {key}")]
    DuplicateKey { key: String },
    
    #[error("Page cache error: {message}")]
    PageCacheError { message: String },
    
    #[error("B-Tree error: {message}")]
    BTreeError { message: String },
    
    #[error("WAL error: {message}")]
    WalError { message: String },
}

impl SqlError {
    pub fn parse_error(message: impl Into<String>) -> Self {
        SqlError::ParseError { message: message.into() }
    }
    
    pub fn type_error(message: impl Into<String>) -> Self {
        SqlError::TypeError { message: message.into() }
    }
    
    pub fn runtime_error(message: impl Into<String>) -> Self {
        SqlError::RuntimeError { message: message.into() }
    }
    
    pub fn io_error(message: impl Into<String>) -> Self {
        SqlError::IoError { message: message.into() }
    }
    
    pub fn transaction_error(message: impl Into<String>) -> Self {
        SqlError::TransactionError { message: message.into() }
    }
    
    pub fn schema_error(message: impl Into<String>) -> Self {
        SqlError::SchemaError { message: message.into() }
    }
    
    pub fn constraint_violation(message: impl Into<String>) -> Self {
        SqlError::ConstraintViolation { message: message.into() }
    }
    
    pub fn table_not_found(table: impl Into<String>) -> Self {
        SqlError::TableNotFound { table: table.into() }
    }
    
    pub fn column_not_found(column: impl Into<String>) -> Self {
        SqlError::ColumnNotFound { column: column.into() }
    }
    
    pub fn index_not_found(index: impl Into<String>) -> Self {
        SqlError::IndexNotFound { index: index.into() }
    }
    
    pub fn duplicate_key(key: impl Into<String>) -> Self {
        SqlError::DuplicateKey { key: key.into() }
    }
    
    pub fn page_cache_error(message: impl Into<String>) -> Self {
        SqlError::PageCacheError { message: message.into() }
    }
    
    pub fn btree_error(message: impl Into<String>) -> Self {
        SqlError::BTreeError { message: message.into() }
    }
    
    pub fn wal_error(message: impl Into<String>) -> Self {
        SqlError::WalError { message: message.into() }
    }
}
