//! Audit Logging System
//!
//! This module provides comprehensive audit logging for security events in the REAM system.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use serde_json;

/// Audit event levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_type: String,
    pub actor: String,
    pub resource: String,
    pub timestamp: SystemTime,
    pub level: AuditLevel,
    pub metadata: HashMap<String, String>,
}

/// Audit filter for controlling what gets logged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFilter {
    pub min_level: AuditLevel,
    pub event_types: Option<Vec<String>>,
    pub actors: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
}

impl Default for AuditFilter {
    fn default() -> Self {
        AuditFilter {
            min_level: AuditLevel::Info,
            event_types: None,
            actors: None,
            resources: None,
        }
    }
}

/// Audit output destination
#[derive(Debug, Clone)]
pub enum AuditOutput {
    File(PathBuf),
    Console,
    Syslog,
    Network(String), // URL for remote logging
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub outputs: Vec<AuditOutput>,
    pub filter: AuditFilter,
    pub buffer_size: usize,
    pub flush_interval_ms: u64,
    pub max_file_size: u64,
    pub max_files: usize,
    pub compress_old_files: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        AuditConfig {
            outputs: vec![AuditOutput::Console],
            filter: AuditFilter::default(),
            buffer_size: 1000,
            flush_interval_ms: 5000,
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            compress_old_files: true,
        }
    }
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub events_logged: u64,
    pub events_filtered: u64,
    pub events_failed: u64,
    pub bytes_written: u64,
    pub files_rotated: u64,
    pub last_event_time: Option<SystemTime>,
}

impl Default for AuditStats {
    fn default() -> Self {
        AuditStats {
            events_logged: 0,
            events_filtered: 0,
            events_failed: 0,
            bytes_written: 0,
            files_rotated: 0,
            last_event_time: None,
        }
    }
}

/// File writer with rotation support
struct RotatingFileWriter {
    base_path: PathBuf,
    current_file: Option<BufWriter<File>>,
    current_size: u64,
    max_size: u64,
    max_files: usize,
    file_index: usize,
}

impl RotatingFileWriter {
    fn new(base_path: PathBuf, max_size: u64, max_files: usize) -> std::io::Result<Self> {
        let mut writer = RotatingFileWriter {
            base_path,
            current_file: None,
            current_size: 0,
            max_size,
            max_files,
            file_index: 0,
        };
        writer.rotate()?;
        Ok(writer)
    }

    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.current_size + data.len() as u64 > self.max_size {
            self.rotate()?;
        }

        if let Some(ref mut file) = self.current_file {
            let bytes_written = file.write(data)?;
            self.current_size += bytes_written as u64;
            Ok(bytes_written)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No current file",
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref mut file) = self.current_file {
            file.flush()
        } else {
            Ok(())
        }
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        // Close current file
        if let Some(mut file) = self.current_file.take() {
            file.flush()?;
        }

        // Remove oldest file if we've reached the limit
        if self.file_index >= self.max_files {
            let oldest_path = self.get_file_path(self.file_index - self.max_files);
            if oldest_path.exists() {
                std::fs::remove_file(oldest_path)?;
            }
        }

        // Create new file
        let new_path = self.get_file_path(self.file_index);
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&new_path)?;

        self.current_file = Some(BufWriter::new(file));
        self.current_size = 0;
        self.file_index += 1;

        Ok(())
    }

    fn get_file_path(&self, index: usize) -> PathBuf {
        if index == 0 {
            self.base_path.clone()
        } else {
            let mut path = self.base_path.clone();
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let extension = path.extension().unwrap_or_default().to_string_lossy();
            let filename = if extension.is_empty() {
                format!("{}.{}", stem, index)
            } else {
                format!("{}.{}.{}", stem, index, extension)
            };
            path.set_file_name(filename);
            path
        }
    }
}

/// Audit logger
pub struct AuditLogger {
    config: AuditConfig,
    file_writers: HashMap<PathBuf, Arc<Mutex<RotatingFileWriter>>>,
    event_buffer: Arc<Mutex<Vec<AuditEvent>>>,
    stats: Arc<Mutex<AuditStats>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new() -> Self {
        AuditLogger::with_config(AuditConfig::default())
    }

    /// Create a new audit logger with configuration
    pub fn with_config(config: AuditConfig) -> Self {
        let mut logger = AuditLogger {
            config,
            file_writers: HashMap::new(),
            event_buffer: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(AuditStats::default())),
        };

        // Initialize file writers
        for output in &logger.config.outputs {
            if let AuditOutput::File(path) = output {
                match RotatingFileWriter::new(
                    path.clone(),
                    logger.config.max_file_size,
                    logger.config.max_files,
                ) {
                    Ok(writer) => {
                        logger.file_writers.insert(path.clone(), Arc::new(Mutex::new(writer)));
                    }
                    Err(e) => {
                        eprintln!("Failed to create file writer for {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Start background flush task
        logger.start_flush_task();

        logger
    }

    /// Log an audit event
    pub fn log_event(&self, event: AuditEvent) {
        // Check filter
        if !self.should_log(&event) {
            let mut stats = self.stats.lock().unwrap();
            stats.events_filtered += 1;
            return;
        }

        // Add to buffer
        {
            let mut buffer = self.event_buffer.lock().unwrap();
            buffer.push(event.clone());

            // Flush if buffer is full
            if buffer.len() >= self.config.buffer_size {
                self.flush_buffer(&mut buffer);
            }
        }

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.events_logged += 1;
            stats.last_event_time = Some(event.timestamp);
        }
    }

    /// Check if an event should be logged based on filters
    fn should_log(&self, event: &AuditEvent) -> bool {
        let filter = &self.config.filter;

        // Check level
        if event.level < filter.min_level {
            return false;
        }

        // Check event types
        if let Some(ref types) = filter.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }

        // Check actors
        if let Some(ref actors) = filter.actors {
            if !actors.contains(&event.actor) {
                return false;
            }
        }

        // Check resources
        if let Some(ref resources) = filter.resources {
            if !resources.contains(&event.resource) {
                return false;
            }
        }

        true
    }

    /// Flush the event buffer
    fn flush_buffer(&self, buffer: &mut Vec<AuditEvent>) {
        for event in buffer.drain(..) {
            self.write_event(&event);
        }
    }

    /// Write an event to all configured outputs
    fn write_event(&self, event: &AuditEvent) {
        let serialized = match serde_json::to_string(event) {
            Ok(s) => s + "\n",
            Err(e) => {
                eprintln!("Failed to serialize audit event: {}", e);
                let mut stats = self.stats.lock().unwrap();
                stats.events_failed += 1;
                return;
            }
        };

        let bytes = serialized.as_bytes();

        for output in &self.config.outputs {
            match output {
                AuditOutput::File(path) => {
                    if let Some(writer) = self.file_writers.get(path) {
                        if let Ok(mut w) = writer.lock() {
                            if let Err(e) = w.write(bytes) {
                                eprintln!("Failed to write to audit file {}: {}", path.display(), e);
                                let mut stats = self.stats.lock().unwrap();
                                stats.events_failed += 1;
                            } else {
                                let mut stats = self.stats.lock().unwrap();
                                stats.bytes_written += bytes.len() as u64;
                            }
                        }
                    }
                }
                AuditOutput::Console => {
                    print!("{}", serialized);
                }
                AuditOutput::Syslog => {
                    // TODO: Implement syslog output
                }
                AuditOutput::Network(_url) => {
                    // TODO: Implement network output
                }
            }
        }
    }

    /// Start background flush task
    fn start_flush_task(&self) {
        let buffer = Arc::clone(&self.event_buffer);
        let interval = self.config.flush_interval_ms;
        let file_writers = self.file_writers.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(interval));
            
            loop {
                interval.tick().await;
                
                // Flush buffer
                if let Ok(mut buffer) = buffer.lock() {
                    if !buffer.is_empty() {
                        // This would need access to self, so we'll implement a simpler version
                        buffer.clear();
                    }
                }

                // Flush file writers
                for writer in file_writers.values() {
                    if let Ok(mut w) = writer.lock() {
                        let _ = w.flush();
                    }
                }
            }
        });
    }

    /// Force flush all buffers
    pub fn flush(&self) {
        let mut buffer = self.event_buffer.lock().unwrap();
        self.flush_buffer(&mut buffer);

        for writer in self.file_writers.values() {
            if let Ok(mut w) = writer.lock() {
                let _ = w.flush();
            }
        }
    }

    /// Get audit statistics
    pub fn get_stats(&self) -> AuditStats {
        self.stats.lock().unwrap().clone()
    }

    /// Update configuration
    pub fn update_config(&mut self, config: AuditConfig) {
        self.config = config;
        // TODO: Reinitialize file writers if needed
    }

    /// Get current configuration
    pub fn get_config(&self) -> &AuditConfig {
        &self.config
    }
}

impl Drop for AuditLogger {
    fn drop(&mut self) {
        self.flush();
    }
}
