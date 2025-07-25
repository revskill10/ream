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

    // Bitwise operations
    /// Bitwise AND
    BitAnd(EffectGrade),
    /// Bitwise OR
    BitOr(EffectGrade),
    /// Bitwise XOR
    BitXor(EffectGrade),
    /// Bitwise NOT
    BitNot(EffectGrade),
    /// Left shift
    ShiftLeft(EffectGrade),
    /// Right shift
    ShiftRight(EffectGrade),
    /// Unsigned right shift
    UnsignedShiftRight(EffectGrade),

    // Enhanced arithmetic
    /// Combined division and modulo
    DivRem(EffectGrade),
    /// Absolute value
    Abs(EffectGrade),
    /// Negation
    Neg(EffectGrade),
    /// Minimum of two values
    Min(EffectGrade),
    /// Maximum of two values
    Max(EffectGrade),
    /// Square root
    Sqrt(EffectGrade),
    /// Power operation
    Pow(EffectGrade),
    /// Sine
    Sin(EffectGrade),
    /// Cosine
    Cos(EffectGrade),
    /// Tangent
    Tan(EffectGrade),
    /// Natural logarithm
    Log(EffectGrade),
    /// Exponential
    Exp(EffectGrade),

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
    
    // String operations
    /// Get string length
    StrLen(EffectGrade),
    /// Concatenate strings
    StrConcat(EffectGrade),
    /// String slice
    StrSlice(u32, u32, EffectGrade), // start, end
    /// String index
    StrIndex(EffectGrade),
    /// String split
    StrSplit(u32, EffectGrade), // delimiter constant index

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
    /// Array slice
    ArraySlice(u32, u32, EffectGrade), // start, end
    /// Array concatenation
    ArrayConcat(EffectGrade),
    /// Array sort
    ArraySort(EffectGrade),
    /// Array map
    ArrayMap(u32, EffectGrade), // function index
    /// Array filter
    ArrayFilter(u32, EffectGrade), // function index

    // Map/Dictionary operations
    /// Create empty map
    MapNew(EffectGrade),
    /// Get map value
    MapGet(EffectGrade),
    /// Put map value
    MapPut(EffectGrade),
    /// Remove map entry
    MapRemove(EffectGrade),
    /// Get map keys
    MapKeys(EffectGrade),
    /// Get map values
    MapValues(EffectGrade),
    /// Get map size
    MapSize(EffectGrade),
    
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
    
    // Memory management operations
    /// Allocate memory
    Alloc(u32, EffectGrade), // size
    /// Free memory
    Free(EffectGrade),
    /// Garbage collection
    GcCollect(EffectGrade),
    /// Garbage collection info
    GcInfo(EffectGrade),
    /// Weak reference
    WeakRef(EffectGrade),
    /// Phantom reference
    PhantomRef(EffectGrade),

    // Atomic operations
    /// Atomic load
    AtomicLoad(u32, EffectGrade), // memory ordering
    /// Atomic store
    AtomicStore(u32, EffectGrade), // memory ordering
    /// Compare and swap
    CompareAndSwap(u32, EffectGrade), // memory ordering
    /// Fetch and add
    FetchAndAdd(u32, EffectGrade), // memory ordering
    /// Fetch and subtract
    FetchAndSub(u32, EffectGrade), // memory ordering
    /// Memory barrier
    MemoryBarrier(u32, EffectGrade), // memory ordering
    /// Fence
    Fence(u32, EffectGrade), // memory ordering

    // I/O operations
    /// Print value
    Print(EffectGrade),
    /// Read input
    Read(EffectGrade),

    // File I/O operations
    /// Open file
    FileOpen(u32, u32, EffectGrade), // path, mode
    /// Read from file
    FileRead(u32, EffectGrade), // size
    /// Write to file
    FileWrite(EffectGrade),
    /// Close file
    FileClose(EffectGrade),
    /// Seek in file
    FileSeek(u32, EffectGrade), // position
    /// Get file status
    FileStat(EffectGrade),

    // Network I/O operations
    /// Create socket
    SocketCreate(u32, EffectGrade), // type
    /// Bind socket
    SocketBind(EffectGrade),
    /// Connect socket
    SocketConnect(EffectGrade),
    /// Send data
    SocketSend(u32, EffectGrade), // flags
    /// Receive data
    SocketRecv(u32, EffectGrade), // size
    /// Close socket
    SocketClose(EffectGrade),

    // Time operations
    /// Get current time
    GetTime(EffectGrade),
    /// Sleep
    Sleep(EffectGrade),
    /// Set timer
    SetTimer(EffectGrade),
    /// Cancel timer
    CancelTimer(EffectGrade),

    // Random operations
    /// Generate random number
    Random(EffectGrade),
    /// Set random seed
    RandomSeed(EffectGrade),
    /// Generate random bytes
    RandomBytes(u32, EffectGrade), // size

    // Cryptographic operations
    /// Hash data
    Hash(u32, EffectGrade), // algorithm
    /// Encrypt data
    Encrypt(u32, EffectGrade), // algorithm
    /// Decrypt data
    Decrypt(u32, EffectGrade), // algorithm
    /// Sign data
    Sign(u32, EffectGrade), // algorithm
    /// Verify signature
    Verify(u32, EffectGrade), // algorithm
    
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
            // Bitwise operations
            Bytecode::BitAnd(effect) => *effect,
            Bytecode::BitOr(effect) => *effect,
            Bytecode::BitXor(effect) => *effect,
            Bytecode::BitNot(effect) => *effect,
            Bytecode::ShiftLeft(effect) => *effect,
            Bytecode::ShiftRight(effect) => *effect,
            Bytecode::UnsignedShiftRight(effect) => *effect,
            // Enhanced arithmetic
            Bytecode::DivRem(effect) => *effect,
            Bytecode::Abs(effect) => *effect,
            Bytecode::Neg(effect) => *effect,
            Bytecode::Min(effect) => *effect,
            Bytecode::Max(effect) => *effect,
            Bytecode::Sqrt(effect) => *effect,
            Bytecode::Pow(effect) => *effect,
            Bytecode::Sin(effect) => *effect,
            Bytecode::Cos(effect) => *effect,
            Bytecode::Tan(effect) => *effect,
            Bytecode::Log(effect) => *effect,
            Bytecode::Exp(effect) => *effect,
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
            // String operations
            Bytecode::StrLen(effect) => *effect,
            Bytecode::StrConcat(effect) => *effect,
            Bytecode::StrSlice(_, _, effect) => *effect,
            Bytecode::StrIndex(effect) => *effect,
            Bytecode::StrSplit(_, effect) => *effect,
            Bytecode::ListNew(effect) => *effect,
            Bytecode::ListLen(effect) => *effect,
            Bytecode::ListGet(effect) => *effect,
            Bytecode::ListSet(effect) => *effect,
            Bytecode::ListAppend(effect) => *effect,
            // Array operations
            Bytecode::ArraySlice(_, _, effect) => *effect,
            Bytecode::ArrayConcat(effect) => *effect,
            Bytecode::ArraySort(effect) => *effect,
            Bytecode::ArrayMap(_, effect) => *effect,
            Bytecode::ArrayFilter(_, effect) => *effect,
            // Map operations
            Bytecode::MapNew(effect) => *effect,
            Bytecode::MapGet(effect) => *effect,
            Bytecode::MapPut(effect) => *effect,
            Bytecode::MapRemove(effect) => *effect,
            Bytecode::MapKeys(effect) => *effect,
            Bytecode::MapValues(effect) => *effect,
            Bytecode::MapSize(effect) => *effect,
            Bytecode::SpawnProcess(_, effect) => *effect,
            Bytecode::SendMessage(_, _, effect) => *effect,
            Bytecode::ReceiveMessage(effect) => *effect,
            Bytecode::Link(_, effect) => *effect,
            Bytecode::Monitor(_, effect) => *effect,
            Bytecode::Self_(effect) => *effect,
            // Memory management
            Bytecode::Alloc(_, effect) => *effect,
            Bytecode::Free(effect) => *effect,
            Bytecode::GcCollect(effect) => *effect,
            Bytecode::GcInfo(effect) => *effect,
            Bytecode::WeakRef(effect) => *effect,
            Bytecode::PhantomRef(effect) => *effect,
            // Atomic operations
            Bytecode::AtomicLoad(_, effect) => *effect,
            Bytecode::AtomicStore(_, effect) => *effect,
            Bytecode::CompareAndSwap(_, effect) => *effect,
            Bytecode::FetchAndAdd(_, effect) => *effect,
            Bytecode::FetchAndSub(_, effect) => *effect,
            Bytecode::MemoryBarrier(_, effect) => *effect,
            Bytecode::Fence(_, effect) => *effect,
            Bytecode::Print(effect) => *effect,
            Bytecode::Read(effect) => *effect,
            // File I/O
            Bytecode::FileOpen(_, _, effect) => *effect,
            Bytecode::FileRead(_, effect) => *effect,
            Bytecode::FileWrite(effect) => *effect,
            Bytecode::FileClose(effect) => *effect,
            Bytecode::FileSeek(_, effect) => *effect,
            Bytecode::FileStat(effect) => *effect,
            // Network I/O
            Bytecode::SocketCreate(_, effect) => *effect,
            Bytecode::SocketBind(effect) => *effect,
            Bytecode::SocketConnect(effect) => *effect,
            Bytecode::SocketSend(_, effect) => *effect,
            Bytecode::SocketRecv(_, effect) => *effect,
            Bytecode::SocketClose(effect) => *effect,
            // Time operations
            Bytecode::GetTime(effect) => *effect,
            Bytecode::Sleep(effect) => *effect,
            Bytecode::SetTimer(effect) => *effect,
            Bytecode::CancelTimer(effect) => *effect,
            // Random operations
            Bytecode::Random(effect) => *effect,
            Bytecode::RandomSeed(effect) => *effect,
            Bytecode::RandomBytes(_, effect) => *effect,
            // Cryptographic operations
            Bytecode::Hash(_, effect) => *effect,
            Bytecode::Encrypt(_, effect) => *effect,
            Bytecode::Decrypt(_, effect) => *effect,
            Bytecode::Sign(_, effect) => *effect,
            Bytecode::Verify(_, effect) => *effect,
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
            // Bitwise operations
            Bytecode::BitAnd(_) => "bit_and",
            Bytecode::BitOr(_) => "bit_or",
            Bytecode::BitXor(_) => "bit_xor",
            Bytecode::BitNot(_) => "bit_not",
            Bytecode::ShiftLeft(_) => "shift_left",
            Bytecode::ShiftRight(_) => "shift_right",
            Bytecode::UnsignedShiftRight(_) => "unsigned_shift_right",
            // Enhanced arithmetic
            Bytecode::DivRem(_) => "div_rem",
            Bytecode::Abs(_) => "abs",
            Bytecode::Neg(_) => "neg",
            Bytecode::Min(_) => "min",
            Bytecode::Max(_) => "max",
            Bytecode::Sqrt(_) => "sqrt",
            Bytecode::Pow(_) => "pow",
            Bytecode::Sin(_) => "sin",
            Bytecode::Cos(_) => "cos",
            Bytecode::Tan(_) => "tan",
            Bytecode::Log(_) => "log",
            Bytecode::Exp(_) => "exp",
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
            // String operations
            Bytecode::StrLen(_) => "str_len",
            Bytecode::StrConcat(_) => "str_concat",
            Bytecode::StrSlice(_, _, _) => "str_slice",
            Bytecode::StrIndex(_) => "str_index",
            Bytecode::StrSplit(_, _) => "str_split",
            Bytecode::ListNew(_) => "list_new",
            Bytecode::ListLen(_) => "list_len",
            Bytecode::ListGet(_) => "list_get",
            Bytecode::ListSet(_) => "list_set",
            Bytecode::ListAppend(_) => "list_append",
            // Array operations
            Bytecode::ArraySlice(_, _, _) => "array_slice",
            Bytecode::ArrayConcat(_) => "array_concat",
            Bytecode::ArraySort(_) => "array_sort",
            Bytecode::ArrayMap(_, _) => "array_map",
            Bytecode::ArrayFilter(_, _) => "array_filter",
            // Map operations
            Bytecode::MapNew(_) => "map_new",
            Bytecode::MapGet(_) => "map_get",
            Bytecode::MapPut(_) => "map_put",
            Bytecode::MapRemove(_) => "map_remove",
            Bytecode::MapKeys(_) => "map_keys",
            Bytecode::MapValues(_) => "map_values",
            Bytecode::MapSize(_) => "map_size",
            Bytecode::SpawnProcess(_, _) => "spawn_process",
            Bytecode::SendMessage(_, _, _) => "send_message",
            Bytecode::ReceiveMessage(_) => "receive_message",
            Bytecode::Link(_, _) => "link",
            Bytecode::Monitor(_, _) => "monitor",
            Bytecode::Self_(_) => "self",
            // Memory management
            Bytecode::Alloc(_, _) => "alloc",
            Bytecode::Free(_) => "free",
            Bytecode::GcCollect(_) => "gc_collect",
            Bytecode::GcInfo(_) => "gc_info",
            Bytecode::WeakRef(_) => "weak_ref",
            Bytecode::PhantomRef(_) => "phantom_ref",
            // Atomic operations
            Bytecode::AtomicLoad(_, _) => "atomic_load",
            Bytecode::AtomicStore(_, _) => "atomic_store",
            Bytecode::CompareAndSwap(_, _) => "compare_and_swap",
            Bytecode::FetchAndAdd(_, _) => "fetch_and_add",
            Bytecode::FetchAndSub(_, _) => "fetch_and_sub",
            Bytecode::MemoryBarrier(_, _) => "memory_barrier",
            Bytecode::Fence(_, _) => "fence",
            Bytecode::Print(_) => "print",
            Bytecode::Read(_) => "read",
            // File I/O
            Bytecode::FileOpen(_, _, _) => "file_open",
            Bytecode::FileRead(_, _) => "file_read",
            Bytecode::FileWrite(_) => "file_write",
            Bytecode::FileClose(_) => "file_close",
            Bytecode::FileSeek(_, _) => "file_seek",
            Bytecode::FileStat(_) => "file_stat",
            // Network I/O
            Bytecode::SocketCreate(_, _) => "socket_create",
            Bytecode::SocketBind(_) => "socket_bind",
            Bytecode::SocketConnect(_) => "socket_connect",
            Bytecode::SocketSend(_, _) => "socket_send",
            Bytecode::SocketRecv(_, _) => "socket_recv",
            Bytecode::SocketClose(_) => "socket_close",
            // Time operations
            Bytecode::GetTime(_) => "get_time",
            Bytecode::Sleep(_) => "sleep",
            Bytecode::SetTimer(_) => "set_timer",
            Bytecode::CancelTimer(_) => "cancel_timer",
            // Random operations
            Bytecode::Random(_) => "random",
            Bytecode::RandomSeed(_) => "random_seed",
            Bytecode::RandomBytes(_, _) => "random_bytes",
            // Cryptographic operations
            Bytecode::Hash(_, _) => "hash",
            Bytecode::Encrypt(_, _) => "encrypt",
            Bytecode::Decrypt(_, _) => "decrypt",
            Bytecode::Sign(_, _) => "sign",
            Bytecode::Verify(_, _) => "verify",
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
