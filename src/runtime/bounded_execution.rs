//! Bounded execution system for infinite loop prevention
//!
//! Implements mathematical divergence detection, fuel systems, and bounded
//! execution to prevent infinite loops and ensure system responsiveness.

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::types::{Pid, ExecutionBounds};
use crate::error::{FaultError, FaultResult};
use crate::runtime::actor::ReamActor;

/// Fuel system for resource-bounded execution
pub struct FuelSystem {
    /// Current fuel amount
    fuel: AtomicU64,
    /// Fuel replenishment rate per cycle
    replenishment_rate: u64,
    /// Maximum fuel capacity
    max_fuel: u64,
    /// Fuel consumption per instruction
    fuel_per_instruction: u64,
}

impl FuelSystem {
    /// Create a new fuel system
    pub fn new(initial_fuel: u64, replenishment_rate: u64, max_fuel: u64) -> Self {
        FuelSystem {
            fuel: AtomicU64::new(initial_fuel),
            replenishment_rate,
            max_fuel,
            fuel_per_instruction: 1,
        }
    }
    
    /// Consume fuel for an instruction
    pub fn consume_fuel(&self) -> bool {
        let current = self.fuel.load(Ordering::Relaxed);
        if current >= self.fuel_per_instruction {
            self.fuel.fetch_sub(self.fuel_per_instruction, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
    
    /// Replenish fuel
    pub fn replenish(&self) {
        let current = self.fuel.load(Ordering::Relaxed);
        let new_fuel = (current + self.replenishment_rate).min(self.max_fuel);
        self.fuel.store(new_fuel, Ordering::Relaxed);
    }
    
    /// Get current fuel level
    pub fn current_fuel(&self) -> u64 {
        self.fuel.load(Ordering::Relaxed)
    }
    
    /// Check if fuel is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.fuel.load(Ordering::Relaxed) < self.fuel_per_instruction
    }
    
    /// Reset fuel to maximum
    pub fn reset(&self) {
        self.fuel.store(self.max_fuel, Ordering::Relaxed);
    }
}

/// Divergence detection for infinite loops
pub struct DivergenceDetector {
    /// Last progress time for each process
    last_progress: RwLock<HashMap<Pid, Instant>>,
    /// Timeout duration for detecting divergence
    timeout: Duration,
    /// Watchdog thread active flag
    watchdog_active: AtomicBool,
}

impl DivergenceDetector {
    /// Create a new divergence detector
    pub fn new(timeout: Duration) -> Self {
        DivergenceDetector {
            last_progress: RwLock::new(HashMap::new()),
            timeout,
            watchdog_active: AtomicBool::new(false),
        }
    }
    
    /// Register a process for monitoring
    pub fn register_process(&self, pid: Pid) {
        let mut progress = self.last_progress.write().unwrap();
        progress.insert(pid, Instant::now());
    }
    
    /// Unregister a process from monitoring
    pub fn unregister_process(&self, pid: Pid) {
        let mut progress = self.last_progress.write().unwrap();
        progress.remove(&pid);
    }
    
    /// Observe progress for a process
    pub fn observe_progress(&self, pid: Pid) -> FaultResult<()> {
        let now = Instant::now();
        
        {
            let mut progress = self.last_progress.write().unwrap();
            if let Some(last) = progress.get(&pid) {
                if now.duration_since(*last) > self.timeout {
                    return Err(FaultError::InstructionLimitExceeded);
                }
            }
            progress.insert(pid, now);
        }
        
        Ok(())
    }
    
    /// Check for divergent processes
    pub fn check_divergence(&self) -> Vec<Pid> {
        let now = Instant::now();
        let progress = self.last_progress.read().unwrap();
        
        progress
            .iter()
            .filter_map(|(pid, last_time)| {
                if now.duration_since(*last_time) > self.timeout {
                    Some(*pid)
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Start watchdog monitoring
    pub fn start_watchdog(&self) {
        self.watchdog_active.store(true, Ordering::Relaxed);
    }
    
    /// Stop watchdog monitoring
    pub fn stop_watchdog(&self) {
        self.watchdog_active.store(false, Ordering::Relaxed);
    }
    
    /// Check if watchdog is active
    pub fn is_watchdog_active(&self) -> bool {
        self.watchdog_active.load(Ordering::Relaxed)
    }
}

/// Divergence types
#[derive(Debug, Clone)]
pub enum Divergence {
    /// Instruction limit exceeded
    InstructionLimit,
    /// Memory limit exceeded
    MemoryLimit,
    /// Timeout occurred
    Timeout,
    /// Fuel exhausted
    FuelExhaustion,
}

/// Divergence error types
#[derive(Debug, Clone)]
pub enum DivergenceError {
    /// Instruction limit exceeded
    InstructionLimitExceeded,
    /// Memory limit exceeded
    MemoryLimitExceeded,
    /// Timeout occurred
    Timeout,
    /// Fuel exhausted
    FuelExhaustion,
}

/// Bounded actor wrapper for execution limits
pub struct BoundedActor {
    /// The wrapped actor
    inner: Box<dyn ReamActor>,
    /// Execution bounds
    bounds: ExecutionBounds,
    /// Instruction counter
    instruction_count: AtomicU64,
    /// Memory usage counter
    memory_usage: AtomicU64,
    /// Message count
    message_count: AtomicU64,
    /// Fuel system
    fuel_system: FuelSystem,
    /// Start time for timeout detection
    start_time: Instant,
}

impl BoundedActor {
    /// Create a new bounded actor
    pub fn new(inner: Box<dyn ReamActor>, bounds: ExecutionBounds) -> Self {
        let fuel_system = FuelSystem::new(
            bounds.instruction_limit,
            bounds.instruction_limit / 100, // 1% replenishment rate
            bounds.instruction_limit,
        );
        
        BoundedActor {
            inner,
            bounds,
            instruction_count: AtomicU64::new(0),
            memory_usage: AtomicU64::new(0),
            message_count: AtomicU64::new(0),
            fuel_system,
            start_time: Instant::now(),
        }
    }
    
    /// Process a message with bounds checking
    pub fn process_message(&mut self) -> Result<Option<()>, DivergenceError> {
        // Check instruction limit
        let instructions = self.instruction_count.fetch_add(1, Ordering::Relaxed);
        if instructions >= self.bounds.instruction_limit {
            return Err(DivergenceError::InstructionLimitExceeded);
        }
        
        // Check memory limit
        if self.memory_usage.load(Ordering::Relaxed) >= self.bounds.memory_limit {
            return Err(DivergenceError::MemoryLimitExceeded);
        }
        
        // Check message limit
        if self.message_count.load(Ordering::Relaxed) >= self.bounds.message_limit {
            return Err(DivergenceError::InstructionLimitExceeded);
        }
        
        // Check fuel
        if !self.fuel_system.consume_fuel() {
            return Err(DivergenceError::FuelExhaustion);
        }
        
        // Check timeout (simple implementation)
        if self.start_time.elapsed() > Duration::from_secs(30) {
            return Err(DivergenceError::Timeout);
        }
        
        // Process message with the inner actor
        // For now, we'll simulate message processing
        Ok(Some(()))
    }
    
    /// Reset all counters
    pub fn reset_counters(&self) {
        self.instruction_count.store(0, Ordering::Relaxed);
        self.memory_usage.store(0, Ordering::Relaxed);
        self.message_count.store(0, Ordering::Relaxed);
        self.fuel_system.reset();
    }
    
    /// Get current resource usage
    pub fn get_resource_usage(&self) -> (u64, u64, u64) {
        (
            self.instruction_count.load(Ordering::Relaxed),
            self.memory_usage.load(Ordering::Relaxed),
            self.message_count.load(Ordering::Relaxed),
        )
    }
    
    /// Get fuel level
    pub fn get_fuel_level(&self) -> u64 {
        self.fuel_system.current_fuel()
    }
    
    /// Replenish fuel
    pub fn replenish_fuel(&self) {
        self.fuel_system.replenish();
    }
}

/// Safety verifier for termination guarantees
pub struct SafetyVerifier {
    /// Safety invariants to check
    invariants: Vec<SafetyInvariant>,
}

/// Safety invariant types
#[derive(Debug, Clone)]
pub enum SafetyInvariant {
    /// Termination guarantee
    Termination,
    /// Memory boundedness
    MemoryBounded,
    /// Message progress
    MessageProgress,
}

/// Safety report
#[derive(Debug, Clone)]
pub struct SafetyReport {
    /// Whether termination is guaranteed
    pub termination_guaranteed: bool,
    /// Whether memory is bounded
    pub memory_bounded: bool,
    /// Whether message progress is ensured
    pub message_progress: bool,
}

impl SafetyVerifier {
    /// Create a new safety verifier
    pub fn new() -> Self {
        SafetyVerifier {
            invariants: vec![
                SafetyInvariant::Termination,
                SafetyInvariant::MemoryBounded,
                SafetyInvariant::MessageProgress,
            ],
        }
    }
    
    /// Verify safety properties of an actor
    pub fn verify_actor(&self, _actor: &dyn ReamActor) -> SafetyReport {
        // For now, we'll provide a simple implementation
        // In a full implementation, this would analyze the actor's code
        SafetyReport {
            termination_guaranteed: true,
            memory_bounded: true,
            message_progress: true,
        }
    }
    
    /// Check termination guarantee
    fn check_termination(&self, _actor: &dyn ReamActor) -> bool {
        // Simplified check - in reality would analyze control flow
        true
    }
    
    /// Check memory boundedness
    fn check_memory_bounds(&self, _actor: &dyn ReamActor) -> bool {
        // Simplified check - in reality would analyze memory allocation
        true
    }
    
    /// Check message progress
    fn check_message_progress(&self, _actor: &dyn ReamActor) -> bool {
        // Simplified check - in reality would analyze message handling
        true
    }
}

impl Default for SafetyVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Infinite loop prevention system
pub struct InfiniteLoopPrevention {
    /// Divergence detector
    pub detector: DivergenceDetector,
    /// Safety verifier
    pub verifier: SafetyVerifier,
    /// Default execution bounds
    pub bounds: ExecutionBounds,
}

impl InfiniteLoopPrevention {
    /// Create a new infinite loop prevention system
    pub fn new(timeout: Duration, bounds: ExecutionBounds) -> Self {
        InfiniteLoopPrevention {
            detector: DivergenceDetector::new(timeout),
            verifier: SafetyVerifier::new(),
            bounds,
        }
    }
    
    /// Wrap an actor with bounded execution
    pub fn wrap_actor(&self, actor: Box<dyn ReamActor>) -> BoundedActor {
        BoundedActor::new(actor, self.bounds)
    }
    
    /// Start monitoring
    pub fn start_monitoring(&self) {
        self.detector.start_watchdog();
    }
    
    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        self.detector.stop_watchdog();
    }
    
    /// Check for divergent processes
    pub fn check_divergence(&self) -> Vec<Pid> {
        self.detector.check_divergence()
    }
}

impl Default for InfiniteLoopPrevention {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(10), // 10 second timeout
            ExecutionBounds::default(),
        )
    }
}
