//! Inter-process communication for daemon mode
//!
//! Provides platform-specific IPC communication between daemon and client processes.

use std::path::PathBuf;
use crate::error::{ReamResult, ReamError};
use super::{DaemonMessage, DaemonResponse, DaemonManager};

#[cfg(unix)]
pub use unix_impl::*;

#[cfg(not(unix))]
pub use windows_impl::*;

#[cfg(unix)]
mod unix_impl {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use serde_json;

/// IPC server for daemon communication
pub struct IpcServer {
    /// Socket path
    socket_path: PathBuf,
    /// Unix domain socket listener
    listener: Option<UnixListener>,
    /// Daemon manager reference
    daemon: std::sync::Arc<DaemonManager>,
}

impl IpcServer {
    /// Create a new IPC server
    pub fn new(socket_path: PathBuf, daemon: std::sync::Arc<DaemonManager>) -> Self {
        IpcServer {
            socket_path,
            listener: None,
            daemon,
        }
    }
    
    /// Start the IPC server
    pub async fn start(&mut self) -> ReamResult<()> {
        // Remove existing socket file if it exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| ReamError::Io(e))?;
        }
        
        // Create Unix domain socket listener
        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| ReamError::Io(e))?;
        
        self.listener = Some(listener);
        
        // Start accepting connections
        self.accept_connections().await
    }
    
    /// Accept and handle incoming connections
    async fn accept_connections(&self) -> ReamResult<()> {
        let listener = self.listener.as_ref()
            .ok_or_else(|| ReamError::Other("Listener not initialized".to_string()))?;
        
        loop {
            match listener.accept() {
                Ok((stream, _addr)) => {
                    let daemon = self.daemon.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, daemon).await {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle a client connection
    async fn handle_client(
        mut stream: UnixStream,
        daemon: std::sync::Arc<DaemonManager>,
    ) -> ReamResult<()> {
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer)
            .map_err(|e| ReamError::Io(e))?;
        
        // Parse the message
        let message: DaemonMessage = serde_json::from_str(&buffer)
            .map_err(|e| ReamError::Other(format!("Failed to parse message: {}", e)))?;
        
        // Process the message
        let response = Self::process_message(message, &daemon).await?;
        
        // Send response
        let response_json = serde_json::to_string(&response)
            .map_err(|e| ReamError::Other(format!("Failed to serialize response: {}", e)))?;
        
        stream.write_all(response_json.as_bytes())
            .map_err(|e| ReamError::Io(e))?;
        
        Ok(())
    }
    
    /// Process a daemon message and return response
    async fn process_message(
        message: DaemonMessage,
        daemon: &DaemonManager,
    ) -> ReamResult<DaemonResponse> {
        match message {
            DaemonMessage::GetSystemInfo => {
                let info = daemon.get_system_info();
                Ok(DaemonResponse::SystemInfo(info))
            }
            DaemonMessage::ListActors { detailed } => {
                let actors = daemon.list_actors(detailed);
                Ok(DaemonResponse::ActorList(actors))
            }
            DaemonMessage::GetActorInfo { pid } => {
                match daemon.get_actor_info(&pid) {
                    Ok(info) => Ok(DaemonResponse::ActorInfo(info)),
                    Err(e) => Ok(DaemonResponse::Error(e.to_string())),
                }
            }
            DaemonMessage::KillActor { pid, reason } => {
                match daemon.kill_actor(&pid, &reason) {
                    Ok(msg) => Ok(DaemonResponse::Success(msg)),
                    Err(e) => Ok(DaemonResponse::Error(e.to_string())),
                }
            }
            DaemonMessage::SuspendActor { pid } => {
                match daemon.suspend_actor(&pid) {
                    Ok(msg) => Ok(DaemonResponse::Success(msg)),
                    Err(e) => Ok(DaemonResponse::Error(e.to_string())),
                }
            }
            DaemonMessage::ResumeActor { pid } => {
                match daemon.resume_actor(&pid) {
                    Ok(msg) => Ok(DaemonResponse::Success(msg)),
                    Err(e) => Ok(DaemonResponse::Error(e.to_string())),
                }
            }
            DaemonMessage::RestartActor { pid } => {
                match daemon.restart_actor(&pid) {
                    Ok(msg) => Ok(DaemonResponse::Success(msg)),
                    Err(e) => Ok(DaemonResponse::Error(e.to_string())),
                }
            }
            DaemonMessage::SendMessage { pid, message } => {
                match daemon.send_message(&pid, &message) {
                    Ok(msg) => Ok(DaemonResponse::Success(msg)),
                    Err(e) => Ok(DaemonResponse::Error(e.to_string())),
                }
            }
            DaemonMessage::Shutdown => {
                // TODO: Implement daemon shutdown
                Ok(DaemonResponse::Success("Shutdown initiated".to_string()))
            }
            DaemonMessage::Ping => {
                Ok(DaemonResponse::Pong)
            }
        }
    }
    
    /// Stop the IPC server
    pub fn stop(&mut self) -> ReamResult<()> {
        if let Some(_listener) = self.listener.take() {
            // Listener will be dropped and closed
        }
        
        // Remove socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| ReamError::Io(e))?;
        }
        
        Ok(())
    }
}

/// IPC client for communicating with daemon
pub struct IpcClient {
    /// Socket path
    socket_path: PathBuf,
}

impl IpcClient {
    /// Create a new IPC client
    pub fn new(socket_path: PathBuf) -> Self {
        IpcClient { socket_path }
    }
    
    /// Send a message to the daemon and get response
    pub async fn send_message(&self, message: DaemonMessage) -> ReamResult<DaemonResponse> {
        // Connect to daemon socket
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| ReamError::Io(e))?;
        
        // Serialize and send message
        let message_json = serde_json::to_string(&message)
            .map_err(|e| ReamError::Other(format!("Failed to serialize message: {}", e)))?;
        
        stream.write_all(message_json.as_bytes())
            .map_err(|e| ReamError::Io(e))?;
        
        // Read response
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer)
            .map_err(|e| ReamError::Io(e))?;
        
        // Parse response
        let response: DaemonResponse = serde_json::from_str(&buffer)
            .map_err(|e| ReamError::Other(format!("Failed to parse response: {}", e)))?;
        
        Ok(response)
    }
    
    /// Check if daemon is running
    pub async fn is_daemon_running(&self) -> bool {
        match self.send_message(DaemonMessage::Ping).await {
            Ok(DaemonResponse::Pong) => true,
            _ => false,
        }
    }
    
    /// Get system information from daemon
    pub async fn get_system_info(&self) -> ReamResult<crate::daemon::SystemInfo> {
        match self.send_message(DaemonMessage::GetSystemInfo).await? {
            DaemonResponse::SystemInfo(info) => Ok(info),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// List actors from daemon
    pub async fn list_actors(&self, detailed: bool) -> ReamResult<Vec<crate::daemon::ActorInfo>> {
        match self.send_message(DaemonMessage::ListActors { detailed }).await? {
            DaemonResponse::ActorList(actors) => Ok(actors),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// Get actor information from daemon
    pub async fn get_actor_info(&self, pid: String) -> ReamResult<crate::daemon::ActorInfo> {
        match self.send_message(DaemonMessage::GetActorInfo { pid }).await? {
            DaemonResponse::ActorInfo(info) => Ok(info),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// Kill an actor
    pub async fn kill_actor(&self, pid: String, reason: String) -> ReamResult<String> {
        match self.send_message(DaemonMessage::KillActor { pid, reason }).await? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// Suspend an actor
    pub async fn suspend_actor(&self, pid: String) -> ReamResult<String> {
        match self.send_message(DaemonMessage::SuspendActor { pid }).await? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// Resume an actor
    pub async fn resume_actor(&self, pid: String) -> ReamResult<String> {
        match self.send_message(DaemonMessage::ResumeActor { pid }).await? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// Restart an actor
    pub async fn restart_actor(&self, pid: String) -> ReamResult<String> {
        match self.send_message(DaemonMessage::RestartActor { pid }).await? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// Send message to an actor
    pub async fn send_actor_message(&self, pid: String, message: String) -> ReamResult<String> {
        match self.send_message(DaemonMessage::SendMessage { pid, message }).await? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
    
    /// Shutdown daemon
    pub async fn shutdown_daemon(&self) -> ReamResult<String> {
        match self.send_message(DaemonMessage::Shutdown).await? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(msg) => Err(ReamError::Other(msg)),
            _ => Err(ReamError::Other("Unexpected response".to_string())),
        }
    }
}

} // End of unix_impl module

#[cfg(not(unix))]
mod windows_impl {
    use super::*;
    use super::super::{SystemInfo, ActorInfo, DaemonConfig};

    /// IPC server for daemon communication (Windows stub)
    pub struct IpcServer {
        socket_path: PathBuf,
        daemon: std::sync::Arc<DaemonManager>,
    }

    impl IpcServer {
        pub fn new(socket_path: PathBuf, daemon: std::sync::Arc<DaemonManager>) -> Self {
            IpcServer { socket_path, daemon }
        }

        pub async fn start(&mut self) -> ReamResult<()> {
            Err(ReamError::NotImplemented("IPC server not implemented on Windows".to_string()))
        }

        pub fn stop(&mut self) -> ReamResult<()> {
            Ok(())
        }
    }

    /// IPC client for communicating with daemon (Windows stub)
    pub struct IpcClient {
        socket_path: PathBuf,
    }

    impl IpcClient {
        pub fn new(socket_path: PathBuf) -> Self {
            IpcClient { socket_path }
        }

        pub async fn send_message(&self, _message: DaemonMessage) -> ReamResult<DaemonResponse> {
            Err(ReamError::NotImplemented("IPC client not implemented on Windows".to_string()))
        }

        pub async fn is_daemon_running(&self) -> bool {
            // On Windows, check if PID file exists and process is running
            let config = DaemonConfig::default();

            if !config.pid_file.exists() {
                return false;
            }

            // Read PID from file
            if let Ok(pid_str) = std::fs::read_to_string(&config.pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // Check if process is still running using Windows API
                    self.is_process_running(pid)
                } else {
                    false
                }
            } else {
                false
            }
        }

        fn is_process_running(&self, pid: u32) -> bool {
            // On Windows, we can check if a process exists by trying to open it
            use std::process::Command;

            let output = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid), "/FO", "CSV"])
                .output();

            if let Ok(output) = output {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // If the process exists, tasklist will return more than just the header
                output_str.lines().count() > 1
            } else {
                false
            }
        }

        pub async fn get_system_info(&self) -> ReamResult<SystemInfo> {
            Err(ReamError::NotImplemented("Get system info not implemented on Windows".to_string()))
        }

        pub async fn list_actors(&self, _detailed: bool) -> ReamResult<Vec<ActorInfo>> {
            Err(ReamError::NotImplemented("List actors not implemented on Windows".to_string()))
        }

        pub async fn get_actor_info(&self, _pid: String) -> ReamResult<ActorInfo> {
            Err(ReamError::NotImplemented("Get actor info not implemented on Windows".to_string()))
        }

        pub async fn kill_actor(&self, _pid: String, _reason: String) -> ReamResult<String> {
            Err(ReamError::NotImplemented("Kill actor not implemented on Windows".to_string()))
        }

        pub async fn suspend_actor(&self, _pid: String) -> ReamResult<String> {
            Err(ReamError::NotImplemented("Suspend actor not implemented on Windows".to_string()))
        }

        pub async fn resume_actor(&self, _pid: String) -> ReamResult<String> {
            Err(ReamError::NotImplemented("Resume actor not implemented on Windows".to_string()))
        }

        pub async fn restart_actor(&self, _pid: String) -> ReamResult<String> {
            Err(ReamError::NotImplemented("Restart actor not implemented on Windows".to_string()))
        }

        pub async fn send_actor_message(&self, _pid: String, _message: String) -> ReamResult<String> {
            Err(ReamError::NotImplemented("Send actor message not implemented on Windows".to_string()))
        }

        pub async fn shutdown_daemon(&self) -> ReamResult<String> {
            Err(ReamError::NotImplemented("Shutdown daemon not implemented on Windows".to_string()))
        }
    }
}
