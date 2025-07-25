//! Daemon runtime implementation
//!
//! Provides the core daemon functionality for running TLisp programs
//! in background mode with monitoring and management.

use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use std::process;
use tokio::time::interval;

#[cfg(unix)]
use daemonize::Daemonize;

use crate::error::{ReamResult, ReamError};
use crate::runtime::ReamRuntime;

use super::{DaemonConfig, DaemonManager, ActorInfo, ActorStatus};
use super::ipc::IpcServer;

/// Daemon runtime implementation
pub struct DaemonRuntime {
    /// Configuration
    config: DaemonConfig,
    /// Daemon manager
    manager: Arc<DaemonManager>,
    /// IPC server
    ipc_server: Option<IpcServer>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl DaemonRuntime {
    /// Create a new daemon runtime
    pub fn new(config: DaemonConfig) -> ReamResult<Self> {
        let manager = Arc::new(DaemonManager::new(config.clone())?);
        let ipc_server = Some(IpcServer::new(config.socket_path.clone(), manager.clone()));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(false));
        
        Ok(DaemonRuntime {
            config,
            manager,
            ipc_server,
            running,
        })
    }
    
    /// Start the daemon
    pub async fn start(&mut self, program_file: PathBuf) -> ReamResult<()> {
        // Check if daemon is already running
        if self.is_daemon_running()? {
            return Err(ReamError::Other("Daemon is already running".to_string()));
        }
        
        // Daemonize if not in foreground mode
        if !self.config.foreground {
            self.daemonize()?;
        }
        
        // Write PID file
        self.write_pid_file()?;
        
        // Set running flag
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // Start IPC server
        if let Some(mut ipc_server) = self.ipc_server.take() {
            let ipc_handle = tokio::spawn(async move {
                if let Err(e) = ipc_server.start().await {
                    eprintln!("IPC server error: {}", e);
                }
            });
        }
        
        // Load and run the TLisp program
        self.load_program(program_file).await?;

        // Start monitoring loop and wait for it
        self.run_main_loop().await?;

        Ok(())
    }
    
    /// Stop the daemon
    pub async fn stop(&mut self) -> ReamResult<()> {
        // Set running flag to false
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        
        // Stop IPC server
        if let Some(mut ipc_server) = self.ipc_server.take() {
            ipc_server.stop()?;
        }
        
        // Stop daemon manager
        self.manager.stop().await?;
        
        // Remove PID file
        self.remove_pid_file()?;
        
        Ok(())
    }
    
    /// Check if daemon is running
    pub fn is_daemon_running(&self) -> ReamResult<bool> {
        if !self.config.pid_file.exists() {
            return Ok(false);
        }
        
        let pid_str = fs::read_to_string(&self.config.pid_file)
            .map_err(|e| ReamError::Io(e))?;
        
        let pid: u32 = pid_str.trim().parse()
            .map_err(|e| ReamError::Other(format!("Invalid PID in file: {}", e)))?;
        
        // Check if process is still running
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            
            match kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                Ok(_) => Ok(true),
                Err(nix::errno::Errno::ESRCH) => Ok(false), // Process not found
                Err(nix::errno::Errno::EPERM) => Ok(true),  // Process exists but no permission
                Err(_) => Ok(false),
            }
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, just check if PID file exists
            Ok(true)
        }
    }
    
    /// Daemonize the process
    fn daemonize(&self) -> ReamResult<()> {
        #[cfg(unix)]
        {
            let daemonize = Daemonize::new()
                .pid_file(&self.config.pid_file)
                .working_directory("/tmp")
                .user("nobody")
                .group("daemon")
                .umask(0o027);

            match daemonize.start() {
                Ok(_) => Ok(()),
                Err(e) => Err(ReamError::Other(format!("Failed to daemonize: {}", e))),
            }
        }

        #[cfg(not(unix))]
        {
            Err(ReamError::NotImplemented("Daemonization not supported on Windows".to_string()))
        }
    }
    
    /// Write PID file
    fn write_pid_file(&self) -> ReamResult<()> {
        let pid = process::id();
        fs::write(&self.config.pid_file, pid.to_string())
            .map_err(|e| ReamError::Io(e))?;
        Ok(())
    }
    
    /// Remove PID file
    fn remove_pid_file(&self) -> ReamResult<()> {
        if self.config.pid_file.exists() {
            fs::remove_file(&self.config.pid_file)
                .map_err(|e| ReamError::Io(e))?;
        }
        Ok(())
    }
    
    /// Load and execute TLisp program
    async fn load_program(&self, program_file: PathBuf) -> ReamResult<()> {
        // Read the program file
        let program_content = fs::read_to_string(&program_file)
            .map_err(|e| ReamError::Io(e))?;

        println!("Loading TLisp program: {}", program_file.display());
        println!("Program content length: {} bytes", program_content.len());

        // Create TLisp runtime with REAM integration
        let mut tlisp_runtime = crate::tlisp::TlispRuntime::new();

        // Execute the program
        match tlisp_runtime.eval(&program_content) {
            Ok(result) => {
                println!("TLisp program executed successfully");
                println!("Result: {:?}", result);
            }
            Err(e) => {
                println!("TLisp program execution failed: {}", e);
                return Err(ReamError::Other(format!("TLisp execution failed: {}", e)));
            }
        }

        // The program should have spawned actors that are now running in the REAM runtime
        let process_count = self.manager.runtime.list_processes().len();
        println!("TLisp program spawned {} processes", process_count);

        Ok(())
    }
    
    /// Run the main daemon loop
    async fn run_main_loop(&self) -> ReamResult<()> {
        println!("Daemon entering main loop...");

        let mut interval = interval(self.config.monitor_interval);

        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            interval.tick().await;

            // Update actor information
            Self::update_actor_info(&self.manager).await;

            // Perform health checks
            Self::perform_health_checks(&self.manager).await;

            // Garbage collection if needed
            Self::perform_gc_if_needed(&self.manager).await;

            // Print status periodically (every 10 seconds)
            if interval.period().as_secs() >= 10 {
                let process_count = self.manager.runtime.list_processes().len();
                println!("Daemon running: {} processes active", process_count);
            }
        }

        println!("Daemon main loop exiting...");
        Ok(())
    }
    
    /// Update actor information
    async fn update_actor_info(manager: &Arc<DaemonManager>) {
        // TODO: Collect actor information from runtime
        // This should gather stats like memory usage, message rates, etc.
    }
    
    /// Perform health checks on actors
    async fn perform_health_checks(manager: &Arc<DaemonManager>) {
        // TODO: Check for crashed or unresponsive actors
        // Restart actors if needed based on supervision strategy
    }
    
    /// Perform garbage collection if needed
    async fn perform_gc_if_needed(manager: &Arc<DaemonManager>) {
        // TODO: Check memory usage and trigger GC if needed
    }
    
    /// Get daemon status
    pub fn get_status(&self) -> ReamResult<String> {
        if self.is_daemon_running()? {
            let system_info = self.manager.get_system_info();
            Ok(format!(
                "Daemon is running\n\
                 Uptime: {:?}\n\
                 Total actors: {}\n\
                 Active actors: {}\n\
                 Memory usage: {} bytes\n\
                 Message rate: {:.2} msg/s",
                system_info.uptime,
                system_info.total_actors,
                system_info.active_actors,
                system_info.total_memory,
                system_info.system_message_rate
            ))
        } else {
            Ok("Daemon is not running".to_string())
        }
    }
    
    /// Force kill daemon
    pub fn force_kill(&self) -> ReamResult<()> {
        if !self.config.pid_file.exists() {
            return Err(ReamError::Other("PID file not found".to_string()));
        }
        
        let pid_str = fs::read_to_string(&self.config.pid_file)
            .map_err(|e| ReamError::Io(e))?;
        
        let _pid: u32 = pid_str.trim().parse()
            .map_err(|e| ReamError::Other(format!("Invalid PID in file: {}", e)))?;
        
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            
            kill(Pid::from_raw(pid as i32), Signal::SIGKILL)
                .map_err(|e| ReamError::Other(format!("Failed to kill process: {}", e)))?;
        }
        
        #[cfg(not(unix))]
        {
            return Err(ReamError::Other("Force kill not supported on this platform".to_string()));
        }
        
        // Remove PID file
        self.remove_pid_file()?;
        
        Ok(())
    }
}
