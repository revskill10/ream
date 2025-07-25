//! Comprehensive tests for preemptive scheduling system
//!
//! Tests the implementation of signal-based preemption and multi-core scheduling
//! as specified in PREEMPTIVE_SCHEDULING.md

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::Duration;
use std::thread;

use ream::runtime::preemption::{PreemptionTimer, ExecutionResult};
use ream::runtime::executor::ProcessExecutor;
use ream::runtime::scheduler::Scheduler;
use ream::runtime::process::{Process, ProcessHandle};
use ream::runtime::actor::ReamActor;
use ream::types::{Pid, Priority, ProcessState};
use ream::error::RuntimeResult;

/// Test actor that simulates CPU-intensive work
struct CpuIntensiveActor {
    work_counter: AtomicU32,
}

impl CpuIntensiveActor {
    fn new() -> Self {
        CpuIntensiveActor {
            work_counter: AtomicU32::new(0),
        }
    }
    
    fn get_work_done(&self) -> u32 {
        self.work_counter.load(Ordering::Relaxed)
    }
}

impl ReamActor for CpuIntensiveActor {
    fn receive(&mut self, _message: ream::types::MessagePayload) -> RuntimeResult<()> {
        // Simulate CPU-intensive work
        for _ in 0..1000 {
            self.work_counter.fetch_add(1, Ordering::Relaxed);
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
fn test_preemption_timer_basic_functionality() {
    let mut timer = PreemptionTimer::new(Duration::from_millis(10));
    
    // Timer should not preempt initially
    assert!(!timer.should_preempt());
    
    // Start the timer
    timer.start().unwrap();
    
    // Start a quantum
    timer.start_quantum();
    assert!(!timer.should_preempt());
    assert_eq!(timer.get_counter(), 0);
    
    // Increment counter
    timer.increment_counter();
    assert_eq!(timer.get_counter(), 1);
    
    // Sleep longer than quantum
    thread::sleep(Duration::from_millis(15));
    
    // Should now preempt
    assert!(timer.should_preempt());
    
    timer.stop();
}

#[test]
fn test_preemption_timer_quantum_expiration() {
    let mut timer = PreemptionTimer::new(Duration::from_millis(5));
    timer.start().unwrap();
    
    // Start quantum and measure time
    timer.start_quantum();
    let start = std::time::Instant::now();
    
    // Wait for quantum to expire
    while !timer.should_preempt() {
        thread::sleep(Duration::from_millis(1));
        timer.increment_counter();
        
        // Safety check to prevent infinite loop
        if start.elapsed() > Duration::from_millis(50) {
            break;
        }
    }
    
    // Should have preempted within reasonable time
    assert!(timer.should_preempt());
    assert!(start.elapsed() >= Duration::from_millis(5));
    assert!(start.elapsed() < Duration::from_millis(20)); // Allow some tolerance
    
    timer.stop();
}

#[test]
fn test_preemption_timer_instruction_count_limit() {
    let mut timer = PreemptionTimer::new(Duration::from_secs(1)); // Long quantum
    timer.start().unwrap();
    
    timer.start_quantum();
    
    // Increment counter beyond threshold
    for _ in 0..15000 {
        timer.increment_counter();
    }
    
    // Give timer thread time to detect the high count
    thread::sleep(Duration::from_millis(200));
    
    // Should preempt due to instruction count
    assert!(timer.should_preempt());
    
    timer.stop();
}

#[test]
fn test_process_executor_preemption() {
    let timer = Arc::new(PreemptionTimer::new(Duration::from_millis(10)));
    timer.start().unwrap();
    
    let mut executor = ProcessExecutor::new(timer.clone());
    
    // Create a CPU-intensive process
    let pid = Pid::new();
    let actor = Box::new(CpuIntensiveActor::new());
    let process = Process::new(pid, actor, Priority::Normal);
    let handle = ProcessHandle::new(process);
    
    // Execute the process
    let result = executor.execute_with_preemption(&handle).unwrap();
    
    // Should have executed some instructions
    assert!(result.instructions_executed() > 0);
    
    // Check that it was preempted (either by time or instruction count)
    match result {
        ExecutionResult::Preempted { .. } => {
            // Expected - process was preempted
        }
        ExecutionResult::MessageLimit { .. } => {
            // Also acceptable - hit message processing limit
        }
        other => {
            panic!("Expected preemption or message limit, got {:?}", other);
        }
    }
    
    // Check executor statistics
    let stats = executor.stats();
    assert_eq!(stats.total_quanta, 1);
    assert!(stats.total_instructions > 0);
    
    timer.stop();
}

#[test]
fn test_scheduler_preemptive_execution() {
    let mut scheduler = Scheduler::new();
    scheduler.start().unwrap();
    
    // Create multiple processes
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    
    scheduler.schedule(pid1, Priority::Normal).unwrap();
    scheduler.schedule(pid2, Priority::High).unwrap();
    
    // High priority process should be scheduled first
    let next_pid = scheduler.next_process();
    assert_eq!(next_pid, Some(pid2));
    
    // Test preemption enforcement
    scheduler.force_preempt();
    
    // After forced preemption, scheduler should handle it gracefully
    let next_pid = scheduler.next_process();
    assert!(next_pid.is_some());
    
    scheduler.stop();
}

#[test]
fn test_execution_result_properties() {
    let result = ExecutionResult::Preempted {
        instructions_executed: 5000,
        messages_processed: 25,
        execution_time: Duration::from_millis(8),
    };
    
    assert_eq!(result.instructions_executed(), 5000);
    assert_eq!(result.messages_processed(), 25);
    assert_eq!(result.execution_time(), Duration::from_millis(8));
    assert!(result.should_reschedule());
    
    let terminated_result = ExecutionResult::Terminated {
        instructions_executed: 1000,
        messages_processed: 5,
        execution_time: Duration::from_millis(2),
    };
    
    assert!(!terminated_result.should_reschedule());
}

#[test]
fn test_preemption_statistics() {
    use ream::runtime::preemption::PreemptionStats;
    
    let mut stats = PreemptionStats::default();
    
    let result1 = ExecutionResult::Preempted {
        instructions_executed: 1000,
        messages_processed: 10,
        execution_time: Duration::from_millis(5),
    };
    
    let result2 = ExecutionResult::Preempted {
        instructions_executed: 2000,
        messages_processed: 20,
        execution_time: Duration::from_millis(10),
    };
    
    let quantum = Duration::from_millis(10);
    
    stats.record_preemption(&result1, quantum);
    stats.record_preemption(&result2, quantum);
    
    assert_eq!(stats.total_preemptions, 2);
    assert_eq!(stats.time_preemptions, 2);
    assert!(stats.average_quantum_utilization > 0.0);
    assert!(stats.max_quantum_utilization <= 1.0);
    assert_eq!(stats.total_quantum_time, Duration::from_millis(15));
}

#[test]
fn test_quantum_utilization_calculation() {
    use ream::runtime::preemption::PreemptionStats;
    
    let mut stats = PreemptionStats::default();
    let quantum = Duration::from_millis(10);
    
    // Full quantum utilization
    let full_result = ExecutionResult::Preempted {
        instructions_executed: 1000,
        messages_processed: 10,
        execution_time: Duration::from_millis(10),
    };
    
    stats.record_preemption(&full_result, quantum);
    assert!((stats.average_quantum_utilization - 1.0).abs() < 0.001);
    assert!((stats.max_quantum_utilization - 1.0).abs() < 0.001);
    
    // Half quantum utilization
    let half_result = ExecutionResult::Preempted {
        instructions_executed: 500,
        messages_processed: 5,
        execution_time: Duration::from_millis(5),
    };
    
    stats.record_preemption(&half_result, quantum);
    assert!((stats.average_quantum_utilization - 0.75).abs() < 0.001); // (1.0 + 0.5) / 2
    assert!((stats.max_quantum_utilization - 1.0).abs() < 0.001); // Still 1.0
}

#[test]
fn test_multiple_timer_instances() {
    let mut timer1 = PreemptionTimer::new(Duration::from_millis(5));
    let mut timer2 = PreemptionTimer::new(Duration::from_millis(10));
    
    timer1.start().unwrap();
    timer2.start().unwrap();
    
    timer1.start_quantum();
    timer2.start_quantum();
    
    // Wait for first timer to expire
    thread::sleep(Duration::from_millis(8));
    
    assert!(timer1.should_preempt());
    assert!(!timer2.should_preempt());
    
    // Wait for second timer to expire
    thread::sleep(Duration::from_millis(5));
    
    assert!(timer1.should_preempt());
    assert!(timer2.should_preempt());
    
    timer1.stop();
    timer2.stop();
}
