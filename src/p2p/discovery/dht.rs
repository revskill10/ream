//! Kademlia-based Distributed Hash Table (DHT)
//!
//! Implements a Kademlia DHT for decentralized node discovery and
//! key-value storage in the P2P network.

use crate::p2p::{P2PResult, P2PError, DiscoveryError, NodeId, NodeInfo};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Kademlia DHT implementation
#[derive(Debug)]
pub struct ReamDHT {
    /// Local node information
    local_node: NodeInfo,
    /// Routing table organized by distance
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Key-value storage
    storage: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    /// DHT configuration
    config: DHTConfig,
    /// DHT statistics
    stats: Arc<RwLock<DHTStats>>,
    /// Active lookups
    active_lookups: Arc<RwLock<HashMap<Vec<u8>, LookupState>>>,
}

impl ReamDHT {
    /// Create a new DHT instance
    pub fn new(local_node: NodeInfo, config: DHTConfig) -> Self {
        Self {
            local_node,
            routing_table: Arc::new(RwLock::new(RoutingTable::new(config.k_bucket_size))),
            storage: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(DHTStats::default())),
            active_lookups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the DHT
    pub async fn start(&mut self) -> P2PResult<()> {
        // Initialize routing table with local node
        let mut routing_table = self.routing_table.write().await;
        routing_table.initialize(self.local_node.node_id);
        Ok(())
    }

    /// Stop the DHT
    pub async fn stop(&mut self) -> P2PResult<()> {
        // Clear active lookups
        self.active_lookups.write().await.clear();
        Ok(())
    }

    /// Join the DHT network using bootstrap nodes
    pub async fn join_network(&mut self, bootstrap_nodes: Vec<NodeInfo>) -> P2PResult<()> {
        if bootstrap_nodes.is_empty() {
            return Err(P2PError::Discovery(DiscoveryError::BootstrapFailed(
                "No bootstrap nodes provided".to_string()
            )));
        }

        // Add bootstrap nodes to routing table
        {
            let mut routing_table = self.routing_table.write().await;
            for node in &bootstrap_nodes {
                routing_table.add_node(node.clone())?;
            }
        }

        // Perform node lookup for our own ID to populate routing table
        self.lookup_nodes(self.local_node.node_id.as_bytes()).await?;

        self.stats.write().await.networks_joined += 1;
        Ok(())
    }

    /// Initialize network as the first node
    pub async fn initialize_network(&mut self) -> P2PResult<()> {
        // Nothing special needed for first node
        self.stats.write().await.networks_created += 1;
        Ok(())
    }

    /// Find nodes close to a given key
    pub async fn find_nodes(&self, key: &[u8], count: usize) -> P2PResult<Vec<NodeInfo>> {
        let nodes = self.lookup_nodes(key).await?;
        Ok(nodes.into_iter().take(count).collect())
    }

    /// Lookup nodes for a given key
    async fn lookup_nodes(&self, key: &[u8]) -> P2PResult<Vec<NodeInfo>> {
        let lookup_id = key.to_vec();
        
        // Check if lookup is already in progress
        {
            let active_lookups = self.active_lookups.read().await;
            if active_lookups.contains_key(&lookup_id) {
                return Err(P2PError::Discovery(DiscoveryError::DHTFailed(
                    "Lookup already in progress".to_string()
                )));
            }
        }

        // Start new lookup
        let mut lookup_state = LookupState::new(key.to_vec(), self.config.alpha);
        
        // Get initial candidates from routing table
        let initial_candidates = {
            let routing_table = self.routing_table.read().await;
            routing_table.find_closest_nodes(key, self.config.alpha)
        };

        lookup_state.add_candidates(initial_candidates);

        // Store lookup state
        {
            let mut active_lookups = self.active_lookups.write().await;
            active_lookups.insert(lookup_id.clone(), lookup_state);
        }

        // Perform iterative lookup
        let result = self.perform_iterative_lookup(&lookup_id).await;

        // Remove lookup state
        {
            let mut active_lookups = self.active_lookups.write().await;
            active_lookups.remove(&lookup_id);
        }

        self.stats.write().await.lookups_performed += 1;
        result
    }

    /// Perform iterative lookup
    async fn perform_iterative_lookup(&self, lookup_id: &[u8]) -> P2PResult<Vec<NodeInfo>> {
        let mut result = Vec::new();
        
        // Simplified lookup - in real implementation would query remote nodes
        let routing_table = self.routing_table.read().await;
        result = routing_table.find_closest_nodes(lookup_id, self.config.k_bucket_size);
        
        Ok(result)
    }

    /// Store a key-value pair
    pub async fn store(&self, key: Vec<u8>, value: Vec<u8>) -> P2PResult<()> {
        // Find nodes responsible for this key
        let responsible_nodes = self.lookup_nodes(&key).await?;
        
        // Store locally if we're one of the closest nodes
        let local_distance = self.local_node.node_id.distance_to(
            &NodeId::from_bytes(key[..16].try_into().unwrap_or([0; 16]))
        );
        
        let should_store_locally = responsible_nodes.len() < self.config.replication_factor ||
            responsible_nodes.iter().any(|node| {
                let node_distance = node.node_id.distance_to(
                    &NodeId::from_bytes(key[..16].try_into().unwrap_or([0; 16]))
                );
                local_distance <= node_distance
            });

        if should_store_locally {
            let mut storage = self.storage.write().await;
            storage.insert(key.clone(), value);
        }

        // TODO: Send store requests to other responsible nodes
        
        self.stats.write().await.keys_stored += 1;
        Ok(())
    }

    /// Retrieve a value for a key
    pub async fn get(&self, key: &[u8]) -> P2PResult<Option<Vec<u8>>> {
        // Check local storage first
        {
            let storage = self.storage.read().await;
            if let Some(value) = storage.get(key) {
                self.stats.write().await.local_hits += 1;
                return Ok(Some(value.clone()));
            }
        }

        // TODO: Query responsible nodes for the key
        
        self.stats.write().await.lookups_performed += 1;
        Ok(None)
    }

    /// Add a node to the routing table
    pub async fn add_node(&self, node_info: NodeInfo) -> P2PResult<()> {
        let mut routing_table = self.routing_table.write().await;
        routing_table.add_node(node_info)?;
        Ok(())
    }

    /// Remove a node from the routing table
    pub async fn remove_node(&self, node_id: NodeId) -> P2PResult<()> {
        let mut routing_table = self.routing_table.write().await;
        routing_table.remove_node(node_id);
        Ok(())
    }

    /// Update node information
    pub async fn update_node_info(&self, node_info: NodeInfo) -> P2PResult<()> {
        let mut routing_table = self.routing_table.write().await;
        routing_table.update_node(node_info)?;
        Ok(())
    }

    /// Announce presence to the network
    pub async fn announce_presence(&self) -> P2PResult<()> {
        // Refresh our own entry in the DHT
        let key = self.local_node.node_id.as_bytes().to_vec();
        let value = bincode::serialize(&self.local_node)
            .map_err(|e| P2PError::Serialization(e.to_string()))?;
        
        self.store(key, value).await?;
        Ok(())
    }

    /// Get list of known nodes
    pub async fn get_known_nodes(&self) -> Vec<NodeInfo> {
        let routing_table = self.routing_table.read().await;
        routing_table.get_all_nodes()
    }

    /// Get DHT statistics
    pub async fn get_stats(&self) -> DHTStats {
        let mut stats = self.stats.read().await.clone();
        
        // Add current state info
        let routing_table = self.routing_table.read().await;
        stats.known_nodes = routing_table.node_count();
        
        let storage = self.storage.read().await;
        stats.stored_keys = storage.len();
        
        stats
    }
}

/// Routing table for Kademlia DHT
#[derive(Debug)]
pub struct RoutingTable {
    /// K-buckets organized by distance
    buckets: Vec<KBucket>,
    /// Local node ID
    local_node_id: Option<NodeId>,
    /// K-bucket size
    k_bucket_size: usize,
}

impl RoutingTable {
    /// Create a new routing table
    pub fn new(k_bucket_size: usize) -> Self {
        Self {
            buckets: (0..160).map(|_| KBucket::new(k_bucket_size)).collect(),
            local_node_id: None,
            k_bucket_size,
        }
    }

    /// Initialize with local node ID
    pub fn initialize(&mut self, local_node_id: NodeId) {
        self.local_node_id = Some(local_node_id);
    }

    /// Add a node to the routing table
    pub fn add_node(&mut self, node_info: NodeInfo) -> P2PResult<()> {
        let local_id = self.local_node_id.ok_or_else(|| {
            P2PError::Discovery(DiscoveryError::RoutingTableUpdateFailed(
                "Routing table not initialized".to_string()
            ))
        })?;

        if node_info.node_id == local_id {
            return Ok(());
        }

        let distance = local_id.distance_to(&node_info.node_id);
        let bucket_index = self.get_bucket_index(distance);
        
        if bucket_index < self.buckets.len() {
            self.buckets[bucket_index].add_node(node_info);
        }

        Ok(())
    }

    /// Remove a node from the routing table
    pub fn remove_node(&mut self, node_id: NodeId) {
        for bucket in &mut self.buckets {
            bucket.remove_node(node_id);
        }
    }

    /// Update node information
    pub fn update_node(&mut self, node_info: NodeInfo) -> P2PResult<()> {
        self.remove_node(node_info.node_id);
        self.add_node(node_info)
    }

    /// Find closest nodes to a key
    pub fn find_closest_nodes(&self, key: &[u8], count: usize) -> Vec<NodeInfo> {
        let local_id = match self.local_node_id {
            Some(id) => id,
            None => return Vec::new(),
        };

        let target_id = NodeId::from_bytes(key[..16].try_into().unwrap_or([0; 16]));
        let distance = local_id.distance_to(&target_id);
        let bucket_index = self.get_bucket_index(distance);

        let mut candidates = Vec::new();

        // Start with the appropriate bucket
        if bucket_index < self.buckets.len() {
            candidates.extend(self.buckets[bucket_index].get_nodes());
        }

        // Add nodes from nearby buckets if needed
        let mut radius = 1;
        while candidates.len() < count && radius <= self.buckets.len() {
            if bucket_index >= radius {
                candidates.extend(self.buckets[bucket_index - radius].get_nodes());
            }
            if bucket_index + radius < self.buckets.len() {
                candidates.extend(self.buckets[bucket_index + radius].get_nodes());
            }
            radius += 1;
        }

        // Sort by distance to target
        candidates.sort_by_key(|node| target_id.distance_to(&node.node_id));
        candidates.truncate(count);
        candidates
    }

    /// Get all nodes in the routing table
    pub fn get_all_nodes(&self) -> Vec<NodeInfo> {
        let mut nodes = Vec::new();
        for bucket in &self.buckets {
            nodes.extend(bucket.get_nodes());
        }
        nodes
    }

    /// Get number of nodes in routing table
    pub fn node_count(&self) -> usize {
        self.buckets.iter().map(|bucket| bucket.size()).sum()
    }

    /// Get bucket index for a given distance
    fn get_bucket_index(&self, distance: u128) -> usize {
        if distance == 0 {
            return 0;
        }
        159 - distance.leading_zeros() as usize
    }
}

/// K-bucket for storing nodes at a specific distance range
#[derive(Debug)]
pub struct KBucket {
    /// Nodes in this bucket
    nodes: Vec<NodeInfo>,
    /// Maximum size of bucket
    max_size: usize,
}

impl KBucket {
    /// Create a new K-bucket
    pub fn new(max_size: usize) -> Self {
        Self {
            nodes: Vec::new(),
            max_size,
        }
    }

    /// Add a node to the bucket
    pub fn add_node(&mut self, node_info: NodeInfo) {
        // Remove if already exists
        self.nodes.retain(|n| n.node_id != node_info.node_id);
        
        // Add to front
        self.nodes.insert(0, node_info);
        
        // Trim if over capacity
        if self.nodes.len() > self.max_size {
            self.nodes.truncate(self.max_size);
        }
    }

    /// Remove a node from the bucket
    pub fn remove_node(&mut self, node_id: NodeId) {
        self.nodes.retain(|n| n.node_id != node_id);
    }

    /// Get all nodes in the bucket
    pub fn get_nodes(&self) -> Vec<NodeInfo> {
        self.nodes.clone()
    }

    /// Get bucket size
    pub fn size(&self) -> usize {
        self.nodes.len()
    }
}

/// State for an active lookup operation
#[derive(Debug)]
pub struct LookupState {
    /// Target key being looked up
    pub target: Vec<u8>,
    /// Candidate nodes
    pub candidates: Vec<NodeInfo>,
    /// Nodes already queried
    pub queried: std::collections::HashSet<NodeId>,
    /// Alpha parameter (concurrency)
    pub alpha: usize,
}

impl LookupState {
    /// Create a new lookup state
    pub fn new(target: Vec<u8>, alpha: usize) -> Self {
        Self {
            target,
            candidates: Vec::new(),
            queried: std::collections::HashSet::new(),
            alpha,
        }
    }

    /// Add candidate nodes
    pub fn add_candidates(&mut self, nodes: Vec<NodeInfo>) {
        for node in nodes {
            if !self.queried.contains(&node.node_id) {
                self.candidates.push(node);
            }
        }
        
        // Sort by distance to target
        let target_id = NodeId::from_bytes(self.target[..16].try_into().unwrap_or([0; 16]));
        self.candidates.sort_by_key(|node| target_id.distance_to(&node.node_id));
    }
}

/// DHT configuration
#[derive(Debug, Clone)]
pub struct DHTConfig {
    /// K-bucket size (typically 20)
    pub k_bucket_size: usize,
    /// Alpha parameter for parallel lookups (typically 3)
    pub alpha: usize,
    /// Replication factor for stored values
    pub replication_factor: usize,
    /// Timeout for DHT operations
    pub operation_timeout: std::time::Duration,
}

impl Default for DHTConfig {
    fn default() -> Self {
        Self {
            k_bucket_size: 20,
            alpha: 3,
            replication_factor: 3,
            operation_timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// DHT statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DHTStats {
    /// Number of networks joined
    pub networks_joined: u64,
    /// Number of networks created
    pub networks_created: u64,
    /// Number of lookups performed
    pub lookups_performed: u64,
    /// Number of keys stored
    pub keys_stored: u64,
    /// Number of local cache hits
    pub local_hits: u64,
    /// Current number of known nodes
    pub known_nodes: usize,
    /// Current number of stored keys
    pub stored_keys: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[test]
    fn test_routing_table() {
        let mut table = RoutingTable::new(20);
        let local_id = NodeId::new();
        table.initialize(local_id);

        let node_info = NodeInfo::new(
            NodeId::new(),
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
        );

        assert!(table.add_node(node_info.clone()).is_ok());
        assert_eq!(table.node_count(), 1);

        let nodes = table.find_closest_nodes(node_info.node_id.as_bytes(), 5);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, node_info.node_id);
    }

    #[test]
    fn test_k_bucket() {
        let mut bucket = KBucket::new(3);
        
        for i in 0..5 {
            let node_info = NodeInfo::new(
                NodeId::new(),
                format!("127.0.0.1:808{}", i).parse::<SocketAddr>().unwrap(),
            );
            bucket.add_node(node_info);
        }

        assert_eq!(bucket.size(), 3); // Should be limited to max_size
    }

    #[tokio::test]
    async fn test_dht_creation() {
        let node_info = NodeInfo::new(
            NodeId::new(),
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
        );
        let config = DHTConfig::default();
        
        let dht = ReamDHT::new(node_info, config);
        assert_eq!(dht.get_known_nodes().await.len(), 0);
    }

    #[tokio::test]
    async fn test_dht_lifecycle() {
        let node_info = NodeInfo::new(
            NodeId::new(),
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
        );
        let config = DHTConfig::default();
        
        let mut dht = ReamDHT::new(node_info, config);
        
        assert!(dht.start().await.is_ok());
        assert!(dht.initialize_network().await.is_ok());
        assert!(dht.stop().await.is_ok());
    }
}
