//! TLISP runtime integration

use std::rc::Rc;

use crate::tlisp::{TlispInterpreter, Value};

use crate::runtime::ReamRuntime;
use crate::error::{TlispError, TlispResult};

/// TLISP runtime with REAM integration
pub struct TlispRuntime {
    /// TLISP interpreter
    interpreter: TlispInterpreter,
    /// REAM runtime (optional)
    ream_runtime: Option<Rc<ReamRuntime>>,
    /// Runtime configuration
    config: TlispConfig,
}

/// TLISP runtime configuration
#[derive(Debug, Clone)]
pub struct TlispConfig {
    /// Enable REAM integration
    pub enable_ream: bool,
    /// Maximum recursion depth
    pub max_recursion_depth: usize,
    /// Enable debugging
    pub debug: bool,
    /// Standard library modules to load
    pub stdlib_modules: Vec<String>,
}

impl Default for TlispConfig {
    fn default() -> Self {
        TlispConfig {
            enable_ream: true,
            max_recursion_depth: 1000,
            debug: false,
            stdlib_modules: vec![
                "core".to_string(),
                "list".to_string(),
                "math".to_string(),
                "string".to_string(),
            ],
        }
    }
}

impl TlispRuntime {
    /// Create a new TLISP runtime
    pub fn new() -> Self {
        Self::with_config(TlispConfig::default())
    }
    
    /// Create TLISP runtime with configuration
    pub fn with_config(config: TlispConfig) -> Self {
        let mut runtime = TlispRuntime {
            interpreter: TlispInterpreter::new(),
            ream_runtime: None,
            config,
        };
        
        // Load standard library modules
        runtime.load_stdlib();

        // Load custom modules (HTTP, JSON, async-utils)
        runtime.load_custom_modules();

        runtime
    }
    
    /// Create TLISP runtime with REAM integration
    pub fn with_ream(ream_runtime: ReamRuntime) -> Self {
        let mut config = TlispConfig::default();
        config.enable_ream = true;
        
        let mut runtime = Self::with_config(config);
        runtime.ream_runtime = Some(Rc::new(ream_runtime));
        
        // Add REAM-specific functions
        runtime.add_ream_functions();
        
        runtime
    }
    
    /// Evaluate TLISP code
    pub fn eval(&mut self, source: &str) -> TlispResult<Value> {
        if self.config.debug {
            println!("Evaluating: {}", source);
        }
        
        let result = self.interpreter.eval(source)?;
        
        if self.config.debug {
            println!("Result: {}", result);
        }
        
        Ok(result)
    }
    
    /// Evaluate multiple expressions
    pub fn eval_multiple(&mut self, sources: &[&str]) -> TlispResult<Vec<Value>> {
        sources.iter().map(|source| self.eval(source)).collect()
    }
    
    /// Load and evaluate a file
    pub fn load_file(&mut self, path: &str) -> TlispResult<Value> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TlispError::Runtime(format!("Failed to read file {}: {}", path, e)))?;
        
        self.eval(&content)
    }
    
    /// Define a variable in the global environment
    pub fn define(&mut self, name: &str, value: Value) {
        self.interpreter.define(name.to_string(), value);
    }
    
    /// Get a variable from the global environment
    pub fn get(&self, name: &str) -> Option<Value> {
        self.interpreter.get(name)
    }
    
    /// Start a REPL (Read-Eval-Print Loop)
    pub fn repl(&mut self) -> TlispResult<()> {
        println!("TLISP REPL - Type 'exit' to quit");
        
        loop {
            print!("tlisp> ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)
                .map_err(|e| TlispError::Runtime(format!("Input error: {}", e)))?;
            
            let input = input.trim();
            
            if input == "exit" || input == "quit" {
                break;
            }
            
            if input.is_empty() {
                continue;
            }
            
            match self.eval(input) {
                Ok(value) => println!("{}", value),
                Err(e) => println!("Error: {}", e),
            }
        }
        
        Ok(())
    }
    
    /// Get runtime statistics
    pub fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            ream_enabled: self.ream_runtime.is_some(),
            debug_enabled: self.config.debug,
            max_recursion_depth: self.config.max_recursion_depth,
            stdlib_modules_loaded: self.config.stdlib_modules.len(),
        }
    }
    
    /// Load standard library modules
    fn load_stdlib(&mut self) {
        for module in &self.config.stdlib_modules.clone() {
            match module.as_str() {
                "core" => self.load_core_module(),
                "list" => self.load_list_module(),
                "math" => self.load_math_module(),
                "string" => self.load_string_module(),
                _ => {
                    if self.config.debug {
                        println!("Unknown stdlib module: {}", module);
                    }
                }
            }
        }
    }
    
    /// Load core module
    fn load_core_module(&mut self) {
        // Core functions are already loaded by default
        self.define("version", Value::String("0.1.0".to_string()));
        self.define("pi", Value::Float(std::f64::consts::PI));
        self.define("e", Value::Float(std::f64::consts::E));
    }
    
    /// Load list module
    fn load_list_module(&mut self) {
        // Additional list functions
        self.define("empty?", Value::Builtin("empty?".to_string()));
        self.define("first", Value::Builtin("car".to_string())); // Alias
        self.define("rest", Value::Builtin("cdr".to_string())); // Alias
        self.define("append", Value::Builtin("append".to_string()));
        self.define("reverse", Value::Builtin("reverse".to_string()));
    }
    
    /// Load math module
    fn load_math_module(&mut self) {
        self.define("abs", Value::Builtin("abs".to_string()));
        self.define("sqrt", Value::Builtin("sqrt".to_string()));
        self.define("sin", Value::Builtin("sin".to_string()));
        self.define("cos", Value::Builtin("cos".to_string()));
        self.define("tan", Value::Builtin("tan".to_string()));
        self.define("floor", Value::Builtin("floor".to_string()));
        self.define("ceil", Value::Builtin("ceil".to_string()));
        self.define("round", Value::Builtin("round".to_string()));
    }
    
    /// Load string module
    fn load_string_module(&mut self) {
        self.define("string-length", Value::Builtin("string-length".to_string()));
        self.define("string-append", Value::Builtin("string-append".to_string()));
        self.define("substring", Value::Builtin("substring".to_string()));
        self.define("string-upcase", Value::Builtin("string-upcase".to_string()));
        self.define("string-downcase", Value::Builtin("string-downcase".to_string()));
    }

    /// Load custom modules (HTTP, JSON, async-utils, actors)
    fn load_custom_modules(&mut self) {
        // Module import function
        self.define("import", Value::Builtin("import".to_string()));

        // HTTP Server module
        self.define("http-server:start", Value::Builtin("http-server:start".to_string()));
        self.define("http-server:stop", Value::Builtin("http-server:stop".to_string()));
        self.define("http-server:get", Value::Builtin("http-server:get".to_string()));
        self.define("http-server:post", Value::Builtin("http-server:post".to_string()));
        self.define("http-server:put", Value::Builtin("http-server:put".to_string()));
        self.define("http-server:delete", Value::Builtin("http-server:delete".to_string()));
        self.define("http-server:send-response", Value::Builtin("http-server:send-response".to_string()));

        // JSON module
        self.define("json:parse", Value::Builtin("json:parse".to_string()));
        self.define("json:stringify", Value::Builtin("json:stringify".to_string()));
        self.define("json:get", Value::Builtin("json:get".to_string()));
        self.define("json:set!", Value::Builtin("json:set!".to_string()));
        self.define("json:object", Value::Builtin("json:object".to_string()));

        // Async utilities module
        self.define("async-utils:now", Value::Builtin("async-utils:now".to_string()));
        self.define("async-utils:timestamp-ms", Value::Builtin("async-utils:timestamp-ms".to_string()));
        self.define("async-utils:format-time", Value::Builtin("async-utils:format-time".to_string()));
        self.define("async-utils:sleep", Value::Builtin("async-utils:sleep".to_string()));
        self.define("async-utils:spawn-task", Value::Builtin("async-utils:spawn-task".to_string()));
        self.define("async-utils:timestamp-iso", Value::Builtin("async-utils:timestamp-iso".to_string()));

        // Ream ORM module
        self.define("ream-orm:connect", Value::Builtin("ream-orm:connect".to_string()));
        self.define("ream-orm:disconnect", Value::Builtin("ream-orm:disconnect".to_string()));
        self.define("ream-orm:execute", Value::Builtin("ream-orm:execute".to_string()));
        self.define("ream-orm:execute-query", Value::Builtin("ream-orm:execute-query".to_string()));
        self.define("ream-orm:execute-query-single", Value::Builtin("ream-orm:execute-query-single".to_string()));
        self.define("ream-orm:execute-mutation", Value::Builtin("ream-orm:execute-mutation".to_string()));
        self.define("ream-orm:execute-transaction", Value::Builtin("ream-orm:execute-transaction".to_string()));
        self.define("ream-orm:create-query-builder", Value::Builtin("ream-orm:create-query-builder".to_string()));
        self.define("ream-orm:select", Value::Builtin("ream-orm:select".to_string()));
        self.define("ream-orm:where", Value::Builtin("ream-orm:where".to_string()));
        self.define("ream-orm:limit", Value::Builtin("ream-orm:limit".to_string()));
        self.define("ream-orm:order-by", Value::Builtin("ream-orm:order-by".to_string()));
        self.define("ream-orm:build-query", Value::Builtin("ream-orm:build-query".to_string()));
        self.define("ream-orm:create-mutation-builder", Value::Builtin("ream-orm:create-mutation-builder".to_string()));
        self.define("ream-orm:insert", Value::Builtin("ream-orm:insert".to_string()));
        self.define("ream-orm:update", Value::Builtin("ream-orm:update".to_string()));
        self.define("ream-orm:delete", Value::Builtin("ream-orm:delete".to_string()));
        self.define("ream-orm:returning", Value::Builtin("ream-orm:returning".to_string()));
        self.define("ream-orm:build-mutation", Value::Builtin("ream-orm:build-mutation".to_string()));
        self.define("ream-orm:get-schema-info", Value::Builtin("ream-orm:get-schema-info".to_string()));

        // Ream GraphQL module
        self.define("ream-graphql:create-context", Value::Builtin("ream-graphql:create-context".to_string()));
        self.define("ream-graphql:parse-query", Value::Builtin("ream-graphql:parse-query".to_string()));
        self.define("ream-graphql:parse-mutation", Value::Builtin("ream-graphql:parse-mutation".to_string()));
        self.define("ream-graphql:compile-query", Value::Builtin("ream-graphql:compile-query".to_string()));
        self.define("ream-graphql:compile-mutation", Value::Builtin("ream-graphql:compile-mutation".to_string()));

        // Actor system functions
        self.define("spawn", Value::Builtin("spawn".to_string()));
        self.define("send", Value::Builtin("send".to_string()));
        self.define("receive", Value::Builtin("receive".to_string()));
        self.define("self", Value::Builtin("self".to_string()));

        // Additional built-in functions that were missing
        self.define("cadr", Value::Builtin("cadr".to_string()));
        self.define("caddr", Value::Builtin("caddr".to_string()));
        self.define("cadddr", Value::Builtin("cadddr".to_string()));
        self.define("set!", Value::Builtin("set!".to_string()));
        self.define("string-split", Value::Builtin("string-split".to_string()));
        self.define("string-starts-with", Value::Builtin("string-starts-with".to_string()));
        self.define("string->number", Value::Builtin("string->number".to_string()));
        self.define("number->string", Value::Builtin("number->string".to_string()));
        self.define("string=?", Value::Builtin("string=?".to_string()));
        self.define("list-ref", Value::Builtin("list-ref".to_string()));
        self.define("println", Value::Builtin("println".to_string()));
    }
    
    /// Add REAM-specific functions
    fn add_ream_functions(&mut self) {
        if self.ream_runtime.is_some() {
            // Enhanced REAM functions
            self.define("spawn-actor", Value::Builtin("spawn-actor".to_string()));
            self.define("send-message", Value::Builtin("send-message".to_string()));
            self.define("receive-message", Value::Builtin("receive-message".to_string()));
            self.define("link-process", Value::Builtin("link-process".to_string()));
            self.define("monitor-process", Value::Builtin("monitor-process".to_string()));
            self.define("process-info", Value::Builtin("process-info".to_string()));
            self.define("list-processes", Value::Builtin("list-processes".to_string()));
        }
    }
}

impl Default for TlispRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub ream_enabled: bool,
    pub debug_enabled: bool,
    pub max_recursion_depth: usize,
    pub stdlib_modules_loaded: usize,
}

/// TLISP runtime builder
pub struct TlispRuntimeBuilder {
    config: TlispConfig,
    ream_runtime: Option<ReamRuntime>,
}

impl TlispRuntimeBuilder {
    /// Create a new runtime builder
    pub fn new() -> Self {
        TlispRuntimeBuilder {
            config: TlispConfig::default(),
            ream_runtime: None,
        }
    }
    
    /// Enable/disable REAM integration
    pub fn ream(mut self, enable: bool) -> Self {
        self.config.enable_ream = enable;
        self
    }
    
    /// Set REAM runtime
    pub fn with_ream_runtime(mut self, runtime: ReamRuntime) -> Self {
        self.ream_runtime = Some(runtime);
        self.config.enable_ream = true;
        self
    }
    
    /// Enable/disable debugging
    pub fn debug(mut self, enable: bool) -> Self {
        self.config.debug = enable;
        self
    }
    
    /// Set maximum recursion depth
    pub fn max_recursion_depth(mut self, depth: usize) -> Self {
        self.config.max_recursion_depth = depth;
        self
    }
    
    /// Add standard library module
    pub fn stdlib_module(mut self, module: String) -> Self {
        if !self.config.stdlib_modules.contains(&module) {
            self.config.stdlib_modules.push(module);
        }
        self
    }
    
    /// Set standard library modules
    pub fn stdlib_modules(mut self, modules: Vec<String>) -> Self {
        self.config.stdlib_modules = modules;
        self
    }
    
    /// Build the runtime
    pub fn build(self) -> TlispRuntime {
        if let Some(ream_runtime) = self.ream_runtime {
            TlispRuntime::with_ream(ream_runtime)
        } else {
            TlispRuntime::with_config(self.config)
        }
    }
}

impl Default for TlispRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// TLISP script runner
pub struct TlispScript {
    runtime: TlispRuntime,
    script_path: String,
}

impl TlispScript {
    /// Create a new script runner
    pub fn new(script_path: String) -> Self {
        TlispScript {
            runtime: TlispRuntime::new(),
            script_path,
        }
    }
    
    /// Create script runner with custom runtime
    pub fn with_runtime(runtime: TlispRuntime, script_path: String) -> Self {
        TlispScript {
            runtime,
            script_path,
        }
    }
    
    /// Run the script
    pub fn run(&mut self) -> TlispResult<Value> {
        self.runtime.load_file(&self.script_path)
    }
    
    /// Run with command line arguments
    pub fn run_with_args(&mut self, args: Vec<String>) -> TlispResult<Value> {
        // Define command line arguments
        let args_list = Value::List(
            args.into_iter().map(Value::String).collect()
        );
        self.runtime.define("*args*", args_list);
        
        self.run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = TlispRuntime::new();
        let stats = runtime.stats();
        
        assert!(!stats.ream_enabled);
        assert!(!stats.debug_enabled);
        assert_eq!(stats.max_recursion_depth, 1000);
    }
    
    #[test]
    fn test_runtime_builder() {
        let runtime = TlispRuntimeBuilder::new()
            .debug(true)
            .max_recursion_depth(500)
            .stdlib_module("extra".to_string())
            .build();
        
        let stats = runtime.stats();
        assert!(stats.debug_enabled);
        assert_eq!(stats.max_recursion_depth, 500);
    }
    
    #[test]
    fn test_basic_evaluation() {
        let mut runtime = TlispRuntime::new();
        
        let result = runtime.eval("42").unwrap();
        assert_eq!(result, Value::Int(42));
        
        let result = runtime.eval("(+ 1 2)").unwrap();
        assert_eq!(result, Value::Int(3));
    }
    
    #[test]
    fn test_variable_definition() {
        let mut runtime = TlispRuntime::new();
        
        runtime.define("x", Value::Int(42));
        assert_eq!(runtime.get("x"), Some(Value::Int(42)));
        
        let result = runtime.eval("x").unwrap();
        assert_eq!(result, Value::Int(42));
    }
    
    #[test]
    fn test_stdlib_loading() {
        let runtime = TlispRuntime::new();
        
        // Should have core functions
        assert!(runtime.get("+").is_some());
        assert!(runtime.get("list").is_some());
        assert!(runtime.get("version").is_some());
    }
}
