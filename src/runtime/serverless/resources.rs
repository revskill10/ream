//! Resource pool management for serverless actors
//! 
//! Provides pre-allocated memory pools, connection pools, and resource management
//! for ultra-fast actor wake-up and optimal resource utilization.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

use crate::types::Pid;
use crate::jit::JitFunction;
use super::cold_start::{HeapSegment, StackSegment, CompiledBytecode};

/// Resource pool manager for all actor resources
pub struct ResourcePools {
    /// Memory pools by actor type
    memory_pools: Arc<RwLock<HashMap<String, MemoryPoolManager>>>,
    /// Connection pools by actor type
    connection_pools: Arc<RwLock<HashMap<String, ConnectionPool>>>,
    /// File descriptor pools by actor type
    fd_pools: Arc<RwLock<HashMap<String, FileDescriptorPool>>>,
    /// Bytecode cache
    bytecode_cache: Arc<RwLock<HashMap<String, CompiledBytecode>>>,
    /// JIT compilation cache
    jit_cache: Arc<RwLock<HashMap<String, JitFunction>>>,
    /// Resource allocation statistics
    stats: Arc<RwLock<ResourceStats>>,
    /// Pool configuration
    config: ResourcePoolConfig,
}

/// Memory pool manager for a specific actor type
#[derive(Debug)]
pub struct MemoryPoolManager {
    /// Available memory segments
    available_segments: VecDeque<MemorySegment>,
    /// In-use memory segments
    in_use_segments: HashMap<Pid, MemorySegment>,
    /// Pool capacity
    capacity: usize,
    /// Segment size
    segment_size: usize,
    /// Pool statistics
    stats: MemoryPoolStats,
}

/// Memory segment for actor allocation
#[derive(Debug, Clone)]
pub struct MemorySegment {
    /// Heap segment
    pub heap: HeapSegment,
    /// Stack segment
    pub stack: StackSegment,
    /// Allocation timestamp
    pub allocated_at: Option<Instant>,
    /// Last access timestamp
    pub last_access: Instant,
}

/// Connection pool for database/network connections
#[derive(Debug)]
pub struct ConnectionPool {
    /// Available connections
    available_connections: VecDeque<Connection>,
    /// In-use connections
    in_use_connections: HashMap<Pid, Connection>,
    /// Pool configuration
    config: ConnectionPoolConfig,
    /// Pool statistics
    stats: ConnectionPoolStats,
}

/// Generic connection wrapper
#[derive(Debug, Clone)]
pub struct Connection {
    /// Connection ID
    pub id: String,
    /// Connection type
    pub connection_type: ConnectionType,
    /// Creation timestamp
    pub created_at: Instant,
    /// Last used timestamp
    pub last_used: Instant,
    /// Connection metadata
    pub metadata: HashMap<String, String>,
}

/// Connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Database { url: String, pool_name: String },
    Redis { url: String, db: u8 },
    Http { base_url: String, timeout: Duration },
    Tcp { address: String, port: u16 },
    Custom { type_name: String, config: HashMap<String, String> },
}

/// File descriptor pool
#[derive(Debug)]
pub struct FileDescriptorPool {
    /// Available file descriptors
    available_fds: VecDeque<FileDescriptor>,
    /// In-use file descriptors
    in_use_fds: HashMap<Pid, Vec<FileDescriptor>>,
    /// Pool configuration
    config: FileDescriptorPoolConfig,
    /// Pool statistics
    stats: FileDescriptorPoolStats,
}

/// File descriptor wrapper
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    /// File descriptor number
    pub fd: i32,
    /// File path (if applicable)
    pub path: Option<String>,
    /// File type
    pub fd_type: FileDescriptorType,
    /// Creation timestamp
    pub created_at: Instant,
}

/// File descriptor types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileDescriptorType {
    File,
    Socket,
    Pipe,
    Device,
    Other,
}

/// Actor resources bundle
#[derive(Debug)]
pub struct ActorResources {
    /// Memory segment
    pub memory: MemorySegment,
    /// Pre-compiled bytecode
    pub bytecode: CompiledBytecode,
    /// JIT compiled function
    pub jit_code: JitFunction,
    /// Database connection (optional)
    pub connection: Option<Connection>,
    /// File descriptors (optional)
    pub file_descriptors: Vec<FileDescriptor>,
}

/// Resource pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePoolConfig {
    /// Memory pool configurations by actor type
    pub memory_pools: HashMap<String, MemoryPoolConfig>,
    /// Connection pool configurations by actor type
    pub connection_pools: HashMap<String, ConnectionPoolConfig>,
    /// File descriptor pool configurations by actor type
    pub fd_pools: HashMap<String, FileDescriptorPoolConfig>,
    /// Enable resource pre-warming
    pub pre_warming_enabled: bool,
    /// Resource cleanup interval
    pub cleanup_interval: Duration,
    /// Maximum idle time before cleanup
    pub max_idle_time: Duration,
}

/// Memory pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolConfig {
    /// Initial pool size
    pub initial_size: usize,
    /// Maximum pool size
    pub max_size: usize,
    /// Memory segment size
    pub segment_size: usize,
    /// Growth factor when expanding
    pub growth_factor: f64,
    /// Shrink threshold (utilization percentage)
    pub shrink_threshold: f64,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Minimum connections
    pub min_connections: usize,
    /// Maximum connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Idle timeout
    pub idle_timeout: Duration,
    /// Connection validation interval
    pub validation_interval: Duration,
}

/// File descriptor pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDescriptorPoolConfig {
    /// Maximum file descriptors
    pub max_fds: usize,
    /// File descriptor types to pre-allocate
    pub pre_allocate_types: Vec<FileDescriptorType>,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

/// Resource allocation statistics
#[derive(Debug, Default, Clone)]
pub struct ResourceStats {
    /// Memory allocations
    pub memory_allocations: u64,
    /// Memory deallocations
    pub memory_deallocations: u64,
    /// Connection allocations
    pub connection_allocations: u64,
    /// Connection deallocations
    pub connection_deallocations: u64,
    /// File descriptor allocations
    pub fd_allocations: u64,
    /// File descriptor deallocations
    pub fd_deallocations: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Pool hits
    pub pool_hits: u64,
    /// Pool misses
    pub pool_misses: u64,
}

/// Memory pool statistics
#[derive(Debug, Default, Clone)]
pub struct MemoryPoolStats {
    /// Total allocations
    pub allocations: u64,
    /// Total deallocations
    pub deallocations: u64,
    /// Current utilization
    pub current_utilization: f64,
    /// Peak utilization
    pub peak_utilization: f64,
    /// Average allocation time
    pub avg_allocation_time: Duration,
}

/// Connection pool statistics
#[derive(Debug, Default, Clone)]
pub struct ConnectionPoolStats {
    /// Active connections
    pub active_connections: usize,
    /// Total connections created
    pub total_created: u64,
    /// Total connections closed
    pub total_closed: u64,
    /// Connection timeouts
    pub timeouts: u64,
    /// Connection errors
    pub errors: u64,
}

/// File descriptor pool statistics
#[derive(Debug, Default, Clone)]
pub struct FileDescriptorPoolStats {
    /// Active file descriptors
    pub active_fds: usize,
    /// Total file descriptors created
    pub total_created: u64,
    /// Total file descriptors closed
    pub total_closed: u64,
    /// File descriptor leaks detected
    pub leaks_detected: u64,
}

/// Resource pool errors
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Pool not found for actor type: {0}")]
    PoolNotFound(String),
    
    #[error("Pool exhausted for actor type: {0}")]
    PoolExhausted(String),
    
    #[error("Resource allocation failed: {0}")]
    AllocationFailed(String),
    
    #[error("Resource deallocation failed: {0}")]
    DeallocationFailed(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("File descriptor error: {0}")]
    FileDescriptor(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
}

pub type PoolResult<T> = Result<T, PoolError>;

impl Default for ResourcePoolConfig {
    fn default() -> Self {
        let mut memory_pools = HashMap::new();
        memory_pools.insert("web-handler".to_string(), MemoryPoolConfig {
            initial_size: 100,
            max_size: 1000,
            segment_size: 64 * 1024, // 64KB
            growth_factor: 1.5,
            shrink_threshold: 25.0,
        });

        let mut connection_pools = HashMap::new();
        connection_pools.insert("database".to_string(), ConnectionPoolConfig {
            min_connections: 5,
            max_connections: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            validation_interval: Duration::from_secs(60),
        });

        let mut fd_pools = HashMap::new();
        fd_pools.insert("file-handler".to_string(), FileDescriptorPoolConfig {
            max_fds: 1000,
            pre_allocate_types: vec![FileDescriptorType::File, FileDescriptorType::Socket],
            cleanup_interval: Duration::from_secs(60),
        });

        ResourcePoolConfig {
            memory_pools,
            connection_pools,
            fd_pools,
            pre_warming_enabled: true,
            cleanup_interval: Duration::from_secs(300),
            max_idle_time: Duration::from_secs(600),
        }
    }
}

impl ResourcePools {
    /// Create a new resource pool manager
    pub fn new(config: ResourcePoolConfig) -> Self {
        ResourcePools {
            memory_pools: Arc::new(RwLock::new(HashMap::new())),
            connection_pools: Arc::new(RwLock::new(HashMap::new())),
            fd_pools: Arc::new(RwLock::new(HashMap::new())),
            bytecode_cache: Arc::new(RwLock::new(HashMap::new())),
            jit_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ResourceStats::default())),
            config,
        }
    }

    /// Pre-warm resources for an actor type
    pub fn pre_warm(&self, actor_type: &str) -> PoolResult<()> {
        if !self.config.pre_warming_enabled {
            return Ok(());
        }

        // Pre-warm memory pool
        if let Some(memory_config) = self.config.memory_pools.get(actor_type) {
            let memory_pool = MemoryPoolManager::new(memory_config.clone())?;
            self.memory_pools.write().unwrap().insert(actor_type.to_string(), memory_pool);
        }

        // Pre-warm connection pool
        if let Some(conn_config) = self.config.connection_pools.get(actor_type) {
            let conn_pool = ConnectionPool::new(conn_config.clone())?;
            self.connection_pools.write().unwrap().insert(actor_type.to_string(), conn_pool);
        }

        // Pre-warm file descriptor pool
        if let Some(fd_config) = self.config.fd_pools.get(actor_type) {
            let fd_pool = FileDescriptorPool::new(fd_config.clone())?;
            self.fd_pools.write().unwrap().insert(actor_type.to_string(), fd_pool);
        }

        Ok(())
    }

    /// Allocate resources for an actor
    pub fn allocate_for_actor(&self, actor_type: &str, pid: Pid) -> PoolResult<ActorResources> {
        let start = Instant::now();

        // Allocate memory
        let memory = {
            let mut pools = self.memory_pools.write().unwrap();
            let pool = pools.get_mut(actor_type)
                .ok_or_else(|| PoolError::PoolNotFound(actor_type.to_string()))?;
            pool.allocate(pid)?
        };

        // Get bytecode from cache
        let bytecode = {
            let cache = self.bytecode_cache.read().unwrap();
            cache.get(actor_type)
                .ok_or_else(|| PoolError::AllocationFailed("Bytecode not found".to_string()))?
                .clone()
        };

        // Get JIT code from cache
        let jit_code = {
            let cache = self.jit_cache.read().unwrap();
            cache.get(actor_type)
                .ok_or_else(|| PoolError::AllocationFailed("JIT code not found".to_string()))?
                .clone()
        };

        // Optionally allocate connection
        let connection = {
            let mut pools = self.connection_pools.write().unwrap();
            if let Some(pool) = pools.get_mut(actor_type) {
                pool.allocate(pid).ok()
            } else {
                None
            }
        };

        // Optionally allocate file descriptors
        let file_descriptors = {
            let mut pools = self.fd_pools.write().unwrap();
            if let Some(pool) = pools.get_mut(actor_type) {
                pool.allocate_multiple(pid, 5).unwrap_or_default()
            } else {
                Vec::new()
            }
        };

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.memory_allocations += 1;
            if connection.is_some() {
                stats.connection_allocations += 1;
            }
            stats.fd_allocations += file_descriptors.len() as u64;
            stats.pool_hits += 1;
        }

        Ok(ActorResources {
            memory,
            bytecode,
            jit_code,
            connection,
            file_descriptors,
        })
    }

    /// Deallocate resources for an actor
    pub fn deallocate_for_actor(&self, actor_type: &str, pid: Pid) -> PoolResult<()> {
        // Deallocate memory
        {
            let mut pools = self.memory_pools.write().unwrap();
            if let Some(pool) = pools.get_mut(actor_type) {
                pool.deallocate(pid)?;
            }
        }

        // Deallocate connection
        {
            let mut pools = self.connection_pools.write().unwrap();
            if let Some(pool) = pools.get_mut(actor_type) {
                pool.deallocate(pid)?;
            }
        }

        // Deallocate file descriptors
        {
            let mut pools = self.fd_pools.write().unwrap();
            if let Some(pool) = pools.get_mut(actor_type) {
                pool.deallocate_all(pid)?;
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.memory_deallocations += 1;
            stats.connection_deallocations += 1;
        }

        Ok(())
    }

    /// Get resource statistics
    pub fn get_stats(&self) -> ResourceStats {
        self.stats.read().unwrap().clone()
    }

    /// Cache bytecode for an actor type
    pub fn cache_bytecode(&self, actor_type: String, bytecode: CompiledBytecode) {
        self.bytecode_cache.write().unwrap().insert(actor_type, bytecode);
    }

    /// Cache JIT code for an actor type
    pub fn cache_jit_code(&self, actor_type: String, jit_code: JitFunction) {
        self.jit_cache.write().unwrap().insert(actor_type, jit_code);
    }
}

impl MemoryPoolManager {
    /// Create a new memory pool manager
    pub fn new(config: MemoryPoolConfig) -> PoolResult<Self> {
        let mut available_segments = VecDeque::new();

        // Pre-allocate initial segments
        for _ in 0..config.initial_size {
            let segment = MemorySegment::new(config.segment_size)?;
            available_segments.push_back(segment);
        }

        Ok(MemoryPoolManager {
            available_segments,
            in_use_segments: HashMap::new(),
            capacity: config.max_size,
            segment_size: config.segment_size,
            stats: MemoryPoolStats::default(),
        })
    }

    /// Allocate a memory segment for an actor
    pub fn allocate(&mut self, pid: Pid) -> PoolResult<MemorySegment> {
        let start = Instant::now();

        let mut segment = self.available_segments.pop_front()
            .ok_or(PoolError::PoolExhausted("Memory pool exhausted".to_string()))?;

        segment.allocated_at = Some(start);
        segment.last_access = start;

        self.in_use_segments.insert(pid, segment.clone());

        // Update statistics
        self.stats.allocations += 1;
        self.stats.avg_allocation_time =
            (self.stats.avg_allocation_time + start.elapsed()) / 2;
        self.update_utilization();

        Ok(segment)
    }

    /// Deallocate a memory segment
    pub fn deallocate(&mut self, pid: Pid) -> PoolResult<()> {
        let mut segment = self.in_use_segments.remove(&pid)
            .ok_or(PoolError::DeallocationFailed("Segment not found".to_string()))?;

        // Reset the segment
        segment.heap.reset();
        segment.stack.reset();
        segment.allocated_at = None;

        self.available_segments.push_back(segment);

        // Update statistics
        self.stats.deallocations += 1;
        self.update_utilization();

        Ok(())
    }

    /// Update utilization statistics
    fn update_utilization(&mut self) {
        let utilization = (self.in_use_segments.len() as f64 / self.capacity as f64) * 100.0;
        self.stats.current_utilization = utilization;
        if utilization > self.stats.peak_utilization {
            self.stats.peak_utilization = utilization;
        }
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> MemoryPoolStats {
        self.stats.clone()
    }
}

impl MemorySegment {
    /// Create a new memory segment
    pub fn new(size: usize) -> PoolResult<Self> {
        let heap = HeapSegment::new(size / 2)
            .map_err(|e| PoolError::AllocationFailed(format!("Heap allocation failed: {}", e)))?;
        let stack = StackSegment::new(size / 4)
            .map_err(|e| PoolError::AllocationFailed(format!("Stack allocation failed: {}", e)))?;

        Ok(MemorySegment {
            heap,
            stack,
            allocated_at: None,
            last_access: Instant::now(),
        })
    }
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> PoolResult<Self> {
        let mut available_connections = VecDeque::new();

        // Pre-allocate minimum connections
        for i in 0..config.min_connections {
            let connection = Connection::new(
                format!("conn_{}", i),
                ConnectionType::Database {
                    url: "placeholder://localhost".to_string(),
                    pool_name: "default".to_string(),
                }
            );
            available_connections.push_back(connection);
        }

        Ok(ConnectionPool {
            available_connections,
            in_use_connections: HashMap::new(),
            config,
            stats: ConnectionPoolStats::default(),
        })
    }

    /// Allocate a connection
    pub fn allocate(&mut self, pid: Pid) -> PoolResult<Connection> {
        let connection = self.available_connections.pop_front()
            .ok_or(PoolError::PoolExhausted("Connection pool exhausted".to_string()))?;

        self.in_use_connections.insert(pid, connection.clone());
        self.stats.active_connections += 1;

        Ok(connection)
    }

    /// Deallocate a connection
    pub fn deallocate(&mut self, pid: Pid) -> PoolResult<()> {
        let mut connection = self.in_use_connections.remove(&pid)
            .ok_or(PoolError::DeallocationFailed("Connection not found".to_string()))?;

        connection.last_used = Instant::now();
        self.available_connections.push_back(connection);
        self.stats.active_connections -= 1;

        Ok(())
    }
}

impl Connection {
    /// Create a new connection
    pub fn new(id: String, connection_type: ConnectionType) -> Self {
        Connection {
            id,
            connection_type,
            created_at: Instant::now(),
            last_used: Instant::now(),
            metadata: HashMap::new(),
        }
    }
}

impl FileDescriptorPool {
    /// Create a new file descriptor pool
    pub fn new(config: FileDescriptorPoolConfig) -> PoolResult<Self> {
        Ok(FileDescriptorPool {
            available_fds: VecDeque::new(),
            in_use_fds: HashMap::new(),
            config,
            stats: FileDescriptorPoolStats::default(),
        })
    }

    /// Allocate multiple file descriptors
    pub fn allocate_multiple(&mut self, pid: Pid, count: usize) -> PoolResult<Vec<FileDescriptor>> {
        let mut fds = Vec::new();

        for i in 0..count {
            if let Some(fd) = self.available_fds.pop_front() {
                fds.push(fd);
            } else {
                // Create new file descriptor
                let fd = FileDescriptor {
                    fd: (1000 + i) as i32, // Placeholder FD number
                    path: None,
                    fd_type: FileDescriptorType::File,
                    created_at: Instant::now(),
                };
                fds.push(fd);
            }
        }

        self.in_use_fds.insert(pid, fds.clone());
        self.stats.active_fds += fds.len();

        Ok(fds)
    }

    /// Deallocate all file descriptors for an actor
    pub fn deallocate_all(&mut self, pid: Pid) -> PoolResult<()> {
        if let Some(fds) = self.in_use_fds.remove(&pid) {
            self.stats.active_fds -= fds.len();
            for fd in fds {
                self.available_fds.push_back(fd);
            }
        }
        Ok(())
    }
}
