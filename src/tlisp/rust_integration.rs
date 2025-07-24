//! Rust Integration for TLISP
//! 
//! Implements seamless Rust function calling and data structure interop from TLISP.

use std::collections::HashMap;
use std::any::Any;
use std::sync::Arc;
use crate::tlisp::Value;
use crate::tlisp::types::Type;
use crate::error::{TlispError, TlispResult};

/// Rust function wrapper for TLISP
pub trait RustFunction: Send + Sync {
    /// Call the Rust function with TLISP values
    fn call(&self, args: &[Value]) -> TlispResult<Value>;
    
    /// Get function signature
    fn signature(&self) -> FunctionSignature;
    
    /// Get function name
    fn name(&self) -> &str;
}

/// Function signature
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Parameter types
    pub param_types: Vec<Type>,
    /// Return type
    pub return_type: Type,
    /// Function name
    pub name: String,
}

/// Rust data converter
pub trait RustConverter<T> {
    /// Convert from TLISP value to Rust type
    fn from_tlisp(value: &Value) -> TlispResult<T>;
    
    /// Convert from Rust type to TLISP value
    fn to_tlisp(value: T) -> TlispResult<Value>;
    
    /// Get the TLISP type for this Rust type
    fn tlisp_type() -> Type;
}

/// Rust integration manager
pub struct RustIntegration {
    /// Registered Rust functions
    functions: HashMap<String, Arc<dyn RustFunction>>,
    /// Type converters
    converters: HashMap<String, Box<dyn Any + Send + Sync>>,
    /// Rust modules
    modules: HashMap<String, RustModule>,
}

/// Rust module
#[derive(Debug, Clone)]
pub struct RustModule {
    /// Module name
    pub name: String,
    /// Module functions
    pub functions: Vec<String>,
    /// Module types
    pub types: Vec<String>,
    /// Module path
    pub path: Option<String>,
}

impl FunctionSignature {
    /// Create a new function signature
    pub fn new(name: String, param_types: Vec<Type>, return_type: Type) -> Self {
        FunctionSignature {
            name,
            param_types,
            return_type,
        }
    }

    /// Check if arguments match signature
    pub fn matches_args(&self, args: &[Value]) -> bool {
        if args.len() != self.param_types.len() {
            return false;
        }

        for (arg, expected_type) in args.iter().zip(&self.param_types) {
            let arg_type = arg.type_of();
            if !self.types_compatible(&arg_type, expected_type) {
                return false;
            }
        }

        true
    }

    /// Check if two types are compatible
    fn types_compatible(&self, actual: &Type, expected: &Type) -> bool {
        // Simple type compatibility check
        // In a full implementation, this would handle subtyping, etc.
        actual == expected || matches!(expected, Type::TypeVar(_))
    }
}

impl RustIntegration {
    /// Create a new Rust integration
    pub fn new() -> Self {
        let mut integration = RustIntegration {
            functions: HashMap::new(),
            converters: HashMap::new(),
            modules: HashMap::new(),
        };
        
        integration.add_builtin_converters();
        integration.add_builtin_functions();
        
        integration
    }

    /// Register a Rust function
    pub fn register_function(&mut self, function: Arc<dyn RustFunction>) {
        let name = function.name().to_string();
        self.functions.insert(name, function);
    }

    /// Call a Rust function
    pub fn call_function(&self, name: &str, args: &[Value]) -> TlispResult<Value> {
        if let Some(function) = self.functions.get(name) {
            // Check signature
            let signature = function.signature();
            if !signature.matches_args(args) {
                return Err(TlispError::Runtime(
                    format!("Arguments don't match signature for function {}", name)
                ));
            }

            function.call(args)
        } else {
            Err(TlispError::Runtime(format!("Rust function {} not found", name)))
        }
    }

    /// Register a Rust module
    pub fn register_module(&mut self, module: RustModule) {
        self.modules.insert(module.name.clone(), module);
    }

    /// Get function signature
    pub fn get_signature(&self, name: &str) -> Option<FunctionSignature> {
        self.functions.get(name).map(|f| f.signature())
    }

    /// List all functions
    pub fn list_functions(&self) -> Vec<&String> {
        self.functions.keys().collect()
    }

    /// List all modules
    pub fn list_modules(&self) -> Vec<&String> {
        self.modules.keys().collect()
    }

    /// Add built-in converters
    fn add_builtin_converters(&mut self) {
        // Basic type converters are implemented as part of RustConverter trait
        // This would register them in a real implementation
    }

    /// Add built-in functions
    fn add_builtin_functions(&mut self) {
        // Add some basic Rust functions
        self.register_function(Arc::new(PrintFunction));
        self.register_function(Arc::new(LengthFunction));
        self.register_function(Arc::new(TypeOfFunction));
    }
}

/// Built-in converters for basic types
impl RustConverter<i64> for i64 {
    fn from_tlisp(value: &Value) -> TlispResult<i64> {
        match value {
            Value::Int(n) => Ok(*n),
            _ => Err(TlispError::Runtime("Expected integer".to_string())),
        }
    }

    fn to_tlisp(value: i64) -> TlispResult<Value> {
        Ok(Value::Int(value))
    }

    fn tlisp_type() -> Type {
        Type::Int
    }
}

impl RustConverter<f64> for f64 {
    fn from_tlisp(value: &Value) -> TlispResult<f64> {
        match value {
            Value::Float(f) => Ok(*f),
            Value::Int(n) => Ok(*n as f64),
            _ => Err(TlispError::Runtime("Expected number".to_string())),
        }
    }

    fn to_tlisp(value: f64) -> TlispResult<Value> {
        Ok(Value::Float(value))
    }

    fn tlisp_type() -> Type {
        Type::Float
    }
}

impl RustConverter<bool> for bool {
    fn from_tlisp(value: &Value) -> TlispResult<bool> {
        match value {
            Value::Bool(b) => Ok(*b),
            _ => Err(TlispError::Runtime("Expected boolean".to_string())),
        }
    }

    fn to_tlisp(value: bool) -> TlispResult<Value> {
        Ok(Value::Bool(value))
    }

    fn tlisp_type() -> Type {
        Type::Bool
    }
}

impl RustConverter<String> for String {
    fn from_tlisp(value: &Value) -> TlispResult<String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            Value::Symbol(s) => Ok(s.clone()),
            _ => Err(TlispError::Runtime("Expected string".to_string())),
        }
    }

    fn to_tlisp(value: String) -> TlispResult<Value> {
        Ok(Value::String(value))
    }

    fn tlisp_type() -> Type {
        Type::String
    }
}

impl RustConverter<Vec<Value>> for Vec<Value> {
    fn from_tlisp(value: &Value) -> TlispResult<Vec<Value>> {
        match value {
            Value::List(list) => Ok(list.clone()),
            _ => Err(TlispError::Runtime("Expected list".to_string())),
        }
    }

    fn to_tlisp(value: Vec<Value>) -> TlispResult<Value> {
        Ok(Value::List(value))
    }

    fn tlisp_type() -> Type {
        Type::List(Box::new(Type::TypeVar("a".to_string())))
    }
}

/// Example Rust functions

/// Print function
struct PrintFunction;

impl RustFunction for PrintFunction {
    fn call(&self, args: &[Value]) -> TlispResult<Value> {
        for arg in args {
            print!("{} ", format_value(arg));
        }
        println!();
        Ok(Value::Unit)
    }

    fn signature(&self) -> FunctionSignature {
        FunctionSignature::new(
            "print".to_string(),
            vec![Type::TypeVar("a".to_string())], // Variadic
            Type::Unit,
        )
    }

    fn name(&self) -> &str {
        "print"
    }
}

/// Length function
struct LengthFunction;

impl RustFunction for LengthFunction {
    fn call(&self, args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("length expects 1 argument".to_string()));
        }

        match &args[0] {
            Value::List(list) => Ok(Value::Int(list.len() as i64)),
            Value::String(s) => Ok(Value::Int(s.len() as i64)),
            _ => Err(TlispError::Runtime("length expects list or string".to_string())),
        }
    }

    fn signature(&self) -> FunctionSignature {
        FunctionSignature::new(
            "length".to_string(),
            vec![Type::List(Box::new(Type::TypeVar("a".to_string())))],
            Type::Int,
        )
    }

    fn name(&self) -> &str {
        "length"
    }
}

/// Type-of function
struct TypeOfFunction;

impl RustFunction for TypeOfFunction {
    fn call(&self, args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("type-of expects 1 argument".to_string()));
        }

        let type_name = match &args[0] {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::String(_) => "String",
            Value::Symbol(_) => "Symbol",
            Value::List(_) => "List",
            Value::Function(_) => "Function",
            Value::Builtin(_) => "Builtin",
            Value::Pid(_) => "Pid",
            Value::StmVar(_) => "StmVar",
            Value::Unit => "Unit",
            Value::Null => "Null",
        };

        Ok(Value::Symbol(type_name.to_string()))
    }

    fn signature(&self) -> FunctionSignature {
        FunctionSignature::new(
            "type-of".to_string(),
            vec![Type::TypeVar("a".to_string())],
            Type::Symbol,
        )
    }

    fn name(&self) -> &str {
        "type-of"
    }
}

/// Utility functions

/// Format a value for display
fn format_value(value: &Value) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Symbol(s) => s.clone(),
        Value::List(list) => {
            let items: Vec<String> = list.iter().map(format_value).collect();
            format!("({})", items.join(" "))
        }
        Value::Function(_) => "#<function>".to_string(),
        Value::Builtin(name) => format!("#<builtin:{}>", name),
        Value::Pid(pid) => format!("#<pid:{}>", pid),
        Value::StmVar(var) => format!("#<stm-var:{}>", var.name()),
        Value::Unit => "#<unit>".to_string(),
        Value::Null => "null".to_string(),
    }
}

/// Rust integration utilities
pub struct RustUtils;

impl RustUtils {
    /// Create a simple Rust function wrapper
    pub fn wrap_function<F, R>(name: String, f: F) -> impl RustFunction
    where
        F: Fn(&[Value]) -> TlispResult<R> + Send + Sync + 'static,
        R: Into<Value>,
    {
        SimpleFunctionWrapper {
            name,
            function: Box::new(move |args| f(args).map(|r| r.into())),
        }
    }

    /// Create a Rust module from functions
    pub fn create_module(name: String, functions: Vec<Arc<dyn RustFunction>>) -> RustModule {
        let function_names = functions.iter().map(|f| f.name().to_string()).collect();
        
        RustModule {
            name,
            functions: function_names,
            types: Vec::new(),
            path: None,
        }
    }
}

/// Simple function wrapper
struct SimpleFunctionWrapper {
    name: String,
    function: Box<dyn Fn(&[Value]) -> TlispResult<Value> + Send + Sync>,
}

impl RustFunction for SimpleFunctionWrapper {
    fn call(&self, args: &[Value]) -> TlispResult<Value> {
        (self.function)(args)
    }

    fn signature(&self) -> FunctionSignature {
        // Generic signature - in a real implementation, this would be more specific
        FunctionSignature::new(
            self.name.clone(),
            vec![Type::TypeVar("a".to_string())],
            Type::TypeVar("b".to_string()),
        )
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Default for RustIntegration {
    fn default() -> Self {
        Self::new()
    }
}
