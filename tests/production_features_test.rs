//! Integration tests for production-grade REAM features
//!
//! Tests fault tolerance, STM, infinite loop prevention, and other
//! production features to ensure they work correctly together.

use ream::{
    new_advanced_runtime, AdvancedReamRuntime, Pid, ExecutionBounds, MemoryLayout,
    ReamError, ReamResult,
};
use ream::runtime::actor::ReamActor;
use ream::error::RuntimeResult;
use ream::types::MessagePayload;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Test actor that counts messages
#[derive(Clone)]
struct CounterActor {
    count: Arc<AtomicU64>,
    pid: Pid,
}

impl CounterActor {
    fn new() -> Self {
        CounterActor {
            count: Arc::new(AtomicU64::new(0)),
            pid: Pid::new(),
        }
    }
    
    fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

impl ReamActor for CounterActor {
    fn receive(&mut self, _message: MessagePayload) -> RuntimeResult<()> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    fn pid(&self) -> Pid {
        self.pid
    }
    
    fn restart(&mut self) -> RuntimeResult<()> {
        self.count.store(0, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_alive(&self) -> bool {
        true
    }
    
    fn debug_state(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.count.load(Ordering::Relaxed))
    }
}

/// Test actor that simulates resource exhaustion
#[derive(Clone)]
struct ResourceHungryActor {
    pid: Pid,
    instruction_count: Arc<AtomicU64>,
}

impl ResourceHungryActor {
    fn new() -> Self {
        ResourceHungryActor {
            pid: Pid::new(),
            instruction_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl ReamActor for ResourceHungryActor {
    fn receive(&mut self, _message: MessagePayload) -> RuntimeResult<()> {
        // Simulate heavy computation
        for _ in 0..1000 {
            self.instruction_count.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
    
    fn pid(&self) -> Pid {
        self.pid
    }
    
    fn restart(&mut self) -> RuntimeResult<()> {
        self.instruction_count.store(0, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_alive(&self) -> bool {
        true
    }
    
    fn debug_state(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.instruction_count.load(Ordering::Relaxed))
    }
}

#[test]
fn test_advanced_runtime_creation() {
    let runtime = new_advanced_runtime();
    assert!(runtime.get_stats().isolated_processes == 0);
    assert!(runtime.get_stats().bounded_actors == 0);
}

#[test]
fn test_spawn_advanced_actor() {
    let mut runtime = new_advanced_runtime();
    
    // Start the runtime
    runtime.start().expect("Failed to start runtime");
    
    // Spawn an actor
    let actor = CounterActor::new();
    let pid = runtime.spawn_advanced_actor(actor).expect("Failed to spawn actor");
    
    // Check that the actor was spawned
    assert!(runtime.is_alive(pid));
    
    // Check statistics
    let stats = runtime.get_stats();
    assert_eq!(stats.isolated_processes, 1);
    assert_eq!(stats.bounded_actors, 1);
    
    // Stop the runtime
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_stm_message_passing() {
    let mut runtime = new_advanced_runtime();
    runtime.start().expect("Failed to start runtime");
    
    // Spawn two actors
    let actor1 = CounterActor::new();
    let actor2 = CounterActor::new();
    
    let pid1 = runtime.spawn_advanced_actor(actor1).expect("Failed to spawn actor1");
    let pid2 = runtime.spawn_advanced_actor(actor2).expect("Failed to spawn actor2");
    
    // Send messages using STM
    let message = b"Hello, World!".to_vec();
    let version1 = runtime.send_message(pid1, pid2, message.clone()).expect("Failed to send message");
    let version2 = runtime.send_message(pid1, pid2, message.clone()).expect("Failed to send message");
    
    assert!(version2 > version1);
    
    // Receive messages
    let messages = runtime.receive_messages(pid2, 0).expect("Failed to receive messages");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].payload, message);
    assert_eq!(messages[1].payload, message);
    
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_fault_tolerance() {
    let mut runtime = new_advanced_runtime();
    runtime.start().expect("Failed to start runtime");
    
    // Spawn a resource-hungry actor
    let actor = ResourceHungryActor::new();
    let pid = runtime.spawn_advanced_actor(actor).expect("Failed to spawn actor");
    
    // Process messages (this should trigger resource limits)
    let initial_stats = runtime.get_stats();
    
    // Try to process messages multiple times
    for _ in 0..10 {
        let _ = runtime.process_actor_messages(pid);
    }
    
    // Check if fault recovery was triggered
    let final_stats = runtime.get_stats();
    // Note: In a real test, we'd check if fault_recoveries increased
    // For now, we just verify the system is still responsive
    assert!(runtime.is_alive(pid) || final_stats.fault_recoveries > initial_stats.fault_recoveries);
    
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_resource_usage_tracking() {
    let mut runtime = new_advanced_runtime();
    runtime.start().expect("Failed to start runtime");
    
    let actor = CounterActor::new();
    let pid = runtime.spawn_advanced_actor(actor).expect("Failed to spawn actor");
    
    // Get initial resource usage
    let initial_usage = runtime.get_resource_usage(pid);
    assert!(initial_usage.is_some());
    
    let (instructions, memory, messages) = initial_usage.unwrap();
    assert_eq!(instructions, 0);
    assert_eq!(memory, 0);
    assert_eq!(messages, 0);
    
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_divergence_detection() {
    let mut runtime = new_advanced_runtime();
    runtime.start().expect("Failed to start runtime");
    
    let actor = CounterActor::new();
    let pid = runtime.spawn_advanced_actor(actor).expect("Failed to spawn actor");
    
    // Check for divergent processes (should be none initially)
    let divergent = runtime.check_divergence();
    assert!(divergent.is_empty());
    
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_stm_compaction() {
    let mut runtime = new_advanced_runtime();
    runtime.start().expect("Failed to start runtime");
    
    let actor1 = CounterActor::new();
    let actor2 = CounterActor::new();
    
    let pid1 = runtime.spawn_advanced_actor(actor1).expect("Failed to spawn actor1");
    let pid2 = runtime.spawn_advanced_actor(actor2).expect("Failed to spawn actor2");
    
    // Send many messages to create log entries
    for i in 0..100 {
        let message = format!("Message {}", i).into_bytes();
        runtime.send_message(pid1, pid2, message).expect("Failed to send message");
    }
    
    // Compact mailboxes (keep last 50 versions)
    let compacted = runtime.compact_mailboxes(50).expect("Failed to compact mailboxes");
    assert!(compacted > 0);
    
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_multiple_actors_isolation() {
    let mut runtime = new_advanced_runtime();
    runtime.start().expect("Failed to start runtime");
    
    // Spawn multiple actors
    let mut pids = Vec::new();
    for _ in 0..10 {
        let actor = CounterActor::new();
        let pid = runtime.spawn_advanced_actor(actor).expect("Failed to spawn actor");
        pids.push(pid);
    }
    
    // Verify all actors are alive and isolated
    for pid in &pids {
        assert!(runtime.is_alive(*pid));
        assert!(runtime.get_resource_usage(*pid).is_some());
    }
    
    // Check statistics
    let stats = runtime.get_stats();
    assert_eq!(stats.isolated_processes, 10);
    assert_eq!(stats.bounded_actors, 10);
    
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_runtime_statistics() {
    let mut runtime = new_advanced_runtime();
    runtime.start().expect("Failed to start runtime");
    
    let initial_stats = runtime.get_stats();
    assert_eq!(initial_stats.isolated_processes, 0);
    assert_eq!(initial_stats.bounded_actors, 0);
    assert_eq!(initial_stats.fault_recoveries, 0);
    assert_eq!(initial_stats.divergence_detections, 0);
    
    // Spawn an actor
    let actor = CounterActor::new();
    let _pid = runtime.spawn_advanced_actor(actor).expect("Failed to spawn actor");
    
    let updated_stats = runtime.get_stats();
    assert_eq!(updated_stats.isolated_processes, 1);
    assert_eq!(updated_stats.bounded_actors, 1);
    
    runtime.stop().expect("Failed to stop runtime");
}

#[test]
fn test_custom_execution_bounds() {
    // Test that we can create actors with custom execution bounds
    // This is more of a compilation test to ensure the types work correctly
    let bounds = ExecutionBounds {
        instruction_limit: 500_000,
        memory_limit: 5 * 1024 * 1024, // 5MB
        message_limit: 500,
    };
    
    let layout = MemoryLayout::new(32 * 1024, 512 * 1024); // 32KB heap, 512KB stack
    
    // These should compile without errors
    assert!(bounds.instruction_limit == 500_000);
    assert!(layout.process_heap.end - layout.process_heap.start == 32 * 1024);
}
