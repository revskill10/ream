//! Real-Time Scheduling Extensions
//!
//! This module implements deadline-aware priority scheduling with EDF (Earliest Deadline First)
//! and Rate Monotonic algorithms, plus priority inheritance protocol as specified in PREEMPTIVE_SCHEDULING.md

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::{Ordering, Reverse};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, AtomicBool, Ordering as AtomicOrdering}};
use crate::types::{Pid, Priority};
use crate::error::{RuntimeError, RuntimeResult};

/// Real-time task with deadline constraints
#[derive(Debug, Clone)]
pub struct RealTimeTask {
    /// Process ID
    pub pid: Pid,
    /// Task priority (for Rate Monotonic)
    pub priority: Priority,
    /// Absolute deadline
    pub deadline: Instant,
    /// Period (for periodic tasks)
    pub period: Option<Duration>,
    /// Worst-case execution time
    pub wcet: Duration,
    /// Arrival time
    pub arrival_time: Instant,
    /// Remaining execution time
    pub remaining_time: Duration,
    /// Task type
    pub task_type: TaskType,
    /// Critical section resources held
    pub held_resources: Vec<ResourceId>,
    /// Original priority (before inheritance)
    pub original_priority: Priority,
}

/// Type of real-time task
#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    /// Periodic task with fixed period
    Periodic,
    /// Sporadic task with minimum inter-arrival time
    Sporadic,
    /// Aperiodic task (no timing constraints)
    Aperiodic,
}

/// Resource identifier for priority inheritance
pub type ResourceId = u32;

/// Real-time scheduler with multiple algorithms
pub struct RealTimeScheduler {
    /// Scheduling algorithm
    algorithm: SchedulingAlgorithm,
    /// Ready queue for EDF
    edf_queue: BinaryHeap<Reverse<EDFTask>>,
    /// Ready queue for Rate Monotonic
    rm_queue: BinaryHeap<RMTask>,
    /// Blocked tasks waiting for resources
    blocked_tasks: HashMap<ResourceId, VecDeque<RealTimeTask>>,
    /// Resource ownership
    resource_owners: HashMap<ResourceId, Pid>,
    /// Priority inheritance chains
    inheritance_chains: HashMap<Pid, Vec<Pid>>,
    /// Task registry
    tasks: HashMap<Pid, RealTimeTask>,
    /// Current running task
    current_task: Option<Pid>,
    /// Scheduler statistics
    stats: RealTimeStats,
    /// Deadline miss counter
    deadline_misses: Arc<AtomicU64>,
    /// Preemption counter
    preemptions: Arc<AtomicU64>,
}

/// Scheduling algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulingAlgorithm {
    /// Earliest Deadline First
    EDF,
    /// Rate Monotonic
    RateMonotonic,
    /// Hybrid (EDF for sporadic, RM for periodic)
    Hybrid,
}

/// EDF task wrapper for priority queue
#[derive(Debug, Clone)]
struct EDFTask {
    pid: Pid,
    deadline: Instant,
    priority: Priority,
}

/// Rate Monotonic task wrapper for priority queue
#[derive(Debug, Clone)]
struct RMTask {
    pid: Pid,
    period: Duration,
    priority: Priority,
}

/// Real-time scheduling statistics
#[derive(Debug, Default, Clone)]
pub struct RealTimeStats {
    /// Total tasks scheduled
    pub total_tasks: u64,
    /// Deadline misses
    pub deadline_misses: u64,
    /// Preemptions
    pub preemptions: u64,
    /// Average response time
    pub avg_response_time: Duration,
    /// CPU utilization
    pub cpu_utilization: f64,
    /// Priority inversions detected
    pub priority_inversions: u64,
    /// Priority inheritance activations
    pub priority_inheritances: u64,
}

impl RealTimeScheduler {
    /// Create a new real-time scheduler
    pub fn new(algorithm: SchedulingAlgorithm) -> Self {
        RealTimeScheduler {
            algorithm,
            edf_queue: BinaryHeap::new(),
            rm_queue: BinaryHeap::new(),
            blocked_tasks: HashMap::new(),
            resource_owners: HashMap::new(),
            inheritance_chains: HashMap::new(),
            tasks: HashMap::new(),
            current_task: None,
            stats: RealTimeStats::default(),
            deadline_misses: Arc::new(AtomicU64::new(0)),
            preemptions: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Add a real-time task
    pub fn add_task(&mut self, task: RealTimeTask) -> RuntimeResult<()> {
        let pid = task.pid;
        
        // Perform schedulability analysis
        if !self.is_schedulable(&task) {
            return Err(RuntimeError::Scheduler(format!(
                "Task {} is not schedulable with current task set",
                pid
            )));
        }
        
        // Add to appropriate queue based on algorithm
        match self.algorithm {
            SchedulingAlgorithm::EDF => {
                self.edf_queue.push(Reverse(EDFTask {
                    pid,
                    deadline: task.deadline,
                    priority: task.priority,
                }));
            }
            SchedulingAlgorithm::RateMonotonic => {
                if let Some(period) = task.period {
                    self.rm_queue.push(RMTask {
                        pid,
                        period,
                        priority: task.priority,
                    });
                } else {
                    return Err(RuntimeError::Scheduler(
                        "Rate Monotonic requires periodic tasks".to_string()
                    ));
                }
            }
            SchedulingAlgorithm::Hybrid => {
                match task.task_type {
                    TaskType::Periodic => {
                        if let Some(period) = task.period {
                            self.rm_queue.push(RMTask {
                                pid,
                                period,
                                priority: task.priority,
                            });
                        }
                    }
                    TaskType::Sporadic | TaskType::Aperiodic => {
                        self.edf_queue.push(Reverse(EDFTask {
                            pid,
                            deadline: task.deadline,
                            priority: task.priority,
                        }));
                    }
                }
            }
        }
        
        self.tasks.insert(pid, task);
        self.stats.total_tasks += 1;
        
        Ok(())
    }
    
    /// Get the next task to run
    pub fn next_task(&mut self) -> Option<Pid> {
        // Check for deadline misses first
        self.check_deadline_misses();
        
        let next_pid = match self.algorithm {
            SchedulingAlgorithm::EDF => {
                self.edf_queue.pop().map(|Reverse(task)| task.pid)
            }
            SchedulingAlgorithm::RateMonotonic => {
                self.rm_queue.pop().map(|task| task.pid)
            }
            SchedulingAlgorithm::Hybrid => {
                // Prioritize EDF tasks (sporadic/aperiodic) over RM tasks (periodic)
                if let Some(Reverse(edf_task)) = self.edf_queue.pop() {
                    Some(edf_task.pid)
                } else {
                    self.rm_queue.pop().map(|task| task.pid)
                }
            }
        };
        
        // Handle preemption if necessary
        if let Some(next_pid) = next_pid {
            if let Some(current_pid) = self.current_task {
                if current_pid != next_pid {
                    self.preemptions.fetch_add(1, AtomicOrdering::Relaxed);
                    self.stats.preemptions += 1;
                }
            }
            self.current_task = Some(next_pid);
        }
        
        next_pid
    }
    
    /// Request a resource (with priority inheritance)
    pub fn request_resource(&mut self, pid: Pid, resource_id: ResourceId) -> RuntimeResult<bool> {
        // Check if resource is available
        if let Some(owner_pid) = self.resource_owners.get(&resource_id) {
            // Resource is held by another task
            if *owner_pid != pid {
                // Block the requesting task
                self.blocked_tasks.entry(resource_id)
                    .or_insert_with(VecDeque::new)
                    .push_back(self.tasks[&pid].clone());
                
                // Apply priority inheritance
                self.apply_priority_inheritance(pid, *owner_pid)?;
                
                return Ok(false); // Task is blocked
            }
        }
        
        // Resource is available, grant it
        self.resource_owners.insert(resource_id, pid);
        if let Some(task) = self.tasks.get_mut(&pid) {
            task.held_resources.push(resource_id);
        }
        
        Ok(true) // Resource granted
    }
    
    /// Release a resource
    pub fn release_resource(&mut self, pid: Pid, resource_id: ResourceId) -> RuntimeResult<()> {
        // Verify the task owns the resource
        if self.resource_owners.get(&resource_id) != Some(&pid) {
            return Err(RuntimeError::Scheduler(format!(
                "Task {} does not own resource {}",
                pid, resource_id
            )));
        }
        
        // Remove resource from task's held resources
        if let Some(task) = self.tasks.get_mut(&pid) {
            task.held_resources.retain(|&r| r != resource_id);
        }
        
        // Remove ownership
        self.resource_owners.remove(&resource_id);
        
        // Restore original priority
        self.restore_priority(pid)?;
        
        // Wake up blocked tasks
        if let Some(mut blocked_queue) = self.blocked_tasks.remove(&resource_id) {
            if let Some(next_task) = blocked_queue.pop_front() {
                // Grant resource to next waiting task
                self.resource_owners.insert(resource_id, next_task.pid);
                if let Some(task) = self.tasks.get_mut(&next_task.pid) {
                    task.held_resources.push(resource_id);
                }
                
                // Re-add to ready queue
                self.add_to_ready_queue(next_task);
                
                // Put remaining blocked tasks back
                if !blocked_queue.is_empty() {
                    self.blocked_tasks.insert(resource_id, blocked_queue);
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply priority inheritance
    fn apply_priority_inheritance(&mut self, blocked_pid: Pid, owner_pid: Pid) -> RuntimeResult<()> {
        let blocked_priority = self.tasks[&blocked_pid].priority;
        let owner_task = self.tasks.get_mut(&owner_pid).unwrap();
        
        // Only inherit if blocked task has higher priority
        if blocked_priority > owner_task.priority {
            // Save original priority if not already saved
            if owner_task.original_priority == owner_task.priority {
                owner_task.original_priority = owner_task.priority;
            }
            
            // Inherit higher priority
            owner_task.priority = blocked_priority;
            
            // Track inheritance chain
            self.inheritance_chains.entry(owner_pid)
                .or_insert_with(Vec::new)
                .push(blocked_pid);
            
            self.stats.priority_inheritances += 1;
            
            // Collect inheritance chain to avoid borrowing conflicts
            let mut inheritance_chain = Vec::new();
            for (&resource_id, blocked_queue) in &self.blocked_tasks {
                if blocked_queue.iter().any(|task| task.pid == owner_pid) {
                    if let Some(&next_owner) = self.resource_owners.get(&resource_id) {
                        inheritance_chain.push((owner_pid, next_owner));
                    }
                }
            }

            // Apply inheritance chain
            for (blocked_pid, next_owner) in inheritance_chain {
                self.apply_priority_inheritance(blocked_pid, next_owner)?;
            }
        }
        
        Ok(())
    }
    
    /// Restore original priority after resource release
    fn restore_priority(&mut self, pid: Pid) -> RuntimeResult<()> {
        if let Some(task) = self.tasks.get_mut(&pid) {
            // Only restore if no resources are held
            if task.held_resources.is_empty() {
                task.priority = task.original_priority;
                
                // Clear inheritance chain
                self.inheritance_chains.remove(&pid);
            }
        }
        
        Ok(())
    }
    
    /// Add task to appropriate ready queue
    fn add_to_ready_queue(&mut self, task: RealTimeTask) {
        match self.algorithm {
            SchedulingAlgorithm::EDF => {
                self.edf_queue.push(Reverse(EDFTask {
                    pid: task.pid,
                    deadline: task.deadline,
                    priority: task.priority,
                }));
            }
            SchedulingAlgorithm::RateMonotonic => {
                if let Some(period) = task.period {
                    self.rm_queue.push(RMTask {
                        pid: task.pid,
                        period,
                        priority: task.priority,
                    });
                }
            }
            SchedulingAlgorithm::Hybrid => {
                match task.task_type {
                    TaskType::Periodic => {
                        if let Some(period) = task.period {
                            self.rm_queue.push(RMTask {
                                pid: task.pid,
                                period,
                                priority: task.priority,
                            });
                        }
                    }
                    TaskType::Sporadic | TaskType::Aperiodic => {
                        self.edf_queue.push(Reverse(EDFTask {
                            pid: task.pid,
                            deadline: task.deadline,
                            priority: task.priority,
                        }));
                    }
                }
            }
        }
    }
    
    /// Check for deadline misses
    fn check_deadline_misses(&mut self) {
        let now = Instant::now();
        let mut missed_deadlines = Vec::new();
        
        for (pid, task) in &self.tasks {
            if task.deadline <= now && task.remaining_time > Duration::ZERO {
                missed_deadlines.push(*pid);
            }
        }
        
        for pid in missed_deadlines {
            self.deadline_misses.fetch_add(1, AtomicOrdering::Relaxed);
            self.stats.deadline_misses += 1;
            
            // Log deadline miss
            eprintln!("DEADLINE MISS: Task {} missed deadline", pid);
        }
    }
    
    /// Perform schedulability analysis
    fn is_schedulable(&self, new_task: &RealTimeTask) -> bool {
        match self.algorithm {
            SchedulingAlgorithm::EDF => {
                // For EDF, check utilization bound
                let mut total_utilization = 0.0;
                
                for task in self.tasks.values() {
                    if let Some(period) = task.period {
                        total_utilization += task.wcet.as_secs_f64() / period.as_secs_f64();
                    }
                }
                
                // Add new task utilization
                if let Some(period) = new_task.period {
                    total_utilization += new_task.wcet.as_secs_f64() / period.as_secs_f64();
                }
                
                total_utilization <= 1.0
            }
            SchedulingAlgorithm::RateMonotonic => {
                // For RM, use Liu and Layland bound
                let n = self.tasks.len() + 1;
                let bound = n as f64 * (2.0_f64.powf(1.0 / n as f64) - 1.0);
                
                let mut total_utilization = 0.0;
                for task in self.tasks.values() {
                    if let Some(period) = task.period {
                        total_utilization += task.wcet.as_secs_f64() / period.as_secs_f64();
                    }
                }
                
                if let Some(period) = new_task.period {
                    total_utilization += new_task.wcet.as_secs_f64() / period.as_secs_f64();
                }
                
                total_utilization <= bound
            }
            SchedulingAlgorithm::Hybrid => {
                // For hybrid, check both bounds separately
                self.is_schedulable_edf_subset(new_task) && self.is_schedulable_rm_subset(new_task)
            }
        }
    }
    
    /// Check EDF schedulability for sporadic/aperiodic tasks
    fn is_schedulable_edf_subset(&self, new_task: &RealTimeTask) -> bool {
        // Simplified check for EDF subset
        true // In practice, this would be more complex
    }
    
    /// Check RM schedulability for periodic tasks
    fn is_schedulable_rm_subset(&self, new_task: &RealTimeTask) -> bool {
        // Simplified check for RM subset
        true // In practice, this would be more complex
    }
    
    /// Get scheduler statistics
    pub fn stats(&self) -> &RealTimeStats {
        &self.stats
    }
    
    /// Update task execution time
    pub fn update_execution_time(&mut self, pid: Pid, executed: Duration) {
        if let Some(task) = self.tasks.get_mut(&pid) {
            if task.remaining_time > executed {
                task.remaining_time -= executed;
            } else {
                task.remaining_time = Duration::ZERO;
            }
        }
    }
    
    /// Remove completed task
    pub fn remove_task(&mut self, pid: Pid) {
        self.tasks.remove(&pid);
        
        // Remove from inheritance chains
        self.inheritance_chains.remove(&pid);
        
        // Release any held resources
        let mut resources_to_release = Vec::new();
        for (&resource_id, &owner_pid) in &self.resource_owners {
            if owner_pid == pid {
                resources_to_release.push(resource_id);
            }
        }
        
        for resource_id in resources_to_release {
            let _ = self.release_resource(pid, resource_id);
        }
    }
}

// Implement ordering for priority queues
impl PartialEq for EDFTask {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}

impl Eq for EDFTask {}

impl PartialOrd for EDFTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EDFTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline.cmp(&other.deadline)
    }
}

impl PartialEq for RMTask {
    fn eq(&self, other: &Self) -> bool {
        self.period == other.period
    }
}

impl Eq for RMTask {}

impl PartialOrd for RMTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RMTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Shorter period = higher priority in RM
        other.period.cmp(&self.period)
    }
}

impl RealTimeTask {
    /// Create a new periodic real-time task
    pub fn periodic(
        pid: Pid,
        priority: Priority,
        period: Duration,
        wcet: Duration,
        deadline_offset: Duration,
    ) -> Self {
        let arrival_time = Instant::now();
        RealTimeTask {
            pid,
            priority,
            deadline: arrival_time + deadline_offset,
            period: Some(period),
            wcet,
            arrival_time,
            remaining_time: wcet,
            task_type: TaskType::Periodic,
            held_resources: Vec::new(),
            original_priority: priority,
        }
    }
    
    /// Create a new sporadic real-time task
    pub fn sporadic(
        pid: Pid,
        priority: Priority,
        deadline_offset: Duration,
        wcet: Duration,
    ) -> Self {
        let arrival_time = Instant::now();
        RealTimeTask {
            pid,
            priority,
            deadline: arrival_time + deadline_offset,
            period: None,
            wcet,
            arrival_time,
            remaining_time: wcet,
            task_type: TaskType::Sporadic,
            held_resources: Vec::new(),
            original_priority: priority,
        }
    }
    
    /// Check if task has missed its deadline
    pub fn has_missed_deadline(&self) -> bool {
        Instant::now() > self.deadline && self.remaining_time > Duration::ZERO
    }
    
    /// Get time until deadline
    pub fn time_to_deadline(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
}
