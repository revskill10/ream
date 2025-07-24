//! Comprehensive unit tests for the TLISP module system

use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;

use crate::tlisp::module_system::{
    Module, ModuleRegistry, ModuleLoader, ModuleLanguage, ModuleMetadata, ModuleExport
};
use crate::tlisp::{Value, Type};
use crate::error::TlispResult;

/// Test helper to create a temporary module registry
fn create_test_module_registry() -> TlispResult<(ModuleRegistry, TempDir)> {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("modules");
    
    fs::create_dir_all(&cache_dir).unwrap();
    
    let registry = ModuleRegistry::new(cache_dir)?;
    Ok((registry, temp_dir))
}

/// Test helper to create sample module
fn create_sample_module(name: &str, language: ModuleLanguage) -> Module {
    let mut metadata = ModuleMetadata {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: Some(format!("Test module {}", name)),
        author: Some("Test Author".to_string()),
        language,
        dependencies: HashMap::new(),
        exports: HashMap::new(),
        source_path: None,
        compiled_path: None,
    };

    // Add some sample exports
    metadata.exports.insert(
        "test_function".to_string(),
        ModuleExport {
            name: "test_function".to_string(),
            export_type: Type::Function(vec![Type::Int], Box::new(Type::String)),
            visibility: crate::tlisp::module_system::Visibility::Public,
            documentation: Some("A test function".to_string()),
        }
    );

    Module::new(metadata)
}

/// Test helper to create sample TLISP source code
fn create_sample_tlisp_source() -> String {
    r#"
(module test-module
  "A test module for unit testing"
  
  (export test-function add-numbers)
  
  (defn test-function [x]
    "A simple test function"
    (str "Hello " x))
  
  (defn add-numbers [a b]
    "Adds two numbers"
    (+ a b))
  
  (defn private-function []
    "This function is not exported"
    42))
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let module = create_sample_module("test-module", ModuleLanguage::TLisp);
        
        assert_eq!(module.metadata().name, "test-module");
        assert_eq!(module.metadata().version, "1.0.0");
        assert_eq!(module.metadata().language, ModuleLanguage::TLisp);
        assert_eq!(module.metadata().exports.len(), 1);
        assert!(module.metadata().exports.contains_key("test_function"));
    }

    #[test]
    fn test_module_registry_creation() {
        let result = create_test_module_registry();
        assert!(result.is_ok());
        
        let (registry, _temp_dir) = result.unwrap();
        assert_eq!(registry.loaded_modules().len(), 0);
    }

    #[test]
    fn test_module_registry_register_module() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        let module = create_sample_module("test-module", ModuleLanguage::TLisp);
        
        let result = registry.register_module(module);
        assert!(result.is_ok());
        
        assert_eq!(registry.loaded_modules().len(), 1);
        assert!(registry.has_module("test-module"));
    }

    #[test]
    fn test_module_registry_register_duplicate_module() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        let module1 = create_sample_module("test-module", ModuleLanguage::TLisp);
        let module2 = create_sample_module("test-module", ModuleLanguage::TLisp);
        
        let result1 = registry.register_module(module1);
        assert!(result1.is_ok());
        
        let result2 = registry.register_module(module2);
        assert!(result2.is_err());
        
        assert_eq!(registry.loaded_modules().len(), 1);
    }

    #[test]
    fn test_module_registry_get_module() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        let module = create_sample_module("test-module", ModuleLanguage::TLisp);
        
        registry.register_module(module).unwrap();
        
        let retrieved = registry.get_module("test-module");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata().name, "test-module");
        
        let nonexistent = registry.get_module("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_module_registry_unregister_module() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        let module = create_sample_module("test-module", ModuleLanguage::TLisp);
        
        registry.register_module(module).unwrap();
        assert!(registry.has_module("test-module"));
        
        let result = registry.unregister_module("test-module");
        assert!(result.is_ok());
        assert!(!registry.has_module("test-module"));
        assert_eq!(registry.loaded_modules().len(), 0);
    }

    #[test]
    fn test_module_registry_unregister_nonexistent_module() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        
        let result = registry.unregister_module("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_module_registry_list_modules() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        
        let modules = vec![
            create_sample_module("module1", ModuleLanguage::TLisp),
            create_sample_module("module2", ModuleLanguage::Rust),
            create_sample_module("module3", ModuleLanguage::JavaScript),
        ];
        
        for module in modules {
            registry.register_module(module).unwrap();
        }
        
        let module_list = registry.list_modules();
        assert_eq!(module_list.len(), 3);
        
        let names: Vec<&str> = module_list.iter().map(|name| name.as_str()).collect();
        assert!(names.contains(&"module1"));
        assert!(names.contains(&"module2"));
        assert!(names.contains(&"module3"));
    }

    #[test]
    fn test_module_registry_clear() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        
        let modules = vec![
            create_sample_module("module1", ModuleLanguage::TLisp),
            create_sample_module("module2", ModuleLanguage::Rust),
        ];
        
        for module in modules {
            registry.register_module(module).unwrap();
        }
        
        assert_eq!(registry.loaded_modules().len(), 2);
        
        registry.clear();
        assert_eq!(registry.loaded_modules().len(), 0);
    }

    #[test]
    fn test_module_loader_creation() {
        let (registry, temp_dir) = create_test_module_registry().unwrap();
        let loader = ModuleLoader::new(registry, temp_dir.path().to_path_buf());
        
        assert_eq!(loader.search_paths().len(), 1);
        assert_eq!(loader.search_paths()[0], temp_dir.path());
    }

    #[test]
    fn test_module_loader_add_search_path() {
        let (registry, temp_dir) = create_test_module_registry().unwrap();
        let mut loader = ModuleLoader::new(registry, temp_dir.path().to_path_buf());
        
        let new_path = temp_dir.path().join("additional");
        fs::create_dir_all(&new_path).unwrap();
        
        loader.add_search_path(new_path.clone());
        assert_eq!(loader.search_paths().len(), 2);
        assert!(loader.search_paths().contains(&new_path));
    }

    #[test]
    fn test_module_loader_remove_search_path() {
        let (registry, temp_dir) = create_test_module_registry().unwrap();
        let mut loader = ModuleLoader::new(registry, temp_dir.path().to_path_buf());
        
        let new_path = temp_dir.path().join("additional");
        fs::create_dir_all(&new_path).unwrap();
        
        loader.add_search_path(new_path.clone());
        assert_eq!(loader.search_paths().len(), 2);
        
        loader.remove_search_path(&new_path);
        assert_eq!(loader.search_paths().len(), 1);
        assert!(!loader.search_paths().contains(&new_path));
    }

    #[test]
    fn test_module_loader_find_module_file() {
        let (registry, temp_dir) = create_test_module_registry().unwrap();
        let loader = ModuleLoader::new(registry, temp_dir.path().to_path_buf());
        
        // Create a test module file
        let module_file = temp_dir.path().join("test_module.tl");
        fs::write(&module_file, create_sample_tlisp_source()).unwrap();
        
        let found = loader.find_module_file("test_module");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), module_file);
        
        let not_found = loader.find_module_file("nonexistent_module");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_module_metadata_creation() {
        let metadata = ModuleMetadata {
            name: "test-module".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test module".to_string()),
            author: Some("Test Author".to_string()),
            language: ModuleLanguage::TLisp,
            dependencies: HashMap::new(),
            exports: HashMap::new(),
            source_path: Some(PathBuf::from("test_module.tl")),
            compiled_path: None,
        };
        
        assert_eq!(metadata.name, "test-module");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, Some("A test module".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.language, ModuleLanguage::TLisp);
        assert_eq!(metadata.source_path, Some(PathBuf::from("test_module.tl")));
        assert!(metadata.compiled_path.is_none());
    }

    #[test]
    fn test_module_export_creation() {
        let export = ModuleExport {
            name: "test_function".to_string(),
            export_type: Type::Function(vec![Type::Int, Type::String], Box::new(Type::Bool)),
            visibility: crate::tlisp::module_system::Visibility::Public,
            documentation: Some("A test function that takes an int and string and returns a bool".to_string()),
        };
        
        assert_eq!(export.name, "test_function");
        assert_eq!(export.visibility, crate::tlisp::module_system::Visibility::Public);
        assert!(export.documentation.is_some());
        
        match &export.export_type {
            Type::Function(params, return_type) => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], Type::Int);
                assert_eq!(params[1], Type::String);
                assert_eq!(**return_type, Type::Bool);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_module_with_dependencies() {
        let mut metadata = ModuleMetadata {
            name: "dependent-module".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A module with dependencies".to_string()),
            author: Some("Test Author".to_string()),
            language: ModuleLanguage::TLisp,
            dependencies: HashMap::new(),
            exports: HashMap::new(),
            source_path: None,
            compiled_path: None,
        };
        
        // Add dependencies
        metadata.dependencies.insert("dep1".to_string(), "^1.0.0".to_string());
        metadata.dependencies.insert("dep2".to_string(), "~2.1.0".to_string());
        
        assert_eq!(metadata.dependencies.len(), 2);
        assert!(metadata.dependencies.contains_key("dep1"));
        assert!(metadata.dependencies.contains_key("dep2"));
        assert_eq!(metadata.dependencies["dep1"], "^1.0.0");
        assert_eq!(metadata.dependencies["dep2"], "~2.1.0");
    }

    #[test]
    fn test_module_language_variants() {
        assert_eq!(ModuleLanguage::TLisp.to_string(), "TLisp");
        assert_eq!(ModuleLanguage::Rust.to_string(), "Rust");
        assert_eq!(ModuleLanguage::JavaScript.to_string(), "JavaScript");
        assert_eq!(ModuleLanguage::Python.to_string(), "Python");
        assert_eq!(ModuleLanguage::C.to_string(), "C");
        
        // Test that different languages are not equal
        assert_ne!(ModuleLanguage::TLisp, ModuleLanguage::Rust);
        assert_ne!(ModuleLanguage::JavaScript, ModuleLanguage::Python);
    }

    #[test]
    fn test_module_registry_get_modules_by_language() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        
        let modules = vec![
            create_sample_module("tlisp1", ModuleLanguage::TLisp),
            create_sample_module("tlisp2", ModuleLanguage::TLisp),
            create_sample_module("rust1", ModuleLanguage::Rust),
            create_sample_module("js1", ModuleLanguage::JavaScript),
        ];
        
        for module in modules {
            registry.register_module(module).unwrap();
        }
        
        let tlisp_modules = registry.get_modules_by_language(ModuleLanguage::TLisp);
        assert_eq!(tlisp_modules.len(), 2);
        
        let rust_modules = registry.get_modules_by_language(ModuleLanguage::Rust);
        assert_eq!(rust_modules.len(), 1);
        
        let js_modules = registry.get_modules_by_language(ModuleLanguage::JavaScript);
        assert_eq!(js_modules.len(), 1);
        
        let python_modules = registry.get_modules_by_language(ModuleLanguage::Python);
        assert_eq!(python_modules.len(), 0);
    }

    #[test]
    fn test_module_registry_statistics() {
        let (mut registry, _temp_dir) = create_test_module_registry().unwrap();
        
        let modules = vec![
            create_sample_module("module1", ModuleLanguage::TLisp),
            create_sample_module("module2", ModuleLanguage::Rust),
            create_sample_module("module3", ModuleLanguage::TLisp),
        ];
        
        for module in modules {
            registry.register_module(module).unwrap();
        }
        
        let stats = registry.statistics();
        assert_eq!(stats.total_modules, 3);
        assert_eq!(stats.modules_by_language.get(&ModuleLanguage::TLisp), Some(&2));
        assert_eq!(stats.modules_by_language.get(&ModuleLanguage::Rust), Some(&1));
        assert_eq!(stats.modules_by_language.get(&ModuleLanguage::JavaScript), None);
    }
}
