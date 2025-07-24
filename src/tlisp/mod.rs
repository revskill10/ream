//! TLISP: Typed Lisp for REAM
//! 
//! A mathematically-grounded Lisp implementation with Hindley-Milner type inference
//! and seamless REAM integration for actor-based programming.

pub mod parser;
pub mod evaluator;
pub mod types;
pub mod type_evaluator;
pub mod constraint_solver;
pub mod dependent_type_checker;
pub mod dependent_type_integration_test;
pub mod environment;
pub mod macros;
pub mod runtime;
pub mod ream_bridge;
// Enable core modules needed for package management
pub mod module_system;
pub mod package_manager;
pub mod package_config;
pub mod package_registry;
pub mod cross_language_bridge;
pub mod rust_integration;
pub mod rust_crate_integration;
pub mod rust_modules;
#[cfg(test)]
pub mod rust_modules_tests;
#[cfg(test)]
pub mod integration_tests;
#[cfg(test)]
pub mod example_tests;
pub mod standard_library;
pub mod pattern_matching;
pub mod serverless;

// Test modules
#[cfg(test)]
pub mod tests;

// TODO: Enable these modules once their dependencies are implemented
// pub mod actor_primitives;
// pub mod session_types;
// pub mod stm_integration;
// pub mod capability_system;
// pub mod effect_system;

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use serde::{Deserialize, Serialize};

use crate::bytecode::{LanguageCompiler, BytecodeCompiler, BytecodeProgram, Bytecode, TypeInfo};
use crate::bytecode::Value as BytecodeValue;
use crate::error::BytecodeResult;
use crate::types::EffectGrade;
use crate::error::TlispResult;

pub use parser::{Parser, Token, Lexer};
pub use evaluator::{Evaluator, EvaluationContext};
pub use types::{Type, TypeChecker, Substitution};
pub use dependent_type_checker::DependentTypeChecker;
pub use environment::Environment;
pub use macros::{MacroRegistry, Macro};
pub use runtime::TlispRuntime;
pub use ream_bridge::TlispReamBridge;
pub use module_system::{Module, ModuleRegistry, ModuleLoader, ModuleLanguage};
pub use package_manager::{PackageManager, PackageMetadata, InstallOptions, VersionRequirement};
pub use package_config::{ProjectConfig, ProjectConfigManager, PackageInfo, DependencySpec};
pub use package_registry::{PackageRegistry, RegistryManager, PublishRequest, SearchQuery, SearchResults};
pub use cross_language_bridge::{CrossLanguageBridge, LanguageBridge, BridgeCallResult, BridgeStats};
pub use rust_integration::{RustIntegration, RustFunction, RustModule};
pub use rust_crate_integration::{RustCrateIntegration, RustCrateMetadata, CrateBuildOptions};
pub use standard_library::StandardLibrary;
pub use pattern_matching::{PatternMatcher, Pattern};

/// TLISP expression as initial algebra
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr<T> {
    /// Symbol with type annotation
    Symbol(String, T),
    /// Number literal
    Number(i64, T),
    /// Float literal
    Float(f64, T),
    /// Boolean literal
    Bool(bool, T),
    /// String literal
    String(String, T),
    /// List expression
    List(Vec<Expr<T>>, T),
    /// Lambda expression
    Lambda(Vec<String>, Box<Expr<T>>, T),
    /// Function application
    Application(Box<Expr<T>>, Vec<Expr<T>>, T),
    /// Let binding
    Let(Vec<(String, Expr<T>)>, Box<Expr<T>>, T),
    /// If expression
    If(Box<Expr<T>>, Box<Expr<T>>, Box<Expr<T>>, T),
    /// Quote expression
    Quote(Box<Expr<T>>, T),
    /// Define expression
    Define(String, Box<Expr<T>>, T),
    /// Set expression (assignment)
    Set(String, Box<Expr<T>>, T),
}

impl<T> Expr<T> {
    /// Get the type annotation
    pub fn get_type(&self) -> &T {
        match self {
            Expr::Symbol(_, t) => t,
            Expr::Number(_, t) => t,
            Expr::Float(_, t) => t,
            Expr::Bool(_, t) => t,
            Expr::String(_, t) => t,
            Expr::List(_, t) => t,
            Expr::Lambda(_, _, t) => t,
            Expr::Application(_, _, t) => t,
            Expr::Let(_, _, t) => t,
            Expr::If(_, _, _, t) => t,
            Expr::Quote(_, t) => t,
            Expr::Define(_, _, t) => t,
            Expr::Set(_, _, t) => t,
        }
    }
    
    /// Set the type annotation
    pub fn set_type(self, new_type: T) -> Expr<T> {
        match self {
            Expr::Symbol(s, _) => Expr::Symbol(s, new_type),
            Expr::Number(n, _) => Expr::Number(n, new_type),
            Expr::Float(f, _) => Expr::Float(f, new_type),
            Expr::Bool(b, _) => Expr::Bool(b, new_type),
            Expr::String(s, _) => Expr::String(s, new_type),
            Expr::List(l, _) => Expr::List(l, new_type),
            Expr::Lambda(p, b, _) => Expr::Lambda(p, b, new_type),
            Expr::Application(f, a, _) => Expr::Application(f, a, new_type),
            Expr::Let(b, e, _) => Expr::Let(b, e, new_type),
            Expr::If(c, t, e, _) => Expr::If(c, t, e, new_type),
            Expr::Quote(e, _) => Expr::Quote(e, new_type),
            Expr::Define(name, value, _) => Expr::Define(name, value, new_type),
            Expr::Set(name, value, _) => Expr::Set(name, value, new_type),
        }
    }
    
    /// Map over the type annotation
    pub fn map_type<U, F>(self, f: F) -> Expr<U>
    where
        F: Fn(T) -> U + Clone,
    {
        match self {
            Expr::Symbol(s, t) => Expr::Symbol(s, f(t)),
            Expr::Number(n, t) => Expr::Number(n, f(t)),
            Expr::Float(fl, t) => Expr::Float(fl, f(t)),
            Expr::Bool(b, t) => Expr::Bool(b, f(t)),
            Expr::String(s, t) => Expr::String(s, f(t)),
            Expr::List(l, t) => {
                let new_list = l.into_iter().map(|e| e.map_type(f.clone())).collect();
                Expr::List(new_list, f(t))
            }
            Expr::Lambda(p, b, t) => {
                let new_body = Box::new(b.map_type(f.clone()));
                Expr::Lambda(p, new_body, f(t))
            }
            Expr::Application(func, args, t) => {
                let new_func = Box::new(func.map_type(f.clone()));
                let new_args = args.into_iter().map(|e| e.map_type(f.clone())).collect();
                Expr::Application(new_func, new_args, f(t))
            }
            Expr::Let(bindings, body, t) => {
                let new_bindings = bindings.into_iter()
                    .map(|(name, expr)| (name, expr.map_type(f.clone())))
                    .collect();
                let new_body = Box::new(body.map_type(f.clone()));
                Expr::Let(new_bindings, new_body, f(t))
            }
            Expr::If(cond, then_expr, else_expr, t) => {
                let new_cond = Box::new(cond.map_type(f.clone()));
                let new_then = Box::new(then_expr.map_type(f.clone()));
                let new_else = Box::new(else_expr.map_type(f.clone()));
                Expr::If(new_cond, new_then, new_else, f(t))
            }
            Expr::Quote(e, t) => {
                let new_expr = Box::new(e.map_type(f.clone()));
                Expr::Quote(new_expr, f(t))
            }
            Expr::Define(name, value, t) => {
                let new_value = Box::new(value.map_type(f.clone()));
                Expr::Define(name, new_value, f(t))
            }
            Expr::Set(name, value, t) => {
                let new_value = Box::new(value.map_type(f.clone()));
                Expr::Set(name, new_value, f(t))
            }
        }
    }
    
    /// Check if expression is a literal
    pub fn is_literal(&self) -> bool {
        matches!(self, 
            Expr::Number(_, _) | 
            Expr::Float(_, _) | 
            Expr::Bool(_, _) | 
            Expr::String(_, _)
        )
    }
    
    /// Check if expression is a symbol
    pub fn is_symbol(&self) -> bool {
        matches!(self, Expr::Symbol(_, _))
    }
    
    /// Get symbol name if this is a symbol
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Expr::Symbol(name, _) => Some(name),
            _ => None,
        }
    }
}

/// TLISP value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
    /// Unit value (void/empty)
    Unit,
    /// Symbol value
    Symbol(String),
    /// List value
    List(Vec<Value>),
    /// Function value
    Function(Function),
    /// Built-in function
    Builtin(String),
    /// Process ID
    Pid(crate::types::Pid),
    /// STM Variable
    StmVar(StmVariable),
    /// Null value
    Null,
}

/// Function representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    /// Parameter names
    pub params: Vec<String>,
    /// Function body
    pub body: Expr<Type>,
    /// Closure environment
    pub env: HashMap<String, Value>,
}

/// STM Variable representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StmVariable {
    /// Variable name
    pub name: String,
    /// Variable type
    pub var_type: Type,
    /// Variable ID
    pub id: u64,
}

impl StmVariable {
    /// Create a new STM variable
    pub fn new(name: String, var_type: Type, id: u64) -> Self {
        StmVariable { name, var_type, id }
    }

    /// Get variable name
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Value {
    /// Get the type of this value
    pub fn type_of(&self) -> Type {
        match self {
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::Bool(_) => Type::Bool,
            Value::String(_) => Type::String,
            Value::Symbol(_) => Type::Symbol,
            Value::List(items) => {
                if items.is_empty() {
                    Type::List(Box::new(Type::TypeVar("a".to_string())))
                } else {
                    Type::List(Box::new(items[0].type_of()))
                }
            }
            Value::Function(func) => {
                let param_types = vec![Type::TypeVar("a".to_string()); func.params.len()];
                Type::Function(param_types, Box::new(Type::TypeVar("b".to_string())))
            }
            Value::Builtin(_) => Type::Function(vec![], Box::new(Type::TypeVar("a".to_string()))),
            Value::Pid(_) => Type::Pid,
            Value::StmVar(var) => var.var_type.clone(),
            Value::Unit => Type::Unit,
            Value::Null => Type::Unit,
        }
    }
    
    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Null => false,
            _ => true,
        }
    }
    
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::String(s) => format!("\"{}\"", s),
            Value::Symbol(s) => s.clone(),
            Value::List(items) => {
                let items_str: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                format!("({})", items_str.join(" "))
            }
            Value::Function(func) => {
                format!("(lambda ({}) ...)", func.params.join(" "))
            }
            Value::Builtin(name) => format!("#<builtin:{}>", name),
            Value::Pid(pid) => format!("#<pid:{}>", pid.raw()),
            Value::StmVar(var) => format!("#<stm-var:{}>", var.name()),
            Value::Unit => "()".to_string(),
            Value::Null => "null".to_string(),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// TLISP interpreter state
pub struct TlispInterpreter {
    /// Lexer
    lexer: Lexer,
    /// Parser
    parser: Parser,
    /// Evaluator
    evaluator: Evaluator,
    /// Type checker
    type_checker: DependentTypeChecker,
    /// Macro registry
    macro_registry: MacroRegistry,
    /// Global environment
    global_env: Rc<RefCell<Environment>>,
    /// Debug mode
    debug: bool,
}

impl TlispInterpreter {
    /// Create a new TLISP interpreter
    pub fn new() -> Self {
        let global_env = Rc::new(RefCell::new(Environment::new()));
        
        // Add built-in functions
        Self::add_builtins(&global_env);
        
        TlispInterpreter {
            lexer: Lexer::new(""), // Placeholder, will be replaced per parse
            parser: Parser::new(),
            evaluator: Evaluator::new(Rc::clone(&global_env)),
            type_checker: DependentTypeChecker::new(),
            macro_registry: MacroRegistry::new(),
            global_env,
            debug: false,
        }
    }
    
    /// Evaluate a string of TLISP code
    pub fn eval(&mut self, source: &str) -> TlispResult<Value> {
        if self.debug {
            println!("TLISP DEBUG: Starting evaluation of source: {}", source);
        }

        // Parse
        let tokens = self.parser.tokenize(source)?;

        if self.debug {
            println!("TLISP DEBUG: Tokenized {} tokens: {:?}", tokens.len(), tokens);
        }

        // Try to parse multiple expressions first
        match self.parser.parse_multiple(&tokens) {
            Ok(expressions) => {
                if self.debug {
                    println!("TLISP DEBUG: Parsed {} expressions", expressions.len());
                }

                // Multiple expressions - expand macros and type check all, then evaluate with shared context
                let mut expanded_expressions = Vec::new();

                for (i, expr) in expressions.iter().enumerate() {
                    if self.debug {
                        println!("TLISP DEBUG: Processing expression {}: {:?}", i, expr);
                    }

                    // Expand macros
                    let expanded = self.macro_registry.expand(&expr)?;

                    if self.debug {
                        println!("TLISP DEBUG: Macro expanded: {:?}", expanded);
                    }

                    // Type check (for now, skip and use untyped expression)
                    // TODO: Integrate DependentTypeChecker properly
                    // let _inferred_type = self.type_checker.infer_type(&expanded)?;

                    if self.debug {
                        println!("TLISP DEBUG: Type checking skipped");
                    }

                    expanded_expressions.push(expanded);
                }

                // Evaluate all expressions with shared context
                let result = self.evaluator.eval_multiple_untyped(&expanded_expressions)?;

                if self.debug {
                    println!("TLISP DEBUG: Final result: {:?}", result);
                }

                Ok(result)
            }
            Err(parse_err) => {
                if self.debug {
                    println!("TLISP DEBUG: Multiple expression parsing failed: {:?}, trying single expression", parse_err);
                }
                // Always print parsing errors for debugging
                println!("PARSING ERROR: Multiple expression parsing failed: {}", parse_err);

                // Fall back to single expression parsing
                let expr = self.parser.parse(&tokens)?;

                if self.debug {
                    println!("TLISP DEBUG: Parsed single expression: {:?}", expr);
                }

                // Expand macros
                let expanded = self.macro_registry.expand(&expr)?;

                if self.debug {
                    println!("TLISP DEBUG: Macro expanded: {:?}", expanded);
                }

                // Type check
                // let _inferred_type = self.type_checker.infer_type(&expanded)?;

                if self.debug {
                    println!("TLISP DEBUG: Type checking skipped");
                }

                // Evaluate
                let result = self.evaluator.eval_untyped(&expanded)?;

                if self.debug {
                    println!("TLISP DEBUG: Evaluated to: {:?}", result);
                }

                Ok(result)
            }
        }
    }
    
    /// Define a variable in the global environment
    pub fn define(&mut self, name: String, value: Value) {
        // Add to runtime environment
        self.global_env.borrow_mut().define(name.clone(), value.clone());

        // Add to type checker environment
        let value_type = match &value {
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::String(_) => Type::String,
            Value::Bool(_) => Type::Bool,
            Value::Symbol(_) => Type::Symbol,
            Value::List(_) => Type::List(Box::new(Type::String)), // Assume string list for *args*
            Value::Null => Type::Unit,
            Value::Unit => Type::Unit,
            Value::Function(_) => Type::Function(vec![], Box::new(Type::Unit)), // Generic function type
            Value::Builtin(_) => Type::Function(vec![], Box::new(Type::Unit)), // Generic function type
            Value::Pid(_) => Type::Unit, // No specific type for PIDs yet
            Value::StmVar(var) => var.var_type.clone(),
        };
        self.type_checker.define_var(name.to_string(), value_type);
    }
    
    /// Get a variable from the global environment
    pub fn get(&self, name: &str) -> Option<Value> {
        self.global_env.borrow().get(name)
    }

    /// Set debug mode
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
    
    /// Add built-in functions to the environment
    fn add_builtins(env: &Rc<RefCell<Environment>>) {
        let mut env = env.borrow_mut();
        
        // Arithmetic
        env.define("+".to_string(), Value::Builtin("add".to_string()));
        env.define("-".to_string(), Value::Builtin("sub".to_string()));
        env.define("*".to_string(), Value::Builtin("mul".to_string()));
        env.define("/".to_string(), Value::Builtin("div".to_string()));
        
        // Comparison
        env.define("=".to_string(), Value::Builtin("eq".to_string()));
        env.define("eq".to_string(), Value::Builtin("eq".to_string())); // Add eq as direct function
        env.define("<".to_string(), Value::Builtin("lt".to_string()));
        env.define("<=".to_string(), Value::Builtin("le".to_string()));
        env.define(">".to_string(), Value::Builtin("gt".to_string()));
        env.define(">=".to_string(), Value::Builtin("ge".to_string()));
        
        // List operations
        env.define("list".to_string(), Value::Builtin("list".to_string()));
        env.define("car".to_string(), Value::Builtin("car".to_string()));
        env.define("cdr".to_string(), Value::Builtin("cdr".to_string()));
        env.define("head".to_string(), Value::Builtin("head".to_string()));
        env.define("tail".to_string(), Value::Builtin("tail".to_string()));
        env.define("cons".to_string(), Value::Builtin("cons".to_string()));
        env.define("append".to_string(), Value::Builtin("append".to_string()));
        env.define("length".to_string(), Value::Builtin("length".to_string()));

        // Control flow
        env.define("begin".to_string(), Value::Builtin("begin".to_string()));
        env.define("cond".to_string(), Value::Builtin("cond".to_string()));

        // Boolean operations
        env.define("and".to_string(), Value::Builtin("and".to_string()));
        env.define("or".to_string(), Value::Builtin("or".to_string()));
        env.define("not".to_string(), Value::Builtin("not".to_string()));
        
        // I/O
        env.define("print".to_string(), Value::Builtin("print".to_string()));
        env.define("println".to_string(), Value::Builtin("println".to_string()));
        env.define("newline".to_string(), Value::Builtin("newline".to_string()));

        // String operations
        env.define("string-append".to_string(), Value::Builtin("string-append".to_string()));
        env.define("number->string".to_string(), Value::Builtin("number->string".to_string()));
        env.define("symbol->string".to_string(), Value::Builtin("symbol->string".to_string()));
        env.define("list->string".to_string(), Value::Builtin("list->string".to_string()));
        env.define("pid->string".to_string(), Value::Builtin("pid->string".to_string()));

        // Type predicates
        env.define("null?".to_string(), Value::Builtin("null?".to_string()));
        env.define("number?".to_string(), Value::Builtin("number?".to_string()));
        env.define("string?".to_string(), Value::Builtin("string?".to_string()));
        env.define("symbol?".to_string(), Value::Builtin("symbol?".to_string()));
        env.define("boolean?".to_string(), Value::Builtin("boolean?".to_string()));
        env.define("list?".to_string(), Value::Builtin("list?".to_string()));
        env.define("equal?".to_string(), Value::Builtin("equal?".to_string()));

        // Math operations
        env.define("modulo".to_string(), Value::Builtin("modulo".to_string()));
        env.define("mod".to_string(), Value::Builtin("modulo".to_string())); // Alias for modulo
        env.define("floor".to_string(), Value::Builtin("floor".to_string()));
        env.define("sqrt".to_string(), Value::Builtin("sqrt".to_string()));
        env.define("abs".to_string(), Value::Builtin("abs".to_string()));

        // System functions
        env.define("error".to_string(), Value::Builtin("error".to_string()));
        env.define("current-time".to_string(), Value::Builtin("current-time".to_string()));
        env.define("random".to_string(), Value::Builtin("random".to_string()));

        // REAM integration
        env.define("spawn".to_string(), Value::Builtin("spawn".to_string()));
        env.define("send".to_string(), Value::Builtin("send".to_string()));
        env.define("receive".to_string(), Value::Builtin("receive".to_string()));
        env.define("self".to_string(), Value::Builtin("self".to_string()));

        // P2P distributed system functions
        // Cluster management functions
        env.define("p2p-create-cluster".to_string(), Value::Builtin("p2p-create-cluster".to_string()));
        env.define("p2p-join-cluster".to_string(), Value::Builtin("p2p-join-cluster".to_string()));
        env.define("p2p-leave-cluster".to_string(), Value::Builtin("p2p-leave-cluster".to_string()));
        env.define("p2p-cluster-info".to_string(), Value::Builtin("p2p-cluster-info".to_string()));
        env.define("p2p-cluster-members".to_string(), Value::Builtin("p2p-cluster-members".to_string()));

        // Actor management functions
        env.define("p2p-spawn-actor".to_string(), Value::Builtin("p2p-spawn-actor".to_string()));
        env.define("p2p-migrate-actor".to_string(), Value::Builtin("p2p-migrate-actor".to_string()));
        env.define("p2p-send-remote".to_string(), Value::Builtin("p2p-send-remote".to_string()));
        env.define("p2p-actor-location".to_string(), Value::Builtin("p2p-actor-location".to_string()));

        // Node management functions
        env.define("p2p-node-info".to_string(), Value::Builtin("p2p-node-info".to_string()));
        env.define("p2p-node-health".to_string(), Value::Builtin("p2p-node-health".to_string()));
        env.define("p2p-discover-nodes".to_string(), Value::Builtin("p2p-discover-nodes".to_string()));

        // Consensus functions
        env.define("p2p-propose".to_string(), Value::Builtin("p2p-propose".to_string()));
        env.define("p2p-consensus-state".to_string(), Value::Builtin("p2p-consensus-state".to_string()));

        // Hypervisor functions
        env.define("hypervisor:start".to_string(), Value::Builtin("hypervisor:start".to_string()));
        env.define("hypervisor:stop".to_string(), Value::Builtin("hypervisor:stop".to_string()));
        env.define("hypervisor:register-actor".to_string(), Value::Builtin("hypervisor:register-actor".to_string()));
        env.define("hypervisor:unregister-actor".to_string(), Value::Builtin("hypervisor:unregister-actor".to_string()));
        env.define("hypervisor:get-actor-metrics".to_string(), Value::Builtin("hypervisor:get-actor-metrics".to_string()));
        env.define("hypervisor:get-system-metrics".to_string(), Value::Builtin("hypervisor:get-system-metrics".to_string()));
        env.define("hypervisor:list-actors".to_string(), Value::Builtin("hypervisor:list-actors".to_string()));
        env.define("hypervisor:health-check".to_string(), Value::Builtin("hypervisor:health-check".to_string()));
        env.define("hypervisor:set-alert-threshold".to_string(), Value::Builtin("hypervisor:set-alert-threshold".to_string()));
        env.define("hypervisor:get-alerts".to_string(), Value::Builtin("hypervisor:get-alerts".to_string()));
        env.define("hypervisor:restart-actor".to_string(), Value::Builtin("hypervisor:restart-actor".to_string()));
        env.define("hypervisor:suspend-actor".to_string(), Value::Builtin("hypervisor:suspend-actor".to_string()));
        env.define("hypervisor:resume-actor".to_string(), Value::Builtin("hypervisor:resume-actor".to_string()));
        env.define("hypervisor:kill-actor".to_string(), Value::Builtin("hypervisor:kill-actor".to_string()));
        env.define("hypervisor:get-supervision-tree".to_string(), Value::Builtin("hypervisor:get-supervision-tree".to_string()));
    }

    /// Parse TLISP source code into expressions
    pub fn parse(&mut self, input: &str) -> TlispResult<Expr<()>> {
        // Create a new lexer for this input
        let mut lexer = Lexer::new(input);
        // Tokenize the input
        let tokens = lexer.tokenize()?;
        // Then parse the tokens
        self.parser.parse(&tokens)
    }

    /// Enable JIT compilation
    pub fn enable_jit(&mut self) {
        // Enable JIT in the evaluator
        // This is a placeholder - in practice would configure JIT compilation
        if self.debug {
            println!("TLISP DEBUG: JIT compilation enabled");
        }
    }

    /// Set optimization level
    pub fn set_optimization_level(&mut self, level: u8) {
        // Configure optimization level
        // This is a placeholder - in practice would configure optimization passes
        if self.debug {
            println!("TLISP DEBUG: Optimization level set to {}", level);
        }
    }
}

impl LanguageCompiler for TlispInterpreter {
    type AST = Expr<()>;

    fn compile_to_bytecode(&self, expr: Expr<()>) -> BytecodeResult<BytecodeProgram> {
        // Create a bytecode compiler
        let mut compiler = BytecodeCompiler::new("tlisp_expr".to_string());

        // Compile the expression to bytecode
        self.compile_expr_to_bytecode(&expr, &mut compiler)?;

        // Finish compilation
        compiler.finish()
    }

    fn get_type_info(&self, ast: &Self::AST) -> TypeInfo {
        // Return type info based on the expression
        match ast {
            Expr::Number(_, _) => TypeInfo::Int,
            Expr::Float(_, _) => TypeInfo::Float,
            Expr::Bool(_, _) => TypeInfo::Bool,
            Expr::String(_, _) => TypeInfo::String,
            Expr::List(_, _) => TypeInfo::List(Box::new(TypeInfo::Unknown)),
            _ => TypeInfo::Unknown,
        }
    }
}

impl TlispInterpreter {
    /// Compile a single expression to bytecode
    fn compile_expr_to_bytecode(&self, expr: &Expr<()>, compiler: &mut BytecodeCompiler) -> BytecodeResult<()> {
        match expr {
            Expr::Number(n, _) => {
                // Add constant to program and load it
                let const_id = compiler.add_constant(BytecodeValue::Int(*n));
                compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
            }
            Expr::Symbol(_name, _) => {
                // Load variable - simplified to load global 0
                compiler.emit(Bytecode::LoadGlobal(0, EffectGrade::Read));
            }
            Expr::List(exprs, _) => {
                // Compile each expression in the list
                for expr in exprs {
                    self.compile_expr_to_bytecode(expr, compiler)?;
                }
            }
            _ => {
                // For other expressions, emit a placeholder
                let const_id = compiler.add_constant(BytecodeValue::Null);
                compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
            }
        }
        Ok(())
    }
}

impl Default for TlispInterpreter {
    fn default() -> Self {
        Self::new()
    }
}


