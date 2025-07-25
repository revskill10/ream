//! JIT runtime integration

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::bytecode::BytecodeProgram;
use crate::jit::{JitContext, JitFunction, JitOptions};
use crate::runtime::ReamRuntime;
use crate::error::{JitError, JitResult};

/// Integrated JIT runtime combining REAM runtime with JIT compilation
pub struct JitRuntime {
    /// REAM runtime
    ream_runtime: Arc<ReamRuntime>,
    /// JIT context
    jit_context: Arc<RwLock<JitContext>>,
    /// Runtime options
    options: JitOptions,
}

impl JitRuntime {
    /// Create a new JIT runtime
    pub fn new(ream_runtime: ReamRuntime) -> Self {
        JitRuntime {
            ream_runtime: Arc::new(ream_runtime),
            jit_context: Arc::new(RwLock::new(JitContext::new())),
            options: JitOptions::default(),
        }
    }
    
    /// Create with custom options
    pub fn with_options(ream_runtime: ReamRuntime, options: JitOptions) -> Self {
        let mut jit_context = JitContext::new();

        // Configure JIT context based on options
        jit_context.set_optimization_level(options.opt_level);
        jit_context.set_debug_info(options.debug_info);

        if options.enable_hot_spots {
            jit_context.enable_profiling();
        }
        
        JitRuntime {
            ream_runtime: Arc::new(ream_runtime),
            jit_context: Arc::new(RwLock::new(jit_context)),
            options,
        }
    }
    
    /// Execute a bytecode program with JIT compilation
    pub fn execute_program(&self, program: &BytecodeProgram) -> JitResult<crate::bytecode::Value> {
        let mut jit = self.jit_context.write().unwrap();
        jit.execute(program, &[])
    }
    
    /// Execute with arguments
    pub fn execute_with_args(
        &self,
        program: &BytecodeProgram,
        args: &[crate::bytecode::Value],
    ) -> JitResult<crate::bytecode::Value> {
        let mut jit = self.jit_context.write().unwrap();
        jit.execute(program, args)
    }
    
    /// Compile a program and return a function handle
    pub fn compile_function(&self, program: &BytecodeProgram) -> JitResult<Arc<JitFunction>> {
        let mut jit = self.jit_context.write().unwrap();
        jit.compile(program)
    }
    
    /// Spawn a JIT-compiled process in REAM
    pub fn spawn_jit_process(&self, program: &BytecodeProgram) -> JitResult<crate::types::Pid> {
        // Compile the program
        let jit_func = self.compile_function(program)?;
        
        // Create an actor that executes the JIT function
        let actor = JitActor::new(jit_func);
        
        // Spawn in REAM runtime
        self.ream_runtime.spawn(actor)
            .map_err(|e| JitError::Execution(format!("Failed to spawn process: {}", e)))
    }
    
    /// Get JIT statistics
    pub fn jit_stats(&self) -> crate::jit::JitStats {
        self.jit_context.read().unwrap().stats().clone()
    }
    
    /// Get REAM runtime reference
    pub fn ream_runtime(&self) -> &ReamRuntime {
        &self.ream_runtime
    }
    
    /// Clear JIT cache
    pub fn clear_jit_cache(&self) {
        self.jit_context.write().unwrap().clear_cache();
    }
}

/// Actor that executes JIT-compiled code
struct JitActor {
    /// JIT function to execute
    jit_function: Arc<JitFunction>,
    /// Actor PID
    pid: crate::types::Pid,
    /// Execution state
    state: JitActorState,
}

// Safety: JitActor contains Arc<JitFunction> which we've already marked as Send/Sync
unsafe impl Send for JitActor {}
unsafe impl Sync for JitActor {}

#[derive(Debug, Clone)]
enum JitActorState {
    Ready,
    Running,
    Completed(crate::bytecode::Value),
    Error(String),
}

impl JitActor {
    fn new(jit_function: Arc<JitFunction>) -> Self {
        JitActor {
            jit_function,
            pid: crate::types::Pid::new(),
            state: JitActorState::Ready,
        }
    }
}

impl crate::runtime::ReamActor for JitActor {
    fn receive(&mut self, message: crate::types::MessagePayload) -> crate::error::RuntimeResult<()> {
        match message {
            crate::types::MessagePayload::Text(cmd) if cmd == "execute" => {
                self.state = JitActorState::Running;
                
                match self.jit_function.call0() {
                    Ok(result) => {
                        self.state = JitActorState::Completed(result);
                    }
                    Err(e) => {
                        self.state = JitActorState::Error(format!("JIT execution failed: {}", e));
                    }
                }
            }
            crate::types::MessagePayload::Text(cmd) if cmd == "status" => {
                // Return current state (in a real implementation, this would send a response)
                println!("JIT Actor {} state: {:?}", self.pid, self.state);
            }
            _ => {
                // Ignore other messages
            }
        }
        
        Ok(())
    }
    
    fn pid(&self) -> crate::types::Pid {
        self.pid
    }
    
    fn restart(&mut self) -> crate::error::RuntimeResult<()> {
        self.state = JitActorState::Ready;
        Ok(())
    }
    
    fn is_alive(&self) -> bool {
        !matches!(self.state, JitActorState::Error(_))
    }
}

/// JIT runtime builder
pub struct JitRuntimeBuilder {
    options: JitOptions,
    ream_config: Option<crate::types::ReamConfig>,
}

impl JitRuntimeBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        JitRuntimeBuilder {
            options: JitOptions::default(),
            ream_config: None,
        }
    }
    
    /// Set JIT options
    pub fn jit_options(mut self, options: JitOptions) -> Self {
        self.options = options;
        self
    }
    
    /// Set REAM configuration
    pub fn ream_config(mut self, config: crate::types::ReamConfig) -> Self {
        self.ream_config = Some(config);
        self
    }
    
    /// Set optimization level
    pub fn opt_level(mut self, level: u8) -> Self {
        self.options.opt_level = level.min(3);
        self
    }
    
    /// Enable/disable hot spot detection
    pub fn hot_spots(mut self, enable: bool) -> Self {
        self.options.enable_hot_spots = enable;
        self
    }
    
    /// Build the JIT runtime
    pub fn build(self) -> JitRuntime {
        let ream_runtime = if let Some(config) = self.ream_config {
            ReamRuntime::with_config(config)
        } else {
            ReamRuntime::new().expect("Failed to create ReamRuntime")
        };
        
        JitRuntime::with_options(ream_runtime, self.options)
    }
}

impl Default for JitRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-language JIT runtime
pub struct MultiLangJitRuntime {
    /// Base JIT runtime
    jit_runtime: JitRuntime,
    /// Language compilers
    compilers: HashMap<String, Box<dyn crate::bytecode::LanguageCompiler<AST = String>>>,
}

impl MultiLangJitRuntime {
    /// Create a new multi-language JIT runtime
    pub fn new(jit_runtime: JitRuntime) -> Self {
        MultiLangJitRuntime {
            jit_runtime,
            compilers: HashMap::new(),
        }
    }
    
    /// Register a language compiler
    pub fn register_language<C>(&mut self, language: String, compiler: C)
    where
        C: crate::bytecode::LanguageCompiler<AST = String> + 'static,
    {
        self.compilers.insert(language, Box::new(compiler));
    }
    
    /// Execute source code in a specific language
    pub fn execute_source(
        &self,
        language: &str,
        source: &str,
    ) -> JitResult<crate::bytecode::Value> {
        // Get compiler
        let compiler = self.compilers.get(language)
            .ok_or_else(|| JitError::Execution(format!("Unknown language: {}", language)))?;
        
        // Compile to bytecode
        let program = compiler.compile_to_bytecode(source.to_string())
            .map_err(|e| JitError::Execution(format!("Compilation failed: {}", e)))?;
        
        // Execute with JIT
        self.jit_runtime.execute_program(&program)
    }
    
    /// Spawn a process from source code
    pub fn spawn_source_process(
        &self,
        language: &str,
        source: &str,
    ) -> JitResult<crate::types::Pid> {
        let compiler = self.compilers.get(language)
            .ok_or_else(|| JitError::Execution(format!("Unknown language: {}", language)))?;
        
        let program = compiler.compile_to_bytecode(source.to_string())
            .map_err(|e| JitError::Execution(format!("Compilation failed: {}", e)))?;
        
        self.jit_runtime.spawn_jit_process(&program)
    }
    
    /// Get supported languages
    pub fn supported_languages(&self) -> Vec<String> {
        self.compilers.keys().cloned().collect()
    }
    
    /// Get underlying JIT runtime
    pub fn jit_runtime(&self) -> &JitRuntime {
        &self.jit_runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_jit_runtime_creation() {
        let ream_runtime = ReamRuntime::new();
        let jit_runtime = JitRuntime::new(ream_runtime.expect("Failed to create ReamRuntime"));
        
        let stats = jit_runtime.jit_stats();
        assert_eq!(stats.functions_compiled, 0);
    }
    
    #[test]
    fn test_jit_runtime_builder() {
        let runtime = JitRuntimeBuilder::new()
            .opt_level(3)
            .hot_spots(true)
            .build();
        
        assert_eq!(runtime.options.opt_level, 3);
        assert!(runtime.options.enable_hot_spots);
    }
    
    #[test]
    fn test_multi_lang_runtime() {
        let ream_runtime = ReamRuntime::new();
        let jit_runtime = JitRuntime::new(ream_runtime.expect("Failed to create ReamRuntime"));
        let mut multi_runtime = MultiLangJitRuntime::new(jit_runtime);
        
        // Register a simple compiler
        multi_runtime.register_language(
            "simple".to_string(),
            crate::bytecode::registry::SimpleCompiler::new("simple".to_string())
        );
        
        let languages = multi_runtime.supported_languages();
        assert!(languages.contains(&"simple".to_string()));
    }
}
