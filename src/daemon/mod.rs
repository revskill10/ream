//! Daemon mode for REAM runtime
//!
//! Provides daemon functionality to run TLisp programs in background mode
//! with monitoring and management capabilities.

pub mod runtime;
pub mod ipc;
pub mod monitor;

#[cfg(feature = "tui")]
pub mod tui;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

use crate::types::{Pid, RuntimeStats};
use crate::error::{ReamResult, ReamError};
use crate::runtime::ReamRuntime;


/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Socket path for IPC
    pub socket_path: PathBuf,
    /// PID file path
    pub pid_file: PathBuf,
    /// Log file path
    pub log_file: PathBuf,
    /// Whether to run in foreground
    pub foreground: bool,
    /// Monitoring update interval
    pub monitor_interval: Duration,
    /// Maximum number of actors
    pub max_actors: usize,
    /// Memory limit per actor (bytes)
    pub memory_limit: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        #[cfg(unix)]
        let (socket_path, pid_file, log_file) = (
            PathBuf::from("/tmp/ream-daemon.sock"),
            PathBuf::from("/tmp/ream-daemon.pid"),
            PathBuf::from("/tmp/ream-daemon.log"),
        );

        #[cfg(windows)]
        let (socket_path, pid_file, log_file) = (
            std::env::temp_dir().join("ream-daemon.sock"),
            std::env::temp_dir().join("ream-daemon.pid"),
            std::env::temp_dir().join("ream-daemon.log"),
        );

        DaemonConfig {
            socket_path,
            pid_file,
            log_file,
            foreground: false,
            monitor_interval: Duration::from_millis(1000),
            max_actors: 10000,
            memory_limit: 64 * 1024 * 1024, // 64MB per actor
        }
    }
}

/// Actor information for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorInfo {
    /// Actor PID
    pub pid: Pid,
    /// Actor status
    pub status: ActorStatus,
    /// Mailbox size
    pub mailbox_size: usize,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Messages processed
    pub messages_processed: u64,
    /// Messages per second
    pub message_rate: f64,
    /// CPU time used (microseconds)
    pub cpu_time: u64,
    /// Actor uptime
    pub uptime: Duration,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Actor type/behavior name
    pub actor_type: String,
    /// Current state description
    pub state_description: String,
    /// Linked processes
    pub links: Vec<Pid>,
    /// Monitored processes
    pub monitors: Vec<Pid>,
    /// Parent supervisor
    pub supervisor: Option<Pid>,
}

/// Actor status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActorStatus {
    /// Actor is running normally
    Running,
    /// Actor is suspended
    Suspended,
    /// Actor is waiting for messages
    Waiting,
    /// Actor is processing a message
    Processing,
    /// Actor has crashed
    Crashed,
    /// Actor is being restarted
    Restarting,
    /// Actor is shutting down
    Terminating,
    /// Actor has terminated
    Terminated,
}

/// System-wide monitoring information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Runtime statistics
    pub runtime_stats: RuntimeStats,
    /// Total number of actors
    pub total_actors: usize,
    /// Active actors
    pub active_actors: usize,
    /// Suspended actors
    pub suspended_actors: usize,
    /// Crashed actors
    pub crashed_actors: usize,
    /// Total memory usage
    pub total_memory: usize,
    /// Total messages processed
    pub total_messages: u64,
    /// System message rate
    pub system_message_rate: f64,
    /// System uptime
    pub uptime: Duration,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage_percent: f64,
    /// Load average
    pub load_average: f64,
}

/// IPC message types for daemon communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonMessage {
    /// Get system information
    GetSystemInfo,
    /// Get list of all actors
    ListActors { detailed: bool },
    /// Get specific actor information
    GetActorInfo { pid: String },
    /// Kill an actor
    KillActor { pid: String, reason: String },
    /// Suspend an actor
    SuspendActor { pid: String },
    /// Resume an actor
    ResumeActor { pid: String },
    /// Restart an actor
    RestartActor { pid: String },
    /// Send message to actor
    SendMessage { pid: String, message: String },
    /// Shutdown daemon
    Shutdown,
    /// Ping daemon
    Ping,
}

/// IPC response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonResponse {
    /// System information response
    SystemInfo(SystemInfo),
    /// Actor list response
    ActorList(Vec<ActorInfo>),
    /// Actor information response
    ActorInfo(ActorInfo),
    /// Operation success
    Success(String),
    /// Operation error
    Error(String),
    /// Pong response
    Pong,
}

/// Daemon runtime manager
pub struct DaemonManager {
    /// Configuration
    config: DaemonConfig,
    /// REAM runtime
    runtime: Arc<ReamRuntime>,
    /// Actor information cache
    actors: Arc<RwLock<std::collections::HashMap<Pid, ActorInfo>>>,
    /// System start time
    start_time: Instant,
    /// IPC command channel
    command_tx: mpsc::UnboundedSender<DaemonMessage>,
    command_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<DaemonMessage>>>>,
    /// Response channel
    response_tx: Arc<RwLock<Option<mpsc::UnboundedSender<DaemonResponse>>>>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl DaemonManager {
    /// Create a new daemon manager
    pub fn new(config: DaemonConfig) -> ReamResult<Self> {
        let runtime = Arc::new(ReamRuntime::new()?);
        let actors = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let start_time = Instant::now();

        let (command_tx, command_rx) = mpsc::unbounded_channel();

        Ok(DaemonManager {
            config,
            runtime,
            actors,
            start_time,
            command_tx,
            command_rx: Arc::new(RwLock::new(Some(command_rx))),
            response_tx: Arc::new(RwLock::new(None)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }
    
    /// Start the daemon
    pub async fn start(&self, program_file: PathBuf) -> ReamResult<()> {
        // Set running flag
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        // Start the REAM runtime
        self.runtime.start()?;

        // Start monitoring loop
        self.start_monitoring_loop().await?;

        Ok(())
    }

    /// Stop the daemon
    pub async fn stop(&self) -> ReamResult<()> {
        // Set running flag to false
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);

        // Stop the REAM runtime
        self.runtime.shutdown()?;

        Ok(())
    }

    /// Start the monitoring loop
    async fn start_monitoring_loop(&self) -> ReamResult<()> {
        let runtime = self.runtime.clone();
        let actors = self.actors.clone();
        let running = self.running.clone();
        let interval = self.config.monitor_interval;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            while running.load(std::sync::atomic::Ordering::SeqCst) {
                interval_timer.tick().await;

                // Update actor information from runtime
                Self::update_actor_cache(&runtime, &actors).await;
            }
        });

        Ok(())
    }

    /// Update actor cache with current runtime data
    async fn update_actor_cache(
        runtime: &Arc<ReamRuntime>,
        actors: &Arc<RwLock<std::collections::HashMap<Pid, ActorInfo>>>,
    ) {
        let mut actor_cache = actors.write().unwrap();
        actor_cache.clear();

        // Get all processes from runtime
        let processes = runtime.list_process_handles();

        for (pid, process_handle) in processes {
            let process_info = process_handle.info();
            let mailbox = process_handle.mailbox();
            let mailbox_size = mailbox.read().unwrap().len();

            let actor_info = ActorInfo {
                pid,
                status: Self::map_process_state(process_info.state),
                mailbox_size,
                memory_usage: process_info.memory_usage,
                messages_processed: 0, // TODO: Add to ProcessInfo
                message_rate: 0.0, // TODO: Calculate from stats
                cpu_time: process_info.cpu_time,
                uptime: process_handle.uptime(),
                last_activity: SystemTime::now(),
                actor_type: "ReamActor".to_string(), // TODO: Add actor type to ProcessInfo
                state_description: format!("{:?}", process_info.state),
                links: process_info.links,
                monitors: process_info.monitors,
                supervisor: process_info.parent,
            };

            actor_cache.insert(pid, actor_info);
        }
    }

    /// Map process state to actor status
    fn map_process_state(state: crate::types::ProcessState) -> ActorStatus {
        match state {
            crate::types::ProcessState::Running => ActorStatus::Running,
            crate::types::ProcessState::Suspended => ActorStatus::Suspended,
            crate::types::ProcessState::Waiting => ActorStatus::Waiting,
            crate::types::ProcessState::Terminated => ActorStatus::Terminated,
        }
    }
    
    /// Get system information
    pub fn get_system_info(&self) -> SystemInfo {
        let actors = self.actors.read().unwrap();
        let runtime_stats = self.runtime.stats();
        
        let total_actors = actors.len();
        let active_actors = actors.values().filter(|a| a.status == ActorStatus::Running).count();
        let suspended_actors = actors.values().filter(|a| a.status == ActorStatus::Suspended).count();
        let crashed_actors = actors.values().filter(|a| a.status == ActorStatus::Crashed).count();
        
        let total_memory = actors.values().map(|a| a.memory_usage).sum();
        let total_messages = actors.values().map(|a| a.messages_processed).sum();
        let system_message_rate = actors.values().map(|a| a.message_rate).sum();
        
        SystemInfo {
            runtime_stats,
            total_actors,
            active_actors,
            suspended_actors,
            crashed_actors,
            total_memory,
            total_messages,
            system_message_rate,
            uptime: self.start_time.elapsed(),
            cpu_usage: 0.0, // TODO: Implement CPU monitoring
            memory_usage_percent: 0.0, // TODO: Implement memory monitoring
            load_average: 0.0, // TODO: Implement load monitoring
        }
    }
    
    /// Get all actors
    pub fn list_actors(&self, detailed: bool) -> Vec<ActorInfo> {
        let actors = self.actors.read().unwrap();
        actors.values().cloned().collect()
    }
    
    /// Get specific actor information
    pub fn get_actor_info(&self, pid_str: &str) -> ReamResult<ActorInfo> {
        let pid = Pid::from_string(pid_str)
            .map_err(|_| ReamError::Other(format!("Invalid PID: {}", pid_str)))?;

        let actors = self.actors.read().unwrap();
        actors.get(&pid)
            .cloned()
            .ok_or_else(|| ReamError::Other(format!("Actor {} not found", pid_str)))
    }

    /// Kill an actor
    pub fn kill_actor(&self, pid_str: &str, reason: &str) -> ReamResult<String> {
        let pid = Pid::from_string(pid_str)
            .map_err(|_| ReamError::Other(format!("Invalid PID: {}", pid_str)))?;

        if let Some(process_handle) = self.runtime.get_process(pid) {
            process_handle.terminate()
                .map_err(|e| ReamError::Other(format!("Failed to terminate process: {}", e)))?;

            // Update actor cache
            let mut actors = self.actors.write().unwrap();
            if let Some(actor_info) = actors.get_mut(&pid) {
                actor_info.status = ActorStatus::Terminated;
            }

            Ok(format!("Actor {} terminated with reason: {}", pid_str, reason))
        } else {
            Err(ReamError::Other(format!("Actor {} not found", pid_str)))
        }
    }

    /// Suspend an actor
    pub fn suspend_actor(&self, pid_str: &str) -> ReamResult<String> {
        let pid = Pid::from_string(pid_str)
            .map_err(|_| ReamError::Other(format!("Invalid PID: {}", pid_str)))?;

        if let Some(process_handle) = self.runtime.get_process(pid) {
            process_handle.suspend()
                .map_err(|e| ReamError::Other(format!("Failed to suspend process: {}", e)))?;

            // Update actor cache
            let mut actors = self.actors.write().unwrap();
            if let Some(actor_info) = actors.get_mut(&pid) {
                actor_info.status = ActorStatus::Suspended;
            }

            Ok(format!("Actor {} suspended", pid_str))
        } else {
            Err(ReamError::Other(format!("Actor {} not found", pid_str)))
        }
    }

    /// Resume an actor
    pub fn resume_actor(&self, pid_str: &str) -> ReamResult<String> {
        let pid = Pid::from_string(pid_str)
            .map_err(|_| ReamError::Other(format!("Invalid PID: {}", pid_str)))?;

        if let Some(process_handle) = self.runtime.get_process(pid) {
            process_handle.resume()
                .map_err(|e| ReamError::Other(format!("Failed to resume process: {}", e)))?;

            // Update actor cache
            let mut actors = self.actors.write().unwrap();
            if let Some(actor_info) = actors.get_mut(&pid) {
                actor_info.status = ActorStatus::Running;
            }

            Ok(format!("Actor {} resumed", pid_str))
        } else {
            Err(ReamError::Other(format!("Actor {} not found", pid_str)))
        }
    }

    /// Restart an actor
    pub fn restart_actor(&self, pid_str: &str) -> ReamResult<String> {
        let pid = Pid::from_string(pid_str)
            .map_err(|_| ReamError::Other(format!("Invalid PID: {}", pid_str)))?;

        if let Some(process_handle) = self.runtime.get_process(pid) {
            process_handle.restart()
                .map_err(|e| ReamError::Other(format!("Failed to restart process: {}", e)))?;

            // Update actor cache
            let mut actors = self.actors.write().unwrap();
            if let Some(actor_info) = actors.get_mut(&pid) {
                actor_info.status = ActorStatus::Running;
            }

            Ok(format!("Actor {} restarted", pid_str))
        } else {
            Err(ReamError::Other(format!("Actor {} not found", pid_str)))
        }
    }

    /// Send a message to an actor
    pub fn send_message(&self, pid_str: &str, message: &str) -> ReamResult<String> {
        let pid = Pid::from_string(pid_str)
            .map_err(|_| ReamError::Other(format!("Invalid PID: {}", pid_str)))?;

        if let Some(process_handle) = self.runtime.get_process(pid) {
            // Parse the message as TLisp and send it
            let mailbox = process_handle.mailbox();
            let mut mb = mailbox.write().unwrap();
            mb.send(crate::types::MessagePayload::Text(message.to_string()));

            Ok(format!("Message sent to actor {}: {}", pid_str, message))
        } else {
            Err(ReamError::Other(format!("Actor {} not found", pid_str)))
        }
    }
}
