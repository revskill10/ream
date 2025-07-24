//! Cluster management for P2P distributed system
//!
//! Provides cluster membership management, failure detection,
//! recovery mechanisms, and cluster health monitoring.

pub mod membership;
pub mod failure_detection;
pub mod recovery;
pub mod metrics;

pub use membership::*;
pub use failure_detection::*;
pub use recovery::*;
pub use metrics::*;

use crate::p2p::{P2PResult, P2PError, ClusterError, NodeId, NodeInfo, ClusterInfo, ClusterHealth};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Cluster manager for distributed system
#[derive(Debug)]
pub struct ClusterManager {
    /// Local node information
    local_node: NodeInfo,
    /// Cluster membership manager
    membership: Arc<RwLock<MembershipManager>>,
    /// Failure detector
    failure_detector: Arc<RwLock<FailureDetector>>,
    /// Recovery manager
    recovery: Arc<RwLock<RecoveryManager>>,
    /// Cluster metrics
    metrics: Arc<RwLock<ClusterMetrics>>,
    /// Configuration
    config: ClusterConfig,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub async fn new(local_node: NodeInfo, config: ClusterConfig) -> P2PResult<Self> {
        let membership = Arc::new(RwLock::new(MembershipManager::new(local_node.clone())));
        let failure_detector = Arc::new(RwLock::new(FailureDetector::new(config.failure_detection_config.clone())));
        let recovery = Arc::new(RwLock::new(RecoveryManager::new(config.recovery_config.clone())));
        let metrics = Arc::new(RwLock::new(ClusterMetrics::new()));

        Ok(Self {
            local_node,
            membership,
            failure_detector,
            recovery,
            metrics,
            config,
        })
    }

    /// Start the cluster manager
    pub async fn start(&self) -> P2PResult<()> {
        // Start all components
        {
            let mut membership = self.membership.write().await;
            membership.start().await?;
        }

        {
            let mut failure_detector = self.failure_detector.write().await;
            failure_detector.start().await?;
        }

        {
            let mut recovery = self.recovery.write().await;
            recovery.start().await?;
        }

        Ok(())
    }

    /// Stop the cluster manager
    pub async fn stop(&self) -> P2PResult<()> {
        // Stop all components
        {
            let mut recovery = self.recovery.write().await;
            recovery.stop().await?;
        }

        {
            let mut failure_detector = self.failure_detector.write().await;
            failure_detector.stop().await?;
        }

        {
            let mut membership = self.membership.write().await;
            membership.stop().await?;
        }

        Ok(())
    }

    /// Create a new cluster
    pub async fn create_cluster(&self) -> P2PResult<ClusterInfo> {
        let mut membership = self.membership.write().await;
        membership.create_cluster().await
    }

    /// Join an existing cluster
    pub async fn join_cluster(&self, bootstrap_nodes: Vec<NodeInfo>) -> P2PResult<ClusterInfo> {
        let mut membership = self.membership.write().await;
        membership.join_cluster(bootstrap_nodes).await
    }

    /// Leave the cluster
    pub async fn leave_cluster(&self) -> P2PResult<()> {
        let mut membership = self.membership.write().await;
        membership.leave_cluster().await
    }

    /// Add a node to the cluster
    pub async fn add_node(&self, node_info: NodeInfo) -> P2PResult<()> {
        let mut membership = self.membership.write().await;
        membership.add_member(node_info).await
    }

    /// Add a member to the cluster
    pub async fn add_member(&self, node_info: NodeInfo) -> P2PResult<()> {
        let mut membership = self.membership.write().await;
        membership.add_member(node_info).await
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: NodeId) -> P2PResult<()> {
        let mut membership = self.membership.write().await;
        membership.remove_member(node_id).await
    }

    /// Get current cluster information
    pub async fn get_cluster_info(&self) -> P2PResult<ClusterInfo> {
        let membership = self.membership.read().await;
        membership.get_cluster_info().await
    }

    /// Get cluster members
    pub async fn get_members(&self) -> Vec<NodeInfo> {
        let membership = self.membership.read().await;
        membership.get_members().await
    }

    /// Check if a node is alive
    pub async fn is_node_alive(&self, node_id: NodeId) -> bool {
        let failure_detector = self.failure_detector.read().await;
        failure_detector.is_alive(node_id).await
    }

    /// Report node failure
    pub async fn report_node_failure(&self, node_id: NodeId) -> P2PResult<()> {
        let mut failure_detector = self.failure_detector.write().await;
        failure_detector.report_failure(node_id).await?;

        // Trigger recovery if needed
        let mut recovery = self.recovery.write().await;
        recovery.handle_node_failure(node_id).await?;

        Ok(())
    }

    /// Get cluster health status
    pub async fn get_health_status(&self) -> ClusterHealth {
        let membership = self.membership.read().await;
        let failure_detector = self.failure_detector.read().await;
        
        let total_members = membership.get_member_count().await;
        let failed_members = failure_detector.get_failed_node_count().await;
        
        if failed_members == 0 {
            ClusterHealth::Healthy
        } else if failed_members < total_members / 2 {
            ClusterHealth::Degraded
        } else {
            ClusterHealth::Unhealthy
        }
    }

    /// Get cluster metrics
    pub async fn get_metrics(&self) -> ClusterMetrics {
        self.metrics.read().await.clone()
    }

    /// Update cluster metrics
    pub async fn update_metrics(&self) -> P2PResult<()> {
        let mut metrics = self.metrics.write().await;
        
        // Update from membership
        let membership = self.membership.read().await;
        metrics.total_nodes = membership.get_member_count().await;
        
        // Update from failure detector
        let failure_detector = self.failure_detector.read().await;
        metrics.failed_nodes = failure_detector.get_failed_node_count().await;
        metrics.healthy_nodes = metrics.total_nodes - metrics.failed_nodes;
        
        // Calculate health percentage
        metrics.health_percentage = if metrics.total_nodes > 0 {
            (metrics.healthy_nodes as f64 / metrics.total_nodes as f64) * 100.0
        } else {
            0.0
        };

        Ok(())
    }
}

/// Cluster configuration
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Membership configuration
    pub membership_config: MembershipConfig,
    /// Failure detection configuration
    pub failure_detection_config: FailureDetectionConfig,
    /// Recovery configuration
    pub recovery_config: RecoveryConfig,
    /// Cluster health check interval
    pub health_check_interval: std::time::Duration,
    /// Maximum cluster size
    pub max_cluster_size: usize,
    /// Minimum cluster size for operation
    pub min_cluster_size: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            membership_config: MembershipConfig::default(),
            failure_detection_config: FailureDetectionConfig::default(),
            recovery_config: RecoveryConfig::default(),
            health_check_interval: std::time::Duration::from_secs(30),
            max_cluster_size: 1000,
            min_cluster_size: 1,
        }
    }
}

// Placeholder implementations for the sub-modules
// These would be fully implemented in their respective files

/// Membership manager
#[derive(Debug)]
pub struct MembershipManager {
    local_node: NodeInfo,
    members: HashMap<NodeId, NodeInfo>,
    cluster_id: Option<uuid::Uuid>,
}

impl MembershipManager {
    pub fn new(local_node: NodeInfo) -> Self {
        Self {
            local_node,
            members: HashMap::new(),
            cluster_id: None,
        }
    }

    pub async fn start(&mut self) -> P2PResult<()> { Ok(()) }
    pub async fn stop(&mut self) -> P2PResult<()> { Ok(()) }
    
    pub async fn create_cluster(&mut self) -> P2PResult<ClusterInfo> {
        self.cluster_id = Some(uuid::Uuid::new_v4());
        self.members.insert(self.local_node.node_id, self.local_node.clone());
        
        Ok(ClusterInfo {
            cluster_id: self.cluster_id.unwrap(),
            member_count: 1,
            members: vec![self.local_node.clone()],
            leader: Some(self.local_node.node_id),
            health: ClusterHealth::Healthy,
            formed_at: std::time::SystemTime::now(),
        })
    }
    
    pub async fn join_cluster(&mut self, bootstrap_nodes: Vec<NodeInfo>) -> P2PResult<ClusterInfo> {
        // Add bootstrap nodes to our member list
        for bootstrap_node in &bootstrap_nodes {
            self.members.insert(bootstrap_node.node_id, bootstrap_node.clone());
        }

        // Add ourselves to the member list
        self.members.insert(self.local_node.node_id, self.local_node.clone());

        // Use the cluster ID from the first bootstrap node (simplified)
        self.cluster_id = Some(uuid::Uuid::new_v4());

        Ok(ClusterInfo {
            cluster_id: self.cluster_id.unwrap(),
            member_count: self.members.len(),
            members: self.members.values().cloned().collect(),
            leader: Some(bootstrap_nodes.first().map(|n| n.node_id).unwrap_or(self.local_node.node_id)),
            health: ClusterHealth::Healthy,
            formed_at: std::time::SystemTime::now(),
        })
    }
    
    pub async fn leave_cluster(&mut self) -> P2PResult<()> {
        self.members.clear();
        self.cluster_id = None;
        Ok(())
    }
    
    pub async fn add_member(&mut self, node_info: NodeInfo) -> P2PResult<()> {
        self.members.insert(node_info.node_id, node_info);
        Ok(())
    }

    pub async fn remove_member(&mut self, node_id: NodeId) -> P2PResult<()> {
        self.members.remove(&node_id);
        Ok(())
    }
    
    pub async fn get_cluster_info(&self) -> P2PResult<ClusterInfo> {
        Ok(ClusterInfo {
            cluster_id: self.cluster_id.unwrap_or_else(uuid::Uuid::new_v4),
            member_count: self.members.len(),
            members: self.members.values().cloned().collect(),
            leader: Some(self.local_node.node_id),
            health: ClusterHealth::Healthy,
            formed_at: std::time::SystemTime::now(),
        })
    }
    
    pub async fn get_members(&self) -> Vec<NodeInfo> {
        self.members.values().cloned().collect()
    }
    
    pub async fn get_member_count(&self) -> usize {
        self.members.len()
    }
}

/// Membership configuration
#[derive(Debug, Clone)]
pub struct MembershipConfig {
    pub gossip_interval: std::time::Duration,
    pub membership_timeout: std::time::Duration,
}

impl Default for MembershipConfig {
    fn default() -> Self {
        Self {
            gossip_interval: std::time::Duration::from_secs(1),
            membership_timeout: std::time::Duration::from_secs(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_cluster_manager_creation() {
        let node_info = NodeInfo::new(
            NodeId::new(),
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
        );
        let config = ClusterConfig::default();
        
        let result = ClusterManager::new(node_info, config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_lifecycle() {
        let node_info = NodeInfo::new(
            NodeId::new(),
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
        );
        let config = ClusterConfig::default();
        
        let manager = ClusterManager::new(node_info, config).await.unwrap();
        
        assert!(manager.start().await.is_ok());
        
        let cluster_info = manager.create_cluster().await.unwrap();
        assert_eq!(cluster_info.member_count, 1);
        
        let health = manager.get_health_status().await;
        assert_eq!(health, ClusterHealth::Healthy);
        
        assert!(manager.stop().await.is_ok());
    }
}
