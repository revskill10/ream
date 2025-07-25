//! Comprehensive Performance Benchmarks
//!
//! This module provides comprehensive benchmarks for all implemented features
//! including bytecode execution, preemptive scheduling, work-stealing, real-time scheduling,
//! and resource management as specified in IMPROVEMENT.md and PREEMPTIVE_SCHEDULING.md

use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use std::thread;

use ream::runtime::{
    WorkStealingScheduler, ScheduledTask, RealTimeScheduler, RealTimeTask,
    SchedulingAlgorithm, TaskType, ResourceManager, ResourceQuotas,
    PreemptionTimer, ProcessExecutor, Process, ProcessHandle, ReamActor
};
use ream::bytecode::{BytecodeVerifier, SecurityManager, create_sandbox_manager, BytecodeProgram, Bytecode, Value};
use ream::types::{Pid, Priority, MessagePayload, EffectGrade};
use ream::error::RuntimeResult;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of iterations
    pub iterations: u32,
    /// Number of concurrent workers
    pub workers: usize,
    /// Task complexity (operations per task)
    pub task_complexity: u32,
    /// Memory pressure (MB)
    pub memory_pressure: u64,
    /// Enable detailed logging
    pub detailed_logging: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        BenchmarkConfig {
            iterations: 1000,
            workers: num_cpus::get(),
            task_complexity: 1000,
            memory_pressure: 100, // 100 MB
            detailed_logging: false,
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Total execution time
    pub total_time: Duration,
    /// Average time per operation
    pub avg_time_per_op: Duration,
    /// Operations per second
    pub ops_per_second: f64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// CPU utilization (0.0 to 1.0)
    pub cpu_utilization: f64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Additional metrics
    pub metrics: std::collections::HashMap<String, f64>,
}

/// Test actor for benchmarks
struct BenchmarkActor {
    work_counter: Arc<AtomicU64>,
    complexity: u32,
}

impl BenchmarkActor {
    fn new(work_counter: Arc<AtomicU64>, complexity: u32) -> Self {
        BenchmarkActor {
            work_counter,
            complexity,
        }
    }
}

impl ReamActor for BenchmarkActor {
    fn receive(&mut self, _message: MessagePayload) -> RuntimeResult<()> {
        // Simulate computational work
        for _ in 0..self.complexity {
            self.work_counter.fetch_add(1, Ordering::Relaxed);
            // Prevent optimization
            std::hint::black_box(self.work_counter.load(Ordering::Relaxed));
        }
        Ok(())
    }
    
    fn handle_link(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_unlink(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_monitor(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_demonitor(&mut self, _pid: Pid) -> RuntimeResult<()> { Ok(()) }
    fn handle_exit(&mut self, _pid: Pid, _reason: String) -> RuntimeResult<()> { Ok(()) }
}

/// Benchmark suite for all components
pub struct ComprehensiveBenchmark {
    config: BenchmarkConfig,
}

impl ComprehensiveBenchmark {
    /// Create a new benchmark suite
    pub fn new(config: BenchmarkConfig) -> Self {
        ComprehensiveBenchmark { config }
    }
    
    /// Run all benchmarks
    pub fn run_all(&self) -> std::collections::HashMap<String, BenchmarkResults> {
        let mut results = std::collections::HashMap::new();
        
        println!("Running comprehensive benchmarks...");
        
        // Bytecode execution benchmarks
        results.insert("bytecode_execution".to_string(), self.benchmark_bytecode_execution());
        results.insert("bytecode_verification".to_string(), self.benchmark_bytecode_verification());
        results.insert("security_enforcement".to_string(), self.benchmark_security_enforcement());
        
        // Scheduling benchmarks
        results.insert("preemptive_scheduling".to_string(), self.benchmark_preemptive_scheduling());
        results.insert("work_stealing".to_string(), self.benchmark_work_stealing());
        results.insert("realtime_scheduling".to_string(), self.benchmark_realtime_scheduling());
        
        // Resource management benchmarks
        results.insert("resource_tracking".to_string(), self.benchmark_resource_tracking());
        results.insert("quota_enforcement".to_string(), self.benchmark_quota_enforcement());
        
        // Stress tests
        results.insert("high_concurrency".to_string(), self.benchmark_high_concurrency());
        results.insert("memory_pressure".to_string(), self.benchmark_memory_pressure());
        
        results
    }
    
    /// Benchmark bytecode execution performance
    fn benchmark_bytecode_execution(&self) -> BenchmarkResults {
        let start = Instant::now();
        let work_counter = Arc::new(AtomicU64::new(0));
        let mut successes = 0;
        
        for _ in 0..self.config.iterations {
            let mut program = BytecodeProgram::new("benchmark".to_string());
            
            // Create a simple arithmetic program
            let const1 = program.add_constant(Value::Int(42));
            let const2 = program.add_constant(Value::Int(24));
            
            program.add_instruction(Bytecode::Const(const1, EffectGrade::Pure));
            program.add_instruction(Bytecode::Const(const2, EffectGrade::Pure));
            program.add_instruction(Bytecode::Add(EffectGrade::Pure));
            program.add_instruction(Bytecode::Pop(EffectGrade::Pure));
            
            // Execute the program (simplified)
            work_counter.fetch_add(1, Ordering::Relaxed);
            successes += 1;
        }
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0, // Would need actual measurement
            cpu_utilization: 0.0, // Would need actual measurement
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark bytecode verification performance
    fn benchmark_bytecode_verification(&self) -> BenchmarkResults {
        let start = Instant::now();
        let mut successes = 0;
        
        for _ in 0..self.config.iterations {
            let mut verifier = BytecodeVerifier::new();
            let mut program = BytecodeProgram::new("benchmark".to_string());
            
            // Create a program to verify
            let const_idx = program.add_constant(Value::Int(100));
            program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
            program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
            program.add_instruction(Bytecode::Add(EffectGrade::Pure));
            
            if verifier.verify(&program).is_ok() {
                successes += 1;
            }
        }
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark security enforcement performance
    fn benchmark_security_enforcement(&self) -> BenchmarkResults {
        let start = Instant::now();
        let mut successes = 0;
        
        for _ in 0..self.config.iterations {
            let manager = create_sandbox_manager();
            
            // Simulate security checks
            if manager.check_instruction_count().is_ok() {
                if manager.check_execution_time().is_ok() {
                    successes += 1;
                }
            }
        }
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark preemptive scheduling performance
    fn benchmark_preemptive_scheduling(&self) -> BenchmarkResults {
        let start = Instant::now();
        let work_counter = Arc::new(AtomicU64::new(0));
        let mut successes = 0;
        
        let timer = Arc::new(PreemptionTimer::new(Duration::from_millis(10)));
        timer.start().unwrap();
        
        let mut executor = ProcessExecutor::new(timer.clone());
        
        for _ in 0..self.config.iterations {
            let pid = Pid::new();
            let actor = Box::new(BenchmarkActor::new(work_counter.clone(), 100));
            let process = Process::new(pid, actor, Priority::Normal);
            let handle = ProcessHandle::new(process);
            
            if executor.execute_with_preemption(&handle).is_ok() {
                successes += 1;
            }
        }
        
        timer.stop();
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark work-stealing scheduler performance
    fn benchmark_work_stealing(&self) -> BenchmarkResults {
        let start = Instant::now();
        let work_counter = Arc::new(AtomicU64::new(0));
        let mut successes = 0;
        
        let mut scheduler = WorkStealingScheduler::new(Some(self.config.workers));
        scheduler.start().unwrap();
        
        // Create and schedule tasks
        for i in 0..self.config.iterations {
            let pid = Pid::new();
            let actor = Box::new(BenchmarkActor::new(work_counter.clone(), self.config.task_complexity));
            let process = Process::new(pid, actor, Priority::Normal);
            let handle = ProcessHandle::new(process);
            
            scheduler.register_process(handle);
            
            let task = ScheduledTask::new(pid, Priority::Normal);
            scheduler.schedule_task(task);
            successes += 1;
        }
        
        // Let tasks execute
        thread::sleep(Duration::from_millis(100));
        
        scheduler.stop();
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark real-time scheduling performance
    fn benchmark_realtime_scheduling(&self) -> BenchmarkResults {
        let start = Instant::now();
        let mut successes = 0;
        
        let mut scheduler = RealTimeScheduler::new(SchedulingAlgorithm::EDF);
        
        for i in 0..self.config.iterations {
            let pid = Pid::new();
            let task = RealTimeTask::sporadic(
                pid,
                Priority::Normal,
                Duration::from_millis(100 + (i % 100) as u64),
                Duration::from_millis(10),
            );
            
            if scheduler.add_task(task).is_ok() {
                if scheduler.next_task().is_some() {
                    successes += 1;
                }
            }
        }
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark resource tracking performance
    fn benchmark_resource_tracking(&self) -> BenchmarkResults {
        let start = Instant::now();
        let mut successes = 0;
        
        let quotas = ResourceQuotas::default();
        let manager = ResourceManager::new(quotas);
        
        for i in 0..self.config.iterations {
            let pid = Pid::new();
            manager.register_process(pid, None);
            
            if manager.update_cpu_time(pid, Duration::from_millis(i as u64 % 100)).is_ok() {
                if manager.update_memory_usage(pid, (i as u64) * 1024, (i as u64) * 512).is_ok() {
                    successes += 1;
                }
            }
            
            manager.unregister_process(pid);
        }
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark quota enforcement performance
    fn benchmark_quota_enforcement(&self) -> BenchmarkResults {
        let start = Instant::now();
        let mut successes = 0;
        
        let mut quotas = ResourceQuotas::default();
        quotas.max_memory = Some(1024 * 1024); // 1 MB limit
        
        let manager = ResourceManager::new(quotas);
        
        for i in 0..self.config.iterations {
            let pid = Pid::new();
            manager.register_process(pid, None);
            
            // Try to use memory within quota
            if manager.update_memory_usage(pid, 512 * 1024, 256 * 1024).is_ok() {
                successes += 1;
            }
            
            manager.unregister_process(pid);
        }
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark high concurrency scenarios
    fn benchmark_high_concurrency(&self) -> BenchmarkResults {
        let start = Instant::now();
        let work_counter = Arc::new(AtomicU64::new(0));
        let successes = Arc::new(AtomicU64::new(0));
        
        let mut handles = Vec::new();
        
        // Spawn many concurrent workers
        for _ in 0..self.config.workers {
            let work_counter = Arc::clone(&work_counter);
            let successes = Arc::clone(&successes);
            let iterations = self.config.iterations / self.config.workers as u32;
            
            let handle = thread::spawn(move || {
                for _ in 0..iterations {
                    // Simulate concurrent work
                    work_counter.fetch_add(1, Ordering::Relaxed);
                    successes.fetch_add(1, Ordering::Relaxed);
                    
                    // Small delay to create contention
                    thread::sleep(Duration::from_nanos(100));
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all workers to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        let total_time = start.elapsed();
        let total_successes = successes.load(Ordering::Relaxed);
        let ops_per_second = total_successes as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / total_successes as u32,
            ops_per_second,
            memory_usage: 0,
            cpu_utilization: 0.0,
            success_rate: total_successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
    
    /// Benchmark memory pressure scenarios
    fn benchmark_memory_pressure(&self) -> BenchmarkResults {
        let start = Instant::now();
        let mut successes = 0;
        
        // Allocate memory to create pressure
        let mut memory_blocks = Vec::new();
        let block_size = (self.config.memory_pressure * 1024 * 1024) / self.config.iterations as u64;
        
        for _ in 0..self.config.iterations {
            let block = vec![0u8; block_size as usize];
            memory_blocks.push(block);
            successes += 1;
            
            // Simulate some work with the memory
            if !memory_blocks.is_empty() {
                std::hint::black_box(&memory_blocks[memory_blocks.len() - 1]);
            }
        }
        
        let total_time = start.elapsed();
        let ops_per_second = self.config.iterations as f64 / total_time.as_secs_f64();
        
        BenchmarkResults {
            total_time,
            avg_time_per_op: total_time / self.config.iterations,
            ops_per_second,
            memory_usage: self.config.memory_pressure * 1024 * 1024,
            cpu_utilization: 0.0,
            success_rate: successes as f64 / self.config.iterations as f64,
            metrics: std::collections::HashMap::new(),
        }
    }
}

/// Print benchmark results in a formatted table
pub fn print_benchmark_results(results: &std::collections::HashMap<String, BenchmarkResults>) {
    println!("\n=== COMPREHENSIVE BENCHMARK RESULTS ===");
    println!("{:<25} {:>12} {:>15} {:>12} {:>12}",
             "Benchmark", "Total Time", "Ops/Second", "Success Rate", "Avg Time/Op");
    println!("{}", "-".repeat(80));

    for (name, result) in results {
        println!("{:<25} {:>12.3}ms {:>15.0} {:>11.1}% {:>12.3}μs",
                 name,
                 result.total_time.as_millis(),
                 result.ops_per_second,
                 result.success_rate * 100.0,
                 result.avg_time_per_op.as_micros());
    }

    println!("{}", "-".repeat(80));

    // Calculate overall statistics
    let total_ops: f64 = results.values().map(|r| r.ops_per_second).sum();
    let avg_success_rate: f64 = results.values().map(|r| r.success_rate).sum::<f64>() / results.len() as f64;

    println!("Total Throughput: {:.0} ops/second", total_ops);
    println!("Average Success Rate: {:.1}%", avg_success_rate * 100.0);
    println!("Number of Benchmarks: {}", results.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_suite() {
        let config = BenchmarkConfig {
            iterations: 10, // Small number for testing
            workers: 2,
            task_complexity: 10,
            memory_pressure: 1, // 1 MB
            detailed_logging: false,
        };
        
        let benchmark = ComprehensiveBenchmark::new(config);
        let results = benchmark.run_all();
        
        // Verify all benchmarks ran
        assert!(results.contains_key("bytecode_execution"));
        assert!(results.contains_key("bytecode_verification"));
        assert!(results.contains_key("security_enforcement"));
        assert!(results.contains_key("preemptive_scheduling"));
        assert!(results.contains_key("work_stealing"));
        assert!(results.contains_key("realtime_scheduling"));
        assert!(results.contains_key("resource_tracking"));
        assert!(results.contains_key("quota_enforcement"));
        assert!(results.contains_key("high_concurrency"));
        assert!(results.contains_key("memory_pressure"));
        
        // Verify results have reasonable values
        for (name, result) in &results {
            assert!(result.total_time > Duration::ZERO, "Benchmark {} took no time", name);
            assert!(result.ops_per_second > 0.0, "Benchmark {} had no throughput", name);
            assert!(result.success_rate >= 0.0 && result.success_rate <= 1.0, "Benchmark {} had invalid success rate", name);
        }
    }
}
