//! REAM Runtime - Actor system with mathematical foundations
//!
//! The runtime is designed as a product category Actor × Scheduler × Memory × Message
//! with coalgebraic state machines and monadic composition.

pub mod actor;
pub mod scheduler;
pub mod memory;
pub mod message;
pub mod supervisor;
pub mod process;
pub mod isolated_process;
pub mod stm_mailbox;
pub mod bounded_execution;
pub mod advanced_runtime;
pub mod serverless;
pub mod serverless_runtime;


use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use dashmap::DashMap;

use crate::types::{Pid, Priority, ProcessInfo, RuntimeStats, ReamConfig, MessagePayload};
use crate::error::{RuntimeError, RuntimeResult};
use crate::daemon::monitor::ActorMonitor;

pub use actor::{Actor, ReamActor, ActorContext};
pub use scheduler::{Scheduler, SchedulingOp};
pub use memory::{GarbageCollector, MemoryManager};
pub use message::{MessageRouter, Mailbox};
pub use supervisor::{Supervisor, ProcessTree};
pub use process::{Process, ProcessHandle};

/// Runtime configuration for macros
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of actors
    pub max_actors: usize,
    /// Number of worker threads
    pub worker_threads: usize,
    /// Garbage collection interval
    pub gc_interval: std::time::Duration,
    /// Enable distributed mode
    pub distributed: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_actors: 1_000_000,
            worker_threads: num_cpus::get(),
            gc_interval: std::time::Duration::from_secs(1),
            distributed: false,
        }
    }
}

/// Main REAM runtime - the categorical composition of all subsystems
pub struct ReamRuntime {
    /// Configuration
    config: ReamConfig,
    
    /// Process table - maps PIDs to process handles
    processes: Arc<DashMap<Pid, ProcessHandle>>,
    
    /// Scheduler for process execution
    scheduler: Arc<Mutex<Scheduler>>,
    
    /// Memory manager with garbage collection
    memory: Arc<Mutex<MemoryManager>>,
    
    /// Message router for inter-process communication
    message_router: Arc<MessageRouter>,
    
    /// Root supervisor
    root_supervisor: Arc<Mutex<Supervisor>>,
    
    /// Runtime statistics
    stats: Arc<RwLock<RuntimeStats>>,
    
    /// Shutdown signal
    shutdown_tx: Sender<()>,
    shutdown_rx: Receiver<()>,
    
    /// Runtime start time
    start_time: Instant,

    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,

    /// Hypervisor monitor for actor monitoring
    hypervisor: Option<Arc<ActorMonitor>>,
}

impl ReamRuntime {
    /// Create a new REAM runtime with default configuration
    pub fn new() -> RuntimeResult<Self> {
        Ok(Self::with_config(ReamConfig::default()))
    }
    
    /// Create a new REAM runtime with custom configuration
    pub fn with_config(config: ReamConfig) -> Self {
        let (shutdown_tx, shutdown_rx) = unbounded();
        
        let runtime = ReamRuntime {
            config,
            processes: Arc::new(DashMap::new()),
            scheduler: Arc::new(Mutex::new(Scheduler::new())),
            memory: Arc::new(Mutex::new(MemoryManager::new())),
            message_router: Arc::new(MessageRouter::new()),
            root_supervisor: Arc::new(Mutex::new(Supervisor::new(
                crate::types::RestartStrategy::OneForOne
            ))),
            stats: Arc::new(RwLock::new(RuntimeStats {
                process_count: 0,
                running_processes: 0,
                memory_usage: 0,
                message_rate: 0.0,
                scheduler_utilization: 0.0,
                gc_collections: 0,
            })),
            shutdown_tx,
            shutdown_rx,
            start_time: Instant::now(),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            hypervisor: None,
        };
        
        runtime
    }
    
    /// Start the runtime
    pub fn start(&self) -> RuntimeResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // Start scheduler thread
        self.start_scheduler()?;
        
        // Start message router
        self.message_router.start()?;
        
        // Start garbage collector
        self.start_gc()?;
        
        // Start statistics collector
        self.start_stats_collector()?;
        
        Ok(())
    }

    /// Get the current runtime instance (placeholder for macro compatibility)
    pub fn current() -> Self {
        // In a real implementation, this would use thread-local storage
        // For now, return a default runtime
        Self::with_config(ReamConfig::default())
    }

    /// Set the current runtime instance (placeholder for macro compatibility)
    pub fn set_current(_runtime: Self) {
        // In a real implementation, this would set thread-local storage
        // For now, this is a no-op
    }

    /// Spawn an actor (placeholder for macro compatibility)
    pub fn spawn_actor<F>(&self, _actor_fn: F) -> RuntimeResult<Pid>
    where
        F: FnOnce() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        // For now, just return a new PID
        Ok(Pid::new())
    }

    /// Send a message to an actor (placeholder for macro compatibility)
    pub fn send_message(&self, _pid: Pid, _message: MessagePayload) -> RuntimeResult<()> {
        // Placeholder implementation
        Ok(())
    }

    /// Ask pattern for request-response (placeholder for macro compatibility)
    pub async fn ask_actor<T>(&self, _pid: Pid, _message: MessagePayload) -> RuntimeResult<T>
    where
        T: Default,
    {
        // Placeholder implementation
        Ok(T::default())
    }

    /// Spawn a task on the runtime (placeholder for macro compatibility)
    pub fn spawn_task<F>(&self, _future: F) -> RuntimeResult<tokio::task::JoinHandle<()>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        Ok(tokio::spawn(_future))
    }

    /// Block on a future (placeholder for macro compatibility)
    pub fn block_on<F>(&self, _future: F) -> RuntimeResult<F::Output>
    where
        F: std::future::Future,
    {
        // This would need a proper async runtime in a real implementation
        // For now, return an error
        Err(RuntimeError::RuntimeError("block_on not implemented".to_string()))
    }
    
    /// Stop the runtime
    pub fn stop(&self) -> RuntimeResult<()> {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);

        // Send shutdown signal
        self.shutdown_tx.send(()).map_err(|_| {
            RuntimeError::Scheduler("Failed to send shutdown signal".to_string())
        })?;

        // Wait for all processes to terminate
        self.terminate_all_processes()?;

        Ok(())
    }

    /// Shutdown the runtime (alias for stop)
    pub fn shutdown(&self) -> RuntimeResult<()> {
        self.stop()
    }

    /// List all processes with handles in the runtime
    pub fn list_process_handles(&self) -> Vec<(Pid, ProcessHandle)> {
        self.processes
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Get a process handle by PID
    pub fn get_process(&self, pid: Pid) -> Option<ProcessHandle> {
        self.processes.get(&pid).map(|entry| entry.value().clone())
    }
    
    /// Check if runtime is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }
    
    /// Spawn a new process with the given actor
    pub fn spawn<A>(&self, actor: A) -> RuntimeResult<Pid>
    where
        A: ReamActor + Send + Sync + 'static,
    {
        if self.processes.len() >= self.config.max_processes {
            return Err(RuntimeError::MaxProcesses(self.config.max_processes));
        }
        
        let pid = Pid::new();
        let process = Process::new(pid, Box::new(actor), Priority::Normal);
        let handle = ProcessHandle::new(process);
        
        // Add to process table
        self.processes.insert(pid, handle.clone());
        
        // Schedule the process
        self.scheduler.lock().schedule(pid, Priority::Normal)?;
        
        // Add to root supervisor
        self.root_supervisor.lock().supervise(pid, handle)?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.process_count += 1;
            stats.running_processes += 1;
        }
        
        Ok(pid)
    }
    
    /// Send a message to a process
    pub fn send(&self, to: Pid, payload: crate::types::MessagePayload) -> RuntimeResult<()> {
        self.message_router.send_message(to, payload)
    }
    
    /// Get process information
    pub fn process_info(&self, pid: Pid) -> RuntimeResult<ProcessInfo> {
        let handle = self.processes.get(&pid)
            .ok_or(RuntimeError::ProcessNotFound(pid))?;
        
        Ok(handle.info())
    }
    
    /// Get runtime statistics
    pub fn stats(&self) -> RuntimeStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Get all process PIDs
    pub fn list_processes(&self) -> Vec<Pid> {
        self.processes.iter().map(|entry| *entry.key()).collect()
    }

    /// Get the current process count
    pub fn process_count(&self) -> usize {
        self.stats.read().unwrap().process_count
    }
    
    /// Terminate a specific process
    pub fn terminate_process(&self, pid: Pid) -> RuntimeResult<()> {
        if let Some((_, handle)) = self.processes.remove(&pid) {
            handle.terminate()?;
            
            // Update statistics
            let mut stats = self.stats.write().unwrap();
            stats.process_count -= 1;
            if stats.running_processes > 0 {
                stats.running_processes -= 1;
            }
        }
        
        Ok(())
    }
    
    /// Get runtime uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    // Private helper methods
    
    fn start_scheduler(&self) -> RuntimeResult<()> {
        let scheduler = Arc::clone(&self.scheduler);
        let processes = Arc::clone(&self.processes);
        let running = Arc::clone(&self.running);
        let shutdown_rx = self.shutdown_rx.clone();
        
        std::thread::spawn(move || {
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }
                
                // Get next process to run
                if let Some(pid) = scheduler.lock().next_process() {
                    if let Some(handle) = processes.get(&pid) {
                        // Execute process quantum
                        let _ = handle.run_quantum();
                    }
                }
                
                // Small yield to prevent busy waiting
                std::thread::sleep(Duration::from_micros(10));
            }
        });
        
        Ok(())
    }
    
    fn start_gc(&self) -> RuntimeResult<()> {
        let memory = Arc::clone(&self.memory);
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let gc_threshold = self.config.gc_threshold;
        
        std::thread::spawn(move || {
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                {
                    let mut mem = memory.lock();
                    if mem.total_allocated() > gc_threshold {
                        mem.collect();
                        
                        // Update GC stats
                        let mut s = stats.write().unwrap();
                        s.gc_collections += 1;
                    }
                }
                
                // GC runs every 100ms
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        
        Ok(())
    }
    
    fn start_stats_collector(&self) -> RuntimeResult<()> {
        let stats = Arc::clone(&self.stats);
        let processes = Arc::clone(&self.processes);
        let running = Arc::clone(&self.running);
        
        std::thread::spawn(move || {
            let mut last_message_count = 0u64;
            let mut last_time = Instant::now();
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                let now = Instant::now();
                let elapsed = now.duration_since(last_time).as_secs_f64();
                
                {
                    let mut s = stats.write().unwrap();
                    s.process_count = processes.len();
                    s.running_processes = processes.iter()
                        .filter(|entry| entry.value().is_running())
                        .count();
                    
                    // Calculate message rate
                    let current_message_count = 0u64; // TODO: Get from message router
                    s.message_rate = (current_message_count - last_message_count) as f64 / elapsed;
                    last_message_count = current_message_count;
                }
                
                last_time = now;
                std::thread::sleep(Duration::from_secs(1));
            }
        });
        
        Ok(())
    }
    
    fn terminate_all_processes(&self) -> RuntimeResult<()> {
        let pids: Vec<Pid> = self.processes.iter().map(|entry| *entry.key()).collect();
        
        for pid in pids {
            let _ = self.terminate_process(pid);
        }
        
        Ok(())
    }

    // ========================================
    // HYPERVISOR MONITORING METHODS
    // ========================================

    /// Start hypervisor monitoring
    pub fn start_monitoring(&mut self) -> RuntimeResult<()> {
        if self.hypervisor.is_none() {
            let monitor = ActorMonitor::new(
                Duration::from_secs(1),  // collection_interval
            );
            self.hypervisor = Some(Arc::new(monitor));
        }

        // Start monitoring in background
        if let Some(ref monitor) = self.hypervisor {
            // In a real implementation, this would start background monitoring tasks
            // For now, we just confirm the monitor is ready
        }

        Ok(())
    }

    /// Stop hypervisor monitoring
    pub fn stop_monitoring(&mut self) -> RuntimeResult<()> {
        self.hypervisor = None;
        Ok(())
    }

    /// Get hypervisor monitor reference
    pub fn get_monitor(&self) -> Option<&Arc<ActorMonitor>> {
        self.hypervisor.as_ref()
    }

    /// Register an actor for monitoring
    pub fn register_actor_for_monitoring(&self, pid: Pid) -> RuntimeResult<()> {
        if let Some(ref monitor) = self.hypervisor {
            // Register actor with monitor
            // In a real implementation, this would add the actor to monitoring
            Ok(())
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// Unregister an actor from monitoring
    pub fn unregister_actor_from_monitoring(&self, pid: Pid) -> RuntimeResult<()> {
        if let Some(ref monitor) = self.hypervisor {
            // Unregister actor from monitor
            // In a real implementation, this would remove the actor from monitoring
            Ok(())
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// Get actor metrics (simplified for demo)
    pub fn get_actor_metrics(&self, pid: Pid) -> RuntimeResult<(u64, usize, bool)> {
        if let Some(ref _monitor) = self.hypervisor {
            // Get real metrics from the process
            if let Some(handle) = self.processes.get(&pid) {
                let process = handle.value();
                Ok((
                    524288, // memory_usage: 512KB - would be real memory usage
                    process.mailbox().read().unwrap().len(), // message_queue_length from actual mailbox
                    process.is_running(), // status from actual process
                ))
            } else {
                Err(RuntimeError::ProcessNotFound(pid))
            }
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// Get system metrics (simplified for demo)
    pub fn get_system_metrics(&self) -> RuntimeResult<(usize, usize, u64, f64, u64)> {
        if let Some(ref _monitor) = self.hypervisor {
            let stats = self.stats.read().unwrap();
            Ok((
                stats.process_count,        // total_actors
                stats.running_processes,    // active_actors
                stats.memory_usage as u64,         // total_memory_usage
                stats.message_rate,         // message_throughput
                self.start_time.elapsed().as_secs(), // uptime
            ))
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// List all monitored actors
    pub fn list_monitored_actors(&self) -> RuntimeResult<Vec<Pid>> {
        if let Some(ref _monitor) = self.hypervisor {
            Ok(self.processes.iter().map(|entry| *entry.key()).collect())
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// Perform health check on all actors
    pub fn perform_health_check(&self) -> RuntimeResult<HashMap<String, usize>> {
        if let Some(ref _monitor) = self.hypervisor {
            let mut healthy = 0;
            let mut unhealthy = 0;
            let mut unresponsive = 0;

            for entry in self.processes.iter() {
                let process = entry.value();
                if process.is_running() {
                    healthy += 1;
                } else {
                    unresponsive += 1;
                }
            }

            let mut results = HashMap::new();
            results.insert("healthy".to_string(), healthy);
            results.insert("unhealthy".to_string(), unhealthy);
            results.insert("unresponsive".to_string(), unresponsive);
            Ok(results)
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// Restart an actor
    pub fn restart_actor(&self, pid: Pid) -> RuntimeResult<()> {
        if let Some(ref _monitor) = self.hypervisor {
            // Use the supervisor to handle the restart
            self.root_supervisor.lock().handle_child_failure(pid)?;
            Ok(())
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// Suspend an actor
    pub fn suspend_actor(&self, pid: Pid) -> RuntimeResult<()> {
        if let Some(ref _monitor) = self.hypervisor {
            if let Some(handle) = self.processes.get(&pid) {
                handle.value().suspend();
                Ok(())
            } else {
                Err(RuntimeError::ProcessNotFound(pid))
            }
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }

    /// Resume an actor
    pub fn resume_actor(&self, pid: Pid) -> RuntimeResult<()> {
        if let Some(ref _monitor) = self.hypervisor {
            if let Some(handle) = self.processes.get(&pid) {
                handle.value().resume();
                Ok(())
            } else {
                Err(RuntimeError::ProcessNotFound(pid))
            }
        } else {
            Err(RuntimeError::InvalidMessage("Hypervisor not started".to_string()))
        }
    }
}

impl Default for ReamRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default ReamRuntime")
    }
}

impl Drop for ReamRuntime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
