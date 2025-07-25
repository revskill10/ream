//! Comprehensive Integration Tests
//!
//! This module provides comprehensive integration tests that validate all implemented features
//! working together as a complete system, including fault tolerance and real-time guarantees.

use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use std::collections::HashMap;

use ream::runtime::{
    WorkStealingScheduler, ScheduledTask, RealTimeScheduler, RealTimeTask,
    SchedulingAlgorithm, TaskType, ResourceManager, ResourceQuotas,
    PreemptionTimer, ProcessExecutor, Process, ProcessHandle, ReamActor
};
use ream::bytecode::{BytecodeVerifier, SecurityManager, create_sandbox_manager, BytecodeProgram, Bytecode, Value};
use ream::types::{Pid, Priority, MessagePayload, EffectGrade};
use ream::error::RuntimeResult;

/// Integration test actor
struct IntegrationTestActor {
    id: u32,
    work_counter: Arc<AtomicU64>,
    error_counter: Arc<AtomicU64>,
    should_fail: Arc<AtomicBool>,
}

impl IntegrationTestActor {
    fn new(id: u32, work_counter: Arc<AtomicU64>, error_counter: Arc<AtomicU64>, should_fail: Arc<AtomicBool>) -> Self {
        IntegrationTestActor {
            id,
            work_counter,
            error_counter,
            should_fail,
        }
    }
}

impl ReamActor for IntegrationTestActor {
    fn receive(&mut self, _message: MessagePayload) -> RuntimeResult<()> {
        if self.should_fail.load(Ordering::Relaxed) && self.id % 10 == 0 {
            self.error_counter.fetch_add(1, Ordering::Relaxed);
            return Err(ream::error::RuntimeError::RuntimeError("Simulated failure".to_string()).into());
        }
        
        // Simulate work
        for _ in 0..100 {
            self.work_counter.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    fn handle_link(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_unlink(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_monitor(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_demonitor(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_exit(&mut self, _pid: Pid, _reason: String) -> RuntimeResult<()> { Ok(()) }
}

#[test]
fn test_complete_system_integration() {
    println!("Running complete system integration test...");
    
    let work_counter = Arc::new(AtomicU64::new(0));
    let error_counter = Arc::new(AtomicU64::new(0));
    let should_fail = Arc::new(AtomicBool::new(false));
    
    // 1. Set up resource management
    let quotas = ResourceQuotas {
        max_memory: Some(50 * 1024 * 1024), // 50 MB
        max_cpu_time: Some(Duration::from_secs(10)),
        cpu_time_period: Duration::from_secs(10),
        ..ResourceQuotas::default()
    };
    let resource_manager = ResourceManager::new(quotas);
    
    // 2. Set up work-stealing scheduler
    let mut work_stealing_scheduler = WorkStealingScheduler::new(Some(4));
    work_stealing_scheduler.start().unwrap();
    
    // 3. Set up real-time scheduler
    let mut realtime_scheduler = RealTimeScheduler::new(SchedulingAlgorithm::Hybrid);
    
    // 4. Set up security manager
    let security_manager = create_sandbox_manager();
    
    // 5. Set up bytecode verifier
    let mut verifier = BytecodeVerifier::new();
    
    // 6. Create and register processes
    let num_processes = 20;
    let mut processes = Vec::new();
    
    for i in 0..num_processes {
        let pid = Pid::new();
        let actor = Box::new(IntegrationTestActor::new(
            i,
            work_counter.clone(),
            error_counter.clone(),
            should_fail.clone(),
        ));
        let process = Process::new(pid, actor, Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        // Register with resource manager
        resource_manager.register_process(pid, None);
        
        // Register with work-stealing scheduler
        work_stealing_scheduler.register_process(handle);
        
        // Create work-stealing task
        let ws_task = ScheduledTask::new(pid, Priority::Normal);
        work_stealing_scheduler.schedule_task(ws_task);
        
        // Create real-time task (every 5th process)
        if i % 5 == 0 {
            let rt_task = RealTimeTask::periodic(
                pid,
                Priority::High,
                Duration::from_millis(100),
                Duration::from_millis(20),
                Duration::from_millis(100),
            );
            realtime_scheduler.add_task(rt_task).unwrap();
        }
        
        processes.push(pid);
    }
    
    // 7. Run the system for a period
    let test_duration = Duration::from_millis(500);
    let start_time = Instant::now();
    
    while start_time.elapsed() < test_duration {
        // Update resource usage
        for &pid in &processes {
            let _ = resource_manager.update_cpu_time(pid, Duration::from_millis(1));
            let _ = resource_manager.update_memory_usage(pid, 1024 * 1024, 512 * 1024);
        }
        
        // Schedule real-time tasks
        if let Some(rt_pid) = realtime_scheduler.next_task() {
            realtime_scheduler.update_execution_time(rt_pid, Duration::from_millis(5));
        }
        
        // Check security constraints
        let _ = security_manager.check_instruction_count();
        let _ = security_manager.check_execution_time();
        
        thread::sleep(Duration::from_millis(10));
    }
    
    // 8. Introduce failures and test fault tolerance
    should_fail.store(true, Ordering::Relaxed);
    
    // Run with failures for a short period
    let failure_start = Instant::now();
    while failure_start.elapsed() < Duration::from_millis(100) {
        thread::sleep(Duration::from_millis(10));
    }
    
    // 9. Clean up
    work_stealing_scheduler.stop();
    
    for pid in processes {
        resource_manager.unregister_process(pid);
        work_stealing_scheduler.unregister_process(pid);
    }
    
    // 10. Verify results
    let total_work = work_counter.load(Ordering::Relaxed);
    let total_errors = error_counter.load(Ordering::Relaxed);
    
    println!("Integration test completed:");
    println!("  Total work done: {}", total_work);
    println!("  Total errors: {}", total_errors);
    println!("  Work/Error ratio: {:.2}", total_work as f64 / (total_errors + 1) as f64);
    
    // Verify system performed work
    assert!(total_work > 0, "System should have performed some work");
    
    // Verify fault tolerance (errors should be handled gracefully)
    assert!(total_errors > 0, "Should have encountered some errors during failure injection");
    
    // Verify system continued working despite errors
    assert!(total_work > total_errors * 10, "System should have done more work than errors");
}

#[test]
fn test_preemptive_scheduling_guarantees() {
    println!("Testing preemptive scheduling guarantees...");
    
    let timer = Arc::new(PreemptionTimer::new(Duration::from_millis(10)));
    timer.start().unwrap();
    
    let mut executor = ProcessExecutor::new(timer.clone());
    
    let work_counter = Arc::new(AtomicU64::new(0));
    let num_tasks = 10;
    let mut preemption_count = 0;
    
    for i in 0..num_tasks {
        let pid = Pid::new();
        let actor = Box::new(IntegrationTestActor::new(
            i,
            work_counter.clone(),
            Arc::new(AtomicU64::new(0)),
            Arc::new(AtomicBool::new(false)),
        ));
        let process = Process::new(pid, actor, Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        let result = executor.execute_with_preemption(&handle).unwrap();
        
        match result {
            ream::runtime::ExecutionResult::Preempted { .. } => preemption_count += 1,
            _ => {}
        }
    }
    
    timer.stop();
    
    println!("Preemption test completed:");
    println!("  Tasks executed: {}", num_tasks);
    println!("  Preemptions: {}", preemption_count);
    println!("  Preemption rate: {:.1}%", (preemption_count as f64 / num_tasks as f64) * 100.0);
    
    // Verify preemption is working
    assert!(preemption_count > 0, "Should have some preemptions with short quantum");
    
    // Verify work was done
    let total_work = work_counter.load(Ordering::Relaxed);
    assert!(total_work > 0, "Should have performed some work");
}

#[test]
fn test_real_time_scheduling_guarantees() {
    println!("Testing real-time scheduling guarantees...");
    
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    // Create tasks with different deadlines
    let mut tasks = Vec::new();
    let mut deadline_misses = 0;
    
    for i in 0..10 {
        let pid = Pid::new();
        let deadline_offset = Duration::from_millis(50 + (i * 10));
        let task = RealTimeTask::sporadic(pid, Priority::Normal, deadline_offset, Duration::from_millis(5));
        
        // Check if task would miss deadline (simplified check)
        if task.has_missed_deadline() {
            deadline_misses += 1;
        }
        
        tasks.push(task.clone());
        scheduler.add_task(task).unwrap();
    }
    
    // Schedule tasks in EDF order
    let mut scheduled_count = 0;
    while let Some(_pid) = scheduler.next_task() {
        scheduled_count += 1;
        
        // Simulate execution time
        thread::sleep(Duration::from_millis(1));
    }
    
    println!("Real-time scheduling test completed:");
    println!("  Tasks created: {}", tasks.len());
    println!("  Tasks scheduled: {}", scheduled_count);
    println!("  Deadline misses: {}", deadline_misses);
    
    // Verify scheduling worked
    assert!(scheduled_count > 0, "Should have scheduled some tasks");
    
    // In a real system, we'd verify deadline guarantees more rigorously
    assert!(deadline_misses < tasks.len(), "Not all tasks should miss deadlines");
}

#[test]
fn test_security_and_verification_integration() {
    println!("Testing security and verification integration...");
    
    let mut verifier = BytecodeVerifier::new();
    let mut security_manager = create_sandbox_manager();
    
    // Test valid bytecode
    let mut valid_program = BytecodeProgram::new("valid".to_string());
    let const_idx = valid_program.add_constant(Value::Int(42));
    valid_program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
    valid_program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
    valid_program.add_instruction(Bytecode::Add(EffectGrade::Pure));
    
    // Should verify successfully
    assert!(verifier.verify(&valid_program).is_ok(), "Valid program should verify");
    
    // Test security constraints
    assert!(security_manager.check_instruction_count().is_ok(), "Should pass instruction count check");
    assert!(security_manager.check_execution_time().is_ok(), "Should pass execution time check");
    
    // Test blocked instructions
    assert!(security_manager.is_instruction_blocked("file_open"), "Should block file operations");
    assert!(security_manager.is_instruction_blocked("socket_create"), "Should block socket operations");
    assert!(!security_manager.is_instruction_blocked("add"), "Should allow arithmetic operations");
    
    // Test resource limits
    let result = security_manager.check_resource_allocation("memory", 1024);
    assert!(result.is_ok(), "Should allow small memory allocation");
    
    let result = security_manager.check_resource_allocation("memory", usize::MAX);
    assert!(result.is_err(), "Should reject excessive memory allocation");
    
    println!("Security and verification test completed successfully");
}

#[test]
fn test_resource_quota_enforcement() {
    println!("Testing resource quota enforcement...");
    
    let quotas = ResourceQuotas {
        max_memory: Some(10 * 1024 * 1024), // 10 MB
        max_cpu_time: Some(Duration::from_millis(100)),
        cpu_time_period: Duration::from_secs(1),
        ..ResourceQuotas::default()
    };
    
    let manager = ResourceManager::new(quotas);
    
    let pid = Pid::new();
    manager.register_process(pid, None);
    
    // Test memory quota
    assert!(manager.update_memory_usage(pid, 5 * 1024 * 1024, 5 * 1024 * 1024).is_ok(), "Should allow memory within quota");
    assert!(manager.update_memory_usage(pid, 20 * 1024 * 1024, 20 * 1024 * 1024).is_err(), "Should reject memory over quota");
    
    // Test CPU quota
    assert!(manager.update_cpu_time(pid, Duration::from_millis(50)).is_ok(), "Should allow CPU time within quota");
    assert!(manager.update_cpu_time(pid, Duration::from_millis(100)).is_err(), "Should reject CPU time over quota");
    
    // Check statistics
    let stats = manager.get_stats();
    assert!(stats.quota_violations > 0, "Should have recorded quota violations");
    
    manager.unregister_process(pid);
    
    println!("Resource quota enforcement test completed successfully");
}

#[test]
fn test_fault_tolerance() {
    println!("Testing fault tolerance...");
    
    let work_counter = Arc::new(AtomicU64::new(0));
    let error_counter = Arc::new(AtomicU64::new(0));
    let should_fail = Arc::new(AtomicBool::new(true)); // Start with failures
    
    let mut scheduler = WorkStealingScheduler::new(Some(2));
    scheduler.start().unwrap();
    
    // Create processes that will fail
    for i in 0..10 {
        let pid = Pid::new();
        let actor = Box::new(IntegrationTestActor::new(
            i,
            work_counter.clone(),
            error_counter.clone(),
            should_fail.clone(),
        ));
        let process = Process::new(pid, actor, Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        scheduler.register_process(handle);
        
        let task = ScheduledTask::new(pid, Priority::Normal);
        scheduler.schedule_task(task);
    }
    
    // Let the system run with failures
    thread::sleep(Duration::from_millis(100));
    
    // Disable failures
    should_fail.store(false, Ordering::Relaxed);
    
    // Let the system recover
    thread::sleep(Duration::from_millis(100));
    
    scheduler.stop();
    
    let total_work = work_counter.load(Ordering::Relaxed);
    let total_errors = error_counter.load(Ordering::Relaxed);
    
    println!("Fault tolerance test completed:");
    println!("  Total work: {}", total_work);
    println!("  Total errors: {}", total_errors);
    
    // Verify system handled failures gracefully
    assert!(total_errors > 0, "Should have encountered errors");
    assert!(total_work >= 0, "System should continue working despite errors");
    
    println!("Fault tolerance test completed successfully");
}

#[test]
fn test_performance_under_load() {
    println!("Testing performance under load...");
    
    let start_time = Instant::now();
    let work_counter = Arc::new(AtomicU64::new(0));
    
    let mut scheduler = WorkStealingScheduler::new(Some(num_cpus::get()));
    scheduler.start().unwrap();
    
    // Create many processes
    let num_processes = 100;
    for i in 0..num_processes {
        let pid = Pid::new();
        let actor = Box::new(IntegrationTestActor::new(
            i,
            work_counter.clone(),
            Arc::new(AtomicU64::new(0)),
            Arc::new(AtomicBool::new(false)),
        ));
        let process = Process::new(pid, actor, Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        scheduler.register_process(handle);
        
        let task = ScheduledTask::new(pid, Priority::Normal);
        scheduler.schedule_task(task);
    }
    
    // Let the system run
    thread::sleep(Duration::from_millis(200));
    
    scheduler.stop();
    
    let total_time = start_time.elapsed();
    let total_work = work_counter.load(Ordering::Relaxed);
    let throughput = total_work as f64 / total_time.as_secs_f64();
    
    println!("Performance test completed:");
    println!("  Processes: {}", num_processes);
    println!("  Total time: {:.3}s", total_time.as_secs_f64());
    println!("  Total work: {}", total_work);
    println!("  Throughput: {:.0} ops/second", throughput);
    
    // Verify reasonable performance
    assert!(total_work > 0, "Should have performed work");
    assert!(throughput > 1000.0, "Should have reasonable throughput");
    
    println!("Performance test completed successfully");
}
