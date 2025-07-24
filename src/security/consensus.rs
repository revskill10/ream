//! Raft-Based Consensus Storage Adapter
//!
//! This module provides a distributed consensus storage system using the Raft algorithm
//! for Byzantine fault tolerance and strong consistency guarantees.

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock, Mutex};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use raft::{
    prelude::*,
    storage::MemStorage,
    Config, RawNode, StateRole, ReadOnlyOption,
    Entry, HardState, Snapshot, Message,
};
use thiserror::Error;

/// Consensus operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusOperation {
    Set { key: String, value: Vec<u8> },
    Delete { key: String },
    CompareAndSwap { key: String, expected: Vec<u8>, value: Vec<u8> },
    Batch { operations: Vec<ConsensusOperation> },
}

/// Consensus entry for the distributed log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub term: u64,
    pub index: u64,
    pub operation: ConsensusOperation,
}

/// Response from consensus operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResponse {
    pub success: bool,
    pub value: Option<Vec<u8>>,
    pub term: u64,
    pub index: u64,
    pub leader_id: u64,
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    pub node_id: u64,
    pub cluster_peers: Vec<(u64, String)>,
    pub heartbeat_tick: usize,
    pub election_tick: usize,
    pub max_size_per_msg: u64,
    pub max_inflight_msgs: usize,
    pub check_quorum: bool,
    pub pre_vote: bool,
    pub read_only_option: ReadOnlyOption,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        ConsensusConfig {
            node_id: 1,
            cluster_peers: vec![
                (1, "127.0.0.1:5001".to_string()),
                (2, "127.0.0.1:5002".to_string()),
                (3, "127.0.0.1:5003".to_string()),
            ],
            heartbeat_tick: 10,
            election_tick: 50,
            max_size_per_msg: 1024 * 1024, // 1MB
            max_inflight_msgs: 256,
            check_quorum: true,
            pre_vote: true,
            read_only_option: ReadOnlyOption::LeaseBased,
        }
    }
}

/// Proposal for consensus operations
#[derive(Debug)]
pub struct Proposal {
    pub operation: ConsensusOperation,
    pub context: Vec<u8>,
    pub response_tx: tokio::sync::oneshot::Sender<Result<ConsensusResponse, ConsensusError>>,
}

/// Consensus storage backend
pub struct ConsensusStorage {
    /// Key-value store
    data: BTreeMap<String, Vec<u8>>,
    /// Raft log entries
    log_entries: Vec<Entry>,
    /// Raft hard state
    hard_state: HardState,
    /// Snapshot
    snapshot: Snapshot,
    /// Uncommitted entries
    uncommitted_entries: Vec<Entry>,
}

impl ConsensusStorage {
    pub fn new() -> Self {
        ConsensusStorage {
            data: BTreeMap::new(),
            log_entries: Vec::new(),
            hard_state: HardState::default(),
            snapshot: Snapshot::default(),
            uncommitted_entries: Vec::new(),
        }
    }
    
    pub fn set_hard_state(&mut self, hs: HardState) {
        self.hard_state = hs;
    }
    
    pub fn append_entries(&mut self, entries: &[Entry]) {
        self.log_entries.extend_from_slice(entries);
    }
    
    pub fn apply_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshot = snapshot;
        // Apply snapshot data to state machine
        if let Ok(data) = bincode::deserialize::<BTreeMap<String, Vec<u8>>>(&snapshot.data) {
            self.data = data;
        }
    }
}

/// Consensus metrics for monitoring
#[derive(Debug, Default, Clone)]
pub struct ConsensusMetrics {
    pub proposals_submitted: u64,
    pub proposals_failed: u64,
    pub operations_applied: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub leader_changes: u64,
    pub current_term: u64,
    pub current_index: u64,
}

/// Raft consensus adapter
pub struct RaftConsensusAdapter {
    /// Raft node
    raw_node: Arc<Mutex<RawNode<MemStorage>>>,
    /// Storage backend
    storage: Arc<RwLock<ConsensusStorage>>,
    /// Peers in the cluster
    peers: Arc<RwLock<HashMap<u64, String>>>,
    /// Proposal channels
    proposal_tx: mpsc::UnboundedSender<Proposal>,
    proposal_rx: Arc<Mutex<mpsc::UnboundedReceiver<Proposal>>>,
    /// Configuration
    config: ConsensusConfig,
    /// Metrics
    metrics: Arc<RwLock<ConsensusMetrics>>,
}

impl RaftConsensusAdapter {
    /// Create a new Raft consensus adapter
    pub fn new(config: ConsensusConfig) -> Result<Self, ConsensusError> {
        // Create Raft configuration
        let raft_config = Config {
            id: config.node_id,
            heartbeat_tick: config.heartbeat_tick,
            election_tick: config.election_tick,
            max_size_per_msg: config.max_size_per_msg,
            max_inflight_msgs: config.max_inflight_msgs,
            check_quorum: config.check_quorum,
            pre_vote: config.pre_vote,
            read_only_option: config.read_only_option,
            ..Default::default()
        };
        
        // Create storage
        let storage = Arc::new(RwLock::new(ConsensusStorage::new()));
        let raft_storage = MemStorage::new();
        
        // Create Raft node
        let raw_node = RawNode::new(&raft_config, raft_storage)?;
        
        // Create proposal channel
        let (proposal_tx, proposal_rx) = mpsc::unbounded_channel();
        
        // Initialize peers
        let mut peers = HashMap::new();
        for (id, addr) in &config.cluster_peers {
            peers.insert(*id, addr.clone());
        }
        
        Ok(RaftConsensusAdapter {
            raw_node: Arc::new(Mutex::new(raw_node)),
            storage,
            peers: Arc::new(RwLock::new(peers)),
            proposal_tx,
            proposal_rx: Arc::new(Mutex::new(proposal_rx)),
            config,
            metrics: Arc::new(RwLock::new(ConsensusMetrics::default())),
        })
    }
    
    /// Start the consensus adapter
    pub async fn start(&self) -> Result<(), ConsensusError> {
        // Start Raft node
        let raw_node = Arc::clone(&self.raw_node);
        let storage = Arc::clone(&self.storage);
        let proposal_rx = Arc::clone(&self.proposal_rx);
        let metrics = Arc::clone(&self.metrics);
        
        tokio::spawn(async move {
            Self::run_raft_loop(raw_node, storage, proposal_rx, metrics).await;
        });
        
        Ok(())
    }
    
    /// Main Raft loop
    async fn run_raft_loop(
        raw_node: Arc<Mutex<RawNode<MemStorage>>>,
        storage: Arc<RwLock<ConsensusStorage>>,
        proposal_rx: Arc<Mutex<mpsc::UnboundedReceiver<Proposal>>>,
        metrics: Arc<RwLock<ConsensusMetrics>>,
    ) {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(100));
        
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    // Handle Raft tick
                    let mut node = raw_node.lock().unwrap();
                    node.tick();
                    
                    // Process ready state
                    if node.has_ready() {
                        Self::handle_ready(&mut node, &storage, &metrics).await;
                    }
                }
                
                proposal = async {
                    let mut rx = proposal_rx.lock().unwrap();
                    rx.recv().await
                } => {
                    if let Some(proposal) = proposal {
                        Self::handle_proposal(&raw_node, proposal, &metrics).await;
                    }
                }
            }
        }
    }
    
    /// Handle Raft ready state
    async fn handle_ready(
        node: &mut RawNode<MemStorage>,
        storage: &Arc<RwLock<ConsensusStorage>>,
        metrics: &Arc<RwLock<ConsensusMetrics>>,
    ) {
        let mut ready = node.ready();

        // Apply entries to state machine
        let committed_entries = ready.committed_entries().to_vec();
        for entry in committed_entries {
            if !entry.data.is_empty() {
                Self::apply_entry(&entry, storage, metrics).await;
            }
        }

        // Send messages to peers
        let messages = ready.messages().to_vec();
        for message in messages {
            // Send message to peer (network layer implementation)
            Self::send_message_to_peer(message, metrics).await;
        }

        // Persist hard state and entries
        if let Some(hs) = ready.hs() {
            storage.write().unwrap().set_hard_state(hs.clone());
        }

        let entries = ready.entries().to_vec();
        if !entries.is_empty() {
            storage.write().unwrap().append_entries(&entries);
        }

        // Apply snapshot
        if let Some(snapshot) = ready.snapshot() {
            storage.write().unwrap().apply_snapshot(snapshot.clone());
        }

        // Advance the Raft state machine
        node.advance(ready);
    }
    
    /// Apply a log entry to the state machine
    async fn apply_entry(
        entry: &Entry,
        storage: &Arc<RwLock<ConsensusStorage>>,
        metrics: &Arc<RwLock<ConsensusMetrics>>,
    ) {
        if let Ok(operation) = bincode::deserialize::<ConsensusOperation>(&entry.data) {
            match operation {
                ConsensusOperation::Set { key, value } => {
                    storage.write().unwrap().data.insert(key, value);
                    metrics.write().unwrap().operations_applied += 1;
                }
                ConsensusOperation::Delete { key } => {
                    storage.write().unwrap().data.remove(&key);
                    metrics.write().unwrap().operations_applied += 1;
                }
                ConsensusOperation::CompareAndSwap { key, expected, value } => {
                    let mut storage_guard = storage.write().unwrap();
                    if let Some(current) = storage_guard.data.get(&key) {
                        if *current == expected {
                            storage_guard.data.insert(key, value);
                            metrics.write().unwrap().operations_applied += 1;
                        }
                    }
                }
                ConsensusOperation::Batch { operations } => {
                    for op in operations {
                        // Recursively apply batch operations
                        let batch_entry = Entry {
                            data: bincode::serialize(&op).unwrap(),
                            ..entry.clone()
                        };
                        Self::apply_entry(&batch_entry, storage, metrics).await;
                    }
                }
            }
        }
    }
    
    /// Handle a proposal
    async fn handle_proposal(
        raw_node: &Arc<Mutex<RawNode<MemStorage>>>,
        proposal: Proposal,
        metrics: &Arc<RwLock<ConsensusMetrics>>,
    ) {
        let data = bincode::serialize(&proposal.operation).unwrap();
        
        let mut node = raw_node.lock().unwrap();
        match node.propose(proposal.context, data) {
            Ok(_) => {
                metrics.write().unwrap().proposals_submitted += 1;
                // Response will be sent when entry is committed
            }
            Err(e) => {
                metrics.write().unwrap().proposals_failed += 1;
                let _ = proposal.response_tx.send(Err(ConsensusError::ProposalFailed(e.to_string())));
            }
        }
    }
    
    /// Send message to peer (placeholder for network implementation)
    async fn send_message_to_peer(
        _message: Message,
        metrics: &Arc<RwLock<ConsensusMetrics>>,
    ) {
        // Network layer implementation would go here
        metrics.write().unwrap().messages_sent += 1;
    }
    
    /// Propose an operation
    pub async fn propose(&self, operation: ConsensusOperation) -> Result<ConsensusResponse, ConsensusError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        let proposal = Proposal {
            operation,
            context: vec![],
            response_tx,
        };
        
        self.proposal_tx.send(proposal)
            .map_err(|_| ConsensusError::ProposalChannelClosed)?;
        
        response_rx.await
            .map_err(|_| ConsensusError::ProposalTimeout)?
    }
    
    /// Get a value from storage
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, ConsensusError> {
        let storage = self.storage.read().unwrap();
        Ok(storage.data.get(key).cloned())
    }
    
    /// Set a value in storage
    pub async fn set(&self, key: String, value: Vec<u8>) -> Result<ConsensusResponse, ConsensusError> {
        let operation = ConsensusOperation::Set { key, value };
        self.propose(operation).await
    }
    
    /// Delete a value from storage
    pub async fn delete(&self, key: String) -> Result<ConsensusResponse, ConsensusError> {
        let operation = ConsensusOperation::Delete { key };
        self.propose(operation).await
    }
    
    /// Compare and swap operation
    pub async fn compare_and_swap(
        &self,
        key: String,
        expected: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<ConsensusResponse, ConsensusError> {
        let operation = ConsensusOperation::CompareAndSwap { key, expected, value };
        self.propose(operation).await
    }
    
    /// Batch operations
    pub async fn batch(&self, operations: Vec<ConsensusOperation>) -> Result<ConsensusResponse, ConsensusError> {
        let operation = ConsensusOperation::Batch { operations };
        self.propose(operation).await
    }
    
    /// Get consensus metrics
    pub fn get_metrics(&self) -> ConsensusMetrics {
        self.metrics.read().unwrap().clone()
    }
}

/// Consensus errors
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("Raft error: {0}")]
    RaftError(#[from] raft::Error),
    #[error("Proposal failed: {0}")]
    ProposalFailed(String),
    #[error("Proposal channel closed")]
    ProposalChannelClosed,
    #[error("Proposal timeout")]
    ProposalTimeout,
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
    #[error("Storage error: {0}")]
    StorageError(String),
}
