//! Cluster metrics and monitoring
//!
//! Provides metrics collection and monitoring for cluster health,
//! performance, and operational status.

use serde::{Deserialize, Serialize};

/// Cluster metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    /// Total number of nodes in cluster
    pub total_nodes: usize,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Number of failed nodes
    pub failed_nodes: usize,
    /// Cluster health percentage
    pub health_percentage: f64,
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    /// Total messages sent
    pub total_messages_sent: u64,
    /// Total messages received
    pub total_messages_received: u64,
    /// Cluster uptime in seconds
    pub uptime_seconds: u64,
}

impl ClusterMetrics {
    /// Create new cluster metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Update metrics with new values
    pub fn update(&mut self, total_nodes: usize, healthy_nodes: usize, failed_nodes: usize) {
        self.total_nodes = total_nodes;
        self.healthy_nodes = healthy_nodes;
        self.failed_nodes = failed_nodes;
        self.health_percentage = if total_nodes > 0 {
            (healthy_nodes as f64 / total_nodes as f64) * 100.0
        } else {
            0.0
        };
    }
}
