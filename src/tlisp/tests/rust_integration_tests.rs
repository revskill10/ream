//! Comprehensive unit tests for the TLISP Rust integration

use std::sync::Arc;

use crate::tlisp::rust_integration::{
    RustIntegration, RustFunction, RustModule, FunctionSignature
};
use crate::tlisp::{Value, Type};
use crate::error::{TlispError, TlispResult};

/// Test implementation of RustFunction for testing
struct TestRustFunction {
    name: String,
    signature: FunctionSignature,
}

impl TestRustFunction {
    fn new(name: &str, param_types: Vec<Type>, return_type: Type) -> Self {
        TestRustFunction {
            name: name.to_string(),
            signature: FunctionSignature {
                name: name.to_string(),
                param_types,
                return_type,
            },
        }
    }
}

impl RustFunction for TestRustFunction {
    fn name(&self) -> &str {
        &self.name
    }

    fn call(&self, args: &[Value]) -> TlispResult<Value> {
        match self.name.as_str() {
            "add_numbers" => {
                if args.len() != 2 {
                    return Err(TlispError::Runtime("add_numbers expects 2 arguments".to_string()));
                }
                
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                    _ => Err(TlispError::Runtime("add_numbers expects two integers".to_string())),
                }
            }
            "multiply_numbers" => {
                if args.len() != 2 {
                    return Err(TlispError::Runtime("multiply_numbers expects 2 arguments".to_string()));
                }
                
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                    _ => Err(TlispError::Runtime("multiply_numbers expects two integers".to_string())),
                }
            }
            "concat_strings" => {
                if args.len() != 2 {
                    return Err(TlispError::Runtime("concat_strings expects 2 arguments".to_string()));
                }
                
                match (&args[0], &args[1]) {
                    (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                    _ => Err(TlispError::Runtime("concat_strings expects two strings".to_string())),
                }
            }
            "is_positive" => {
                if args.len() != 1 {
                    return Err(TlispError::Runtime("is_positive expects 1 argument".to_string()));
                }
                
                match &args[0] {
                    Value::Int(n) => Ok(Value::Bool(*n > 0)),
                    _ => Err(TlispError::Runtime("is_positive expects an integer".to_string())),
                }
            }
            "get_length" => {
                if args.len() != 1 {
                    return Err(TlispError::Runtime("get_length expects 1 argument".to_string()));
                }
                
                match &args[0] {
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
                    Value::List(items) => Ok(Value::Int(items.len() as i64)),
                    _ => Err(TlispError::Runtime("get_length expects a string or list".to_string())),
                }
            }
            _ => Err(TlispError::Runtime(format!("Unknown function: {}", self.name))),
        }
    }

    fn signature(&self) -> FunctionSignature {
        self.signature.clone()
    }
}

/// Test helper to create a sample Rust module
fn create_test_rust_module() -> RustModule {
    let mut module = RustModule {
        name: "test_module".to_string(),
        functions: Vec::new(),
        types: Vec::new(),
        path: None,
    };

    // Add test function names
    module.functions = vec![
        "add_numbers".to_string(),
        "multiply_numbers".to_string(),
        "concat_strings".to_string(),
        "is_positive".to_string(),
        "get_length".to_string(),
    ];

    module
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_function_creation() {
        let func = TestRustFunction::new("test_func", vec![Type::Int], Type::String);
        
        assert_eq!(func.name, "test_func");
        assert_eq!(func.signature.name, "test_func");
        assert_eq!(func.signature.param_types, vec![Type::Int]);
        assert_eq!(func.signature.return_type, Type::String);
    }

    #[test]
    fn test_rust_function_add_numbers() {
        let func = TestRustFunction::new("add_numbers", vec![Type::Int, Type::Int], Type::Int);
        
        let args = vec![Value::Int(5), Value::Int(3)];
        let result = func.call(&args);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(8));
    }

    #[test]
    fn test_rust_function_add_numbers_wrong_args() {
        let func = TestRustFunction::new("add_numbers", vec![Type::Int, Type::Int], Type::Int);
        
        // Wrong number of arguments
        let args = vec![Value::Int(5)];
        let result = func.call(&args);
        assert!(result.is_err());
        
        // Wrong argument types
        let args = vec![Value::String("5".to_string()), Value::Int(3)];
        let result = func.call(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_rust_function_multiply_numbers() {
        let func = TestRustFunction::new("multiply_numbers", vec![Type::Int, Type::Int], Type::Int);
        
        let args = vec![Value::Int(4), Value::Int(7)];
        let result = func.call(&args);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(28));
    }

    #[test]
    fn test_rust_function_concat_strings() {
        let func = TestRustFunction::new("concat_strings", vec![Type::String, Type::String], Type::String);
        
        let args = vec![Value::String("Hello ".to_string()), Value::String("World!".to_string())];
        let result = func.call(&args);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("Hello World!".to_string()));
    }

    #[test]
    fn test_rust_function_is_positive() {
        let func = TestRustFunction::new("is_positive", vec![Type::Int], Type::Bool);
        
        // Test positive number
        let args = vec![Value::Int(5)];
        let result = func.call(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Bool(true));
        
        // Test negative number
        let args = vec![Value::Int(-3)];
        let result = func.call(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Bool(false));
        
        // Test zero
        let args = vec![Value::Int(0)];
        let result = func.call(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_rust_function_get_length() {
        let func = TestRustFunction::new("get_length", vec![Type::String], Type::Int);
        
        // Test string length
        let args = vec![Value::String("Hello".to_string())];
        let result = func.call(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(5));
        
        // Test list length
        let args = vec![Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])];
        let result = func.call(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    #[test]
    fn test_rust_module_creation() {
        let module = RustModule {
            name: "test_module".to_string(),
            functions: Vec::new(),
            types: Vec::new(),
            path: None,
        };

        assert_eq!(module.name, "test_module");
        assert_eq!(module.functions.len(), 0);
    }

    #[test]
    fn test_rust_module_with_functions() {
        let module = create_test_rust_module();

        assert_eq!(module.name, "test_module");
        assert_eq!(module.functions.len(), 5);
        assert!(module.functions.contains(&"add_numbers".to_string()));
        assert!(module.functions.contains(&"multiply_numbers".to_string()));
        assert!(module.functions.contains(&"concat_strings".to_string()));
        assert!(module.functions.contains(&"is_positive".to_string()));
        assert!(module.functions.contains(&"get_length".to_string()));
    }

    #[test]
    fn test_rust_integration_creation() {
        let integration = RustIntegration::new();

        assert_eq!(integration.list_modules().len(), 0);
    }

    #[test]
    fn test_rust_integration_register_module() {
        let mut integration = RustIntegration::new();
        let module = create_test_rust_module();

        integration.register_module(module);

        assert_eq!(integration.list_modules().len(), 1);
        assert!(integration.list_modules().contains(&&"test_module".to_string()));
    }

    #[test]
    fn test_rust_integration_register_multiple_modules() {
        let mut integration = RustIntegration::new();
        let module1 = create_test_rust_module();
        let mut module2 = create_test_rust_module();
        module2.name = "test_module_2".to_string();

        integration.register_module(module1);
        integration.register_module(module2);

        assert_eq!(integration.list_modules().len(), 2);
        assert!(integration.list_modules().contains(&&"test_module".to_string()));
        assert!(integration.list_modules().contains(&&"test_module_2".to_string()));
    }

    #[test]
    fn test_rust_integration_register_function() {
        let mut integration = RustIntegration::new();
        let initial_count = integration.list_functions().len();
        let func = Arc::new(TestRustFunction::new("test_func", vec![Type::Int], Type::String));

        integration.register_function(func);

        assert_eq!(integration.list_functions().len(), initial_count + 1);
        assert!(integration.list_functions().contains(&&"test_func".to_string()));
    }

    #[test]
    fn test_rust_integration_call_function() {
        let mut integration = RustIntegration::new();
        let func = Arc::new(TestRustFunction::new("add_numbers", vec![Type::Int, Type::Int], Type::Int));

        integration.register_function(func);

        // Test calling add_numbers
        let args = vec![Value::Int(10), Value::Int(20)];
        let result = integration.call_function("add_numbers", &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(30));

        // Test calling nonexistent function
        let result = integration.call_function("nonexistent", &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_rust_integration_get_function_signature() {
        let mut integration = RustIntegration::new();
        let func = Arc::new(TestRustFunction::new("add_numbers", vec![Type::Int, Type::Int], Type::Int));

        integration.register_function(func);

        let signature = integration.get_signature("add_numbers");
        assert!(signature.is_some());

        let sig = signature.unwrap();
        assert_eq!(sig.name, "add_numbers");
        assert_eq!(sig.param_types, vec![Type::Int, Type::Int]);
        assert_eq!(sig.return_type, Type::Int);

        let nonexistent = integration.get_signature("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_rust_integration_list_modules() {
        let mut integration = RustIntegration::new();

        let module1 = create_test_rust_module();
        let module2 = RustModule {
            name: "another_module".to_string(),
            functions: vec!["test_func".to_string()],
            types: Vec::new(),
            path: None,
        };

        integration.register_module(module1);
        integration.register_module(module2);

        let module_names = integration.list_modules();
        assert_eq!(module_names.len(), 2);
        assert!(module_names.contains(&&"test_module".to_string()));
        assert!(module_names.contains(&&"another_module".to_string()));
    }

    #[test]
    fn test_function_signature_creation() {
        let signature = FunctionSignature {
            name: "test_function".to_string(),
            param_types: vec![Type::Int, Type::String],
            return_type: Type::Bool,
        };

        assert_eq!(signature.name, "test_function");
        assert_eq!(signature.param_types.len(), 2);
        assert_eq!(signature.param_types[0], Type::Int);
        assert_eq!(signature.param_types[1], Type::String);
        assert_eq!(signature.return_type, Type::Bool);
    }
}
