//! Comprehensive tests for real-time scheduling system
//!
//! Tests the implementation of EDF, Rate Monotonic, and priority inheritance
//! as specified in PREEMPTIVE_SCHEDULING.md

use std::time::{Duration, Instant};
use ream::runtime::{
    RealTimeScheduler, RealTimeTask, SchedulingAlgorithm, TaskType, ResourceId
};
use ream::types::{Pid, Priority};

#[test]
fn test_realtime_scheduler_creation() {
    let edf_scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    let rm_scheduler = RealTimeScheduler::new(SchedulingAlgorithm::RateMonotonic);
    let hybrid_scheduler = RealTimeScheduler::new(SchedulingAlgorithm::Hybrid);
    
    // Should create successfully
    assert_eq!(edf_scheduler.stats().total_tasks, 0);
    assert_eq!(rm_scheduler.stats().total_tasks, 0);
    assert_eq!(hybrid_scheduler.stats().total_tasks, 0);
}

#[test]
fn test_realtime_task_creation() {
    let pid = Pid::new();
    
    // Test periodic task
    let periodic_task = RealTimeTask::periodic(
        pid,
        Priority::Normal,
        Duration::from_millis(100), // period
        Duration::from_millis(30),  // wcet
        Duration::from_millis(100), // deadline offset
    );
    
    assert_eq!(periodic_task.pid, pid);
    assert_eq!(periodic_task.priority, Priority::Normal);
    assert_eq!(periodic_task.task_type, TaskType::Periodic);
    assert_eq!(periodic_task.period, Some(Duration::from_millis(100)));
    assert_eq!(periodic_task.wcet, Duration::from_millis(30));
    assert_eq!(periodic_task.remaining_time, Duration::from_millis(30));
    assert!(periodic_task.held_resources.is_empty());
    
    // Test sporadic task
    let sporadic_task = RealTimeTask::sporadic(
        Pid::new(),
        Priority::High,
        Duration::from_millis(50), // deadline offset
        Duration::from_millis(20), // wcet
    );
    
    assert_eq!(sporadic_task.task_type, TaskType::Sporadic);
    assert_eq!(sporadic_task.period, None);
    assert_eq!(sporadic_task.wcet, Duration::from_millis(20));
}

#[test]
fn test_task_deadline_checking() {
    let pid = Pid::new();
    let task = RealTimeTask::sporadic(
        pid,
        Priority::Normal,
        Duration::from_millis(1), // Very short deadline
        Duration::from_millis(10),
    );
    
    // Should not have missed deadline initially
    assert!(!task.has_missed_deadline());
    
    // Sleep past deadline
    std::thread::sleep(Duration::from_millis(5));
    
    // Should have missed deadline now
    assert!(task.has_missed_deadline());
}

#[test]
fn test_edf_scheduling() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    // Create tasks with different deadlines
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    let pid3 = Pid::new();
    
    let task1 = RealTimeTask::sporadic(pid1, Priority::Normal, Duration::from_millis(100), Duration::from_millis(10));
    let task2 = RealTimeTask::sporadic(pid2, Priority::Normal, Duration::from_millis(50), Duration::from_millis(10));
    let task3 = RealTimeTask::sporadic(pid3, Priority::Normal, Duration::from_millis(200), Duration::from_millis(10));
    
    // Add tasks (task2 has earliest deadline)
    scheduler.add_task(task1).unwrap();
    scheduler.add_task(task2).unwrap();
    scheduler.add_task(task3).unwrap();
    
    // EDF should schedule task2 first (earliest deadline)
    assert_eq!(scheduler.next_task(), Some(pid2));
    
    // Then task1
    assert_eq!(scheduler.next_task(), Some(pid1));
    
    // Then task3
    assert_eq!(scheduler.next_task(), Some(pid3));
    
    // No more tasks
    assert_eq!(scheduler.next_task(), None);
}

#[test]
fn test_rate_monotonic_scheduling() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::RateMonotonic);
    
    // Create periodic tasks with different periods
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    let pid3 = Pid::new();
    
    let task1 = RealTimeTask::periodic(pid1, Priority::Normal, Duration::from_millis(100), Duration::from_millis(20), Duration::from_millis(100));
    let task2 = RealTimeTask::periodic(pid2, Priority::Normal, Duration::from_millis(50), Duration::from_millis(15), Duration::from_millis(50));
    let task3 = RealTimeTask::periodic(pid3, Priority::Normal, Duration::from_millis(200), Duration::from_millis(30), Duration::from_millis(200));
    
    // Add tasks
    scheduler.add_task(task1).unwrap();
    scheduler.add_task(task2).unwrap();
    scheduler.add_task(task3).unwrap();
    
    // RM should schedule task2 first (shortest period)
    assert_eq!(scheduler.next_task(), Some(pid2));
    
    // Then task1
    assert_eq!(scheduler.next_task(), Some(pid1));
    
    // Then task3
    assert_eq!(scheduler.next_task(), Some(pid3));
}

#[test]
fn test_hybrid_scheduling() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::Hybrid);
    
    // Create mix of periodic and sporadic tasks
    let periodic_pid = Pid::new();
    let sporadic_pid = Pid::new();
    
    let periodic_task = RealTimeTask::periodic(
        periodic_pid,
        Priority::Normal,
        Duration::from_millis(100),
        Duration::from_millis(20),
        Duration::from_millis(100)
    );
    
    let sporadic_task = RealTimeTask::sporadic(
        sporadic_pid,
        Priority::Normal,
        Duration::from_millis(50), // Earlier deadline
        Duration::from_millis(15)
    );
    
    scheduler.add_task(periodic_task).unwrap();
    scheduler.add_task(sporadic_task).unwrap();
    
    // Hybrid should prioritize sporadic task (EDF) over periodic task (RM)
    assert_eq!(scheduler.next_task(), Some(sporadic_pid));
    assert_eq!(scheduler.next_task(), Some(periodic_pid));
}

#[test]
fn test_resource_management() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    let resource_id: ResourceId = 1;
    
    let task1 = RealTimeTask::sporadic(pid1, Priority::Normal, Duration::from_millis(100), Duration::from_millis(10));
    let task2 = RealTimeTask::sporadic(pid2, Priority::High, Duration::from_millis(50), Duration::from_millis(10));
    
    scheduler.add_task(task1).unwrap();
    scheduler.add_task(task2).unwrap();
    
    // Task1 requests resource first
    assert!(scheduler.request_resource(pid1, resource_id).unwrap());
    
    // Task2 requests same resource (should be blocked)
    assert!(!scheduler.request_resource(pid2, resource_id).unwrap());
    
    // Task1 releases resource
    scheduler.release_resource(pid1, resource_id).unwrap();
    
    // Now task2 should be able to get the resource
    assert!(scheduler.request_resource(pid2, resource_id).unwrap());
}

#[test]
fn test_priority_inheritance() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    let low_priority_pid = Pid::new();
    let high_priority_pid = Pid::new();
    let resource_id: ResourceId = 1;
    
    let low_priority_task = RealTimeTask::sporadic(
        low_priority_pid,
        Priority::Low,
        Duration::from_millis(100),
        Duration::from_millis(10)
    );
    
    let high_priority_task = RealTimeTask::sporadic(
        high_priority_pid,
        Priority::High,
        Duration::from_millis(50),
        Duration::from_millis(10)
    );
    
    scheduler.add_task(low_priority_task).unwrap();
    scheduler.add_task(high_priority_task).unwrap();
    
    // Low priority task gets resource
    assert!(scheduler.request_resource(low_priority_pid, resource_id).unwrap());
    
    // High priority task requests same resource (should trigger inheritance)
    assert!(!scheduler.request_resource(high_priority_pid, resource_id).unwrap());
    
    // Check that low priority task inherited high priority
    let low_task = &scheduler.tasks[&low_priority_pid];
    assert_eq!(low_task.priority, Priority::High);
    assert_eq!(low_task.original_priority, Priority::Low);
    
    // Release resource
    scheduler.release_resource(low_priority_pid, resource_id).unwrap();
    
    // Priority should be restored
    let low_task = &scheduler.tasks[&low_priority_pid];
    assert_eq!(low_task.priority, Priority::Low);
}

#[test]
fn test_schedulability_analysis() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    // Create tasks that together have utilization > 1.0
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    
    let task1 = RealTimeTask::periodic(
        pid1,
        Priority::Normal,
        Duration::from_millis(100),
        Duration::from_millis(60), // 60% utilization
        Duration::from_millis(100)
    );
    
    let task2 = RealTimeTask::periodic(
        pid2,
        Priority::Normal,
        Duration::from_millis(100),
        Duration::from_millis(50), // 50% utilization (total 110%)
        Duration::from_millis(100)
    );
    
    // First task should be accepted
    assert!(scheduler.add_task(task1).is_ok());
    
    // Second task should be rejected (would exceed utilization bound)
    assert!(scheduler.add_task(task2).is_err());
}

#[test]
fn test_deadline_miss_detection() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    let pid = Pid::new();
    let task = RealTimeTask::sporadic(
        pid,
        Priority::Normal,
        Duration::from_millis(1), // Very short deadline
        Duration::from_millis(10)
    );
    
    scheduler.add_task(task).unwrap();
    
    // Sleep past deadline
    std::thread::sleep(Duration::from_millis(5));
    
    // Check for deadline misses (this would normally be called by scheduler)
    scheduler.check_deadline_misses();
    
    // Should have detected deadline miss
    assert!(scheduler.stats().deadline_misses > 0);
}

#[test]
fn test_task_execution_time_update() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    let pid = Pid::new();
    let task = RealTimeTask::sporadic(
        pid,
        Priority::Normal,
        Duration::from_millis(100),
        Duration::from_millis(50)
    );
    
    scheduler.add_task(task).unwrap();
    
    // Update execution time
    scheduler.update_execution_time(pid, Duration::from_millis(20));
    
    // Remaining time should be reduced
    let updated_task = &scheduler.tasks[&pid];
    assert_eq!(updated_task.remaining_time, Duration::from_millis(30));
    
    // Update with more time than remaining
    scheduler.update_execution_time(pid, Duration::from_millis(40));
    
    // Remaining time should be zero
    let updated_task = &scheduler.tasks[&pid];
    assert_eq!(updated_task.remaining_time, Duration::ZERO);
}

#[test]
fn test_task_removal() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    let pid = Pid::new();
    let resource_id: ResourceId = 1;
    
    let task = RealTimeTask::sporadic(
        pid,
        Priority::Normal,
        Duration::from_millis(100),
        Duration::from_millis(10)
    );
    
    scheduler.add_task(task).unwrap();
    
    // Task requests resource
    scheduler.request_resource(pid, resource_id).unwrap();
    
    // Remove task (should clean up resources)
    scheduler.remove_task(pid);
    
    // Task should be gone
    assert!(!scheduler.tasks.contains_key(&pid));
    
    // Resource should be available again
    let new_pid = Pid::new();
    let new_task = RealTimeTask::sporadic(
        new_pid,
        Priority::Normal,
        Duration::from_millis(100),
        Duration::from_millis(10)
    );
    
    scheduler.add_task(new_task).unwrap();
    assert!(scheduler.request_resource(new_pid, resource_id).unwrap());
}

#[test]
fn test_statistics_tracking() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    
    let task1 = RealTimeTask::sporadic(pid1, Priority::Normal, Duration::from_millis(100), Duration::from_millis(10));
    let task2 = RealTimeTask::sporadic(pid2, Priority::High, Duration::from_millis(50), Duration::from_millis(10));
    
    // Add tasks
    scheduler.add_task(task1).unwrap();
    scheduler.add_task(task2).unwrap();
    
    // Check initial stats
    assert_eq!(scheduler.stats().total_tasks, 2);
    assert_eq!(scheduler.stats().preemptions, 0);
    
    // Schedule tasks (should cause preemption)
    scheduler.next_task(); // First task
    scheduler.next_task(); // Second task (preemption)
    
    // Check updated stats
    assert_eq!(scheduler.stats().preemptions, 1);
}

#[test]
fn test_time_to_deadline() {
    let pid = Pid::new();
    let deadline_offset = Duration::from_millis(100);
    let task = RealTimeTask::sporadic(pid, Priority::Normal, deadline_offset, Duration::from_millis(10));
    
    // Time to deadline should be approximately the offset
    let time_to_deadline = task.time_to_deadline();
    assert!(time_to_deadline <= deadline_offset);
    assert!(time_to_deadline > Duration::from_millis(90)); // Allow some tolerance
}

#[test]
fn test_multiple_resource_inheritance() {
    let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
    
    let low_pid = Pid::new();
    let med_pid = Pid::new();
    let high_pid = Pid::new();
    
    let resource1: ResourceId = 1;
    let resource2: ResourceId = 2;
    
    let low_task = RealTimeTask::sporadic(low_pid, Priority::Low, Duration::from_millis(300), Duration::from_millis(10));
    let med_task = RealTimeTask::sporadic(med_pid, Priority::Normal, Duration::from_millis(200), Duration::from_millis(10));
    let high_task = RealTimeTask::sporadic(high_pid, Priority::High, Duration::from_millis(100), Duration::from_millis(10));
    
    scheduler.add_task(low_task).unwrap();
    scheduler.add_task(med_task).unwrap();
    scheduler.add_task(high_task).unwrap();
    
    // Low priority task gets resource1
    assert!(scheduler.request_resource(low_pid, resource1).unwrap());
    
    // Medium priority task gets resource2
    assert!(scheduler.request_resource(med_pid, resource2).unwrap());
    
    // High priority task requests resource1 (should inherit to low priority task)
    assert!(!scheduler.request_resource(high_pid, resource1).unwrap());
    
    // Check inheritance
    let low_task = &scheduler.tasks[&low_pid];
    assert_eq!(low_task.priority, Priority::High);
    
    // Release resources
    scheduler.release_resource(low_pid, resource1).unwrap();
    scheduler.release_resource(med_pid, resource2).unwrap();
    
    // Priorities should be restored
    let low_task = &scheduler.tasks[&low_pid];
    assert_eq!(low_task.priority, Priority::Low);
}
