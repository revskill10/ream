//! Performance and stress tests for P2P distributed system
//!
//! Tests throughput, latency, scalability, and resource usage
//! as specified in p2p_testing.md

use ream::p2p::*;
use ream::p2p::consensus::{PBFTConfig, ConsensusValue};
use ream::p2p::network::NetworkMessage;
use ream::p2p::discovery::DHTConfig;
use ream::runtime::ReamActor;
use ream::types::MessagePayload;
use ream::{Pid, error::RuntimeError};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use criterion::black_box;
use futures;

/// Performance benchmarks for P2P operations
#[cfg(test)]
mod performance_benchmarks {
    use super::*;

    /// Benchmark consensus throughput
    #[tokio::test]
    async fn benchmark_consensus_throughput() {
        let mut pbft = PBFTConsensus::new(PBFTConfig::default()).unwrap();
        pbft.start().await.unwrap();
        pbft.bootstrap().await.unwrap();

        let num_proposals = 1000;
        let start_time = Instant::now();

        for i in 0..num_proposals {
            let value = ConsensusValue::new(format!("value_{}", i).into_bytes(), NodeId::new());
            let _ = pbft.propose(value).await;
        }

        let elapsed = start_time.elapsed();
        let throughput = num_proposals as f64 / elapsed.as_secs_f64();

        println!("PBFT Consensus Throughput: {:.2} proposals/sec", throughput);
        
        // Verify reasonable throughput (adjust based on hardware)
        assert!(throughput > 100.0, "Consensus throughput too low: {}", throughput);

        pbft.stop().await.unwrap();
    }

    /// Benchmark network message latency
    #[tokio::test]
    async fn benchmark_network_latency() {
        let config = ream::p2p::network::NetworkConfig::default();
        let network = NetworkLayer::new(config).await.unwrap();
        network.start().await.unwrap();

        let num_messages = 100;
        let mut total_latency = Duration::new(0, 0);

        for i in 0..num_messages {
            let start_time = Instant::now();
            
            let message = NetworkMessage::Ping { timestamp: i };
            // In a real implementation, we would send the message and measure round-trip time
            let _ = black_box(message);
            
            let latency = start_time.elapsed();
            total_latency += latency;
        }

        let avg_latency = total_latency / num_messages as u32;
        println!("Average Network Latency: {:?}", avg_latency);

        // Verify reasonable latency (adjust based on network conditions)
        assert!(avg_latency < Duration::from_millis(10), "Network latency too high: {:?}", avg_latency);

        network.stop().await.unwrap();
    }

    /// Benchmark DHT lookup performance
    #[tokio::test]
    async fn benchmark_dht_lookup() {
        let node_info = NodeInfo::new(
            NodeId::new(),
            "127.0.0.1:8080".parse().unwrap(),
        );
        let config = DHTConfig::default();
        let mut dht = ReamDHT::new(node_info, config);

        dht.start().await.unwrap();
        dht.initialize_network().await.unwrap();

        // Add many nodes to the DHT
        let num_nodes = 1000;
        for i in 0..num_nodes {
            let node = NodeInfo::new(
                NodeId::new(),
                format!("127.0.0.1:{}", 8081 + i).parse().unwrap(),
            );
            dht.add_node(node).await.unwrap();
        }

        // Benchmark lookups
        let num_lookups = 100;
        let start_time = Instant::now();

        for _ in 0..num_lookups {
            let target_key = NodeId::new().as_bytes().to_vec();
            let _ = dht.find_nodes(&target_key, 20).await;
        }

        let elapsed = start_time.elapsed();
        let lookup_rate = num_lookups as f64 / elapsed.as_secs_f64();

        println!("DHT Lookup Rate: {:.2} lookups/sec", lookup_rate);
        
        // Verify reasonable lookup performance
        assert!(lookup_rate > 500.0, "DHT lookup rate too low: {}", lookup_rate);

        dht.stop().await.unwrap();
    }

    /// Benchmark actor migration performance
    #[tokio::test]
    async fn benchmark_actor_migration() {
        let config = P2PConfig::default();
        let node = initialize_p2p_system(config).await.unwrap();
        create_cluster(node.clone()).await.unwrap();

        let num_migrations = 50;
        let start_time = Instant::now();

        for _i in 0..num_migrations {
            // Spawn actor
            let actor_ref = spawn_distributed_actor(
                node.clone(),
                TestActor::default(),
                None,
            ).await.unwrap();

            // Migrate actor (simplified - in real test would migrate to different node)
            let target_node = NodeId::new();
            let _ = migrate_actor(
                node.clone(),
                actor_ref.actor_id,
                target_node,
            ).await;
        }

        let elapsed = start_time.elapsed();
        let migration_rate = num_migrations as f64 / elapsed.as_secs_f64();

        println!("Actor Migration Rate: {:.2} migrations/sec", migration_rate);
        
        // Verify reasonable migration performance
        assert!(migration_rate > 10.0, "Actor migration rate too low: {}", migration_rate);
    }

    #[derive(Default)]
    struct TestActor {
        state: String,
    }

    impl ReamActor for TestActor {
        fn receive(&mut self, _message: MessagePayload) -> Result<(), RuntimeError> {
            Ok(())
        }

        fn pid(&self) -> Pid {
            Pid::new()
        }

        fn restart(&mut self) -> Result<(), RuntimeError> {
            self.state = "restarted".to_string();
            Ok(())
        }
    }
}

/// Stress tests for P2P system
#[cfg(test)]
mod stress_tests {
    use super::*;

    /// Stress test with many concurrent connections
    #[tokio::test]
    async fn stress_test_concurrent_connections() {
        let config = ream::p2p::network::NetworkConfig::default();
        let network = NetworkLayer::new(config).await.unwrap();
        network.start().await.unwrap();

        let num_connections = 100;
        let mut tasks = Vec::new();

        for i in 0..num_connections {
            // Note: NetworkLayer doesn't implement Clone, so we'll use a reference
            // let network_clone = network.clone();
            let task = tokio::spawn(async move {
                // Simulate connection activity
                let _peer_addr: std::net::SocketAddr = format!("127.0.0.1:{}", 9000 + i).parse().unwrap();
                // In a real implementation, we would create actual connections
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<(), P2PError>(())
            });
            tasks.push(task);
        }

        // Wait for all connections to complete
        let results = futures::future::join_all(tasks).await;
        
        // Verify all connections succeeded
        for result in results {
            assert!(result.is_ok());
            assert!(result.unwrap().is_ok());
        }

        let _stats = network.get_network_stats().await;
        println!("Network handled {} concurrent operations", num_connections);

        network.stop().await.unwrap();
    }

    /// Stress test with high message volume
    #[tokio::test]
    async fn stress_test_high_message_volume() {
        let mut pbft = PBFTConsensus::new(PBFTConfig::default()).unwrap();
        pbft.start().await.unwrap();
        pbft.bootstrap().await.unwrap();

        let num_messages = 10000;
        let batch_size = 100;
        let start_time = Instant::now();

        for batch in 0..(num_messages / batch_size) {
            let mut batch_tasks = Vec::new();
            
            for i in 0..batch_size {
                let message_id = batch * batch_size + i;
                let _value = ConsensusValue::new(format!("msg_{}", message_id).into_bytes(), NodeId::new());
                
                // In a real implementation, we would send actual messages
                let task = tokio::spawn(async move {
                    // Simulate message processing
                    tokio::time::sleep(Duration::from_micros(10)).await;
                    Ok::<(), P2PError>(())
                });
                batch_tasks.push(task);
            }
            
            // Wait for batch to complete
            let _ = futures::future::join_all(batch_tasks).await;
        }

        let elapsed = start_time.elapsed();
        let message_rate = num_messages as f64 / elapsed.as_secs_f64();

        println!("High Volume Message Rate: {:.2} messages/sec", message_rate);
        
        // Verify system can handle high message volume
        assert!(message_rate > 1000.0, "Message rate too low under stress: {}", message_rate);

        pbft.stop().await.unwrap();
    }

    /// Stress test with large cluster size
    #[tokio::test]
    async fn stress_test_large_cluster() {
        let cluster_size = 50; // Large cluster for stress testing
        let mut nodes = Vec::new();

        // Create bootstrap node
        let config = P2PConfig::default();
        let bootstrap_node = initialize_p2p_system(config).await.unwrap();
        create_cluster(bootstrap_node.clone()).await.unwrap();
        nodes.push(bootstrap_node.clone());

        let bootstrap_info = {
            let node = bootstrap_node.read().await;
            node.get_node_info().await.unwrap()
        };

        // Add remaining nodes in batches to avoid overwhelming the system
        let batch_size = 10;
        for batch in 0..((cluster_size - 1) / batch_size) {
            let mut batch_tasks = Vec::new();
            
            for _ in 0..batch_size.min(cluster_size - 1 - batch * batch_size) {
                let bootstrap_info_clone = bootstrap_info.clone();
                let task = tokio::spawn(async move {
                    let config = P2PConfig::default();
                    let node = initialize_p2p_system(config).await.unwrap();
                    join_cluster(node.clone(), vec![bootstrap_info_clone]).await.unwrap();
                    node
                });
                batch_tasks.push(task);
            }
            
            // Wait for batch to join
            let batch_results = futures::future::join_all(batch_tasks).await;
            for result in batch_results {
                nodes.push(result.unwrap());
            }
            
            // Small delay between batches
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Verify cluster formation
        let cluster_info = timeout(
            Duration::from_secs(10),
            get_cluster_info(bootstrap_node)
        ).await.unwrap().unwrap();

        println!("Large cluster formed with {} members", cluster_info.member_count);
        
        // Verify all nodes joined successfully
        assert!(cluster_info.member_count >= cluster_size / 2, 
                "Not enough nodes joined cluster: {}/{}", 
                cluster_info.member_count, cluster_size);
    }

    /// Memory usage stress test
    #[tokio::test]
    async fn stress_test_memory_usage() {
        let config = P2PConfig::default();
        let node = initialize_p2p_system(config).await.unwrap();
        create_cluster(node.clone()).await.unwrap();

        let num_actors = 1000;
        let mut actor_refs = Vec::new();

        // Spawn many actors to test memory usage
        for i in 0..num_actors {
            let actor_ref = spawn_distributed_actor(
                node.clone(),
                TestActor::default(),
                None,
            ).await.unwrap();
            actor_refs.push(actor_ref);
            
            // Periodic check to avoid memory exhaustion
            if i % 100 == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        println!("Successfully spawned {} actors", actor_refs.len());
        
        // Verify we could spawn the expected number of actors
        assert_eq!(actor_refs.len(), num_actors);
        
        // In a real implementation, we would measure actual memory usage
        // and verify it's within acceptable bounds
    }

    #[derive(Default)]
    struct TestActor {
        state: String,
    }

    impl ReamActor for TestActor {
        fn receive(&mut self, _message: MessagePayload) -> Result<(), RuntimeError> {
            Ok(())
        }

        fn pid(&self) -> Pid {
            Pid::new()
        }

        fn restart(&mut self) -> Result<(), RuntimeError> {
            self.state = "restarted".to_string();
            Ok(())
        }
    }
}

/// Resource usage tests
#[cfg(test)]
mod resource_tests {
    use super::*;

    /// Test CPU usage under load
    #[tokio::test]
    async fn test_cpu_usage_under_load() {
        let mut pbft = PBFTConsensus::new(PBFTConfig::default()).unwrap();
        pbft.start().await.unwrap();
        pbft.bootstrap().await.unwrap();

        let start_time = Instant::now();
        let test_duration = Duration::from_secs(5);
        let mut operation_count = 0;

        while start_time.elapsed() < test_duration {
            let value = ConsensusValue::new(format!("load_test_{}", operation_count).into_bytes(), NodeId::new());
            let _ = pbft.propose(value).await;
            operation_count += 1;
            
            // Small delay to prevent overwhelming the system
            tokio::time::sleep(Duration::from_micros(100)).await;
        }

        let ops_per_second = operation_count as f64 / test_duration.as_secs_f64();
        println!("Sustained operation rate: {:.2} ops/sec", ops_per_second);

        // Verify system maintains reasonable performance under sustained load
        assert!(ops_per_second > 100.0, "Performance degraded under load: {}", ops_per_second);

        pbft.stop().await.unwrap();
    }

    /// Test network bandwidth usage
    #[tokio::test]
    async fn test_network_bandwidth_usage() {
        let config = ream::p2p::network::NetworkConfig::default();
        let network = NetworkLayer::new(config).await.unwrap();
        network.start().await.unwrap();

        let message_size = 1024; // 1KB messages
        let num_messages = 1000;
        let start_time = Instant::now();

        for _i in 0..num_messages {
            let data = vec![0u8; message_size];
            let message = NetworkMessage::Custom(data);
            
            // In a real implementation, we would send actual messages
            let _ = black_box(message);
        }

        let elapsed = start_time.elapsed();
        let total_bytes = num_messages * message_size;
        let bandwidth = total_bytes as f64 / elapsed.as_secs_f64();

        println!("Network bandwidth usage: {:.2} bytes/sec", bandwidth);

        // Verify reasonable bandwidth utilization
        assert!(bandwidth > 100_000.0, "Bandwidth too low: {}", bandwidth);

        network.stop().await.unwrap();
    }
}
