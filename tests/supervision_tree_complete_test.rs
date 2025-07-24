//! Comprehensive tests for REAM Supervision Tree System
//! 
//! Tests the complete supervision tree implementation including:
//! - Child specifications and restart policies
//! - Supervisor specifications and strategies
//! - Hierarchical supervision trees
//! - Fault tolerance and restart mechanisms
//! - Advanced supervision features

use ream::runtime::supervisor::*;
use ream::runtime::process::ProcessHandle;
use ream::types::{Pid, RestartStrategy};
use std::time::Duration;

#[test]
fn test_child_spec_comprehensive() {
    // Test all child specification options
    let spec = ChildSpec::new("comprehensive_child".to_string())
        .restart_policy(RestartPolicy::Transient)
        .shutdown_timeout(Duration::from_secs(30))
        .child_type(ChildType::Supervisor)
        .max_restart_intensity(10);

    assert_eq!(spec.id, "comprehensive_child");
    assert_eq!(spec.restart_policy, RestartPolicy::Transient);
    assert_eq!(spec.shutdown_timeout, Duration::from_secs(30));
    assert_eq!(spec.child_type, ChildType::Supervisor);
    assert_eq!(spec.max_restart_intensity, 10);

    // Test default values
    let default_spec = ChildSpec::new("default_child".to_string());
    assert_eq!(default_spec.restart_policy, RestartPolicy::Permanent);
    assert_eq!(default_spec.shutdown_timeout, Duration::from_secs(5));
    assert_eq!(default_spec.child_type, ChildType::Worker);
    assert_eq!(default_spec.max_restart_intensity, 5);
}

#[test]
fn test_supervisor_spec_comprehensive() {
    let child1 = ChildSpec::new("worker1".to_string())
        .restart_policy(RestartPolicy::Permanent)
        .child_type(ChildType::Worker);
    
    let child2 = ChildSpec::new("worker2".to_string())
        .restart_policy(RestartPolicy::Transient)
        .child_type(ChildType::Worker);
    
    let child3 = ChildSpec::new("sub_supervisor".to_string())
        .restart_policy(RestartPolicy::Permanent)
        .child_type(ChildType::Supervisor);

    let spec = SupervisorSpec::new("main_supervisor".to_string())
        .strategy(RestartStrategy::RestForOne)
        .max_restarts(15)
        .restart_window(Duration::from_secs(300))
        .child(child1)
        .child(child2)
        .child(child3);

    assert_eq!(spec.name, "main_supervisor");
    assert_eq!(spec.strategy, RestartStrategy::RestForOne);
    assert_eq!(spec.max_restarts, 15);
    assert_eq!(spec.restart_window, Duration::from_secs(300));
    assert_eq!(spec.children.len(), 3);

    // Verify child specifications
    assert_eq!(spec.children[0].id, "worker1");
    assert_eq!(spec.children[0].restart_policy, RestartPolicy::Permanent);
    assert_eq!(spec.children[0].child_type, ChildType::Worker);

    assert_eq!(spec.children[1].id, "worker2");
    assert_eq!(spec.children[1].restart_policy, RestartPolicy::Transient);

    assert_eq!(spec.children[2].id, "sub_supervisor");
    assert_eq!(spec.children[2].child_type, ChildType::Supervisor);
}

#[test]
fn test_supervisor_state_restart_window() {
    let mut state = SupervisorState::new();
    let window = Duration::from_millis(100); // Short window for testing
    let max_restarts = 3;

    // Should allow restarts initially
    assert!(state.can_restart(max_restarts, window));

    // Record restarts within window
    state.record_restart(window);
    assert_eq!(state.restart_count, 1);
    assert!(state.can_restart(max_restarts, window));

    state.record_restart(window);
    assert_eq!(state.restart_count, 2);
    assert!(state.can_restart(max_restarts, window));

    state.record_restart(window);
    assert_eq!(state.restart_count, 3);
    assert!(!state.can_restart(max_restarts, window)); // At limit

    // Wait for window to expire
    std::thread::sleep(Duration::from_millis(150));

    // Should allow restart after window expires
    assert!(state.can_restart(max_restarts, window));

    // Recording restart should reset window
    state.record_restart(window);
    assert_eq!(state.restart_count, 1); // Reset to 1
}

#[test]
fn test_restart_policy_behavior() {
    let permanent_spec = ChildSpec::new("permanent".to_string())
        .restart_policy(RestartPolicy::Permanent);
    let transient_spec = ChildSpec::new("transient".to_string())
        .restart_policy(RestartPolicy::Transient);
    let temporary_spec = ChildSpec::new("temporary".to_string())
        .restart_policy(RestartPolicy::Temporary);

    let permanent_tree = ProcessTree::process(Pid::new(), permanent_spec);
    let transient_tree = ProcessTree::process(Pid::new(), transient_spec);
    let temporary_tree = ProcessTree::process(Pid::new(), temporary_spec);

    // Permanent should always be restartable
    assert!(permanent_tree.can_restart());

    // Transient should be restartable (in real implementation, this would depend on exit reason)
    assert!(transient_tree.can_restart());

    // Temporary should never be restartable
    assert!(!temporary_tree.can_restart());
}

#[test]
fn test_restart_strategies_comprehensive() {
    // Create supervisor with different restart strategies
    let test_strategies = vec![
        RestartStrategy::OneForOne,
        RestartStrategy::OneForAll,
        RestartStrategy::RestForOne,
    ];

    for strategy in test_strategies {
        let spec = SupervisorSpec::new(format!("supervisor_{:?}", strategy))
            .strategy(strategy)
            .max_restarts(5)
            .restart_window(Duration::from_secs(60));

        let mut tree = ProcessTree::supervisor(Pid::new(), spec);

        // Add multiple children
        let child1_pid = Pid::new();
        let child2_pid = Pid::new();
        let child3_pid = Pid::new();

        let child1_spec = ChildSpec::new("child1".to_string()).restart_policy(RestartPolicy::Permanent);
        let child2_spec = ChildSpec::new("child2".to_string()).restart_policy(RestartPolicy::Permanent);
        let child3_spec = ChildSpec::new("child3".to_string()).restart_policy(RestartPolicy::Permanent);

        tree.add_child(ProcessTree::process(child1_pid, child1_spec)).unwrap();
        tree.add_child(ProcessTree::process(child2_pid, child2_spec)).unwrap();
        tree.add_child(ProcessTree::process(child3_pid, child3_spec)).unwrap();

        // Test failure handling
        let pids_to_restart = tree.handle_child_failure(child2_pid).unwrap();

        match strategy {
            RestartStrategy::OneForOne => {
                // Only the failed child should be restarted
                assert_eq!(pids_to_restart.len(), 1);
                assert_eq!(pids_to_restart[0], child2_pid);
            }
            RestartStrategy::OneForAll => {
                // All children should be restarted
                assert_eq!(pids_to_restart.len(), 3);
                assert!(pids_to_restart.contains(&child1_pid));
                assert!(pids_to_restart.contains(&child2_pid));
                assert!(pids_to_restart.contains(&child3_pid));
            }
            RestartStrategy::RestForOne => {
                // Failed child and subsequent children should be restarted
                assert_eq!(pids_to_restart.len(), 2);
                assert!(pids_to_restart.contains(&child2_pid));
                assert!(pids_to_restart.contains(&child3_pid));
                assert!(!pids_to_restart.contains(&child1_pid));
            }
        }
    }
}

#[test]
fn test_complex_hierarchical_supervision() {
    // Create a complex supervision hierarchy
    // Root Supervisor
    //   ├── Worker 1
    //   ├── Sub-Supervisor A
    //   │   ├── Worker A1
    //   │   └── Worker A2
    //   └── Sub-Supervisor B
    //       ├── Worker B1
    //       ├── Worker B2
    //       └── Sub-Sub-Supervisor
    //           ├── Worker C1
    //           └── Worker C2

    // Create leaf workers
    let worker1_pid = Pid::new();
    let worker_a1_pid = Pid::new();
    let worker_a2_pid = Pid::new();
    let worker_b1_pid = Pid::new();
    let worker_b2_pid = Pid::new();
    let worker_c1_pid = Pid::new();
    let worker_c2_pid = Pid::new();

    // Create worker specs
    let worker1_spec = ChildSpec::new("worker1".to_string()).restart_policy(RestartPolicy::Permanent);
    let worker_a1_spec = ChildSpec::new("worker_a1".to_string()).restart_policy(RestartPolicy::Permanent);
    let worker_a2_spec = ChildSpec::new("worker_a2".to_string()).restart_policy(RestartPolicy::Transient);
    let worker_b1_spec = ChildSpec::new("worker_b1".to_string()).restart_policy(RestartPolicy::Permanent);
    let worker_b2_spec = ChildSpec::new("worker_b2".to_string()).restart_policy(RestartPolicy::Temporary);
    let worker_c1_spec = ChildSpec::new("worker_c1".to_string()).restart_policy(RestartPolicy::Permanent);
    let worker_c2_spec = ChildSpec::new("worker_c2".to_string()).restart_policy(RestartPolicy::Permanent);

    // Create sub-sub-supervisor
    let sub_sub_supervisor_spec = SupervisorSpec::new("sub_sub_supervisor".to_string())
        .strategy(RestartStrategy::OneForOne);
    let mut sub_sub_supervisor = ProcessTree::supervisor(Pid::new(), sub_sub_supervisor_spec);
    sub_sub_supervisor.add_child(ProcessTree::process(worker_c1_pid, worker_c1_spec)).unwrap();
    sub_sub_supervisor.add_child(ProcessTree::process(worker_c2_pid, worker_c2_spec)).unwrap();

    // Create sub-supervisor A
    let sub_supervisor_a_spec = SupervisorSpec::new("sub_supervisor_a".to_string())
        .strategy(RestartStrategy::OneForAll);
    let mut sub_supervisor_a = ProcessTree::supervisor(Pid::new(), sub_supervisor_a_spec);
    sub_supervisor_a.add_child(ProcessTree::process(worker_a1_pid, worker_a1_spec)).unwrap();
    sub_supervisor_a.add_child(ProcessTree::process(worker_a2_pid, worker_a2_spec)).unwrap();

    // Create sub-supervisor B
    let sub_supervisor_b_spec = SupervisorSpec::new("sub_supervisor_b".to_string())
        .strategy(RestartStrategy::RestForOne);
    let mut sub_supervisor_b = ProcessTree::supervisor(Pid::new(), sub_supervisor_b_spec);
    sub_supervisor_b.add_child(ProcessTree::process(worker_b1_pid, worker_b1_spec)).unwrap();
    sub_supervisor_b.add_child(ProcessTree::process(worker_b2_pid, worker_b2_spec)).unwrap();
    sub_supervisor_b.add_child(sub_sub_supervisor).unwrap();

    // Create root supervisor
    let root_spec = SupervisorSpec::new("root_supervisor".to_string())
        .strategy(RestartStrategy::OneForOne)
        .max_restarts(10)
        .restart_window(Duration::from_secs(120));
    let mut root_supervisor = ProcessTree::supervisor(Pid::new(), root_spec);
    root_supervisor.add_child(ProcessTree::process(worker1_pid, worker1_spec)).unwrap();
    root_supervisor.add_child(sub_supervisor_a).unwrap();
    root_supervisor.add_child(sub_supervisor_b).unwrap();

    // Test tree structure
    let all_pids = root_supervisor.all_pids();
    // 1 root + 1 worker1 + 1 sub_supervisor_a + 2 workers (a1, a2) +
    // 1 sub_supervisor_b + 2 workers (b1, b2) + 1 sub_sub_supervisor + 2 workers (c1, c2) = 11 total
    assert_eq!(all_pids.len(), 11);

    // Test finding processes at different levels
    assert!(root_supervisor.find_process(worker1_pid).is_some());
    assert!(root_supervisor.find_process(worker_a1_pid).is_some());
    assert!(root_supervisor.find_process(worker_b2_pid).is_some());
    assert!(root_supervisor.find_process(worker_c1_pid).is_some());

    // Test catamorphism - count all processes
    let total_count = root_supervisor.cata(&|node, children: Vec<usize>| {
        match node {
            ProcessTree::Process { .. } => 1,
            ProcessTree::Supervisor { .. } => 1 + children.iter().sum::<usize>(),
        }
    });
    assert_eq!(total_count, 11);
}

#[test]
fn test_supervisor_with_specs() {
    // Test creating supervisor from specification
    let child1_spec = ChildSpec::new("worker1".to_string())
        .restart_policy(RestartPolicy::Permanent)
        .shutdown_timeout(Duration::from_secs(10));
    
    let child2_spec = ChildSpec::new("worker2".to_string())
        .restart_policy(RestartPolicy::Transient)
        .shutdown_timeout(Duration::from_secs(5));

    let supervisor_spec = SupervisorSpec::new("test_supervisor".to_string())
        .strategy(RestartStrategy::OneForAll)
        .max_restarts(8)
        .restart_window(Duration::from_secs(180))
        .child(child1_spec.clone())
        .child(child2_spec.clone());

    let supervisor = Supervisor::from_spec(supervisor_spec);

    assert_eq!(supervisor.strategy(), RestartStrategy::OneForAll);
    
    // Test adding children with specifications
    let pid1 = Pid::new();
    let pid2 = Pid::new();

    // Create dummy processes for testing
    use ream::runtime::process::Process;
    use ream::types::Priority;
    use ream::runtime::actor::CounterActor;

    let actor1 = CounterActor::new(pid1, 0);
    let actor2 = CounterActor::new(pid2, 0);
    let process1 = Process::new(pid1, Box::new(actor1), Priority::Normal);
    let process2 = Process::new(pid2, Box::new(actor2), Priority::Normal);
    let handle1 = ProcessHandle::new(process1);
    let handle2 = ProcessHandle::new(process2);

    let mut supervisor = supervisor;
    supervisor.supervise_with_spec(pid1, handle1, child1_spec).unwrap();
    supervisor.supervise_with_spec(pid2, handle2, child2_spec).unwrap();

    assert_eq!(supervisor.child_count(), 2);
}
