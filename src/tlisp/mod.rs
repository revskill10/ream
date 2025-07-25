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
pub mod enhanced_compiler;
pub mod actor_system;
pub mod security_integration;
pub mod resource_integration;
pub mod production_stdlib;
pub mod production_runtime;

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
use std::sync::{Arc, Mutex};
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
pub use enhanced_compiler::EnhancedTlispCompiler;
pub use actor_system::{TlispActorSystem, TlispActor, ActorSystemConfig, RealTimeConstraints, SecurityLevel, ActorInfo, ActorSystemStats};
pub use security_integration::{TlispSecurityManager, TlispSecurityLevel, TlispSecurityPolicy, TlispMemoryLimits, TlispIOPermissions, TlispNetworkPermissions, TlispAuditEvent, TlispSecurityStats};
pub use resource_integration::{TlispResourceManager, TlispResourceQuotas, TlispSpecificLimits, TlispResourceUsage, TlispResourceStats, TlispResourceConfig, TlispWarningThresholds};
pub use production_stdlib::{ProductionStandardLibrary, GlobalState};
pub use production_runtime::{ProductionTlispRuntime, ProductionRuntimeConfig, RuntimeStats, ExecutionResult, ExecutionMode, ExecutionMetrics};

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
    /// Macro definition
    Macro(String, Vec<String>, Box<Expr<T>>, T),
    /// Type annotation
    TypeAnnotation(Box<Expr<T>>, Box<Expr<T>>, T),
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
            Expr::Macro(_, _, _, t) => t,
            Expr::TypeAnnotation(_, _, t) => t,
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
            Expr::Macro(name, params, body, _) => Expr::Macro(name, params, body, new_type),
            Expr::TypeAnnotation(expr, type_expr, _) => Expr::TypeAnnotation(expr, type_expr, new_type),
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
            Expr::Macro(name, params, body, t) => {
                let new_body = Box::new(body.map_type(f.clone()));
                Expr::Macro(name, params, new_body, f(t))
            }
            Expr::TypeAnnotation(expr, type_expr, t) => {
                let new_expr = Box::new(expr.map_type(f.clone()));
                let new_type_expr = Box::new(type_expr.map_type(f.clone()));
                Expr::TypeAnnotation(new_expr, new_type_expr, f(t))
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
    global_env: Arc<Mutex<Environment>>,
    /// Debug mode
    debug: bool,
}

impl TlispInterpreter {
    /// Create a new TLISP interpreter
    pub fn new() -> Self {
        let global_env = Arc::new(Mutex::new(Environment::new()));

        // Add built-in functions
        Self::add_builtins(&global_env);

        TlispInterpreter {
            lexer: Lexer::new(""), // Placeholder, will be replaced per parse
            parser: Parser::new(),
            evaluator: Evaluator::new(Arc::clone(&global_env)),
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
        self.global_env.lock().unwrap().define(name.clone(), value.clone());

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
        self.global_env.lock().unwrap().get(name)
    }

    /// Set debug mode
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
    
    /// Add built-in functions to the environment (Arc<Mutex> version)
    fn add_builtins(env: &Arc<Mutex<Environment>>) {
        let mut env = env.lock().unwrap();

        // Arithmetic
        env.define("+".to_string(), Value::Builtin("add".to_string()));
        env.define("-".to_string(), Value::Builtin("sub".to_string()));
        env.define("*".to_string(), Value::Builtin("mul".to_string()));
        env.define("/".to_string(), Value::Builtin("div".to_string()));
        env.define("%".to_string(), Value::Builtin("mod".to_string()));

        // Comparison
        env.define("=".to_string(), Value::Builtin("eq".to_string()));
        env.define("<".to_string(), Value::Builtin("lt".to_string()));
        env.define("<=".to_string(), Value::Builtin("le".to_string()));
        env.define(">".to_string(), Value::Builtin("gt".to_string()));
        env.define(">=".to_string(), Value::Builtin("ge".to_string()));

        // List operations
        env.define("list".to_string(), Value::Builtin("list".to_string()));
        env.define("car".to_string(), Value::Builtin("car".to_string()));
        env.define("cdr".to_string(), Value::Builtin("cdr".to_string()));
        env.define("cons".to_string(), Value::Builtin("cons".to_string()));

        // I/O
        env.define("print".to_string(), Value::Builtin("print".to_string()));
        env.define("println".to_string(), Value::Builtin("println".to_string()));

        // Type predicates
        env.define("null?".to_string(), Value::Builtin("null?".to_string()));
        env.define("number?".to_string(), Value::Builtin("number?".to_string()));
        env.define("string?".to_string(), Value::Builtin("string?".to_string()));
        env.define("list?".to_string(), Value::Builtin("list?".to_string()));
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

    /// Compile an untyped expression to bytecode (public interface)
    pub fn compile_to_bytecode_untyped(&self, expr: Expr<()>) -> BytecodeResult<BytecodeProgram> {
        // Convert to typed expression
        let typed_expr = self.annotate_types(expr);

        // Use the LanguageCompiler trait method
        LanguageCompiler::compile_to_bytecode(self, typed_expr)
    }
}

impl LanguageCompiler for TlispInterpreter {
    type AST = Expr<Type>;

    fn compile_to_bytecode(&self, expr: Expr<Type>) -> BytecodeResult<BytecodeProgram> {
        // Create an enhanced compiler
        let mut compiler = EnhancedTlispCompiler::new("tlisp_expr".to_string());

        // Compile the expression to bytecode using enhanced features
        compiler.compile_expr(&expr)?;

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
    /// Convert Expr<()> to Expr<Type> by adding type annotations
    pub fn annotate_types(&self, expr: Expr<()>) -> Expr<Type> {
        match expr {
            Expr::Symbol(name, _) => Expr::Symbol(name, Type::Unknown),
            Expr::Number(n, _) => Expr::Number(n, Type::Int),
            Expr::Float(f, _) => Expr::Float(f, Type::Float),
            Expr::Bool(b, _) => Expr::Bool(b, Type::Bool),
            Expr::String(s, _) => Expr::String(s, Type::String),
            Expr::List(exprs, _) => {
                let typed_exprs = exprs.into_iter().map(|e| self.annotate_types(e)).collect();
                Expr::List(typed_exprs, Type::List(Box::new(Type::Unknown)))
            },
            Expr::Application(func, args, _) => {
                let typed_func = Box::new(self.annotate_types(*func));
                let typed_args = args.into_iter().map(|e| self.annotate_types(e)).collect();
                Expr::Application(typed_func, typed_args, Type::Unknown)
            },
            Expr::Lambda(params, body, _) => {
                let typed_body = Box::new(self.annotate_types(*body));
                Expr::Lambda(params, typed_body, Type::Function(vec![Type::Unknown], Box::new(Type::Unknown)))
            },
            Expr::Let(bindings, body, _) => {
                let typed_bindings = bindings.into_iter()
                    .map(|(name, expr)| (name, self.annotate_types(expr)))
                    .collect();
                let typed_body = Box::new(self.annotate_types(*body));
                Expr::Let(typed_bindings, typed_body, Type::Unknown)
            },
            Expr::If(cond, then_expr, else_expr, _) => {
                let typed_cond = Box::new(self.annotate_types(*cond));
                let typed_then = Box::new(self.annotate_types(*then_expr));
                let typed_else = Box::new(self.annotate_types(*else_expr));
                Expr::If(typed_cond, typed_then, typed_else, Type::Unknown)
            },
            Expr::Quote(expr, _) => {
                let typed_expr = Box::new(self.annotate_types(*expr));
                Expr::Quote(typed_expr, Type::Unknown)
            },
            Expr::Define(name, expr, _) => {
                let typed_expr = Box::new(self.annotate_types(*expr));
                Expr::Define(name, typed_expr, Type::Unit)
            },
            Expr::Set(name, expr, _) => {
                let typed_expr = Box::new(self.annotate_types(*expr));
                Expr::Set(name, typed_expr, Type::Unit)
            },
            Expr::Macro(name, params, body, _) => {
                let typed_body = Box::new(self.annotate_types(*body));
                Expr::Macro(name, params, typed_body, Type::Macro)
            },
            Expr::TypeAnnotation(expr, type_expr, _) => {
                let typed_expr = Box::new(self.annotate_types(*expr));
                let typed_type_expr = Box::new(self.annotate_types(*type_expr));
                Expr::TypeAnnotation(typed_expr, typed_type_expr, Type::Unknown)
            },
        }
    }

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


