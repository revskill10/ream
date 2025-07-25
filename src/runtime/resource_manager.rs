//! Resource Management and Quotas System
//!
//! This module implements CPU time accounting, resource quotas, and adaptive load balancing
//! to prevent resource abuse as specified in PREEMPTIVE_SCHEDULING.md

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, Instant, SystemTime};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::types::{Pid, Priority};
use crate::error::{RuntimeError, RuntimeResult};

/// Resource manager for tracking and enforcing quotas
pub struct ResourceManager {
    /// Process resource usage tracking
    process_usage: Arc<RwLock<HashMap<Pid, ProcessResourceUsage>>>,
    /// Global resource quotas
    global_quotas: ResourceQuotas,
    /// Per-process quotas
    process_quotas: Arc<RwLock<HashMap<Pid, ResourceQuotas>>>,
    /// Resource accounting
    accounting: Arc<RwLock<ResourceAccounting>>,
    /// Load balancer
    load_balancer: AdaptiveLoadBalancer,
    /// Quota enforcement enabled
    enforcement_enabled: Arc<AtomicBool>,
    /// Statistics
    stats: Arc<RwLock<ResourceManagerStats>>,
}

/// Resource usage tracking for a single process
#[derive(Debug, Clone)]
pub struct ProcessResourceUsage {
    /// CPU time used
    pub cpu_time: Duration,
    /// Memory allocated (bytes)
    pub memory_allocated: u64,
    /// Memory currently used (bytes)
    pub memory_used: u64,
    /// File handles opened
    pub file_handles: u32,
    /// Socket handles opened
    pub socket_handles: u32,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
    /// Disk bytes read
    pub disk_bytes_read: u64,
    /// Disk bytes written
    pub disk_bytes_written: u64,
    /// Number of system calls
    pub system_calls: u64,
    /// Last update timestamp
    pub last_update: Instant,
    /// Process start time
    pub start_time: Instant,
}

impl Default for ProcessResourceUsage {
    fn default() -> Self {
        let now = Instant::now();
        ProcessResourceUsage {
            cpu_time: Duration::default(),
            memory_allocated: 0,
            memory_used: 0,
            file_handles: 0,
            socket_handles: 0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            disk_bytes_read: 0,
            disk_bytes_written: 0,
            system_calls: 0,
            last_update: now,
            start_time: now,
        }
    }
}

/// Resource quotas and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuotas {
    /// Maximum CPU time per period
    pub max_cpu_time: Option<Duration>,
    /// CPU time period
    pub cpu_time_period: Duration,
    /// Maximum memory usage (bytes)
    pub max_memory: Option<u64>,
    /// Maximum file handles
    pub max_file_handles: Option<u32>,
    /// Maximum socket handles
    pub max_socket_handles: Option<u32>,
    /// Maximum network bandwidth (bytes per second)
    pub max_network_bandwidth: Option<u64>,
    /// Maximum disk I/O (bytes per second)
    pub max_disk_io: Option<u64>,
    /// Maximum system calls per second
    pub max_syscalls_per_second: Option<u64>,
    /// Priority boost for quota compliance
    pub priority_boost: Option<Priority>,
}

/// Global resource accounting
#[derive(Debug, Clone)]
pub struct ResourceAccounting {
    /// Total CPU time across all processes
    pub total_cpu_time: Duration,
    /// Total memory allocated
    pub total_memory_allocated: u64,
    /// Total memory used
    pub total_memory_used: u64,
    /// Total network traffic
    pub total_network_bytes: u64,
    /// Total disk I/O
    pub total_disk_bytes: u64,
    /// Total system calls
    pub total_system_calls: u64,
    /// Accounting start time
    pub start_time: Instant,
    /// Last accounting update
    pub last_update: Instant,
}

impl Default for ResourceAccounting {
    fn default() -> Self {
        let now = Instant::now();
        ResourceAccounting {
            total_cpu_time: Duration::default(),
            total_memory_allocated: 0,
            total_memory_used: 0,
            total_network_bytes: 0,
            total_disk_bytes: 0,
            total_system_calls: 0,
            start_time: now,
            last_update: now,
        }
    }
}

/// Adaptive load balancer
pub struct AdaptiveLoadBalancer {
    /// Load metrics per core
    core_loads: Arc<RwLock<Vec<CoreLoad>>>,
    /// Load balancing strategy
    strategy: LoadBalancingStrategy,
    /// Rebalancing threshold
    rebalance_threshold: f64,
    /// Last rebalance time
    last_rebalance: Arc<Mutex<Instant>>,
    /// Rebalance interval
    rebalance_interval: Duration,
}

/// Load metrics for a single core
#[derive(Debug, Clone, Default)]
pub struct CoreLoad {
    /// CPU utilization (0.0 to 1.0)
    pub cpu_utilization: f64,
    /// Number of processes
    pub process_count: u32,
    /// Memory pressure
    pub memory_pressure: f64,
    /// I/O wait time
    pub io_wait: Duration,
    /// Load average
    pub load_average: f64,
}

/// Load balancing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Round-robin assignment
    RoundRobin,
    /// Least loaded core
    LeastLoaded,
    /// CPU utilization based
    CpuUtilization,
    /// Memory pressure based
    MemoryPressure,
    /// Adaptive (changes based on conditions)
    Adaptive,
}

/// Resource manager statistics
#[derive(Debug, Default, Clone)]
pub struct ResourceManagerStats {
    /// Total quota violations
    pub quota_violations: u64,
    /// Processes throttled
    pub processes_throttled: u64,
    /// Load balancing operations
    pub load_balance_operations: u64,
    /// Average resource utilization
    pub avg_resource_utilization: f64,
    /// Peak memory usage
    pub peak_memory_usage: u64,
    /// Peak CPU utilization
    pub peak_cpu_utilization: f64,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new(global_quotas: ResourceQuotas) -> Self {
        let num_cores = num_cpus::get();
        let core_loads = vec![CoreLoad::default(); num_cores];
        
        ResourceManager {
            process_usage: Arc::new(RwLock::new(HashMap::new())),
            global_quotas,
            process_quotas: Arc::new(RwLock::new(HashMap::new())),
            accounting: Arc::new(RwLock::new(ResourceAccounting {
                start_time: Instant::now(),
                last_update: Instant::now(),
                ..Default::default()
            })),
            load_balancer: AdaptiveLoadBalancer {
                core_loads: Arc::new(RwLock::new(core_loads)),
                strategy: LoadBalancingStrategy::Adaptive,
                rebalance_threshold: 0.2, // 20% imbalance threshold
                last_rebalance: Arc::new(Mutex::new(Instant::now())),
                rebalance_interval: Duration::from_secs(1),
            },
            enforcement_enabled: Arc::new(AtomicBool::new(true)),
            stats: Arc::new(RwLock::new(ResourceManagerStats::default())),
        }
    }
    
    /// Register a new process
    pub fn register_process(&self, pid: Pid, quotas: Option<ResourceQuotas>) {
        let usage = ProcessResourceUsage {
            start_time: Instant::now(),
            last_update: Instant::now(),
            ..Default::default()
        };
        
        self.process_usage.write().insert(pid, usage);
        
        if let Some(quotas) = quotas {
            self.process_quotas.write().insert(pid, quotas);
        }
    }
    
    /// Unregister a process
    pub fn unregister_process(&self, pid: Pid) {
        self.process_usage.write().remove(&pid);
        self.process_quotas.write().remove(&pid);
    }
    
    /// Update CPU time for a process
    pub fn update_cpu_time(&self, pid: Pid, cpu_time: Duration) -> RuntimeResult<()> {
        let mut usage_map = self.process_usage.write();
        if let Some(usage) = usage_map.get_mut(&pid) {
            usage.cpu_time += cpu_time;
            usage.last_update = Instant::now();
            
            // Check CPU quota
            if self.enforcement_enabled.load(Ordering::Relaxed) {
                self.check_cpu_quota(pid, usage)?;
            }
            
            // Update global accounting
            self.accounting.write().total_cpu_time += cpu_time;
        }
        
        Ok(())
    }
    
    /// Update memory usage for a process
    pub fn update_memory_usage(&self, pid: Pid, allocated: u64, used: u64) -> RuntimeResult<()> {
        let mut usage_map = self.process_usage.write();
        if let Some(usage) = usage_map.get_mut(&pid) {
            let old_allocated = usage.memory_allocated;
            let old_used = usage.memory_used;
            
            usage.memory_allocated = allocated;
            usage.memory_used = used;
            usage.last_update = Instant::now();
            
            // Check memory quota
            if self.enforcement_enabled.load(Ordering::Relaxed) {
                self.check_memory_quota(pid, usage)?;
            }
            
            // Update global accounting
            let mut accounting = self.accounting.write();
            accounting.total_memory_allocated = accounting.total_memory_allocated
                .saturating_sub(old_allocated)
                .saturating_add(allocated);
            accounting.total_memory_used = accounting.total_memory_used
                .saturating_sub(old_used)
                .saturating_add(used);
            
            // Update peak memory usage
            let mut stats = self.stats.write();
            if used > stats.peak_memory_usage {
                stats.peak_memory_usage = used;
            }
        }
        
        Ok(())
    }
    
    /// Update network usage for a process
    pub fn update_network_usage(&self, pid: Pid, bytes_sent: u64, bytes_received: u64) -> RuntimeResult<()> {
        let mut usage_map = self.process_usage.write();
        if let Some(usage) = usage_map.get_mut(&pid) {
            usage.network_bytes_sent += bytes_sent;
            usage.network_bytes_received += bytes_received;
            usage.last_update = Instant::now();
            
            // Check network quota
            if self.enforcement_enabled.load(Ordering::Relaxed) {
                self.check_network_quota(pid, usage)?;
            }
            
            // Update global accounting
            self.accounting.write().total_network_bytes += bytes_sent + bytes_received;
        }
        
        Ok(())
    }
    
    /// Update disk I/O for a process
    pub fn update_disk_io(&self, pid: Pid, bytes_read: u64, bytes_written: u64) -> RuntimeResult<()> {
        let mut usage_map = self.process_usage.write();
        if let Some(usage) = usage_map.get_mut(&pid) {
            usage.disk_bytes_read += bytes_read;
            usage.disk_bytes_written += bytes_written;
            usage.last_update = Instant::now();
            
            // Check disk I/O quota
            if self.enforcement_enabled.load(Ordering::Relaxed) {
                self.check_disk_quota(pid, usage)?;
            }
            
            // Update global accounting
            self.accounting.write().total_disk_bytes += bytes_read + bytes_written;
        }
        
        Ok(())
    }
    
    /// Update system call count for a process
    pub fn update_syscall_count(&self, pid: Pid, count: u64) -> RuntimeResult<()> {
        let mut usage_map = self.process_usage.write();
        if let Some(usage) = usage_map.get_mut(&pid) {
            usage.system_calls += count;
            usage.last_update = Instant::now();
            
            // Check syscall quota
            if self.enforcement_enabled.load(Ordering::Relaxed) {
                self.check_syscall_quota(pid, usage)?;
            }
            
            // Update global accounting
            self.accounting.write().total_system_calls += count;
        }
        
        Ok(())
    }
    
    /// Check CPU quota for a process
    fn check_cpu_quota(&self, pid: Pid, usage: &ProcessResourceUsage) -> RuntimeResult<()> {
        let quotas = self.get_effective_quotas(pid);
        
        if let Some(max_cpu_time) = quotas.max_cpu_time {
            let period_start = usage.last_update - quotas.cpu_time_period;
            
            // In a real implementation, we'd track CPU usage over the period
            // For now, we'll use a simplified check
            if usage.cpu_time > max_cpu_time {
                self.stats.write().quota_violations += 1;
                return Err(RuntimeError::Scheduler(format!(
                    "Process {} exceeded CPU quota: {} > {}",
                    pid,
                    usage.cpu_time.as_millis(),
                    max_cpu_time.as_millis()
                )));
            }
        }
        
        Ok(())
    }
    
    /// Check memory quota for a process
    fn check_memory_quota(&self, pid: Pid, usage: &ProcessResourceUsage) -> RuntimeResult<()> {
        let quotas = self.get_effective_quotas(pid);
        
        if let Some(max_memory) = quotas.max_memory {
            if usage.memory_used > max_memory {
                self.stats.write().quota_violations += 1;
                return Err(RuntimeError::Scheduler(format!(
                    "Process {} exceeded memory quota: {} > {}",
                    pid, usage.memory_used, max_memory
                )));
            }
        }
        
        Ok(())
    }
    
    /// Check network quota for a process
    fn check_network_quota(&self, pid: Pid, usage: &ProcessResourceUsage) -> RuntimeResult<()> {
        let quotas = self.get_effective_quotas(pid);
        
        if let Some(max_bandwidth) = quotas.max_network_bandwidth {
            let elapsed = usage.last_update.duration_since(usage.start_time);
            let total_bytes = usage.network_bytes_sent + usage.network_bytes_received;
            let bandwidth = total_bytes as f64 / elapsed.as_secs_f64();
            
            if bandwidth > max_bandwidth as f64 {
                self.stats.write().quota_violations += 1;
                return Err(RuntimeError::Scheduler(format!(
                    "Process {} exceeded network bandwidth quota: {:.0} > {}",
                    pid, bandwidth, max_bandwidth
                )));
            }
        }
        
        Ok(())
    }
    
    /// Check disk I/O quota for a process
    fn check_disk_quota(&self, pid: Pid, usage: &ProcessResourceUsage) -> RuntimeResult<()> {
        let quotas = self.get_effective_quotas(pid);
        
        if let Some(max_disk_io) = quotas.max_disk_io {
            let elapsed = usage.last_update.duration_since(usage.start_time);
            let total_bytes = usage.disk_bytes_read + usage.disk_bytes_written;
            let io_rate = total_bytes as f64 / elapsed.as_secs_f64();
            
            if io_rate > max_disk_io as f64 {
                self.stats.write().quota_violations += 1;
                return Err(RuntimeError::Scheduler(format!(
                    "Process {} exceeded disk I/O quota: {:.0} > {}",
                    pid, io_rate, max_disk_io
                )));
            }
        }
        
        Ok(())
    }
    
    /// Check system call quota for a process
    fn check_syscall_quota(&self, pid: Pid, usage: &ProcessResourceUsage) -> RuntimeResult<()> {
        let quotas = self.get_effective_quotas(pid);
        
        if let Some(max_syscalls) = quotas.max_syscalls_per_second {
            let elapsed = usage.last_update.duration_since(usage.start_time);
            let syscall_rate = usage.system_calls as f64 / elapsed.as_secs_f64();
            
            if syscall_rate > max_syscalls as f64 {
                self.stats.write().quota_violations += 1;
                return Err(RuntimeError::Scheduler(format!(
                    "Process {} exceeded syscall quota: {:.0} > {}",
                    pid, syscall_rate, max_syscalls
                )));
            }
        }
        
        Ok(())
    }
    
    /// Get effective quotas for a process (process-specific or global)
    fn get_effective_quotas(&self, pid: Pid) -> ResourceQuotas {
        self.process_quotas.read()
            .get(&pid)
            .cloned()
            .unwrap_or_else(|| self.global_quotas.clone())
    }
    
    /// Get process resource usage
    pub fn get_process_usage(&self, pid: Pid) -> Option<ProcessResourceUsage> {
        self.process_usage.read().get(&pid).cloned()
    }
    
    /// Get global resource accounting
    pub fn get_accounting(&self) -> ResourceAccounting {
        (*self.accounting.read()).clone()
    }

    /// Get resource manager statistics
    pub fn get_stats(&self) -> ResourceManagerStats {
        (*self.stats.read()).clone()
    }
    
    /// Enable or disable quota enforcement
    pub fn set_enforcement(&self, enabled: bool) {
        self.enforcement_enabled.store(enabled, Ordering::Relaxed);
    }
    
    /// Perform load balancing
    pub fn balance_load(&self) -> RuntimeResult<Vec<LoadBalanceRecommendation>> {
        let mut recommendations = Vec::new();
        
        // Check if rebalancing is needed
        let now = Instant::now();
        let mut last_rebalance = self.load_balancer.last_rebalance.lock().unwrap();
        
        if now.duration_since(*last_rebalance) < self.load_balancer.rebalance_interval {
            return Ok(recommendations);
        }
        
        *last_rebalance = now;
        
        // Calculate load imbalance
        let core_loads = self.load_balancer.core_loads.read();
        let avg_load = core_loads.iter().map(|load| load.cpu_utilization).sum::<f64>() / core_loads.len() as f64;
        
        let max_load = core_loads.iter().map(|load| load.cpu_utilization).fold(0.0, f64::max);
        let min_load = core_loads.iter().map(|load| load.cpu_utilization).fold(1.0, f64::min);
        
        let imbalance = (max_load - min_load) / avg_load;
        
        if imbalance > self.load_balancer.rebalance_threshold {
            // Find overloaded and underloaded cores
            for (core_id, load) in core_loads.iter().enumerate() {
                if load.cpu_utilization > avg_load + self.load_balancer.rebalance_threshold {
                    // Find target core
                    if let Some((target_core, _)) = core_loads.iter().enumerate()
                        .min_by(|(_, a), (_, b)| a.cpu_utilization.partial_cmp(&b.cpu_utilization).unwrap()) {
                        
                        recommendations.push(LoadBalanceRecommendation {
                            source_core: core_id,
                            target_core,
                            processes_to_move: 1, // Simplified
                            reason: LoadBalanceReason::CpuImbalance,
                        });
                    }
                }
            }
            
            self.stats.write().load_balance_operations += 1;
        }
        
        Ok(recommendations)
    }
    
    /// Update core load metrics
    pub fn update_core_load(&self, core_id: usize, load: CoreLoad) {
        if let Some(core_load) = self.load_balancer.core_loads.write().get_mut(core_id) {
            *core_load = load;
        }
    }
}

/// Load balancing recommendation
#[derive(Debug, Clone)]
pub struct LoadBalanceRecommendation {
    /// Source core (overloaded)
    pub source_core: usize,
    /// Target core (underloaded)
    pub target_core: usize,
    /// Number of processes to move
    pub processes_to_move: u32,
    /// Reason for load balancing
    pub reason: LoadBalanceReason,
}

/// Reason for load balancing
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalanceReason {
    /// CPU utilization imbalance
    CpuImbalance,
    /// Memory pressure imbalance
    MemoryImbalance,
    /// I/O wait imbalance
    IoImbalance,
    /// Process count imbalance
    ProcessCountImbalance,
}

impl Default for ResourceQuotas {
    fn default() -> Self {
        ResourceQuotas {
            max_cpu_time: Some(Duration::from_secs(60)), // 1 minute per period
            cpu_time_period: Duration::from_secs(60),
            max_memory: Some(100 * 1024 * 1024), // 100 MB
            max_file_handles: Some(100),
            max_socket_handles: Some(50),
            max_network_bandwidth: Some(10 * 1024 * 1024), // 10 MB/s
            max_disk_io: Some(50 * 1024 * 1024), // 50 MB/s
            max_syscalls_per_second: Some(10000),
            priority_boost: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_manager_creation() {
        let quotas = ResourceQuotas::default();
        let manager = ResourceManager::new(quotas);
        
        let stats = manager.get_stats();
        assert_eq!(stats.quota_violations, 0);
        assert_eq!(stats.processes_throttled, 0);
    }

    #[test]
    fn test_process_registration() {
        let quotas = ResourceQuotas::default();
        let manager = ResourceManager::new(quotas);
        
        let pid = Pid::new();
        manager.register_process(pid, None);
        
        let usage = manager.get_process_usage(pid);
        assert!(usage.is_some());
        
        manager.unregister_process(pid);
        let usage = manager.get_process_usage(pid);
        assert!(usage.is_none());
    }

    #[test]
    fn test_cpu_time_tracking() {
        let quotas = ResourceQuotas::default();
        let manager = ResourceManager::new(quotas);
        
        let pid = Pid::new();
        manager.register_process(pid, None);
        
        // Update CPU time
        manager.update_cpu_time(pid, Duration::from_millis(100)).unwrap();
        
        let usage = manager.get_process_usage(pid).unwrap();
        assert_eq!(usage.cpu_time, Duration::from_millis(100));
    }

    #[test]
    fn test_memory_tracking() {
        let quotas = ResourceQuotas::default();
        let manager = ResourceManager::new(quotas);
        
        let pid = Pid::new();
        manager.register_process(pid, None);
        
        // Update memory usage
        manager.update_memory_usage(pid, 1024, 512).unwrap();
        
        let usage = manager.get_process_usage(pid).unwrap();
        assert_eq!(usage.memory_allocated, 1024);
        assert_eq!(usage.memory_used, 512);
    }

    #[test]
    fn test_quota_enforcement() {
        let mut quotas = ResourceQuotas::default();
        quotas.max_memory = Some(1000); // Very low limit
        
        let manager = ResourceManager::new(quotas);
        
        let pid = Pid::new();
        manager.register_process(pid, None);
        
        // Should fail due to quota violation
        let result = manager.update_memory_usage(pid, 2000, 2000);
        assert!(result.is_err());
        
        let stats = manager.get_stats();
        assert!(stats.quota_violations > 0);
    }
}
