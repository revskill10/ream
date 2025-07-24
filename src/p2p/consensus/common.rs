//! Common types and utilities for consensus algorithms
//!
//! Shared data structures and functions used by both PBFT and Raft
//! consensus implementations.

use crate::p2p::{NodeId, P2PResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Value being proposed for consensus
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsensusValue {
    /// Unique identifier for this value
    pub id: uuid::Uuid,
    /// The actual data being proposed
    pub data: Vec<u8>,
    /// Timestamp when value was created
    pub timestamp: u64,
    /// Node that proposed this value
    pub proposer: NodeId,
}

impl ConsensusValue {
    /// Create a new consensus value
    pub fn new(data: Vec<u8>, proposer: NodeId) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            proposer,
        }
    }

    /// Create from string data
    pub fn from_string(data: String, proposer: NodeId) -> Self {
        Self::new(data.into_bytes(), proposer)
    }

    /// Get data as string
    pub fn as_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.data.clone())
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Result of a consensus decision
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// The decided value
    pub value: ConsensusValue,
    /// Term/view when decision was made
    pub term: u64,
    /// Sequence number of this decision
    pub sequence: u64,
    /// Nodes that participated in the decision
    pub participants: Vec<NodeId>,
    /// Timestamp when decision was made
    pub decided_at: u64,
}

impl ConsensusResult {
    /// Create a new consensus result
    pub fn new(
        value: ConsensusValue,
        term: u64,
        sequence: u64,
        participants: Vec<NodeId>,
    ) -> Self {
        Self {
            value,
            term,
            sequence,
            participants,
            decided_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

/// Current state of the consensus system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Current term/view number
    pub term: u64,
    /// Current role of this node
    pub role: ConsensusRole,
    /// Current leader (if known)
    pub leader: Option<NodeId>,
    /// Last committed sequence number
    pub last_committed: u64,
    /// Number of cluster members
    pub cluster_size: usize,
    /// Health status
    pub health: ConsensusHealth,
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self {
            term: 0,
            role: ConsensusRole::Follower,
            leader: None,
            last_committed: 0,
            cluster_size: 1,
            health: ConsensusHealth::Healthy,
        }
    }
}

/// Role of a node in consensus
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusRole {
    /// Leader/Primary node
    Leader,
    /// Follower/Backup node
    Follower,
    /// Candidate (during election)
    Candidate,
    /// Observer (non-voting)
    Observer,
}

impl std::fmt::Display for ConsensusRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusRole::Leader => write!(f, "Leader"),
            ConsensusRole::Follower => write!(f, "Follower"),
            ConsensusRole::Candidate => write!(f, "Candidate"),
            ConsensusRole::Observer => write!(f, "Observer"),
        }
    }
}

/// Health status of consensus system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusHealth {
    /// System is healthy and making progress
    Healthy,
    /// System is degraded but operational
    Degraded,
    /// System is partitioned
    Partitioned,
    /// System is unhealthy
    Unhealthy,
}

/// Log entry for consensus algorithms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    /// Entry index in the log
    pub index: u64,
    /// Term when entry was created
    pub term: u64,
    /// The consensus value
    pub value: ConsensusValue,
    /// Whether this entry is committed
    pub committed: bool,
    /// Timestamp when entry was created
    pub created_at: u64,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(index: u64, term: u64, value: ConsensusValue) -> Self {
        Self {
            index,
            term,
            value,
            committed: false,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Mark entry as committed
    pub fn commit(&mut self) {
        self.committed = true;
    }
}

/// Vote in consensus algorithm
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    /// Node casting the vote
    pub voter: NodeId,
    /// Term/view of the vote
    pub term: u64,
    /// Whether vote is granted
    pub granted: bool,
    /// Optional reason for vote decision
    pub reason: Option<String>,
    /// Timestamp of vote
    pub timestamp: u64,
}

impl Vote {
    /// Create a new vote
    pub fn new(voter: NodeId, term: u64, granted: bool) -> Self {
        Self {
            voter,
            term,
            granted,
            reason: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Create a vote with reason
    pub fn with_reason(voter: NodeId, term: u64, granted: bool, reason: String) -> Self {
        Self {
            voter,
            term,
            granted,
            reason: Some(reason),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

/// Cluster membership information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMembership {
    /// All cluster members
    pub members: Vec<NodeId>,
    /// Current configuration version
    pub version: u64,
    /// Voting members (subset of all members)
    pub voting_members: Vec<NodeId>,
    /// Observer members (non-voting)
    pub observer_members: Vec<NodeId>,
}

impl ClusterMembership {
    /// Create a new cluster membership
    pub fn new(members: Vec<NodeId>) -> Self {
        Self {
            voting_members: members.clone(),
            observer_members: Vec::new(),
            members,
            version: 1,
        }
    }

    /// Add a member to the cluster
    pub fn add_member(&mut self, node_id: NodeId, voting: bool) {
        if !self.members.contains(&node_id) {
            self.members.push(node_id);
            if voting {
                self.voting_members.push(node_id);
            } else {
                self.observer_members.push(node_id);
            }
            self.version += 1;
        }
    }

    /// Remove a member from the cluster
    pub fn remove_member(&mut self, node_id: NodeId) {
        self.members.retain(|&id| id != node_id);
        self.voting_members.retain(|&id| id != node_id);
        self.observer_members.retain(|&id| id != node_id);
        self.version += 1;
    }

    /// Get quorum size for voting members
    pub fn quorum_size(&self) -> usize {
        (self.voting_members.len() / 2) + 1
    }

    /// Check if we have a quorum of votes
    pub fn has_quorum(&self, votes: &[Vote]) -> bool {
        let granted_votes = votes.iter().filter(|v| v.granted).count();
        granted_votes >= self.quorum_size()
    }

    /// Get cluster size
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Check if node is a voting member
    pub fn is_voting_member(&self, node_id: NodeId) -> bool {
        self.voting_members.contains(&node_id)
    }
}

/// Utility functions for consensus algorithms

/// Calculate Byzantine fault tolerance threshold
pub fn byzantine_threshold(cluster_size: usize) -> usize {
    if cluster_size < 4 {
        0
    } else {
        (cluster_size - 1) / 3
    }
}

/// Check if cluster can tolerate Byzantine faults
pub fn can_tolerate_byzantine_faults(cluster_size: usize, failed_nodes: usize) -> bool {
    failed_nodes <= byzantine_threshold(cluster_size)
}

/// Calculate majority threshold for simple majority
pub fn majority_threshold(cluster_size: usize) -> usize {
    (cluster_size / 2) + 1
}

/// Check if we have a simple majority
pub fn has_majority(cluster_size: usize, votes: usize) -> bool {
    votes >= majority_threshold(cluster_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_value() {
        let proposer = NodeId::new();
        let value = ConsensusValue::from_string("test data".to_string(), proposer);
        
        assert_eq!(value.proposer, proposer);
        assert_eq!(value.as_string().unwrap(), "test data");
        assert_eq!(value.size(), 9);
    }

    #[test]
    fn test_consensus_result() {
        let proposer = NodeId::new();
        let value = ConsensusValue::from_string("test".to_string(), proposer);
        let participants = vec![NodeId::new(), NodeId::new()];
        
        let result = ConsensusResult::new(value.clone(), 1, 1, participants.clone());
        
        assert_eq!(result.value, value);
        assert_eq!(result.term, 1);
        assert_eq!(result.sequence, 1);
        assert_eq!(result.participants, participants);
    }

    #[test]
    fn test_cluster_membership() {
        let members = vec![NodeId::new(), NodeId::new(), NodeId::new()];
        let mut membership = ClusterMembership::new(members.clone());
        
        assert_eq!(membership.size(), 3);
        assert_eq!(membership.quorum_size(), 2);
        
        let new_member = NodeId::new();
        membership.add_member(new_member, true);
        assert_eq!(membership.size(), 4);
        assert_eq!(membership.quorum_size(), 3);
        
        membership.remove_member(new_member);
        assert_eq!(membership.size(), 3);
    }

    #[test]
    fn test_byzantine_threshold() {
        assert_eq!(byzantine_threshold(1), 0);
        assert_eq!(byzantine_threshold(3), 0);
        assert_eq!(byzantine_threshold(4), 1);
        assert_eq!(byzantine_threshold(7), 2);
        assert_eq!(byzantine_threshold(10), 3);
    }

    #[test]
    fn test_majority_threshold() {
        assert_eq!(majority_threshold(1), 1);
        assert_eq!(majority_threshold(3), 2);
        assert_eq!(majority_threshold(5), 3);
        assert_eq!(majority_threshold(7), 4);
    }

    #[test]
    fn test_vote() {
        let voter = NodeId::new();
        let vote = Vote::new(voter, 1, true);
        
        assert_eq!(vote.voter, voter);
        assert_eq!(vote.term, 1);
        assert!(vote.granted);
        assert!(vote.reason.is_none());
        
        let vote_with_reason = Vote::with_reason(voter, 1, false, "Invalid proposal".to_string());
        assert!(!vote_with_reason.granted);
        assert_eq!(vote_with_reason.reason.unwrap(), "Invalid proposal");
    }
}
