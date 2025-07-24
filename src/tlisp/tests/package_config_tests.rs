//! Comprehensive unit tests for the TLISP package configuration system

use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::tlisp::package_config::{
    ProjectConfig, ProjectConfigManager, DependencySpec, DetailedDependency,
    BuildConfig, BinaryTarget, LibraryConfig
};


/// Test helper to create a temporary project directory
fn create_test_project() -> (ProjectConfigManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let manager = ProjectConfigManager::new(project_root);
    (manager, temp_dir)
}

/// Test helper to create sample project config
fn create_sample_project_config(name: &str, version: &str) -> ProjectConfig {
    let mut config = ProjectConfig::new(name.to_string(), version.to_string());
    
    config.package.description = Some("A test project".to_string());
    config.package.authors = vec!["Test Author <test@example.com>".to_string()];
    config.package.license = Some("MIT".to_string());
    config.package.homepage = Some("https://example.com".to_string());
    config.package.repository = Some("https://github.com/example/test".to_string());
    config.package.keywords = vec!["test".to_string(), "sample".to_string()];
    config.package.categories = vec!["testing".to_string()];
    
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_config_creation() {
        let config = ProjectConfig::new("test-project".to_string(), "1.0.0".to_string());
        
        assert_eq!(config.package.name, "test-project");
        assert_eq!(config.package.version, "1.0.0");
        assert_eq!(config.package.edition, "2024");
        assert!(config.dependencies.is_empty());
        assert!(config.dev_dependencies.is_empty());
        assert!(config.features.is_empty());
        assert!(config.build.is_none());
        assert!(config.workspace.is_none());
    }

    #[test]
    fn test_project_config_manager_creation() {
        let (manager, _temp_dir) = create_test_project();

        assert!(!manager.is_initialized());
        assert!(manager.config().is_none());
    }

    #[test]
    fn test_project_config_manager_init_project() {
        let (mut manager, _temp_dir) = create_test_project();
        
        let result = manager.init_project("test-project".to_string(), "1.0.0".to_string(), None);
        assert!(result.is_ok());
        
        assert!(manager.is_initialized());
        assert!(manager.config().is_some());
        
        let config = manager.config().unwrap();
        assert_eq!(config.package.name, "test-project");
        assert_eq!(config.package.version, "1.0.0");
    }

    #[test]
    fn test_project_config_manager_init_project_with_template() {
        let (mut manager, _temp_dir) = create_test_project();
        
        let result = manager.init_project("test-lib".to_string(), "1.0.0".to_string(), Some("lib"));
        assert!(result.is_ok());
        
        let config = manager.config().unwrap();
        assert!(config.package.lib.is_some());
        
        let lib_config = config.package.lib.as_ref().unwrap();
        assert_eq!(lib_config.name, Some("test-lib".to_string()));
        assert_eq!(lib_config.path, Some(PathBuf::from("src/lib.tl")));
    }

    #[test]
    fn test_project_config_manager_init_project_bin_template() {
        let (mut manager, _temp_dir) = create_test_project();
        
        let result = manager.init_project("test-bin".to_string(), "1.0.0".to_string(), Some("bin"));
        assert!(result.is_ok());
        
        let config = manager.config().unwrap();
        assert_eq!(config.package.bin.len(), 1);
        
        let bin_config = &config.package.bin[0];
        assert_eq!(bin_config.name, "test-bin");
        assert_eq!(bin_config.path, Some(PathBuf::from("src/main.tl")));
    }

    #[test]
    fn test_project_config_manager_init_project_already_exists() {
        let (mut manager, _temp_dir) = create_test_project();
        
        // Initialize project first time
        let result1 = manager.init_project("test-project".to_string(), "1.0.0".to_string(), None);
        assert!(result1.is_ok());
        
        // Try to initialize again
        let result2 = manager.init_project("test-project".to_string(), "1.0.0".to_string(), None);
        assert!(result2.is_err());
    }

    #[test]
    fn test_project_config_save_and_load() {
        let (mut manager, _temp_dir) = create_test_project();
        
        // Initialize and save
        manager.init_project("test-project".to_string(), "1.0.0".to_string(), None).unwrap();
        let result = manager.save();
        assert!(result.is_ok());
        
        // Create new manager and load
        let project_root = manager.project_root().to_path_buf();
        let mut new_manager = ProjectConfigManager::new(project_root);
        
        let result = new_manager.load();
        assert!(result.is_ok());
        
        let config = new_manager.config().unwrap();
        assert_eq!(config.package.name, "test-project");
        assert_eq!(config.package.version, "1.0.0");
    }

    #[test]
    fn test_project_config_load_or_create() {
        let (mut manager, _temp_dir) = create_test_project();
        
        // Should create new config since none exists
        let result = manager.load_or_create("new-project".to_string(), "1.0.0".to_string());
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert_eq!(config.package.name, "new-project");
        assert_eq!(config.package.version, "1.0.0");
        
        // Should load existing config
        let result2 = manager.load_or_create("different-name".to_string(), "2.0.0".to_string());
        assert!(result2.is_ok());
        
        let config2 = result2.unwrap();
        // Should still have original name and version
        assert_eq!(config2.package.name, "new-project");
        assert_eq!(config2.package.version, "1.0.0");
    }

    #[test]
    fn test_dependency_spec_simple() {
        let spec = DependencySpec::Simple("1.0.0".to_string());
        
        match spec {
            DependencySpec::Simple(version) => assert_eq!(version, "1.0.0"),
            _ => panic!("Expected simple dependency spec"),
        }
    }

    #[test]
    fn test_dependency_spec_detailed() {
        let detailed = DetailedDependency {
            version: Some("^1.0.0".to_string()),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            path: None,
            registry: None,
            features: vec!["feature1".to_string()],
            default_features: false,
            optional: true,
            package: Some("different-name".to_string()),
        };
        
        let spec = DependencySpec::Detailed(detailed.clone());
        
        match spec {
            DependencySpec::Detailed(dep) => {
                assert_eq!(dep.version, Some("^1.0.0".to_string()));
                assert_eq!(dep.features, vec!["feature1"]);
                assert!(!dep.default_features);
                assert!(dep.optional);
                assert_eq!(dep.package, Some("different-name".to_string()));
            }
            _ => panic!("Expected detailed dependency spec"),
        }
    }

    #[test]
    fn test_project_config_add_dependency() {
        let mut config = create_sample_project_config("test-project", "1.0.0");
        
        config.add_dependency("dep1".to_string(), DependencySpec::Simple("1.0.0".to_string()));
        config.add_dev_dependency("test-dep".to_string(), DependencySpec::Simple("^2.0.0".to_string()));
        config.add_optional_dependency("opt-dep".to_string(), DependencySpec::Simple("~1.5.0".to_string()));
        
        assert_eq!(config.dependencies.len(), 1);
        assert_eq!(config.dev_dependencies.len(), 1);
        assert_eq!(config.optional_dependencies.len(), 1);
        
        assert!(config.dependencies.contains_key("dep1"));
        assert!(config.dev_dependencies.contains_key("test-dep"));
        assert!(config.optional_dependencies.contains_key("opt-dep"));
    }

    #[test]
    fn test_project_config_add_feature() {
        let mut config = create_sample_project_config("test-project", "1.0.0");
        
        config.add_feature("default".to_string(), vec!["feature1".to_string(), "feature2".to_string()]);
        config.add_feature("extra".to_string(), vec!["feature3".to_string()]);
        
        assert_eq!(config.features.len(), 2);
        assert!(config.has_feature("default"));
        assert!(config.has_feature("extra"));
        assert!(!config.has_feature("nonexistent"));
        
        let default_deps = config.feature_dependencies("default");
        assert_eq!(default_deps.len(), 2);
        assert!(default_deps.contains(&&"feature1".to_string()));
        assert!(default_deps.contains(&&"feature2".to_string()));
    }

    #[test]
    fn test_project_config_add_script() {
        let mut config = create_sample_project_config("test-project", "1.0.0");
        
        config.add_script("build".to_string(), "tlisp build --release".to_string());
        config.add_script("test".to_string(), "tlisp test".to_string());
        
        assert_eq!(config.scripts.len(), 2);
        assert!(config.scripts.contains_key("build"));
        assert!(config.scripts.contains_key("test"));
        assert_eq!(config.scripts["build"], "tlisp build --release");
        assert_eq!(config.scripts["test"], "tlisp test");
    }

    #[test]
    fn test_project_config_add_env() {
        let mut config = create_sample_project_config("test-project", "1.0.0");
        
        config.add_env("RUST_LOG".to_string(), "debug".to_string());
        config.add_env("TLISP_DEBUG".to_string(), "1".to_string());
        
        assert_eq!(config.env.len(), 2);
        assert!(config.env.contains_key("RUST_LOG"));
        assert!(config.env.contains_key("TLISP_DEBUG"));
        assert_eq!(config.env["RUST_LOG"], "debug");
        assert_eq!(config.env["TLISP_DEBUG"], "1");
    }

    #[test]
    fn test_project_config_all_dependencies() {
        let mut config = create_sample_project_config("test-project", "1.0.0");
        
        config.add_dependency("dep1".to_string(), DependencySpec::Simple("1.0.0".to_string()));
        config.add_dev_dependency("test-dep".to_string(), DependencySpec::Simple("^2.0.0".to_string()));
        config.add_optional_dependency("opt-dep".to_string(), DependencySpec::Simple("~1.5.0".to_string()));
        
        let all_deps = config.all_dependencies();
        assert_eq!(all_deps.len(), 3);
        assert!(all_deps.contains_key("dep1"));
        assert!(all_deps.contains_key("test-dep"));
        assert!(all_deps.contains_key("opt-dep"));
    }

    #[test]
    fn test_project_config_validation() {
        let config = create_sample_project_config("test-project", "1.0.0");
        
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_project_config_validation_empty_name() {
        let config = ProjectConfig::new("".to_string(), "1.0.0".to_string());
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_project_config_validation_empty_version() {
        let config = ProjectConfig::new("test-project".to_string(), "".to_string());
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_project_config_validation_invalid_dependency() {
        let mut config = create_sample_project_config("test-project", "1.0.0");
        
        // Add dependency with empty name
        config.dependencies.insert("".to_string(), DependencySpec::Simple("1.0.0".to_string()));
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_project_config_validation_invalid_feature() {
        let mut config = create_sample_project_config("test-project", "1.0.0");
        
        // Add feature that references non-existent dependency
        config.add_feature("test-feature".to_string(), vec!["nonexistent-dep".to_string()]);
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_build_config_creation() {
        let build_config = BuildConfig {
            script: Some("build.sh".to_string()),
            dependencies: HashMap::new(),
            features: vec!["build-feature".to_string()],
            target_dir: Some(PathBuf::from("target")),
            incremental: true,
            opt_level: Some("3".to_string()),
            debug: Some(false),
            lto: Some(true),
            codegen_units: Some(1),
            panic: Some("abort".to_string()),
        };
        
        assert_eq!(build_config.script, Some("build.sh".to_string()));
        assert_eq!(build_config.features, vec!["build-feature"]);
        assert_eq!(build_config.target_dir, Some(PathBuf::from("target")));
        assert!(build_config.incremental);
        assert_eq!(build_config.opt_level, Some("3".to_string()));
        assert_eq!(build_config.debug, Some(false));
        assert_eq!(build_config.lto, Some(true));
        assert_eq!(build_config.codegen_units, Some(1));
        assert_eq!(build_config.panic, Some("abort".to_string()));
    }

    #[test]
    fn test_binary_target_creation() {
        let binary = BinaryTarget {
            name: "my-binary".to_string(),
            path: Some(PathBuf::from("src/bin/my-binary.tl")),
            required_features: vec!["cli".to_string()],
        };
        
        assert_eq!(binary.name, "my-binary");
        assert_eq!(binary.path, Some(PathBuf::from("src/bin/my-binary.tl")));
        assert_eq!(binary.required_features, vec!["cli"]);
    }

    #[test]
    fn test_library_config_creation() {
        let library = LibraryConfig {
            name: Some("my-lib".to_string()),
            path: Some(PathBuf::from("src/lib.tl")),
            crate_type: vec!["tlisp".to_string(), "staticlib".to_string()],
            required_features: vec!["std".to_string()],
        };
        
        assert_eq!(library.name, Some("my-lib".to_string()));
        assert_eq!(library.path, Some(PathBuf::from("src/lib.tl")));
        assert_eq!(library.crate_type, vec!["tlisp", "staticlib"]);
        assert_eq!(library.required_features, vec!["std"]);
    }

    #[test]
    fn test_project_config_to_package_metadata() {
        let config = create_sample_project_config("test-project", "1.0.0");
        let metadata = config.to_package_metadata();
        
        assert_eq!(metadata.name, "test-project");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, Some("A test project".to_string()));
        assert_eq!(metadata.author, Some("Test Author <test@example.com>".to_string()));
        assert_eq!(metadata.license, Some("MIT".to_string()));
        assert_eq!(metadata.homepage, Some("https://example.com".to_string()));
        assert_eq!(metadata.repository, Some("https://github.com/example/test".to_string()));
        assert_eq!(metadata.keywords, vec!["test", "sample"]);
        assert_eq!(metadata.language, crate::tlisp::ModuleLanguage::TLisp);
    }
}
