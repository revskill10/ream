//! TLisp Resource Management and Quotas Integration
//!
//! This module integrates the resource management system with TLisp programs
//! to track and enforce resource usage limits for safe execution.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

use crate::runtime::{ResourceManager, ResourceQuotas, ProcessResourceUsage, ResourceAccounting};
use crate::tlisp::{Value as TlispValue, TlispInterpreter};
use crate::types::Pid;
use crate::error::{TlispError, TlispResult, RuntimeError};

/// TLisp-specific resource quotas
#[derive(Debug, Clone)]
pub struct TlispResourceQuotas {
    /// Base resource quotas
    pub base_quotas: ResourceQuotas,
    /// TLisp-specific limits
    pub tlisp_limits: TlispSpecificLimits,
}

/// TLisp-specific resource limits
#[derive(Debug, Clone)]
pub struct TlispSpecificLimits {
    /// Maximum number of variables in environment
    pub max_variables: Option<usize>,
    /// Maximum recursion depth
    pub max_recursion_depth: Option<usize>,
    /// Maximum string length
    pub max_string_length: Option<usize>,
    /// Maximum list length
    pub max_list_length: Option<usize>,
    /// Maximum number of function calls
    pub max_function_calls: Option<u64>,
    /// Maximum evaluation time
    pub max_evaluation_time: Option<Duration>,
    /// Maximum garbage collection frequency
    pub max_gc_frequency: Option<Duration>,
}

/// TLisp resource usage tracking
#[derive(Debug, Clone)]
pub struct TlispResourceUsage {
    /// Base resource usage
    pub base_usage: ProcessResourceUsage,
    /// TLisp-specific usage
    pub tlisp_usage: TlispSpecificUsage,
}

/// TLisp-specific resource usage
#[derive(Debug, Clone)]
pub struct TlispSpecificUsage {
    /// Number of variables in environment
    pub variable_count: usize,
    /// Current recursion depth
    pub recursion_depth: usize,
    /// Total string allocations
    pub string_allocations: u64,
    /// Total list allocations
    pub list_allocations: u64,
    /// Total function calls
    pub function_calls: u64,
    /// Total evaluation time
    pub evaluation_time: Duration,
    /// Garbage collection count
    pub gc_count: u64,
    /// Last garbage collection time
    pub last_gc_time: Instant,
}

/// TLisp resource manager
pub struct TlispResourceManager {
    /// Underlying resource manager
    resource_manager: Arc<ResourceManager>,
    /// TLisp-specific quotas by process
    tlisp_quotas: Arc<Mutex<HashMap<Pid, TlispSpecificLimits>>>,
    /// TLisp-specific usage by process
    tlisp_usage: Arc<Mutex<HashMap<Pid, TlispSpecificUsage>>>,
    /// Resource violation callbacks
    violation_callbacks: Arc<Mutex<Vec<Box<dyn Fn(Pid, &str) + Send + Sync>>>>,
    /// Statistics
    stats: Arc<Mutex<TlispResourceStats>>,
}

/// TLisp resource statistics
#[derive(Debug, Clone)]
pub struct TlispResourceStats {
    /// Total processes tracked
    pub total_processes: u64,
    /// Active processes
    pub active_processes: u64,
    /// Total quota violations
    pub quota_violations: u64,
    /// Total resource warnings
    pub resource_warnings: u64,
    /// Average memory usage
    pub avg_memory_usage: u64,
    /// Average CPU usage
    pub avg_cpu_usage: Duration,
    /// Peak memory usage
    pub peak_memory_usage: u64,
    /// Total garbage collections
    pub total_gc_count: u64,
}

/// Resource monitoring configuration
#[derive(Debug, Clone)]
pub struct TlispResourceConfig {
    /// Enable resource tracking
    pub enable_tracking: bool,
    /// Enable quota enforcement
    pub enable_enforcement: bool,
    /// Resource check interval
    pub check_interval: Duration,
    /// Warning thresholds (percentage of quota)
    pub warning_thresholds: TlispWarningThresholds,
    /// Auto garbage collection
    pub auto_gc: bool,
    /// GC threshold (memory usage percentage)
    pub gc_threshold: f64,
}

/// Warning thresholds for resource usage
#[derive(Debug, Clone)]
pub struct TlispWarningThresholds {
    /// Memory usage warning threshold (0.0 to 1.0)
    pub memory_threshold: f64,
    /// CPU time warning threshold (0.0 to 1.0)
    pub cpu_threshold: f64,
    /// Variable count warning threshold (0.0 to 1.0)
    pub variable_threshold: f64,
    /// Recursion depth warning threshold (0.0 to 1.0)
    pub recursion_threshold: f64,
}

impl Default for TlispSpecificLimits {
    fn default() -> Self {
        TlispSpecificLimits {
            max_variables: Some(10000),
            max_recursion_depth: Some(1000),
            max_string_length: Some(1024 * 1024), // 1 MB
            max_list_length: Some(100000),
            max_function_calls: Some(1000000),
            max_evaluation_time: Some(Duration::from_secs(30)),
            max_gc_frequency: Some(Duration::from_millis(100)),
        }
    }
}

impl Default for TlispSpecificUsage {
    fn default() -> Self {
        TlispSpecificUsage {
            variable_count: 0,
            recursion_depth: 0,
            string_allocations: 0,
            list_allocations: 0,
            function_calls: 0,
            evaluation_time: Duration::ZERO,
            gc_count: 0,
            last_gc_time: Instant::now(),
        }
    }
}

impl Default for TlispResourceConfig {
    fn default() -> Self {
        TlispResourceConfig {
            enable_tracking: true,
            enable_enforcement: true,
            check_interval: Duration::from_millis(100),
            warning_thresholds: TlispWarningThresholds::default(),
            auto_gc: true,
            gc_threshold: 0.8, // 80%
        }
    }
}

impl Default for TlispWarningThresholds {
    fn default() -> Self {
        TlispWarningThresholds {
            memory_threshold: 0.8,
            cpu_threshold: 0.8,
            variable_threshold: 0.8,
            recursion_threshold: 0.9,
        }
    }
}

impl TlispResourceManager {
    /// Create a new TLisp resource manager
    pub fn new(resource_manager: Arc<ResourceManager>) -> Self {
        TlispResourceManager {
            resource_manager,
            tlisp_quotas: Arc::new(Mutex::new(HashMap::new())),
            tlisp_usage: Arc::new(Mutex::new(HashMap::new())),
            violation_callbacks: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(TlispResourceStats::default())),
        }
    }
    
    /// Register a TLisp process with resource tracking
    pub fn register_tlisp_process(&self, pid: Pid, quotas: TlispResourceQuotas) -> TlispResult<()> {
        // Register with base resource manager
        self.resource_manager.register_process(pid, Some(quotas.base_quotas));
        
        // Register TLisp-specific quotas and usage
        self.tlisp_quotas.lock().unwrap().insert(pid, quotas.tlisp_limits);
        self.tlisp_usage.lock().unwrap().insert(pid, TlispSpecificUsage::default());
        
        // Update statistics
        let mut stats = self.stats.lock().unwrap();
        stats.total_processes += 1;
        stats.active_processes += 1;
        
        Ok(())
    }
    
    /// Unregister a TLisp process
    pub fn unregister_tlisp_process(&self, pid: Pid) {
        // Unregister from base resource manager
        self.resource_manager.unregister_process(pid);
        
        // Remove TLisp-specific data
        self.tlisp_quotas.lock().unwrap().remove(&pid);
        self.tlisp_usage.lock().unwrap().remove(&pid);
        
        // Update statistics
        let mut stats = self.stats.lock().unwrap();
        if stats.active_processes > 0 {
            stats.active_processes -= 1;
        }
    }
    
    /// Update variable count for a process
    pub fn update_variable_count(&self, pid: Pid, count: usize) -> TlispResult<()> {
        // Check quota
        if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
            if let Some(max_vars) = limits.max_variables {
                if count > max_vars {
                    self.handle_quota_violation(pid, &format!("Variable count {} exceeds limit {}", count, max_vars));
                    return Err(TlispError::ResourceError(format!("Variable count quota exceeded: {} > {}", count, max_vars)));
                }
            }
        }
        
        // Update usage
        if let Some(usage) = self.tlisp_usage.lock().unwrap().get_mut(&pid) {
            usage.variable_count = count;
        }
        
        Ok(())
    }
    
    /// Update recursion depth for a process
    pub fn update_recursion_depth(&self, pid: Pid, depth: usize) -> TlispResult<()> {
        // Check quota
        if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
            if let Some(max_depth) = limits.max_recursion_depth {
                if depth > max_depth {
                    self.handle_quota_violation(pid, &format!("Recursion depth {} exceeds limit {}", depth, max_depth));
                    return Err(TlispError::ResourceError(format!("Recursion depth quota exceeded: {} > {}", depth, max_depth)));
                }
            }
        }
        
        // Update usage
        if let Some(usage) = self.tlisp_usage.lock().unwrap().get_mut(&pid) {
            usage.recursion_depth = depth;
        }
        
        Ok(())
    }
    
    /// Track string allocation
    pub fn track_string_allocation(&self, pid: Pid, length: usize) -> TlispResult<()> {
        // Check quota
        if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
            if let Some(max_length) = limits.max_string_length {
                if length > max_length {
                    self.handle_quota_violation(pid, &format!("String length {} exceeds limit {}", length, max_length));
                    return Err(TlispError::ResourceError(format!("String length quota exceeded: {} > {}", length, max_length)));
                }
            }
        }
        
        // Update usage
        if let Some(usage) = self.tlisp_usage.lock().unwrap().get_mut(&pid) {
            usage.string_allocations += 1;
        }
        
        Ok(())
    }
    
    /// Track list allocation
    pub fn track_list_allocation(&self, pid: Pid, length: usize) -> TlispResult<()> {
        // Check quota
        if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
            if let Some(max_length) = limits.max_list_length {
                if length > max_length {
                    self.handle_quota_violation(pid, &format!("List length {} exceeds limit {}", length, max_length));
                    return Err(TlispError::ResourceError(format!("List length quota exceeded: {} > {}", length, max_length)));
                }
            }
        }
        
        // Update usage
        if let Some(usage) = self.tlisp_usage.lock().unwrap().get_mut(&pid) {
            usage.list_allocations += 1;
        }
        
        Ok(())
    }
    
    /// Track function call
    pub fn track_function_call(&self, pid: Pid) -> TlispResult<()> {
        // Check quota
        if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
            if let Some(max_calls) = limits.max_function_calls {
                let current_calls = self.tlisp_usage.lock().unwrap()
                    .get(&pid)
                    .map(|u| u.function_calls)
                    .unwrap_or(0);
                
                if current_calls >= max_calls {
                    self.handle_quota_violation(pid, &format!("Function call count {} exceeds limit {}", current_calls, max_calls));
                    return Err(TlispError::ResourceError(format!("Function call quota exceeded: {} >= {}", current_calls, max_calls)));
                }
            }
        }
        
        // Update usage
        if let Some(usage) = self.tlisp_usage.lock().unwrap().get_mut(&pid) {
            usage.function_calls += 1;
        }
        
        Ok(())
    }
    
    /// Track evaluation time
    pub fn track_evaluation_time(&self, pid: Pid, duration: Duration) -> TlispResult<()> {
        // Check quota
        if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
            if let Some(max_time) = limits.max_evaluation_time {
                let current_time = self.tlisp_usage.lock().unwrap()
                    .get(&pid)
                    .map(|u| u.evaluation_time)
                    .unwrap_or(Duration::ZERO);
                
                let new_time = current_time + duration;
                if new_time > max_time {
                    self.handle_quota_violation(pid, &format!("Evaluation time {:?} exceeds limit {:?}", new_time, max_time));
                    return Err(TlispError::ResourceError(format!("Evaluation time quota exceeded: {:?} > {:?}", new_time, max_time)));
                }
            }
        }
        
        // Update usage
        if let Some(usage) = self.tlisp_usage.lock().unwrap().get_mut(&pid) {
            usage.evaluation_time += duration;
        }
        
        Ok(())
    }
    
    /// Trigger garbage collection for a process
    pub fn trigger_gc(&self, pid: Pid) -> TlispResult<()> {
        // Check GC frequency limit
        if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
            if let Some(min_interval) = limits.max_gc_frequency {
                if let Some(usage) = self.tlisp_usage.lock().unwrap().get(&pid) {
                    if usage.last_gc_time.elapsed() < min_interval {
                        return Err(TlispError::ResourceError("GC frequency limit exceeded".to_string()));
                    }
                }
            }
        }
        
        // Update GC statistics
        if let Some(usage) = self.tlisp_usage.lock().unwrap().get_mut(&pid) {
            usage.gc_count += 1;
            usage.last_gc_time = Instant::now();
        }
        
        // Update global statistics
        self.stats.lock().unwrap().total_gc_count += 1;
        
        Ok(())
    }
    
    /// Get TLisp resource usage for a process
    pub fn get_tlisp_usage(&self, pid: Pid) -> Option<TlispResourceUsage> {
        let base_usage = self.resource_manager.get_process_usage(pid)?;
        let tlisp_usage = self.tlisp_usage.lock().unwrap().get(&pid)?.clone();
        
        Some(TlispResourceUsage {
            base_usage,
            tlisp_usage,
        })
    }
    
    /// Get resource statistics
    pub fn get_stats(&self) -> TlispResourceStats {
        let mut stats = self.stats.lock().unwrap().clone();
        
        // Update dynamic statistics
        let usage_map = self.tlisp_usage.lock().unwrap();
        if !usage_map.is_empty() {
            let total_memory: u64 = usage_map.values()
                .map(|_| {
                    let memory_used: u64 = self.resource_manager.get_accounting().total_memory_used;
                    memory_used
                })
                .sum();
            stats.avg_memory_usage = total_memory / usage_map.len() as u64;
            
            let total_cpu: Duration = usage_map.values()
                .map(|u| u.evaluation_time)
                .sum();
            stats.avg_cpu_usage = total_cpu / usage_map.len() as u32;
        }
        
        stats
    }
    
    /// Add violation callback
    pub fn add_violation_callback<F>(&self, callback: F)
    where
        F: Fn(Pid, &str) + Send + Sync + 'static,
    {
        self.violation_callbacks.lock().unwrap().push(Box::new(callback));
    }
    
    /// Check resource warnings
    pub fn check_warnings(&self, pid: Pid, thresholds: &TlispWarningThresholds) -> Vec<String> {
        let mut warnings = Vec::new();
        
        if let Some(usage) = self.get_tlisp_usage(pid) {
            if let Some(limits) = self.tlisp_quotas.lock().unwrap().get(&pid) {
                // Check variable count
                if let Some(max_vars) = limits.max_variables {
                    let usage_ratio = usage.tlisp_usage.variable_count as f64 / max_vars as f64;
                    if usage_ratio > thresholds.variable_threshold {
                        warnings.push(format!("Variable count at {:.1}% of limit", usage_ratio * 100.0));
                    }
                }
                
                // Check recursion depth
                if let Some(max_depth) = limits.max_recursion_depth {
                    let usage_ratio = usage.tlisp_usage.recursion_depth as f64 / max_depth as f64;
                    if usage_ratio > thresholds.recursion_threshold {
                        warnings.push(format!("Recursion depth at {:.1}% of limit", usage_ratio * 100.0));
                    }
                }
                
                // Check memory usage
                let max_memory: u64 = usage.base_usage.memory_allocated;
                if max_memory > 0 {
                    let usage_ratio = usage.base_usage.memory_used as f64 / max_memory as f64;
                    if usage_ratio > thresholds.memory_threshold {
                        warnings.push(format!("Memory usage at {:.1}% of limit", usage_ratio * 100.0));
                    }
                }
            }
        }
        
        warnings
    }
    
    /// Handle quota violation
    fn handle_quota_violation(&self, pid: Pid, message: &str) {
        // Update statistics
        self.stats.lock().unwrap().quota_violations += 1;
        
        // Call violation callbacks
        let callbacks = self.violation_callbacks.lock().unwrap();
        for callback in callbacks.iter() {
            callback(pid, message);
        }
    }
}

impl Default for TlispResourceStats {
    fn default() -> Self {
        TlispResourceStats {
            total_processes: 0,
            active_processes: 0,
            quota_violations: 0,
            resource_warnings: 0,
            avg_memory_usage: 0,
            avg_cpu_usage: Duration::ZERO,
            peak_memory_usage: 0,
            total_gc_count: 0,
        }
    }
}

impl Default for TlispResourceQuotas {
    fn default() -> Self {
        TlispResourceQuotas {
            base_quotas: ResourceQuotas::default(),
            tlisp_limits: TlispSpecificLimits::default(),
        }
    }
}
