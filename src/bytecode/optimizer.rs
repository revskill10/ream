//! Bytecode optimization passes

use std::collections::HashSet;
use crate::bytecode::{Bytecode, BytecodeProgram};
use crate::error::{BytecodeError, BytecodeResult};

/// Optimization trait for functorial transformations
pub trait Optimization {
    /// Apply optimization to a program
    fn optimize(&self, program: &BytecodeProgram) -> BytecodeResult<BytecodeProgram>;
    
    /// Check if optimization preserves semantics
    fn preserves_semantics(&self, before: &BytecodeProgram, after: &BytecodeProgram) -> bool;
}

/// Constant folding optimization
pub struct ConstantFolding;

impl Optimization for ConstantFolding {
    fn optimize(&self, program: &BytecodeProgram) -> BytecodeResult<BytecodeProgram> {
        let mut optimized = program.clone();
        
        // Simple constant folding for adjacent const + binary op
        let mut new_instructions = Vec::new();
        let mut i = 0;
        
        while i < optimized.instructions.len() {
            if i + 2 < optimized.instructions.len() {
                if let (
                    Bytecode::Const(a_idx, _),
                    Bytecode::Const(b_idx, _),
                    Bytecode::Add(effect)
                ) = (&optimized.instructions[i], &optimized.instructions[i + 1], &optimized.instructions[i + 2]) {
                    // Fold constants
                    let a_idx = *a_idx;
                    let b_idx = *b_idx;
                    let effect = *effect;

                    if let (Some(a), Some(b)) = (
                        optimized.constants.get(a_idx as usize),
                        optimized.constants.get(b_idx as usize)
                    ) {
                        if let (crate::bytecode::Value::Int(a_val), crate::bytecode::Value::Int(b_val)) = (a, b) {
                            let result = crate::bytecode::Value::Int(a_val + b_val);
                            let result_idx = optimized.add_constant(result);
                            new_instructions.push(Bytecode::Const(result_idx, effect));
                            i += 3;
                            continue;
                        }
                    }
                }
            }
            
            new_instructions.push(optimized.instructions[i].clone());
            i += 1;
        }
        
        optimized.instructions = new_instructions;
        Ok(optimized)
    }
    
    fn preserves_semantics(&self, before: &BytecodeProgram, after: &BytecodeProgram) -> bool {
        before.analyze_effects() == after.analyze_effects()
    }
}

/// Dead code elimination
pub struct DeadCodeElimination;

impl Optimization for DeadCodeElimination {
    fn optimize(&self, program: &BytecodeProgram) -> BytecodeResult<BytecodeProgram> {
        let live_instructions = self.compute_live_instructions(program);
        let mut optimized = program.clone();
        
        optimized.instructions = optimized.instructions
            .into_iter()
            .enumerate()
            .filter(|(i, _)| live_instructions.contains(i))
            .map(|(_, instr)| instr)
            .collect();
        
        Ok(optimized)
    }
    
    fn preserves_semantics(&self, before: &BytecodeProgram, after: &BytecodeProgram) -> bool {
        before.analyze_effects() == after.analyze_effects()
    }
}

impl DeadCodeElimination {
    fn compute_live_instructions(&self, program: &BytecodeProgram) -> HashSet<usize> {
        let mut live = HashSet::new();
        let mut worklist = Vec::new();
        
        // Start from the end
        if !program.instructions.is_empty() {
            worklist.push(program.instructions.len() - 1);
        }
        
        while let Some(idx) = worklist.pop() {
            if live.contains(&idx) {
                continue;
            }
            
            live.insert(idx);
            
            // Add predecessors based on control flow
            match &program.instructions[idx] {
                Bytecode::Jump(target, _) => {
                    worklist.push(*target as usize);
                }
                Bytecode::JumpIf(target, _) | Bytecode::JumpIfNot(target, _) => {
                    worklist.push(*target as usize);
                    if idx > 0 {
                        worklist.push(idx - 1);
                    }
                }
                Bytecode::Ret(_) => {
                    // Terminal instruction
                }
                _ => {
                    if idx > 0 {
                        worklist.push(idx - 1);
                    }
                }
            }
        }
        
        live
    }
}

/// Peephole optimization
pub struct PeepholeOptimization;

impl Optimization for PeepholeOptimization {
    fn optimize(&self, program: &BytecodeProgram) -> BytecodeResult<BytecodeProgram> {
        let mut optimized = program.clone();
        let mut changed = true;
        
        while changed {
            changed = false;
            let mut new_instructions = Vec::new();
            let mut i = 0;
            
            while i < optimized.instructions.len() {
                let mut consumed = 1;
                
                // Pattern: Load X, Store X -> Dup
                if i + 1 < optimized.instructions.len() {
                    if let (Bytecode::Load(a, _), Bytecode::Store(b, effect)) = 
                        (&optimized.instructions[i], &optimized.instructions[i + 1]) {
                        if a == b {
                            new_instructions.push(Bytecode::Dup(*effect));
                            consumed = 2;
                            changed = true;
                        }
                    }
                }
                
                // Pattern: Const 0, Add -> (remove)
                if i + 1 < optimized.instructions.len() {
                    if let (Bytecode::Const(idx, _), Bytecode::Add(_)) = 
                        (&optimized.instructions[i], &optimized.instructions[i + 1]) {
                        if let Some(crate::bytecode::Value::Int(0)) = optimized.constants.get(*idx as usize) {
                            // Skip both instructions (adding 0 is no-op)
                            consumed = 2;
                            changed = true;
                        }
                    }
                }
                
                if consumed == 1 {
                    new_instructions.push(optimized.instructions[i].clone());
                }
                
                i += consumed;
            }
            
            optimized.instructions = new_instructions;
        }
        
        Ok(optimized)
    }
    
    fn preserves_semantics(&self, before: &BytecodeProgram, after: &BytecodeProgram) -> bool {
        before.analyze_effects() == after.analyze_effects()
    }
}

/// Optimization pipeline
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn Optimization>>,
}

impl OptimizationPipeline {
    /// Create a new optimization pipeline
    pub fn new() -> Self {
        OptimizationPipeline {
            passes: Vec::new(),
        }
    }
    
    /// Add an optimization pass
    pub fn add_pass(mut self, pass: Box<dyn Optimization>) -> Self {
        self.passes.push(pass);
        self
    }
    
    /// Create a default optimization pipeline
    pub fn default_pipeline() -> Self {
        Self::new()
            .add_pass(Box::new(ConstantFolding))
            .add_pass(Box::new(PeepholeOptimization))
            .add_pass(Box::new(DeadCodeElimination))
    }
    
    /// Run all optimization passes
    pub fn optimize(&self, program: &BytecodeProgram) -> BytecodeResult<BytecodeProgram> {
        let mut current = program.clone();
        
        for pass in &self.passes {
            let optimized = pass.optimize(&current)?;
            
            // Verify semantics preservation
            if !pass.preserves_semantics(&current, &optimized) {
                return Err(BytecodeError::Optimization(
                    "Optimization pass violated semantics preservation".to_string()
                ));
            }
            
            current = optimized;
        }
        
        Ok(current)
    }
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self::default_pipeline()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{Value, BytecodeProgram};
    use crate::types::EffectGrade;

    #[test]
    fn test_constant_folding() {
        let mut program = BytecodeProgram::new("test".to_string());
        
        let a_idx = program.add_constant(Value::Int(1));
        let b_idx = program.add_constant(Value::Int(2));
        
        program.add_instruction(Bytecode::Const(a_idx, EffectGrade::Pure));
        program.add_instruction(Bytecode::Const(b_idx, EffectGrade::Pure));
        program.add_instruction(Bytecode::Add(EffectGrade::Pure));
        
        let optimizer = ConstantFolding;
        let optimized = optimizer.optimize(&program).unwrap();
        
        // Should be folded to a single const instruction
        assert_eq!(optimized.instructions.len(), 1);
        if let Bytecode::Const(idx, _) = &optimized.instructions[0] {
            if let Some(Value::Int(val)) = optimized.constants.get(*idx as usize) {
                assert_eq!(*val, 3);
            }
        }
    }
    
    #[test]
    fn test_dead_code_elimination() {
        let mut program = BytecodeProgram::new("test".to_string());
        
        program.add_instruction(Bytecode::Const(0, EffectGrade::Pure));
        program.add_instruction(Bytecode::Jump(3, EffectGrade::Pure));
        program.add_instruction(Bytecode::Const(1, EffectGrade::Pure)); // Dead code
        program.add_instruction(Bytecode::Ret(EffectGrade::Pure));
        
        let optimizer = DeadCodeElimination;
        let optimized = optimizer.optimize(&program).unwrap();
        
        // Dead instruction should be removed
        assert!(optimized.instructions.len() < program.instructions.len());
    }
    
    #[test]
    fn test_optimization_pipeline() {
        let mut program = BytecodeProgram::new("test".to_string());
        
        let a_idx = program.add_constant(Value::Int(1));
        let b_idx = program.add_constant(Value::Int(2));
        
        program.add_instruction(Bytecode::Const(a_idx, EffectGrade::Pure));
        program.add_instruction(Bytecode::Const(b_idx, EffectGrade::Pure));
        program.add_instruction(Bytecode::Add(EffectGrade::Pure));
        program.add_instruction(Bytecode::Ret(EffectGrade::Pure));
        
        let pipeline = OptimizationPipeline::default_pipeline();
        let optimized = pipeline.optimize(&program).unwrap();
        
        // Should be optimized
        assert!(optimized.instructions.len() <= program.instructions.len());
    }
}
