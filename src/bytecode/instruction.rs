//! Bytecode instructions as initial algebra of REAM operations

use serde::{Deserialize, Serialize};
use crate::types::EffectGrade;

/// Bytecode instructions with effect annotations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Bytecode {
    // Pure operations
    /// Load constant value
    Const(u32, EffectGrade),
    /// Add two values
    Add(EffectGrade),
    /// Subtract two values
    Sub(EffectGrade),
    /// Multiply two values
    Mul(EffectGrade),
    /// Divide two values
    Div(EffectGrade),
    /// Modulo operation
    Mod(EffectGrade),
    /// Logical AND
    And(EffectGrade),
    /// Logical OR
    Or(EffectGrade),
    /// Logical NOT
    Not(EffectGrade),
    /// Equality comparison
    Eq(EffectGrade),
    /// Less than comparison
    Lt(EffectGrade),
    /// Less than or equal comparison
    Le(EffectGrade),
    /// Greater than comparison
    Gt(EffectGrade),
    /// Greater than or equal comparison
    Ge(EffectGrade),
    
    // Memory operations
    /// Load local variable
    Load(u32, EffectGrade),
    /// Store to local variable
    Store(u32, EffectGrade),
    /// Load from global
    LoadGlobal(u32, EffectGrade),
    /// Store to global
    StoreGlobal(u32, EffectGrade),
    
    // Control flow
    /// Unconditional jump
    Jump(u32, EffectGrade),
    /// Conditional jump (jump if true)
    JumpIf(u32, EffectGrade),
    /// Conditional jump (jump if false)
    JumpIfNot(u32, EffectGrade),
    /// Function call
    Call(u32, EffectGrade),
    /// Return from function
    Ret(EffectGrade),
    
    // Stack operations
    /// Duplicate top of stack
    Dup(EffectGrade),
    /// Pop top of stack
    Pop(EffectGrade),
    /// Swap top two stack elements
    Swap(EffectGrade),
    
    // List operations
    /// Create empty list
    ListNew(EffectGrade),
    /// Get list length
    ListLen(EffectGrade),
    /// Get list element
    ListGet(EffectGrade),
    /// Set list element
    ListSet(EffectGrade),
    /// Append to list
    ListAppend(EffectGrade),
    
    // Actor operations
    /// Spawn a new process
    SpawnProcess(u32, EffectGrade),
    /// Send message to process
    SendMessage(u32, u32, EffectGrade),
    /// Receive message
    ReceiveMessage(EffectGrade),
    /// Link to process
    Link(u32, EffectGrade),
    /// Monitor process
    Monitor(u32, EffectGrade),
    /// Get current process ID
    Self_(EffectGrade),
    
    // I/O operations
    /// Print value
    Print(EffectGrade),
    /// Read input
    Read(EffectGrade),
    
    // Type operations
    /// Get type of value
    TypeOf(EffectGrade),
    /// Type cast
    Cast(u32, EffectGrade),
    
    // Debug operations
    /// Debug print
    Debug(EffectGrade),
    /// Breakpoint
    Break(EffectGrade),
    
    // No-op
    Nop(EffectGrade),
}

impl Bytecode {
    /// Get the effect grade of this instruction
    pub fn effect_grade(&self) -> EffectGrade {
        match self {
            Bytecode::Const(_, effect) => *effect,
            Bytecode::Add(effect) => *effect,
            Bytecode::Sub(effect) => *effect,
            Bytecode::Mul(effect) => *effect,
            Bytecode::Div(effect) => *effect,
            Bytecode::Mod(effect) => *effect,
            Bytecode::And(effect) => *effect,
            Bytecode::Or(effect) => *effect,
            Bytecode::Not(effect) => *effect,
            Bytecode::Eq(effect) => *effect,
            Bytecode::Lt(effect) => *effect,
            Bytecode::Le(effect) => *effect,
            Bytecode::Gt(effect) => *effect,
            Bytecode::Ge(effect) => *effect,
            Bytecode::Load(_, effect) => *effect,
            Bytecode::Store(_, effect) => *effect,
            Bytecode::LoadGlobal(_, effect) => *effect,
            Bytecode::StoreGlobal(_, effect) => *effect,
            Bytecode::Jump(_, effect) => *effect,
            Bytecode::JumpIf(_, effect) => *effect,
            Bytecode::JumpIfNot(_, effect) => *effect,
            Bytecode::Call(_, effect) => *effect,
            Bytecode::Ret(effect) => *effect,
            Bytecode::Dup(effect) => *effect,
            Bytecode::Pop(effect) => *effect,
            Bytecode::Swap(effect) => *effect,
            Bytecode::ListNew(effect) => *effect,
            Bytecode::ListLen(effect) => *effect,
            Bytecode::ListGet(effect) => *effect,
            Bytecode::ListSet(effect) => *effect,
            Bytecode::ListAppend(effect) => *effect,
            Bytecode::SpawnProcess(_, effect) => *effect,
            Bytecode::SendMessage(_, _, effect) => *effect,
            Bytecode::ReceiveMessage(effect) => *effect,
            Bytecode::Link(_, effect) => *effect,
            Bytecode::Monitor(_, effect) => *effect,
            Bytecode::Self_(effect) => *effect,
            Bytecode::Print(effect) => *effect,
            Bytecode::Read(effect) => *effect,
            Bytecode::TypeOf(effect) => *effect,
            Bytecode::Cast(_, effect) => *effect,
            Bytecode::Debug(effect) => *effect,
            Bytecode::Break(effect) => *effect,
            Bytecode::Nop(effect) => *effect,
        }
    }
    
    /// Check if this instruction has side effects
    pub fn has_side_effects(&self) -> bool {
        self.effect_grade() != EffectGrade::Pure
    }
    
    /// Get instruction name
    pub fn name(&self) -> &'static str {
        match self {
            Bytecode::Const(_, _) => "const",
            Bytecode::Add(_) => "add",
            Bytecode::Sub(_) => "sub",
            Bytecode::Mul(_) => "mul",
            Bytecode::Div(_) => "div",
            Bytecode::Mod(_) => "mod",
            Bytecode::And(_) => "and",
            Bytecode::Or(_) => "or",
            Bytecode::Not(_) => "not",
            Bytecode::Eq(_) => "eq",
            Bytecode::Lt(_) => "lt",
            Bytecode::Le(_) => "le",
            Bytecode::Gt(_) => "gt",
            Bytecode::Ge(_) => "ge",
            Bytecode::Load(_, _) => "load",
            Bytecode::Store(_, _) => "store",
            Bytecode::LoadGlobal(_, _) => "load_global",
            Bytecode::StoreGlobal(_, _) => "store_global",
            Bytecode::Jump(_, _) => "jump",
            Bytecode::JumpIf(_, _) => "jump_if",
            Bytecode::JumpIfNot(_, _) => "jump_if_not",
            Bytecode::Call(_, _) => "call",
            Bytecode::Ret(_) => "ret",
            Bytecode::Dup(_) => "dup",
            Bytecode::Pop(_) => "pop",
            Bytecode::Swap(_) => "swap",
            Bytecode::ListNew(_) => "list_new",
            Bytecode::ListLen(_) => "list_len",
            Bytecode::ListGet(_) => "list_get",
            Bytecode::ListSet(_) => "list_set",
            Bytecode::ListAppend(_) => "list_append",
            Bytecode::SpawnProcess(_, _) => "spawn_process",
            Bytecode::SendMessage(_, _, _) => "send_message",
            Bytecode::ReceiveMessage(_) => "receive_message",
            Bytecode::Link(_, _) => "link",
            Bytecode::Monitor(_, _) => "monitor",
            Bytecode::Self_(_) => "self",
            Bytecode::Print(_) => "print",
            Bytecode::Read(_) => "read",
            Bytecode::TypeOf(_) => "typeof",
            Bytecode::Cast(_, _) => "cast",
            Bytecode::Debug(_) => "debug",
            Bytecode::Break(_) => "break",
            Bytecode::Nop(_) => "nop",
        }
    }
    
    /// Get operand count
    pub fn operand_count(&self) -> usize {
        match self {
            Bytecode::Const(_, _) => 1,
            Bytecode::Load(_, _) => 1,
            Bytecode::Store(_, _) => 1,
            Bytecode::LoadGlobal(_, _) => 1,
            Bytecode::StoreGlobal(_, _) => 1,
            Bytecode::Jump(_, _) => 1,
            Bytecode::JumpIf(_, _) => 1,
            Bytecode::JumpIfNot(_, _) => 1,
            Bytecode::Call(_, _) => 1,
            Bytecode::SpawnProcess(_, _) => 1,
            Bytecode::Link(_, _) => 1,
            Bytecode::Monitor(_, _) => 1,
            Bytecode::Cast(_, _) => 1,
            Bytecode::SendMessage(_, _, _) => 2,
            _ => 0,
        }
    }
    
    /// Check if this is a control flow instruction
    pub fn is_control_flow(&self) -> bool {
        matches!(self, 
            Bytecode::Jump(_, _) | 
            Bytecode::JumpIf(_, _) | 
            Bytecode::JumpIfNot(_, _) | 
            Bytecode::Call(_, _) | 
            Bytecode::Ret(_)
        )
    }
    
    /// Check if this is a terminator instruction
    pub fn is_terminator(&self) -> bool {
        matches!(self, Bytecode::Ret(_))
    }
}

/// Instruction with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    /// The bytecode instruction
    pub opcode: Bytecode,
    /// Source location (line number)
    pub line: Option<u32>,
    /// Source file
    pub file: Option<String>,
    /// Instruction comment
    pub comment: Option<String>,
}

impl Instruction {
    /// Create a new instruction
    pub fn new(opcode: Bytecode) -> Self {
        Instruction {
            opcode,
            line: None,
            file: None,
            comment: None,
        }
    }
    
    /// Create instruction with source location
    pub fn with_location(opcode: Bytecode, line: u32, file: String) -> Self {
        Instruction {
            opcode,
            line: Some(line),
            file: Some(file),
            comment: None,
        }
    }
    
    /// Add a comment to the instruction
    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }
    
    /// Get effect grade
    pub fn effect_grade(&self) -> EffectGrade {
        self.opcode.effect_grade()
    }
    
    /// Get instruction name
    pub fn name(&self) -> &'static str {
        self.opcode.name()
    }
}

impl From<Bytecode> for Instruction {
    fn from(opcode: Bytecode) -> Self {
        Instruction::new(opcode)
    }
}

/// Effect tracking for instruction sequences
pub trait EffectTracking {
    /// Get the effect grade
    fn effect_grade(&self) -> EffectGrade;
    
    /// Compose effects with another instruction
    fn compose_effects(&self, other: &Self) -> EffectGrade;
}

impl EffectTracking for Instruction {
    fn effect_grade(&self) -> EffectGrade {
        self.opcode.effect_grade()
    }
    
    fn compose_effects(&self, other: &Self) -> EffectGrade {
        self.effect_grade().combine(other.effect_grade())
    }
}

impl EffectTracking for Vec<Instruction> {
    fn effect_grade(&self) -> EffectGrade {
        self.iter().fold(EffectGrade::Pure, |acc, instr| {
            acc.combine(instr.effect_grade())
        })
    }
    
    fn compose_effects(&self, other: &Self) -> EffectGrade {
        self.effect_grade().combine(other.effect_grade())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_effects() {
        let pure_instr = Bytecode::Add(EffectGrade::Pure);
        let io_instr = Bytecode::Print(EffectGrade::IO);
        
        assert_eq!(pure_instr.effect_grade(), EffectGrade::Pure);
        assert_eq!(io_instr.effect_grade(), EffectGrade::IO);
        assert!(!pure_instr.has_side_effects());
        assert!(io_instr.has_side_effects());
    }
    
    #[test]
    fn test_instruction_metadata() {
        let instr = Instruction::with_location(
            Bytecode::Const(0, EffectGrade::Pure),
            42,
            "test.ream".to_string()
        ).with_comment("Load constant 0".to_string());
        
        assert_eq!(instr.line, Some(42));
        assert_eq!(instr.file, Some("test.ream".to_string()));
        assert_eq!(instr.comment, Some("Load constant 0".to_string()));
    }
    
    #[test]
    fn test_effect_composition() {
        let instrs = vec![
            Instruction::new(Bytecode::Const(0, EffectGrade::Pure)),
            Instruction::new(Bytecode::Print(EffectGrade::IO)),
        ];
        
        assert_eq!(instrs.effect_grade(), EffectGrade::IO);
    }
}
