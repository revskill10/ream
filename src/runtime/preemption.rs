//! Preemption Timer System for REAM Runtime
//!
//! This module implements signal-based preemption to replace cooperative scheduling
//! with true preemptive scheduling as specified in PREEMPTIVE_SCHEDULING.md

use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use crate::error::{RuntimeError, RuntimeResult};

/// Preemption timer for enforcing quantum limits
pub struct PreemptionTimer {
    /// Quantum duration
    quantum: Duration,
    /// Current process start time
    process_start: Arc<Mutex<Option<Instant>>>,
    /// Preemption flag
    preempt_flag: Arc<AtomicBool>,
    /// Timer thread handle
    timer_handle: Option<thread::JoinHandle<()>>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Preemption counter for frequent checking
    preemption_counter: Arc<AtomicU32>,
}

impl PreemptionTimer {
    /// Create a new preemption timer with the given quantum
    pub fn new(quantum: Duration) -> Self {
        let process_start = Arc::new(Mutex::new(None));
        let preempt_flag = Arc::new(AtomicBool::new(false));
        let running = Arc::new(AtomicBool::new(false));
        let preemption_counter = Arc::new(AtomicU32::new(0));
        
        PreemptionTimer {
            quantum,
            process_start,
            preempt_flag,
            timer_handle: None,
            running,
            preemption_counter,
        }
    }
    
    /// Start the preemption timer
    pub fn start(&mut self) -> RuntimeResult<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.running.store(true, Ordering::Relaxed);
        
        let timer_handle = Self::start_timer_thread(
            self.quantum,
            Arc::clone(&self.process_start),
            Arc::clone(&self.preempt_flag),
            Arc::clone(&self.running),
            Arc::clone(&self.preemption_counter),
        );
        
        self.timer_handle = Some(timer_handle);
        Ok(())
    }
    
    /// Stop the preemption timer
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        
        if let Some(handle) = self.timer_handle.take() {
            let _ = handle.join();
        }
    }
    
    /// Start a new quantum for the current process
    pub fn start_quantum(&self) {
        *self.process_start.lock().unwrap() = Some(Instant::now());
        self.preempt_flag.store(false, Ordering::Relaxed);
        self.preemption_counter.store(0, Ordering::Relaxed);
    }
    
    /// Check if the current process should be preempted
    pub fn should_preempt(&self) -> bool {
        self.preempt_flag.load(Ordering::Relaxed)
    }
    
    /// Increment the preemption counter (called frequently during execution)
    pub fn increment_counter(&self) {
        self.preemption_counter.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get the current preemption counter value
    pub fn get_counter(&self) -> u32 {
        self.preemption_counter.load(Ordering::Relaxed)
    }
    
    /// Force preemption (for emergency situations)
    pub fn force_preempt(&self) {
        self.preempt_flag.store(true, Ordering::Relaxed);
    }
    
    /// Get the quantum duration
    pub fn quantum(&self) -> Duration {
        self.quantum
    }
    
    /// Set a new quantum duration
    pub fn set_quantum(&mut self, quantum: Duration) {
        self.quantum = quantum;
    }
    
    /// Get the elapsed time since quantum start
    pub fn elapsed_time(&self) -> Option<Duration> {
        self.process_start.lock().unwrap()
            .map(|start| start.elapsed())
    }
    
    /// Check if quantum has expired
    pub fn quantum_expired(&self) -> bool {
        if let Some(start) = *self.process_start.lock().unwrap() {
            start.elapsed() >= self.quantum
        } else {
            false
        }
    }
    
    /// Start the timer thread
    fn start_timer_thread(
        quantum: Duration,
        process_start: Arc<Mutex<Option<Instant>>>,
        preempt_flag: Arc<AtomicBool>,
        running: Arc<AtomicBool>,
        preemption_counter: Arc<AtomicU32>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            // High-resolution timer interval (100 microseconds)
            let timer_interval = Duration::from_micros(100);
            
            while running.load(Ordering::Relaxed) {
                thread::sleep(timer_interval);
                
                // Check if a process is currently running
                if let Some(start) = *process_start.lock().unwrap() {
                    let elapsed = start.elapsed();
                    
                    // Check if quantum has expired
                    if elapsed >= quantum {
                        preempt_flag.store(true, Ordering::Relaxed);
                        *process_start.lock().unwrap() = None;
                    }
                    
                    // Also check if we've exceeded instruction count threshold
                    let counter = preemption_counter.load(Ordering::Relaxed);
                    if counter > 10000 { // 10k instructions per quantum max
                        preempt_flag.store(true, Ordering::Relaxed);
                        *process_start.lock().unwrap() = None;
                    }
                }
            }
        })
    }
}

impl Drop for PreemptionTimer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Execution result from a quantum
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Process was preempted due to time limit
    Preempted {
        instructions_executed: u64,
        messages_processed: u32,
        execution_time: Duration,
    },
    /// Process hit message processing limit
    MessageLimit {
        instructions_executed: u64,
        messages_processed: u32,
        execution_time: Duration,
    },
    /// Process voluntarily yielded
    Yielded {
        instructions_executed: u64,
        messages_processed: u32,
        execution_time: Duration,
    },
    /// Process is blocked waiting for I/O or messages
    Blocked {
        instructions_executed: u64,
        messages_processed: u32,
        execution_time: Duration,
    },
    /// Process terminated
    Terminated {
        instructions_executed: u64,
        messages_processed: u32,
        execution_time: Duration,
    },
}

impl ExecutionResult {
    /// Get the number of instructions executed
    pub fn instructions_executed(&self) -> u64 {
        match self {
            ExecutionResult::Preempted { instructions_executed, .. } => *instructions_executed,
            ExecutionResult::MessageLimit { instructions_executed, .. } => *instructions_executed,
            ExecutionResult::Yielded { instructions_executed, .. } => *instructions_executed,
            ExecutionResult::Blocked { instructions_executed, .. } => *instructions_executed,
            ExecutionResult::Terminated { instructions_executed, .. } => *instructions_executed,
        }
    }
    
    /// Get the number of messages processed
    pub fn messages_processed(&self) -> u32 {
        match self {
            ExecutionResult::Preempted { messages_processed, .. } => *messages_processed,
            ExecutionResult::MessageLimit { messages_processed, .. } => *messages_processed,
            ExecutionResult::Yielded { messages_processed, .. } => *messages_processed,
            ExecutionResult::Blocked { messages_processed, .. } => *messages_processed,
            ExecutionResult::Terminated { messages_processed, .. } => *messages_processed,
        }
    }
    
    /// Get the execution time
    pub fn execution_time(&self) -> Duration {
        match self {
            ExecutionResult::Preempted { execution_time, .. } => *execution_time,
            ExecutionResult::MessageLimit { execution_time, .. } => *execution_time,
            ExecutionResult::Yielded { execution_time, .. } => *execution_time,
            ExecutionResult::Blocked { execution_time, .. } => *execution_time,
            ExecutionResult::Terminated { execution_time, .. } => *execution_time,
        }
    }
    
    /// Check if the process should be rescheduled
    pub fn should_reschedule(&self) -> bool {
        match self {
            ExecutionResult::Preempted { .. } => true,
            ExecutionResult::MessageLimit { .. } => true,
            ExecutionResult::Yielded { .. } => true,
            ExecutionResult::Blocked { .. } => false, // Will be rescheduled when unblocked
            ExecutionResult::Terminated { .. } => false,
        }
    }
}

/// Statistics for preemption system
#[derive(Debug, Default)]
pub struct PreemptionStats {
    /// Total preemptions
    pub total_preemptions: u64,
    /// Preemptions due to time limit
    pub time_preemptions: u64,
    /// Preemptions due to instruction count
    pub instruction_preemptions: u64,
    /// Average quantum utilization (0.0 to 1.0)
    pub average_quantum_utilization: f64,
    /// Maximum quantum utilization seen
    pub max_quantum_utilization: f64,
    /// Total quantum time used
    pub total_quantum_time: Duration,
}

impl PreemptionStats {
    /// Record a preemption event
    pub fn record_preemption(&mut self, result: &ExecutionResult, quantum: Duration) {
        self.total_preemptions += 1;
        
        let utilization = result.execution_time().as_secs_f64() / quantum.as_secs_f64();
        self.average_quantum_utilization = 
            (self.average_quantum_utilization * (self.total_preemptions - 1) as f64 + utilization) 
            / self.total_preemptions as f64;
        
        if utilization > self.max_quantum_utilization {
            self.max_quantum_utilization = utilization;
        }
        
        self.total_quantum_time += result.execution_time();
        
        match result {
            ExecutionResult::Preempted { .. } => self.time_preemptions += 1,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_preemption_timer_basic() {
        let mut timer = PreemptionTimer::new(Duration::from_millis(10));
        timer.start().unwrap();
        
        // Should not preempt initially
        assert!(!timer.should_preempt());
        
        // Start a quantum
        timer.start_quantum();
        assert!(!timer.should_preempt());
        
        // Sleep longer than quantum
        thread::sleep(Duration::from_millis(15));
        
        // Should now preempt
        assert!(timer.should_preempt());
        
        timer.stop();
    }
    
    #[test]
    fn test_preemption_counter() {
        let timer = PreemptionTimer::new(Duration::from_millis(100));
        
        assert_eq!(timer.get_counter(), 0);
        
        timer.increment_counter();
        assert_eq!(timer.get_counter(), 1);
        
        timer.start_quantum();
        assert_eq!(timer.get_counter(), 0); // Reset on quantum start
    }
    
    #[test]
    fn test_execution_result() {
        let result = ExecutionResult::Preempted {
            instructions_executed: 1000,
            messages_processed: 5,
            execution_time: Duration::from_millis(10),
        };
        
        assert_eq!(result.instructions_executed(), 1000);
        assert_eq!(result.messages_processed(), 5);
        assert_eq!(result.execution_time(), Duration::from_millis(10));
        assert!(result.should_reschedule());
    }
}
