//! JIT optimization and performance monitoring

use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::bytecode::BytecodeProgram;
use crate::jit::JitOptions;
use crate::error::JitResult;

/// Performance monitor for hot spot detection
pub struct PerformanceMonitor {
    /// Execution counts per instruction
    instruction_counts: HashMap<usize, u64>,
    /// Execution times per program
    execution_times: HashMap<String, Vec<Duration>>,
    /// Hot spot threshold
    hot_spot_threshold: u64,
    /// Total executions
    total_executions: u64,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        PerformanceMonitor {
            instruction_counts: HashMap::new(),
            execution_times: HashMap::new(),
            hot_spot_threshold: 1000,
            total_executions: 0,
        }
    }
    
    /// Set hot spot threshold
    pub fn set_hot_spot_threshold(&mut self, threshold: u64) {
        self.hot_spot_threshold = threshold;
    }
    
    /// Observe program execution
    pub fn observe_execution(&mut self, program: &BytecodeProgram, duration: Duration) {
        let program_key = self.generate_program_key(program);
        
        self.execution_times
            .entry(program_key)
            .or_insert_with(Vec::new)
            .push(duration);
        
        self.total_executions += 1;
        
        // Update instruction counts
        for (pc, _) in program.instructions.iter().enumerate() {
            *self.instruction_counts.entry(pc).or_insert(0) += 1;
        }
    }
    
    /// Check if a program should be optimized
    pub fn should_optimize(&self, program: &BytecodeProgram) -> bool {
        let program_key = self.generate_program_key(program);
        
        if let Some(times) = self.execution_times.get(&program_key) {
            times.len() as u64 >= self.hot_spot_threshold
        } else {
            false
        }
    }
    
    /// Get hot spots for a program
    pub fn get_hot_spots(&self, program: &BytecodeProgram) -> Vec<usize> {
        let mut hot_spots = Vec::new();
        
        for (pc, _) in program.instructions.iter().enumerate() {
            if let Some(&count) = self.instruction_counts.get(&pc) {
                if count >= self.hot_spot_threshold {
                    hot_spots.push(pc);
                }
            }
        }
        
        hot_spots
    }
    
    /// Get average execution time for a program
    pub fn average_execution_time(&self, program: &BytecodeProgram) -> Option<Duration> {
        let program_key = self.generate_program_key(program);
        
        if let Some(times) = self.execution_times.get(&program_key) {
            if !times.is_empty() {
                let total: Duration = times.iter().sum();
                Some(total / times.len() as u32)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Get execution statistics
    pub fn stats(&self) -> MonitorStats {
        MonitorStats {
            total_executions: self.total_executions,
            programs_monitored: self.execution_times.len(),
            hot_spots_detected: self.instruction_counts.values().filter(|&&count| count >= self.hot_spot_threshold).count(),
        }
    }
    
    /// Reset monitoring data
    pub fn reset(&mut self) {
        self.instruction_counts.clear();
        self.execution_times.clear();
        self.total_executions = 0;
    }

    /// Enable profiling
    pub fn enable_profiling(&mut self) {
        // Enable detailed profiling - in practice this would configure
        // more detailed monitoring and instrumentation
        self.hot_spot_threshold = self.hot_spot_threshold.min(500); // Lower threshold for profiling
    }
    
    fn generate_program_key(&self, program: &BytecodeProgram) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        program.metadata.name.hash(&mut hasher);
        format!("prog_{:x}", hasher.finish())
    }
}

/// Performance monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitorStats {
    pub total_executions: u64,
    pub programs_monitored: usize,
    pub hot_spots_detected: usize,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Hot spot optimizer for aggressive optimization
pub struct HotSpotOptimizer {
    /// Optimization options
    options: JitOptions,
    /// Optimization cache
    cache: HashMap<String, BytecodeProgram>,
}

impl HotSpotOptimizer {
    /// Create a new hot spot optimizer
    pub fn new() -> Self {
        HotSpotOptimizer {
            options: JitOptions::default(),
            cache: HashMap::new(),
        }
    }
    
    /// Set optimization options
    pub fn set_options(&mut self, options: &JitOptions) {
        self.options = options.clone();
    }
    
    /// Optimize a program based on hot spots
    pub fn optimize_program(
        &mut self,
        program: &BytecodeProgram,
        hot_spots: &[usize],
    ) -> JitResult<BytecodeProgram> {
        let cache_key = self.generate_cache_key(program, hot_spots);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        let mut optimized = program.clone();
        
        // Apply hot spot specific optimizations
        if self.options.enable_inlining {
            optimized = self.inline_hot_functions(&optimized, hot_spots)?;
        }
        
        // Apply loop unrolling for hot loops
        optimized = self.unroll_hot_loops(&optimized, hot_spots)?;
        
        // Apply instruction scheduling
        optimized = self.schedule_instructions(&optimized, hot_spots)?;
        
        // Cache the result
        self.cache.insert(cache_key, optimized.clone());
        
        Ok(optimized)
    }
    
    /// Inline hot functions
    fn inline_hot_functions(
        &self,
        program: &BytecodeProgram,
        hot_spots: &[usize],
    ) -> JitResult<BytecodeProgram> {
        let mut optimized = program.clone();
        
        // Find function calls in hot spots
        for &pc in hot_spots {
            if let Some(instruction) = program.instructions.get(pc) {
                if let crate::bytecode::Bytecode::Call(func_id, effect) = instruction {
                    // Check if function is small enough to inline
                    if let Some(function) = program.get_function(*func_id) {
                        if function.instruction_count() <= self.options.inline_threshold {
                            // Inline the function (simplified)
                            optimized.instructions[pc] = crate::bytecode::Bytecode::Nop(*effect);
                            
                            // Insert function instructions
                            let mut insert_pos = pc + 1;
                            for instr in &function.instructions {
                                optimized.instructions.insert(insert_pos, instr.clone());
                                insert_pos += 1;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(optimized)
    }
    
    /// Unroll hot loops
    fn unroll_hot_loops(
        &self,
        program: &BytecodeProgram,
        _hot_spots: &[usize],
    ) -> JitResult<BytecodeProgram> {
        // Simplified loop unrolling
        // In a real implementation, this would detect loop patterns and unroll them
        Ok(program.clone())
    }
    
    /// Schedule instructions for better performance
    fn schedule_instructions(
        &self,
        program: &BytecodeProgram,
        _hot_spots: &[usize],
    ) -> JitResult<BytecodeProgram> {
        // Simplified instruction scheduling
        // In a real implementation, this would reorder instructions to minimize pipeline stalls
        Ok(program.clone())
    }
    
    fn generate_cache_key(&self, program: &BytecodeProgram, hot_spots: &[usize]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        program.metadata.name.hash(&mut hasher);
        hot_spots.hash(&mut hasher);
        self.options.opt_level.hash(&mut hasher);
        
        format!("opt_{:x}", hasher.finish())
    }
    
    /// Clear optimization cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.capacity())
    }
}

impl Default for HotSpotOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive optimization controller
pub struct AdaptiveOptimizer {
    /// Performance monitor
    monitor: PerformanceMonitor,
    /// Hot spot optimizer
    optimizer: HotSpotOptimizer,
    /// Optimization history
    history: HashMap<String, OptimizationRecord>,
}

/// Optimization record
#[derive(Debug, Clone)]
struct OptimizationRecord {
    optimizations_applied: u32,
    performance_improvement: f64,
    last_optimized: Instant,
}

impl AdaptiveOptimizer {
    /// Create a new adaptive optimizer
    pub fn new() -> Self {
        AdaptiveOptimizer {
            monitor: PerformanceMonitor::new(),
            optimizer: HotSpotOptimizer::new(),
            history: HashMap::new(),
        }
    }
    
    /// Observe program execution and potentially trigger optimization
    pub fn observe_and_optimize(
        &mut self,
        program: &BytecodeProgram,
        duration: Duration,
    ) -> JitResult<Option<BytecodeProgram>> {
        // Record execution
        self.monitor.observe_execution(program, duration);
        
        // Check if optimization is needed
        if self.should_optimize_now(program) {
            let hot_spots = self.monitor.get_hot_spots(program);
            let optimized = self.optimizer.optimize_program(program, &hot_spots)?;
            
            // Record optimization
            let program_key = self.generate_program_key(program);
            let record = self.history.entry(program_key).or_insert(OptimizationRecord {
                optimizations_applied: 0,
                performance_improvement: 0.0,
                last_optimized: Instant::now(),
            });
            
            record.optimizations_applied += 1;
            record.last_optimized = Instant::now();
            
            Ok(Some(optimized))
        } else {
            Ok(None)
        }
    }
    
    /// Check if optimization should be triggered
    fn should_optimize_now(&self, program: &BytecodeProgram) -> bool {
        let program_key = self.generate_program_key(program);
        
        // Check if enough time has passed since last optimization
        if let Some(record) = self.history.get(&program_key) {
            if record.last_optimized.elapsed() < Duration::from_secs(60) {
                return false;
            }
        }
        
        // Check if program is hot enough
        self.monitor.should_optimize(program)
    }
    
    fn generate_program_key(&self, program: &BytecodeProgram) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        program.metadata.name.hash(&mut hasher);
        format!("adaptive_{:x}", hasher.finish())
    }
    
    /// Get optimization statistics
    pub fn stats(&self) -> AdaptiveStats {
        AdaptiveStats {
            monitor_stats: self.monitor.stats(),
            programs_optimized: self.history.len(),
            total_optimizations: self.history.values().map(|r| r.optimizations_applied).sum(),
        }
    }
}

/// Adaptive optimization statistics
#[derive(Debug, Clone)]
pub struct AdaptiveStats {
    pub monitor_stats: MonitorStats,
    pub programs_optimized: usize,
    pub total_optimizations: u32,
}

impl Default for AdaptiveOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{Bytecode, BytecodeProgram};
    use crate::types::EffectGrade;

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();
        let mut program = BytecodeProgram::new("test".to_string());
        
        program.add_instruction(Bytecode::Const(0, EffectGrade::Pure));
        program.add_instruction(Bytecode::Ret(EffectGrade::Pure));
        
        // Simulate multiple executions
        for _ in 0..10 {
            monitor.observe_execution(&program, Duration::from_millis(1));
        }
        
        let stats = monitor.stats();
        assert_eq!(stats.total_executions, 10);
        assert_eq!(stats.programs_monitored, 1);
    }
    
    #[test]
    fn test_hot_spot_optimizer() {
        let mut optimizer = HotSpotOptimizer::new();
        let mut program = BytecodeProgram::new("test".to_string());
        
        program.add_instruction(Bytecode::Const(0, EffectGrade::Pure));
        program.add_instruction(Bytecode::Ret(EffectGrade::Pure));
        
        let hot_spots = vec![0];
        let optimized = optimizer.optimize_program(&program, &hot_spots).unwrap();
        
        // Should return a program (possibly optimized)
        assert!(optimized.instructions.len() >= program.instructions.len());
    }
    
    #[test]
    fn test_adaptive_optimizer() {
        let mut optimizer = AdaptiveOptimizer::new();
        let mut program = BytecodeProgram::new("test".to_string());
        
        program.add_instruction(Bytecode::Const(0, EffectGrade::Pure));
        program.add_instruction(Bytecode::Ret(EffectGrade::Pure));
        
        // Should not optimize on first execution
        let result = optimizer.observe_and_optimize(&program, Duration::from_millis(1)).unwrap();
        assert!(result.is_none());
        
        let stats = optimizer.stats();
        assert_eq!(stats.programs_optimized, 0);
    }
}
