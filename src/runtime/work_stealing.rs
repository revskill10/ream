//! Work-Stealing Scheduler for Multi-Core Systems
//!
//! This module implements a work-stealing scheduler that maximizes CPU utilization
//! across multiple cores as specified in PREEMPTIVE_SCHEDULING.md

use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::collections::{VecDeque, HashMap};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use crossbeam::deque::{Injector, Stealer, Worker};
use parking_lot::RwLock;
use crate::runtime::process::{Process, ProcessHandle};
use crate::runtime::preemption::{PreemptionTimer, ExecutionResult};
use crate::runtime::executor::ProcessExecutor;
use crate::types::{Pid, Priority};
use crate::error::{RuntimeError, RuntimeResult};

/// Work-stealing scheduler for multi-core systems
pub struct WorkStealingScheduler {
    /// Number of worker threads
    num_workers: usize,
    /// Global task injector
    global_queue: Arc<Injector<ScheduledTask>>,
    /// Per-worker task queues
    worker_queues: Vec<Worker<ScheduledTask>>,
    /// Stealers for each worker queue
    stealers: Vec<Stealer<ScheduledTask>>,
    /// Worker thread handles
    worker_handles: Vec<Option<JoinHandle<()>>>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Statistics
    stats: Arc<RwLock<WorkStealingStats>>,
    /// Process registry
    processes: Arc<RwLock<HashMap<Pid, ProcessHandle>>>,
    /// Load balancer
    load_balancer: LoadBalancer,
}

/// A task scheduled for execution
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Process ID
    pub pid: Pid,
    /// Task priority
    pub priority: Priority,
    /// Scheduling timestamp
    pub scheduled_at: Instant,
    /// Number of times rescheduled
    pub reschedule_count: u32,
    /// Preferred core (for affinity)
    pub preferred_core: Option<usize>,
}

/// Statistics for work-stealing scheduler
#[derive(Debug, Default, Clone)]
pub struct WorkStealingStats {
    /// Total tasks executed
    pub total_tasks: u64,
    /// Tasks stolen between workers
    pub tasks_stolen: u64,
    /// Tasks executed per worker
    pub tasks_per_worker: Vec<u64>,
    /// Steal attempts per worker
    pub steal_attempts_per_worker: Vec<u64>,
    /// Successful steals per worker
    pub successful_steals_per_worker: Vec<u64>,
    /// Average task execution time
    pub avg_execution_time: Duration,
    /// Load imbalance factor (0.0 = perfect balance, 1.0 = maximum imbalance)
    pub load_imbalance: f64,
    /// Total idle time per worker
    pub idle_time_per_worker: Vec<Duration>,
}

/// Load balancer for work distribution
struct LoadBalancer {
    /// Load per worker (number of tasks)
    worker_loads: Arc<RwLock<Vec<AtomicUsize>>>,
    /// Last rebalance time
    last_rebalance: Arc<Mutex<Instant>>,
    /// Rebalance interval
    rebalance_interval: Duration,
}

impl WorkStealingScheduler {
    /// Create a new work-stealing scheduler
    pub fn new(num_workers: Option<usize>) -> Self {
        let num_workers = num_workers.unwrap_or_else(|| num_cpus::get());
        let global_queue = Arc::new(Injector::new());
        
        let mut worker_queues = Vec::with_capacity(num_workers);
        let mut stealers = Vec::with_capacity(num_workers);
        
        // Create worker queues and stealers
        for _ in 0..num_workers {
            let worker = Worker::new_fifo();
            let stealer = worker.stealer();
            worker_queues.push(worker);
            stealers.push(stealer);
        }
        
        let stats = Arc::new(RwLock::new(WorkStealingStats {
            tasks_per_worker: vec![0; num_workers],
            steal_attempts_per_worker: vec![0; num_workers],
            successful_steals_per_worker: vec![0; num_workers],
            idle_time_per_worker: vec![Duration::default(); num_workers],
            ..Default::default()
        }));
        
        let load_balancer = LoadBalancer::new(num_workers);
        
        WorkStealingScheduler {
            num_workers,
            global_queue,
            worker_queues,
            stealers,
            worker_handles: (0..num_workers).map(|_| None).collect(),
            running: Arc::new(AtomicBool::new(false)),
            stats,
            processes: Arc::new(RwLock::new(HashMap::new())),
            load_balancer,
        }
    }
    
    /// Start the scheduler
    pub fn start(&mut self) -> RuntimeResult<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.running.store(true, Ordering::Relaxed);
        
        // Start worker threads
        for worker_id in 0..self.num_workers {
            let handle = self.start_worker_thread(worker_id)?;
            self.worker_handles[worker_id] = Some(handle);
        }
        
        Ok(())
    }
    
    /// Stop the scheduler
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        
        // Wait for all worker threads to finish
        for handle in self.worker_handles.iter_mut() {
            if let Some(handle) = handle.take() {
                let _ = handle.join();
            }
        }
    }
    
    /// Schedule a task
    pub fn schedule_task(&self, task: ScheduledTask) {
        // Try to place on preferred core first
        if let Some(preferred_core) = task.preferred_core {
            if preferred_core < self.num_workers {
                if self.worker_queues[preferred_core].len() < 100 { // Avoid overloading
                    self.worker_queues[preferred_core].push(task);
                    self.load_balancer.increment_load(preferred_core);
                    return;
                }
            }
        }
        
        // Find least loaded worker
        let least_loaded = self.load_balancer.find_least_loaded_worker();
        if self.worker_queues[least_loaded].len() < 100 {
            self.worker_queues[least_loaded].push(task);
            self.load_balancer.increment_load(least_loaded);
        } else {
            // All workers are busy, use global queue
            self.global_queue.push(task);
        }
    }
    
    /// Register a process
    pub fn register_process(&self, handle: ProcessHandle) {
        let pid = handle.pid();
        self.processes.write().insert(pid, handle);
    }
    
    /// Unregister a process
    pub fn unregister_process(&self, pid: Pid) {
        self.processes.write().remove(&pid);
    }
    
    /// Get scheduler statistics
    pub fn stats(&self) -> WorkStealingStats {
        (*self.stats.read()).clone()
    }
    
    /// Start a worker thread
    fn start_worker_thread(&self, worker_id: usize) -> RuntimeResult<JoinHandle<()>> {
        let global_queue = Arc::clone(&self.global_queue);
        // TODO: Fix Worker clone issue - crossbeam::deque::Worker doesn't implement Clone
        // This needs architectural changes to properly handle work-stealing
        let stealers = self.stealers.clone();
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let processes = Arc::clone(&self.processes);
        let load_balancer = self.load_balancer.clone();

        let handle = thread::Builder::new()
            .name(format!("work-stealing-{}", worker_id))
            .spawn(move || {
                // TODO: Fix ProcessExecutor timer access - timer field is private
                // let mut executor = ProcessExecutor::new(Arc::new(PreemptionTimer::new(Duration::from_millis(10))));
                // executor.timer.start().unwrap();
                
                // TODO: Implement proper work-stealing worker thread
                // This requires fixing the Worker clone issue and ProcessExecutor timer access
                while running.load(Ordering::Relaxed) {
                    // Placeholder implementation - just sleep to avoid busy waiting
                    thread::sleep(Duration::from_millis(10));
                }
            })?;
        
        Ok(handle)
    }
    
    /// Find a task to execute (local -> global -> steal)
    fn find_task(
        worker_queue: &Worker<ScheduledTask>,
        global_queue: &Injector<ScheduledTask>,
        stealers: &[Stealer<ScheduledTask>],
        worker_id: usize,
        stats: &Arc<RwLock<WorkStealingStats>>,
    ) -> Option<ScheduledTask> {
        // 1. Try local queue first
        if let Some(task) = worker_queue.pop() {
            return Some(task);
        }
        
        // 2. Try global queue
        loop {
            match global_queue.steal_batch_and_pop(worker_queue) {
                crossbeam::deque::Steal::Success(task) => return Some(task),
                crossbeam::deque::Steal::Empty => break,
                crossbeam::deque::Steal::Retry => continue,
            }
        }
        
        // 3. Try stealing from other workers
        stats.write().steal_attempts_per_worker[worker_id] += 1;
        
        // Randomize steal order to avoid contention
        let mut steal_order: Vec<usize> = (0..stealers.len()).collect();
        steal_order.remove(worker_id); // Don't steal from self
        
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        steal_order.shuffle(&mut rng);
        
        for &target in &steal_order {
            loop {
                match stealers[target].steal_batch_and_pop(worker_queue) {
                    crossbeam::deque::Steal::Success(task) => {
                        stats.write().successful_steals_per_worker[worker_id] += 1;
                        stats.write().tasks_stolen += 1;
                        return Some(task);
                    }
                    crossbeam::deque::Steal::Empty => break,
                    crossbeam::deque::Steal::Retry => continue,
                }
            }
        }
        
        None
    }
    
    /// Handle execution result and potentially reschedule
    fn handle_execution_result(
        mut task: ScheduledTask,
        result: ExecutionResult,
        global_queue: &Injector<ScheduledTask>,
        stats: &Arc<RwLock<WorkStealingStats>>,
    ) {
        stats.write().total_tasks += 1;
        
        match result {
            ExecutionResult::Preempted { .. } | ExecutionResult::MessageLimit { .. } => {
                // Reschedule the task
                task.reschedule_count += 1;
                task.scheduled_at = Instant::now();
                global_queue.push(task);
            }
            ExecutionResult::Yielded { .. } => {
                // Reschedule with lower priority
                task.reschedule_count += 1;
                task.scheduled_at = Instant::now();
                global_queue.push(task);
            }
            ExecutionResult::Blocked { .. } => {
                // Don't reschedule blocked tasks immediately
                // They will be rescheduled when unblocked
            }
            ExecutionResult::Terminated { .. } => {
                // Task completed, don't reschedule
            }
        }
    }
}

impl LoadBalancer {
    fn new(num_workers: usize) -> Self {
        let worker_loads = Arc::new(RwLock::new(
            (0..num_workers).map(|_| AtomicUsize::new(0)).collect()
        ));
        
        LoadBalancer {
            worker_loads,
            last_rebalance: Arc::new(Mutex::new(Instant::now())),
            rebalance_interval: Duration::from_millis(100),
        }
    }
    
    fn clone(&self) -> Self {
        LoadBalancer {
            worker_loads: Arc::clone(&self.worker_loads),
            last_rebalance: Arc::clone(&self.last_rebalance),
            rebalance_interval: self.rebalance_interval,
        }
    }
    
    fn increment_load(&self, worker_id: usize) {
        if let Some(load) = self.worker_loads.read().get(worker_id) {
            load.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    fn decrement_load(&self, worker_id: usize) {
        if let Some(load) = self.worker_loads.read().get(worker_id) {
            load.fetch_sub(1, Ordering::Relaxed);
        }
    }
    
    fn find_least_loaded_worker(&self) -> usize {
        let loads = self.worker_loads.read();
        loads.iter()
            .enumerate()
            .min_by_key(|(_, load)| load.load(Ordering::Relaxed))
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }
}

impl ScheduledTask {
    /// Create a new scheduled task
    pub fn new(pid: Pid, priority: Priority) -> Self {
        ScheduledTask {
            pid,
            priority,
            scheduled_at: Instant::now(),
            reschedule_count: 0,
            preferred_core: None,
        }
    }
    
    /// Create a task with core affinity
    pub fn with_affinity(pid: Pid, priority: Priority, preferred_core: usize) -> Self {
        ScheduledTask {
            pid,
            priority,
            scheduled_at: Instant::now(),
            reschedule_count: 0,
            preferred_core: Some(preferred_core),
        }
    }
    
    /// Get task age
    pub fn age(&self) -> Duration {
        self.scheduled_at.elapsed()
    }
    
    /// Check if task should be deprioritized due to excessive rescheduling
    pub fn should_deprioritize(&self) -> bool {
        self.reschedule_count > 10
    }
}

impl Drop for WorkStealingScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::actor::TestActor;

    #[test]
    fn test_work_stealing_scheduler_creation() {
        let scheduler = WorkStealingScheduler::new(Some(4));
        assert_eq!(scheduler.num_workers, 4);
        assert_eq!(scheduler.worker_queues.len(), 4);
        assert_eq!(scheduler.stealers.len(), 4);
    }

    #[test]
    fn test_scheduled_task_creation() {
        let pid = Pid::new();
        let task = ScheduledTask::new(pid, Priority::Normal);
        
        assert_eq!(task.pid, pid);
        assert_eq!(task.priority, Priority::Normal);
        assert_eq!(task.reschedule_count, 0);
        assert!(task.preferred_core.is_none());
    }

    #[test]
    fn test_task_with_affinity() {
        let pid = Pid::new();
        let task = ScheduledTask::with_affinity(pid, Priority::High, 2);
        
        assert_eq!(task.preferred_core, Some(2));
    }

    #[test]
    fn test_load_balancer() {
        let balancer = LoadBalancer::new(4);
        
        // Initially, worker 0 should be least loaded
        assert_eq!(balancer.find_least_loaded_worker(), 0);
        
        // Add load to worker 0
        balancer.increment_load(0);
        balancer.increment_load(0);
        
        // Now worker 1 should be least loaded
        assert_eq!(balancer.find_least_loaded_worker(), 1);
    }
}
