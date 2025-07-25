//! TLisp Actor System with Preemptive Scheduling
//!
//! This module integrates TLisp actors with the advanced scheduling systems
//! including work-stealing, real-time scheduling, and resource management.

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;

use crate::runtime::{
    WorkStealingScheduler, ScheduledTask, RealTimeScheduler, RealTimeTask,
    SchedulingAlgorithm, TaskType, ResourceManager, ResourceQuotas,
    PreemptionTimer, ProcessExecutor, Process, ProcessHandle, ReamActor
};
use crate::tlisp::{Expr, Value as TlispValue, Type, TlispInterpreter, EnhancedTlispCompiler};
use crate::bytecode::{BytecodeProgram, SecurityManager, BytecodeVerifier};
use crate::types::{Pid, Priority, MessagePayload};
use crate::error::{RuntimeError, RuntimeResult};

/// TLisp Actor with enhanced scheduling capabilities
#[derive(Clone)]
pub struct TlispActor {
    /// Actor ID
    id: String,
    /// TLisp interpreter instance (wrapped for thread safety)
    interpreter: Arc<Mutex<TlispInterpreter>>,
    /// Actor behavior (TLisp function)
    behavior: Expr<Type>,
    /// Actor state
    state: HashMap<String, TlispValue>,
    /// Message queue
    message_queue: Vec<TlispValue>,
    /// Actor priority
    priority: Priority,
    /// Real-time constraints (if any)
    rt_constraints: Option<RealTimeConstraints>,
    /// Resource quotas
    quotas: Option<ResourceQuotas>,
    /// Security level
    security_level: SecurityLevel,
}

/// Real-time constraints for TLisp actors
#[derive(Debug, Clone)]
pub struct RealTimeConstraints {
    /// Deadline for message processing
    pub deadline: Duration,
    /// Period for periodic actors
    pub period: Option<Duration>,
    /// Worst-case execution time
    pub wcet: Duration,
    /// Task type
    pub task_type: TaskType,
}

/// Security levels for TLisp actors
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    /// Full access (trusted code)
    Trusted,
    /// Limited access (restricted permissions)
    Restricted,
    /// Sandboxed (minimal permissions)
    Sandboxed,
}

/// Production-grade TLisp Actor System
pub struct TlispActorSystem {
    /// Work-stealing scheduler for general actors
    work_stealing_scheduler: Arc<Mutex<WorkStealingScheduler>>,
    /// Real-time scheduler for time-critical actors
    realtime_scheduler: Arc<Mutex<RealTimeScheduler>>,
    /// Resource manager
    resource_manager: Arc<ResourceManager>,
    /// Security manager
    security_manager: Arc<SecurityManager>,
    /// Bytecode verifier
    verifier: Arc<Mutex<BytecodeVerifier>>,
    /// Actor registry
    actors: Arc<Mutex<HashMap<Pid, TlispActor>>>,
    /// Process handles
    processes: Arc<Mutex<HashMap<Pid, ProcessHandle>>>,
    /// System configuration
    config: ActorSystemConfig,
}

/// Actor system configuration
#[derive(Debug, Clone)]
pub struct ActorSystemConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    /// Default quantum duration
    pub default_quantum: Duration,
    /// Default resource quotas
    pub default_quotas: ResourceQuotas,
    /// Enable real-time scheduling
    pub enable_realtime: bool,
    /// Enable security verification
    pub enable_security: bool,
    /// Maximum actors per system
    pub max_actors: usize,
}

impl Default for ActorSystemConfig {
    fn default() -> Self {
        ActorSystemConfig {
            worker_threads: num_cpus::get(),
            default_quantum: Duration::from_millis(10),
            default_quotas: ResourceQuotas::default(),
            enable_realtime: true,
            enable_security: true,
            max_actors: 10000,
        }
    }
}

impl TlispActor {
    /// Create a new TLisp actor
    pub fn new(
        id: String,
        behavior: Expr<Type>,
        priority: Priority,
        rt_constraints: Option<RealTimeConstraints>,
        quotas: Option<ResourceQuotas>,
        security_level: SecurityLevel,
    ) -> Self {
        TlispActor {
            id,
            interpreter: Arc::new(Mutex::new(TlispInterpreter::new())),
            behavior,
            state: HashMap::new(),
            message_queue: Vec::new(),
            priority,
            rt_constraints,
            quotas,
            security_level,
        }
    }
    
    /// Get actor ID
    pub fn id(&self) -> &str {
        &self.id
    }
    
    /// Get actor priority
    pub fn priority(&self) -> Priority {
        self.priority
    }
    
    /// Get real-time constraints
    pub fn rt_constraints(&self) -> Option<&RealTimeConstraints> {
        self.rt_constraints.as_ref()
    }
    
    /// Get resource quotas
    pub fn quotas(&self) -> Option<&ResourceQuotas> {
        self.quotas.as_ref()
    }
    
    /// Get security level
    pub fn security_level(&self) -> &SecurityLevel {
        &self.security_level
    }
    
    /// Add message to queue
    pub fn enqueue_message(&mut self, message: TlispValue) {
        self.message_queue.push(message);
    }
    
    /// Process next message
    pub fn process_message(&mut self) -> RuntimeResult<Option<TlispValue>> {
        if let Some(message) = self.message_queue.pop() {
            // Access interpreter through mutex
            let mut interpreter = self.interpreter.lock().unwrap();

            // Set up message in interpreter context
            interpreter.define("*message*".to_string(), message.clone());

            // Execute behavior with message
            match interpreter.evaluator.eval(&self.behavior) {
                Ok(result) => Ok(Some(result)),
                Err(e) => Err(RuntimeError::ActorError(format!("Actor {} failed to process message: {}", self.id, e))),
            }
        } else {
            Ok(None)
        }
    }
    
    /// Get current state
    pub fn get_state(&self) -> &HashMap<String, TlispValue> {
        &self.state
    }
    
    /// Update state
    pub fn update_state(&mut self, key: String, value: TlispValue) {
        self.state.insert(key, value);
    }
}

impl ReamActor for TlispActor {
    fn receive(&mut self, message: MessagePayload) -> RuntimeResult<()> {
        // Convert MessagePayload to TlispValue
        let tlisp_message = match message {
            MessagePayload::Text(text) => TlispValue::String(text),
            MessagePayload::Bytes(data) => TlispValue::String(String::from_utf8_lossy(&data).to_string()),
            MessagePayload::Data(json) => TlispValue::String(json.to_string()),
            MessagePayload::Control(_) => TlispValue::String("control".to_string()),
        };

        // Enqueue message
        self.enqueue_message(tlisp_message);

        // Process message
        self.process_message()?;

        Ok(())
    }

    fn pid(&self) -> Pid {
        // Return the actor's PID
        Pid::from_string(&self.id).unwrap_or_else(|_| Pid::new())
    }

    fn restart(&mut self) -> RuntimeResult<()> {
        // Clear mailbox and reset state
        self.message_queue.clear();
        Ok(())
    }

}

impl TlispActorSystem {
    /// Create a new TLisp actor system
    pub fn new(config: ActorSystemConfig) -> RuntimeResult<Self> {
        // Create schedulers
        let work_stealing_scheduler = Arc::new(Mutex::new(
            WorkStealingScheduler::new(Some(config.worker_threads))
        ));
        
        let realtime_scheduler = Arc::new(Mutex::new(
            RealTimeScheduler::new(SchedulingAlgorithm::Hybrid)
        ));
        
        // Create resource manager
        let resource_manager = Arc::new(ResourceManager::new(config.default_quotas.clone()));
        
        // Create security manager
        let security_manager = if config.enable_security {
            Arc::new(crate::bytecode::create_sandbox_manager())
        } else {
            Arc::new(SecurityManager::new())
        };
        
        // Create bytecode verifier
        let verifier = Arc::new(Mutex::new(BytecodeVerifier::new()));
        
        Ok(TlispActorSystem {
            work_stealing_scheduler,
            realtime_scheduler,
            resource_manager,
            security_manager,
            verifier,
            actors: Arc::new(Mutex::new(HashMap::new())),
            processes: Arc::new(Mutex::new(HashMap::new())),
            config,
        })
    }
    
    /// Start the actor system
    pub fn start(&self) -> RuntimeResult<()> {
        // Start work-stealing scheduler
        self.work_stealing_scheduler.lock().unwrap().start()?;
        
        Ok(())
    }
    
    /// Stop the actor system
    pub fn stop(&self) {
        // Stop work-stealing scheduler
        self.work_stealing_scheduler.lock().unwrap().stop();
    }
    
    /// Spawn a new TLisp actor
    pub fn spawn_actor(
        &self,
        id: String,
        behavior: Expr<Type>,
        priority: Priority,
        rt_constraints: Option<RealTimeConstraints>,
        quotas: Option<ResourceQuotas>,
        security_level: SecurityLevel,
    ) -> RuntimeResult<Pid> {
        // Check actor limit
        if self.actors.lock().unwrap().len() >= self.config.max_actors {
            return Err(RuntimeError::ActorError("Maximum number of actors reached".to_string()));
        }
        
        // Verify bytecode if security is enabled
        if self.config.enable_security {
            self.verify_actor_behavior(&behavior)?;
        }
        
        // Create actor
        let actor = TlispActor::new(
            id.clone(),
            behavior,
            priority,
            rt_constraints.clone(),
            quotas.clone(),
            security_level,
        );
        
        // Create process
        let pid = Pid::new();
        let process = Process::new(pid, Box::new(actor.clone()), priority);
        let handle = ProcessHandle::new(process);
        
        // Register with resource manager
        let effective_quotas = quotas.unwrap_or_else(|| self.config.default_quotas.clone());
        self.resource_manager.register_process(pid, Some(effective_quotas));
        
        // Register with appropriate scheduler
        if let Some(rt_constraints) = rt_constraints {
            // Register with real-time scheduler
            let rt_task = RealTimeTask::sporadic(
                pid,
                priority,
                rt_constraints.deadline,
                rt_constraints.wcet,
            );
            self.realtime_scheduler.lock().unwrap().add_task(rt_task)?;
        } else {
            // Register with work-stealing scheduler
            self.work_stealing_scheduler.lock().unwrap().register_process(handle.clone());
            
            let task = ScheduledTask::new(pid, priority);
            self.work_stealing_scheduler.lock().unwrap().schedule_task(task);
        }
        
        // Store actor and process
        self.actors.lock().unwrap().insert(pid, actor);
        self.processes.lock().unwrap().insert(pid, handle);
        
        Ok(pid)
    }
    
    /// Send message to actor
    pub fn send_message(&self, pid: Pid, message: TlispValue) -> RuntimeResult<()> {
        if let Some(actor) = self.actors.lock().unwrap().get_mut(&pid) {
            actor.enqueue_message(message);
            Ok(())
        } else {
            Err(RuntimeError::ActorError(format!("Actor {} not found", pid)))
        }
    }
    
    /// Kill an actor
    pub fn kill_actor(&self, pid: Pid) -> RuntimeResult<()> {
        // Remove from schedulers
        self.work_stealing_scheduler.lock().unwrap().unregister_process(pid);
        self.realtime_scheduler.lock().unwrap().remove_task(pid);
        
        // Remove from resource manager
        self.resource_manager.unregister_process(pid);
        
        // Remove from registries
        self.actors.lock().unwrap().remove(&pid);
        self.processes.lock().unwrap().remove(&pid);
        
        Ok(())
    }
    
    /// Get actor information
    pub fn get_actor_info(&self, pid: Pid) -> Option<ActorInfo> {
        self.actors.lock().unwrap().get(&pid).map(|actor| ActorInfo {
            id: actor.id().to_string(),
            priority: actor.priority(),
            rt_constraints: actor.rt_constraints().cloned(),
            quotas: actor.quotas().cloned(),
            security_level: actor.security_level().clone(),
            message_queue_size: actor.message_queue.len(),
            state_size: actor.state.len(),
        })
    }
    
    /// Get system statistics
    pub fn get_statistics(&self) -> ActorSystemStats {
        let actors = self.actors.lock().unwrap();
        let ws_scheduler = self.work_stealing_scheduler.lock().unwrap();
        let ws_stats = ws_scheduler.stats();
        let rt_scheduler = self.realtime_scheduler.lock().unwrap();
        let rt_stats = rt_scheduler.stats();
        let resource_stats = self.resource_manager.get_stats();
        
        ActorSystemStats {
            total_actors: actors.len(),
            active_actors: actors.values().filter(|a| !a.message_queue.is_empty()).count(),
            work_stealing_stats: ws_stats,
            realtime_stats: rt_stats.clone(),
            resource_stats,
            total_messages_queued: actors.values().map(|a| a.message_queue.len()).sum(),
        }
    }
    
    /// Verify actor behavior bytecode
    fn verify_actor_behavior(&self, behavior: &Expr<Type>) -> RuntimeResult<()> {
        // Compile behavior to bytecode
        let mut compiler = EnhancedTlispCompiler::new("actor_behavior".to_string());
        compiler.compile_expr(behavior).map_err(|e| RuntimeError::ActorError(format!("Failed to compile actor behavior: {}", e)))?;
        let program = compiler.finish().map_err(|e| RuntimeError::ActorError(format!("Failed to finish compilation: {}", e)))?;
        
        // Verify bytecode
        self.verifier.lock().unwrap().verify(&program)
            .map_err(|e| RuntimeError::ActorError(format!("Bytecode verification failed: {}", e)))?;
        
        Ok(())
    }
}

/// Actor information
#[derive(Debug, Clone)]
pub struct ActorInfo {
    pub id: String,
    pub priority: Priority,
    pub rt_constraints: Option<RealTimeConstraints>,
    pub quotas: Option<ResourceQuotas>,
    pub security_level: SecurityLevel,
    pub message_queue_size: usize,
    pub state_size: usize,
}

/// Actor system statistics
#[derive(Debug)]
pub struct ActorSystemStats {
    pub total_actors: usize,
    pub active_actors: usize,
    pub work_stealing_stats: crate::runtime::WorkStealingStats,
    pub realtime_stats: crate::runtime::RealTimeStats,
    pub resource_stats: crate::runtime::resource_manager::ResourceManagerStats,
    pub total_messages_queued: usize,
}

impl Drop for TlispActorSystem {
    fn drop(&mut self) {
        self.stop();
    }
}
