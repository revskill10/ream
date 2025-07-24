//! # REAM: Rust Erlang Abstract Machine
//! 
//! A mathematically-grounded actor runtime with bytecode JIT compilation and TLISP.
//! 
//! REAM is designed as a product category of Actor × Scheduler × Memory × Message,
//! providing a composable foundation for distributed computing.

#![allow(missing_docs)]
#![warn(clippy::all)]
#![allow(dead_code)] // Allow during development

pub mod runtime;
pub mod bytecode;
pub mod jit;
pub mod tlisp;
pub mod types;
pub mod error;
pub mod debug;
pub mod security;
pub mod p2p;
pub mod sqlite;
pub mod orm;
// Re-export procedural macros from ream-macros crate
pub use ream_macros::*;
/// Command-line interface and argument parsing
pub mod cli;
/// Read-Eval-Print Loop implementation
pub mod repl;
/// Command execution and orchestration
pub mod commands;
/// Daemon mode and monitoring
pub mod daemon;

// Re-export main types
pub use runtime::{ReamRuntime, advanced_runtime::AdvancedReamRuntime};
pub use types::{Pid, Priority, ProcessState, EffectGrade, ExecutionBounds, MemoryLayout, Versioned};
pub use error::{ReamError, ReamResult, FaultError, StmError, WasmError};
pub use bytecode::{BytecodeProgram, BytecodeVM, Value as BytecodeValue};
pub use jit::{JitContext, JitRuntime};
pub use tlisp::{TlispRuntime, TlispInterpreter, Value as TlispValue};
pub use orm::{OrmContext, SqliteOrm, PostgresOrm, Schema, Query, Driver};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Create a new REAM runtime with default configuration
pub fn new_runtime() -> ReamRuntime {
    ReamRuntime::new().unwrap()
}

/// Create a new REAM runtime with JIT compilation
pub fn new_jit_runtime() -> JitRuntime {
    let ream_runtime = ReamRuntime::new().unwrap();
    JitRuntime::new(ream_runtime)
}

/// Create a new TLISP runtime with REAM integration
pub fn new_tlisp_runtime() -> TlispRuntime {
    let ream_runtime = ReamRuntime::new().unwrap();
    TlispRuntime::with_ream(ream_runtime)
}

/// Create a complete REAM system with all components
pub fn new_complete_system() -> CompleteReamSystem {
    CompleteReamSystem::new()
}

/// Create a new advanced REAM runtime with production-grade features
pub fn new_advanced_runtime() -> AdvancedReamRuntime {
    AdvancedReamRuntime::new().expect("Failed to create advanced runtime")
}

/// Complete REAM system with all components integrated
pub struct CompleteReamSystem {
    /// Core REAM runtime
    pub ream_runtime: ReamRuntime,
    /// JIT compilation runtime
    pub jit_runtime: JitRuntime,
    /// TLISP runtime
    pub tlisp_runtime: TlispRuntime,
    /// Bytecode VM
    pub bytecode_vm: BytecodeVM,
}

impl CompleteReamSystem {
    /// Create a new complete REAM system
    pub fn new() -> Self {
        let ream_runtime = ReamRuntime::new().unwrap();
        let jit_runtime = JitRuntime::new(ReamRuntime::new().unwrap());
        let tlisp_runtime = TlispRuntime::with_ream(ReamRuntime::new().unwrap());
        let bytecode_vm = BytecodeVM::new();

        CompleteReamSystem {
            ream_runtime,
            jit_runtime,
            tlisp_runtime,
            bytecode_vm,
        }
    }

    /// Start all runtime components
    pub fn start(&self) -> ReamResult<()> {
        self.ream_runtime.start()?;
        Ok(())
    }

    /// Stop all runtime components
    pub fn stop(&self) -> ReamResult<()> {
        self.ream_runtime.stop()?;
        Ok(())
    }

    /// Execute TLISP code
    pub fn eval_tlisp(&mut self, code: &str) -> Result<TlispValue, error::TlispError> {
        self.tlisp_runtime.eval(code)
    }

    /// Execute bytecode
    pub fn execute_bytecode(&mut self, program: &BytecodeProgram) -> Result<BytecodeValue, error::BytecodeError> {
        self.bytecode_vm.execute_program(program)
    }

    /// Execute with JIT compilation
    pub fn execute_jit(&mut self, program: &BytecodeProgram) -> Result<BytecodeValue, error::JitError> {
        self.jit_runtime.execute_program(program)
    }
}

impl Default for CompleteReamSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ream_initialization() {
        let runtime = new_runtime();
        // Basic runtime creation test
        assert_eq!(runtime.process_count(), 0);
    }

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_complete_system() {
        let system = new_complete_system();
        // Test that all components are created
        assert_eq!(system.ream_runtime.process_count(), 0);
    }

    #[test]
    fn test_jit_runtime() {
        let _jit_runtime = new_jit_runtime();
        // Basic JIT runtime creation test
    }

    #[test]
    fn test_tlisp_runtime() {
        let _tlisp_runtime = new_tlisp_runtime();
        // Basic TLISP runtime creation test
    }
}
