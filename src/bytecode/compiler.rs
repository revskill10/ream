//! Bytecode compiler with cross-language support

use std::collections::HashMap;
use crate::bytecode::{Bytecode, Value, BytecodeProgram, BytecodeFunction, TypeInfo};
use crate::bytecode::program::FunctionSignature;
use crate::types::EffectGrade;
use crate::error::{BytecodeError, BytecodeResult};

/// Language-agnostic compiler interface
pub trait LanguageCompiler {
    /// Source AST type
    type AST;
    
    /// Compile AST to bytecode
    fn compile_to_bytecode(&self, ast: Self::AST) -> BytecodeResult<BytecodeProgram>;
    
    /// Get type information for AST node
    fn get_type_info(&self, ast: &Self::AST) -> TypeInfo;
}

/// Bytecode compiler with effect tracking
pub struct BytecodeCompiler {
    /// Current program being built
    program: BytecodeProgram,
    /// Label table for jumps
    labels: HashMap<String, u32>,
    /// Pending label references
    pending_labels: Vec<(usize, String)>,
    /// Local variable mapping
    locals: HashMap<String, u32>,
    /// Current local count
    local_count: u32,
    /// Effect stack for tracking nested effects
    effect_stack: Vec<EffectGrade>,
    /// Current function being compiled
    current_function: Option<BytecodeFunction>,
}

impl BytecodeCompiler {
    /// Create a new compiler
    pub fn new(program_name: String) -> Self {
        BytecodeCompiler {
            program: BytecodeProgram::new(program_name),
            labels: HashMap::new(),
            pending_labels: Vec::new(),
            locals: HashMap::new(),
            local_count: 0,
            effect_stack: vec![EffectGrade::Pure],
            current_function: None,
        }
    }
    
    /// Emit an instruction
    pub fn emit(&mut self, instruction: Bytecode) {
        self.update_effect_stack(instruction.effect_grade());
        
        if let Some(ref mut func) = self.current_function {
            func.add_instruction(instruction);
        } else {
            self.program.add_instruction(instruction);
        }
    }
    
    /// Add a constant and return its index
    pub fn add_const(&mut self, value: Value) -> u32 {
        self.program.add_constant(value)
    }
    
    /// Define a label at current position
    pub fn define_label(&mut self, name: String) {
        let pc = if let Some(ref func) = self.current_function {
            func.instruction_count() as u32
        } else {
            self.program.instructions.len() as u32
        };
        
        self.labels.insert(name.clone(), pc);
        
        // Resolve pending references
        self.resolve_pending_labels(&name, pc);
    }
    
    /// Reference a label (for jumps)
    pub fn label_ref(&mut self, name: String) -> u32 {
        if let Some(&pc) = self.labels.get(&name) {
            pc
        } else {
            // Add to pending list
            let current_pc = if let Some(ref func) = self.current_function {
                func.instruction_count()
            } else {
                self.program.instructions.len()
            };
            self.pending_labels.push((current_pc, name));
            0 // Placeholder
        }
    }
    
    /// Allocate a local variable
    pub fn alloc_local(&mut self, name: String) -> u32 {
        let index = self.local_count;
        self.locals.insert(name, index);
        self.local_count += 1;
        index
    }
    
    /// Resolve a local variable
    pub fn resolve_local(&self, name: &str) -> BytecodeResult<u32> {
        self.locals.get(name).copied()
            .ok_or_else(|| BytecodeError::InvalidOperand(format!("Undefined variable: {}", name)))
    }
    
    /// Start compiling a function
    pub fn start_function(&mut self, name: String, param_count: usize) -> u32 {
        let id = self.program.functions.len() as u32;
        let function = BytecodeFunction::new(id, name, param_count);
        
        // Save current state
        self.current_function = Some(function);
        self.locals.clear();
        self.local_count = param_count as u32; // Parameters are first locals
        
        id
    }
    
    /// Finish compiling current function
    pub fn finish_function(&mut self) -> BytecodeResult<u32> {
        if let Some(mut function) = self.current_function.take() {
            function.local_count = self.local_count as usize;
            function.effect_grade = self.current_effect();
            
            let id = function.id;
            self.program.add_function(function);
            
            // Reset state
            self.locals.clear();
            self.local_count = 0;
            
            Ok(id)
        } else {
            Err(BytecodeError::CompilationFailed("No function being compiled".to_string()))
        }
    }
    
    /// Set function signature
    pub fn set_function_signature(&mut self, signature: FunctionSignature) -> BytecodeResult<()> {
        if let Some(ref mut function) = self.current_function {
            function.set_signature(signature);
            Ok(())
        } else {
            Err(BytecodeError::CompilationFailed("No function being compiled".to_string()))
        }
    }
    
    /// Export a function
    pub fn export_function(&mut self, name: String, function_id: u32) -> BytecodeResult<()> {
        self.program.export_function(name, function_id)
    }
    
    /// Push effect onto stack
    pub fn push_effect(&mut self, effect: EffectGrade) {
        self.effect_stack.push(effect);
    }
    
    /// Pop effect from stack
    pub fn pop_effect(&mut self) -> EffectGrade {
        self.effect_stack.pop().unwrap_or(EffectGrade::Pure)
    }
    
    /// Get current effect
    pub fn current_effect(&self) -> EffectGrade {
        self.effect_stack.iter().fold(EffectGrade::Pure, |acc, &effect| acc.combine(effect))
    }
    
    /// Finish compilation and return program
    pub fn finish(mut self) -> BytecodeResult<BytecodeProgram> {
        // Resolve any remaining pending labels
        if !self.pending_labels.is_empty() {
            return Err(BytecodeError::CompilationFailed(
                format!("Unresolved labels: {:?}",
                    self.pending_labels.iter().map(|(_, name)| name).collect::<Vec<_>>())
            ));
        }

        // Perform final optimizations on the instruction sequence
        self.optimize_instructions();

        // Validate jump targets
        self.validate_jump_targets()?;
        
        // Validate the program
        self.program.validate()?;
        
        Ok(self.program)
    }
    
    // Helper methods
    
    fn update_effect_stack(&mut self, effect: EffectGrade) {
        if let Some(last) = self.effect_stack.last_mut() {
            *last = last.combine(effect);
        }
    }
    
    fn resolve_pending_labels(&mut self, name: &str, pc: u32) {
        let mut resolved = Vec::new();

        for (i, (instr_pc, label_name)) in self.pending_labels.iter().enumerate() {
            if label_name == name {
                // Update the instruction with the correct PC
                if let Some(ref mut func) = self.current_function {
                    if let Some(instr) = func.instructions.get_mut(*instr_pc) {
                        Self::update_instruction_target_static(instr, pc);
                    }
                } else if let Some(instr) = self.program.instructions.get_mut(*instr_pc) {
                    Self::update_instruction_target_static(instr, pc);
                }
                resolved.push(i);
            }
        }

        // Remove resolved labels (in reverse order to maintain indices)
        for &i in resolved.iter().rev() {
            self.pending_labels.remove(i);
        }
    }
    
    fn update_instruction_target_static(instruction: &mut Bytecode, target: u32) {
        match instruction {
            Bytecode::Jump(ref mut t, _) => *t = target,
            Bytecode::JumpIf(ref mut t, _) => *t = target,
            Bytecode::JumpIfNot(ref mut t, _) => *t = target,
            _ => {}
        }
    }
}

/// High-level compilation helpers
impl BytecodeCompiler {
    /// Compile a literal value
    pub fn compile_literal(&mut self, value: Value) {
        let const_id = self.add_const(value);
        self.emit(Bytecode::Const(const_id, EffectGrade::Pure));
    }
    
    /// Compile a variable load
    pub fn compile_load(&mut self, name: &str) -> BytecodeResult<()> {
        let local_id = self.resolve_local(name)?;
        self.emit(Bytecode::Load(local_id, EffectGrade::Read));
        Ok(())
    }
    
    /// Compile a variable store
    pub fn compile_store(&mut self, name: &str) -> BytecodeResult<()> {
        let local_id = self.resolve_local(name)?;
        self.emit(Bytecode::Store(local_id, EffectGrade::Write));
        Ok(())
    }
    
    /// Compile binary operation
    pub fn compile_binary_op(&mut self, op: &str) -> BytecodeResult<()> {
        match op {
            "+" => self.emit(Bytecode::Add(EffectGrade::Pure)),
            "-" => self.emit(Bytecode::Sub(EffectGrade::Pure)),
            "*" => self.emit(Bytecode::Mul(EffectGrade::Pure)),
            "/" => self.emit(Bytecode::Div(EffectGrade::Pure)),
            "%" => self.emit(Bytecode::Mod(EffectGrade::Pure)),
            "==" => self.emit(Bytecode::Eq(EffectGrade::Pure)),
            "<" => self.emit(Bytecode::Lt(EffectGrade::Pure)),
            "<=" => self.emit(Bytecode::Le(EffectGrade::Pure)),
            ">" => self.emit(Bytecode::Gt(EffectGrade::Pure)),
            ">=" => self.emit(Bytecode::Ge(EffectGrade::Pure)),
            "&&" => self.emit(Bytecode::And(EffectGrade::Pure)),
            "||" => self.emit(Bytecode::Or(EffectGrade::Pure)),
            _ => return Err(BytecodeError::InvalidOperand(format!("Unknown operator: {}", op))),
        }
        Ok(())
    }
    
    /// Compile unary operation
    pub fn compile_unary_op(&mut self, op: &str) -> BytecodeResult<()> {
        match op {
            "!" => self.emit(Bytecode::Not(EffectGrade::Pure)),
            _ => return Err(BytecodeError::InvalidOperand(format!("Unknown unary operator: {}", op))),
        }
        Ok(())
    }
    
    /// Compile function call
    pub fn compile_call(&mut self, function_id: u32) {
        self.emit(Bytecode::Call(function_id, EffectGrade::Send));
    }
    
    /// Compile return statement
    pub fn compile_return(&mut self) {
        self.emit(Bytecode::Ret(EffectGrade::Pure));
    }
    
    /// Compile conditional jump
    pub fn compile_if(&mut self, else_label: &str, _end_label: &str) {
        let else_pc = self.label_ref(else_label.to_string());
        self.emit(Bytecode::JumpIfNot(else_pc, EffectGrade::Pure));
    }
    
    /// Compile unconditional jump
    pub fn compile_jump(&mut self, label: &str) {
        let pc = self.label_ref(label.to_string());
        self.emit(Bytecode::Jump(pc, EffectGrade::Pure));
    }
    
    /// Compile actor spawn
    pub fn compile_spawn(&mut self, function_id: u32) {
        self.emit(Bytecode::SpawnProcess(function_id, EffectGrade::Spawn));
    }
    
    /// Compile message send
    pub fn compile_send(&mut self, pid_local: u32, msg_local: u32) {
        self.emit(Bytecode::SendMessage(pid_local, msg_local, EffectGrade::Send));
    }
    
    /// Compile message receive
    pub fn compile_receive(&mut self) {
        self.emit(Bytecode::ReceiveMessage(EffectGrade::Read));
    }

    /// Add a compiled program to this compiler
    pub fn add_program(&mut self, program: BytecodeProgram) {
        // Merge the program's instructions into this compiler's program
        for instruction in &program.instructions {
            self.program.add_instruction(instruction.clone());
        }

        // Merge constants, functions, etc.
        // This is a simplified implementation
    }

    /// Add a constant to the program
    pub fn add_constant(&mut self, value: Value) -> u32 {
        self.program.add_constant(value)
    }

    /// Perform final optimizations on the instruction sequence
    fn optimize_instructions(&mut self) {
        // Basic peephole optimizations
        let mut i = 0;

        while i < self.program.instructions.len().saturating_sub(1) {
            // Check for redundant load/store pairs
            let should_remove = {
                if let (Some(load), Some(store)) = (
                    self.program.instructions.get(i),
                    self.program.instructions.get(i + 1)
                ) {
                    self.is_redundant_load_store(load, store)
                } else {
                    false
                }
            };

            if should_remove {
                self.program.instructions.remove(i);
                self.program.instructions.remove(i); // Remove the next one (now at position i)
                continue;
            }
            i += 1;
        }
    }

    /// Check if a load/store pair is redundant
    fn is_redundant_load_store(&self, _load: &Bytecode, _store: &Bytecode) -> bool {
        // Simplified check - in practice this would be more sophisticated
        false
    }

    /// Validate that all jump targets are valid
    fn validate_jump_targets(&self) -> BytecodeResult<()> {
        let instructions = &self.program.instructions;

        for (i, instruction) in instructions.iter().enumerate() {
            match instruction {
                Bytecode::Jump(target, _) | Bytecode::JumpIf(target, _) | Bytecode::JumpIfNot(target, _) => {
                    if *target as usize >= instructions.len() {
                        return Err(BytecodeError::CompilationFailed(
                            format!("Invalid jump target {} at instruction {}", target, i)
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_compilation() {
        let mut compiler = BytecodeCompiler::new("test".to_string());
        
        // Compile: 1 + 2
        compiler.compile_literal(Value::Int(1));
        compiler.compile_literal(Value::Int(2));
        compiler.compile_binary_op("+").unwrap();
        
        let program = compiler.finish().unwrap();
        assert_eq!(program.instructions.len(), 3);
        assert_eq!(program.constants.len(), 2);
    }
    
    #[test]
    fn test_function_compilation() {
        let mut compiler = BytecodeCompiler::new("test".to_string());

        // Compile function: fn add(a, b) { return a + b; }
        let _func_id = compiler.start_function("add".to_string(), 2);

        // Allocate local variables first
        compiler.alloc_local("a".to_string());
        compiler.alloc_local("b".to_string());

        // Now we can load them
        compiler.compile_load("a").unwrap();
        compiler.compile_load("b").unwrap();
        compiler.compile_binary_op("+").unwrap();
        compiler.compile_return();

        let _function = compiler.finish_function().unwrap();

        // Test the structure
        assert!(compiler.current_function.is_none()); // Function should be ended
    }
    
    #[test]
    fn test_label_resolution() {
        let mut compiler = BytecodeCompiler::new("test".to_string());
        
        compiler.define_label("start".to_string());
        compiler.compile_literal(Value::Int(1));
        compiler.compile_jump("start");
        
        let program = compiler.finish().unwrap();
        assert_eq!(program.instructions.len(), 2);
    }
}
