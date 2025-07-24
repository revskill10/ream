//! Bytecode program representation and analysis

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::bytecode::{Bytecode, Value, TypeInfo};

use crate::types::EffectGrade;
use crate::error::{BytecodeError, BytecodeResult};

/// Bytecode function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeFunction {
    /// Function ID
    pub id: u32,
    /// Function name
    pub name: String,
    /// Parameter count
    pub param_count: usize,
    /// Local variable count
    pub local_count: usize,
    /// Starting program counter
    pub start_pc: usize,
    /// Function instructions
    pub instructions: Vec<Bytecode>,
    /// Function signature
    pub signature: FunctionSignature,
    /// Effect grade
    pub effect_grade: EffectGrade,
}

/// Function signature with type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Parameter types
    pub params: Vec<TypeInfo>,
    /// Return type
    pub return_type: TypeInfo,
    /// Effect constraints
    pub effects: EffectGrade,
}

impl BytecodeFunction {
    /// Create a new function
    pub fn new(id: u32, name: String, param_count: usize) -> Self {
        BytecodeFunction {
            id,
            name,
            param_count,
            local_count: 0,
            start_pc: 0,
            instructions: Vec::new(),
            signature: FunctionSignature {
                params: vec![TypeInfo::Unknown; param_count],
                return_type: TypeInfo::Unknown,
                effects: EffectGrade::Pure,
            },
            effect_grade: EffectGrade::Pure,
        }
    }
    
    /// Add an instruction to the function
    pub fn add_instruction(&mut self, instruction: Bytecode) {
        self.effect_grade = self.effect_grade.combine(instruction.effect_grade());
        self.instructions.push(instruction);
    }
    
    /// Set function signature
    pub fn set_signature(&mut self, signature: FunctionSignature) {
        self.signature = signature;
    }
    
    /// Get instruction count
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }
    
    /// Analyze function effects
    pub fn analyze_effects(&self) -> EffectGrade {
        self.instructions.iter().fold(EffectGrade::Pure, |acc, instr| {
            acc.combine(instr.effect_grade())
        })
    }
}

/// Complete bytecode program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeProgram {
    /// Program instructions
    pub instructions: Vec<Bytecode>,
    /// Constant pool
    pub constants: Vec<Value>,
    /// Function definitions
    pub functions: Vec<BytecodeFunction>,
    /// Global variable names
    pub globals: Vec<String>,
    /// Export table (name -> function id)
    pub exports: HashMap<String, u32>,
    /// Import table
    pub imports: HashMap<String, ImportInfo>,
    /// Program metadata
    pub metadata: ProgramMetadata,
}

/// Import information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    /// Module name
    pub module: String,
    /// Function name
    pub function: String,
    /// Function signature
    pub signature: FunctionSignature,
}

/// Program metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramMetadata {
    /// Program name
    pub name: String,
    /// Version
    pub version: String,
    /// Source language
    pub source_language: String,
    /// Compilation timestamp
    pub compiled_at: u64,
    /// Compiler version
    pub compiler_version: String,
    /// Debug information
    pub debug_info: Option<DebugInfo>,
}

/// Debug information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    /// Source file mapping
    pub source_files: Vec<String>,
    /// Line number mapping
    pub line_mapping: HashMap<usize, (usize, u32)>, // pc -> (file_index, line)
    /// Variable names
    pub variable_names: HashMap<usize, String>, // local_index -> name
}

impl BytecodeProgram {
    /// Create a new empty program
    pub fn new(name: String) -> Self {
        BytecodeProgram {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
            exports: HashMap::new(),
            imports: HashMap::new(),
            metadata: ProgramMetadata {
                name,
                version: "1.0.0".to_string(),
                source_language: "unknown".to_string(),
                compiled_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                debug_info: None,
            },
        }
    }
    
    /// Add a constant to the pool
    pub fn add_constant(&mut self, value: Value) -> u32 {
        // Check if constant already exists
        for (i, existing) in self.constants.iter().enumerate() {
            if *existing == value {
                return i as u32;
            }
        }
        
        let index = self.constants.len() as u32;
        self.constants.push(value);
        index
    }
    
    /// Add an instruction
    pub fn add_instruction(&mut self, instruction: Bytecode) {
        self.instructions.push(instruction);
    }

    /// Get mutable reference to instructions
    pub fn instructions_mut(&mut self) -> &mut Vec<Bytecode> {
        &mut self.instructions
    }
    
    /// Add a function
    pub fn add_function(&mut self, mut function: BytecodeFunction) -> u32 {
        function.start_pc = self.instructions.len();
        let id = function.id;
        
        // Add function instructions to main program
        self.instructions.extend(function.instructions.clone());
        
        self.functions.push(function);
        id
    }
    
    /// Add a global variable
    pub fn add_global(&mut self, name: String) -> u32 {
        let index = self.globals.len() as u32;
        self.globals.push(name);
        index
    }
    
    /// Export a function
    pub fn export_function(&mut self, name: String, function_id: u32) -> BytecodeResult<()> {
        if self.functions.iter().any(|f| f.id == function_id) {
            self.exports.insert(name, function_id);
            Ok(())
        } else {
            Err(BytecodeError::InvalidOperand(format!("Function {} not found", function_id)))
        }
    }
    
    /// Import a function
    pub fn import_function(&mut self, name: String, import_info: ImportInfo) {
        self.imports.insert(name, import_info);
    }
    
    /// Get function by ID
    pub fn get_function(&self, id: u32) -> Option<&BytecodeFunction> {
        self.functions.iter().find(|f| f.id == id)
    }
    
    /// Get function by name
    pub fn get_function_by_name(&self, name: &str) -> Option<&BytecodeFunction> {
        if let Some(&id) = self.exports.get(name) {
            self.get_function(id)
        } else {
            None
        }
    }
    
    /// Analyze program effects
    pub fn analyze_effects(&self) -> EffectGrade {
        self.instructions.iter().fold(EffectGrade::Pure, |acc, instr| {
            acc.combine(instr.effect_grade())
        })
    }
    
    /// Get program size in bytes (approximate)
    pub fn size(&self) -> usize {
        std::mem::size_of::<Bytecode>() * self.instructions.len() +
        self.constants.iter().map(|c| match c {
            Value::String(s) => s.len(),
            Value::List(l) => l.len() * std::mem::size_of::<Value>(),
            _ => std::mem::size_of::<Value>(),
        }).sum::<usize>() +
        self.functions.iter().map(|f| f.instruction_count() * std::mem::size_of::<Bytecode>()).sum::<usize>()
    }
    
    /// Validate program structure
    pub fn validate(&self) -> BytecodeResult<()> {
        // Check that all function calls reference valid functions
        for instruction in &self.instructions {
            match instruction {
                Bytecode::Call(func_id, _) => {
                    if !self.functions.iter().any(|f| f.id == *func_id) {
                        return Err(BytecodeError::InvalidOperand(
                            format!("Function {} not found", func_id)
                        ));
                    }
                }
                Bytecode::Const(const_id, _) => {
                    if *const_id as usize >= self.constants.len() {
                        return Err(BytecodeError::InvalidOperand(
                            format!("Constant {} not found", const_id)
                        ));
                    }
                }
                _ => {}
            }
        }
        
        // Check that all exports reference valid functions
        for (name, &func_id) in &self.exports {
            if !self.functions.iter().any(|f| f.id == func_id) {
                return Err(BytecodeError::InvalidOperand(
                    format!("Exported function {} references invalid function {}", name, func_id)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Combine two programs
    pub fn combine(mut self, other: BytecodeProgram) -> Self {
        // Merge constants
        let const_offset = self.constants.len() as u32;
        self.constants.extend(other.constants);
        
        // Merge functions with ID offset
        let func_id_offset = self.functions.iter().map(|f| f.id).max().unwrap_or(0) + 1;
        for mut func in other.functions {
            func.id += func_id_offset;
            func.start_pc += self.instructions.len();
            self.functions.push(func);
        }
        
        // Merge instructions with constant/function ID adjustments
        for mut instr in other.instructions {
            match &mut instr {
                Bytecode::Const(ref mut id, _) => *id += const_offset,
                Bytecode::Call(ref mut id, _) => *id += func_id_offset,
                _ => {}
            }
            self.instructions.push(instr);
        }
        
        // Merge globals
        self.globals.extend(other.globals);
        
        // Merge exports with function ID adjustment
        for (name, func_id) in other.exports {
            self.exports.insert(name, func_id + func_id_offset);
        }
        
        // Merge imports
        self.imports.extend(other.imports);
        
        self
    }
    
    /// Set debug information
    pub fn set_debug_info(&mut self, debug_info: DebugInfo) {
        self.metadata.debug_info = Some(debug_info);
    }
    
    /// Get source location for instruction
    pub fn get_source_location(&self, pc: usize) -> Option<(String, u32)> {
        if let Some(ref debug_info) = self.metadata.debug_info {
            if let Some(&(file_index, line)) = debug_info.line_mapping.get(&pc) {
                if let Some(file) = debug_info.source_files.get(file_index) {
                    return Some((file.clone(), line));
                }
            }
        }
        None
    }
}

impl Default for BytecodeProgram {
    fn default() -> Self {
        Self::new("unnamed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_creation() {
        let mut program = BytecodeProgram::new("test".to_string());
        
        let const_id = program.add_constant(Value::Int(42));
        program.add_instruction(Bytecode::Const(const_id, EffectGrade::Pure));
        
        assert_eq!(program.constants.len(), 1);
        assert_eq!(program.instructions.len(), 1);
    }
    
    #[test]
    fn test_function_management() {
        let mut program = BytecodeProgram::new("test".to_string());
        let mut function = BytecodeFunction::new(0, "main".to_string(), 0);
        
        function.add_instruction(Bytecode::Const(0, EffectGrade::Pure));
        function.add_instruction(Bytecode::Ret(EffectGrade::Pure));
        
        let func_id = program.add_function(function);
        program.export_function("main".to_string(), func_id).unwrap();
        
        assert_eq!(program.functions.len(), 1);
        assert!(program.exports.contains_key("main"));
    }
    
    #[test]
    fn test_program_validation() {
        let mut program = BytecodeProgram::new("test".to_string());
        
        // Valid program
        let const_id = program.add_constant(Value::Int(42));
        program.add_instruction(Bytecode::Const(const_id, EffectGrade::Pure));
        assert!(program.validate().is_ok());
        
        // Invalid constant reference
        program.add_instruction(Bytecode::Const(999, EffectGrade::Pure));
        assert!(program.validate().is_err());
    }
}
