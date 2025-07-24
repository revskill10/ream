//! JIT compiler implementation

use std::collections::HashMap;
use crate::bytecode::{Bytecode, BytecodeProgram, Value};
use crate::jit::{JitFunction, JitMetadata};

use crate::error::{JitError, JitResult};

/// REAM JIT compiler
pub struct ReamJIT {
    /// Optimization level
    opt_level: u8,
    /// Generated code buffer
    code_buffer: Vec<u8>,
    /// Function metadata
    metadata: JitMetadata,
    /// Label table for jumps
    labels: HashMap<u32, usize>,
    /// Pending label references
    pending_labels: Vec<(usize, u32)>,
}

impl ReamJIT {
    /// Create a new JIT compiler
    pub fn new() -> Self {
        ReamJIT {
            opt_level: 2,
            code_buffer: Vec::new(),
            metadata: JitMetadata {
                bytecode_size: 0,
                native_size: 0,
                compile_time: std::time::Duration::new(0, 0),
                opt_level: 2,
                hot_spots: Vec::new(),
            },
            labels: HashMap::new(),
            pending_labels: Vec::new(),
        }
    }
    
    /// Set optimization level
    pub fn set_optimization_level(&mut self, level: u8) {
        self.opt_level = level.min(3);
        self.metadata.opt_level = self.opt_level;
    }
    
    /// Compile a bytecode program to native code
    pub fn compile_program(&mut self, program: &BytecodeProgram) -> JitResult<JitFunction> {
        let start_time = std::time::Instant::now();
        
        self.metadata.bytecode_size = program.instructions.len();
        self.code_buffer.clear();
        self.labels.clear();
        self.pending_labels.clear();
        
        // Generate function prologue
        self.emit_prologue()?;
        
        // Compile each instruction
        for (pc, instruction) in program.instructions.iter().enumerate() {
            // Define a label for this instruction position
            self.labels.insert(pc as u32, self.code_buffer.len());

            self.compile_instruction(instruction, pc, program)?;
        }
        
        // Generate function epilogue
        self.emit_epilogue()?;
        
        // Resolve pending labels
        self.resolve_labels()?;
        
        // Allocate executable memory
        let function_ptr = self.allocate_executable_memory()?;
        
        self.metadata.native_size = self.code_buffer.len();
        self.metadata.compile_time = start_time.elapsed();
        
        let effect_grade = program.analyze_effects();
        
        Ok(JitFunction::new(
            function_ptr,
            self.code_buffer.len(),
            effect_grade,
            self.metadata.clone(),
        ))
    }
    
    /// Compile a single instruction
    fn compile_instruction(
        &mut self,
        instruction: &Bytecode,
        _pc: usize,
        program: &BytecodeProgram,
    ) -> JitResult<()> {
        match instruction {
            Bytecode::Const(idx, _) => {
                self.emit_load_constant(*idx, program)?;
            }
            Bytecode::Add(_) => {
                self.emit_add()?;
            }
            Bytecode::Sub(_) => {
                self.emit_sub()?;
            }
            Bytecode::Mul(_) => {
                self.emit_mul()?;
            }
            Bytecode::Load(idx, _) => {
                self.emit_load_local(*idx)?;
            }
            Bytecode::Store(idx, _) => {
                self.emit_store_local(*idx)?;
            }
            Bytecode::LoadGlobal(idx, _) => {
                self.emit_load_global(*idx)?;
            }
            Bytecode::StoreGlobal(idx, _) => {
                self.emit_store_global(*idx)?;
            }
            Bytecode::Jump(target, _) => {
                self.emit_jump(*target)?;
            }
            Bytecode::JumpIf(target, _) => {
                self.emit_jump_if(*target)?;
            }
            Bytecode::JumpIfNot(target, _) => {
                self.emit_jump_if_not(*target)?;
            }
            Bytecode::Call(func_idx, _) => {
                self.emit_call(*func_idx)?;
            }
            Bytecode::Ret(_) => {
                self.emit_return()?;
            }
            Bytecode::SpawnProcess(func_idx, _) => {
                self.emit_spawn(*func_idx)?;
            }
            Bytecode::SendMessage(pid_idx, msg_idx, _) => {
                self.emit_send(*pid_idx, *msg_idx)?;
            }
            Bytecode::ReceiveMessage(_) => {
                self.emit_receive()?;
            }
            Bytecode::Print(_) => {
                self.emit_print()?;
            }
            Bytecode::Nop(_) => {
                self.emit_nop()?;
            }
            _ => {
                // For unimplemented instructions, emit a placeholder
                self.emit_nop()?;
            }
        }
        
        Ok(())
    }
    
    // Code generation methods (simplified x86-64 assembly)
    
    fn emit_prologue(&mut self) -> JitResult<()> {
        // push rbp
        self.code_buffer.push(0x55);
        // mov rbp, rsp
        self.code_buffer.extend_from_slice(&[0x48, 0x89, 0xe5]);
        // sub rsp, 0x100 (allocate stack space)
        self.code_buffer.extend_from_slice(&[0x48, 0x81, 0xec, 0x00, 0x01, 0x00, 0x00]);
        Ok(())
    }
    
    fn emit_epilogue(&mut self) -> JitResult<()> {
        // mov rsp, rbp
        self.code_buffer.extend_from_slice(&[0x48, 0x89, 0xec]);
        // pop rbp
        self.code_buffer.push(0x5d);
        // ret
        self.code_buffer.push(0xc3);
        Ok(())
    }
    
    fn emit_load_constant(&mut self, idx: u32, program: &BytecodeProgram) -> JitResult<()> {
        // Simplified: load constant into rax
        if let Some(constant) = program.constants.get(idx as usize) {
            match constant {
                Value::Int(val) => {
                    // mov rax, immediate
                    self.code_buffer.extend_from_slice(&[0x48, 0xb8]);
                    self.code_buffer.extend_from_slice(&val.to_le_bytes());
                }
                _ => {
                    // For other types, use a placeholder
                    self.emit_nop()?;
                }
            }
        }
        Ok(())
    }
    
    fn emit_add(&mut self) -> JitResult<()> {
        // Simplified: add two values on stack
        // pop rbx
        self.code_buffer.push(0x5b);
        // pop rax
        self.code_buffer.push(0x58);
        // add rax, rbx
        self.code_buffer.extend_from_slice(&[0x48, 0x01, 0xd8]);
        // push rax
        self.code_buffer.push(0x50);
        Ok(())
    }
    
    fn emit_sub(&mut self) -> JitResult<()> {
        // Simplified: subtract two values on stack
        // pop rbx
        self.code_buffer.push(0x5b);
        // pop rax
        self.code_buffer.push(0x58);
        // sub rax, rbx
        self.code_buffer.extend_from_slice(&[0x48, 0x29, 0xd8]);
        // push rax
        self.code_buffer.push(0x50);
        Ok(())
    }
    
    fn emit_mul(&mut self) -> JitResult<()> {
        // Simplified: multiply two values on stack
        // pop rbx
        self.code_buffer.push(0x5b);
        // pop rax
        self.code_buffer.push(0x58);
        // imul rax, rbx
        self.code_buffer.extend_from_slice(&[0x48, 0x0f, 0xaf, 0xc3]);
        // push rax
        self.code_buffer.push(0x50);
        Ok(())
    }
    
    fn emit_load_local(&mut self, idx: u32) -> JitResult<()> {
        // Simplified: load local variable
        // mov rax, [rbp - offset]
        let offset = (idx + 1) * 8; // 8 bytes per local
        self.code_buffer.extend_from_slice(&[0x48, 0x8b, 0x45]);
        self.code_buffer.push(-(offset as i8) as u8);
        // push rax
        self.code_buffer.push(0x50);
        Ok(())
    }
    
    fn emit_store_local(&mut self, idx: u32) -> JitResult<()> {
        // Simplified: store to local variable
        // pop rax
        self.code_buffer.push(0x58);
        // mov [rbp - offset], rax
        let offset = (idx + 1) * 8;
        self.code_buffer.extend_from_slice(&[0x48, 0x89, 0x45]);
        self.code_buffer.push(-(offset as i8) as u8);
        Ok(())
    }

    fn emit_load_global(&mut self, _idx: u32) -> JitResult<()> {
        // Placeholder for global variable loading
        // In a real implementation, this would access a global variable table
        self.emit_nop()
    }

    fn emit_store_global(&mut self, _idx: u32) -> JitResult<()> {
        // Placeholder for global variable storing
        // In a real implementation, this would access a global variable table
        self.emit_nop()
    }
    
    fn emit_jump(&mut self, target: u32) -> JitResult<()> {
        // jmp relative
        self.code_buffer.push(0xe9);
        
        if let Some(&target_addr) = self.labels.get(&target) {
            let current_addr = self.code_buffer.len();
            let offset = (target_addr as i32) - (current_addr as i32) - 4;
            self.code_buffer.extend_from_slice(&offset.to_le_bytes());
        } else {
            // Add to pending labels
            self.pending_labels.push((self.code_buffer.len(), target));
            self.code_buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Placeholder
        }
        
        Ok(())
    }
    
    fn emit_jump_if(&mut self, target: u32) -> JitResult<()> {
        // Simplified: conditional jump (jump if true)
        // pop rax
        self.code_buffer.push(0x58);
        // test rax, rax
        self.code_buffer.extend_from_slice(&[0x48, 0x85, 0xc0]);
        // jnz relative
        self.code_buffer.extend_from_slice(&[0x0f, 0x85]);

        if let Some(&target_addr) = self.labels.get(&target) {
            let current_addr = self.code_buffer.len();
            let offset = (target_addr as i32) - (current_addr as i32) - 4;
            self.code_buffer.extend_from_slice(&offset.to_le_bytes());
        } else {
            self.pending_labels.push((self.code_buffer.len(), target));
            self.code_buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        }

        Ok(())
    }

    fn emit_jump_if_not(&mut self, target: u32) -> JitResult<()> {
        // Simplified: conditional jump (jump if false)
        // pop rax
        self.code_buffer.push(0x58);
        // test rax, rax
        self.code_buffer.extend_from_slice(&[0x48, 0x85, 0xc0]);
        // jz relative (jump if zero/false)
        self.code_buffer.extend_from_slice(&[0x0f, 0x84]);

        if let Some(&target_addr) = self.labels.get(&target) {
            let current_addr = self.code_buffer.len();
            let offset = (target_addr as i32) - (current_addr as i32) - 4;
            self.code_buffer.extend_from_slice(&offset.to_le_bytes());
        } else {
            self.pending_labels.push((self.code_buffer.len(), target));
            self.code_buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        }

        Ok(())
    }
    
    fn emit_call(&mut self, _func_idx: u32) -> JitResult<()> {
        // Simplified: function call (placeholder)
        self.emit_nop()
    }
    
    fn emit_return(&mut self) -> JitResult<()> {
        // pop rax (return value)
        self.code_buffer.push(0x58);
        // jmp to epilogue (simplified)
        self.emit_epilogue()
    }
    
    fn emit_spawn(&mut self, _func_idx: u32) -> JitResult<()> {
        // Placeholder for process spawning
        self.emit_nop()
    }
    
    fn emit_send(&mut self, _pid_idx: u32, _msg_idx: u32) -> JitResult<()> {
        // Placeholder for message sending
        self.emit_nop()
    }
    
    fn emit_receive(&mut self) -> JitResult<()> {
        // Placeholder for message receiving
        self.emit_nop()
    }

    fn emit_print(&mut self) -> JitResult<()> {
        // Placeholder for print operation
        // In a real implementation, this would call a print function
        self.emit_nop()
    }

    fn emit_nop(&mut self) -> JitResult<()> {
        // nop instruction
        self.code_buffer.push(0x90);
        Ok(())
    }
    
    fn resolve_labels(&mut self) -> JitResult<()> {
        for (patch_addr, target) in &self.pending_labels {
            if let Some(&target_addr) = self.labels.get(target) {
                let offset = (target_addr as i32) - (*patch_addr as i32) - 4;
                let offset_bytes = offset.to_le_bytes();
                
                for (i, &byte) in offset_bytes.iter().enumerate() {
                    if patch_addr + i < self.code_buffer.len() {
                        self.code_buffer[patch_addr + i] = byte;
                    }
                }
            } else {
                return Err(JitError::CodeGeneration(
                    format!("Unresolved label: {}", target)
                ));
            }
        }
        Ok(())
    }
    
    fn allocate_executable_memory(&self) -> JitResult<*const u8> {
        // In a real implementation, this would use mmap or VirtualAlloc
        // to allocate executable memory. For now, we'll use a simplified approach.
        
        #[cfg(unix)]
        {
            use std::ptr;
            
            unsafe {
                let size = self.code_buffer.len();
                let ptr = libc::mmap(
                    ptr::null_mut(),
                    size,
                    libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                );
                
                if ptr == libc::MAP_FAILED {
                    return Err(JitError::CodeGeneration("Failed to allocate executable memory".to_string()));
                }
                
                // Copy code to executable memory
                ptr::copy_nonoverlapping(self.code_buffer.as_ptr(), ptr as *mut u8, size);
                
                Ok(ptr as *const u8)
            }
        }
        
        #[cfg(windows)]
        {
            use std::ptr;

            unsafe {
                let size = self.code_buffer.len();

                // Allocate executable memory using VirtualAlloc
                let ptr = winapi::um::memoryapi::VirtualAlloc(
                    ptr::null_mut(),
                    size,
                    winapi::um::winnt::MEM_COMMIT | winapi::um::winnt::MEM_RESERVE,
                    winapi::um::winnt::PAGE_EXECUTE_READWRITE,
                );

                if ptr.is_null() {
                    return Err(JitError::CodeGeneration("Failed to allocate executable memory".to_string()));
                }

                // Copy code to executable memory
                ptr::copy_nonoverlapping(self.code_buffer.as_ptr(), ptr as *mut u8, size);

                Ok(ptr as *const u8)
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            // Fallback for other systems - return error instead of non-executable memory
            Err(JitError::CodeGeneration("JIT compilation not supported on this platform".to_string()))
        }
    }
}

impl Default for ReamJIT {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{BytecodeProgram, Value};
    use crate::types::EffectGrade;

    #[test]
    fn test_jit_creation() {
        let jit = ReamJIT::new();
        assert_eq!(jit.opt_level, 2);
        assert_eq!(jit.code_buffer.len(), 0);
    }
    
    #[test]
    fn test_code_generation() {
        let mut jit = ReamJIT::new();
        
        // Test prologue generation
        jit.emit_prologue().unwrap();
        assert!(jit.code_buffer.len() > 0);
        
        // Test instruction emission
        jit.emit_nop().unwrap();
        assert!(jit.code_buffer.len() > 7); // Prologue + nop
    }
    
    #[test]
    fn test_program_compilation() {
        let mut jit = ReamJIT::new();
        let mut program = BytecodeProgram::new("test".to_string());
        
        let const_id = program.add_constant(Value::Int(42));
        program.add_instruction(Bytecode::Const(const_id, EffectGrade::Pure));
        program.add_instruction(Bytecode::Ret(EffectGrade::Pure));
        
        // This should compile without errors
        let result = jit.compile_program(&program);
        assert!(result.is_ok());
    }
}
