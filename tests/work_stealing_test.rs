//! Comprehensive tests for work-stealing scheduler
//!
//! Tests the implementation of multi-core work-stealing scheduler
//! as specified in PREEMPTIVE_SCHEDULING.md

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::Duration;
use std::thread;

use ream::runtime::{
    WorkStealingScheduler, ScheduledTask, WorkStealingStats,
    Process, ProcessHandle, ReamActor
};
use ream::types::{Pid, Priority, MessagePayload};
use ream::error::RuntimeResult;

/// Test actor for work-stealing tests
struct WorkStealingTestActor {
    work_counter: Arc<AtomicU32>,
    work_amount: u32,
}

impl WorkStealingTestActor {
    fn new(work_counter: Arc<AtomicU32>, work_amount: u32) -> Self {
        WorkStealingTestActor {
            work_counter,
            work_amount,
        }
    }
}

impl ReamActor for WorkStealingTestActor {
    fn receive(&mut self, _message: MessagePayload) -> RuntimeResult<()> {
        // Simulate work
        for _ in 0..self.work_amount {
            self.work_counter.fetch_add(1, Ordering::Relaxed);
            // Small delay to make work visible
            thread::sleep(Duration::from_nanos(1));
        }
        Ok(())
    }
    
    fn handle_link(&mut self, _pid: Pid) -> RuntimeResult<()> {
        Ok(())
    }
    
    fn handle_unlink(&mut self, _pid: Pid) -> RuntimeResult<()> {
        Ok(())
    }
    
    fn handle_monitor(&mut self, _pid: Pid) -> RuntimeResult<()> {
        Ok(())
    }
    
    fn handle_demonitor(&mut self, _pid: Pid) -> RuntimeResult<()> {
        Ok(())
    }
    
    fn handle_exit(&mut self, _pid: Pid, _reason: String) -> RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn test_work_stealing_scheduler_creation() {
    let scheduler = WorkStealingScheduler::new(Some(4));
    assert_eq!(scheduler.num_workers, 4);
    
    // Test default worker count (should be number of CPUs)
    let default_scheduler = WorkStealingScheduler::new(None);
    assert!(default_scheduler.num_workers > 0);
    assert!(default_scheduler.num_workers <= num_cpus::get());
}

#[test]
fn test_scheduled_task_creation() {
    let pid = Pid::new();
    let task = ScheduledTask::new(pid, Priority::Normal);
    
    assert_eq!(task.pid, pid);
    assert_eq!(task.priority, Priority::Normal);
    assert_eq!(task.reschedule_count, 0);
    assert!(task.preferred_core.is_none());
    assert!(task.age() < Duration::from_millis(10)); // Should be very recent
}

#[test]
fn test_scheduled_task_with_affinity() {
    let pid = Pid::new();
    let task = ScheduledTask::with_affinity(pid, Priority::High, 2);
    
    assert_eq!(task.pid, pid);
    assert_eq!(task.priority, Priority::High);
    assert_eq!(task.preferred_core, Some(2));
}

#[test]
fn test_task_aging() {
    let pid = Pid::new();
    let task = ScheduledTask::new(pid, Priority::Normal);
    
    // Age should be very small initially
    assert!(task.age() < Duration::from_millis(10));
    
    // Sleep and check age increased
    thread::sleep(Duration::from_millis(5));
    assert!(task.age() >= Duration::from_millis(5));
}

#[test]
fn test_task_deprioritization() {
    let pid = Pid::new();
    let mut task = ScheduledTask::new(pid, Priority::Normal);
    
    // Should not be deprioritized initially
    assert!(!task.should_deprioritize());
    
    // Simulate many reschedules
    task.reschedule_count = 15;
    assert!(task.should_deprioritize());
}

#[test]
fn test_scheduler_start_stop() {
    let mut scheduler = WorkStealingScheduler::new(Some(2));
    
    // Should start successfully
    assert!(scheduler.start().is_ok());
    
    // Starting again should be OK (idempotent)
    assert!(scheduler.start().is_ok());
    
    // Stop should work
    scheduler.stop();
    
    // Should be able to restart
    assert!(scheduler.start().is_ok());
    scheduler.stop();
}

#[test]
fn test_process_registration() {
    let scheduler = WorkStealingScheduler::new(Some(2));
    
    // Create a test process
    let pid = Pid::new();
    let work_counter = Arc::new(AtomicU32::new(0));
    let actor = Box::new(WorkStealingTestActor::new(work_counter.clone(), 100));
    let process = Process::new(pid, actor, Priority::Normal);
    let handle = ProcessHandle::new(process);
    
    // Register process
    scheduler.register_process(handle);
    
    // Unregister process
    scheduler.unregister_process(pid);
}

#[test]
fn test_task_scheduling() {
    let scheduler = WorkStealingScheduler::new(Some(2));
    
    // Create tasks
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    let task1 = ScheduledTask::new(pid1, Priority::Normal);
    let task2 = ScheduledTask::with_affinity(pid2, Priority::High, 1);
    
    // Schedule tasks
    scheduler.schedule_task(task1);
    scheduler.schedule_task(task2);
    
    // Tasks should be scheduled (we can't easily verify without starting the scheduler)
    // This test mainly ensures the scheduling API works
}

#[test]
fn test_scheduler_statistics() {
    let scheduler = WorkStealingScheduler::new(Some(4));
    let stats = scheduler.stats();
    
    // Initial statistics should be empty/default
    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.tasks_stolen, 0);
    assert_eq!(stats.tasks_per_worker.len(), 4);
    assert_eq!(stats.steal_attempts_per_worker.len(), 4);
    assert_eq!(stats.successful_steals_per_worker.len(), 4);
    assert_eq!(stats.idle_time_per_worker.len(), 4);
    
    // All worker stats should be zero initially
    for &count in &stats.tasks_per_worker {
        assert_eq!(count, 0);
    }
    
    for &count in &stats.steal_attempts_per_worker {
        assert_eq!(count, 0);
    }
    
    for &count in &stats.successful_steals_per_worker {
        assert_eq!(count, 0);
    }
}

#[test]
fn test_work_stealing_integration() {
    let mut scheduler = WorkStealingScheduler::new(Some(2));
    
    // Start the scheduler
    scheduler.start().unwrap();
    
    // Create multiple processes with different work loads
    let work_counter = Arc::new(AtomicU32::new(0));
    let mut processes = Vec::new();
    
    for i in 0..4 {
        let pid = Pid::new();
        let actor = Box::new(WorkStealingTestActor::new(work_counter.clone(), 50));
        let process = Process::new(pid, actor, Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        scheduler.register_process(handle);
        
        // Create and schedule task
        let task = if i % 2 == 0 {
            ScheduledTask::new(pid, Priority::Normal)
        } else {
            ScheduledTask::with_affinity(pid, Priority::High, i % 2)
        };
        
        scheduler.schedule_task(task);
        processes.push(pid);
    }
    
    // Let the scheduler run for a bit
    thread::sleep(Duration::from_millis(100));
    
    // Check that some work was done
    let work_done = work_counter.load(Ordering::Relaxed);
    assert!(work_done > 0, "Expected some work to be done, got {}", work_done);
    
    // Check statistics
    let stats = scheduler.stats();
    // Note: Due to timing, we can't guarantee specific values, but we can check structure
    assert_eq!(stats.tasks_per_worker.len(), 2);
    assert_eq!(stats.steal_attempts_per_worker.len(), 2);
    
    // Clean up
    for pid in processes {
        scheduler.unregister_process(pid);
    }
    
    scheduler.stop();
}

#[test]
fn test_load_balancing() {
    let scheduler = WorkStealingScheduler::new(Some(4));
    
    // Create many tasks to test load balancing
    let mut tasks = Vec::new();
    for i in 0..20 {
        let pid = Pid::new();
        let priority = if i % 3 == 0 { Priority::High } else { Priority::Normal };
        let task = ScheduledTask::new(pid, priority);
        tasks.push(task);
    }
    
    // Schedule all tasks
    for task in tasks {
        scheduler.schedule_task(task);
    }
    
    // The load balancer should distribute tasks across workers
    // We can't easily verify the exact distribution without accessing internal state
    // This test mainly ensures the load balancing API works
}

#[test]
fn test_core_affinity() {
    let scheduler = WorkStealingScheduler::new(Some(4));
    
    // Create tasks with specific core affinity
    for core in 0..4 {
        let pid = Pid::new();
        let task = ScheduledTask::with_affinity(pid, Priority::Normal, core);
        scheduler.schedule_task(task);
    }
    
    // Tasks should be scheduled to their preferred cores when possible
    // This test ensures the affinity API works
}

#[test]
fn test_priority_handling() {
    let scheduler = WorkStealingScheduler::new(Some(2));
    
    // Create tasks with different priorities
    let high_priority_pid = Pid::new();
    let normal_priority_pid = Pid::new();
    let low_priority_pid = Pid::new();
    
    let high_task = ScheduledTask::new(high_priority_pid, Priority::High);
    let normal_task = ScheduledTask::new(normal_priority_pid, Priority::Normal);
    let low_task = ScheduledTask::new(low_priority_pid, Priority::Low);
    
    // Schedule in reverse priority order
    scheduler.schedule_task(low_task);
    scheduler.schedule_task(normal_task);
    scheduler.schedule_task(high_task);
    
    // The scheduler should handle different priorities appropriately
    // Exact behavior depends on the scheduling algorithm implementation
}

#[test]
fn test_scheduler_drop() {
    // Test that dropping the scheduler properly cleans up
    {
        let mut scheduler = WorkStealingScheduler::new(Some(2));
        scheduler.start().unwrap();
        
        // Schedule some tasks
        for i in 0..5 {
            let pid = Pid::new();
            let task = ScheduledTask::new(pid, Priority::Normal);
            scheduler.schedule_task(task);
        }
        
        // Let it run briefly
        thread::sleep(Duration::from_millis(10));
    } // scheduler is dropped here
    
    // Should not crash or hang
    thread::sleep(Duration::from_millis(10));
}

#[test]
fn test_concurrent_scheduling() {
    let scheduler = Arc::new(WorkStealingScheduler::new(Some(4)));
    scheduler.start().unwrap();
    
    let mut handles = Vec::new();
    
    // Spawn multiple threads that schedule tasks concurrently
    for thread_id in 0..4 {
        let scheduler_clone = Arc::clone(&scheduler);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                let pid = Pid::new();
                let task = ScheduledTask::new(pid, Priority::Normal);
                scheduler_clone.schedule_task(task);
                
                // Small delay to create some concurrency
                thread::sleep(Duration::from_millis(1));
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Let the scheduler process tasks
    thread::sleep(Duration::from_millis(50));
    
    // Should not crash or deadlock
    let stats = scheduler.stats();
    // We can't guarantee specific values due to timing, but structure should be correct
    assert_eq!(stats.tasks_per_worker.len(), 4);
}
