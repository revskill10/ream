//! Standard Library for TLISP
//! 
//! Implements comprehensive standard library with data structures, algorithms, and utilities.

use std::collections::HashMap;
use crate::tlisp::Value;
use crate::tlisp::types::Type;


/// Standard library functions
pub struct StandardLibrary {
    /// Built-in functions
    functions: HashMap<String, Value>,
    /// Built-in types
    types: HashMap<String, Type>,
}

impl StandardLibrary {
    /// Create a new standard library
    pub fn new() -> Self {
        let mut stdlib = StandardLibrary {
            functions: HashMap::new(),
            types: HashMap::new(),
        };
        
        stdlib.add_core_functions();
        stdlib.add_list_functions();
        stdlib.add_math_functions();
        stdlib.add_string_functions();
        stdlib.add_io_functions();
        stdlib.add_type_functions();
        stdlib.add_p2p_functions();
        
        stdlib
    }

    /// Add core functions
    fn add_core_functions(&mut self) {
        // Identity function
        self.functions.insert("identity".to_string(), Value::Builtin("identity".to_string()));
        
        // Composition
        self.functions.insert("compose".to_string(), Value::Builtin("compose".to_string()));
        
        // Conditional functions
        self.functions.insert("if".to_string(), Value::Builtin("if".to_string()));
        self.functions.insert("cond".to_string(), Value::Builtin("cond".to_string()));
        self.functions.insert("when".to_string(), Value::Builtin("when".to_string()));
        self.functions.insert("unless".to_string(), Value::Builtin("unless".to_string()));
        
        // Boolean functions
        self.functions.insert("and".to_string(), Value::Builtin("and".to_string()));
        self.functions.insert("or".to_string(), Value::Builtin("or".to_string()));
        self.functions.insert("not".to_string(), Value::Builtin("not".to_string()));
        
        // Comparison functions
        self.functions.insert("=".to_string(), Value::Builtin("=".to_string()));
        self.functions.insert("!=".to_string(), Value::Builtin("!=".to_string()));
        self.functions.insert("<".to_string(), Value::Builtin("<".to_string()));
        self.functions.insert("<=".to_string(), Value::Builtin("<=".to_string()));
        self.functions.insert(">".to_string(), Value::Builtin(">".to_string()));
        self.functions.insert(">=".to_string(), Value::Builtin(">=".to_string()));
        
        // Type predicates
        self.functions.insert("null?".to_string(), Value::Builtin("null?".to_string()));
        self.functions.insert("number?".to_string(), Value::Builtin("number?".to_string()));
        self.functions.insert("string?".to_string(), Value::Builtin("string?".to_string()));
        self.functions.insert("boolean?".to_string(), Value::Builtin("boolean?".to_string()));
        self.functions.insert("list?".to_string(), Value::Builtin("list?".to_string()));
        self.functions.insert("function?".to_string(), Value::Builtin("function?".to_string()));
    }

    /// Add list functions
    fn add_list_functions(&mut self) {
        // List construction
        self.functions.insert("list".to_string(), Value::Builtin("list".to_string()));
        self.functions.insert("cons".to_string(), Value::Builtin("cons".to_string()));
        
        // List access
        self.functions.insert("car".to_string(), Value::Builtin("car".to_string()));
        self.functions.insert("cdr".to_string(), Value::Builtin("cdr".to_string()));
        self.functions.insert("first".to_string(), Value::Builtin("first".to_string()));
        self.functions.insert("rest".to_string(), Value::Builtin("rest".to_string()));
        self.functions.insert("last".to_string(), Value::Builtin("last".to_string()));
        self.functions.insert("nth".to_string(), Value::Builtin("nth".to_string()));
        
        // List properties
        self.functions.insert("length".to_string(), Value::Builtin("length".to_string()));
        self.functions.insert("empty?".to_string(), Value::Builtin("empty?".to_string()));
        
        // List operations
        self.functions.insert("append".to_string(), Value::Builtin("append".to_string()));
        self.functions.insert("reverse".to_string(), Value::Builtin("reverse".to_string()));
        self.functions.insert("sort".to_string(), Value::Builtin("sort".to_string()));
        
        // Higher-order list functions
        self.functions.insert("map".to_string(), Value::Builtin("map".to_string()));
        self.functions.insert("filter".to_string(), Value::Builtin("filter".to_string()));
        self.functions.insert("fold".to_string(), Value::Builtin("fold".to_string()));
        self.functions.insert("reduce".to_string(), Value::Builtin("reduce".to_string()));
        self.functions.insert("fold-left".to_string(), Value::Builtin("fold-left".to_string()));
        self.functions.insert("fold-right".to_string(), Value::Builtin("fold-right".to_string()));
        
        // List searching
        self.functions.insert("find".to_string(), Value::Builtin("find".to_string()));
        self.functions.insert("member".to_string(), Value::Builtin("member".to_string()));
        self.functions.insert("contains?".to_string(), Value::Builtin("contains?".to_string()));
        
        // List transformation
        self.functions.insert("take".to_string(), Value::Builtin("take".to_string()));
        self.functions.insert("drop".to_string(), Value::Builtin("drop".to_string()));
        self.functions.insert("take-while".to_string(), Value::Builtin("take-while".to_string()));
        self.functions.insert("drop-while".to_string(), Value::Builtin("drop-while".to_string()));
        
        // List aggregation
        self.functions.insert("all?".to_string(), Value::Builtin("all?".to_string()));
        self.functions.insert("any?".to_string(), Value::Builtin("any?".to_string()));
        self.functions.insert("count".to_string(), Value::Builtin("count".to_string()));
        
        // List generation
        self.functions.insert("range".to_string(), Value::Builtin("range".to_string()));
        self.functions.insert("repeat".to_string(), Value::Builtin("repeat".to_string()));
        self.functions.insert("replicate".to_string(), Value::Builtin("replicate".to_string()));
    }

    /// Add math functions
    fn add_math_functions(&mut self) {
        // Arithmetic
        self.functions.insert("+".to_string(), Value::Builtin("+".to_string()));
        self.functions.insert("-".to_string(), Value::Builtin("-".to_string()));
        self.functions.insert("*".to_string(), Value::Builtin("*".to_string()));
        self.functions.insert("/".to_string(), Value::Builtin("/".to_string()));
        self.functions.insert("mod".to_string(), Value::Builtin("mod".to_string()));
        self.functions.insert("rem".to_string(), Value::Builtin("rem".to_string()));
        
        // Math functions
        self.functions.insert("abs".to_string(), Value::Builtin("abs".to_string()));
        self.functions.insert("min".to_string(), Value::Builtin("min".to_string()));
        self.functions.insert("max".to_string(), Value::Builtin("max".to_string()));
        self.functions.insert("sqrt".to_string(), Value::Builtin("sqrt".to_string()));
        self.functions.insert("pow".to_string(), Value::Builtin("pow".to_string()));
        self.functions.insert("exp".to_string(), Value::Builtin("exp".to_string()));
        self.functions.insert("log".to_string(), Value::Builtin("log".to_string()));
        
        // Trigonometric functions
        self.functions.insert("sin".to_string(), Value::Builtin("sin".to_string()));
        self.functions.insert("cos".to_string(), Value::Builtin("cos".to_string()));
        self.functions.insert("tan".to_string(), Value::Builtin("tan".to_string()));
        self.functions.insert("asin".to_string(), Value::Builtin("asin".to_string()));
        self.functions.insert("acos".to_string(), Value::Builtin("acos".to_string()));
        self.functions.insert("atan".to_string(), Value::Builtin("atan".to_string()));
        
        // Number predicates
        self.functions.insert("even?".to_string(), Value::Builtin("even?".to_string()));
        self.functions.insert("odd?".to_string(), Value::Builtin("odd?".to_string()));
        self.functions.insert("positive?".to_string(), Value::Builtin("positive?".to_string()));
        self.functions.insert("negative?".to_string(), Value::Builtin("negative?".to_string()));
        self.functions.insert("zero?".to_string(), Value::Builtin("zero?".to_string()));
        
        // Number conversion
        self.functions.insert("floor".to_string(), Value::Builtin("floor".to_string()));
        self.functions.insert("ceiling".to_string(), Value::Builtin("ceiling".to_string()));
        self.functions.insert("round".to_string(), Value::Builtin("round".to_string()));
        self.functions.insert("truncate".to_string(), Value::Builtin("truncate".to_string()));
    }

    /// Add string functions
    fn add_string_functions(&mut self) {
        // String construction
        self.functions.insert("string".to_string(), Value::Builtin("string".to_string()));
        self.functions.insert("string-append".to_string(), Value::Builtin("string-append".to_string()));
        
        // String properties
        self.functions.insert("string-length".to_string(), Value::Builtin("string-length".to_string()));
        self.functions.insert("string-empty?".to_string(), Value::Builtin("string-empty?".to_string()));
        
        // String access
        self.functions.insert("string-ref".to_string(), Value::Builtin("string-ref".to_string()));
        self.functions.insert("substring".to_string(), Value::Builtin("substring".to_string()));
        
        // String comparison
        self.functions.insert("string=?".to_string(), Value::Builtin("string=?".to_string()));
        self.functions.insert("string<?".to_string(), Value::Builtin("string<?".to_string()));
        self.functions.insert("string>?".to_string(), Value::Builtin("string>?".to_string()));
        
        // String transformation
        self.functions.insert("string-upcase".to_string(), Value::Builtin("string-upcase".to_string()));
        self.functions.insert("string-downcase".to_string(), Value::Builtin("string-downcase".to_string()));
        self.functions.insert("string-trim".to_string(), Value::Builtin("string-trim".to_string()));
        
        // String searching
        self.functions.insert("string-contains?".to_string(), Value::Builtin("string-contains?".to_string()));
        self.functions.insert("string-starts-with?".to_string(), Value::Builtin("string-starts-with?".to_string()));
        self.functions.insert("string-ends-with?".to_string(), Value::Builtin("string-ends-with?".to_string()));
        
        // String conversion
        self.functions.insert("string->list".to_string(), Value::Builtin("string->list".to_string()));
        self.functions.insert("list->string".to_string(), Value::Builtin("list->string".to_string()));
        self.functions.insert("string->number".to_string(), Value::Builtin("string->number".to_string()));
        self.functions.insert("number->string".to_string(), Value::Builtin("number->string".to_string()));
    }

    /// Add I/O functions
    fn add_io_functions(&mut self) {
        // Output functions
        self.functions.insert("print".to_string(), Value::Builtin("print".to_string()));
        self.functions.insert("println".to_string(), Value::Builtin("println".to_string()));
        self.functions.insert("display".to_string(), Value::Builtin("display".to_string()));
        self.functions.insert("newline".to_string(), Value::Builtin("newline".to_string()));
        
        // Input functions
        self.functions.insert("read".to_string(), Value::Builtin("read".to_string()));
        self.functions.insert("read-line".to_string(), Value::Builtin("read-line".to_string()));
        self.functions.insert("read-char".to_string(), Value::Builtin("read-char".to_string()));
        
        // File I/O
        self.functions.insert("open-file".to_string(), Value::Builtin("open-file".to_string()));
        self.functions.insert("close-file".to_string(), Value::Builtin("close-file".to_string()));
        self.functions.insert("read-file".to_string(), Value::Builtin("read-file".to_string()));
        self.functions.insert("write-file".to_string(), Value::Builtin("write-file".to_string()));
        
        // File system
        self.functions.insert("file-exists?".to_string(), Value::Builtin("file-exists?".to_string()));
        self.functions.insert("directory-exists?".to_string(), Value::Builtin("directory-exists?".to_string()));
        self.functions.insert("list-directory".to_string(), Value::Builtin("list-directory".to_string()));
    }

    /// Add type functions
    fn add_type_functions(&mut self) {
        // Type checking
        self.functions.insert("type-of".to_string(), Value::Builtin("type-of".to_string()));
        self.functions.insert("instance-of?".to_string(), Value::Builtin("instance-of?".to_string()));
        
        // Type conversion
        self.functions.insert("->int".to_string(), Value::Builtin("->int".to_string()));
        self.functions.insert("->float".to_string(), Value::Builtin("->float".to_string()));
        self.functions.insert("->string".to_string(), Value::Builtin("->string".to_string()));
        self.functions.insert("->bool".to_string(), Value::Builtin("->bool".to_string()));
        
        // Add basic types
        self.types.insert("Int".to_string(), Type::Int);
        self.types.insert("Float".to_string(), Type::Float);
        self.types.insert("Bool".to_string(), Type::Bool);
        self.types.insert("String".to_string(), Type::String);
        self.types.insert("Symbol".to_string(), Type::Symbol);
        self.types.insert("Unit".to_string(), Type::Unit);
        self.types.insert("List".to_string(), Type::List(Box::new(Type::TypeVar("a".to_string()))));
    }

    /// Get all functions
    pub fn functions(&self) -> &HashMap<String, Value> {
        &self.functions
    }

    /// Get all types
    pub fn types(&self) -> &HashMap<String, Type> {
        &self.types
    }

    /// Get function by name
    pub fn get_function(&self, name: &str) -> Option<&Value> {
        self.functions.get(name)
    }

    /// Get type by name
    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.types.get(name)
    }

    /// Check if function exists
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Check if type exists
    pub fn has_type(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }

    /// Add custom function
    pub fn add_function(&mut self, name: String, function: Value) {
        self.functions.insert(name, function);
    }

    /// Add P2P distributed system functions
    fn add_p2p_functions(&mut self) {
        // Cluster management functions
        self.functions.insert("p2p-create-cluster".to_string(), Value::Builtin("p2p-create-cluster".to_string()));
        self.functions.insert("p2p-join-cluster".to_string(), Value::Builtin("p2p-join-cluster".to_string()));
        self.functions.insert("p2p-leave-cluster".to_string(), Value::Builtin("p2p-leave-cluster".to_string()));
        self.functions.insert("p2p-cluster-info".to_string(), Value::Builtin("p2p-cluster-info".to_string()));
        self.functions.insert("p2p-cluster-members".to_string(), Value::Builtin("p2p-cluster-members".to_string()));

        // Actor management functions
        self.functions.insert("p2p-spawn-actor".to_string(), Value::Builtin("p2p-spawn-actor".to_string()));
        self.functions.insert("p2p-migrate-actor".to_string(), Value::Builtin("p2p-migrate-actor".to_string()));
        self.functions.insert("p2p-send-remote".to_string(), Value::Builtin("p2p-send-remote".to_string()));
        self.functions.insert("p2p-actor-location".to_string(), Value::Builtin("p2p-actor-location".to_string()));

        // Node management functions
        self.functions.insert("p2p-node-info".to_string(), Value::Builtin("p2p-node-info".to_string()));
        self.functions.insert("p2p-node-health".to_string(), Value::Builtin("p2p-node-health".to_string()));
        self.functions.insert("p2p-discover-nodes".to_string(), Value::Builtin("p2p-discover-nodes".to_string()));

        // Consensus functions
        self.functions.insert("p2p-propose".to_string(), Value::Builtin("p2p-propose".to_string()));
        self.functions.insert("p2p-consensus-state".to_string(), Value::Builtin("p2p-consensus-state".to_string()));
    }

    /// Add custom type
    pub fn add_type(&mut self, name: String, type_def: Type) {
        self.types.insert(name, type_def);
    }

    /// Remove function
    pub fn remove_function(&mut self, name: &str) -> Option<Value> {
        self.functions.remove(name)
    }

    /// Remove type
    pub fn remove_type(&mut self, name: &str) -> Option<Type> {
        self.types.remove(name)
    }

    /// List all function names
    pub fn list_functions(&self) -> Vec<&String> {
        self.functions.keys().collect()
    }

    /// List all type names
    pub fn list_types(&self) -> Vec<&String> {
        self.types.keys().collect()
    }

    /// Get function count
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Get type count
    pub fn type_count(&self) -> usize {
        self.types.len()
    }
}

/// Standard library utilities
pub struct StdLibUtils;

impl StdLibUtils {
    /// Create minimal standard library (core functions only)
    pub fn minimal() -> StandardLibrary {
        let mut stdlib = StandardLibrary {
            functions: HashMap::new(),
            types: HashMap::new(),
        };
        
        // Add only essential functions
        stdlib.functions.insert("+".to_string(), Value::Builtin("+".to_string()));
        stdlib.functions.insert("-".to_string(), Value::Builtin("-".to_string()));
        stdlib.functions.insert("*".to_string(), Value::Builtin("*".to_string()));
        stdlib.functions.insert("/".to_string(), Value::Builtin("/".to_string()));
        stdlib.functions.insert("=".to_string(), Value::Builtin("=".to_string()));
        stdlib.functions.insert("if".to_string(), Value::Builtin("if".to_string()));
        stdlib.functions.insert("list".to_string(), Value::Builtin("list".to_string()));
        stdlib.functions.insert("car".to_string(), Value::Builtin("car".to_string()));
        stdlib.functions.insert("cdr".to_string(), Value::Builtin("cdr".to_string()));
        stdlib.functions.insert("cons".to_string(), Value::Builtin("cons".to_string()));
        
        // Add basic types
        stdlib.types.insert("Int".to_string(), Type::Int);
        stdlib.types.insert("Bool".to_string(), Type::Bool);
        stdlib.types.insert("List".to_string(), Type::List(Box::new(Type::TypeVar("a".to_string()))));
        
        stdlib
    }

    /// Create extended standard library (with additional modules)
    pub fn extended() -> StandardLibrary {
        let mut stdlib = StandardLibrary::new();
        
        // Add additional utility functions
        stdlib.add_function("curry".to_string(), Value::Builtin("curry".to_string()));
        stdlib.add_function("uncurry".to_string(), Value::Builtin("uncurry".to_string()));
        stdlib.add_function("partial".to_string(), Value::Builtin("partial".to_string()));
        stdlib.add_function("memoize".to_string(), Value::Builtin("memoize".to_string()));
        
        // Add data structure functions
        stdlib.add_function("make-hash-table".to_string(), Value::Builtin("make-hash-table".to_string()));
        stdlib.add_function("hash-ref".to_string(), Value::Builtin("hash-ref".to_string()));
        stdlib.add_function("hash-set!".to_string(), Value::Builtin("hash-set!".to_string()));
        stdlib.add_function("make-vector".to_string(), Value::Builtin("make-vector".to_string()));
        stdlib.add_function("vector-ref".to_string(), Value::Builtin("vector-ref".to_string()));
        stdlib.add_function("vector-set!".to_string(), Value::Builtin("vector-set!".to_string()));
        
        stdlib
    }
}

impl Default for StandardLibrary {
    fn default() -> Self {
        Self::new()
    }
}
