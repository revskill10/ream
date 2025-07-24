//! Comprehensive unit tests for the TLISP cross-language bridge

use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;

use crate::tlisp::cross_language_bridge::{
    CrossLanguageBridge, LanguageBridge, TypeConverterRegistry, BridgeUtils
};
use crate::tlisp::rust_integration::{RustFunction, FunctionSignature};
use crate::tlisp::{Value, Type, ModuleLanguage};
use crate::error::{TlispError, TlispResult};

/// Test implementation of RustFunction for bridge testing
struct TestBridgeFunction {
    name: String,
    signature: FunctionSignature,
}

impl TestBridgeFunction {
    fn new(name: &str, param_types: Vec<Type>, return_type: Type) -> Self {
        TestBridgeFunction {
            name: name.to_string(),
            signature: FunctionSignature {
                name: name.to_string(),
                param_types,
                return_type,
            },
        }
    }
}

impl RustFunction for TestBridgeFunction {
    fn name(&self) -> &str {
        &self.name
    }

    fn call(&self, args: &[Value]) -> TlispResult<Value> {
        match self.name.as_str() {
            "bridge_add" => {
                if args.len() != 2 {
                    return Err(TlispError::Runtime("bridge_add expects 2 arguments".to_string()));
                }
                
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                    _ => Err(TlispError::Runtime("bridge_add expects two integers".to_string())),
                }
            }
            "bridge_concat" => {
                if args.len() != 2 {
                    return Err(TlispError::Runtime("bridge_concat expects 2 arguments".to_string()));
                }
                
                match (&args[0], &args[1]) {
                    (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                    _ => Err(TlispError::Runtime("bridge_concat expects two strings".to_string())),
                }
            }
            "bridge_negate" => {
                if args.len() != 1 {
                    return Err(TlispError::Runtime("bridge_negate expects 1 argument".to_string()));
                }
                
                match &args[0] {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    _ => Err(TlispError::Runtime("bridge_negate expects a boolean".to_string())),
                }
            }
            _ => Err(TlispError::Runtime(format!("Unknown bridge function: {}", self.name))),
        }
    }

    fn signature(&self) -> FunctionSignature {
        self.signature.clone()
    }
}

/// Test implementation of LanguageBridge for testing
struct TestLanguageBridge {
    language: ModuleLanguage,
    functions: HashMap<String, Arc<dyn RustFunction>>,
}

impl TestLanguageBridge {
    fn new(language: ModuleLanguage) -> Self {
        let mut bridge = TestLanguageBridge {
            language,
            functions: HashMap::new(),
        };
        
        // Add test functions
        bridge.functions.insert(
            "bridge_add".to_string(),
            Arc::new(TestBridgeFunction::new("bridge_add", vec![Type::Int, Type::Int], Type::Int))
        );
        
        bridge.functions.insert(
            "bridge_concat".to_string(),
            Arc::new(TestBridgeFunction::new("bridge_concat", vec![Type::String, Type::String], Type::String))
        );
        
        bridge.functions.insert(
            "bridge_negate".to_string(),
            Arc::new(TestBridgeFunction::new("bridge_negate", vec![Type::Bool], Type::Bool))
        );
        
        bridge
    }
}

impl LanguageBridge for TestLanguageBridge {
    fn language(&self) -> ModuleLanguage {
        self.language.clone()
    }

    fn call_function(&self, name: &str, args: &[Value]) -> TlispResult<Value> {
        if let Some(function) = self.functions.get(name) {
            function.call(args)
        } else {
            Err(TlispError::Runtime(format!("Function '{}' not found in test bridge", name)))
        }
    }

    fn get_function_signature(&self, name: &str) -> Option<FunctionSignature> {
        self.functions.get(name).map(|f| f.signature())
    }

    fn list_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    fn convert_from_tlisp(&self, value: &Value, _target_type: &str) -> TlispResult<Box<dyn Any + '_>> {
        // Simple pass-through conversion for testing
        Ok(Box::new(value.clone()))
    }

    fn convert_to_tlisp(&self, value: Box<dyn Any>, _source_type: &str) -> TlispResult<Value> {
        // Try to downcast back to Value
        if let Some(val) = value.downcast_ref::<Value>() {
            Ok(val.clone())
        } else {
            Err(TlispError::Runtime("Failed to convert value to TLISP".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_language_bridge_creation() {
        let bridge = CrossLanguageBridge::new();
        
        // Should have at least the Rust bridge registered by default
        let languages = bridge.list_languages();
        assert!(languages.contains(&ModuleLanguage::Rust));
    }

    #[test]
    fn test_cross_language_bridge_register_bridge() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        let languages = bridge.list_languages();
        assert!(languages.contains(&ModuleLanguage::JavaScript));
    }

    #[test]
    fn test_cross_language_bridge_call_function() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        // Test calling bridge_add function
        let args = vec![Value::Int(5), Value::Int(3)];
        let result = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args);
        
        assert!(result.is_ok());
        let call_result = result.unwrap();
        assert_eq!(call_result.result, Value::Int(8));
        assert!(!call_result.was_cached);
        assert_eq!(call_result.conversions, 2);
    }

    #[test]
    fn test_cross_language_bridge_call_function_cached() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        let args = vec![Value::Int(5), Value::Int(3)];
        
        // First call should not be cached
        let result1 = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args);
        assert!(result1.is_ok());
        assert!(!result1.unwrap().was_cached);
        
        // Second call with same arguments should be cached
        let result2 = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args);
        assert!(result2.is_ok());
        assert!(result2.unwrap().was_cached);
    }

    #[test]
    fn test_cross_language_bridge_call_nonexistent_function() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        let args = vec![Value::Int(5), Value::Int(3)];
        let result = bridge.call_function(ModuleLanguage::JavaScript, "nonexistent_function", &args);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_language_bridge_call_nonexistent_language() {
        let mut bridge = CrossLanguageBridge::new();
        
        let args = vec![Value::Int(5), Value::Int(3)];
        let result = bridge.call_function(ModuleLanguage::Python, "some_function", &args);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_language_bridge_get_function_signature() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        let signature = bridge.get_function_signature(ModuleLanguage::JavaScript, "bridge_add");
        assert!(signature.is_some());
        
        let sig = signature.unwrap();
        assert_eq!(sig.name, "bridge_add");
        assert_eq!(sig.param_types, vec![Type::Int, Type::Int]);
        assert_eq!(sig.return_type, Type::Int);
        
        let nonexistent = bridge.get_function_signature(ModuleLanguage::JavaScript, "nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_cross_language_bridge_list_functions() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        let functions = bridge.list_functions(ModuleLanguage::JavaScript);
        assert_eq!(functions.len(), 3);
        assert!(functions.contains(&"bridge_add".to_string()));
        assert!(functions.contains(&"bridge_concat".to_string()));
        assert!(functions.contains(&"bridge_negate".to_string()));
        
        let empty_functions = bridge.list_functions(ModuleLanguage::Python);
        assert_eq!(empty_functions.len(), 0);
    }

    #[test]
    fn test_cross_language_bridge_has_function() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        assert!(bridge.has_function(ModuleLanguage::JavaScript, "bridge_add"));
        assert!(bridge.has_function(ModuleLanguage::JavaScript, "bridge_concat"));
        assert!(!bridge.has_function(ModuleLanguage::JavaScript, "nonexistent"));
        assert!(!bridge.has_function(ModuleLanguage::Python, "bridge_add"));
    }

    #[test]
    fn test_cross_language_bridge_clear_cache() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        let args = vec![Value::Int(5), Value::Int(3)];
        
        // Make a call to populate cache
        let _result = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args);
        
        // Clear cache
        bridge.clear_cache();
        
        // Next call should not be cached
        let result = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args);
        assert!(result.is_ok());
        assert!(!result.unwrap().was_cached);
    }

    #[test]
    fn test_cross_language_bridge_set_cache_size() {
        let mut bridge = CrossLanguageBridge::new();
        
        // Set a small cache size
        bridge.set_cache_size(2);
        
        // This should work without errors
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        bridge.register_bridge(test_bridge);
        
        let args = vec![Value::Int(5), Value::Int(3)];
        let _result = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args);
    }

    #[test]
    fn test_cross_language_bridge_stats() {
        let mut bridge = CrossLanguageBridge::new();
        let test_bridge = Box::new(TestLanguageBridge::new(ModuleLanguage::JavaScript));
        
        bridge.register_bridge(test_bridge);
        
        let args = vec![Value::Int(5), Value::Int(3)];
        
        // Make some calls
        let result1 = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args);
        assert!(result1.is_ok());

        let result2 = bridge.call_function(ModuleLanguage::JavaScript, "bridge_add", &args); // Should be cached
        assert!(result2.is_ok());

        let result3 = bridge.call_function(ModuleLanguage::JavaScript, "bridge_concat", &vec![Value::String("a".to_string()), Value::String("b".to_string())]);
        assert!(result3.is_ok());

        let stats = bridge.stats();
        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.successful_calls, 3);
        assert_eq!(stats.failed_calls, 0);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 2);
    }

    #[test]
    fn test_type_converter_registry_creation() {
        let _registry = TypeConverterRegistry::new();

        // Should be created successfully
        // Note: We can't test much here since the registry methods are not public
        // This test mainly ensures the constructor works
    }

    #[test]
    fn test_bridge_utils_get_default_type_mapping() {
        let rust_mapping = BridgeUtils::get_default_type_mapping(ModuleLanguage::Rust);
        assert!(rust_mapping.contains_key(&Type::Int));
        assert!(rust_mapping.contains_key(&Type::Float));
        assert!(rust_mapping.contains_key(&Type::Bool));
        assert!(rust_mapping.contains_key(&Type::String));
        assert!(rust_mapping.contains_key(&Type::Unit));
        
        assert_eq!(rust_mapping[&Type::Int], "i64");
        assert_eq!(rust_mapping[&Type::Float], "f64");
        assert_eq!(rust_mapping[&Type::Bool], "bool");
        assert_eq!(rust_mapping[&Type::String], "String");
        assert_eq!(rust_mapping[&Type::Unit], "()");
        
        let js_mapping = BridgeUtils::get_default_type_mapping(ModuleLanguage::JavaScript);
        assert_eq!(js_mapping[&Type::Int], "number");
        assert_eq!(js_mapping[&Type::Float], "number");
        assert_eq!(js_mapping[&Type::Bool], "boolean");
        assert_eq!(js_mapping[&Type::String], "string");
        assert_eq!(js_mapping[&Type::Unit], "undefined");
        
        let python_mapping = BridgeUtils::get_default_type_mapping(ModuleLanguage::Python);
        assert_eq!(python_mapping[&Type::Int], "int");
        assert_eq!(python_mapping[&Type::Float], "float");
        assert_eq!(python_mapping[&Type::Bool], "bool");
        assert_eq!(python_mapping[&Type::String], "str");
        assert_eq!(python_mapping[&Type::Unit], "None");
        
        let c_mapping = BridgeUtils::get_default_type_mapping(ModuleLanguage::C);
        assert_eq!(c_mapping[&Type::Int], "long long");
        assert_eq!(c_mapping[&Type::Float], "double");
        assert_eq!(c_mapping[&Type::Bool], "int");
        assert_eq!(c_mapping[&Type::String], "char*");
        assert_eq!(c_mapping[&Type::Unit], "void");
    }

    #[test]
    fn test_test_language_bridge_implementation() {
        let bridge = TestLanguageBridge::new(ModuleLanguage::JavaScript);
        
        assert_eq!(bridge.language(), ModuleLanguage::JavaScript);
        assert_eq!(bridge.list_functions().len(), 3);
        assert!(bridge.has_function("bridge_add"));
        assert!(bridge.has_function("bridge_concat"));
        assert!(bridge.has_function("bridge_negate"));
        assert!(!bridge.has_function("nonexistent"));
        
        // Test function calls
        let add_result = bridge.call_function("bridge_add", &[Value::Int(2), Value::Int(3)]);
        assert!(add_result.is_ok());
        assert_eq!(add_result.unwrap(), Value::Int(5));
        
        let concat_result = bridge.call_function("bridge_concat", &[Value::String("Hello ".to_string()), Value::String("World".to_string())]);
        assert!(concat_result.is_ok());
        assert_eq!(concat_result.unwrap(), Value::String("Hello World".to_string()));
        
        let negate_result = bridge.call_function("bridge_negate", &[Value::Bool(true)]);
        assert!(negate_result.is_ok());
        assert_eq!(negate_result.unwrap(), Value::Bool(false));
        
        let error_result = bridge.call_function("nonexistent", &[]);
        assert!(error_result.is_err());
    }
}
