//! REAM Bytecode - Polymorphic bytecode format with effect tracking
//! 
//! Bytecode forms the initial algebra over a graded monad of instruction effects

pub mod instruction;
pub mod program;
pub mod compiler;
pub mod optimizer;
pub mod registry;
pub mod verifier;
pub mod security;

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::types::{EffectGrade, Pid};
use crate::error::{BytecodeError, BytecodeResult};

pub use instruction::{Bytecode, Instruction};
pub use program::{BytecodeProgram, BytecodeFunction};
pub use compiler::{BytecodeCompiler, LanguageCompiler};
pub use optimizer::{Optimization, ConstantFolding, DeadCodeElimination};
pub use registry::BytecodeRegistry;
pub use verifier::{BytecodeVerifier, TypeInfo as VerifierTypeInfo, VerificationError};
pub use security::{SecurityManager, Permission, SecurityPolicy, ResourceLimits, SecurityEvent, SecurityEventType, create_sandbox_manager};

/// Value types in REAM bytecode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    /// Integer value
    Int(i64),
    /// Unsigned integer value
    UInt(u64),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
    /// Byte array
    Bytes(Vec<u8>),
    /// List of values
    List(Vec<Value>),
    /// Map/Dictionary
    Map(std::collections::HashMap<String, Value>),
    /// Set of values
    Set(std::collections::HashSet<Value>),
    /// Tuple of values
    Tuple(Vec<Value>),
    /// Function reference
    Function(u32),
    /// Process ID
    Pid(Pid),
    /// File handle
    FileHandle(u32),
    /// Socket handle
    SocketHandle(u32),
    /// Timer handle
    TimerHandle(u32),
    /// Memory reference
    MemoryRef(u32),
    /// Weak reference
    WeakRef(u32),
    /// Null value
    Null,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::UInt(a), Value::UInt(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => {
                // Handle NaN case: NaN != NaN
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            },
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Pid(a), Value::Pid(b)) => a == b,
            (Value::FileHandle(a), Value::FileHandle(b)) => a == b,
            (Value::SocketHandle(a), Value::SocketHandle(b)) => a == b,
            (Value::TimerHandle(a), Value::TimerHandle(b)) => a == b,
            (Value::MemoryRef(a), Value::MemoryRef(b)) => a == b,
            (Value::WeakRef(a), Value::WeakRef(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Int(v) => {
                0u8.hash(state);
                v.hash(state);
            },
            Value::UInt(v) => {
                1u8.hash(state);
                v.hash(state);
            },
            Value::Float(v) => {
                2u8.hash(state);
                // For floats, we need to handle NaN and use a consistent representation
                if v.is_nan() {
                    // All NaNs hash to the same value
                    f64::NAN.to_bits().hash(state);
                } else {
                    v.to_bits().hash(state);
                }
            },
            Value::Bool(v) => {
                3u8.hash(state);
                v.hash(state);
            },
            Value::String(v) => {
                4u8.hash(state);
                v.hash(state);
            },
            Value::Bytes(v) => {
                5u8.hash(state);
                v.hash(state);
            },
            Value::List(v) => {
                6u8.hash(state);
                v.hash(state);
            },
            Value::Map(v) => {
                7u8.hash(state);
                // HashMap doesn't implement Hash, so we need to hash the sorted entries
                let mut entries: Vec<_> = v.iter().collect();
                entries.sort_by_key(|(k, _)| *k);
                entries.hash(state);
            },
            Value::Set(v) => {
                8u8.hash(state);
                // HashSet doesn't implement Hash, so we hash the count and each element
                v.len().hash(state);
                // We can't sort Values easily, so we'll just hash each element
                // This is not ideal but works for our use case
                for element in v.iter() {
                    element.hash(state);
                }
            },
            Value::Tuple(v) => {
                9u8.hash(state);
                v.hash(state);
            },
            Value::Function(v) => {
                10u8.hash(state);
                v.hash(state);
            },
            Value::Pid(v) => {
                11u8.hash(state);
                v.hash(state);
            },
            Value::FileHandle(v) => {
                12u8.hash(state);
                v.hash(state);
            },
            Value::SocketHandle(v) => {
                13u8.hash(state);
                v.hash(state);
            },
            Value::TimerHandle(v) => {
                14u8.hash(state);
                v.hash(state);
            },
            Value::MemoryRef(v) => {
                15u8.hash(state);
                v.hash(state);
            },
            Value::WeakRef(v) => {
                16u8.hash(state);
                v.hash(state);
            },
            Value::Null => {
                17u8.hash(state);
            },
        }
    }
}

impl Value {
    /// Get the type of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::UInt(_) => "uint",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Set(_) => "set",
            Value::Tuple(_) => "tuple",
            Value::Function(_) => "function",
            Value::Pid(_) => "pid",
            Value::FileHandle(_) => "file",
            Value::SocketHandle(_) => "socket",
            Value::TimerHandle(_) => "timer",
            Value::MemoryRef(_) => "memory",
            Value::WeakRef(_) => "weakref",
            Value::Null => "null",
        }
    }
    
    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::UInt(u) => *u != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Bytes(b) => !b.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Set(s) => !s.is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Null => false,
            _ => true,
        }
    }
    
    /// Convert to integer if possible
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::UInt(u) => Some(*u as i64),
            Value::Float(f) => Some(*f as i64),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Convert to unsigned integer if possible
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Value::UInt(u) => Some(*u),
            Value::Int(i) => if *i >= 0 { Some(*i as u64) } else { None },
            Value::Float(f) => if *f >= 0.0 { Some(*f as u64) } else { None },
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Convert to float if possible
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::UInt(u) => Some(*u as f64),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Int(i) => i.to_string(),
            Value::UInt(u) => u.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => format!("{:?}", self),
        }
    }

}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

/// Type information for bytecode values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeInfo {
    /// Integer type
    Int,
    /// Float type
    Float,
    /// Boolean type
    Bool,
    /// String type
    String,
    /// List type with element type
    List(Box<TypeInfo>),
    /// Function type with parameter and return types
    Function(Vec<TypeInfo>, Box<TypeInfo>),
    /// Process ID type
    Pid,
    /// Unit type
    Unit,
    /// Type variable
    TypeVar(String),
    /// Unknown type
    Unknown,
}

impl TypeInfo {
    /// Check if this type is compatible with another
    pub fn is_compatible(&self, other: &TypeInfo) -> bool {
        match (self, other) {
            (TypeInfo::TypeVar(_), _) | (_, TypeInfo::TypeVar(_)) => true,
            (TypeInfo::Unknown, _) | (_, TypeInfo::Unknown) => true,
            (TypeInfo::Int, TypeInfo::Int) => true,
            (TypeInfo::Float, TypeInfo::Float) => true,
            (TypeInfo::Bool, TypeInfo::Bool) => true,
            (TypeInfo::String, TypeInfo::String) => true,
            (TypeInfo::Pid, TypeInfo::Pid) => true,
            (TypeInfo::Unit, TypeInfo::Unit) => true,
            (TypeInfo::List(a), TypeInfo::List(b)) => a.is_compatible(b),
            (TypeInfo::Function(a_params, a_ret), TypeInfo::Function(b_params, b_ret)) => {
                a_params.len() == b_params.len() &&
                a_params.iter().zip(b_params.iter()).all(|(a, b)| a.is_compatible(b)) &&
                a_ret.is_compatible(b_ret)
            }
            _ => false,
        }
    }
    
    /// Get the default value for this type
    pub fn default_value(&self) -> Value {
        match self {
            TypeInfo::Int => Value::Int(0),
            TypeInfo::Float => Value::Float(0.0),
            TypeInfo::Bool => Value::Bool(false),
            TypeInfo::String => Value::String(String::new()),
            TypeInfo::List(_) => Value::List(Vec::new()),
            TypeInfo::Pid => Value::Pid(Pid::new()),
            TypeInfo::Unit | TypeInfo::TypeVar(_) | TypeInfo::Unknown => Value::Null,
            TypeInfo::Function(_, _) => Value::Function(0),
        }
    }
}

/// Bytecode execution context
#[derive(Debug)]
pub struct ExecutionContext {
    /// Value stack
    pub stack: Vec<Value>,
    /// Local variables
    pub locals: Vec<Value>,
    /// Program counter
    pub pc: usize,
    /// Current effect grade
    pub effect_grade: EffectGrade,
    /// Call stack
    pub call_stack: Vec<CallFrame>,
}

/// Call frame for function calls
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Return address
    pub return_pc: usize,
    /// Local variable base
    pub local_base: usize,
    /// Function ID
    pub function_id: u32,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        ExecutionContext {
            stack: Vec::new(),
            locals: Vec::new(),
            pc: 0,
            effect_grade: EffectGrade::Pure,
            call_stack: Vec::new(),
        }
    }
    
    /// Push a value onto the stack
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
    
    /// Pop a value from the stack
    pub fn pop(&mut self) -> BytecodeResult<Value> {
        self.stack.pop().ok_or(BytecodeError::InvalidOperand("Stack underflow".to_string()))
    }
    
    /// Peek at the top of the stack
    pub fn peek(&self) -> BytecodeResult<&Value> {
        self.stack.last().ok_or(BytecodeError::InvalidOperand("Stack empty".to_string()))
    }
    
    /// Get a local variable
    pub fn get_local(&self, index: usize) -> BytecodeResult<&Value> {
        self.locals.get(index).ok_or(BytecodeError::InvalidOperand(
            format!("Local variable {} not found", index)
        ))
    }
    
    /// Set a local variable
    pub fn set_local(&mut self, index: usize, value: Value) -> BytecodeResult<()> {
        if index >= self.locals.len() {
            self.locals.resize(index + 1, Value::Null);
        }
        self.locals[index] = value;
        Ok(())
    }
    
    /// Update effect grade
    pub fn update_effect(&mut self, effect: EffectGrade) {
        self.effect_grade = self.effect_grade.combine(effect);
    }
    
    /// Push a call frame
    pub fn push_call(&mut self, function_id: u32, return_pc: usize) {
        let frame = CallFrame {
            return_pc,
            local_base: self.locals.len(),
            function_id,
        };
        self.call_stack.push(frame);
    }
    
    /// Pop a call frame
    pub fn pop_call(&mut self) -> BytecodeResult<CallFrame> {
        self.call_stack.pop().ok_or(BytecodeError::InvalidOperand("Call stack underflow".to_string()))
    }
    
    /// Reset the context
    pub fn reset(&mut self) {
        self.stack.clear();
        self.locals.clear();
        self.pc = 0;
        self.effect_grade = EffectGrade::Pure;
        self.call_stack.clear();
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Bytecode virtual machine
pub struct BytecodeVM {
    /// Execution context
    context: ExecutionContext,
    /// Loaded programs
    programs: HashMap<String, BytecodeProgram>,
    /// Runtime statistics
    stats: VMStats,
}

#[derive(Debug, Default)]
struct VMStats {
    instructions_executed: u64,
    function_calls: u64,
    stack_operations: u64,
}

impl BytecodeVM {
    /// Create a new bytecode VM
    pub fn new() -> Self {
        BytecodeVM {
            context: ExecutionContext::new(),
            programs: HashMap::new(),
            stats: VMStats::default(),
        }
    }
    
    /// Load a bytecode program
    pub fn load_program(&mut self, name: String, program: BytecodeProgram) {
        self.programs.insert(name, program);
    }
    
    /// Execute a program
    pub fn execute(&mut self, program_name: &str) -> BytecodeResult<Value> {
        let program = self.programs.get(program_name)
            .ok_or_else(|| BytecodeError::CompilationFailed(format!("Program {} not found", program_name)))?
            .clone();
        
        self.execute_program(&program)
    }
    
    /// Execute a bytecode program
    pub fn execute_program(&mut self, program: &BytecodeProgram) -> BytecodeResult<Value> {
        self.context.reset();
        
        while self.context.pc < program.instructions.len() {
            let instruction = &program.instructions[self.context.pc];
            self.execute_instruction(instruction, program)?;
            self.stats.instructions_executed += 1;
        }
        
        // Return top of stack or null
        if self.context.stack.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(self.context.pop()?)
        }
    }
    
    /// Execute a single instruction
    fn execute_instruction(&mut self, instruction: &Bytecode, program: &BytecodeProgram) -> BytecodeResult<()> {
        use crate::bytecode::instruction::Bytecode::*;
        
        match instruction {
            Const(idx, effect) => {
                self.context.update_effect(*effect);
                let value = program.constants.get(*idx as usize)
                    .ok_or_else(|| BytecodeError::InvalidOperand(format!("Constant {} not found", idx)))?
                    .clone();
                self.context.push(value);
                self.context.pc += 1;
            }
            Load(idx, effect) => {
                self.context.update_effect(*effect);
                let value = self.context.get_local(*idx as usize)?.clone();
                self.context.push(value);
                self.context.pc += 1;
            }
            Store(idx, effect) => {
                self.context.update_effect(*effect);
                let value = self.context.pop()?;
                self.context.set_local(*idx as usize, value)?;
                self.context.pc += 1;
            }
            Add(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.add_values(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            Sub(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.sub_values(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            Mul(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.mul_values(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            Jump(target, effect) => {
                self.context.update_effect(*effect);
                self.context.pc = *target as usize;
            }
            JumpIf(target, effect) => {
                self.context.update_effect(*effect);
                let condition = self.context.pop()?;
                if condition.is_truthy() {
                    self.context.pc = *target as usize;
                } else {
                    self.context.pc += 1;
                }
            }
            Call(func_idx, effect) => {
                self.context.update_effect(*effect);
                self.context.push_call(*func_idx, self.context.pc + 1);
                
                // Jump to function
                if let Some(function) = program.functions.get(*func_idx as usize) {
                    self.context.pc = function.start_pc;
                } else {
                    return Err(BytecodeError::InvalidOperand(format!("Function {} not found", func_idx)));
                }
                
                self.stats.function_calls += 1;
            }
            Ret(effect) => {
                self.context.update_effect(*effect);
                let frame = self.context.pop_call()?;
                self.context.pc = frame.return_pc;

                // Restore locals
                self.context.locals.truncate(frame.local_base);
            }
            LoadGlobal(idx, effect) => {
                self.context.update_effect(*effect);
                // For now, treat globals as constants (simplified implementation)
                let value = program.constants.get(*idx as usize)
                    .ok_or_else(|| BytecodeError::InvalidOperand(format!("Global {} not found", idx)))?
                    .clone();
                self.context.push(value);
                self.context.pc += 1;
            }
            StoreGlobal(_idx, effect) => {
                self.context.update_effect(*effect);
                // For now, just pop the value (simplified implementation)
                let _value = self.context.pop()?;
                self.context.pc += 1;
            }
            Print(effect) => {
                self.context.update_effect(*effect);
                let value = self.context.pop()?;
                println!("{}", value);
                self.context.pc += 1;
            }
            Nop(effect) => {
                self.context.update_effect(*effect);
                self.context.pc += 1;
            }
            JumpIfNot(target, effect) => {
                self.context.update_effect(*effect);
                let condition = self.context.pop()?;
                if !condition.is_truthy() {
                    self.context.pc = *target as usize;
                } else {
                    self.context.pc += 1;
                }
            }
            // Bitwise operations
            BitAnd(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.bitwise_and(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            BitOr(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.bitwise_or(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            BitXor(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.bitwise_xor(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            BitNot(effect) => {
                self.context.update_effect(*effect);
                let a = self.context.pop()?;
                let result = self.bitwise_not(a)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            ShiftLeft(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.shift_left(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            ShiftRight(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.shift_right(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            UnsignedShiftRight(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.unsigned_shift_right(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            // Enhanced arithmetic
            DivRem(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let (div, rem) = self.div_rem_values(a, b)?;
                self.context.push(rem);
                self.context.push(div);
                self.context.pc += 1;
            }
            Abs(effect) => {
                self.context.update_effect(*effect);
                let a = self.context.pop()?;
                let result = self.abs_value(a)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            Neg(effect) => {
                self.context.update_effect(*effect);
                let a = self.context.pop()?;
                let result = self.neg_value(a)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            Min(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.min_values(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            Max(effect) => {
                self.context.update_effect(*effect);
                let b = self.context.pop()?;
                let a = self.context.pop()?;
                let result = self.max_values(a, b)?;
                self.context.push(result);
                self.context.pc += 1;
            }
            _ => {
                return Err(BytecodeError::InvalidInstruction(format!("Unimplemented instruction: {:?}", instruction)));
            }
        }
        
        Ok(())
    }
    
    // Arithmetic operations
    
    fn add_values(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
            _ => Err(BytecodeError::InvalidOperand("Cannot add these types".to_string())),
        }
    }
    
    fn sub_values(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
            _ => Err(BytecodeError::InvalidOperand("Cannot subtract these types".to_string())),
        }
    }
    
    fn mul_values(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
            _ => Err(BytecodeError::InvalidOperand("Cannot multiply these types".to_string())),
        }
    }

    // Bitwise operations

    fn bitwise_and(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
            (Value::UInt(a), Value::UInt(b)) => Ok(Value::UInt(a & b)),
            (Value::Int(a), Value::UInt(b)) => Ok(Value::Int(a & (b as i64))),
            (Value::UInt(a), Value::Int(b)) => Ok(Value::Int((a as i64) & b)),
            _ => Err(BytecodeError::InvalidOperand("Cannot perform bitwise AND on these types".to_string())),
        }
    }

    fn bitwise_or(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
            (Value::UInt(a), Value::UInt(b)) => Ok(Value::UInt(a | b)),
            (Value::Int(a), Value::UInt(b)) => Ok(Value::Int(a | (b as i64))),
            (Value::UInt(a), Value::Int(b)) => Ok(Value::Int((a as i64) | b)),
            _ => Err(BytecodeError::InvalidOperand("Cannot perform bitwise OR on these types".to_string())),
        }
    }

    fn bitwise_xor(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
            (Value::UInt(a), Value::UInt(b)) => Ok(Value::UInt(a ^ b)),
            (Value::Int(a), Value::UInt(b)) => Ok(Value::Int(a ^ (b as i64))),
            (Value::UInt(a), Value::Int(b)) => Ok(Value::Int((a as i64) ^ b)),
            _ => Err(BytecodeError::InvalidOperand("Cannot perform bitwise XOR on these types".to_string())),
        }
    }

    fn bitwise_not(&self, a: Value) -> BytecodeResult<Value> {
        match a {
            Value::Int(a) => Ok(Value::Int(!a)),
            Value::UInt(a) => Ok(Value::UInt(!a)),
            _ => Err(BytecodeError::InvalidOperand("Cannot perform bitwise NOT on this type".to_string())),
        }
    }

    fn shift_left(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                if b >= 0 && b < 64 {
                    Ok(Value::Int(a << b))
                } else {
                    Err(BytecodeError::InvalidOperand("Shift amount out of range".to_string()))
                }
            }
            (Value::UInt(a), Value::Int(b)) => {
                if b >= 0 && b < 64 {
                    Ok(Value::UInt(a << b))
                } else {
                    Err(BytecodeError::InvalidOperand("Shift amount out of range".to_string()))
                }
            }
            _ => Err(BytecodeError::InvalidOperand("Cannot shift these types".to_string())),
        }
    }

    fn shift_right(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                if b >= 0 && b < 64 {
                    Ok(Value::Int(a >> b))
                } else {
                    Err(BytecodeError::InvalidOperand("Shift amount out of range".to_string()))
                }
            }
            (Value::UInt(a), Value::Int(b)) => {
                if b >= 0 && b < 64 {
                    Ok(Value::UInt(a >> b))
                } else {
                    Err(BytecodeError::InvalidOperand("Shift amount out of range".to_string()))
                }
            }
            _ => Err(BytecodeError::InvalidOperand("Cannot shift these types".to_string())),
        }
    }

    fn unsigned_shift_right(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                if b >= 0 && b < 64 {
                    Ok(Value::UInt((a as u64) >> b))
                } else {
                    Err(BytecodeError::InvalidOperand("Shift amount out of range".to_string()))
                }
            }
            (Value::UInt(a), Value::Int(b)) => {
                if b >= 0 && b < 64 {
                    Ok(Value::UInt(a >> b))
                } else {
                    Err(BytecodeError::InvalidOperand("Shift amount out of range".to_string()))
                }
            }
            _ => Err(BytecodeError::InvalidOperand("Cannot shift these types".to_string())),
        }
    }

    // Enhanced arithmetic operations

    fn div_rem_values(&self, a: Value, b: Value) -> BytecodeResult<(Value, Value)> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                if b == 0 {
                    Err(BytecodeError::InvalidOperand("Division by zero".to_string()))
                } else {
                    Ok((Value::Int(a / b), Value::Int(a % b)))
                }
            }
            (Value::UInt(a), Value::UInt(b)) => {
                if b == 0 {
                    Err(BytecodeError::InvalidOperand("Division by zero".to_string()))
                } else {
                    Ok((Value::UInt(a / b), Value::UInt(a % b)))
                }
            }
            _ => Err(BytecodeError::InvalidOperand("Cannot divide these types".to_string())),
        }
    }

    fn abs_value(&self, a: Value) -> BytecodeResult<Value> {
        match a {
            Value::Int(a) => Ok(Value::Int(a.abs())),
            Value::Float(a) => Ok(Value::Float(a.abs())),
            _ => Err(BytecodeError::InvalidOperand("Cannot take absolute value of this type".to_string())),
        }
    }

    fn neg_value(&self, a: Value) -> BytecodeResult<Value> {
        match a {
            Value::Int(a) => Ok(Value::Int(-a)),
            Value::Float(a) => Ok(Value::Float(-a)),
            _ => Err(BytecodeError::InvalidOperand("Cannot negate this type".to_string())),
        }
    }

    fn min_values(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.min(b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float((a as f64).min(b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(b as f64))),
            _ => Err(BytecodeError::InvalidOperand("Cannot compare these types".to_string())),
        }
    }

    fn max_values(&self, a: Value, b: Value) -> BytecodeResult<Value> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.max(b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float((a as f64).max(b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(b as f64))),
            _ => Err(BytecodeError::InvalidOperand("Cannot compare these types".to_string())),
        }
    }
    
    /// Get VM statistics
    pub fn stats(&self) -> &VMStats {
        &self.stats
    }
}

impl Default for BytecodeVM {
    fn default() -> Self {
        Self::new()
    }
}
