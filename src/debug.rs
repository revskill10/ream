//! Debug utilities for REAM actors and runtime
//! 
//! This module provides debugging and tracing capabilities for actor systems.

use std::time::Instant;
use crate::types::Pid;

/// Debug information for actors
#[derive(Debug, Clone)]
pub struct ActorDebugInfo {
    /// Actor identifier
    pub actor_id: Pid,
    /// Actor name/type
    pub actor_name: String,
    /// Number of messages processed
    pub message_count: u64,
    /// Total processing time
    pub total_processing_time: std::time::Duration,
    /// Average processing time per message
    pub average_processing_time: std::time::Duration,
    /// Number of errors encountered
    pub error_count: u64,
    /// Last message processing time
    pub last_message_time: Instant,
    /// Actor creation time
    pub creation_time: Instant,
}

impl ActorDebugInfo {
    /// Create new debug info for an actor
    pub fn new(actor_name: &str) -> Self {
        Self {
            actor_id: Pid::new(),
            actor_name: actor_name.to_string(),
            message_count: 0,
            total_processing_time: std::time::Duration::ZERO,
            average_processing_time: std::time::Duration::ZERO,
            error_count: 0,
            last_message_time: Instant::now(),
            creation_time: Instant::now(),
        }
    }
    
    /// Update processing statistics
    pub fn update_stats(&mut self, processing_time: std::time::Duration) {
        self.message_count += 1;
        self.total_processing_time += processing_time;
        self.average_processing_time = self.total_processing_time / self.message_count as u32;
        self.last_message_time = Instant::now();
    }
    
    /// Record an error
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }
    
    /// Get actor uptime
    pub fn uptime(&self) -> std::time::Duration {
        self.creation_time.elapsed()
    }
    
    /// Get time since last message
    pub fn time_since_last_message(&self) -> std::time::Duration {
        self.last_message_time.elapsed()
    }
}

/// Tracing event for TLISP execution
#[derive(Debug, Clone)]
pub struct TlispTraceEvent {
    /// Event timestamp
    pub timestamp: Instant,
    /// Event type
    pub event_type: TraceEventType,
    /// Event description
    pub description: String,
    /// Associated data
    pub data: Option<String>,
}

/// Types of trace events
#[derive(Debug, Clone)]
pub enum TraceEventType {
    /// Function call
    FunctionCall,
    /// Function return
    FunctionReturn,
    /// Variable binding
    VariableBinding,
    /// Expression evaluation
    ExpressionEval,
    /// Error occurred
    Error,
}

/// TLISP execution tracer
#[derive(Debug)]
pub struct TlispTracer {
    /// Collected events
    events: Vec<TlispTraceEvent>,
    /// Maximum number of events to keep
    max_events: usize,
    /// Tracing enabled flag
    enabled: bool,
}

impl TlispTracer {
    /// Create a new tracer
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            max_events: 10000,
            enabled: true,
        }
    }
    
    /// Enable tracing
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable tracing
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Add a trace event
    pub fn trace(&mut self, event_type: TraceEventType, description: String, data: Option<String>) {
        if !self.enabled {
            return;
        }
        
        let event = TlispTraceEvent {
            timestamp: Instant::now(),
            event_type,
            description,
            data,
        };
        
        self.events.push(event);
        
        // Keep only the most recent events
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }
    
    /// Get all trace events
    pub fn events(&self) -> &[TlispTraceEvent] {
        &self.events
    }
    
    /// Clear all trace events
    pub fn clear(&mut self) {
        self.events.clear();
    }
    
    /// Get events of a specific type
    pub fn events_of_type(&self, event_type: TraceEventType) -> Vec<&TlispTraceEvent> {
        self.events.iter()
            .filter(|event| std::mem::discriminant(&event.event_type) == std::mem::discriminant(&event_type))
            .collect()
    }
}

impl Default for TlispTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance profiler for actors
#[derive(Debug)]
pub struct ActorProfiler {
    /// Actor identifier
    pub actor_id: Pid,
    /// Performance samples
    pub samples: Vec<PerformanceSample>,
    /// Maximum number of samples to keep
    pub max_samples: usize,
}

/// Performance sample
#[derive(Debug, Clone)]
pub struct PerformanceSample {
    /// Sample timestamp
    pub timestamp: Instant,
    /// Processing time
    pub processing_time: std::time::Duration,
    /// Memory usage
    pub memory_usage: usize,
    /// Message queue length
    pub queue_length: usize,
}

impl ActorProfiler {
    /// Create a new profiler
    pub fn new(actor_id: Pid) -> Self {
        Self {
            actor_id,
            samples: Vec::new(),
            max_samples: 1000,
        }
    }
    
    /// Add a performance sample
    pub fn sample(&mut self, processing_time: std::time::Duration, memory_usage: usize, queue_length: usize) {
        let sample = PerformanceSample {
            timestamp: Instant::now(),
            processing_time,
            memory_usage,
            queue_length,
        };
        
        self.samples.push(sample);
        
        // Keep only the most recent samples
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
    }
    
    /// Get average processing time
    pub fn average_processing_time(&self) -> std::time::Duration {
        if self.samples.is_empty() {
            return std::time::Duration::ZERO;
        }
        
        let total: std::time::Duration = self.samples.iter()
            .map(|s| s.processing_time)
            .sum();
        
        total / self.samples.len() as u32
    }
    
    /// Get peak memory usage
    pub fn peak_memory_usage(&self) -> usize {
        self.samples.iter()
            .map(|s| s.memory_usage)
            .max()
            .unwrap_or(0)
    }
    
    /// Get average queue length
    pub fn average_queue_length(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        
        let total: usize = self.samples.iter()
            .map(|s| s.queue_length)
            .sum();
        
        total as f64 / self.samples.len() as f64
    }
}
