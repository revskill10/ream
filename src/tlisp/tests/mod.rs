//! Comprehensive test suite for TLISP module and package management system
//! 
//! This module contains all unit tests for the TLISP module system, package manager,
//! Rust integration, and cross-language bridge functionality.

// Test modules
pub mod package_manager_tests;
pub mod rust_integration_tests;
pub mod cross_language_bridge_tests;
pub mod package_config_tests;

#[cfg(test)]
mod test_utils {
    //! Utility functions and helpers for testing
    
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;
    use crate::error::TlispResult;
    
    /// Create a temporary directory for testing
    pub fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temporary directory")
    }
    
    /// Create a temporary file with given content
    pub fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        fs::write(&file_path, content).expect("Failed to write temporary file");
        file_path
    }
    
    /// Create a sample TLISP module file
    pub fn create_sample_tlisp_module(dir: &TempDir, name: &str) -> PathBuf {
        let content = format!(
            r#"
(module {}
  "A sample TLISP module for testing"
  
  (export hello-world add-numbers factorial)
  
  (defn hello-world [name]
    "Returns a greeting message"
    (str "Hello, " name "!"))
  
  (defn add-numbers [a b]
    "Adds two numbers"
    (+ a b))
  
  (defn factorial [n]
    "Calculates factorial of n"
    (if (<= n 1)
        1
        (* n (factorial (- n 1)))))
  
  (defn private-helper []
    "This function is not exported"
    42))
"#,
            name
        );
        
        create_temp_file(dir, &format!("{}.tl", name), &content)
    }
    
    /// Create a sample package.toml file
    pub fn create_sample_package_toml(dir: &TempDir, name: &str, version: &str) -> PathBuf {
        let content = format!(
            r#"
[package]
name = "{}"
version = "{}"
description = "A sample TLISP package for testing"
authors = ["Test Author <test@example.com>"]
license = "MIT"
homepage = "https://example.com/{}"
repository = "https://github.com/example/{}"
keywords = ["test", "sample", "tlisp"]
categories = ["testing", "development"]
edition = "2024"

[dependencies]
# No dependencies for this sample

[dev-dependencies]
# No dev dependencies for this sample

[features]
default = []
extra = ["feature1", "feature2"]

[build]
incremental = true
opt_level = "2"
debug = false

[[bin]]
name = "{}"
path = "src/main.tl"

[lib]
name = "{}"
path = "src/lib.tl"
"#,
            name, version, name, name, name, name
        );
        
        create_temp_file(dir, "package.toml", &content)
    }
    
    /// Create a sample Rust integration module
    pub fn create_sample_rust_module_source(dir: &TempDir) -> PathBuf {
        let content = r#"
//! Sample Rust module for TLISP integration testing

use tlisp_integration::*;

#[tlisp_export]
pub fn rust_add(a: i64, b: i64) -> i64 {
    a + b
}

#[tlisp_export]
pub fn rust_multiply(a: i64, b: i64) -> i64 {
    a * b
}

#[tlisp_export]
pub fn rust_concat(a: String, b: String) -> String {
    format!("{}{}", a, b)
}

#[tlisp_export]
pub fn rust_is_even(n: i64) -> bool {
    n % 2 == 0
}

#[tlisp_export]
pub fn rust_fibonacci(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        rust_fibonacci(n - 1) + rust_fibonacci(n - 2)
    }
}
"#;
        
        create_temp_file(dir, "rust_module.rs", content)
    }
    
    /// Verify that a directory structure exists
    pub fn verify_directory_structure(base_dir: &PathBuf, expected_dirs: &[&str]) -> bool {
        for dir in expected_dirs {
            let dir_path = base_dir.join(dir);
            if !dir_path.exists() || !dir_path.is_dir() {
                return false;
            }
        }
        true
    }
    
    /// Verify that files exist
    pub fn verify_files_exist(base_dir: &Path, expected_files: &[&str]) -> bool {
        for file in expected_files {
            let file_path = base_dir.join(file);
            if !file_path.exists() || !file_path.is_file() {
                return false;
            }
        }
        true
    }
    
    /// Count files in a directory with a specific extension
    pub fn count_files_with_extension(dir: &Path, extension: &str) -> usize {
        if !dir.exists() || !dir.is_dir() {
            return 0;
        }
        
        fs::read_dir(dir)
            .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", dir))
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().is_file() && 
                entry.path().extension().map_or(false, |ext| ext == extension)
            })
            .count()
    }
    
    /// Read file content as string
    pub fn read_file_content(file_path: &PathBuf) -> TlispResult<String> {
        fs::read_to_string(file_path)
            .map_err(|e| crate::error::TlispError::Runtime(format!("Failed to read file: {}", e)))
    }
    
    /// Check if a string contains all expected substrings
    pub fn contains_all_substrings(text: &str, substrings: &[&str]) -> bool {
        substrings.iter().all(|substring| text.contains(substring))
    }
    
    /// Generate a unique test name with timestamp
    pub fn generate_test_name(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("{}_{}", prefix, timestamp)
    }
}

#[cfg(test)]
mod comprehensive_tests {
    //! Comprehensive integration tests that test multiple components together
    
    use super::test_utils::*;
    use crate::tlisp::package_manager::PackageManager;
    use crate::tlisp::module_system::ModuleRegistry;
    use crate::tlisp::rust_integration::RustIntegration;
    use crate::tlisp::cross_language_bridge::CrossLanguageBridge;
    
    #[test]
    fn test_full_system_integration() {
        // Create temporary directories
        let temp_dir = create_temp_dir();
        let cache_dir = temp_dir.path().join("cache");
        let config_dir = temp_dir.path().join("config");
        let modules_dir = temp_dir.path().join("modules");
        
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&modules_dir).unwrap();
        
        // Create package manager
        let mut package_manager = PackageManager::new(cache_dir, config_dir);

        // Create module registry
        let mut module_registry = ModuleRegistry::new();
        
        // Create Rust integration
        let mut rust_integration = RustIntegration::new();
        
        // Create cross-language bridge
        let mut bridge = CrossLanguageBridge::new();
        
        // Verify all components are created successfully
        assert_eq!(package_manager.list_installed().len(), 0);
        assert_eq!(module_registry.list_modules().len(), 0);
        assert_eq!(rust_integration.list_modules().len(), 0);
        assert!(bridge.list_languages().contains(&crate::tlisp::ModuleLanguage::Rust));
        
        // Test that the system can handle basic operations
        // This is a smoke test to ensure all components work together
    }
    
    #[test]
    fn test_project_structure_creation() {
        let temp_dir = create_temp_dir();
        let project_name = generate_test_name("test_project");
        
        // Create sample project files
        let _package_toml = create_sample_package_toml(&temp_dir, &project_name, "1.0.0");
        let _main_module = create_sample_tlisp_module(&temp_dir, "main");
        let _lib_module = create_sample_tlisp_module(&temp_dir, "lib");
        
        // Verify project structure
        assert!(verify_files_exist(temp_dir.path(), &["package.toml", "main.tl", "lib.tl"]));
        
        // Verify file contents
        let package_content = read_file_content(&temp_dir.path().join("package.toml")).unwrap();
        assert!(contains_all_substrings(&package_content, &[&project_name, "1.0.0", "MIT"]));
        
        let main_content = read_file_content(&temp_dir.path().join("main.tl")).unwrap();
        assert!(contains_all_substrings(&main_content, &["module main", "hello-world", "factorial"]));
    }
    
    #[test]
    fn test_multi_language_support() {
        let temp_dir = create_temp_dir();
        
        // Create files for different languages
        let _tlisp_file = create_sample_tlisp_module(&temp_dir, "tlisp_module");
        let _rust_file = create_sample_rust_module_source(&temp_dir);
        
        // Verify files exist
        assert!(verify_files_exist(temp_dir.path(), &["tlisp_module.tl", "rust_module.rs"]));
        
        // Count files by extension
        assert_eq!(count_files_with_extension(temp_dir.path(), "tl"), 1);
        assert_eq!(count_files_with_extension(temp_dir.path(), "rs"), 1);
    }
    
    #[test]
    fn test_error_handling_across_components() {
        let temp_dir = create_temp_dir();
        let cache_dir = temp_dir.path().join("cache");
        let config_dir = temp_dir.path().join("config");
        
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        
        let mut package_manager = PackageManager::new(cache_dir, config_dir);
        
        // Test error handling for nonexistent packages
        let result = package_manager.uninstall("nonexistent_package");
        assert!(result.is_err());

        let package = package_manager.get_package_info("nonexistent_package");
        assert!(package.is_none());
        
        // Test that the system remains stable after errors
        assert_eq!(package_manager.list_installed().len(), 0);
    }
}
