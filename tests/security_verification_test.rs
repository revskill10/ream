//! Comprehensive tests for security and verification system
//!
//! Tests the implementation of bytecode verification, security manager,
//! and bounds checking as specified in IMPROVEMENT.md

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use ream::bytecode::{
    BytecodeVerifier, SecurityManager, Permission, SecurityPolicy, ResourceLimits,
    create_sandbox_manager, BytecodeProgram, Bytecode, Value, VerifierTypeInfo
};
use ream::types::EffectGrade;
use ream::error::BytecodeError;

#[test]
fn test_bytecode_verifier_basic() {
    let mut verifier = BytecodeVerifier::new();
    let program = BytecodeProgram::new("test".to_string());
    
    // Empty program should verify successfully
    assert!(verifier.verify(&program).is_ok());
}

#[test]
fn test_bytecode_verifier_type_checking() {
    let mut verifier = BytecodeVerifier::new();
    let mut program = BytecodeProgram::new("test".to_string());
    
    // Add some constants
    let const_idx = program.add_constant(Value::Int(42));
    
    // Add instructions
    program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
    program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
    program.add_instruction(Bytecode::Add(EffectGrade::Pure));
    
    // Should verify successfully
    assert!(verifier.verify(&program).is_ok());
}

#[test]
fn test_bytecode_verifier_invalid_constant() {
    let mut verifier = BytecodeVerifier::new();
    let mut program = BytecodeProgram::new("test".to_string());
    
    // Reference non-existent constant
    program.add_instruction(Bytecode::Const(999, EffectGrade::Pure));
    
    // Should fail verification
    assert!(verifier.verify(&program).is_err());
}

#[test]
fn test_bytecode_verifier_stack_operations() {
    let mut verifier = BytecodeVerifier::new();
    let mut program = BytecodeProgram::new("test".to_string());
    
    // Add constants
    let const1 = program.add_constant(Value::Int(10));
    let const2 = program.add_constant(Value::Int(20));
    
    // Valid stack operations
    program.add_instruction(Bytecode::Const(const1, EffectGrade::Pure));
    program.add_instruction(Bytecode::Const(const2, EffectGrade::Pure));
    program.add_instruction(Bytecode::Add(EffectGrade::Pure));
    program.add_instruction(Bytecode::Pop(EffectGrade::Pure));
    
    assert!(verifier.verify(&program).is_ok());
}

#[test]
fn test_bytecode_verifier_stack_underflow() {
    let mut verifier = BytecodeVerifier::new();
    let mut program = BytecodeProgram::new("test".to_string());
    
    // Try to add without enough operands
    program.add_instruction(Bytecode::Add(EffectGrade::Pure));
    
    // Should fail due to stack underflow
    assert!(verifier.verify(&program).is_err());
}

#[test]
fn test_type_info_compatibility() {
    assert!(VerifierTypeInfo::Int.is_compatible_with(&VerifierTypeInfo::UInt));
    assert!(VerifierTypeInfo::Int.is_compatible_with(&VerifierTypeInfo::Float));
    assert!(VerifierTypeInfo::Any.is_compatible_with(&VerifierTypeInfo::String));
    assert!(!VerifierTypeInfo::String.is_compatible_with(&VerifierTypeInfo::Int));
}

#[test]
fn test_type_info_from_value() {
    let int_value = Value::Int(42);
    let type_info = VerifierTypeInfo::from_value(&int_value);
    assert_eq!(type_info, VerifierTypeInfo::Int);
    
    let string_value = Value::String("hello".to_string());
    let type_info = VerifierTypeInfo::from_value(&string_value);
    assert_eq!(type_info, VerifierTypeInfo::String);
}

#[test]
fn test_security_manager_basic() {
    let mut manager = SecurityManager::new();
    
    let permission = Permission::FileRead(PathBuf::from("/tmp/test.txt"));
    
    // Should not have permission initially
    assert!(!manager.check_permission(&permission));
    
    // Grant permission
    manager.grant_permission(permission.clone());
    assert!(manager.check_permission(&permission));
    
    // Revoke permission
    manager.revoke_permission(&permission);
    assert!(!manager.check_permission(&permission));
}

#[test]
fn test_security_manager_wildcard_permissions() {
    let mut manager = SecurityManager::new();
    
    // Grant wildcard file permission
    manager.grant_permission(Permission::FileAll);
    
    // Should have access to any file operation
    assert!(manager.check_permission(&Permission::FileRead(PathBuf::from("/tmp/test.txt"))));
    assert!(manager.check_permission(&Permission::FileWrite(PathBuf::from("/home/user/doc.txt"))));
    assert!(manager.check_permission(&Permission::FileExecute(PathBuf::from("/bin/ls"))));
    
    // Should not have network permissions
    assert!(!manager.check_permission(&Permission::NetworkConnect("127.0.0.1:8080".parse().unwrap())));
}

#[test]
fn test_security_manager_permission_enforcement() {
    let mut manager = SecurityManager::new();
    
    let permission = Permission::FileRead(PathBuf::from("/etc/passwd"));
    
    // Should deny by default in default policy
    assert!(manager.enforce_permission(permission.clone()).is_ok()); // Default policy allows
    
    // Create restrictive policy
    let policy = SecurityPolicy {
        default_deny: true,
        sandbox_mode: true,
        allowed_effects: HashSet::new(),
        blocked_instructions: HashSet::new(),
        max_execution_time: Some(Duration::from_secs(5)),
        audit_logging: false,
    };
    
    manager.update_policy(policy);
    
    // Should deny now
    assert!(manager.enforce_permission(permission.clone()).is_err());
    
    // Grant permission
    manager.grant_permission(permission.clone());
    assert!(manager.enforce_permission(permission).is_ok());
}

#[test]
fn test_security_manager_resource_limits() {
    let mut manager = SecurityManager::new();
    
    // Should succeed within limits
    assert!(manager.check_resource_allocation("memory", 1024).is_ok());
    assert!(manager.check_resource_allocation("file_handle", 1).is_ok());
    
    // Should fail when exceeding limits
    assert!(manager.check_resource_allocation("memory", usize::MAX).is_err());
    
    // Check that usage is tracked
    let usage = manager.get_usage();
    assert_eq!(usage.memory_used, 1024);
    assert_eq!(usage.file_handles, 1);
}

#[test]
fn test_security_manager_instruction_blocking() {
    let mut policy = SecurityPolicy::default();
    policy.blocked_instructions.insert("file_open".to_string());
    policy.blocked_instructions.insert("socket_create".to_string());
    
    let manager = SecurityManager::with_policy(policy, ResourceLimits::default());
    
    assert!(manager.is_instruction_blocked("file_open"));
    assert!(manager.is_instruction_blocked("socket_create"));
    assert!(!manager.is_instruction_blocked("add"));
}

#[test]
fn test_security_manager_execution_time_limit() {
    let mut manager = SecurityManager::new();
    
    // Should pass immediately
    assert!(manager.check_execution_time().is_ok());
    
    // Simulate time passing (in real test, we'd need to actually wait)
    // For now, just test the mechanism exists
    assert!(manager.check_instruction_count().is_ok());
}

#[test]
fn test_security_manager_stack_depth_limit() {
    let mut manager = SecurityManager::new();
    
    // Should pass for reasonable depth
    assert!(manager.check_stack_depth(10).is_ok());
    assert!(manager.check_stack_depth(100).is_ok());
    
    // Should fail for excessive depth
    assert!(manager.check_stack_depth(10000).is_err());
}

#[test]
fn test_sandbox_manager() {
    let manager = create_sandbox_manager();
    
    // Should be in sandbox mode
    assert!(manager.get_policy().sandbox_mode);
    assert!(manager.get_policy().default_deny);
    
    // Should have restrictive limits
    assert_eq!(manager.get_usage().memory_used, 0);
    
    // Should block dangerous instructions
    assert!(manager.is_instruction_blocked("file_open"));
    assert!(manager.is_instruction_blocked("socket_create"));
    assert!(manager.is_instruction_blocked("spawn_process"));
    
    // Should allow safe instructions
    assert!(!manager.is_instruction_blocked("add"));
    assert!(!manager.is_instruction_blocked("const"));
}

#[test]
fn test_security_manager_reset_usage() {
    let mut manager = SecurityManager::new();
    
    // Use some resources
    manager.check_resource_allocation("memory", 1024).unwrap();
    manager.check_instruction_count().unwrap();
    
    // Check usage is tracked
    assert_eq!(manager.get_usage().memory_used, 1024);
    assert_eq!(manager.get_usage().instructions_executed, 1);
    
    // Reset usage
    manager.reset_usage();
    
    // Usage should be reset
    assert_eq!(manager.get_usage().memory_used, 0);
    assert_eq!(manager.get_usage().instructions_executed, 0);
}

#[test]
fn test_integrated_verification_and_security() {
    let mut verifier = BytecodeVerifier::new();
    let mut security_manager = create_sandbox_manager();
    
    // Create a simple program
    let mut program = BytecodeProgram::new("test".to_string());
    let const_idx = program.add_constant(Value::Int(42));
    program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
    
    // Should verify successfully
    assert!(verifier.verify(&program).is_ok());
    
    // Should pass security checks for pure operations
    assert!(security_manager.enforce_permission(Permission::SystemProperty("test".to_string())).is_err());
    
    // But should allow basic computation
    assert!(security_manager.check_instruction_count().is_ok());
}

#[test]
fn test_verifier_with_custom_limits() {
    let mut allowed_effects = HashSet::new();
    allowed_effects.insert(EffectGrade::Pure);
    
    let mut verifier = BytecodeVerifier::with_limits(10, 5, allowed_effects);
    
    let mut program = BytecodeProgram::new("test".to_string());
    
    // Add instructions that would exceed stack limit
    for i in 0..15 {
        let const_idx = program.add_constant(Value::Int(i));
        program.add_instruction(Bytecode::Const(const_idx, EffectGrade::Pure));
    }
    
    // Should fail due to stack depth limit
    assert!(verifier.verify(&program).is_err());
}

#[test]
fn test_security_policy_serialization() {
    let policy = SecurityPolicy::default();
    
    // Should be able to serialize and deserialize
    let serialized = serde_json::to_string(&policy).unwrap();
    let deserialized: SecurityPolicy = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(policy.default_deny, deserialized.default_deny);
    assert_eq!(policy.sandbox_mode, deserialized.sandbox_mode);
}

#[test]
fn test_resource_limits_defaults() {
    let limits = ResourceLimits::default();
    
    assert_eq!(limits.max_memory, 100 * 1024 * 1024); // 100 MB
    assert_eq!(limits.max_file_handles, 10);
    assert_eq!(limits.max_socket_handles, 5);
    assert_eq!(limits.max_timer_handles, 20);
    assert_eq!(limits.max_execution_time, Duration::from_secs(30));
    assert_eq!(limits.max_stack_depth, 1000);
    assert_eq!(limits.max_instructions, 1_000_000);
}
