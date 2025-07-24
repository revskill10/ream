//! Real consensus algorithm tests
//!
//! Tests that verify actual consensus functionality with real leader election,
//! log replication, and distributed agreement.

use ream::p2p::consensus::{RaftConsensus, RaftConfig, RaftMessage, ConsensusValue, ConsensusRole};
use ream::p2p::NodeId;
use std::time::Duration;

/// Helper function to create a test RaftConfig
fn test_raft_config() -> RaftConfig {
    RaftConfig {
        election_timeout_min: Duration::from_millis(150),
        election_timeout_max: Duration::from_millis(300),
        heartbeat_interval: Duration::from_millis(50),
        max_entries_per_request: 100,
    }
}

/// Test basic Raft consensus with single node
#[tokio::test]
async fn test_single_node_raft_consensus() {
    let config = test_raft_config();

    let node_id = NodeId::new();
    let mut raft = RaftConsensus::new(config, node_id, vec![node_id]).unwrap();

    // Start consensus
    raft.start().await.unwrap();

    // Single node should be able to start election and become leader
    raft.start_election().await.unwrap();

    // Check state
    let state = raft.get_state().await;
    assert_eq!(state.role, ConsensusRole::Leader);
    assert_eq!(state.term, 1);

    // Propose a value
    let value = ConsensusValue::new(b"test_value".to_vec(), node_id);
    let result = raft.propose(value.clone()).await.unwrap();

    assert_eq!(result.value, value);
    assert_eq!(result.term, 1);
    assert_eq!(result.sequence, 1);

    // Check that value was committed
    let final_state = raft.get_state().await;
    assert_eq!(final_state.last_committed, 1);
}

/// Test Raft message handling
#[tokio::test]
async fn test_raft_message_handling() {
    let config = test_raft_config();

    let node1 = NodeId::new();
    let node2 = NodeId::new();
    let mut raft = RaftConsensus::new(config, node1, vec![node1, node2]).unwrap();

    raft.start().await.unwrap();

    // Test RequestVote message
    let vote_request = RaftMessage::RequestVote {
        term: 2,
        candidate_id: node2,
        last_log_index: 0,
        last_log_term: 0,
    };

    let response = raft.handle_message(vote_request).await.unwrap();
    
    match response {
        Some(RaftMessage::RequestVoteResponse { term, vote_granted }) => {
            assert_eq!(term, 2);
            assert!(vote_granted); // Should grant vote to valid candidate
        }
        _ => panic!("Expected RequestVoteResponse"),
    }

    // Check that term was updated
    let state = raft.get_state().await;
    assert_eq!(state.term, 2);
    assert_eq!(state.role, ConsensusRole::Follower);
}

/// Test AppendEntries handling
#[tokio::test]
async fn test_append_entries_handling() {
    let config = test_raft_config();

    let node1 = NodeId::new();
    let node2 = NodeId::new();
    let mut raft = RaftConsensus::new(config, node1, vec![node1, node2]).unwrap();

    raft.start().await.unwrap();

    // Test heartbeat (empty AppendEntries)
    let heartbeat = RaftMessage::AppendEntries {
        term: 1,
        leader_id: node2,
        prev_log_index: 0,
        prev_log_term: 0,
        entries: vec![],
        leader_commit: 0,
    };

    let response = raft.handle_message(heartbeat).await.unwrap();
    
    match response {
        Some(RaftMessage::AppendEntriesResponse { term, success, match_index }) => {
            assert_eq!(term, 1);
            assert!(success);
            assert_eq!(match_index, 0);
        }
        _ => panic!("Expected AppendEntriesResponse"),
    }

    // Check that heartbeat was received
    assert!(!raft.is_election_timeout());
}

/// Test leader election timeout
#[tokio::test]
async fn test_election_timeout() {
    let config = test_raft_config();

    let node_id = NodeId::new();
    let mut raft = RaftConsensus::new(config, node_id, vec![node_id]).unwrap();

    raft.start().await.unwrap();

    // Initially should not be timed out
    assert!(!raft.is_election_timeout());

    // Force election timeout for testing
    raft.force_election_timeout();

    // Should detect election timeout
    assert!(raft.is_election_timeout());
}

/// Test log replication with multiple entries
#[tokio::test]
async fn test_log_replication() {
    let config = test_raft_config();

    let node_id = NodeId::new();
    let mut raft = RaftConsensus::new(config, node_id, vec![node_id]).unwrap();

    // Start and become leader
    raft.start().await.unwrap();
    raft.start_election().await.unwrap();

    // Propose multiple values
    for i in 0..5 {
        let value = ConsensusValue::new(format!("value_{}", i).into_bytes(), node_id);
        let result = raft.propose(value.clone()).await.unwrap();
        
        assert_eq!(result.sequence, i + 1);
        assert_eq!(result.term, 1);
    }

    // Check final state
    let state = raft.get_state().await;
    assert_eq!(state.last_committed, 5);
    assert_eq!(state.role, ConsensusRole::Leader);
}

/// Test consensus with term changes
#[tokio::test]
async fn test_term_changes() {
    let config = test_raft_config();

    let node1 = NodeId::new();
    let node2 = NodeId::new();
    let mut raft = RaftConsensus::new(config, node1, vec![node1, node2]).unwrap();

    raft.start().await.unwrap();

    // Start election (term becomes 1)
    raft.start_election().await.unwrap();
    assert_eq!(raft.get_state().await.term, 1);

    // Receive message with higher term
    let higher_term_message = RaftMessage::RequestVote {
        term: 5,
        candidate_id: node2,
        last_log_index: 0,
        last_log_term: 0,
    };

    raft.handle_message(higher_term_message).await.unwrap();

    // Should update to higher term and become follower
    let state = raft.get_state().await;
    assert_eq!(state.term, 5);
    assert_eq!(state.role, ConsensusRole::Follower);
}

/// Test consensus statistics
#[tokio::test]
async fn test_consensus_statistics() {
    let config = test_raft_config();

    let node_id = NodeId::new();
    let mut raft = RaftConsensus::new(config, node_id, vec![node_id]).unwrap();

    raft.start().await.unwrap();
    raft.start_election().await.unwrap();

    // Get initial stats
    let stats = raft.get_stats().await;
    assert_eq!(stats.elections_held, 1);
    assert_eq!(stats.log_size, 0); // No entries yet

    // Propose some values
    for i in 0..3 {
        let value = ConsensusValue::new(format!("test_{}", i).into_bytes(), node_id);
        raft.propose(value).await.unwrap();
    }

    let final_stats = raft.get_stats().await;
    assert_eq!(final_stats.entries_committed, 3);
}
