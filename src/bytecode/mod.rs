//! REAM Bytecode - Polymorphic bytecode format with effect tracking
//! 
//! Bytecode forms the initial algebra over a graded monad of instruction effects

pub mod instruction;
pub mod program;
pub mod compiler;
pub mod optimizer;
pub mod registry;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::types::{EffectGrade, Pid};
use crate::error::{BytecodeError, BytecodeResult};

pub use instruction::{Bytecode, Instruction};
pub use program::{BytecodeProgram, BytecodeFunction};
pub use compiler::{BytecodeCompiler, LanguageCompiler};
pub use optimizer::{Optimization, ConstantFolding, DeadCodeElimination};
pub use registry::BytecodeRegistry;

/// Value types in REAM bytecode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Integer value
    Int(i64),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
    /// List of values
    List(Vec<Value>),
    /// Function reference
    Function(u32),
    /// Process ID
    Pid(Pid),
    /// Null value
    Null,
}

impl Value {
    /// Get the type of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Function(_) => "function",
            Value::Pid(_) => "pid",
            Value::Null => "null",
        }
    }
    
    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Null => false,
            _ => true,
        }
    }
    
    /// Convert to integer if possible
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }
    
    /// Convert to float if possible
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
    
    /// Convert to string
    pub fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::List(l) => format!("[{}]", l.iter().map(|v| v.as_string()).collect::<Vec<_>>().join(", ")),
            Value::Function(f) => format!("function#{}", f),
            Value::Pid(p) => format!("pid#{}", p.raw()),
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
