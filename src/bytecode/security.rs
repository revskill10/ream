//! Bytecode Security Manager
//!
//! This module implements fine-grained security controls for bytecode execution
//! including permission systems, sandboxing, and resource limits.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::net::SocketAddr;
use std::time::{Duration, Instant, SystemTime};
use serde::{Deserialize, Serialize};
use crate::error::{BytecodeError, BytecodeResult};

/// Security manager for bytecode execution
pub struct SecurityManager {
    /// Granted permissions
    permissions: HashSet<Permission>,
    /// Security policy
    policy: SecurityPolicy,
    /// Resource limits
    limits: ResourceLimits,
    /// Current resource usage
    usage: ResourceUsage,
    /// Execution start time
    start_time: Instant,
}

/// Permission types for bytecode operations
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    /// Read file permission
    FileRead(PathBuf),
    /// Write file permission
    FileWrite(PathBuf),
    /// Execute file permission
    FileExecute(PathBuf),
    /// Network connect permission
    NetworkConnect(SocketAddr),
    /// Network bind permission
    NetworkBind(SocketAddr),
    /// Process spawn permission
    ProcessSpawn,
    /// System property access
    SystemProperty(String),
    /// Memory allocation permission
    MemoryAlloc(usize),
    /// Timer creation permission
    TimerCreate,
    /// Cryptographic operations
    CryptoOperation(String),
    /// All file operations (wildcard)
    FileAll,
    /// All network operations (wildcard)
    NetworkAll,
    /// All system operations (wildcard)
    SystemAll,
}

/// Security policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Default deny policy (if true, deny by default)
    default_deny: bool,
    /// Sandbox mode (if true, very restrictive)
    sandbox_mode: bool,
    /// Allowed effect grades
    allowed_effects: HashSet<String>,
    /// Blocked instruction patterns
    blocked_instructions: HashSet<String>,
    /// Maximum execution time
    max_execution_time: Option<Duration>,
    /// Enable audit logging
    audit_logging: bool,
}

/// Resource limits for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory allocation (bytes)
    max_memory: usize,
    /// Maximum file handles
    max_file_handles: u32,
    /// Maximum socket handles
    max_socket_handles: u32,
    /// Maximum timer handles
    max_timer_handles: u32,
    /// Maximum execution time
    max_execution_time: Duration,
    /// Maximum stack depth
    max_stack_depth: usize,
    /// Maximum instruction count
    max_instructions: u64,
}

/// Current resource usage tracking
#[derive(Debug, Default)]
struct ResourceUsage {
    /// Current memory usage
    memory_used: usize,
    /// Current file handles
    file_handles: u32,
    /// Current socket handles
    socket_handles: u32,
    /// Current timer handles
    timer_handles: u32,
    /// Instructions executed
    instructions_executed: u64,
    /// Current stack depth
    current_stack_depth: usize,
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event type
    pub event_type: SecurityEventType,
    /// Permission requested
    pub permission: Option<Permission>,
    /// Whether access was granted
    pub granted: bool,
    /// Additional context
    pub context: String,
}

/// Types of security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Permission check
    PermissionCheck,
    /// Resource allocation
    ResourceAllocation,
    /// Resource limit exceeded
    ResourceLimitExceeded,
    /// Execution time limit exceeded
    ExecutionTimeExceeded,
    /// Blocked instruction attempted
    BlockedInstruction,
    /// Security violation
    SecurityViolation,
}

impl SecurityManager {
    /// Create a new security manager with default policy
    pub fn new() -> Self {
        SecurityManager {
            permissions: HashSet::new(),
            policy: SecurityPolicy::default(),
            limits: ResourceLimits::default(),
            usage: ResourceUsage::default(),
            start_time: Instant::now(),
        }
    }
    
    /// Create a security manager with custom policy
    pub fn with_policy(policy: SecurityPolicy, limits: ResourceLimits) -> Self {
        SecurityManager {
            permissions: HashSet::new(),
            policy,
            limits,
            usage: ResourceUsage::default(),
            start_time: Instant::now(),
        }
    }
    
    /// Grant a permission
    pub fn grant_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }
    
    /// Revoke a permission
    pub fn revoke_permission(&mut self, permission: &Permission) {
        self.permissions.remove(permission);
    }
    
    /// Check if a permission is granted
    pub fn check_permission(&self, permission: &Permission) -> bool {
        // Check for exact permission
        if self.permissions.contains(permission) {
            return true;
        }
        
        // Check for wildcard permissions
        match permission {
            Permission::FileRead(path) | Permission::FileWrite(path) | Permission::FileExecute(path) => {
                self.permissions.contains(&Permission::FileAll)
            }
            Permission::NetworkConnect(_) | Permission::NetworkBind(_) => {
                self.permissions.contains(&Permission::NetworkAll)
            }
            Permission::ProcessSpawn | Permission::SystemProperty(_) => {
                self.permissions.contains(&Permission::SystemAll)
            }
            _ => false,
        }
    }
    
    /// Check and enforce a permission
    pub fn enforce_permission(&mut self, permission: Permission) -> BytecodeResult<()> {
        let granted = if self.policy.default_deny {
            self.check_permission(&permission)
        } else {
            !self.policy.sandbox_mode || self.check_permission(&permission)
        };
        
        if self.policy.audit_logging {
            self.log_security_event(SecurityEvent {
                timestamp: SystemTime::now(),
                event_type: SecurityEventType::PermissionCheck,
                permission: Some(permission.clone()),
                granted,
                context: format!("Permission check for {:?}", permission),
            });
        }
        
        if granted {
            Ok(())
        } else {
            Err(BytecodeError::SecurityViolation(format!(
                "Permission denied: {:?}",
                permission
            )))
        }
    }
    
    /// Check resource allocation
    pub fn check_resource_allocation(&mut self, resource_type: &str, amount: usize) -> BytecodeResult<()> {
        let would_exceed = match resource_type {
            "memory" => self.usage.memory_used + amount > self.limits.max_memory,
            "file_handle" => self.usage.file_handles + 1 > self.limits.max_file_handles,
            "socket_handle" => self.usage.socket_handles + 1 > self.limits.max_socket_handles,
            "timer_handle" => self.usage.timer_handles + 1 > self.limits.max_timer_handles,
            _ => false,
        };
        
        if would_exceed {
            if self.policy.audit_logging {
                self.log_security_event(SecurityEvent {
                    timestamp: SystemTime::now(),
                    event_type: SecurityEventType::ResourceLimitExceeded,
                    permission: None,
                    granted: false,
                    context: format!("Resource limit exceeded for {}", resource_type),
                });
            }
            
            return Err(BytecodeError::ResourceLimitExceeded(format!(
                "Resource limit exceeded for {}: requested {}, limit {}",
                resource_type,
                amount,
                match resource_type {
                    "memory" => self.limits.max_memory,
                    _ => 0,
                }
            )));
        }
        
        // Update usage
        match resource_type {
            "memory" => self.usage.memory_used += amount,
            "file_handle" => self.usage.file_handles += 1,
            "socket_handle" => self.usage.socket_handles += 1,
            "timer_handle" => self.usage.timer_handles += 1,
            _ => {}
        }
        
        Ok(())
    }
    
    /// Check execution time limit
    pub fn check_execution_time(&self) -> BytecodeResult<()> {
        let elapsed = self.start_time.elapsed();
        if elapsed > self.limits.max_execution_time {
            return Err(BytecodeError::ExecutionTimeExceeded(elapsed));
        }
        Ok(())
    }
    
    /// Check instruction count limit
    pub fn check_instruction_count(&mut self) -> BytecodeResult<()> {
        self.usage.instructions_executed += 1;
        if self.usage.instructions_executed > self.limits.max_instructions {
            return Err(BytecodeError::InstructionLimitExceeded(self.usage.instructions_executed));
        }
        Ok(())
    }
    
    /// Check stack depth limit
    pub fn check_stack_depth(&mut self, depth: usize) -> BytecodeResult<()> {
        self.usage.current_stack_depth = depth;
        if depth > self.limits.max_stack_depth {
            return Err(BytecodeError::StackOverflow(depth));
        }
        Ok(())
    }
    
    /// Check if an instruction is blocked
    pub fn is_instruction_blocked(&self, instruction_name: &str) -> bool {
        self.policy.blocked_instructions.contains(instruction_name)
    }
    
    /// Reset resource usage (for new execution)
    pub fn reset_usage(&mut self) {
        self.usage = ResourceUsage::default();
        self.start_time = Instant::now();
    }
    
    /// Get current resource usage
    pub fn get_usage(&self) -> &ResourceUsage {
        &self.usage
    }
    
    /// Get security policy
    pub fn get_policy(&self) -> &SecurityPolicy {
        &self.policy
    }
    
    /// Update security policy
    pub fn update_policy(&mut self, policy: SecurityPolicy) {
        self.policy = policy;
    }
    
    /// Log a security event
    fn log_security_event(&self, event: SecurityEvent) {
        // In a real implementation, this would write to an audit log
        eprintln!("SECURITY EVENT: {:?}", event);
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        let mut allowed_effects = HashSet::new();
        allowed_effects.insert("Pure".to_string());
        allowed_effects.insert("IO".to_string());
        
        SecurityPolicy {
            default_deny: false,
            sandbox_mode: false,
            allowed_effects,
            blocked_instructions: HashSet::new(),
            max_execution_time: Some(Duration::from_secs(30)),
            audit_logging: true,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
            max_memory: 100 * 1024 * 1024, // 100 MB
            max_file_handles: 10,
            max_socket_handles: 5,
            max_timer_handles: 20,
            max_execution_time: Duration::from_secs(30),
            max_stack_depth: 1000,
            max_instructions: 1_000_000,
        }
    }
}

/// Create a sandboxed security manager
pub fn create_sandbox_manager() -> SecurityManager {
    let policy = SecurityPolicy {
        default_deny: true,
        sandbox_mode: true,
        allowed_effects: {
            let mut effects = HashSet::new();
            effects.insert("Pure".to_string());
            effects
        },
        blocked_instructions: {
            let mut blocked = HashSet::new();
            blocked.insert("file_open".to_string());
            blocked.insert("socket_create".to_string());
            blocked.insert("spawn_process".to_string());
            blocked
        },
        max_execution_time: Some(Duration::from_secs(5)),
        audit_logging: true,
    };
    
    let limits = ResourceLimits {
        max_memory: 10 * 1024 * 1024, // 10 MB
        max_file_handles: 0,
        max_socket_handles: 0,
        max_timer_handles: 5,
        max_execution_time: Duration::from_secs(5),
        max_stack_depth: 100,
        max_instructions: 100_000,
    };
    
    SecurityManager::with_policy(policy, limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_check() {
        let mut manager = SecurityManager::new();
        
        let permission = Permission::FileRead(PathBuf::from("/tmp/test.txt"));
        assert!(!manager.check_permission(&permission));
        
        manager.grant_permission(permission.clone());
        assert!(manager.check_permission(&permission));
    }

    #[test]
    fn test_wildcard_permissions() {
        let mut manager = SecurityManager::new();
        manager.grant_permission(Permission::FileAll);
        
        let read_permission = Permission::FileRead(PathBuf::from("/tmp/test.txt"));
        assert!(manager.check_permission(&read_permission));
        
        let write_permission = Permission::FileWrite(PathBuf::from("/tmp/test.txt"));
        assert!(manager.check_permission(&write_permission));
    }

    #[test]
    fn test_resource_limits() {
        let mut manager = SecurityManager::new();
        
        // Should succeed within limits
        assert!(manager.check_resource_allocation("memory", 1024).is_ok());
        
        // Should fail when exceeding limits
        assert!(manager.check_resource_allocation("memory", usize::MAX).is_err());
    }

    #[test]
    fn test_sandbox_manager() {
        let manager = create_sandbox_manager();
        
        assert!(manager.policy.sandbox_mode);
        assert!(manager.policy.default_deny);
        assert_eq!(manager.limits.max_memory, 10 * 1024 * 1024);
        assert!(manager.is_instruction_blocked("file_open"));
    }
}
