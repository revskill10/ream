//! Scheduler implementation as free monad over scheduling algebra

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::time::{Duration, Instant};
use crate::types::{Pid, Priority};
use crate::error::{RuntimeError, RuntimeResult};

/// Scheduling operations as algebraic data type
#[derive(Debug)]
pub enum SchedulingOp {
    /// Schedule a process with given priority
    Schedule(Pid, Priority),
    /// Yield current process
    Yield(Pid),
    /// Suspend a process
    Suspend(Pid),
    /// Resume a suspended process
    Resume(Pid),
    /// Remove a process from scheduling
    Remove(Pid),
}

/// Scheduled process entry
#[derive(Debug, Clone)]
struct ScheduledProcess {
    pid: Pid,
    priority: Priority,
    quantum_start: Option<Instant>,
    total_runtime: Duration,
    last_scheduled: Instant,
}

impl ScheduledProcess {
    fn new(pid: Pid, priority: Priority) -> Self {
        ScheduledProcess {
            pid,
            priority,
            quantum_start: None,
            total_runtime: Duration::new(0, 0),
            last_scheduled: Instant::now(),
        }
    }
}

impl PartialEq for ScheduledProcess {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}

impl Eq for ScheduledProcess {}

impl PartialOrd for ScheduledProcess {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledProcess {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first (reverse order since High=0, Normal=1, Low=2)
        // then by last scheduled time (fairness)
        match other.priority.cmp(&self.priority) {
            Ordering::Equal => other.last_scheduled.cmp(&self.last_scheduled),
            priority_order => priority_order,
        }
    }
}

/// Process scheduler with priority queues and fairness
pub struct Scheduler {
    /// Ready queue (priority heap)
    ready_queue: BinaryHeap<ScheduledProcess>,
    
    /// Suspended processes
    suspended: HashMap<Pid, ScheduledProcess>,
    
    /// Currently running process
    current: Option<ScheduledProcess>,
    
    /// Quantum duration
    quantum: Duration,
    
    /// Total scheduled processes
    total_scheduled: u64,
    
    /// Scheduler statistics
    stats: SchedulerStats,
}

#[derive(Debug, Default)]
struct SchedulerStats {
    context_switches: u64,
    total_quantum_time: Duration,
    processes_scheduled: u64,
}

impl Scheduler {
    /// Create a new scheduler with default quantum (1ms)
    pub fn new() -> Self {
        Self::with_quantum(Duration::from_millis(1))
    }
    
    /// Create a new scheduler with custom quantum
    pub fn with_quantum(quantum: Duration) -> Self {
        Scheduler {
            ready_queue: BinaryHeap::new(),
            suspended: HashMap::new(),
            current: None,
            quantum,
            total_scheduled: 0,
            stats: SchedulerStats::default(),
        }
    }
    
    /// Schedule a process with given priority
    pub fn schedule(&mut self, pid: Pid, priority: Priority) -> RuntimeResult<()> {
        let process = ScheduledProcess::new(pid, priority);
        self.ready_queue.push(process);
        self.total_scheduled += 1;
        self.stats.processes_scheduled += 1;
        Ok(())
    }
    
    /// Get the next process to run
    pub fn next_process(&mut self) -> Option<Pid> {
        // Check if current process quantum expired
        if let Some(ref current) = self.current {
            if let Some(start) = current.quantum_start {
                if start.elapsed() >= self.quantum {
                    // Quantum expired, preempt current process
                    self.preempt_current();
                }
            }
        }
        
        // If no current process, get next from ready queue
        if self.current.is_none() {
            if let Some(mut process) = self.ready_queue.pop() {
                process.quantum_start = Some(Instant::now());
                process.last_scheduled = Instant::now();
                let pid = process.pid;
                self.current = Some(process);
                self.stats.context_switches += 1;
                return Some(pid);
            }
        }
        
        // Return current process PID if still running
        self.current.as_ref().map(|p| p.pid)
    }
    
    /// Yield the current process
    pub fn yield_process(&mut self, pid: Pid) -> RuntimeResult<()> {
        if let Some(current) = self.current.take() {
            if current.pid == pid {
                // Add quantum time to total runtime
                if let Some(start) = current.quantum_start {
                    let quantum_time = start.elapsed();
                    let mut updated = current;
                    updated.total_runtime += quantum_time;
                    updated.quantum_start = None;
                    self.stats.total_quantum_time += quantum_time;
                    
                    // Re-queue the process
                    self.ready_queue.push(updated);
                }
            } else {
                // Wrong process trying to yield
                let current_pid = current.pid;
                self.current = Some(current);
                return Err(RuntimeError::Scheduler(
                    format!("Process {} tried to yield, but {} is running", pid, current_pid)
                ));
            }
        }
        Ok(())
    }
    
    /// Suspend a process
    pub fn suspend(&mut self, pid: Pid) -> RuntimeResult<()> {
        // Remove from ready queue
        let mut new_ready = BinaryHeap::new();
        let mut found = false;
        
        while let Some(process) = self.ready_queue.pop() {
            if process.pid == pid {
                self.suspended.insert(pid, process);
                found = true;
            } else {
                new_ready.push(process);
            }
        }
        
        self.ready_queue = new_ready;
        
        // Check if it's the current process
        if let Some(current) = self.current.take() {
            if current.pid == pid {
                self.suspended.insert(pid, current);
                found = true;
            } else {
                self.current = Some(current);
            }
        }
        
        if !found {
            return Err(RuntimeError::ProcessNotFound(pid));
        }
        
        Ok(())
    }
    
    /// Resume a suspended process
    pub fn resume(&mut self, pid: Pid) -> RuntimeResult<()> {
        if let Some(process) = self.suspended.remove(&pid) {
            self.ready_queue.push(process);
            Ok(())
        } else {
            Err(RuntimeError::ProcessNotFound(pid))
        }
    }
    
    /// Remove a process from scheduling
    pub fn remove(&mut self, pid: Pid) -> RuntimeResult<()> {
        // Remove from ready queue
        let mut new_ready = BinaryHeap::new();
        let mut found = false;
        
        while let Some(process) = self.ready_queue.pop() {
            if process.pid == pid {
                found = true;
            } else {
                new_ready.push(process);
            }
        }
        
        self.ready_queue = new_ready;
        
        // Remove from suspended
        if self.suspended.remove(&pid).is_some() {
            found = true;
        }
        
        // Check if it's the current process
        if let Some(current) = self.current.take() {
            if current.pid == pid {
                found = true;
            } else {
                self.current = Some(current);
            }
        }
        
        if !found {
            return Err(RuntimeError::ProcessNotFound(pid));
        }
        
        Ok(())
    }
    
    /// Get scheduler statistics
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }
    
    /// Get number of processes in ready queue
    pub fn ready_count(&self) -> usize {
        self.ready_queue.len()
    }
    
    /// Get number of suspended processes
    pub fn suspended_count(&self) -> usize {
        self.suspended.len()
    }
    
    /// Check if a process is scheduled
    pub fn is_scheduled(&self, pid: Pid) -> bool {
        self.ready_queue.iter().any(|p| p.pid == pid) ||
        self.suspended.contains_key(&pid) ||
        self.current.as_ref().map_or(false, |p| p.pid == pid)
    }
    
    /// Get current running process
    pub fn current_process(&self) -> Option<Pid> {
        self.current.as_ref().map(|p| p.pid)
    }
    
    // Private helper methods
    
    fn preempt_current(&mut self) {
        if let Some(current) = self.current.take() {
            if let Some(start) = current.quantum_start {
                let quantum_time = start.elapsed();
                let mut updated = current;
                updated.total_runtime += quantum_time;
                updated.quantum_start = None;
                self.stats.total_quantum_time += quantum_time;
                
                // Re-queue the process
                self.ready_queue.push(updated);
            }
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_scheduler_basic() {
        let mut scheduler = Scheduler::new();
        let pid1 = Pid::new();
        let pid2 = Pid::new();

        scheduler.schedule(pid1, Priority::Normal).unwrap();
        scheduler.schedule(pid2, Priority::High).unwrap();

        // High priority should come first
        let next_pid = scheduler.next_process();
        assert_eq!(next_pid, Some(pid2));
        assert_eq!(scheduler.current_process(), Some(pid2));

        // After yielding, high priority process should still come first
        scheduler.yield_process(pid2).unwrap();
        let next_pid = scheduler.next_process();
        assert_eq!(next_pid, Some(pid2)); // High priority process runs again

        // Remove the high priority process to test normal priority
        scheduler.remove(pid2).unwrap();
        let next_pid = scheduler.next_process();
        assert_eq!(next_pid, Some(pid1)); // Now normal priority runs
    }
    
    #[test]
    fn test_scheduler_suspend_resume() {
        let mut scheduler = Scheduler::new();
        let pid = Pid::new();
        
        scheduler.schedule(pid, Priority::Normal).unwrap();
        scheduler.suspend(pid).unwrap();
        
        assert_eq!(scheduler.next_process(), None);
        assert_eq!(scheduler.suspended_count(), 1);
        
        scheduler.resume(pid).unwrap();
        assert_eq!(scheduler.next_process(), Some(pid));
        assert_eq!(scheduler.suspended_count(), 0);
    }
    
    #[test]
    fn test_scheduler_quantum() {
        let mut scheduler = Scheduler::with_quantum(Duration::from_millis(10));
        let pid = Pid::new();

        scheduler.schedule(pid, Priority::Normal).unwrap();
        assert_eq!(scheduler.next_process(), Some(pid));

        // Sleep longer than quantum
        thread::sleep(Duration::from_millis(15));

        // Should still return same process until we call next_process again
        assert_eq!(scheduler.current_process(), Some(pid));

        // Next call should preempt and re-schedule the same process (since it's the only one)
        assert_eq!(scheduler.next_process(), Some(pid));
        // Process should be current again, not in ready queue
        assert_eq!(scheduler.ready_count(), 0);
        assert_eq!(scheduler.current_process(), Some(pid));
    }
}
