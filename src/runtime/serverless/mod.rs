//! REAM Serverless Architecture
//! 
//! Ultra-low latency scale-to-zero actor execution with mathematical foundations
//! in category theory and coalgebraic state machines.

pub mod hibernation;
pub mod cold_start;
pub mod resources;
pub mod metrics;
pub mod zero_copy;

#[cfg(test)]
pub mod tests;

// Re-export main types to avoid conflicts
pub use hibernation::{
    HibernationManager, HibernationState, HibernationPolicy, HibernationStats,
    HibernationError, HibernationResult, MemorySnapshot, WakeTriggerSystem
};
pub use cold_start::{
    ColdStartOptimizer, ColdStartConfig, ColdStartStats, ColdStartError, ColdStartResult,
    MemoryPool, CompiledBytecode
};
pub use resources::{
    ResourcePools, ResourcePoolConfig, ResourceStats, PoolError, PoolResult,
    ActorResources, Connection, ConnectionType
};
pub use metrics::{
    ServerlessMetrics, MetricsConfig, MetricsExporter, PrometheusExporter,
    JsonExporter, StatsDExporter, MetricsError, MetricsResult
};
pub use zero_copy::{
    ZeroCopyHibernation, ZeroCopyConfig, ZeroCopyStats, MmapStorage
};

use crate::types::Pid;
use crate::error::RuntimeResult;
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Serverless configuration for the runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessConfig {
    /// Default hibernation timeout
    pub hibernation_timeout: Duration,
    /// Memory threshold for automatic hibernation (percentage)
    pub memory_threshold: f64,
    /// CPU threshold for automatic hibernation (percentage)
    pub cpu_threshold: f64,
    /// Maximum hibernated actors
    pub max_hibernated_actors: usize,
    /// Pre-warm pool sizes by actor type
    pub pre_warm_pools: std::collections::HashMap<String, usize>,
    /// Enable zero-copy hibernation
    pub zero_copy_enabled: bool,
    /// Enable JIT compilation caching
    pub jit_cache_enabled: bool,
    /// Hibernation storage size (bytes)
    pub hibernation_storage_size: usize,
}

impl Default for ServerlessConfig {
    fn default() -> Self {
        let mut pre_warm_pools = std::collections::HashMap::new();
        pre_warm_pools.insert("web-handler".to_string(), 100);
        pre_warm_pools.insert("api-processor".to_string(), 50);
        pre_warm_pools.insert("data-processor".to_string(), 25);
        
        ServerlessConfig {
            hibernation_timeout: Duration::from_secs(30),
            memory_threshold: 80.0,
            cpu_threshold: 5.0,
            max_hibernated_actors: 10000,
            pre_warm_pools,
            zero_copy_enabled: true,
            jit_cache_enabled: true,
            hibernation_storage_size: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Wake trigger types for hibernated actors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WakeTrigger {
    /// Wake on incoming message
    IncomingMessage,
    /// Wake on HTTP request
    HttpRequest { path: String, method: String },
    /// Wake on scheduled event
    ScheduledEvent { timestamp: std::time::SystemTime },
    /// Wake on resource threshold
    ResourceThreshold { threshold: f64 },
    /// Wake on external signal
    ExternalSignal { signal: String },
}

/// Serverless function metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessFunction {
    /// Function name
    pub name: String,
    /// Actor type
    pub actor_type: String,
    /// Memory limit (bytes)
    pub memory_limit: usize,
    /// Timeout duration
    pub timeout: Duration,
    /// Concurrency limit
    pub concurrency: usize,
    /// Wake triggers
    pub wake_triggers: Vec<WakeTrigger>,
    /// Environment variables
    pub environment: std::collections::HashMap<String, String>,
}

/// Serverless deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessDeployment {
    /// Deployment name
    pub name: String,
    /// Functions in this deployment
    pub functions: Vec<ServerlessFunction>,
    /// Auto-scaling configuration
    pub auto_scaling: AutoScalingConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

/// Auto-scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingConfig {
    /// Minimum instances
    pub min_instances: usize,
    /// Maximum instances
    pub max_instances: usize,
    /// Target CPU utilization (percentage)
    pub target_cpu_utilization: f64,
    /// Target memory utilization (percentage)
    pub target_memory_utilization: f64,
    /// Scale up cooldown
    pub scale_up_cooldown: Duration,
    /// Scale down cooldown
    pub scale_down_cooldown: Duration,
    /// Hibernation delay
    pub hibernation_delay: Duration,
}

impl Default for AutoScalingConfig {
    fn default() -> Self {
        AutoScalingConfig {
            min_instances: 0,
            max_instances: 1000,
            target_cpu_utilization: 70.0,
            target_memory_utilization: 80.0,
            scale_up_cooldown: Duration::from_secs(10),
            scale_down_cooldown: Duration::from_secs(60),
            hibernation_delay: Duration::from_secs(60),
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics export
    pub metrics_enabled: bool,
    /// Metrics export format
    pub metrics_format: MetricsFormat,
    /// Logging level
    pub log_level: LogLevel,
    /// Trace sampling rate (0.0 to 1.0)
    pub trace_sampling: f64,
    /// Health check interval
    pub health_check_interval: Duration,
}

/// Metrics export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricsFormat {
    Prometheus,
    Json,
    StatsD,
}

/// Logging level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        MonitoringConfig {
            metrics_enabled: true,
            metrics_format: MetricsFormat::Prometheus,
            log_level: LogLevel::Info,
            trace_sampling: 0.01,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

/// Serverless runtime errors
#[derive(Debug, thiserror::Error)]
pub enum ServerlessError {
    #[error("Hibernation error: {0}")]
    Hibernation(#[from] HibernationError),
    
    #[error("Cold start error: {0}")]
    ColdStart(#[from] ColdStartError),
    
    #[error("Resource pool error: {0}")]
    ResourcePool(#[from] PoolError),
    
    #[error("Zero-copy error: {0}")]
    ZeroCopy(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Actor not found: {0:?}")]
    ActorNotFound(Pid),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("Deployment error: {0}")]
    Deployment(String),
    
    #[error("Scaling error: {0}")]
    Scaling(String),
    
    #[error("Monitoring error: {0}")]
    Monitoring(String),
}

pub type ServerlessResult<T> = Result<T, ServerlessError>;

/// Trait for serverless-enabled actors
pub trait ServerlessActor: crate::runtime::actor::ReamActor {
    /// Get actor type for resource pooling
    fn actor_type(&self) -> &str;
    
    /// Check if actor can be hibernated
    fn can_hibernate(&self) -> bool {
        true
    }
    
    /// Prepare for hibernation (cleanup, state saving)
    fn prepare_hibernation(&mut self) -> RuntimeResult<()> {
        Ok(())
    }
    
    /// Restore from hibernation
    fn restore_from_hibernation(&mut self) -> RuntimeResult<()> {
        Ok(())
    }
    
    /// Get memory usage estimate
    fn memory_usage(&self) -> usize {
        std::mem::size_of_val(self)
    }
    
    /// Get CPU usage estimate (0.0 to 1.0)
    fn cpu_usage(&self) -> f64 {
        0.0
    }
    
    /// Check if actor is idle
    fn is_idle(&self) -> bool {
        false
    }
}

/// Serverless runtime trait
pub trait ServerlessRuntime {
    /// Deploy a serverless function
    fn deploy_function(&mut self, function: ServerlessFunction) -> ServerlessResult<()>;
    
    /// Undeploy a serverless function
    fn undeploy_function(&mut self, name: &str) -> ServerlessResult<()>;
    
    /// Invoke a serverless function
    fn invoke_function(&mut self, name: &str, payload: Vec<u8>) -> ServerlessResult<Vec<u8>>;
    
    /// Get function metrics
    fn get_function_metrics(&self, name: &str) -> ServerlessResult<FunctionMetrics>;
    
    /// List deployed functions
    fn list_functions(&self) -> Vec<String>;
    
    /// Scale function instances
    fn scale_function(&mut self, name: &str, instances: usize) -> ServerlessResult<()>;
}

/// Function execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetrics {
    /// Function name
    pub name: String,
    /// Total invocations
    pub invocations: u64,
    /// Average execution time
    pub avg_execution_time: Duration,
    /// Average cold start time
    pub avg_cold_start_time: Duration,
    /// Current active instances
    pub active_instances: usize,
    /// Current hibernated instances
    pub hibernated_instances: usize,
    /// Memory usage
    pub memory_usage: usize,
    /// Error rate
    pub error_rate: f64,
}
