use crate::transaction::{IsolationLevel, WalSyncMode};
use std::time::Duration;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    // Storage configuration
    pub page_size: usize,
    pub cache_size: usize,
    pub btree_order: usize,
    
    // Page cache configuration
    pub enable_write_through: bool,
    pub enable_prefetch: bool,
    pub max_dirty_pages: usize,
    
    // Transaction configuration
    pub default_isolation_level: IsolationLevel,
    pub transaction_timeout: Duration,
    pub max_active_transactions: usize,
    pub enable_wal: bool,
    pub wal_sync_mode: WalSyncMode,
    pub checkpoint_interval: Duration,
    
    // Performance configuration
    pub enable_query_cache: bool,
    pub query_cache_size: usize,
    pub enable_statistics: bool,
    pub statistics_update_interval: Duration,
    
    // Concurrency configuration
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub enable_connection_pooling: bool,
    
    // Maintenance configuration
    pub auto_vacuum: bool,
    pub vacuum_threshold: f64, // Percentage of free space to trigger vacuum
    pub auto_checkpoint: bool,
    pub checkpoint_threshold: usize, // Number of WAL entries to trigger checkpoint
    
    // Security configuration
    pub enable_encryption: bool,
    pub encryption_key: Option<String>,
    pub enable_audit_log: bool,
    
    // Debug configuration
    pub enable_debug_logging: bool,
    pub log_level: LogLevel,
    pub enable_profiling: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            // Storage defaults
            page_size: 4096,
            cache_size: 1000,
            btree_order: 4,
            
            // Page cache defaults
            enable_write_through: false,
            enable_prefetch: true,
            max_dirty_pages: 100,
            
            // Transaction defaults
            default_isolation_level: IsolationLevel::ReadCommitted,
            transaction_timeout: Duration::from_secs(300), // 5 minutes
            max_active_transactions: 1000,
            enable_wal: true,
            wal_sync_mode: WalSyncMode::Normal,
            checkpoint_interval: Duration::from_secs(60), // 1 minute
            
            // Performance defaults
            enable_query_cache: true,
            query_cache_size: 100,
            enable_statistics: true,
            statistics_update_interval: Duration::from_secs(30),
            
            // Concurrency defaults
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),
            enable_connection_pooling: true,
            
            // Maintenance defaults
            auto_vacuum: true,
            vacuum_threshold: 0.25, // 25% free space
            auto_checkpoint: true,
            checkpoint_threshold: 1000,
            
            // Security defaults
            enable_encryption: false,
            encryption_key: None,
            enable_audit_log: false,
            
            // Debug defaults
            enable_debug_logging: false,
            log_level: LogLevel::Info,
            enable_profiling: false,
        }
    }
}

impl DatabaseConfig {
    /// Create a configuration optimized for performance
    pub fn performance_optimized() -> Self {
        DatabaseConfig {
            page_size: 8192, // Larger pages
            cache_size: 10000, // Larger cache
            btree_order: 8, // Higher order B-trees
            enable_write_through: false, // Batch writes
            enable_prefetch: true,
            max_dirty_pages: 1000, // More dirty pages allowed
            enable_query_cache: true,
            query_cache_size: 1000, // Larger query cache
            wal_sync_mode: WalSyncMode::Normal, // Balanced sync
            ..Default::default()
        }
    }

    /// Create a configuration optimized for safety/durability
    pub fn safety_optimized() -> Self {
        DatabaseConfig {
            enable_write_through: true, // Immediate writes
            max_dirty_pages: 10, // Minimal dirty pages
            wal_sync_mode: WalSyncMode::Full, // Full sync
            checkpoint_interval: Duration::from_secs(10), // Frequent checkpoints
            checkpoint_threshold: 100, // Low WAL threshold
            enable_audit_log: true,
            ..Default::default()
        }
    }

    /// Create a configuration optimized for memory usage
    pub fn memory_optimized() -> Self {
        DatabaseConfig {
            page_size: 2048, // Smaller pages
            cache_size: 100, // Smaller cache
            btree_order: 3, // Lower order B-trees
            max_dirty_pages: 10,
            enable_query_cache: false, // Disable query cache
            query_cache_size: 0,
            max_connections: 10, // Fewer connections
            ..Default::default()
        }
    }

    /// Create a configuration for development/testing
    pub fn development() -> Self {
        DatabaseConfig {
            enable_debug_logging: true,
            log_level: LogLevel::Debug,
            enable_profiling: true,
            transaction_timeout: Duration::from_secs(60), // Shorter timeout
            enable_statistics: true,
            statistics_update_interval: Duration::from_secs(5), // Frequent stats
            ..Default::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.page_size < 512 || self.page_size > 65536 {
            return Err("Page size must be between 512 and 65536 bytes".to_string());
        }

        if !self.page_size.is_power_of_two() {
            return Err("Page size must be a power of 2".to_string());
        }

        if self.cache_size == 0 {
            return Err("Cache size must be greater than 0".to_string());
        }

        if self.btree_order < 3 {
            return Err("B-tree order must be at least 3".to_string());
        }

        if self.max_dirty_pages == 0 {
            return Err("Max dirty pages must be greater than 0".to_string());
        }

        if self.max_active_transactions == 0 {
            return Err("Max active transactions must be greater than 0".to_string());
        }

        if self.max_connections == 0 {
            return Err("Max connections must be greater than 0".to_string());
        }

        if self.vacuum_threshold < 0.0 || self.vacuum_threshold > 1.0 {
            return Err("Vacuum threshold must be between 0.0 and 1.0".to_string());
        }

        if self.checkpoint_threshold == 0 {
            return Err("Checkpoint threshold must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Get memory usage estimate in bytes
    pub fn estimated_memory_usage(&self) -> usize {
        let page_cache_memory = self.cache_size * self.page_size;
        let query_cache_memory = if self.enable_query_cache {
            self.query_cache_size * 1024 // Estimate 1KB per cached query
        } else {
            0
        };
        let connection_memory = self.max_connections * 64 * 1024; // Estimate 64KB per connection
        let transaction_memory = self.max_active_transactions * 1024; // Estimate 1KB per transaction

        page_cache_memory + query_cache_memory + connection_memory + transaction_memory
    }

    /// Get recommended configuration based on available memory
    pub fn for_memory_size(memory_mb: usize) -> Self {
        let mut config = DatabaseConfig::default();

        if memory_mb < 64 {
            // Very low memory
            config = DatabaseConfig::memory_optimized();
        } else if memory_mb < 256 {
            // Low memory
            config.cache_size = 100;
            config.max_connections = 20;
            config.query_cache_size = 50;
        } else if memory_mb < 1024 {
            // Medium memory
            config.cache_size = 500;
            config.max_connections = 50;
            config.query_cache_size = 200;
        } else {
            // High memory
            config = DatabaseConfig::performance_optimized();
        }

        config
    }

    /// Apply environment-specific overrides
    pub fn apply_environment_overrides(&mut self) {
        // Check environment variables for overrides
        if let Ok(page_size) = std::env::var("SQLITE_PAGE_SIZE") {
            if let Ok(size) = page_size.parse::<usize>() {
                self.page_size = size;
            }
        }

        if let Ok(cache_size) = std::env::var("SQLITE_CACHE_SIZE") {
            if let Ok(size) = cache_size.parse::<usize>() {
                self.cache_size = size;
            }
        }

        if let Ok(debug) = std::env::var("SQLITE_DEBUG") {
            self.enable_debug_logging = debug.to_lowercase() == "true" || debug == "1";
        }

        if let Ok(wal) = std::env::var("SQLITE_WAL") {
            self.enable_wal = wal.to_lowercase() == "true" || wal == "1";
        }
    }

    /// Create a builder for fluent configuration
    pub fn builder() -> DatabaseConfigBuilder {
        DatabaseConfigBuilder::new()
    }
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Configuration builder for fluent API
#[derive(Debug)]
pub struct DatabaseConfigBuilder {
    config: DatabaseConfig,
}

impl DatabaseConfigBuilder {
    pub fn new() -> Self {
        DatabaseConfigBuilder {
            config: DatabaseConfig::default(),
        }
    }

    pub fn page_size(mut self, size: usize) -> Self {
        self.config.page_size = size;
        self
    }

    pub fn cache_size(mut self, size: usize) -> Self {
        self.config.cache_size = size;
        self
    }

    pub fn btree_order(mut self, order: usize) -> Self {
        self.config.btree_order = order;
        self
    }

    pub fn enable_wal(mut self, enable: bool) -> Self {
        self.config.enable_wal = enable;
        self
    }

    pub fn isolation_level(mut self, level: IsolationLevel) -> Self {
        self.config.default_isolation_level = level;
        self
    }

    pub fn transaction_timeout(mut self, timeout: Duration) -> Self {
        self.config.transaction_timeout = timeout;
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }

    pub fn enable_debug(mut self, enable: bool) -> Self {
        self.config.enable_debug_logging = enable;
        if enable {
            self.config.log_level = LogLevel::Debug;
        }
        self
    }

    pub fn build(self) -> Result<DatabaseConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for DatabaseConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = DatabaseConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_performance_config() {
        let config = DatabaseConfig::performance_optimized();
        assert!(config.validate().is_ok());
        assert_eq!(config.page_size, 8192);
        assert_eq!(config.cache_size, 10000);
    }

    #[test]
    fn test_config_builder() {
        let config = DatabaseConfig::builder()
            .page_size(8192)
            .cache_size(2000)
            .enable_wal(true)
            .build()
            .unwrap();

        assert_eq!(config.page_size, 8192);
        assert_eq!(config.cache_size, 2000);
        assert!(config.enable_wal);
    }

    #[test]
    fn test_invalid_config() {
        let mut config = DatabaseConfig::default();
        config.page_size = 100; // Too small
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_memory_estimation() {
        let config = DatabaseConfig::default();
        let memory = config.estimated_memory_usage();
        assert!(memory > 0);
        
        // Should be roughly: 1000 * 4096 (page cache) + other overhead
        assert!(memory > 4_000_000); // At least 4MB
    }
}
