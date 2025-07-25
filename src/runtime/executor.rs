//! Process Executor with Preemptive Scheduling
//!
//! This module implements the process executor that enforces preemption
//! and provides detailed execution statistics.

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::{Duration, Instant};
use crate::runtime::preemption::{PreemptionTimer, ExecutionResult, PreemptionStats};
use crate::runtime::process::{Process, ProcessHandle};
use crate::runtime::actor::ReamActor;
use crate::types::{Pid, ProcessState};
use crate::error::{RuntimeError, RuntimeResult};

/// Maximum messages processed per quantum
const MAX_MESSAGES_PER_QUANTUM: u32 = 100;

/// Instruction count check interval
const PREEMPTION_CHECK_INTERVAL: u32 = 1000;

/// Process executor with preemptive scheduling
pub struct ProcessExecutor {
    /// Preemption timer
    timer: Arc<PreemptionTimer>,
    /// Execution statistics
    stats: ExecutorStats,
    /// Process being executed
    current_process: Option<Pid>,
}

/// Executor statistics
#[derive(Debug, Default)]
pub struct ExecutorStats {
    /// Total quanta executed
    pub total_quanta: u64,
    /// Total instructions executed
    pub total_instructions: u64,
    /// Total messages processed
    pub total_messages: u64,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Preemption statistics
    pub preemption_stats: PreemptionStats,
    /// Average instructions per quantum
    pub avg_instructions_per_quantum: f64,
    /// Average messages per quantum
    pub avg_messages_per_quantum: f64,
}

impl ProcessExecutor {
    /// Create a new process executor
    pub fn new(timer: Arc<PreemptionTimer>) -> Self {
        ProcessExecutor {
            timer,
            stats: ExecutorStats::default(),
            current_process: None,
        }
    }
    
    /// Execute a process with preemption
    pub fn execute_with_preemption(&mut self, handle: &ProcessHandle) -> RuntimeResult<ExecutionResult> {
        let pid = handle.pid();
        self.current_process = Some(pid);
        
        // Start the quantum timer
        self.timer.start_quantum();
        
        let start_time = Instant::now();
        let mut instructions_executed = 0u64;
        let mut messages_processed = 0u32;
        
        // Execute the process quantum
        let result = loop {
            // Check preemption every N instructions
            if instructions_executed % PREEMPTION_CHECK_INTERVAL as u64 == 0 {
                if self.timer.should_preempt() {
                    break ExecutionResult::Preempted {
                        instructions_executed,
                        messages_processed,
                        execution_time: start_time.elapsed(),
                    };
                }
            }
            
            // Increment instruction counter
            instructions_executed += 1;
            self.timer.increment_counter();
            
            // Check if process is still running
            if !handle.is_running() {
                break ExecutionResult::Terminated {
                    instructions_executed,
                    messages_processed,
                    execution_time: start_time.elapsed(),
                };
            }
            
            // Process messages from mailbox
            match self.process_messages(handle, &mut messages_processed)? {
                MessageProcessingResult::Continue => {
                    // Continue execution
                }
                MessageProcessingResult::MessageLimit => {
                    break ExecutionResult::MessageLimit {
                        instructions_executed,
                        messages_processed,
                        execution_time: start_time.elapsed(),
                    };
                }
                MessageProcessingResult::Blocked => {
                    break ExecutionResult::Blocked {
                        instructions_executed,
                        messages_processed,
                        execution_time: start_time.elapsed(),
                    };
                }
                MessageProcessingResult::Yielded => {
                    break ExecutionResult::Yielded {
                        instructions_executed,
                        messages_processed,
                        execution_time: start_time.elapsed(),
                    };
                }
            }
            
            // Simulate some work (in a real implementation, this would be
            // the actual process execution)
            if instructions_executed >= 10000 {
                // Prevent infinite loops in testing
                break ExecutionResult::Preempted {
                    instructions_executed,
                    messages_processed,
                    execution_time: start_time.elapsed(),
                };
            }
        };
        
        // Update statistics
        self.update_stats(&result);
        self.current_process = None;
        
        Ok(result)
    }
    
    /// Process messages from the process mailbox
    fn process_messages(
        &self,
        handle: &ProcessHandle,
        messages_processed: &mut u32,
    ) -> RuntimeResult<MessageProcessingResult> {
        // In a real implementation, this would process actual messages
        // For now, we simulate message processing
        
        let messages_in_mailbox = self.simulate_message_count();
        
        for _ in 0..messages_in_mailbox {
            if *messages_processed >= MAX_MESSAGES_PER_QUANTUM {
                return Ok(MessageProcessingResult::MessageLimit);
            }
            
            // Check for preemption during message processing
            if self.timer.should_preempt() {
                return Ok(MessageProcessingResult::Continue);
            }
            
            // Simulate message processing
            *messages_processed += 1;
            
            // Simulate different message processing outcomes
            if *messages_processed % 50 == 0 {
                // Occasionally yield
                return Ok(MessageProcessingResult::Yielded);
            }
        }
        
        // No more messages to process
        if messages_in_mailbox == 0 {
            Ok(MessageProcessingResult::Blocked)
        } else {
            Ok(MessageProcessingResult::Continue)
        }
    }
    
    /// Simulate message count (replace with actual mailbox check)
    fn simulate_message_count(&self) -> u32 {
        // Simulate varying message loads
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        
        (hash % 20) as u32 // 0-19 messages
    }
    
    /// Update executor statistics
    fn update_stats(&mut self, result: &ExecutionResult) {
        self.stats.total_quanta += 1;
        self.stats.total_instructions += result.instructions_executed();
        self.stats.total_messages += result.messages_processed() as u64;
        self.stats.total_execution_time += result.execution_time();
        
        // Update preemption statistics
        self.stats.preemption_stats.record_preemption(result, self.timer.quantum());
        
        // Update averages
        self.stats.avg_instructions_per_quantum = 
            self.stats.total_instructions as f64 / self.stats.total_quanta as f64;
        self.stats.avg_messages_per_quantum = 
            self.stats.total_messages as f64 / self.stats.total_quanta as f64;
    }
    
    /// Get current process being executed
    pub fn current_process(&self) -> Option<Pid> {
        self.current_process
    }
    
    /// Get executor statistics
    pub fn stats(&self) -> &ExecutorStats {
        &self.stats
    }
    
    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = ExecutorStats::default();
    }
    
    /// Force preemption of current process
    pub fn force_preempt(&self) {
        self.timer.force_preempt();
    }
    
    /// Check if currently executing a process
    pub fn is_executing(&self) -> bool {
        self.current_process.is_some()
    }
}

/// Result of message processing
#[derive(Debug, Clone, Copy)]
enum MessageProcessingResult {
    /// Continue execution
    Continue,
    /// Hit message processing limit
    MessageLimit,
    /// Process is blocked waiting for messages
    Blocked,
    /// Process voluntarily yielded
    Yielded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::process::Process;
    use crate::runtime::actor::TestActor;
    use crate::types::Priority;
    use std::sync::Arc;

    #[test]
    fn test_process_executor_basic() {
        let timer = Arc::new(PreemptionTimer::new(Duration::from_millis(10)));
        timer.start().unwrap();
        
        let mut executor = ProcessExecutor::new(timer);
        
        // Create a test process
        let pid = Pid::new();
        let actor = Box::new(TestActor::new());
        let process = Process::new(pid, actor, Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        // Execute the process
        let result = executor.execute_with_preemption(&handle).unwrap();
        
        // Should have executed some instructions
        assert!(result.instructions_executed() > 0);
        
        // Check statistics
        let stats = executor.stats();
        assert_eq!(stats.total_quanta, 1);
        assert!(stats.total_instructions > 0);
    }
    
    #[test]
    fn test_preemption_enforcement() {
        let timer = Arc::new(PreemptionTimer::new(Duration::from_millis(1))); // Very short quantum
        timer.start().unwrap();
        
        let mut executor = ProcessExecutor::new(timer);
        
        // Create a test process
        let pid = Pid::new();
        let actor = Box::new(TestActor::new());
        let process = Process::new(pid, actor, Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        // Execute the process
        let result = executor.execute_with_preemption(&handle).unwrap();
        
        // Should be preempted due to time limit
        match result {
            ExecutionResult::Preempted { .. } => {
                // Expected
            }
            _ => panic!("Expected preemption, got {:?}", result),
        }
    }
}
