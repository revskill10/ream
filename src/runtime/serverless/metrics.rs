//! Serverless metrics and monitoring system
//! 
//! Provides comprehensive metrics collection, monitoring, and export
//! for hibernation performance, resource utilization, and system health.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use super::{
    hibernation::HibernationStats,
    cold_start::ColdStartStats,
    resources::ResourceStats,
};

/// Comprehensive serverless metrics collector
pub struct ServerlessMetrics {
    /// Hibernation metrics
    hibernation_metrics: Arc<RwLock<HibernationMetrics>>,
    /// Cold start metrics
    cold_start_metrics: Arc<RwLock<ColdStartMetrics>>,
    /// Resource metrics
    resource_metrics: Arc<RwLock<ResourceMetrics>>,
    /// Function execution metrics
    function_metrics: Arc<RwLock<HashMap<String, FunctionMetrics>>>,
    /// System health metrics
    health_metrics: Arc<RwLock<HealthMetrics>>,
    /// Metrics configuration
    config: MetricsConfig,
    /// Metrics export handlers
    exporters: Vec<Box<dyn MetricsExporter>>,
}

/// Hibernation-specific metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HibernationMetrics {
    /// Total hibernations performed
    pub hibernation_count: u64,
    /// Total wake-ups performed
    pub wake_count: u64,
    /// Total hibernation time
    pub hibernation_time_total: Duration,
    /// Total wake time
    pub wake_time_total: Duration,
    /// Ultra-fast wakes (< 1ms)
    pub ultra_fast_wakes: u64,
    /// Fast wakes (< 10ms)
    pub fast_wakes: u64,
    /// Slow wakes (> 10ms)
    pub slow_wakes: u64,
    /// Hibernation failures
    pub hibernation_failures: u64,
    /// Wake failures
    pub wake_failures: u64,
    /// Memory saved through hibernation (bytes)
    pub memory_saved: u64,
    /// Storage used for hibernation (bytes)
    pub storage_used: u64,
    /// Average compression ratio
    pub avg_compression_ratio: f64,
    /// Current hibernating actors
    pub current_hibernating: u64,
    /// Peak hibernating actors
    pub peak_hibernating: u64,
}

/// Cold start specific metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ColdStartMetrics {
    /// Total cold starts
    pub cold_starts: u64,
    /// Total warm starts
    pub warm_starts: u64,
    /// Total cold start time
    pub cold_start_time_total: Duration,
    /// Total warm start time
    pub warm_start_time_total: Duration,
    /// Ultra-fast cold starts (< 1ms)
    pub ultra_fast_cold_starts: u64,
    /// Fast cold starts (< 10ms)
    pub fast_cold_starts: u64,
    /// Slow cold starts (> 10ms)
    pub slow_cold_starts: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Memory pool hits
    pub pool_hits: u64,
    /// Memory pool misses
    pub pool_misses: u64,
    /// JIT compilation cache hits
    pub jit_cache_hits: u64,
    /// JIT compilation cache misses
    pub jit_cache_misses: u64,
}

/// Resource utilization metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Memory allocations
    pub memory_allocations: u64,
    /// Memory deallocations
    pub memory_deallocations: u64,
    /// Connection allocations
    pub connection_allocations: u64,
    /// Connection deallocations
    pub connection_deallocations: u64,
    /// File descriptor allocations
    pub fd_allocations: u64,
    /// File descriptor deallocations
    pub fd_deallocations: u64,
    /// Current memory usage (bytes)
    pub current_memory_usage: u64,
    /// Peak memory usage (bytes)
    pub peak_memory_usage: u64,
    /// Current active connections
    pub current_connections: u64,
    /// Peak active connections
    pub peak_connections: u64,
    /// Current active file descriptors
    pub current_fds: u64,
    /// Peak active file descriptors
    pub peak_fds: u64,
}

/// Function execution metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FunctionMetrics {
    /// Function name
    pub name: String,
    /// Total invocations
    pub invocations: u64,
    /// Successful invocations
    pub successful_invocations: u64,
    /// Failed invocations
    pub failed_invocations: u64,
    /// Total execution time
    pub execution_time_total: Duration,
    /// Total cold start time
    pub cold_start_time_total: Duration,
    /// Current active instances
    pub active_instances: u64,
    /// Current hibernated instances
    pub hibernated_instances: u64,
    /// Peak active instances
    pub peak_active_instances: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// CPU usage (percentage)
    pub cpu_usage: f64,
    /// Last invocation time
    pub last_invocation: Option<SystemTime>,
    /// Error rate (percentage)
    pub error_rate: f64,
    /// Percentile response times
    pub response_time_percentiles: ResponseTimePercentiles,
}

/// Response time percentiles
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResponseTimePercentiles {
    pub p50: Duration,
    pub p90: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub p99_9: Duration,
}

/// System health metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// System uptime
    pub uptime: Duration,
    /// Total system memory (bytes)
    pub total_memory: u64,
    /// Available system memory (bytes)
    pub available_memory: u64,
    /// CPU usage (percentage)
    pub cpu_usage: f64,
    /// Load average (1 minute)
    pub load_average_1m: f64,
    /// Load average (5 minutes)
    pub load_average_5m: f64,
    /// Load average (15 minutes)
    pub load_average_15m: f64,
    /// Network bytes in
    pub network_bytes_in: u64,
    /// Network bytes out
    pub network_bytes_out: u64,
    /// Disk bytes read
    pub disk_bytes_read: u64,
    /// Disk bytes written
    pub disk_bytes_written: u64,
    /// Open file descriptors
    pub open_file_descriptors: u64,
    /// Maximum file descriptors
    pub max_file_descriptors: u64,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Collection interval
    pub collection_interval: Duration,
    /// Retention period
    pub retention_period: Duration,
    /// Enable Prometheus export
    pub prometheus_enabled: bool,
    /// Prometheus export port
    pub prometheus_port: u16,
    /// Enable JSON export
    pub json_enabled: bool,
    /// JSON export path
    pub json_export_path: String,
    /// Enable StatsD export
    pub statsd_enabled: bool,
    /// StatsD server address
    pub statsd_address: String,
    /// Metrics aggregation window
    pub aggregation_window: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        MetricsConfig {
            enabled: true,
            collection_interval: Duration::from_secs(10),
            retention_period: Duration::from_secs(3600), // 1 hour
            prometheus_enabled: true,
            prometheus_port: 9090,
            json_enabled: false,
            json_export_path: "/tmp/ream_metrics.json".to_string(),
            statsd_enabled: false,
            statsd_address: "localhost:8125".to_string(),
            aggregation_window: Duration::from_secs(60),
        }
    }
}

/// Metrics exporter trait
pub trait MetricsExporter: Send + Sync {
    /// Export metrics in the specific format
    fn export(&self, metrics: &ServerlessMetrics) -> Result<String, MetricsError>;
    
    /// Get exporter name
    fn name(&self) -> &str;
    
    /// Get content type for HTTP responses
    fn content_type(&self) -> &str;
}

/// Prometheus metrics exporter
pub struct PrometheusExporter;

/// JSON metrics exporter
pub struct JsonExporter;

/// StatsD metrics exporter
pub struct StatsDExporter {
    address: String,
}

/// Metrics errors
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Export error: {0}")]
    Export(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Collection error: {0}")]
    Collection(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type MetricsResult<T> = Result<T, MetricsError>;

impl ServerlessMetrics {
    /// Create a new serverless metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        let mut exporters: Vec<Box<dyn MetricsExporter>> = Vec::new();
        
        if config.prometheus_enabled {
            exporters.push(Box::new(PrometheusExporter));
        }
        
        if config.json_enabled {
            exporters.push(Box::new(JsonExporter));
        }
        
        if config.statsd_enabled {
            exporters.push(Box::new(StatsDExporter {
                address: config.statsd_address.clone(),
            }));
        }
        
        ServerlessMetrics {
            hibernation_metrics: Arc::new(RwLock::new(HibernationMetrics::default())),
            cold_start_metrics: Arc::new(RwLock::new(ColdStartMetrics::default())),
            resource_metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
            function_metrics: Arc::new(RwLock::new(HashMap::new())),
            health_metrics: Arc::new(RwLock::new(HealthMetrics::default())),
            config,
            exporters,
        }
    }
    
    /// Update hibernation metrics
    pub fn update_hibernation_metrics(&self, stats: HibernationStats) {
        let mut metrics = self.hibernation_metrics.write().unwrap();
        metrics.hibernation_count = stats.hibernation_count;
        metrics.wake_count = stats.wake_count;
        metrics.hibernation_time_total = stats.hibernation_time_total;
        metrics.wake_time_total = stats.wake_time_total;
        metrics.ultra_fast_wakes = stats.ultra_fast_wakes;
        metrics.fast_wakes = stats.fast_wakes;
        metrics.slow_wakes = stats.slow_wakes;
        metrics.hibernation_failures = stats.hibernation_failures;
        metrics.wake_failures = stats.wake_failures;
        metrics.memory_saved = stats.memory_saved;
        metrics.storage_used = stats.storage_used;
        metrics.avg_compression_ratio = stats.avg_compression_ratio;
    }
    
    /// Update cold start metrics
    pub fn update_cold_start_metrics(&self, stats: ColdStartStats) {
        let mut metrics = self.cold_start_metrics.write().unwrap();
        metrics.cold_starts = stats.cold_starts;
        metrics.warm_starts = stats.warm_starts;
        metrics.cold_start_time_total = stats.cold_start_time_total;
        metrics.warm_start_time_total = stats.warm_start_time_total;
        metrics.ultra_fast_cold_starts = stats.ultra_fast_cold_starts;
        metrics.fast_cold_starts = stats.fast_cold_starts;
        metrics.slow_cold_starts = stats.slow_cold_starts;
        metrics.cache_hits = stats.cache_hits;
        metrics.cache_misses = stats.cache_misses;
        metrics.pool_hits = stats.pool_hits;
        metrics.pool_misses = stats.pool_misses;
    }
    
    /// Update resource metrics
    pub fn update_resource_metrics(&self, stats: ResourceStats) {
        let mut metrics = self.resource_metrics.write().unwrap();
        metrics.memory_allocations = stats.memory_allocations;
        metrics.memory_deallocations = stats.memory_deallocations;
        metrics.connection_allocations = stats.connection_allocations;
        metrics.connection_deallocations = stats.connection_deallocations;
        metrics.fd_allocations = stats.fd_allocations;
        metrics.fd_deallocations = stats.fd_deallocations;
    }
    
    /// Record function invocation
    pub fn record_function_invocation(&self, function_name: &str, execution_time: Duration, success: bool) {
        let mut function_metrics = self.function_metrics.write().unwrap();
        let metrics = function_metrics.entry(function_name.to_string())
            .or_insert_with(|| FunctionMetrics {
                name: function_name.to_string(),
                ..Default::default()
            });
        
        metrics.invocations += 1;
        if success {
            metrics.successful_invocations += 1;
        } else {
            metrics.failed_invocations += 1;
        }
        metrics.execution_time_total += execution_time;
        metrics.last_invocation = Some(SystemTime::now());
        
        // Update error rate
        metrics.error_rate = (metrics.failed_invocations as f64 / metrics.invocations as f64) * 100.0;
    }
    
    /// Export metrics in all configured formats
    pub fn export_all(&self) -> Vec<(String, String, String)> {
        let mut exports = Vec::new();
        
        for exporter in &self.exporters {
            match exporter.export(self) {
                Ok(data) => {
                    exports.push((
                        exporter.name().to_string(),
                        exporter.content_type().to_string(),
                        data
                    ));
                }
                Err(e) => {
                    eprintln!("Failed to export metrics with {}: {}", exporter.name(), e);
                }
            }
        }
        
        exports
    }
    
    /// Get hibernation metrics
    pub fn get_hibernation_metrics(&self) -> HibernationMetrics {
        self.hibernation_metrics.read().unwrap().clone()
    }
    
    /// Get cold start metrics
    pub fn get_cold_start_metrics(&self) -> ColdStartMetrics {
        self.cold_start_metrics.read().unwrap().clone()
    }
    
    /// Get resource metrics
    pub fn get_resource_metrics(&self) -> ResourceMetrics {
        self.resource_metrics.read().unwrap().clone()
    }
    
    /// Get function metrics
    pub fn get_function_metrics(&self, function_name: &str) -> Option<FunctionMetrics> {
        self.function_metrics.read().unwrap().get(function_name).cloned()
    }
    
    /// Get all function metrics
    pub fn get_all_function_metrics(&self) -> HashMap<String, FunctionMetrics> {
        self.function_metrics.read().unwrap().clone()
    }
    
    /// Get health metrics
    pub fn get_health_metrics(&self) -> HealthMetrics {
        self.health_metrics.read().unwrap().clone()
    }
}

impl MetricsExporter for PrometheusExporter {
    fn export(&self, metrics: &ServerlessMetrics) -> MetricsResult<String> {
        let hibernation = metrics.get_hibernation_metrics();
        let cold_start = metrics.get_cold_start_metrics();
        let resource = metrics.get_resource_metrics();
        let health = metrics.get_health_metrics();

        let mut output = String::new();

        // Hibernation metrics
        output.push_str(&format!(r#"
# HELP ream_hibernation_count Total number of hibernations
# TYPE ream_hibernation_count counter
ream_hibernation_count {}

# HELP ream_wake_count Total number of wake-ups
# TYPE ream_wake_count counter
ream_wake_count {}

# HELP ream_hibernation_time_seconds Total hibernation time
# TYPE ream_hibernation_time_seconds counter
ream_hibernation_time_seconds {}

# HELP ream_wake_time_seconds Total wake time
# TYPE ream_wake_time_seconds counter
ream_wake_time_seconds {}

# HELP ream_ultra_fast_wakes Total ultra-fast wake-ups (<1ms)
# TYPE ream_ultra_fast_wakes counter
ream_ultra_fast_wakes {}

# HELP ream_memory_saved_bytes Total memory saved through hibernation
# TYPE ream_memory_saved_bytes gauge
ream_memory_saved_bytes {}

# HELP ream_storage_used_bytes Total storage used for hibernation
# TYPE ream_storage_used_bytes gauge
ream_storage_used_bytes {}

# HELP ream_current_hibernating Current number of hibernating actors
# TYPE ream_current_hibernating gauge
ream_current_hibernating {}
"#,
            hibernation.hibernation_count,
            hibernation.wake_count,
            hibernation.hibernation_time_total.as_secs_f64(),
            hibernation.wake_time_total.as_secs_f64(),
            hibernation.ultra_fast_wakes,
            hibernation.memory_saved,
            hibernation.storage_used,
            hibernation.current_hibernating
        ));

        // Cold start metrics
        output.push_str(&format!(r#"
# HELP ream_cold_starts Total number of cold starts
# TYPE ream_cold_starts counter
ream_cold_starts {}

# HELP ream_warm_starts Total number of warm starts
# TYPE ream_warm_starts counter
ream_warm_starts {}

# HELP ream_cold_start_time_seconds Total cold start time
# TYPE ream_cold_start_time_seconds counter
ream_cold_start_time_seconds {}

# HELP ream_cache_hits Total cache hits
# TYPE ream_cache_hits counter
ream_cache_hits {}

# HELP ream_cache_misses Total cache misses
# TYPE ream_cache_misses counter
ream_cache_misses {}
"#,
            cold_start.cold_starts,
            cold_start.warm_starts,
            cold_start.cold_start_time_total.as_secs_f64(),
            cold_start.cache_hits,
            cold_start.cache_misses
        ));

        // Resource metrics
        output.push_str(&format!(r#"
# HELP ream_memory_allocations Total memory allocations
# TYPE ream_memory_allocations counter
ream_memory_allocations {}

# HELP ream_current_memory_usage Current memory usage in bytes
# TYPE ream_current_memory_usage gauge
ream_current_memory_usage {}

# HELP ream_current_connections Current active connections
# TYPE ream_current_connections gauge
ream_current_connections {}
"#,
            resource.memory_allocations,
            resource.current_memory_usage,
            resource.current_connections
        ));

        // Health metrics
        output.push_str(&format!(r#"
# HELP ream_uptime_seconds System uptime in seconds
# TYPE ream_uptime_seconds gauge
ream_uptime_seconds {}

# HELP ream_cpu_usage CPU usage percentage
# TYPE ream_cpu_usage gauge
ream_cpu_usage {}

# HELP ream_available_memory_bytes Available system memory
# TYPE ream_available_memory_bytes gauge
ream_available_memory_bytes {}
"#,
            health.uptime.as_secs_f64(),
            health.cpu_usage,
            health.available_memory
        ));

        // Function metrics
        for (name, function_metrics) in metrics.get_all_function_metrics() {
            output.push_str(&format!(r#"
# HELP ream_function_invocations Total function invocations
# TYPE ream_function_invocations counter
ream_function_invocations{{function="{}"}} {}

# HELP ream_function_errors Total function errors
# TYPE ream_function_errors counter
ream_function_errors{{function="{}"}} {}

# HELP ream_function_active_instances Current active instances
# TYPE ream_function_active_instances gauge
ream_function_active_instances{{function="{}"}} {}

# HELP ream_function_hibernated_instances Current hibernated instances
# TYPE ream_function_hibernated_instances gauge
ream_function_hibernated_instances{{function="{}"}} {}
"#,
                name, function_metrics.invocations,
                name, function_metrics.failed_invocations,
                name, function_metrics.active_instances,
                name, function_metrics.hibernated_instances
            ));
        }

        Ok(output)
    }

    fn name(&self) -> &str {
        "prometheus"
    }

    fn content_type(&self) -> &str {
        "text/plain; version=0.0.4; charset=utf-8"
    }
}

impl MetricsExporter for JsonExporter {
    fn export(&self, metrics: &ServerlessMetrics) -> MetricsResult<String> {
        let export_data = serde_json::json!({
            "hibernation": metrics.get_hibernation_metrics(),
            "cold_start": metrics.get_cold_start_metrics(),
            "resource": metrics.get_resource_metrics(),
            "health": metrics.get_health_metrics(),
            "functions": metrics.get_all_function_metrics(),
            "timestamp": SystemTime::now()
        });

        Ok(serde_json::to_string_pretty(&export_data)?)
    }

    fn name(&self) -> &str {
        "json"
    }

    fn content_type(&self) -> &str {
        "application/json"
    }
}

impl MetricsExporter for StatsDExporter {
    fn export(&self, metrics: &ServerlessMetrics) -> MetricsResult<String> {
        let hibernation = metrics.get_hibernation_metrics();
        let cold_start = metrics.get_cold_start_metrics();
        let resource = metrics.get_resource_metrics();

        let mut output = String::new();

        // StatsD format: metric_name:value|type
        output.push_str(&format!("ream.hibernation.count:{}|c\n", hibernation.hibernation_count));
        output.push_str(&format!("ream.wake.count:{}|c\n", hibernation.wake_count));
        output.push_str(&format!("ream.hibernation.time:{}|ms\n", hibernation.hibernation_time_total.as_millis()));
        output.push_str(&format!("ream.wake.time:{}|ms\n", hibernation.wake_time_total.as_millis()));
        output.push_str(&format!("ream.ultra_fast_wakes:{}|c\n", hibernation.ultra_fast_wakes));
        output.push_str(&format!("ream.memory_saved:{}|g\n", hibernation.memory_saved));
        output.push_str(&format!("ream.cold_starts:{}|c\n", cold_start.cold_starts));
        output.push_str(&format!("ream.warm_starts:{}|c\n", cold_start.warm_starts));
        output.push_str(&format!("ream.cache_hits:{}|c\n", cold_start.cache_hits));
        output.push_str(&format!("ream.cache_misses:{}|c\n", cold_start.cache_misses));
        output.push_str(&format!("ream.memory_allocations:{}|c\n", resource.memory_allocations));
        output.push_str(&format!("ream.current_memory:{}|g\n", resource.current_memory_usage));

        Ok(output)
    }

    fn name(&self) -> &str {
        "statsd"
    }

    fn content_type(&self) -> &str {
        "text/plain"
    }
}

impl HibernationMetrics {
    /// Calculate average hibernation time
    pub fn average_hibernation_time(&self) -> Duration {
        if self.hibernation_count > 0 {
            self.hibernation_time_total / self.hibernation_count as u32
        } else {
            Duration::ZERO
        }
    }

    /// Calculate average wake time
    pub fn average_wake_time(&self) -> Duration {
        if self.wake_count > 0 {
            self.wake_time_total / self.wake_count as u32
        } else {
            Duration::ZERO
        }
    }

    /// Calculate ultra-fast wake percentage
    pub fn ultra_fast_wake_percentage(&self) -> f64 {
        if self.wake_count > 0 {
            (self.ultra_fast_wakes as f64 / self.wake_count as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate hibernation success rate
    pub fn hibernation_success_rate(&self) -> f64 {
        let total_attempts = self.hibernation_count + self.hibernation_failures;
        if total_attempts > 0 {
            (self.hibernation_count as f64 / total_attempts as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl FunctionMetrics {
    /// Calculate average execution time
    pub fn average_execution_time(&self) -> Duration {
        if self.invocations > 0 {
            self.execution_time_total / self.invocations as u32
        } else {
            Duration::ZERO
        }
    }

    /// Calculate average cold start time
    pub fn average_cold_start_time(&self) -> Duration {
        if self.invocations > 0 {
            self.cold_start_time_total / self.invocations as u32
        } else {
            Duration::ZERO
        }
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.invocations > 0 {
            (self.successful_invocations as f64 / self.invocations as f64) * 100.0
        } else {
            0.0
        }
    }
}
