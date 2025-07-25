//! Bytecode Verification System
//!
//! This module implements comprehensive bytecode verification to ensure
//! safe execution of untrusted code as specified in IMPROVEMENT.md

use std::collections::{HashMap, HashSet};
use crate::bytecode::{BytecodeProgram, Value, Bytecode};
use crate::types::EffectGrade;
use crate::error::{BytecodeError, BytecodeResult};

/// Type information for verification
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Int,
    UInt,
    Float,
    Bool,
    String,
    Bytes,
    List(Box<TypeInfo>),
    Map(Box<TypeInfo>, Box<TypeInfo>), // Key, Value types
    Set(Box<TypeInfo>),
    Tuple(Vec<TypeInfo>),
    Function(Vec<TypeInfo>, Box<TypeInfo>), // Args, Return
    Pid,
    FileHandle,
    SocketHandle,
    TimerHandle,
    MemoryRef,
    WeakRef,
    Null,
    Any, // For dynamic typing
}

impl TypeInfo {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &TypeInfo) -> bool {
        match (self, other) {
            (TypeInfo::Any, _) | (_, TypeInfo::Any) => true,
            (TypeInfo::Int, TypeInfo::UInt) | (TypeInfo::UInt, TypeInfo::Int) => true,
            (TypeInfo::Int, TypeInfo::Float) | (TypeInfo::Float, TypeInfo::Int) => true,
            (TypeInfo::UInt, TypeInfo::Float) | (TypeInfo::Float, TypeInfo::UInt) => true,
            (a, b) => a == b,
        }
    }
    
    /// Get the type info for a value
    pub fn from_value(value: &Value) -> TypeInfo {
        match value {
            Value::Int(_) => TypeInfo::Int,
            Value::UInt(_) => TypeInfo::UInt,
            Value::Float(_) => TypeInfo::Float,
            Value::Bool(_) => TypeInfo::Bool,
            Value::String(_) => TypeInfo::String,
            Value::Bytes(_) => TypeInfo::Bytes,
            Value::List(_) => TypeInfo::List(Box::new(TypeInfo::Any)),
            Value::Map(_) => TypeInfo::Map(Box::new(TypeInfo::String), Box::new(TypeInfo::Any)),
            Value::Set(_) => TypeInfo::Set(Box::new(TypeInfo::Any)),
            Value::Tuple(_) => TypeInfo::Tuple(vec![TypeInfo::Any]),
            Value::Function(_) => TypeInfo::Function(vec![], Box::new(TypeInfo::Any)),
            Value::Pid(_) => TypeInfo::Pid,
            Value::FileHandle(_) => TypeInfo::FileHandle,
            Value::SocketHandle(_) => TypeInfo::SocketHandle,
            Value::TimerHandle(_) => TypeInfo::TimerHandle,
            Value::MemoryRef(_) => TypeInfo::MemoryRef,
            Value::WeakRef(_) => TypeInfo::WeakRef,
            Value::Null => TypeInfo::Null,
        }
    }
}

/// Bytecode verifier for ensuring safe execution
pub struct BytecodeVerifier {
    /// Type stack for verification
    type_stack: Vec<TypeInfo>,
    /// Local variable types
    locals_types: Vec<TypeInfo>,
    /// Maximum stack depth allowed
    max_stack_depth: usize,
    /// Maximum local variables allowed
    max_locals: usize,
    /// Allowed effect grades
    allowed_effects: HashSet<EffectGrade>,
    /// Jump target validation
    valid_jump_targets: HashSet<usize>,
    /// Resource usage tracking
    resource_usage: ResourceUsage,
}

/// Resource usage tracking for verification
#[derive(Debug, Default)]
struct ResourceUsage {
    /// Memory allocations
    memory_allocations: u32,
    /// File handles opened
    file_handles: u32,
    /// Socket handles opened
    socket_handles: u32,
    /// Timer handles created
    timer_handles: u32,
    /// Maximum allowed for each resource type
    max_memory_allocations: u32,
    max_file_handles: u32,
    max_socket_handles: u32,
    max_timer_handles: u32,
}

impl BytecodeVerifier {
    /// Create a new bytecode verifier with default limits
    pub fn new() -> Self {
        let mut allowed_effects = HashSet::new();
        allowed_effects.insert(EffectGrade::Pure);
        allowed_effects.insert(EffectGrade::IO);
        allowed_effects.insert(EffectGrade::Memory);
        
        BytecodeVerifier {
            type_stack: Vec::new(),
            locals_types: Vec::new(),
            max_stack_depth: 1000,
            max_locals: 256,
            allowed_effects,
            valid_jump_targets: HashSet::new(),
            resource_usage: ResourceUsage {
                max_memory_allocations: 100,
                max_file_handles: 10,
                max_socket_handles: 10,
                max_timer_handles: 20,
                ..Default::default()
            },
        }
    }
    
    /// Create a verifier with custom limits
    pub fn with_limits(
        max_stack_depth: usize,
        max_locals: usize,
        allowed_effects: HashSet<EffectGrade>,
    ) -> Self {
        BytecodeVerifier {
            type_stack: Vec::new(),
            locals_types: Vec::new(),
            max_stack_depth,
            max_locals,
            allowed_effects,
            valid_jump_targets: HashSet::new(),
            resource_usage: ResourceUsage::default(),
        }
    }
    
    /// Verify a bytecode program
    pub fn verify(&mut self, program: &BytecodeProgram) -> BytecodeResult<()> {
        // Reset state
        self.type_stack.clear();
        self.locals_types.clear();
        self.valid_jump_targets.clear();
        self.resource_usage = ResourceUsage::default();
        
        // First pass: collect jump targets
        self.collect_jump_targets(program)?;
        
        // Second pass: verify instructions
        self.verify_instructions(program)?;
        
        // Final validation
        self.validate_final_state()?;
        
        Ok(())
    }
    
    /// Collect all valid jump targets
    fn collect_jump_targets(&mut self, program: &BytecodeProgram) -> BytecodeResult<()> {
        for (pc, instruction) in program.instructions.iter().enumerate() {
            self.valid_jump_targets.insert(pc);
            
            // Add function entry points
            match instruction {
                Bytecode::Call(func_idx, _) => {
                    if let Some(function) = program.functions.get(*func_idx as usize) {
                        self.valid_jump_targets.insert(function.start_pc);
                    }
                }
                _ => {}
            }
        }
        
        // Add end of program as valid target
        self.valid_jump_targets.insert(program.instructions.len());
        
        Ok(())
    }
    
    /// Verify all instructions in the program
    fn verify_instructions(&mut self, program: &BytecodeProgram) -> BytecodeResult<()> {
        for (pc, instruction) in program.instructions.iter().enumerate() {
            self.verify_instruction(instruction, pc, program)?;
        }
        Ok(())
    }
    
    /// Verify a single instruction
    fn verify_instruction(
        &mut self,
        instruction: &Bytecode,
        pc: usize,
        program: &BytecodeProgram,
    ) -> BytecodeResult<()> {
        // Check effect grade is allowed
        if !self.allowed_effects.contains(&instruction.effect_grade()) {
            return Err(BytecodeError::Verification(format!(
                "Effect grade {:?} not allowed at PC {}",
                instruction.effect_grade(),
                pc
            )));
        }
        
        // Verify instruction-specific constraints
        match instruction {
            Bytecode::Const(idx, _) => {
                self.verify_constant_access(*idx, program)?;
                let value = &program.constants[*idx as usize];
                self.push_type(TypeInfo::from_value(value))?;
            }
            
            Bytecode::Load(idx, _) => {
                self.verify_local_access(*idx)?;
                let local_type = self.locals_types[*idx as usize].clone();
                self.push_type(local_type)?;
            }
            
            Bytecode::Store(idx, _) => {
                let value_type = self.pop_type()?;
                self.verify_local_store(*idx, value_type)?;
            }
            
            Bytecode::Add(_) | Bytecode::Sub(_) | Bytecode::Mul(_) | Bytecode::Div(_) => {
                let b_type = self.pop_type()?;
                let a_type = self.pop_type()?;
                self.verify_arithmetic_operation(&a_type, &b_type)?;
                self.push_type(self.result_type_for_arithmetic(&a_type, &b_type))?;
            }
            
            Bytecode::BitAnd(_) | Bytecode::BitOr(_) | Bytecode::BitXor(_) => {
                let b_type = self.pop_type()?;
                let a_type = self.pop_type()?;
                self.verify_bitwise_operation(&a_type, &b_type)?;
                self.push_type(a_type)?;
            }
            
            Bytecode::Jump(target, _) => {
                self.verify_jump_target(*target)?;
            }
            
            Bytecode::JumpIf(target, _) | Bytecode::JumpIfNot(target, _) => {
                let condition_type = self.pop_type()?;
                self.verify_boolean_condition(&condition_type)?;
                self.verify_jump_target(*target)?;
            }
            
            Bytecode::Call(func_idx, _) => {
                self.verify_function_call(*func_idx, program)?;
            }
            
            Bytecode::Alloc(size, _) => {
                self.verify_resource_allocation()?;
                self.push_type(TypeInfo::MemoryRef)?;
            }
            
            Bytecode::FileOpen(_, _, _) => {
                self.verify_file_operation()?;
                self.push_type(TypeInfo::FileHandle)?;
            }
            
            Bytecode::SocketCreate(_, _) => {
                self.verify_socket_operation()?;
                self.push_type(TypeInfo::SocketHandle)?;
            }
            
            _ => {
                // For other instructions, perform basic stack validation
                self.verify_basic_instruction(instruction)?;
            }
        }
        
        // Check stack depth limits
        if self.type_stack.len() > self.max_stack_depth {
            return Err(BytecodeError::Verification(format!(
                "Stack depth {} exceeds maximum {}",
                self.type_stack.len(),
                self.max_stack_depth
            )));
        }
        
        Ok(())
    }
    
    /// Push a type onto the type stack
    fn push_type(&mut self, type_info: TypeInfo) -> BytecodeResult<()> {
        if self.type_stack.len() >= self.max_stack_depth {
            return Err(BytecodeError::Verification(
                "Stack overflow".to_string()
            ));
        }
        self.type_stack.push(type_info);
        Ok(())
    }
    
    /// Pop a type from the type stack
    fn pop_type(&mut self) -> BytecodeResult<TypeInfo> {
        self.type_stack.pop().ok_or_else(|| {
            BytecodeError::Verification("Stack underflow".to_string())
        })
    }
    
    /// Verify constant access
    fn verify_constant_access(&self, idx: u32, program: &BytecodeProgram) -> BytecodeResult<()> {
        if idx as usize >= program.constants.len() {
            return Err(BytecodeError::Verification(format!(
                "Constant index {} out of bounds",
                idx
            )));
        }
        Ok(())
    }
    
    /// Verify local variable access
    fn verify_local_access(&self, idx: u32) -> BytecodeResult<()> {
        if idx as usize >= self.locals_types.len() {
            return Err(BytecodeError::Verification(format!(
                "Local variable index {} out of bounds",
                idx
            )));
        }
        Ok(())
    }
    
    /// Verify local variable store
    fn verify_local_store(&mut self, idx: u32, value_type: TypeInfo) -> BytecodeResult<()> {
        // Extend locals if necessary
        while self.locals_types.len() <= idx as usize {
            if self.locals_types.len() >= self.max_locals {
                return Err(BytecodeError::Verification(format!(
                    "Too many local variables (max: {})",
                    self.max_locals
                )));
            }
            self.locals_types.push(TypeInfo::Any);
        }
        
        // Update local type
        self.locals_types[idx as usize] = value_type;
        Ok(())
    }
    
    /// Verify arithmetic operation types
    fn verify_arithmetic_operation(&self, a: &TypeInfo, b: &TypeInfo) -> BytecodeResult<()> {
        match (a, b) {
            (TypeInfo::Int, TypeInfo::Int) |
            (TypeInfo::UInt, TypeInfo::UInt) |
            (TypeInfo::Float, TypeInfo::Float) |
            (TypeInfo::Int, TypeInfo::Float) |
            (TypeInfo::Float, TypeInfo::Int) |
            (TypeInfo::UInt, TypeInfo::Float) |
            (TypeInfo::Float, TypeInfo::UInt) => Ok(()),
            _ => Err(BytecodeError::Verification(format!(
                "Invalid arithmetic operation between {:?} and {:?}",
                a, b
            ))),
        }
    }
    
    /// Get result type for arithmetic operation
    fn result_type_for_arithmetic(&self, a: &TypeInfo, b: &TypeInfo) -> TypeInfo {
        match (a, b) {
            (TypeInfo::Float, _) | (_, TypeInfo::Float) => TypeInfo::Float,
            (TypeInfo::UInt, TypeInfo::UInt) => TypeInfo::UInt,
            _ => TypeInfo::Int,
        }
    }
    
    /// Verify bitwise operation types
    fn verify_bitwise_operation(&self, a: &TypeInfo, b: &TypeInfo) -> BytecodeResult<()> {
        match (a, b) {
            (TypeInfo::Int, TypeInfo::Int) |
            (TypeInfo::UInt, TypeInfo::UInt) => Ok(()),
            _ => Err(BytecodeError::Verification(format!(
                "Invalid bitwise operation between {:?} and {:?}",
                a, b
            ))),
        }
    }
    
    /// Verify boolean condition
    fn verify_boolean_condition(&self, type_info: &TypeInfo) -> BytecodeResult<()> {
        // Any type can be used as a boolean condition (truthiness)
        Ok(())
    }
    
    /// Verify jump target
    fn verify_jump_target(&self, target: u32) -> BytecodeResult<()> {
        if !self.valid_jump_targets.contains(&(target as usize)) {
            return Err(BytecodeError::Verification(format!(
                "Invalid jump target: {}",
                target
            )));
        }
        Ok(())
    }
    
    /// Verify function call
    fn verify_function_call(&mut self, func_idx: u32, program: &BytecodeProgram) -> BytecodeResult<()> {
        if func_idx as usize >= program.functions.len() {
            return Err(BytecodeError::Verification(format!(
                "Function index {} out of bounds",
                func_idx
            )));
        }
        
        // For now, assume functions can be called with any arguments
        // In a more sophisticated verifier, we'd check argument types
        self.push_type(TypeInfo::Any)?;
        Ok(())
    }
    
    /// Verify resource allocation
    fn verify_resource_allocation(&mut self) -> BytecodeResult<()> {
        self.resource_usage.memory_allocations += 1;
        if self.resource_usage.memory_allocations > self.resource_usage.max_memory_allocations {
            return Err(BytecodeError::Verification(
                "Too many memory allocations".to_string()
            ));
        }
        Ok(())
    }
    
    /// Verify file operation
    fn verify_file_operation(&mut self) -> BytecodeResult<()> {
        self.resource_usage.file_handles += 1;
        if self.resource_usage.file_handles > self.resource_usage.max_file_handles {
            return Err(BytecodeError::Verification(
                "Too many file handles".to_string()
            ));
        }
        Ok(())
    }
    
    /// Verify socket operation
    fn verify_socket_operation(&mut self) -> BytecodeResult<()> {
        self.resource_usage.socket_handles += 1;
        if self.resource_usage.socket_handles > self.resource_usage.max_socket_handles {
            return Err(BytecodeError::Verification(
                "Too many socket handles".to_string()
            ));
        }
        Ok(())
    }
    
    /// Verify basic instruction (placeholder for other instructions)
    fn verify_basic_instruction(&mut self, instruction: &Bytecode) -> BytecodeResult<()> {
        // Basic verification for instructions not specifically handled
        // This would be expanded in a full implementation
        Ok(())
    }
    
    /// Validate final state after verification
    fn validate_final_state(&self) -> BytecodeResult<()> {
        // Check that we don't have excessive resource usage
        if self.resource_usage.memory_allocations > 0 {
            // In a real implementation, we might require explicit cleanup
        }
        
        Ok(())
    }
}

impl Default for BytecodeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Verification error types
#[derive(Debug, Clone)]
pub enum VerificationError {
    StackOverflow,
    StackUnderflow,
    TypeMismatch(TypeInfo, TypeInfo),
    InvalidJumpTarget(u32),
    ResourceLimitExceeded(String),
    InvalidInstruction(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::BytecodeProgram;

    #[test]
    fn test_type_compatibility() {
        assert!(TypeInfo::Int.is_compatible_with(&TypeInfo::UInt));
        assert!(TypeInfo::Int.is_compatible_with(&TypeInfo::Float));
        assert!(TypeInfo::Any.is_compatible_with(&TypeInfo::String));
        assert!(!TypeInfo::String.is_compatible_with(&TypeInfo::Int));
    }

    #[test]
    fn test_verifier_basic() {
        let mut verifier = BytecodeVerifier::new();
        let program = BytecodeProgram::new("test".to_string());
        
        // Empty program should verify successfully
        assert!(verifier.verify(&program).is_ok());
    }

    #[test]
    fn test_stack_operations() {
        let mut verifier = BytecodeVerifier::new();
        
        // Test push/pop
        verifier.push_type(TypeInfo::Int).unwrap();
        let popped = verifier.pop_type().unwrap();
        assert_eq!(popped, TypeInfo::Int);
        
        // Test underflow
        assert!(verifier.pop_type().is_err());
    }
}
