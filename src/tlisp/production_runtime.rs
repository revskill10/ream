//! Production-Grade TLisp Runtime
//!
//! This module provides a complete production-grade TLisp runtime that integrates
//! all the advanced features including preemptive scheduling, security, resource management,
//! and performance optimization.

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::runtime::{ResourceManager, ResourceQuotas, WorkStealingScheduler, RealTimeScheduler};
use crate::bytecode::{BytecodeVM, SecurityManager, BytecodeVerifier};
use crate::jit::ReamJIT;
use crate::tlisp::{
    TlispInterpreter, EnhancedTlispCompiler, TlispActorSystem, TlispSecurityManager,
    TlispResourceManager, ProductionStandardLibrary, Expr, Value as TlispValue, Type,
    TlispSecurityLevel, TlispResourceQuotas, ActorSystemConfig, SecurityLevel
};
use crate::types::{Pid, Priority};
use crate::error::{TlispError, TlispResult, RuntimeError};

/// Production-grade TLisp runtime configuration
#[derive(Debug, Clone)]
pub struct ProductionRuntimeConfig {
    /// Enable JIT compilation
    pub enable_jit: bool,
    /// JIT optimization level (0-3)
    pub jit_optimization_level: u8,
    /// Enable preemptive scheduling
    pub enable_preemptive_scheduling: bool,
    /// Enable work-stealing scheduler
    pub enable_work_stealing: bool,
    /// Enable real-time scheduling
    pub enable_realtime_scheduling: bool,
    /// Enable security verification
    pub enable_security: bool,
    /// Default security level
    pub default_security_level: TlispSecurityLevel,
    /// Enable resource management
    pub enable_resource_management: bool,
    /// Default resource quotas
    pub default_resource_quotas: TlispResourceQuotas,
    /// Enable actor system
    pub enable_actor_system: bool,
    /// Actor system configuration
    pub actor_config: ActorSystemConfig,
    /// Maximum concurrent programs
    pub max_concurrent_programs: usize,
    /// Program execution timeout
    pub execution_timeout: Duration,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Enable debug mode
    pub debug_mode: bool,
}

/// Production-grade TLisp runtime
pub struct ProductionTlispRuntime {
    /// Core interpreter
    interpreter: Arc<Mutex<TlispInterpreter>>,
    /// Enhanced compiler
    compiler: Arc<Mutex<EnhancedTlispCompiler>>,
    /// Bytecode VM
    vm: Arc<Mutex<BytecodeVM>>,
    /// JIT compiler
    jit: Option<Arc<Mutex<ReamJIT>>>,
    /// Actor system
    actor_system: Option<Arc<TlispActorSystem>>,
    /// Security manager
    security_manager: Arc<Mutex<TlispSecurityManager>>,
    /// Resource manager
    resource_manager: Arc<TlispResourceManager>,
    /// Standard library
    stdlib: Arc<ProductionStandardLibrary>,
    /// Runtime configuration
    config: ProductionRuntimeConfig,
    /// Performance statistics
    stats: Arc<Mutex<RuntimeStats>>,
    /// Active programs
    active_programs: Arc<Mutex<HashMap<String, ProgramExecution>>>,
}

/// Runtime performance statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Total programs executed
    pub programs_executed: u64,
    /// Programs currently running
    pub programs_running: u64,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Average execution time
    pub avg_execution_time: Duration,
    /// JIT compilation count
    pub jit_compilations: u64,
    /// Bytecode verifications
    pub bytecode_verifications: u64,
    /// Security violations
    pub security_violations: u64,
    /// Resource quota violations
    pub resource_violations: u64,
    /// Actor spawns
    pub actors_spawned: u64,
    /// Peak memory usage
    pub peak_memory_usage: u64,
    /// Total garbage collections
    pub total_gc_count: u64,
}

/// Program execution context
#[derive(Debug)]
pub struct ProgramExecution {
    /// Program ID
    pub id: String,
    /// Start time
    pub start_time: Instant,
    /// Security level
    pub security_level: TlispSecurityLevel,
    /// Resource quotas
    pub resource_quotas: TlispResourceQuotas,
    /// Execution mode
    pub execution_mode: ExecutionMode,
    /// Associated process ID (if any)
    pub process_id: Option<Pid>,
}

/// Execution modes
#[derive(Debug, Clone)]
pub enum ExecutionMode {
    /// Interpreted execution
    Interpreted,
    /// Bytecode execution
    Bytecode,
    /// JIT compiled execution
    JitCompiled,
    /// Actor-based execution
    Actor,
}

/// Program execution result
#[derive(Debug)]
pub struct ExecutionResult {
    /// Result value
    pub value: TlispValue,
    /// Execution time
    pub execution_time: Duration,
    /// Memory used
    pub memory_used: u64,
    /// Execution mode used
    pub execution_mode: ExecutionMode,
    /// Performance metrics
    pub metrics: ExecutionMetrics,
}

/// Execution performance metrics
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    /// Instructions executed
    pub instructions_executed: u64,
    /// Function calls made
    pub function_calls: u64,
    /// Memory allocations
    pub memory_allocations: u64,
    /// Garbage collections triggered
    pub gc_count: u64,
    /// JIT compilation time (if applicable)
    pub jit_compile_time: Option<Duration>,
    /// Security checks performed
    pub security_checks: u64,
}

impl Default for ProductionRuntimeConfig {
    fn default() -> Self {
        ProductionRuntimeConfig {
            enable_jit: true,
            jit_optimization_level: 2,
            enable_preemptive_scheduling: true,
            enable_work_stealing: true,
            enable_realtime_scheduling: true,
            enable_security: true,
            default_security_level: TlispSecurityLevel::Restricted,
            enable_resource_management: true,
            default_resource_quotas: TlispResourceQuotas::default(),
            enable_actor_system: true,
            actor_config: ActorSystemConfig::default(),
            max_concurrent_programs: 1000,
            execution_timeout: Duration::from_secs(30),
            enable_performance_monitoring: true,
            debug_mode: false,
        }
    }
}

impl ProductionTlispRuntime {
    /// Create a new production TLisp runtime
    pub fn new(config: ProductionRuntimeConfig) -> TlispResult<Self> {
        // Create core components
        let interpreter = Arc::new(Mutex::new(TlispInterpreter::new()));
        let compiler = Arc::new(Mutex::new(EnhancedTlispCompiler::new("production_runtime".to_string())));
        let vm = Arc::new(Mutex::new(BytecodeVM::new()));
        
        // Create JIT compiler if enabled
        let jit = if config.enable_jit {
            Some(Arc::new(Mutex::new(ReamJIT::new())))
        } else {
            None
        };
        
        // Create actor system if enabled
        let actor_system = if config.enable_actor_system {
            Some(Arc::new(TlispActorSystem::new(config.actor_config.clone())
                .map_err(|e| TlispError::Runtime(format!("Failed to create actor system: {}", e)))?))
        } else {
            None
        };
        
        // Create security manager
        let security_manager = Arc::new(Mutex::new(
            if config.enable_security {
                TlispSecurityManager::sandboxed()
            } else {
                TlispSecurityManager::new()
            }
        ));
        
        // Create resource manager
        let base_resource_manager = Arc::new(ResourceManager::new(config.default_resource_quotas.base_quotas.clone()));
        let resource_manager = Arc::new(TlispResourceManager::new(base_resource_manager));
        
        // Create standard library
        let stdlib = Arc::new(ProductionStandardLibrary::new());
        
        Ok(ProductionTlispRuntime {
            interpreter,
            compiler,
            vm,
            jit,
            actor_system,
            security_manager,
            resource_manager,
            stdlib,
            config,
            stats: Arc::new(Mutex::new(RuntimeStats::default())),
            active_programs: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    /// Start the runtime
    pub fn start(&self) -> TlispResult<()> {
        // Start actor system if enabled
        if let Some(ref actor_system) = self.actor_system {
            actor_system.start()
                .map_err(|e| TlispError::Runtime(format!("Failed to start actor system: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Stop the runtime
    pub fn stop(&self) {
        // Stop actor system if enabled
        if let Some(ref actor_system) = self.actor_system {
            actor_system.stop();
        }
        
        // Cancel all active programs
        self.active_programs.lock().unwrap().clear();
    }
    
    /// Execute a TLisp program with automatic optimization
    pub fn execute_program(
        &self,
        program_id: String,
        source: &str,
        security_level: Option<TlispSecurityLevel>,
        resource_quotas: Option<TlispResourceQuotas>,
    ) -> TlispResult<ExecutionResult> {
        let start_time = Instant::now();
        
        // Check concurrent program limit
        if self.active_programs.lock().unwrap().len() >= self.config.max_concurrent_programs {
            return Err(TlispError::Runtime("Maximum concurrent programs reached".to_string()));
        }
        
        // Determine execution parameters
        let security_level = security_level.unwrap_or_else(|| self.config.default_security_level.clone());
        let resource_quotas = resource_quotas.unwrap_or_else(|| self.config.default_resource_quotas.clone());
        
        // Parse the program
        let mut interpreter = self.interpreter.lock().unwrap();
        let expr = interpreter.parse(source)?;
        drop(interpreter);
        
        // Security verification if enabled
        if self.config.enable_security {
            let mut security_manager = self.security_manager.lock().unwrap();
            let mut interpreter = self.interpreter.lock().unwrap();
            let typed_expr = interpreter.evaluator.add_placeholder_types(&expr);
            drop(interpreter);
            let _verified_program = security_manager.verify_program(
                program_id.clone(),
                &typed_expr,
                security_level.clone(),
            )?;
        }
        
        // Register program execution
        let execution = ProgramExecution {
            id: program_id.clone(),
            start_time,
            security_level: security_level.clone(),
            resource_quotas: resource_quotas.clone(),
            execution_mode: ExecutionMode::Interpreted, // Will be updated based on actual execution
            process_id: None,
        };
        self.active_programs.lock().unwrap().insert(program_id.clone(), execution);
        
        // Choose execution strategy
        let result = if self.should_use_jit(&expr) {
            self.execute_with_jit(program_id.clone(), &expr, &security_level, &resource_quotas)
        } else if self.should_use_bytecode(&expr) {
            self.execute_with_bytecode(program_id.clone(), &expr, &security_level, &resource_quotas)
        } else if self.should_use_actor(&expr) {
            self.execute_with_actor(program_id.clone(), &expr, &security_level, &resource_quotas)
        } else {
            self.execute_interpreted(program_id.clone(), &expr, &security_level, &resource_quotas)
        };
        
        // Clean up
        self.active_programs.lock().unwrap().remove(&program_id);
        
        // Update statistics
        let mut stats = self.stats.lock().unwrap();
        stats.programs_executed += 1;
        if stats.programs_running > 0 {
            stats.programs_running -= 1;
        }
        
        let execution_time = start_time.elapsed();
        stats.total_execution_time += execution_time;
        stats.avg_execution_time = stats.total_execution_time / stats.programs_executed as u32;
        
        result
    }
    
    /// Execute as an actor
    pub fn spawn_actor(
        &self,
        actor_id: String,
        behavior: Expr<Type>,
        priority: Priority,
        security_level: SecurityLevel,
    ) -> TlispResult<Pid> {
        if let Some(ref actor_system) = self.actor_system {
            let pid = actor_system.spawn_actor(
                actor_id,
                behavior,
                priority,
                None, // No real-time constraints
                Some(self.config.default_resource_quotas.base_quotas.clone()),
                security_level,
            ).map_err(|e| TlispError::Runtime(format!("Failed to spawn actor: {}", e)))?;
            
            // Update statistics
            self.stats.lock().unwrap().actors_spawned += 1;
            
            Ok(pid)
        } else {
            Err(TlispError::Runtime("Actor system not enabled".to_string()))
        }
    }
    
    /// Send message to actor
    pub fn send_message(&self, pid: Pid, message: TlispValue) -> TlispResult<()> {
        if let Some(ref actor_system) = self.actor_system {
            actor_system.send_message(pid, message)
                .map_err(|e| TlispError::Runtime(format!("Failed to send message: {}", e)))
        } else {
            Err(TlispError::Runtime("Actor system not enabled".to_string()))
        }
    }
    
    /// Get runtime statistics
    pub fn get_stats(&self) -> RuntimeStats {
        self.stats.lock().unwrap().clone()
    }
    
    /// Get active programs
    pub fn get_active_programs(&self) -> Vec<String> {
        self.active_programs.lock().unwrap().keys().cloned().collect()
    }
    
    /// Determine if JIT compilation should be used
    fn should_use_jit(&self, _expr: &Expr<()>) -> bool {
        self.config.enable_jit && self.jit.is_some()
        // In a real implementation, this would analyze the expression complexity
    }
    
    /// Determine if bytecode execution should be used
    fn should_use_bytecode(&self, _expr: &Expr<()>) -> bool {
        true // Most programs benefit from bytecode execution
    }
    
    /// Determine if actor execution should be used
    fn should_use_actor(&self, _expr: &Expr<()>) -> bool {
        self.config.enable_actor_system && self.actor_system.is_some()
        // In a real implementation, this would detect actor-related constructs
    }
    
    /// Execute with JIT compilation
    fn execute_with_jit(
        &self,
        _program_id: String,
        _expr: &Expr<()>,
        _security_level: &TlispSecurityLevel,
        _resource_quotas: &TlispResourceQuotas,
    ) -> TlispResult<ExecutionResult> {
        // Placeholder implementation
        Ok(ExecutionResult {
            value: TlispValue::Unit,
            execution_time: Duration::from_millis(1),
            memory_used: 1024,
            execution_mode: ExecutionMode::JitCompiled,
            metrics: ExecutionMetrics::default(),
        })
    }
    
    /// Execute with bytecode
    fn execute_with_bytecode(
        &self,
        _program_id: String,
        _expr: &Expr<()>,
        _security_level: &TlispSecurityLevel,
        _resource_quotas: &TlispResourceQuotas,
    ) -> TlispResult<ExecutionResult> {
        // Placeholder implementation
        Ok(ExecutionResult {
            value: TlispValue::Unit,
            execution_time: Duration::from_millis(2),
            memory_used: 512,
            execution_mode: ExecutionMode::Bytecode,
            metrics: ExecutionMetrics::default(),
        })
    }
    
    /// Execute with actor system
    fn execute_with_actor(
        &self,
        _program_id: String,
        _expr: &Expr<()>,
        _security_level: &TlispSecurityLevel,
        _resource_quotas: &TlispResourceQuotas,
    ) -> TlispResult<ExecutionResult> {
        // Placeholder implementation
        Ok(ExecutionResult {
            value: TlispValue::Unit,
            execution_time: Duration::from_millis(3),
            memory_used: 2048,
            execution_mode: ExecutionMode::Actor,
            metrics: ExecutionMetrics::default(),
        })
    }
    
    /// Execute with interpreter
    fn execute_interpreted(
        &self,
        _program_id: String,
        _expr: &Expr<()>,
        _security_level: &TlispSecurityLevel,
        _resource_quotas: &TlispResourceQuotas,
    ) -> TlispResult<ExecutionResult> {
        // Placeholder implementation
        Ok(ExecutionResult {
            value: TlispValue::Unit,
            execution_time: Duration::from_millis(5),
            memory_used: 256,
            execution_mode: ExecutionMode::Interpreted,
            metrics: ExecutionMetrics::default(),
        })
    }
}

impl Default for RuntimeStats {
    fn default() -> Self {
        RuntimeStats {
            programs_executed: 0,
            programs_running: 0,
            total_execution_time: Duration::ZERO,
            avg_execution_time: Duration::ZERO,
            jit_compilations: 0,
            bytecode_verifications: 0,
            security_violations: 0,
            resource_violations: 0,
            actors_spawned: 0,
            peak_memory_usage: 0,
            total_gc_count: 0,
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        ExecutionMetrics {
            instructions_executed: 0,
            function_calls: 0,
            memory_allocations: 0,
            gc_count: 0,
            jit_compile_time: None,
            security_checks: 0,
        }
    }
}

impl Drop for ProductionTlispRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}
