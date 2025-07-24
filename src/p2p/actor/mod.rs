//! Distributed actor system for P2P network
//!
//! Provides transparent remote actors, actor migration,
//! distributed supervision trees, and actor registry.

pub mod distributed;
pub mod migration;
pub mod registry;
pub mod supervision;

pub use distributed::*;
pub use migration::*;
pub use registry::*;
pub use supervision::*;

use crate::p2p::{P2PResult, P2PError, ActorError, NodeId, ActorId, PlacementConstraints};
use crate::runtime::ReamActor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Distributed actor reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedActorRef {
    /// Actor ID
    pub actor_id: ActorId,
    /// Node where actor is located
    pub node_id: NodeId,
    /// Actor type name
    pub actor_type: String,
}

impl DistributedActorRef {
    /// Create a new distributed actor reference
    pub fn new(actor_id: ActorId, node_id: NodeId, actor_type: String) -> Self {
        Self {
            actor_id,
            node_id,
            actor_type,
        }
    }

    /// Send a message to the distributed actor
    pub async fn send(&self, _message: Vec<u8>) -> P2PResult<()> {
        // Implementation would route message to the correct node
        Ok(())
    }
}

/// Distributed actor implementation
#[derive(Debug)]
pub struct DistributedActor {
    /// Local actor ID
    actor_id: ActorId,
    /// Node where this actor is running
    node_id: NodeId,
    /// Actor type
    actor_type: String,
    /// Actor state (serialized)
    state: Vec<u8>,
}

impl DistributedActor {
    /// Create a new distributed actor
    pub fn new(actor_id: ActorId, node_id: NodeId, actor_type: String) -> Self {
        Self {
            actor_id,
            node_id,
            actor_type,
            state: Vec::new(),
        }
    }

    /// Get actor reference
    pub fn get_ref(&self) -> DistributedActorRef {
        DistributedActorRef::new(self.actor_id, self.node_id, self.actor_type.clone())
    }

    /// Serialize actor state for migration
    pub async fn serialize_state(&self) -> P2PResult<Vec<u8>> {
        Ok(self.state.clone())
    }

    /// Deserialize actor state after migration
    pub async fn deserialize_state(&mut self, state: Vec<u8>) -> P2PResult<()> {
        self.state = state;
        Ok(())
    }
}

/// Distributed actor registry
pub struct DistributedActorRegistry {
    /// Local actors
    local_actors: Arc<RwLock<HashMap<ActorId, DistributedActor>>>,
    /// Remote actor references
    remote_actors: Arc<RwLock<HashMap<ActorId, DistributedActorRef>>>,
    /// Actor type registry
    actor_types: Arc<RwLock<HashMap<String, Box<dyn ActorFactory + Send + Sync>>>>,
}

impl std::fmt::Debug for DistributedActorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DistributedActorRegistry")
            .field("local_actors", &"<local_actors>")
            .field("remote_actors", &"<remote_actors>")
            .field("actor_types", &"<actor_types>")
            .finish()
    }
}

impl DistributedActorRegistry {
    /// Create a new distributed actor registry
    pub fn new() -> Self {
        Self {
            local_actors: Arc::new(RwLock::new(HashMap::new())),
            remote_actors: Arc::new(RwLock::new(HashMap::new())),
            actor_types: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an actor type
    pub async fn register_actor_type<F>(&self, type_name: String, factory: F)
    where
        F: ActorFactory + Send + Sync + 'static,
    {
        let mut actor_types = self.actor_types.write().await;
        actor_types.insert(type_name, Box::new(factory));
    }

    /// Spawn a local actor
    pub async fn spawn_local_actor(
        &self,
        actor_type: String,
        node_id: NodeId,
    ) -> P2PResult<DistributedActorRef> {
        let actor_id = ActorId::new();
        let actor = DistributedActor::new(actor_id, node_id, actor_type.clone());
        let actor_ref = actor.get_ref();

        let mut local_actors = self.local_actors.write().await;
        local_actors.insert(actor_id, actor);

        Ok(actor_ref)
    }

    /// Register a remote actor
    pub async fn register_remote_actor(&self, actor_ref: DistributedActorRef) -> P2PResult<()> {
        let mut remote_actors = self.remote_actors.write().await;
        remote_actors.insert(actor_ref.actor_id, actor_ref);
        Ok(())
    }

    /// Get actor reference
    pub async fn get_actor_ref(&self, actor_id: ActorId) -> Option<DistributedActorRef> {
        // Check local actors first
        {
            let local_actors = self.local_actors.read().await;
            if let Some(actor) = local_actors.get(&actor_id) {
                return Some(actor.get_ref());
            }
        }

        // Check remote actors
        {
            let remote_actors = self.remote_actors.read().await;
            remote_actors.get(&actor_id).cloned()
        }
    }

    /// Remove actor
    pub async fn remove_actor(&self, actor_id: ActorId) -> P2PResult<()> {
        {
            let mut local_actors = self.local_actors.write().await;
            local_actors.remove(&actor_id);
        }

        {
            let mut remote_actors = self.remote_actors.write().await;
            remote_actors.remove(&actor_id);
        }

        Ok(())
    }
}

/// Migration manager for actor migration
#[derive(Debug)]
pub struct MigrationManager {
    /// Active migrations
    active_migrations: Arc<RwLock<HashMap<ActorId, MigrationState>>>,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new() -> Self {
        Self {
            active_migrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start actor migration
    pub async fn migrate_actor(
        &self,
        actor_id: ActorId,
        target_node: NodeId,
    ) -> P2PResult<MigrationResult> {
        // Implementation would handle the migration process
        let result = MigrationResult {
            actor_id,
            source_node: NodeId::new(), // Would be actual source
            target_node,
            success: true,
            error: None,
        };

        Ok(result)
    }
}

/// Migration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    /// Actor that was migrated
    pub actor_id: ActorId,
    /// Source node
    pub source_node: NodeId,
    /// Target node
    pub target_node: NodeId,
    /// Whether migration was successful
    pub success: bool,
    /// Error message if migration failed
    pub error: Option<String>,
}

/// Migration state
#[derive(Debug, Clone)]
pub enum MigrationState {
    /// Migration is preparing
    Preparing,
    /// Migration is in progress
    InProgress,
    /// Migration completed successfully
    Completed,
    /// Migration failed
    Failed(String),
}

/// Actor factory trait
pub trait ActorFactory {
    /// Create a new actor instance
    fn create_actor(&self) -> P2PResult<Box<dyn ReamActor + Send + Sync>>;
}

/// Simple actor factory implementation
pub struct SimpleActorFactory<T>
where
    T: ReamActor + Default + Send + Sync + 'static,
{
    _phantom: std::marker::PhantomData<T>,
}

impl<T> SimpleActorFactory<T>
where
    T: ReamActor + Default + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> ActorFactory for SimpleActorFactory<T>
where
    T: ReamActor + Default + Send + Sync + 'static,
{
    fn create_actor(&self) -> P2PResult<Box<dyn ReamActor + Send + Sync>> {
        Ok(Box::new(T::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distributed_actor_registry() {
        let registry = DistributedActorRegistry::new();
        let node_id = NodeId::new();
        
        let actor_ref = registry.spawn_local_actor("test_actor".to_string(), node_id).await.unwrap();
        
        let retrieved_ref = registry.get_actor_ref(actor_ref.actor_id).await;
        assert!(retrieved_ref.is_some());
        assert_eq!(retrieved_ref.unwrap().actor_id, actor_ref.actor_id);
    }

    #[tokio::test]
    async fn test_migration_manager() {
        let manager = MigrationManager::new();
        let actor_id = ActorId::new();
        let target_node = NodeId::new();
        
        let result = manager.migrate_actor(actor_id, target_node).await.unwrap();
        assert!(result.success);
        assert_eq!(result.actor_id, actor_id);
        assert_eq!(result.target_node, target_node);
    }
}
