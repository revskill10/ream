//! Error types for REAM


use thiserror::Error;
use crate::types::Pid;

/// Main error type for REAM operations
#[derive(Error, Debug)]
pub enum ReamError {
    /// Runtime errors
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),

    /// Bytecode compilation errors
    #[error("Bytecode error: {0}")]
    Bytecode(#[from] BytecodeError),

    /// JIT compilation errors
    #[error("JIT error: {0}")]
    Jit(#[from] JitError),

    /// TLISP errors
    #[error("TLISP error: {0}")]
    Tlisp(#[from] TlispError),

    /// Fault tolerance errors
    #[error("Fault error: {0}")]
    Fault(#[from] FaultError),

    /// STM errors
    #[error("STM error: {0}")]
    Stm(#[from] StmError),

    /// WASM errors
    #[error("WASM error: {0}")]
    Wasm(#[from] WasmError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Feature not implemented yet
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Other errors
    #[error("Error: {0}")]
    Other(String),
}

/// Runtime-specific errors
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// Process not found
    #[error("Process {0} not found")]
    ProcessNotFound(Pid),
    
    /// Mailbox is full
    #[error("Mailbox full for process {0}")]
    MailboxFull(Pid),
    
    /// Invalid message type
    #[error("Invalid message type: {0}")]
    InvalidMessage(String),
    
    /// Scheduler error
    #[error("Scheduler error: {0}")]
    Scheduler(String),
    
    /// Memory allocation error
    #[error("Memory allocation failed: {0}")]
    Memory(String),
    
    /// Supervision error
    #[error("Supervision error: {0}")]
    Supervision(String),

    /// Serverless error
    #[error("Serverless error: {0}")]
    Serverless(String),
    
    /// Process already exists
    #[error("Process {0} already exists")]
    ProcessExists(Pid),
    
    /// Maximum processes reached
    #[error("Maximum number of processes ({0}) reached")]
    MaxProcesses(usize),

    /// General runtime error
    #[error("Runtime error: {0}")]
    RuntimeError(String),

    /// TLISP error
    #[error("TLISP error: {0}")]
    TlispError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Bytecode compilation errors
#[derive(Error, Debug)]
pub enum BytecodeError {
    /// Invalid instruction
    #[error("Invalid instruction: {0}")]
    InvalidInstruction(String),

    #[error("Optimization error: {0}")]
    Optimization(String),


    
    /// Invalid operand
    #[error("Invalid operand: {0}")]
    InvalidOperand(String),
    
    /// Effect grade mismatch
    #[error("Effect grade mismatch: expected {expected:?}, got {actual:?}")]
    EffectMismatch { expected: crate::EffectGrade, actual: crate::EffectGrade },
    
    /// Compilation failed
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    
    /// Unknown language
    #[error("Unknown language: {0}")]
    UnknownLanguage(String),
    
    /// No bridge available
    #[error("No bridge available for language: {0}")]
    NoBridge(String),
}

/// JIT compilation errors
#[derive(Error, Debug)]
pub enum JitError {
    /// Code generation failed
    #[error("Code generation failed: {0}")]
    CodeGeneration(String),
    
    /// Optimization failed
    #[error("Optimization failed: {0}")]
    Optimization(String),
    
    /// Assembly error
    #[error("Assembly error: {0}")]
    Assembly(String),
    
    /// Execution error
    #[error("Execution error: {0}")]
    Execution(String),
    
    /// Invalid function signature
    #[error("Invalid function signature: {0}")]
    InvalidSignature(String),
}

/// TLISP-specific errors
#[derive(Error, Debug)]
pub enum TlispError {
    /// Parse errors
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    
    /// Type errors
    #[error("Type error: {0}")]
    Type(#[from] TypeError),
    
    /// Runtime errors
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    /// Macro expansion errors
    #[error("Macro error: {0}")]
    Macro(#[from] MacroError),
}

/// Parse errors for TLISP
#[derive(Error, Debug)]
pub enum ParseError {
    /// Unexpected token
    #[error("Unexpected token at position {position}: {token}")]
    UnexpectedToken { position: usize, token: String },
    
    /// Unterminated list
    #[error("Unterminated list starting at position {0}")]
    UnterminatedList(usize),
    
    /// Invalid number format
    #[error("Invalid number format: {0}")]
    InvalidNumber(String),
    
    /// Invalid symbol
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),
    
    /// EOF while parsing
    #[error("Unexpected end of input")]
    UnexpectedEof,
}

/// Type system errors
#[derive(Error, Debug)]
pub enum TypeError {
    /// Type mismatch
    #[error("Type mismatch: expected {expected}, got {actual}")]
    Mismatch { expected: String, actual: String },
    
    /// Unification failure
    #[error("Cannot unify types: {left} and {right}")]
    UnificationFailure { left: String, right: String },
    
    /// Arity mismatch
    #[error("Arity mismatch: expected {expected} arguments, got {actual}")]
    ArityMismatch { expected: usize, actual: usize },
    
    /// Undefined variable
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    /// Type mismatch (legacy format for macro compatibility)
    #[error("Type mismatch: expected {0}, got {1}")]
    TypeMismatch(String, String),

    /// Missing field in struct
    #[error("Missing field: {0}")]
    MissingField(String),
    
    /// Occurs check failure
    #[error("Occurs check failed: {var} occurs in {ty}")]
    OccursCheck { var: String, ty: String },

    /// Invalid type expression
    #[error("Invalid type expression: {0}")]
    InvalidTypeExpression(String),

    /// Kind mismatch
    #[error("Kind mismatch: expected {expected}, got {actual}")]
    KindMismatch { expected: String, actual: String },

    /// Not a type function
    #[error("Not a type function: {0}")]
    NotATypeFunction(String),

    /// Invalid type condition
    #[error("Invalid type condition: {0}")]
    InvalidTypeCondition(String),

    /// Invalid type comparison
    #[error("Invalid type comparison: {0}")]
    InvalidTypeComparison(String),

    /// Invalid type term
    #[error("Invalid type term: {0}")]
    InvalidTypeTerm(String),

    /// Maximum depth exceeded
    #[error("Maximum depth exceeded: {0}")]
    MaxDepthExceeded(usize),

    /// Unsupported expression
    #[error("Unsupported expression: {0}")]
    UnsupportedExpression(String),
}

/// Macro expansion errors
#[derive(Error, Debug)]
pub enum MacroError {
    /// Arity mismatch in macro call
    #[error("Macro arity mismatch: expected {expected}, got {actual}")]
    ArityMismatch { expected: usize, actual: usize },
    
    /// Invalid macro definition
    #[error("Invalid macro definition: {0}")]
    InvalidDefinition(String),
    
    /// Expansion failed
    #[error("Macro expansion failed: {0}")]
    ExpansionFailed(String),
    
    /// Recursive expansion
    #[error("Recursive macro expansion detected: {0}")]
    RecursiveExpansion(String),
}

/// Result type for REAM operations
pub type ReamResult<T> = Result<T, ReamError>;

/// Result type for runtime operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Result type for bytecode operations
pub type BytecodeResult<T> = Result<T, BytecodeError>;

/// Result type for JIT operations
pub type JitResult<T> = Result<T, JitError>;

/// Result type for TLISP operations
pub type TlispResult<T> = Result<T, TlispError>;

// ============================================================================
// PRODUCTION-GRADE ERROR TYPES FOR FAULT TOLERANCE, STM, AND WASM
// ============================================================================

/// Fault tolerance errors
#[derive(Error, Debug)]
pub enum FaultError {
    /// Process isolation violation
    #[error("Process isolation violation")]
    IsolationViolation,

    /// Memory boundary exceeded
    #[error("Memory boundary exceeded")]
    MemoryBoundaryExceeded,

    /// Instruction limit exceeded
    #[error("Instruction limit exceeded")]
    InstructionLimitExceeded,

    /// Message quota exceeded
    #[error("Message quota exceeded")]
    MessageQuotaExceeded,

    /// Fault handler error
    #[error("Fault handler error: {0}")]
    FaultHandler(String),

    /// Supervisor error
    #[error("Supervisor error: {0}")]
    Supervisor(String),

    /// Recovery action failed
    #[error("Recovery action failed: {0}")]
    RecoveryFailed(String),
}

/// STM (Software Transactional Memory) errors
#[derive(Error, Debug)]
pub enum StmError {
    /// Transaction conflict detected
    #[error("Transaction conflict")]
    Conflict,

    /// Transaction timeout
    #[error("Transaction timeout")]
    Timeout,

    /// Resource exhausted
    #[error("Resource exhausted")]
    ResourceExhausted,

    /// Invalid transaction state
    #[error("Invalid transaction state: {0}")]
    InvalidState(String),

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u64, actual: u64 },

    /// Deadlock detected
    #[error("Deadlock detected")]
    Deadlock,

    /// Retry limit exceeded
    #[error("Retry limit exceeded")]
    RetryLimitExceeded,
}

/// WebAssembly compilation and runtime errors
#[derive(Error, Debug)]
pub enum WasmError {
    /// WASM compilation failed
    #[error("WASM compilation failed: {0}")]
    CompilationFailed(String),

    /// WASM instantiation failed
    #[error("WASM instantiation failed: {0}")]
    InstantiationFailed(String),

    /// WASM execution failed
    #[error("WASM execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid WASM module
    #[error("Invalid WASM module: {0}")]
    InvalidModule(String),

    /// Missing export
    #[error("Missing export: {0}")]
    MissingExport(String),

    /// Type mismatch in WASM call
    #[error("Type mismatch in WASM call: {0}")]
    TypeMismatch(String),

    /// Memory access violation
    #[error("Memory access violation: {0}")]
    MemoryViolation(String),

    /// Stack overflow
    #[error("Stack overflow")]
    StackOverflow,

    /// Trap occurred
    #[error("Trap occurred: {0}")]
    Trap(String),
}

/// Result type for fault tolerance operations
pub type FaultResult<T> = Result<T, FaultError>;

/// Result type for STM operations
pub type StmResult<T> = Result<T, StmError>;

/// Result type for WASM operations
pub type WasmResult<T> = Result<T, WasmError>;
