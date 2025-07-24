/// Plugin architecture - Natural transformations Driver → Driver
/// 
/// This module implements plugins as natural transformations:
/// - Plugin trait transforms any driver into an extended driver
/// - Composable plugin system with middleware pattern
/// - Built-in plugins for common functionality
/// - Type-safe plugin composition

use async_trait::async_trait;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::orm::{Driver, Schema, SqlResult, Transaction, DriverMetadata};
use crate::sqlite::types::Value;

/// Plugin trait - natural transformation Driver → Driver
/// A plugin transforms any driver into an extended driver with additional functionality
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;
}

/// Plugin transformer trait for type-safe plugin composition
pub trait PluginTransformer<D: Driver>: Send + Sync {
    type Output: Driver;

    /// Transform a driver into an extended driver
    fn transform(&self, base: D) -> Self::Output;
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

impl PluginMetadata {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            author: author.into(),
        }
    }
}

/// Logging plugin - logs all SQL operations
pub struct LoggingPlugin {
    metadata: PluginMetadata,
}

impl LoggingPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "logging",
                "1.0.0",
                "Logs all SQL operations",
                "ream-orm",
            ),
        }
    }
}

impl Plugin for LoggingPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }
}

impl<D: Driver + 'static> PluginTransformer<D> for LoggingPlugin {
    type Output = LoggingDriver<D>;

    fn transform(&self, base: D) -> Self::Output {
        LoggingDriver::new(base)
    }
}

/// Logging driver wrapper
pub struct LoggingDriver<D> {
    base: D,
}

impl<D> LoggingDriver<D> {
    fn new(base: D) -> Self {
        Self { base }
    }
}

#[async_trait]
impl<D: Driver + Send + Sync> Driver for LoggingDriver<D> {
    type Row = D::Row;
    
    async fn observe(&self, sql: &str, binds: &[Value]) -> SqlResult<Vec<Self::Row>> {
        println!("[SQL] Executing: {} with binds: {:?}", sql, binds);
        let start = Instant::now();
        let result = self.base.observe(sql, binds).await;
        let duration = start.elapsed();
        
        match &result {
            Ok(rows) => println!("[SQL] Completed in {:?}, returned {} rows", duration, rows.len()),
            Err(e) => println!("[SQL] Failed in {:?}: {}", duration, e),
        }
        
        result
    }
    
    async fn migrate(&self, schema: &Schema) -> SqlResult<()> {
        println!("[SQL] Running migration");
        let result = self.base.migrate(schema).await;
        match &result {
            Ok(_) => println!("[SQL] Migration completed successfully"),
            Err(e) => println!("[SQL] Migration failed: {}", e),
        }
        result
    }
    
    async fn begin_transaction(&self) -> SqlResult<Box<dyn Transaction>> {
        println!("[SQL] Beginning transaction");
        self.base.begin_transaction().await
    }
    
    async fn health_check(&self) -> SqlResult<bool> {
        self.base.health_check().await
    }
    
    fn metadata(&self) -> DriverMetadata {
        let mut metadata = self.base.metadata();
        metadata.name = format!("{} (with logging)", metadata.name);
        metadata
    }
}

/// Caching plugin - caches query results
pub struct CachingPlugin {
    metadata: PluginMetadata,
    ttl: Duration,
}

impl CachingPlugin {
    pub fn new(ttl: Duration) -> Self {
        Self {
            metadata: PluginMetadata::new(
                "caching",
                "1.0.0",
                "Caches query results for improved performance",
                "ream-orm",
            ),
            ttl,
        }
    }
}

impl Plugin for CachingPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }
}

impl<D: Driver + 'static> PluginTransformer<D> for CachingPlugin {
    type Output = CachingDriver<D>;

    fn transform(&self, base: D) -> Self::Output {
        CachingDriver::new(base, self.ttl)
    }
}

/// Cache entry
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    created_at: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            created_at: Instant::now(),
            ttl,
        }
    }
    
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Caching driver wrapper
pub struct CachingDriver<D> {
    base: D,
    cache: Arc<Mutex<HashMap<String, CacheEntry<Vec<String>>>>>, // Simplified cache
    ttl: Duration,
}

impl<D> CachingDriver<D> {
    fn new(base: D, ttl: Duration) -> Self {
        Self {
            base,
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }
    
    fn cache_key(&self, sql: &str, binds: &[Value]) -> String {
        format!("{}:{:?}", sql, binds)
    }
}

#[async_trait]
impl<D: Driver + Send + Sync> Driver for CachingDriver<D> {
    type Row = D::Row;
    
    async fn observe(&self, sql: &str, binds: &[Value]) -> SqlResult<Vec<Self::Row>> {
        // For read-only queries, check cache first
        if sql.trim().to_uppercase().starts_with("SELECT") {
            let key = self.cache_key(sql, binds);
            
            // Check cache
            {
                let cache = self.cache.lock().unwrap();
                if let Some(entry) = cache.get(&key) {
                    if !entry.is_expired() {
                        println!("[CACHE] Cache hit for query: {}", sql);
                        // TODO: Convert cached data back to D::Row
                        // For now, fall through to actual query
                    }
                }
            }
        }
        
        // Execute query
        let result = self.base.observe(sql, binds).await;
        
        // Cache the result for SELECT queries
        if sql.trim().to_uppercase().starts_with("SELECT") {
            if let Ok(ref rows) = result {
                let key = self.cache_key(sql, binds);
                let cached_data = vec![format!("{} rows", rows.len())]; // Simplified
                let entry = CacheEntry::new(cached_data, self.ttl);
                
                let mut cache = self.cache.lock().unwrap();
                cache.insert(key, entry);
                println!("[CACHE] Cached result for query: {}", sql);
            }
        }
        
        result
    }
    
    async fn migrate(&self, schema: &Schema) -> SqlResult<()> {
        // Clear cache on schema changes
        {
            let mut cache = self.cache.lock().unwrap();
            cache.clear();
            println!("[CACHE] Cache cleared due to migration");
        }
        
        self.base.migrate(schema).await
    }
    
    async fn begin_transaction(&self) -> SqlResult<Box<dyn Transaction>> {
        self.base.begin_transaction().await
    }
    
    async fn health_check(&self) -> SqlResult<bool> {
        self.base.health_check().await
    }
    
    fn metadata(&self) -> DriverMetadata {
        let mut metadata = self.base.metadata();
        metadata.name = format!("{} (with caching)", metadata.name);
        metadata
    }
}

/// Metrics plugin - collects performance metrics
pub struct MetricsPlugin {
    metadata: PluginMetadata,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "metrics",
                "1.0.0",
                "Collects performance metrics for database operations",
                "ream-orm",
            ),
        }
    }
}

impl Plugin for MetricsPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }
}

impl<D: Driver + 'static> PluginTransformer<D> for MetricsPlugin {
    type Output = MetricsDriver<D>;

    fn transform(&self, base: D) -> Self::Output {
        MetricsDriver::new(base)
    }
}

/// Metrics data
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub total_queries: u64,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub error_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl Metrics {
    pub fn record_query(&mut self, duration: Duration, success: bool) {
        self.total_queries += 1;
        self.total_duration += duration;
        self.average_duration = self.total_duration / self.total_queries as u32;
        
        if !success {
            self.error_count += 1;
        }
    }
    
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }
    
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }
}

/// Metrics driver wrapper
pub struct MetricsDriver<D> {
    base: D,
    metrics: Arc<Mutex<Metrics>>,
}

impl<D> MetricsDriver<D> {
    fn new(base: D) -> Self {
        Self {
            base,
            metrics: Arc::new(Mutex::new(Metrics::default())),
        }
    }
    
    pub fn get_metrics(&self) -> Metrics {
        self.metrics.lock().unwrap().clone()
    }
}

#[async_trait]
impl<D: Driver + Send + Sync> Driver for MetricsDriver<D> {
    type Row = D::Row;
    
    async fn observe(&self, sql: &str, binds: &[Value]) -> SqlResult<Vec<Self::Row>> {
        let start = Instant::now();
        let result = self.base.observe(sql, binds).await;
        let duration = start.elapsed();
        
        // Record metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.record_query(duration, result.is_ok());
        }
        
        result
    }
    
    async fn migrate(&self, schema: &Schema) -> SqlResult<()> {
        let start = Instant::now();
        let result = self.base.migrate(schema).await;
        let duration = start.elapsed();
        
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.record_query(duration, result.is_ok());
        }
        
        result
    }
    
    async fn begin_transaction(&self) -> SqlResult<Box<dyn Transaction>> {
        self.base.begin_transaction().await
    }
    
    async fn health_check(&self) -> SqlResult<bool> {
        self.base.health_check().await
    }
    
    fn metadata(&self) -> DriverMetadata {
        let mut metadata = self.base.metadata();
        metadata.name = format!("{} (with metrics)", metadata.name);
        metadata
    }
}

/// Plugin composer - composes multiple plugins
/// Note: This is a simplified version. In practice, plugin composition
/// would require more sophisticated type-level programming.
pub struct PluginComposer {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginComposer {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn add_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
        self.plugins.push(plugin);
        self
    }

    /// Get metadata for all plugins
    pub fn plugin_metadata(&self) -> Vec<PluginMetadata> {
        self.plugins.iter().map(|p| p.metadata()).collect()
    }

    /// Get the number of plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orm::driver::SqliteDriver;
    use std::time::Duration;

    #[tokio::test]
    async fn test_logging_plugin() {
        let base_driver = SqliteDriver::new(":memory:");
        let plugin = LoggingPlugin::new();
        let wrapped_driver = plugin.transform(base_driver);

        let metadata = wrapped_driver.metadata();
        assert!(metadata.name.contains("logging"));

        // Test that it still works
        let result = wrapped_driver.observe("SELECT 1", &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_caching_plugin() {
        let base_driver = SqliteDriver::new(":memory:");
        let plugin = CachingPlugin::new(Duration::from_secs(60));
        let wrapped_driver = plugin.transform(base_driver);

        let metadata = wrapped_driver.metadata();
        assert!(metadata.name.contains("caching"));

        // Test that it still works
        let result = wrapped_driver.observe("SELECT 1", &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_plugin() {
        let base_driver = SqliteDriver::new(":memory:");
        let plugin = MetricsPlugin::new();
        let wrapped_driver = plugin.transform(base_driver);

        let metadata = wrapped_driver.metadata();
        assert!(metadata.name.contains("metrics"));

        // Test that it still works
        let result = wrapped_driver.observe("SELECT 1", &[]).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_plugin_composer() {
        let composer = PluginComposer::new()
            .add_plugin(Box::new(LoggingPlugin::new()))
            .add_plugin(Box::new(CachingPlugin::new(Duration::from_secs(60))))
            .add_plugin(Box::new(MetricsPlugin::new()));

        assert_eq!(composer.plugin_count(), 3);

        let metadata = composer.plugin_metadata();
        assert_eq!(metadata.len(), 3);
        assert!(metadata.iter().any(|m| m.name == "logging"));
        assert!(metadata.iter().any(|m| m.name == "caching"));
        assert!(metadata.iter().any(|m| m.name == "metrics"));
    }
}
