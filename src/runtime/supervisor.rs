//! Supervision trees as initial algebra of process hierarchies

use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::types::{Pid, RestartStrategy};
use crate::error::{RuntimeError, RuntimeResult};
use crate::runtime::process::ProcessHandle;


/// Restart policy for child processes
#[derive(Debug, Clone, PartialEq)]
pub enum RestartPolicy {
    /// Always restart the child
    Permanent,
    /// Restart only if child exits abnormally
    Transient,
    /// Never restart the child
    Temporary,
}

/// Child specification for supervision
#[derive(Debug, Clone)]
pub struct ChildSpec {
    /// Child identifier
    pub id: String,
    /// Restart policy
    pub restart_policy: RestartPolicy,
    /// Shutdown timeout
    pub shutdown_timeout: Duration,
    /// Child type (worker or supervisor)
    pub child_type: ChildType,
    /// Maximum restart intensity
    pub max_restart_intensity: u32,
}

/// Type of child process
#[derive(Debug, Clone, PartialEq)]
pub enum ChildType {
    /// Worker process
    Worker,
    /// Supervisor process
    Supervisor,
}

/// Supervisor specification
#[derive(Debug, Clone)]
pub struct SupervisorSpec {
    /// Supervisor name
    pub name: String,
    /// Restart strategy
    pub strategy: RestartStrategy,
    /// Maximum restarts allowed
    pub max_restarts: u32,
    /// Time window for restart counting
    pub restart_window: Duration,
    /// Child specifications
    pub children: Vec<ChildSpec>,
}

/// Supervisor state for tracking restarts
#[derive(Debug, Clone)]
pub struct SupervisorState {
    /// Restart count within current window
    pub restart_count: u32,
    /// Window start time
    pub window_start: Instant,
    /// Whether supervisor is shutting down
    pub shutting_down: bool,
}

impl ChildSpec {
    /// Create a new child specification
    pub fn new(id: String) -> Self {
        ChildSpec {
            id,
            restart_policy: RestartPolicy::Permanent,
            shutdown_timeout: Duration::from_secs(5),
            child_type: ChildType::Worker,
            max_restart_intensity: 5,
        }
    }

    /// Set restart policy
    pub fn restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }

    /// Set shutdown timeout
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Set child type
    pub fn child_type(mut self, child_type: ChildType) -> Self {
        self.child_type = child_type;
        self
    }

    /// Set maximum restart intensity
    pub fn max_restart_intensity(mut self, intensity: u32) -> Self {
        self.max_restart_intensity = intensity;
        self
    }
}

impl SupervisorSpec {
    /// Create a new supervisor specification
    pub fn new(name: String) -> Self {
        SupervisorSpec {
            name,
            strategy: RestartStrategy::OneForOne,
            max_restarts: 5,
            restart_window: Duration::from_secs(60),
            children: Vec::new(),
        }
    }

    /// Set restart strategy
    pub fn strategy(mut self, strategy: RestartStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set maximum restarts
    pub fn max_restarts(mut self, max_restarts: u32) -> Self {
        self.max_restarts = max_restarts;
        self
    }

    /// Set restart window
    pub fn restart_window(mut self, window: Duration) -> Self {
        self.restart_window = window;
        self
    }

    /// Add a child specification
    pub fn child(mut self, child: ChildSpec) -> Self {
        self.children.push(child);
        self
    }
}

impl SupervisorState {
    /// Create a new supervisor state
    pub fn new() -> Self {
        SupervisorState {
            restart_count: 0,
            window_start: Instant::now(),
            shutting_down: false,
        }
    }

    /// Check if restart is allowed within the window
    pub fn can_restart(&self, max_restarts: u32, window: Duration) -> bool {
        if self.shutting_down {
            return false;
        }

        let now = Instant::now();
        if now.duration_since(self.window_start) > window {
            // Window has expired, reset is allowed
            true
        } else {
            // Within window, check restart count
            self.restart_count < max_restarts
        }
    }

    /// Record a restart
    pub fn record_restart(&mut self, window: Duration) {
        let now = Instant::now();
        if now.duration_since(self.window_start) > window {
            // Reset window
            self.window_start = now;
            self.restart_count = 1;
        } else {
            self.restart_count += 1;
        }
    }

    /// Start shutdown process
    pub fn start_shutdown(&mut self) {
        self.shutting_down = true;
    }
}

/// Process tree as algebraic data type
#[derive(Debug, Clone)]
pub enum ProcessTree {
    /// Leaf process with child specification
    Process {
        pid: Pid,
        spec: ChildSpec,
    },
    /// Supervisor with children and specification
    Supervisor {
        pid: Pid,
        spec: SupervisorSpec,
        children: Vec<ProcessTree>,
        state: SupervisorState,
    },
}

impl ProcessTree {
    /// Create a new process leaf
    pub fn process(pid: Pid, spec: ChildSpec) -> Self {
        ProcessTree::Process { pid, spec }
    }

    /// Create a new supervisor
    pub fn supervisor(pid: Pid, spec: SupervisorSpec) -> Self {
        ProcessTree::Supervisor {
            pid,
            spec,
            children: Vec::new(),
            state: SupervisorState::new(),
        }
    }

    /// Add a child to a supervisor
    pub fn add_child(&mut self, child: ProcessTree) -> RuntimeResult<()> {
        match self {
            ProcessTree::Supervisor { children, .. } => {
                children.push(child);
                Ok(())
            }
            ProcessTree::Process { .. } => {
                Err(RuntimeError::Supervision("Cannot add child to process".to_string()))
            }
        }
    }

    /// Get the PID of this tree node
    pub fn pid(&self) -> Pid {
        match self {
            ProcessTree::Process { pid, .. } => *pid,
            ProcessTree::Supervisor { pid, .. } => *pid,
        }
    }

    /// Check if this tree node can restart
    pub fn can_restart(&self) -> bool {
        match self {
            ProcessTree::Process { spec, .. } => {
                spec.restart_policy != RestartPolicy::Temporary
            }
            ProcessTree::Supervisor { state, spec, .. } => {
                state.can_restart(spec.max_restarts, spec.restart_window)
            }
        }
    }

    /// Handle child failure according to restart strategy
    pub fn handle_child_failure(&mut self, failed_pid: Pid) -> RuntimeResult<Vec<Pid>> {
        match self {
            ProcessTree::Process { .. } => {
                Err(RuntimeError::Supervision("Process cannot handle child failures".to_string()))
            }
            ProcessTree::Supervisor { children, spec, state, .. } => {
                if !state.can_restart(spec.max_restarts, spec.restart_window) {
                    return Err(RuntimeError::Supervision("Restart limit exceeded".to_string()));
                }

                let mut pids_to_restart = Vec::new();

                match spec.strategy {
                    RestartStrategy::OneForOne => {
                        // Restart only the failed child
                        if let Some(child) = children.iter().find(|c| c.pid() == failed_pid) {
                            if child.can_restart() {
                                pids_to_restart.push(failed_pid);
                            }
                        }
                    }
                    RestartStrategy::OneForAll => {
                        // Restart all children
                        for child in children.iter() {
                            if child.can_restart() {
                                pids_to_restart.push(child.pid());
                            }
                        }
                    }
                    RestartStrategy::RestForOne => {
                        // Restart failed child and all subsequent children
                        let mut found_failed = false;
                        for child in children.iter() {
                            if child.pid() == failed_pid {
                                found_failed = true;
                            }
                            if found_failed && child.can_restart() {
                                pids_to_restart.push(child.pid());
                            }
                        }
                    }
                }

                if !pids_to_restart.is_empty() {
                    state.record_restart(spec.restart_window);
                }

                Ok(pids_to_restart)
            }
        }
    }
    
    /// Get all PIDs in the tree
    pub fn all_pids(&self) -> Vec<Pid> {
        match self {
            ProcessTree::Process { pid, .. } => vec![*pid],
            ProcessTree::Supervisor { pid, children, .. } => {
                let mut pids = vec![*pid];
                for child in children {
                    pids.extend(child.all_pids());
                }
                pids
            }
        }
    }
    
    /// Find a process in the tree
    pub fn find_process(&self, target: Pid) -> Option<&ProcessTree> {
        match self {
            ProcessTree::Process { pid, .. } if *pid == target => Some(self),
            ProcessTree::Process { .. } => None,
            ProcessTree::Supervisor { pid, children, .. } => {
                if *pid == target {
                    Some(self)
                } else {
                    children.iter().find_map(|child| child.find_process(target))
                }
            }
        }
    }
    
    /// Catamorphic fold over the tree
    pub fn cata<F, T>(&self, f: &F) -> T
    where
        F: Fn(&ProcessTree, Vec<T>) -> T,
    {
        match self {
            ProcessTree::Process { .. } => f(self, Vec::new()),
            ProcessTree::Supervisor { children, .. } => {
                let child_results: Vec<T> = children.iter().map(|child| child.cata(f)).collect();
                f(self, child_results)
            }
        }
    }
}

/// Child process information
#[derive(Clone)]
struct ChildInfo {
    handle: ProcessHandle,
    restart_count: u32,
    last_restart: Option<Instant>,
    max_restarts: u32,
    restart_window: Duration,
}

impl ChildInfo {
    fn new(handle: ProcessHandle) -> Self {
        ChildInfo {
            handle,
            restart_count: 0,
            last_restart: None,
            max_restarts: 5,
            restart_window: Duration::from_secs(60),
        }
    }
    
    fn can_restart(&self) -> bool {
        if let Some(last) = self.last_restart {
            if last.elapsed() > self.restart_window {
                // Reset restart count after window
                return true;
            }
            self.restart_count < self.max_restarts
        } else {
            true
        }
    }
    
    fn record_restart(&mut self) {
        self.restart_count += 1;
        self.last_restart = Some(Instant::now());
    }
}

/// Supervisor for managing child processes
pub struct Supervisor {
    /// Supervisor PID
    pid: Pid,
    
    /// Child processes
    children: HashMap<Pid, ChildInfo>,
    
    /// Restart strategy
    strategy: RestartStrategy,
    
    /// Process tree
    tree: ProcessTree,
    
    /// Supervisor statistics
    stats: SupervisorStats,
}

#[derive(Debug, Default, Clone)]
struct SupervisorStats {
    children_started: u32,
    children_terminated: u32,
    restarts_performed: u32,
    restart_failures: u32,
}

impl Supervisor {
    /// Create a new supervisor
    pub fn new(strategy: RestartStrategy) -> Self {
        let pid = Pid::new();
        let spec = SupervisorSpec::new("supervisor".to_string()).strategy(strategy);
        Supervisor {
            pid,
            children: HashMap::new(),
            strategy,
            tree: ProcessTree::supervisor(pid, spec),
            stats: SupervisorStats::default(),
        }
    }

    /// Create a new supervisor from specification
    pub fn from_spec(spec: SupervisorSpec) -> Self {
        let pid = Pid::new();
        let strategy = spec.strategy;
        Supervisor {
            pid,
            children: HashMap::new(),
            strategy,
            tree: ProcessTree::supervisor(pid, spec),
            stats: SupervisorStats::default(),
        }
    }
    
    /// Get supervisor PID
    pub fn pid(&self) -> Pid {
        self.pid
    }
    
    /// Add a child process to supervision
    pub fn supervise(&mut self, pid: Pid, handle: ProcessHandle) -> RuntimeResult<()> {
        let child_info = ChildInfo::new(handle);
        self.children.insert(pid, child_info);

        // Add to process tree with default child spec
        let child_spec = ChildSpec::new(format!("child_{}", pid));
        self.tree.add_child(ProcessTree::process(pid, child_spec))?;

        self.stats.children_started += 1;
        Ok(())
    }

    /// Add a child process with specification
    pub fn supervise_with_spec(&mut self, pid: Pid, handle: ProcessHandle, spec: ChildSpec) -> RuntimeResult<()> {
        let child_info = ChildInfo::new(handle);
        self.children.insert(pid, child_info);

        // Add to process tree
        self.tree.add_child(ProcessTree::process(pid, spec))?;

        self.stats.children_started += 1;
        Ok(())
    }
    
    /// Remove a child from supervision
    pub fn unsupervise(&mut self, pid: Pid) -> RuntimeResult<()> {
        if self.children.remove(&pid).is_some() {
            self.stats.children_terminated += 1;
            Ok(())
        } else {
            Err(RuntimeError::ProcessNotFound(pid))
        }
    }
    
    /// Handle child process failure
    pub fn handle_child_failure(&mut self, failed_pid: Pid) -> RuntimeResult<bool> {
        let child_info = self.children.get_mut(&failed_pid)
            .ok_or(RuntimeError::ProcessNotFound(failed_pid))?;
        
        if !child_info.can_restart() {
            self.stats.restart_failures += 1;
            return Ok(false);
        }
        
        match self.strategy {
            RestartStrategy::OneForOne => {
                self.restart_child(failed_pid)?;
            }
            RestartStrategy::OneForAll => {
                self.restart_all_children()?;
            }
            RestartStrategy::RestForOne => {
                self.restart_from_child(failed_pid)?;
            }
        }
        
        self.stats.restarts_performed += 1;
        Ok(true)
    }
    
    /// Get list of supervised children
    pub fn children(&self) -> Vec<Pid> {
        self.children.keys().copied().collect()
    }
    
    /// Get child count
    pub fn child_count(&self) -> usize {
        self.children.len()
    }
    
    /// Get supervisor statistics
    pub fn stats(&self) -> &SupervisorStats {
        &self.stats
    }

    /// Get the restart strategy
    pub fn strategy(&self) -> RestartStrategy {
        self.strategy
    }
    
    /// Get process tree
    pub fn tree(&self) -> &ProcessTree {
        &self.tree
    }
    
    /// Check if a process is supervised
    pub fn is_supervised(&self, pid: Pid) -> bool {
        self.children.contains_key(&pid)
    }
    
    // Private helper methods
    
    fn restart_child(&mut self, pid: Pid) -> RuntimeResult<()> {
        if let Some(child_info) = self.children.get_mut(&pid) {
            child_info.handle.restart()?;
            child_info.record_restart();
        }
        Ok(())
    }
    
    fn restart_all_children(&mut self) -> RuntimeResult<()> {
        let pids: Vec<Pid> = self.children.keys().copied().collect();
        for pid in pids {
            self.restart_child(pid)?;
        }
        Ok(())
    }
    
    fn restart_from_child(&mut self, failed_pid: Pid) -> RuntimeResult<()> {
        // Find the position of the failed child
        let pids: Vec<Pid> = self.children.keys().copied().collect();
        if let Some(pos) = pids.iter().position(|&pid| pid == failed_pid) {
            // Restart from this position onwards
            for &pid in &pids[pos..] {
                self.restart_child(pid)?;
            }
        }
        Ok(())
    }
}

/// Supervisor builder for creating supervision trees
pub struct SupervisorBuilder {
    strategy: RestartStrategy,
    max_restarts: u32,
    restart_window: Duration,
}

impl SupervisorBuilder {
    /// Create a new supervisor builder
    pub fn new() -> Self {
        SupervisorBuilder {
            strategy: RestartStrategy::OneForOne,
            max_restarts: 5,
            restart_window: Duration::from_secs(60),
        }
    }
    
    /// Set restart strategy
    pub fn strategy(mut self, strategy: RestartStrategy) -> Self {
        self.strategy = strategy;
        self
    }
    
    /// Set maximum restarts
    pub fn max_restarts(mut self, max_restarts: u32) -> Self {
        self.max_restarts = max_restarts;
        self
    }
    
    /// Set restart window
    pub fn restart_window(mut self, window: Duration) -> Self {
        self.restart_window = window;
        self
    }
    
    /// Build the supervisor
    pub fn build(self) -> Supervisor {
        Supervisor::new(self.strategy)
    }
}

impl Default for SupervisorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::actor::CounterActor;
    use crate::runtime::process::Process;

    #[test]
    fn test_process_tree() {
        let spec = SupervisorSpec::new("test_supervisor".to_string()).strategy(RestartStrategy::OneForOne);
        let mut tree = ProcessTree::supervisor(Pid::new(), spec);
        let child_pid = Pid::new();
        let child_spec = ChildSpec::new("test_child".to_string());

        tree.add_child(ProcessTree::process(child_pid, child_spec)).unwrap();

        let pids = tree.all_pids();
        assert_eq!(pids.len(), 2);
        assert!(pids.contains(&child_pid));
    }
    
    #[test]
    fn test_supervisor_basic() {
        let mut supervisor = Supervisor::new(RestartStrategy::OneForOne);
        let pid = Pid::new();
        
        let actor = CounterActor::new(pid, 0);
        let process = Process::new(pid, Box::new(actor), crate::types::Priority::Normal);
        let handle = crate::runtime::process::ProcessHandle::new(process);
        
        supervisor.supervise(pid, handle).unwrap();
        
        assert_eq!(supervisor.child_count(), 1);
        assert!(supervisor.is_supervised(pid));
    }
    
    #[test]
    fn test_supervisor_tree_operations() {
        let spec = SupervisorSpec::new("test_supervisor".to_string()).strategy(RestartStrategy::OneForOne);
        let tree = ProcessTree::supervisor(Pid::new(), spec);

        // Test catamorphism - count processes
        let count = tree.cata(&|node, children: Vec<usize>| {
            match node {
                ProcessTree::Process { .. } => 1,
                ProcessTree::Supervisor { .. } => 1 + children.iter().sum::<usize>(),
            }
        });

        assert_eq!(count, 1); // Just the supervisor
    }

    #[test]
    fn test_child_spec_builder() {
        let spec = ChildSpec::new("test_child".to_string())
            .restart_policy(RestartPolicy::Transient)
            .shutdown_timeout(Duration::from_secs(10))
            .child_type(ChildType::Worker)
            .max_restart_intensity(3);

        assert_eq!(spec.id, "test_child");
        assert_eq!(spec.restart_policy, RestartPolicy::Transient);
        assert_eq!(spec.shutdown_timeout, Duration::from_secs(10));
        assert_eq!(spec.child_type, ChildType::Worker);
        assert_eq!(spec.max_restart_intensity, 3);
    }

    #[test]
    fn test_supervisor_spec_builder() {
        let child1 = ChildSpec::new("child1".to_string()).restart_policy(RestartPolicy::Permanent);
        let child2 = ChildSpec::new("child2".to_string()).restart_policy(RestartPolicy::Transient);

        let spec = SupervisorSpec::new("test_supervisor".to_string())
            .strategy(RestartStrategy::OneForAll)
            .max_restarts(10)
            .restart_window(Duration::from_secs(120))
            .child(child1)
            .child(child2);

        assert_eq!(spec.name, "test_supervisor");
        assert_eq!(spec.strategy, RestartStrategy::OneForAll);
        assert_eq!(spec.max_restarts, 10);
        assert_eq!(spec.restart_window, Duration::from_secs(120));
        assert_eq!(spec.children.len(), 2);
    }

    #[test]
    fn test_supervisor_state_restart_limits() {
        let mut state = SupervisorState::new();
        let window = Duration::from_secs(60);
        let max_restarts = 3;

        // Should allow restarts initially
        assert!(state.can_restart(max_restarts, window));

        // Record restarts up to the limit
        for _ in 0..max_restarts {
            state.record_restart(window);
            if state.restart_count < max_restarts {
                assert!(state.can_restart(max_restarts, window));
            }
        }

        // Should not allow more restarts within window
        assert!(!state.can_restart(max_restarts, window));

        // Start shutdown
        state.start_shutdown();
        assert!(!state.can_restart(max_restarts, window));
    }

    #[test]
    fn test_restart_strategies() {
        let spec = SupervisorSpec::new("test_supervisor".to_string())
            .strategy(RestartStrategy::OneForOne);
        let mut tree = ProcessTree::supervisor(Pid::new(), spec);

        let child1_pid = Pid::new();
        let child2_pid = Pid::new();
        let child1_spec = ChildSpec::new("child1".to_string()).restart_policy(RestartPolicy::Permanent);
        let child2_spec = ChildSpec::new("child2".to_string()).restart_policy(RestartPolicy::Permanent);

        tree.add_child(ProcessTree::process(child1_pid, child1_spec)).unwrap();
        tree.add_child(ProcessTree::process(child2_pid, child2_spec)).unwrap();

        // Test OneForOne strategy
        let pids_to_restart = tree.handle_child_failure(child1_pid).unwrap();
        assert_eq!(pids_to_restart.len(), 1);
        assert_eq!(pids_to_restart[0], child1_pid);
    }

    #[test]
    fn test_hierarchical_supervision() {
        // Create a hierarchical supervision tree
        let child_spec = ChildSpec::new("worker".to_string()).restart_policy(RestartPolicy::Permanent);
        let worker_pid = Pid::new();
        let worker = ProcessTree::process(worker_pid, child_spec);

        let sub_supervisor_spec = SupervisorSpec::new("sub_supervisor".to_string())
            .strategy(RestartStrategy::OneForOne);
        let mut sub_supervisor = ProcessTree::supervisor(Pid::new(), sub_supervisor_spec);
        sub_supervisor.add_child(worker).unwrap();

        let root_spec = SupervisorSpec::new("root_supervisor".to_string())
            .strategy(RestartStrategy::OneForAll);
        let mut root_supervisor = ProcessTree::supervisor(Pid::new(), root_spec);
        root_supervisor.add_child(sub_supervisor).unwrap();

        // Test tree structure
        let all_pids = root_supervisor.all_pids();
        assert_eq!(all_pids.len(), 3); // root + sub_supervisor + worker

        // Test finding processes
        assert!(root_supervisor.find_process(worker_pid).is_some());
    }
}
