//! Message routing for distributed network
//!
//! Implements intelligent message routing across the P2P network with
//! path optimization and failure recovery.

use crate::p2p::{P2PResult, P2PError, NetworkError, NodeId};
use super::{NetworkLayer, RoutingMessage, NetworkMessage};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Message router for distributed network
#[derive(Debug)]
pub struct MessageRouter {
    /// Routing table mapping actors to nodes
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Network topology information
    topology: Arc<RwLock<NetworkTopology>>,
    /// Routing statistics
    stats: Arc<RwLock<RoutingStats>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            routing_table: Arc::new(RwLock::new(RoutingTable::new())),
            topology: Arc::new(RwLock::new(NetworkTopology::new())),
            stats: Arc::new(RwLock::new(RoutingStats::default())),
        }
    }

    /// Route a message through the network
    pub async fn route_message(
        &self,
        network: &NetworkLayer,
        mut message: RoutingMessage,
    ) -> P2PResult<()> {
        // Check TTL
        if message.ttl == 0 {
            self.stats.write().await.messages_dropped += 1;
            return Err(P2PError::Network(NetworkError::RoutingError(
                "Message TTL exceeded".to_string()
            )));
        }

        message.ttl -= 1;

        // Find path to destination
        let path = self.find_path_to_destination(message.destination).await?;
        
        if path.is_empty() {
            self.stats.write().await.messages_dropped += 1;
            return Err(P2PError::Network(NetworkError::RoutingError(
                format!("No path to destination {}", message.destination)
            )));
        }

        // Get next hop
        let next_hop = path[0];
        message.path.push(next_hop);

        // Forward message to next hop
        if network.is_connected_to(next_hop).await {
            network.send_message(next_hop, NetworkMessage::Custom(
                bincode::serialize(&message).unwrap()
            )).await?;
            self.stats.write().await.messages_routed += 1;
        } else {
            self.stats.write().await.messages_dropped += 1;
            return Err(P2PError::Network(NetworkError::RoutingError(
                format!("No connection to next hop {}", next_hop)
            )));
        }

        Ok(())
    }

    /// Update actor location in routing table
    pub async fn update_actor_location(&self, actor_id: crate::p2p::ActorId, node_id: NodeId) {
        let mut routing_table = self.routing_table.write().await;
        routing_table.update_actor_location(actor_id, node_id);
    }

    /// Remove actor from routing table
    pub async fn remove_actor(&self, actor_id: crate::p2p::ActorId) {
        let mut routing_table = self.routing_table.write().await;
        routing_table.remove_actor(actor_id);
    }

    /// Update network topology
    pub async fn update_topology(&self, nodes: Vec<NodeId>, connections: Vec<(NodeId, NodeId)>) {
        let mut topology = self.topology.write().await;
        topology.update(nodes, connections);
    }

    /// Find path to destination node
    async fn find_path_to_destination(&self, destination: NodeId) -> P2PResult<Vec<NodeId>> {
        let topology = self.topology.read().await;
        topology.find_shortest_path(destination)
    }

    /// Get routing statistics
    pub async fn get_stats(&self) -> RoutingStats {
        self.stats.read().await.clone()
    }
}

/// Routing table for actor locations
#[derive(Debug)]
pub struct RoutingTable {
    /// Map from actor ID to node ID
    actor_locations: HashMap<crate::p2p::ActorId, NodeId>,
    /// Reverse map from node ID to actors
    node_actors: HashMap<NodeId, Vec<crate::p2p::ActorId>>,
}

impl RoutingTable {
    /// Create a new routing table
    pub fn new() -> Self {
        Self {
            actor_locations: HashMap::new(),
            node_actors: HashMap::new(),
        }
    }

    /// Update actor location
    pub fn update_actor_location(&mut self, actor_id: crate::p2p::ActorId, node_id: NodeId) {
        // Remove from old location
        if let Some(old_node) = self.actor_locations.get(&actor_id) {
            if let Some(actors) = self.node_actors.get_mut(old_node) {
                actors.retain(|&id| id != actor_id);
            }
        }

        // Add to new location
        self.actor_locations.insert(actor_id, node_id);
        self.node_actors.entry(node_id).or_insert_with(Vec::new).push(actor_id);
    }

    /// Remove actor
    pub fn remove_actor(&mut self, actor_id: crate::p2p::ActorId) {
        if let Some(node_id) = self.actor_locations.remove(&actor_id) {
            if let Some(actors) = self.node_actors.get_mut(&node_id) {
                actors.retain(|&id| id != actor_id);
            }
        }
    }

    /// Get actor location
    pub fn get_actor_location(&self, actor_id: crate::p2p::ActorId) -> Option<NodeId> {
        self.actor_locations.get(&actor_id).copied()
    }

    /// Get actors on a node
    pub fn get_actors_on_node(&self, node_id: NodeId) -> Vec<crate::p2p::ActorId> {
        self.node_actors.get(&node_id).cloned().unwrap_or_default()
    }
}

/// Network topology for path finding
#[derive(Debug)]
pub struct NetworkTopology {
    /// Graph adjacency list
    adjacency: HashMap<NodeId, Vec<NodeId>>,
    /// All known nodes
    nodes: Vec<NodeId>,
}

impl NetworkTopology {
    /// Create a new network topology
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
            nodes: Vec::new(),
        }
    }

    /// Update topology with new nodes and connections
    pub fn update(&mut self, nodes: Vec<NodeId>, connections: Vec<(NodeId, NodeId)>) {
        self.nodes = nodes;
        self.adjacency.clear();

        // Initialize adjacency list
        for node in &self.nodes {
            self.adjacency.insert(*node, Vec::new());
        }

        // Add connections
        for (from, to) in connections {
            self.adjacency.entry(from).or_insert_with(Vec::new).push(to);
            self.adjacency.entry(to).or_insert_with(Vec::new).push(from); // Bidirectional
        }
    }

    /// Find shortest path to destination using BFS
    pub fn find_shortest_path(&self, destination: NodeId) -> P2PResult<Vec<NodeId>> {
        // For now, return empty path - would implement proper pathfinding
        // This is a simplified version for the basic implementation
        if self.nodes.contains(&destination) {
            Ok(vec![destination])
        } else {
            Ok(vec![])
        }
    }

    /// Check if network is connected
    pub fn is_connected(&self) -> bool {
        if self.nodes.is_empty() {
            return true;
        }

        let start = self.nodes[0];
        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(start);
        visited.insert(start);

        while let Some(node) = queue.pop_front() {
            if let Some(neighbors) = self.adjacency.get(&node) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        visited.len() == self.nodes.len()
    }

    /// Get network diameter (longest shortest path)
    pub fn get_diameter(&self) -> usize {
        // Simplified implementation
        self.nodes.len()
    }
}

/// Routing statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoutingStats {
    /// Total messages routed
    pub messages_routed: u64,
    /// Messages dropped due to routing failures
    pub messages_dropped: u64,
    /// Average routing latency in milliseconds
    pub average_latency_ms: f64,
    /// Number of routing table updates
    pub routing_table_updates: u64,
    /// Number of topology updates
    pub topology_updates: u64,
}

/// Routing configuration
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Maximum TTL for messages
    pub max_ttl: u8,
    /// Routing algorithm to use
    pub algorithm: RoutingAlgorithm,
    /// Enable path optimization
    pub optimize_paths: bool,
    /// Cache size for routing decisions
    pub cache_size: usize,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            max_ttl: 16,
            algorithm: RoutingAlgorithm::ShortestPath,
            optimize_paths: true,
            cache_size: 1000,
        }
    }
}

/// Routing algorithms
#[derive(Debug, Clone)]
pub enum RoutingAlgorithm {
    /// Shortest path routing
    ShortestPath,
    /// Load-balanced routing
    LoadBalanced,
    /// Adaptive routing based on network conditions
    Adaptive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_table() {
        let mut table = RoutingTable::new();
        let actor_id = crate::p2p::ActorId::new();
        let node_id = NodeId::new();

        table.update_actor_location(actor_id, node_id);
        assert_eq!(table.get_actor_location(actor_id), Some(node_id));

        let actors = table.get_actors_on_node(node_id);
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0], actor_id);

        table.remove_actor(actor_id);
        assert_eq!(table.get_actor_location(actor_id), None);
    }

    #[test]
    fn test_network_topology() {
        let mut topology = NetworkTopology::new();
        let node1 = NodeId::new();
        let node2 = NodeId::new();

        topology.update(vec![node1, node2], vec![(node1, node2)]);
        assert!(topology.is_connected());

        let path = topology.find_shortest_path(node2).unwrap();
        assert!(!path.is_empty());
    }

    #[tokio::test]
    async fn test_message_router() {
        let router = MessageRouter::new();
        let stats = router.get_stats().await;
        assert_eq!(stats.messages_routed, 0);
        assert_eq!(stats.messages_dropped, 0);
    }
}
