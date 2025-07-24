//! Comprehensive unit tests for the TLISP package manager

use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

use crate::tlisp::package_manager::{
    PackageManager, PackageMetadata, VersionRequirement, InstallOptions, BuildConfig
};
use crate::tlisp::module_system::ModuleLanguage;


/// Test helper to create a temporary package manager
fn create_test_package_manager() -> (PackageManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let config_dir = temp_dir.path().join("config");

    fs::create_dir_all(&cache_dir).unwrap();
    fs::create_dir_all(&config_dir).unwrap();

    let manager = PackageManager::new(cache_dir, config_dir);
    (manager, temp_dir)
}

/// Test helper to create sample package metadata
fn create_sample_package(name: &str, version: &str) -> PackageMetadata {
    let mut metadata = PackageMetadata::new(name.to_string(), version.to_string());
    metadata.description = Some(format!("Test package {}", name));
    metadata.author = Some("Test Author".to_string());
    metadata.license = Some("MIT".to_string());
    metadata.keywords = vec!["test".to_string(), "sample".to_string()];
    metadata.categories = vec!["testing".to_string()];
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_creation() {
        let (manager, _temp_dir) = create_test_package_manager();
        assert_eq!(manager.list_installed().len(), 0);
    }

    #[test]
    fn test_package_metadata_creation() {
        let metadata = create_sample_package("test-package", "1.0.0");
        
        assert_eq!(metadata.name, "test-package");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, Some("Test package test-package".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.license, Some("MIT".to_string()));
        assert_eq!(metadata.keywords, vec!["test", "sample"]);
        assert_eq!(metadata.categories, vec!["testing"]);
        assert_eq!(metadata.language, ModuleLanguage::TLisp);
    }

    #[test]
    fn test_version_requirement_creation() {
        let req = VersionRequirement::new("1.0.0".to_string());
        assert_eq!(req.constraint, "1.0.0");
        assert!(!req.optional);
        assert!(req.default_features);
        assert!(req.features.is_empty());
    }

    #[test]
    fn test_version_requirement_with_features() {
        let mut req = VersionRequirement::new("^1.0.0".to_string());
        req.features = vec!["feature1".to_string(), "feature2".to_string()];
        req.optional = true;
        req.default_features = false;
        
        assert_eq!(req.constraint, "^1.0.0");
        assert!(req.optional);
        assert!(!req.default_features);
        assert_eq!(req.features, vec!["feature1", "feature2"]);
    }

    #[test]
    fn test_install_options_default() {
        let options = InstallOptions::new();
        assert!(!options.force);
        assert!(!options.optional);
    }

    #[test]
    fn test_install_options_custom() {
        let options = InstallOptions {
            force: true,
            optional: true,
            dev: false,
            global: false,
            features: Vec::new(),
        };
        assert!(options.force);
        assert!(options.optional);
        assert!(!options.dev);
        assert!(!options.global);
    }

    #[test]
    fn test_package_manager_install_options() {
        let options = InstallOptions::new()
            .with_dev()
            .global()
            .force()
            .with_optional()
            .with_feature("test-feature".to_string());

        assert!(options.dev);
        assert!(options.global);
        assert!(options.force);
        assert!(options.optional);
        assert!(options.features.contains(&"test-feature".to_string()));
    }

    #[test]
    fn test_package_manager_uninstall() {
        let (mut manager, _temp_dir) = create_test_package_manager();

        // Try to uninstall non-existent package
        let result = manager.uninstall("nonexistent");
        assert!(result.is_err());

        // Check that we can call list_installed
        let installed = manager.list_installed();
        assert_eq!(installed.len(), 0);
    }

    #[test]
    fn test_package_manager_basic_operations() {
        let (manager, _temp_dir) = create_test_package_manager();

        // Test basic functionality that exists
        assert_eq!(manager.list_installed().len(), 0);
        assert!(manager.get_package_info("nonexistent").is_none());
        assert!(!manager.is_installed("nonexistent"));
    }



    #[test]
    fn test_package_manager_update_cache() {
        let (mut manager, _temp_dir) = create_test_package_manager();

        // Test update cache functionality
        let result = manager.update_cache();
        assert!(result.is_ok());

        // Test search functionality
        let search_results = manager.search("test");
        assert!(search_results.is_ok());
    }

    #[test]
    fn test_package_manager_package_info() {
        let (manager, _temp_dir) = create_test_package_manager();

        // Test getting package info for non-existent package
        let info = manager.get_package_info("nonexistent");
        assert!(info.is_none());

        // Test checking if package is installed
        let is_installed = manager.is_installed("nonexistent");
        assert!(!is_installed);
    }

    #[test]
    fn test_build_config_creation() {
        let build_config = BuildConfig {
            script: Some("build.sh".to_string()),
            dependencies: HashMap::new(),
            features: vec!["feature1".to_string()],
            target_dir: Some("target".to_string()),
        };

        assert_eq!(build_config.script, Some("build.sh".to_string()));
        assert_eq!(build_config.features, vec!["feature1"]);
        assert_eq!(build_config.target_dir, Some("target".to_string()));
        assert_eq!(build_config.dependencies.len(), 0);
    }

    #[test]
    fn test_package_with_dependencies() {
        let mut metadata = create_sample_package("main-package", "1.0.0");
        
        // Add dependencies
        metadata.dependencies.insert(
            "dep1".to_string(),
            VersionRequirement::new("^1.0.0".to_string())
        );
        metadata.dependencies.insert(
            "dep2".to_string(),
            VersionRequirement::new("~2.1.0".to_string())
        );
        
        // Add dev dependencies
        metadata.dev_dependencies.insert(
            "test-dep".to_string(),
            VersionRequirement::new("*".to_string())
        );
        
        assert_eq!(metadata.dependencies.len(), 2);
        assert_eq!(metadata.dev_dependencies.len(), 1);
        assert!(metadata.dependencies.contains_key("dep1"));
        assert!(metadata.dependencies.contains_key("dep2"));
        assert!(metadata.dev_dependencies.contains_key("test-dep"));
    }

    #[test]
    fn test_package_with_features() {
        let mut metadata = create_sample_package("feature-package", "1.0.0");
        
        // Add features
        metadata.features.insert(
            "default".to_string(),
            vec!["feature1".to_string(), "feature2".to_string()]
        );
        metadata.features.insert(
            "extra".to_string(),
            vec!["feature3".to_string()]
        );
        
        assert_eq!(metadata.features.len(), 2);
        assert!(metadata.features.contains_key("default"));
        assert!(metadata.features.contains_key("extra"));
        assert_eq!(metadata.features["default"], vec!["feature1", "feature2"]);
        assert_eq!(metadata.features["extra"], vec!["feature3"]);
    }
}
