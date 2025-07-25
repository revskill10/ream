//! TLisp Security and Verification Integration
//!
//! This module integrates the security manager and bytecode verification
//! system with TLisp programs to ensure safe execution of untrusted code.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::bytecode::{
    SecurityManager, BytecodeVerifier, BytecodeProgram, SecurityPolicy,
    Permission, ResourceLimits, VerificationError
};
use crate::tlisp::{Expr, Value as TlispValue, Type, EnhancedTlispCompiler};
use crate::error::{TlispError, TlispResult, BytecodeError};

/// Security levels for TLisp programs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TlispSecurityLevel {
    /// Full access - no restrictions
    Trusted,
    /// Limited access - some operations restricted
    Restricted,
    /// Sandboxed - minimal permissions only
    Sandboxed,
}

/// TLisp-specific security policy
#[derive(Debug, Clone, PartialEq)]
pub struct TlispSecurityPolicy {
    /// Allowed TLisp functions
    pub allowed_functions: Vec<String>,
    /// Blocked TLisp functions
    pub blocked_functions: Vec<String>,
    /// Maximum recursion depth
    pub max_recursion_depth: usize,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Memory limits
    pub memory_limits: TlispMemoryLimits,
    /// I/O permissions
    pub io_permissions: TlispIOPermissions,
    /// Network permissions
    pub network_permissions: TlispNetworkPermissions,
}

/// Memory limits for TLisp programs
#[derive(Debug, Clone, PartialEq)]
pub struct TlispMemoryLimits {
    /// Maximum heap allocation
    pub max_heap: usize,
    /// Maximum stack depth
    pub max_stack_depth: usize,
    /// Maximum number of variables
    pub max_variables: usize,
    /// Maximum string length
    pub max_string_length: usize,
    /// Maximum list length
    pub max_list_length: usize,
}

/// I/O permissions for TLisp programs
#[derive(Debug, Clone, PartialEq)]
pub struct TlispIOPermissions {
    /// Allow file reading
    pub allow_file_read: bool,
    /// Allow file writing
    pub allow_file_write: bool,
    /// Allowed file paths (if any)
    pub allowed_paths: Vec<String>,
    /// Allow console I/O
    pub allow_console: bool,
}

/// Network permissions for TLisp programs
#[derive(Debug, Clone, PartialEq)]
pub struct TlispNetworkPermissions {
    /// Allow outbound connections
    pub allow_outbound: bool,
    /// Allow inbound connections
    pub allow_inbound: bool,
    /// Allowed hosts
    pub allowed_hosts: Vec<String>,
    /// Allowed ports
    pub allowed_ports: Vec<u16>,
}

/// TLisp security manager
pub struct TlispSecurityManager {
    /// Underlying security manager
    security_manager: SecurityManager,
    /// Bytecode verifier
    verifier: BytecodeVerifier,
    /// Security policies by level
    policies: HashMap<TlispSecurityLevel, TlispSecurityPolicy>,
    /// Audit log
    audit_log: Vec<TlispAuditEvent>,
    /// Execution statistics
    stats: TlispSecurityStats,
}

/// TLisp audit event
#[derive(Debug, Clone)]
pub struct TlispAuditEvent {
    /// Timestamp
    pub timestamp: Instant,
    /// Event type
    pub event_type: TlispAuditEventType,
    /// Program identifier
    pub program_id: String,
    /// Security level
    pub security_level: TlispSecurityLevel,
    /// Details
    pub details: String,
}

/// TLisp audit event types
#[derive(Debug, Clone)]
pub enum TlispAuditEventType {
    /// Program verification started
    VerificationStarted,
    /// Program verification completed
    VerificationCompleted,
    /// Program verification failed
    VerificationFailed,
    /// Security violation detected
    SecurityViolation,
    /// Permission denied
    PermissionDenied,
    /// Resource limit exceeded
    ResourceLimitExceeded,
    /// Execution started
    ExecutionStarted,
    /// Execution completed
    ExecutionCompleted,
    /// Execution failed
    ExecutionFailed,
}

/// TLisp security statistics
#[derive(Debug, Clone)]
pub struct TlispSecurityStats {
    /// Total programs verified
    pub programs_verified: u64,
    /// Verification failures
    pub verification_failures: u64,
    /// Security violations
    pub security_violations: u64,
    /// Permission denials
    pub permission_denials: u64,
    /// Resource limit violations
    pub resource_limit_violations: u64,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Average verification time
    pub avg_verification_time: Duration,
}

impl Default for TlispSecurityPolicy {
    fn default() -> Self {
        TlispSecurityPolicy {
            allowed_functions: vec![
                // Basic arithmetic
                "+".to_string(), "-".to_string(), "*".to_string(), "/".to_string(),
                // Comparison
                "=".to_string(), "<".to_string(), ">".to_string(),
                // List operations
                "list".to_string(), "car".to_string(), "cdr".to_string(),
                // Control flow
                "if".to_string(), "cond".to_string(), "begin".to_string(),
                // Safe I/O
                "print".to_string(),
            ],
            blocked_functions: vec![
                // Dangerous operations
                "eval".to_string(), "load".to_string(), "require".to_string(),
                // File operations
                "file-open".to_string(), "file-write".to_string(), "file-delete".to_string(),
                // Network operations
                "socket-open".to_string(), "http-request".to_string(),
                // System operations
                "system".to_string(), "exec".to_string(), "exit".to_string(),
            ],
            max_recursion_depth: 100,
            max_execution_time: Duration::from_secs(5),
            memory_limits: TlispMemoryLimits::default(),
            io_permissions: TlispIOPermissions::default(),
            network_permissions: TlispNetworkPermissions::default(),
        }
    }
}

impl Default for TlispMemoryLimits {
    fn default() -> Self {
        TlispMemoryLimits {
            max_heap: 10 * 1024 * 1024, // 10 MB
            max_stack_depth: 1000,
            max_variables: 10000,
            max_string_length: 1024 * 1024, // 1 MB
            max_list_length: 100000,
        }
    }
}

impl Default for TlispIOPermissions {
    fn default() -> Self {
        TlispIOPermissions {
            allow_file_read: false,
            allow_file_write: false,
            allowed_paths: vec![],
            allow_console: true,
        }
    }
}

impl Default for TlispNetworkPermissions {
    fn default() -> Self {
        TlispNetworkPermissions {
            allow_outbound: false,
            allow_inbound: false,
            allowed_hosts: vec![],
            allowed_ports: vec![],
        }
    }
}

impl TlispSecurityManager {
    /// Create a new TLisp security manager
    pub fn new() -> Self {
        let mut manager = TlispSecurityManager {
            security_manager: SecurityManager::new(),
            verifier: BytecodeVerifier::new(),
            policies: HashMap::new(),
            audit_log: Vec::new(),
            stats: TlispSecurityStats::default(),
        };
        
        // Set up default policies
        manager.setup_default_policies();
        
        manager
    }
    
    /// Create a sandboxed security manager
    pub fn sandboxed() -> Self {
        let mut manager = Self::new();
        
        // Configure for maximum security
        manager.security_manager = crate::bytecode::create_sandbox_manager();
        
        manager
    }
    
    /// Verify a TLisp program
    pub fn verify_program(
        &mut self,
        program_id: String,
        expr: &Expr<Type>,
        security_level: TlispSecurityLevel,
    ) -> TlispResult<BytecodeProgram> {
        let start_time = Instant::now();
        
        // Log verification start
        self.log_audit_event(TlispAuditEvent {
            timestamp: start_time,
            event_type: TlispAuditEventType::VerificationStarted,
            program_id: program_id.clone(),
            security_level: security_level.clone(),
            details: "Starting program verification".to_string(),
        });
        
        // Check function permissions
        self.check_function_permissions(expr, &security_level)?;
        
        // Compile to bytecode
        let mut compiler = EnhancedTlispCompiler::new(program_id.clone());
        compiler.compile_expr(expr)
            .map_err(|e| TlispError::SecurityError(format!("Compilation failed: {}", e)))?;
        
        let program = compiler.finish()
            .map_err(|e| TlispError::SecurityError(format!("Compilation failed: {}", e)))?;
        
        // Verify bytecode
        match self.verifier.verify(&program) {
            Ok(_) => {
                let verification_time = start_time.elapsed();
                
                // Update statistics
                self.stats.programs_verified += 1;
                self.stats.total_execution_time += verification_time;
                self.update_avg_verification_time();
                
                // Log success
                self.log_audit_event(TlispAuditEvent {
                    timestamp: Instant::now(),
                    event_type: TlispAuditEventType::VerificationCompleted,
                    program_id: program_id.clone(),
                    security_level,
                    details: format!("Verification completed in {:?}", verification_time),
                });
                
                Ok(program)
            }
            Err(e) => {
                // Update statistics
                self.stats.verification_failures += 1;
                
                // Log failure
                self.log_audit_event(TlispAuditEvent {
                    timestamp: Instant::now(),
                    event_type: TlispAuditEventType::VerificationFailed,
                    program_id,
                    security_level,
                    details: format!("Verification failed: {}", e),
                });
                
                Err(TlispError::SecurityError(format!("Bytecode verification failed: {}", e)))
            }
        }
    }
    
    /// Check if a function call is permitted
    pub fn check_function_permission(&self, function_name: &str, security_level: &TlispSecurityLevel) -> TlispResult<()> {
        if let Some(policy) = self.policies.get(security_level) {
            // Check if function is explicitly blocked
            if policy.blocked_functions.contains(&function_name.to_string()) {
                return Err(TlispError::SecurityError(format!("Function '{}' is blocked", function_name)));
            }
            
            // For restricted/sandboxed modes, check if function is explicitly allowed
            match security_level {
                TlispSecurityLevel::Trusted => Ok(()),
                TlispSecurityLevel::Restricted | TlispSecurityLevel::Sandboxed => {
                    if policy.allowed_functions.contains(&function_name.to_string()) {
                        Ok(())
                    } else {
                        Err(TlispError::SecurityError(format!("Function '{}' is not allowed", function_name)))
                    }
                }
            }
        } else {
            Err(TlispError::SecurityError("No security policy found".to_string()))
        }
    }
    
    /// Check resource limits
    pub fn check_resource_limits(&self, resource_type: &str, amount: usize, security_level: &TlispSecurityLevel) -> TlispResult<()> {
        if let Some(policy) = self.policies.get(security_level) {
            match resource_type {
                "heap" => {
                    if amount > policy.memory_limits.max_heap {
                        return Err(TlispError::SecurityError(format!("Heap allocation {} exceeds limit {}", amount, policy.memory_limits.max_heap)));
                    }
                }
                "stack" => {
                    if amount > policy.memory_limits.max_stack_depth {
                        return Err(TlispError::SecurityError(format!("Stack depth {} exceeds limit {}", amount, policy.memory_limits.max_stack_depth)));
                    }
                }
                "variables" => {
                    if amount > policy.memory_limits.max_variables {
                        return Err(TlispError::SecurityError(format!("Variable count {} exceeds limit {}", amount, policy.memory_limits.max_variables)));
                    }
                }
                "string_length" => {
                    if amount > policy.memory_limits.max_string_length {
                        return Err(TlispError::SecurityError(format!("String length {} exceeds limit {}", amount, policy.memory_limits.max_string_length)));
                    }
                }
                "list_length" => {
                    if amount > policy.memory_limits.max_list_length {
                        return Err(TlispError::SecurityError(format!("List length {} exceeds limit {}", amount, policy.memory_limits.max_list_length)));
                    }
                }
                _ => {
                    return Err(TlispError::SecurityError(format!("Unknown resource type: {}", resource_type)));
                }
            }
        }
        Ok(())
    }
    
    /// Get security statistics
    pub fn get_stats(&self) -> &TlispSecurityStats {
        &self.stats
    }
    
    /// Get audit log
    pub fn get_audit_log(&self) -> &[TlispAuditEvent] {
        &self.audit_log
    }
    
    /// Clear audit log
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }
    
    /// Set custom security policy
    pub fn set_policy(&mut self, level: TlispSecurityLevel, policy: TlispSecurityPolicy) {
        self.policies.insert(level, policy);
    }
    
    /// Check function permissions recursively in an expression
    fn check_function_permissions(&self, expr: &Expr<Type>, security_level: &TlispSecurityLevel) -> TlispResult<()> {
        match expr {
            Expr::Symbol(name, _) => {
                // Check if this is a function call
                self.check_function_permission(name, security_level)?;
            }
            Expr::Application(func, args, _) => {
                // Check function
                self.check_function_permissions(func, security_level)?;
                // Check arguments
                for arg in args {
                    self.check_function_permissions(arg, security_level)?;
                }
            }
            Expr::List(exprs, _) => {
                for expr in exprs {
                    self.check_function_permissions(expr, security_level)?;
                }
            }
            Expr::Lambda(_, body, _) => {
                self.check_function_permissions(body, security_level)?;
            }
            Expr::Let(bindings, body, _) => {
                for (_, expr) in bindings {
                    self.check_function_permissions(expr, security_level)?;
                }
                self.check_function_permissions(body, security_level)?;
            }
            Expr::If(cond, then_expr, else_expr, _) => {
                self.check_function_permissions(cond, security_level)?;
                self.check_function_permissions(then_expr, security_level)?;
                self.check_function_permissions(else_expr, security_level)?;
            }
            Expr::Quote(expr, _) => {
                self.check_function_permissions(expr, security_level)?;
            }
            Expr::Define(_, expr, _) => {
                self.check_function_permissions(expr, security_level)?;
            }
            Expr::Set(_, expr, _) => {
                self.check_function_permissions(expr, security_level)?;
            }
            Expr::Macro(_, _, body, _) => {
                self.check_function_permissions(body, security_level)?;
            }
            Expr::TypeAnnotation(expr, type_expr, _) => {
                self.check_function_permissions(expr, security_level)?;
                self.check_function_permissions(type_expr, security_level)?;
            }
            // Literals are always safe
            Expr::Number(_, _) | Expr::Float(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => {}
        }
        Ok(())
    }
    
    /// Set up default security policies
    fn setup_default_policies(&mut self) {
        // Trusted policy - allow everything
        let trusted_policy = TlispSecurityPolicy {
            allowed_functions: vec![], // Empty means allow all
            blocked_functions: vec![],
            max_recursion_depth: 10000,
            max_execution_time: Duration::from_secs(3600), // 1 hour
            memory_limits: TlispMemoryLimits {
                max_heap: 1024 * 1024 * 1024, // 1 GB
                max_stack_depth: 10000,
                max_variables: 1000000,
                max_string_length: 100 * 1024 * 1024, // 100 MB
                max_list_length: 10000000,
            },
            io_permissions: TlispIOPermissions {
                allow_file_read: true,
                allow_file_write: true,
                allowed_paths: vec!["*".to_string()], // All paths
                allow_console: true,
            },
            network_permissions: TlispNetworkPermissions {
                allow_outbound: true,
                allow_inbound: true,
                allowed_hosts: vec!["*".to_string()], // All hosts
                allowed_ports: vec![], // All ports
            },
        };
        
        // Restricted policy - moderate restrictions
        let restricted_policy = TlispSecurityPolicy::default();
        
        // Sandboxed policy - maximum restrictions
        let sandboxed_policy = TlispSecurityPolicy {
            allowed_functions: vec![
                "+".to_string(), "-".to_string(), "*".to_string(), "/".to_string(),
                "=".to_string(), "<".to_string(), ">".to_string(),
                "if".to_string(), "list".to_string(), "car".to_string(), "cdr".to_string(),
            ],
            blocked_functions: vec![], // Everything not explicitly allowed is blocked
            max_recursion_depth: 50,
            max_execution_time: Duration::from_secs(1),
            memory_limits: TlispMemoryLimits {
                max_heap: 1024 * 1024, // 1 MB
                max_stack_depth: 100,
                max_variables: 1000,
                max_string_length: 1024, // 1 KB
                max_list_length: 1000,
            },
            io_permissions: TlispIOPermissions {
                allow_file_read: false,
                allow_file_write: false,
                allowed_paths: vec![],
                allow_console: false,
            },
            network_permissions: TlispNetworkPermissions {
                allow_outbound: false,
                allow_inbound: false,
                allowed_hosts: vec![],
                allowed_ports: vec![],
            },
        };
        
        self.policies.insert(TlispSecurityLevel::Trusted, trusted_policy);
        self.policies.insert(TlispSecurityLevel::Restricted, restricted_policy);
        self.policies.insert(TlispSecurityLevel::Sandboxed, sandboxed_policy);
    }
    
    /// Log an audit event
    fn log_audit_event(&mut self, event: TlispAuditEvent) {
        self.audit_log.push(event);
        
        // Keep audit log size manageable
        if self.audit_log.len() > 10000 {
            self.audit_log.drain(0..1000); // Remove oldest 1000 entries
        }
    }
    
    /// Update average verification time
    fn update_avg_verification_time(&mut self) {
        if self.stats.programs_verified > 0 {
            self.stats.avg_verification_time = self.stats.total_execution_time / self.stats.programs_verified as u32;
        }
    }
}

impl Default for TlispSecurityStats {
    fn default() -> Self {
        TlispSecurityStats {
            programs_verified: 0,
            verification_failures: 0,
            security_violations: 0,
            permission_denials: 0,
            resource_limit_violations: 0,
            total_execution_time: Duration::ZERO,
            avg_verification_time: Duration::ZERO,
        }
    }
}
