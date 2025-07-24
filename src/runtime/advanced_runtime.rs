//! Advanced REAM runtime with production-grade features
//!
//! Integrates fault tolerance, STM, infinite loop prevention, and other
//! production features into a cohesive runtime system.

use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::types::{Pid, ExecutionBounds, MemoryLayout};
use crate::error::{ReamError, ReamResult};
use crate::runtime::{
    ReamRuntime,
    actor::ReamActor,
    isolated_process::{IsolatedProcess, FaultHandler, DefaultFaultHandler, ProcessFault, RecoveryAction},
    stm_mailbox::{StmEngine, StmStats},
    bounded_execution::InfiniteLoopPrevention,
};

/// Advanced REAM runtime with production-grade features
pub struct AdvancedReamRuntime {
    /// Core REAM runtime
    core_runtime: ReamRuntime,
    
    /// STM engine for transactional memory
    stm_engine: Arc<StmEngine>,
    
    /// Infinite loop prevention system
    loop_prevention: Arc<InfiniteLoopPrevention>,
    
    /// Isolated processes
    isolated_processes: Arc<RwLock<HashMap<Pid, IsolatedProcess>>>,
    
    /// Fault handler
    fault_handler: Arc<dyn FaultHandler>,
    
    /// Default execution bounds
    default_bounds: ExecutionBounds,
    
    /// Default memory layout
    default_memory_layout: MemoryLayout,
    
    /// Runtime statistics
    stats: Arc<RwLock<AdvancedRuntimeStats>>,
}

/// Advanced runtime statistics
#[derive(Debug, Clone)]
pub struct AdvancedRuntimeStats {
    /// Total isolated processes
    pub isolated_processes: usize,
    /// Active bounded actors
    pub bounded_actors: usize,
    /// STM statistics
    pub stm_stats: StmStats,
    /// Fault recovery count
    pub fault_recoveries: u64,
    /// Divergence detections
    pub divergence_detections: u64,
}

impl AdvancedReamRuntime {
    /// Create a new advanced REAM runtime
    pub fn new() -> ReamResult<Self> {
        let core_runtime = ReamRuntime::new()?;
        let stm_engine = Arc::new(StmEngine::new());
        let loop_prevention = Arc::new(InfiniteLoopPrevention::default());
        let isolated_processes = Arc::new(RwLock::new(HashMap::new()));
        let fault_handler = Arc::new(DefaultFaultHandler);
        let default_bounds = ExecutionBounds::default();
        let default_memory_layout = MemoryLayout::new(64 * 1024, 1024 * 1024); // 64KB heap, 1MB stack
        
        let stats = Arc::new(RwLock::new(AdvancedRuntimeStats {
            isolated_processes: 0,
            bounded_actors: 0,
            stm_stats: stm_engine.get_stats(),
            fault_recoveries: 0,
            divergence_detections: 0,
        }));
        
        Ok(AdvancedReamRuntime {
            core_runtime,
            stm_engine,
            loop_prevention,
            isolated_processes,
            fault_handler,
            default_bounds,
            default_memory_layout,
            stats,
        })
    }
    
    /// Start the advanced runtime
    pub fn start(&self) -> ReamResult<()> {
        self.core_runtime.start()?;
        self.loop_prevention.start_monitoring();
        Ok(())
    }
    
    /// Stop the advanced runtime
    pub fn stop(&self) -> ReamResult<()> {
        self.loop_prevention.stop_monitoring();
        self.core_runtime.stop()?;
        Ok(())
    }
    
    /// Spawn an advanced actor with all production features
    pub fn spawn_advanced_actor<A>(&mut self, actor: A) -> ReamResult<Pid>
    where
        A: ReamActor + 'static,
    {
        // 1. Create isolated process
        let isolated = IsolatedProcess::new(
            Box::new(actor),
            self.default_memory_layout.clone(),
            self.default_bounds,
        ).map_err(|e| ReamError::Fault(e))?;
        
        let pid = isolated.pid();
        
        // 2. Register with STM engine
        self.stm_engine.create_mailbox(pid)
            .map_err(|e| ReamError::Stm(e))?;
        
        // 3. Register with divergence detector
        self.loop_prevention.detector.register_process(pid);
        
        // 4. Store isolated process
        {
            let mut processes = self.isolated_processes.write().unwrap();
            processes.insert(pid, isolated);
        }
        
        // 5. Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.isolated_processes += 1;
            stats.bounded_actors += 1;
        }
        
        Ok(pid)
    }
    
    /// Send a message using STM
    pub fn send_message(&self, from: Pid, to: Pid, message: Vec<u8>) -> ReamResult<u64> {
        self.stm_engine.stm_send_message(from, to, message)
            .map_err(|e| ReamError::Stm(e))
    }
    
    /// Receive messages using STM
    pub fn receive_messages(&self, pid: Pid, from_version: u64) -> ReamResult<Vec<crate::types::Versioned<Vec<u8>>>> {
        self.stm_engine.stm_receive_messages(pid, from_version)
            .map_err(|e| ReamError::Stm(e))
    }
    
    /// Process messages for a specific actor
    pub fn process_actor_messages(&mut self, pid: Pid) -> ReamResult<bool> {
        // First, try to process the message
        let process_result = {
            let mut processes = self.isolated_processes.write().unwrap();
            if let Some(process) = processes.get_mut(&pid) {
                process.process_message()
            } else {
                return Err(ReamError::Runtime(crate::error::RuntimeError::ProcessNotFound(pid)));
            }
        };

        // Handle the result without holding the lock
        match process_result {
            Ok(Some(_)) => {
                // Progress made, observe it
                if let Err(_) = self.loop_prevention.detector.observe_progress(pid) {
                    // Divergence detected
                    let mut stats = self.stats.write().unwrap();
                    stats.divergence_detections += 1;
                    drop(stats); // Release stats lock

                    // Handle divergence
                    self.handle_divergence(pid)?;
                }
                Ok(true)
            }
            Ok(None) => {
                // Actor terminated normally
                self.cleanup_actor(pid)?;
                Ok(false)
            }
            Err(fault_error) => {
                // Handle fault
                self.handle_fault(pid, fault_error)?;
                Ok(true)
            }
        }
    }
    
    /// Handle a fault in an actor
    fn handle_fault(&mut self, pid: Pid, fault_error: crate::error::FaultError) -> ReamResult<()> {
        let fault = match fault_error {
            crate::error::FaultError::InstructionLimitExceeded => ProcessFault::InstructionLimit,
            crate::error::FaultError::MemoryBoundaryExceeded => ProcessFault::OutOfMemory,
            crate::error::FaultError::MessageQuotaExceeded => ProcessFault::MessageOverflow,
            _ => ProcessFault::Panic(format!("{:?}", fault_error)),
        };
        
        let action = self.fault_handler.handle_fault(fault);
        
        match action {
            RecoveryAction::Restart => {
                self.restart_actor(pid)?;
                let mut stats = self.stats.write().unwrap();
                stats.fault_recoveries += 1;
            }
            RecoveryAction::Kill => {
                self.kill_actor(pid)?;
            }
            RecoveryAction::Suspend => {
                // For now, we'll treat suspend as kill
                self.kill_actor(pid)?;
            }
            _ => {
                // Other actions not implemented yet
                self.kill_actor(pid)?;
            }
        }
        
        Ok(())
    }
    
    /// Handle divergence detection
    fn handle_divergence(&mut self, pid: Pid) -> ReamResult<()> {
        // For now, restart the actor
        self.restart_actor(pid)
    }
    
    /// Restart an actor
    fn restart_actor(&mut self, pid: Pid) -> ReamResult<()> {
        let mut processes = self.isolated_processes.write().unwrap();
        
        if let Some(process) = processes.get_mut(&pid) {
            process.restart().map_err(|e| ReamError::Fault(e))?;
        }
        
        Ok(())
    }
    
    /// Kill an actor
    fn kill_actor(&mut self, pid: Pid) -> ReamResult<()> {
        {
            let processes = self.isolated_processes.read().unwrap();
            if let Some(process) = processes.get(&pid) {
                process.kill();
            }
        }
        
        self.cleanup_actor(pid)
    }
    
    /// Clean up an actor's resources
    fn cleanup_actor(&mut self, pid: Pid) -> ReamResult<()> {
        // Remove from isolated processes
        {
            let mut processes = self.isolated_processes.write().unwrap();
            processes.remove(&pid);
        }
        
        // Remove from STM engine
        self.stm_engine.remove_mailbox(pid)
            .map_err(|e| ReamError::Stm(e))?;
        
        // Remove from divergence detector
        self.loop_prevention.detector.unregister_process(pid);
        
        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.isolated_processes = stats.isolated_processes.saturating_sub(1);
            stats.bounded_actors = stats.bounded_actors.saturating_sub(1);
        }
        
        Ok(())
    }
    
    /// Check for divergent processes
    pub fn check_divergence(&self) -> Vec<Pid> {
        self.loop_prevention.check_divergence()
    }
    
    /// Get runtime statistics
    pub fn get_stats(&self) -> AdvancedRuntimeStats {
        let mut stats = self.stats.write().unwrap();
        stats.stm_stats = self.stm_engine.get_stats();
        stats.clone()
    }
    
    /// Compact STM mailboxes
    pub fn compact_mailboxes(&self, keep_versions: u64) -> ReamResult<usize> {
        let processes = self.isolated_processes.read().unwrap();
        let mut total_compacted = 0;
        
        for pid in processes.keys() {
            if let Ok(compacted) = self.stm_engine.compact_mailbox(*pid, keep_versions) {
                total_compacted += compacted;
            }
        }
        
        Ok(total_compacted)
    }
    
    /// Check if an actor is alive
    pub fn is_alive(&self, pid: Pid) -> bool {
        let processes = self.isolated_processes.read().unwrap();
        processes.get(&pid).map_or(false, |p| p.is_alive())
    }
    
    /// Get resource usage for an actor
    pub fn get_resource_usage(&self, pid: Pid) -> Option<(u64, u64, u64)> {
        let processes = self.isolated_processes.read().unwrap();
        processes.get(&pid).map(|p| p.get_resource_usage())
    }
}

impl Default for AdvancedReamRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default AdvancedReamRuntime")
    }
}
