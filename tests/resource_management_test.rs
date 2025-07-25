//! Comprehensive tests for resource management and quotas system
//!
//! Tests the implementation of CPU time accounting, resource quotas, and adaptive load balancing
//! as specified in PREEMPTIVE_SCHEDULING.md

use std::time::Duration;
use ream::runtime::{
    ResourceManager, ResourceQuotas, ProcessResourceUsage, ResourceAccounting,
    LoadBalanceRecommendation, LoadBalanceReason, CoreLoad, LoadBalancingStrategy
};
use ream::types::{Pid, Priority};

#[test]
fn test_resource_manager_creation() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let stats = manager.get_stats();
    assert_eq!(stats.quota_violations, 0);
    assert_eq!(stats.processes_throttled, 0);
    assert_eq!(stats.load_balance_operations, 0);
    
    let accounting = manager.get_accounting();
    assert_eq!(accounting.total_cpu_time, Duration::ZERO);
    assert_eq!(accounting.total_memory_allocated, 0);
    assert_eq!(accounting.total_memory_used, 0);
}

#[test]
fn test_default_resource_quotas() {
    let quotas = ResourceQuotas::default();
    
    assert_eq!(quotas.max_cpu_time, Some(Duration::from_secs(60)));
    assert_eq!(quotas.cpu_time_period, Duration::from_secs(60));
    assert_eq!(quotas.max_memory, Some(100 * 1024 * 1024)); // 100 MB
    assert_eq!(quotas.max_file_handles, Some(100));
    assert_eq!(quotas.max_socket_handles, Some(50));
    assert_eq!(quotas.max_network_bandwidth, Some(10 * 1024 * 1024)); // 10 MB/s
    assert_eq!(quotas.max_disk_io, Some(50 * 1024 * 1024)); // 50 MB/s
    assert_eq!(quotas.max_syscalls_per_second, Some(10000));
    assert!(quotas.priority_boost.is_none());
}

#[test]
fn test_process_registration() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    
    // Initially no usage
    assert!(manager.get_process_usage(pid).is_none());
    
    // Register process
    manager.register_process(pid, None);
    
    // Should have usage now
    let usage = manager.get_process_usage(pid);
    assert!(usage.is_some());
    
    let usage = usage.unwrap();
    assert_eq!(usage.cpu_time, Duration::ZERO);
    assert_eq!(usage.memory_allocated, 0);
    assert_eq!(usage.memory_used, 0);
    assert_eq!(usage.file_handles, 0);
    assert_eq!(usage.socket_handles, 0);
    
    // Unregister process
    manager.unregister_process(pid);
    
    // Should be gone
    assert!(manager.get_process_usage(pid).is_none());
}

#[test]
fn test_process_registration_with_custom_quotas() {
    let global_quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(global_quotas);
    
    let pid = Pid::new();
    let custom_quotas = ResourceQuotas {
        max_memory: Some(50 * 1024 * 1024), // 50 MB (less than global)
        max_cpu_time: Some(Duration::from_secs(30)), // 30 seconds
        ..ResourceQuotas::default()
    };
    
    manager.register_process(pid, Some(custom_quotas));
    
    // Process should be registered
    assert!(manager.get_process_usage(pid).is_some());
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
    
    // Update again
    manager.update_cpu_time(pid, Duration::from_millis(50)).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.cpu_time, Duration::from_millis(150));
    
    // Check global accounting
    let accounting = manager.get_accounting();
    assert_eq!(accounting.total_cpu_time, Duration::from_millis(150));
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
    
    // Update again
    manager.update_memory_usage(pid, 2048, 1024).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.memory_allocated, 2048);
    assert_eq!(usage.memory_used, 1024);
    
    // Check global accounting
    let accounting = manager.get_accounting();
    assert_eq!(accounting.total_memory_allocated, 2048);
    assert_eq!(accounting.total_memory_used, 1024);
    
    // Check peak memory tracking
    let stats = manager.get_stats();
    assert_eq!(stats.peak_memory_usage, 1024);
}

#[test]
fn test_network_tracking() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    
    // Update network usage
    manager.update_network_usage(pid, 1000, 500).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.network_bytes_sent, 1000);
    assert_eq!(usage.network_bytes_received, 500);
    
    // Update again
    manager.update_network_usage(pid, 2000, 1500).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.network_bytes_sent, 3000);
    assert_eq!(usage.network_bytes_received, 2000);
    
    // Check global accounting
    let accounting = manager.get_accounting();
    assert_eq!(accounting.total_network_bytes, 5000);
}

#[test]
fn test_disk_io_tracking() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    
    // Update disk I/O
    manager.update_disk_io(pid, 2048, 1024).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.disk_bytes_read, 2048);
    assert_eq!(usage.disk_bytes_written, 1024);
    
    // Update again
    manager.update_disk_io(pid, 1024, 2048).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.disk_bytes_read, 3072);
    assert_eq!(usage.disk_bytes_written, 3072);
    
    // Check global accounting
    let accounting = manager.get_accounting();
    assert_eq!(accounting.total_disk_bytes, 6144);
}

#[test]
fn test_syscall_tracking() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    
    // Update syscall count
    manager.update_syscall_count(pid, 100).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.system_calls, 100);
    
    // Update again
    manager.update_syscall_count(pid, 50).unwrap();
    
    let usage = manager.get_process_usage(pid).unwrap();
    assert_eq!(usage.system_calls, 150);
    
    // Check global accounting
    let accounting = manager.get_accounting();
    assert_eq!(accounting.total_system_calls, 150);
}

#[test]
fn test_memory_quota_enforcement() {
    let mut quotas = ResourceQuotas::default();
    quotas.max_memory = Some(1000); // Very low limit
    
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    
    // Should succeed within quota
    assert!(manager.update_memory_usage(pid, 500, 500).is_ok());
    
    // Should fail due to quota violation
    let result = manager.update_memory_usage(pid, 2000, 2000);
    assert!(result.is_err());
    
    let stats = manager.get_stats();
    assert!(stats.quota_violations > 0);
}

#[test]
fn test_cpu_quota_enforcement() {
    let mut quotas = ResourceQuotas::default();
    quotas.max_cpu_time = Some(Duration::from_millis(100)); // Very low limit
    
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    
    // Should succeed within quota
    assert!(manager.update_cpu_time(pid, Duration::from_millis(50)).is_ok());
    
    // Should fail due to quota violation
    let result = manager.update_cpu_time(pid, Duration::from_millis(100));
    assert!(result.is_err());
    
    let stats = manager.get_stats();
    assert!(stats.quota_violations > 0);
}

#[test]
fn test_quota_enforcement_toggle() {
    let mut quotas = ResourceQuotas::default();
    quotas.max_memory = Some(1000); // Very low limit
    
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    
    // Should fail with enforcement enabled
    let result = manager.update_memory_usage(pid, 2000, 2000);
    assert!(result.is_err());
    
    // Disable enforcement
    manager.set_enforcement(false);
    
    // Should succeed with enforcement disabled
    let result = manager.update_memory_usage(pid, 3000, 3000);
    assert!(result.is_ok());
    
    // Re-enable enforcement
    manager.set_enforcement(true);
    
    // Should fail again
    let result = manager.update_memory_usage(pid, 4000, 4000);
    assert!(result.is_err());
}

#[test]
fn test_core_load_tracking() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let core_load = CoreLoad {
        cpu_utilization: 0.75,
        process_count: 5,
        memory_pressure: 0.6,
        io_wait: Duration::from_millis(10),
        load_average: 2.5,
    };
    
    // Update core load
    manager.update_core_load(0, core_load.clone());
    
    // Load balancing should work (though we can't easily verify the internal state)
    let recommendations = manager.balance_load().unwrap();
    // Recommendations might be empty if no imbalance is detected
}

#[test]
fn test_load_balancing() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    // Create imbalanced load scenario
    let high_load = CoreLoad {
        cpu_utilization: 0.9,
        process_count: 10,
        memory_pressure: 0.8,
        io_wait: Duration::from_millis(50),
        load_average: 4.0,
    };
    
    let low_load = CoreLoad {
        cpu_utilization: 0.1,
        process_count: 1,
        memory_pressure: 0.2,
        io_wait: Duration::from_millis(5),
        load_average: 0.5,
    };
    
    // Update core loads to create imbalance
    manager.update_core_load(0, high_load);
    manager.update_core_load(1, low_load);
    
    // Force load balancing by waiting for the interval
    std::thread::sleep(Duration::from_millis(1100));
    
    let recommendations = manager.balance_load().unwrap();
    
    // Should have recommendations due to imbalance
    // Note: The exact behavior depends on the load balancing algorithm
    // In a real test, we might need to verify specific recommendations
}

#[test]
fn test_multiple_processes() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    let pid3 = Pid::new();
    
    // Register multiple processes
    manager.register_process(pid1, None);
    manager.register_process(pid2, None);
    manager.register_process(pid3, None);
    
    // Update resources for each process
    manager.update_cpu_time(pid1, Duration::from_millis(100)).unwrap();
    manager.update_memory_usage(pid1, 1024, 512).unwrap();
    
    manager.update_cpu_time(pid2, Duration::from_millis(200)).unwrap();
    manager.update_memory_usage(pid2, 2048, 1024).unwrap();
    
    manager.update_cpu_time(pid3, Duration::from_millis(150)).unwrap();
    manager.update_memory_usage(pid3, 1536, 768).unwrap();
    
    // Check individual usage
    let usage1 = manager.get_process_usage(pid1).unwrap();
    assert_eq!(usage1.cpu_time, Duration::from_millis(100));
    assert_eq!(usage1.memory_used, 512);
    
    let usage2 = manager.get_process_usage(pid2).unwrap();
    assert_eq!(usage2.cpu_time, Duration::from_millis(200));
    assert_eq!(usage2.memory_used, 1024);
    
    let usage3 = manager.get_process_usage(pid3).unwrap();
    assert_eq!(usage3.cpu_time, Duration::from_millis(150));
    assert_eq!(usage3.memory_used, 768);
    
    // Check global accounting
    let accounting = manager.get_accounting();
    assert_eq!(accounting.total_cpu_time, Duration::from_millis(450));
    assert_eq!(accounting.total_memory_allocated, 4608);
    assert_eq!(accounting.total_memory_used, 2304);
}

#[test]
fn test_process_specific_quotas() {
    let global_quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(global_quotas);
    
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    
    // Register one process with custom quotas
    let custom_quotas = ResourceQuotas {
        max_memory: Some(500), // Very restrictive
        ..ResourceQuotas::default()
    };
    
    manager.register_process(pid1, Some(custom_quotas));
    manager.register_process(pid2, None); // Uses global quotas
    
    // pid1 should fail with restrictive quota
    let result = manager.update_memory_usage(pid1, 1000, 1000);
    assert!(result.is_err());
    
    // pid2 should succeed with global quota
    let result = manager.update_memory_usage(pid2, 1000, 1000);
    assert!(result.is_ok());
}

#[test]
fn test_resource_accounting_timestamps() {
    let quotas = ResourceQuotas::default();
    let manager = ResourceManager::new(quotas);
    
    let accounting = manager.get_accounting();
    let start_time = accounting.start_time;
    
    // Sleep briefly
    std::thread::sleep(Duration::from_millis(10));
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    manager.update_cpu_time(pid, Duration::from_millis(10)).unwrap();
    
    let updated_accounting = manager.get_accounting();
    
    // Last update should be after start time
    assert!(updated_accounting.last_update > start_time);
    
    // Process usage should have timestamps
    let usage = manager.get_process_usage(pid).unwrap();
    assert!(usage.last_update >= usage.start_time);
}
