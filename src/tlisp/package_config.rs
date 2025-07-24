//! Package Configuration System for TLISP
//! 
//! Provides package.toml configuration files for TLISP projects with
//! dependency management, build configuration, and project metadata.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use crate::error::{TlispError, TlispResult};
use crate::tlisp::package_manager::{VersionRequirement, PackageMetadata};
use crate::tlisp::ModuleLanguage;

/// TLISP project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Package metadata
    pub package: PackageInfo,
    /// Dependencies
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    /// Development dependencies
    #[serde(default)]
    pub dev_dependencies: HashMap<String, DependencySpec>,
    /// Build dependencies
    #[serde(default)]
    pub build_dependencies: HashMap<String, DependencySpec>,
    /// Optional dependencies
    #[serde(default)]
    pub optional_dependencies: HashMap<String, DependencySpec>,
    /// Features
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
    /// Build configuration
    #[serde(default)]
    pub build: Option<BuildConfig>,
    /// Target configurations
    #[serde(default)]
    pub target: HashMap<String, TargetConfig>,
    /// Workspace configuration
    #[serde(default)]
    pub workspace: Option<WorkspaceConfig>,
    /// Scripts
    #[serde(default)]
    pub scripts: HashMap<String, String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Package authors
    #[serde(default)]
    pub authors: Vec<String>,
    /// Package license
    pub license: Option<String>,
    /// Package homepage
    pub homepage: Option<String>,
    /// Package repository
    pub repository: Option<String>,
    /// Package documentation URL
    pub documentation: Option<String>,
    /// Package keywords
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Package categories
    #[serde(default)]
    pub categories: Vec<String>,
    /// Package edition (for compatibility)
    #[serde(default = "default_edition")]
    pub edition: String,
    /// Main entry point
    pub main: Option<String>,
    /// Binary targets
    #[serde(default)]
    pub bin: Vec<BinaryTarget>,
    /// Library configuration
    pub lib: Option<LibraryConfig>,
    /// Example targets
    #[serde(default)]
    pub examples: Vec<ExampleTarget>,
    /// Test targets
    #[serde(default)]
    pub tests: Vec<TestTarget>,
    /// Benchmark targets
    #[serde(default)]
    pub benches: Vec<BenchTarget>,
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string
    Simple(String),
    /// Detailed dependency specification
    Detailed(DetailedDependency),
}

/// Detailed dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDependency {
    /// Version requirement
    pub version: Option<String>,
    /// Git repository
    pub git: Option<String>,
    /// Git branch
    pub branch: Option<String>,
    /// Git tag
    pub tag: Option<String>,
    /// Git revision
    pub rev: Option<String>,
    /// Local path
    pub path: Option<PathBuf>,
    /// Registry
    pub registry: Option<String>,
    /// Features to enable
    #[serde(default)]
    pub features: Vec<String>,
    /// Whether to use default features
    #[serde(default = "default_true")]
    pub default_features: bool,
    /// Whether dependency is optional
    #[serde(default)]
    pub optional: bool,
    /// Package name (if different from dependency name)
    pub package: Option<String>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Build script
    pub script: Option<String>,
    /// Build dependencies
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    /// Build features
    #[serde(default)]
    pub features: Vec<String>,
    /// Target directory
    pub target_dir: Option<PathBuf>,
    /// Incremental compilation
    #[serde(default = "default_true")]
    pub incremental: bool,
    /// Optimization level
    pub opt_level: Option<String>,
    /// Debug information
    pub debug: Option<bool>,
    /// Link-time optimization
    pub lto: Option<bool>,
    /// Code generation units
    pub codegen_units: Option<u32>,
    /// Panic strategy
    pub panic: Option<String>,
}

/// Target-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Target-specific dependencies
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    /// Target-specific build configuration
    pub build: Option<BuildConfig>,
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace members
    pub members: Vec<String>,
    /// Excluded members
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Default members
    #[serde(default)]
    pub default_members: Vec<String>,
    /// Workspace dependencies
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    /// Workspace metadata
    #[serde(default)]
    pub metadata: HashMap<String, toml::Value>,
}

/// Binary target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryTarget {
    /// Binary name
    pub name: String,
    /// Source path
    pub path: Option<PathBuf>,
    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,
}

/// Library configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    /// Library name
    pub name: Option<String>,
    /// Source path
    pub path: Option<PathBuf>,
    /// Crate types
    #[serde(default)]
    pub crate_type: Vec<String>,
    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,
}

/// Example target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleTarget {
    /// Example name
    pub name: String,
    /// Source path
    pub path: Option<PathBuf>,
    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,
}

/// Test target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTarget {
    /// Test name
    pub name: String,
    /// Source path
    pub path: Option<PathBuf>,
    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,
    /// Test harness
    #[serde(default = "default_true")]
    pub harness: bool,
}

/// Benchmark target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchTarget {
    /// Benchmark name
    pub name: String,
    /// Source path
    pub path: Option<PathBuf>,
    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,
    /// Benchmark harness
    #[serde(default = "default_true")]
    pub harness: bool,
}

/// Project configuration manager
pub struct ProjectConfigManager {
    /// Current project configuration
    config: Option<ProjectConfig>,
    /// Configuration file path
    config_path: Option<PathBuf>,
    /// Project root directory
    project_root: PathBuf,
}

// Default value functions for serde
fn default_edition() -> String {
    "2024".to_string()
}

fn default_true() -> bool {
    true
}

impl ProjectConfig {
    /// Create a new project configuration
    pub fn new(name: String, version: String) -> Self {
        ProjectConfig {
            package: PackageInfo {
                name,
                version,
                description: None,
                authors: Vec::new(),
                license: None,
                homepage: None,
                repository: None,
                documentation: None,
                keywords: Vec::new(),
                categories: Vec::new(),
                edition: default_edition(),
                main: None,
                bin: Vec::new(),
                lib: None,
                examples: Vec::new(),
                tests: Vec::new(),
                benches: Vec::new(),
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build_dependencies: HashMap::new(),
            optional_dependencies: HashMap::new(),
            features: HashMap::new(),
            build: None,
            target: HashMap::new(),
            workspace: None,
            scripts: HashMap::new(),
            env: HashMap::new(),
        }
    }

    /// Load project configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> TlispResult<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| TlispError::Runtime(format!("Failed to read package.toml: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| TlispError::Runtime(format!("Failed to parse package.toml: {}", e)))
    }

    /// Save project configuration to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> TlispResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| TlispError::Runtime(format!("Failed to serialize package.toml: {}", e)))?;
        
        fs::write(path, content)
            .map_err(|e| TlispError::Runtime(format!("Failed to write package.toml: {}", e)))
    }

    /// Convert to PackageMetadata for package manager
    pub fn to_package_metadata(&self) -> PackageMetadata {
        let mut metadata = PackageMetadata::new(
            self.package.name.clone(),
            self.package.version.clone(),
        );

        metadata.description = self.package.description.clone();
        metadata.author = self.package.authors.first().cloned();
        metadata.license = self.package.license.clone();
        metadata.homepage = self.package.homepage.clone();
        metadata.repository = self.package.repository.clone();
        metadata.keywords = self.package.keywords.clone();
        metadata.language = ModuleLanguage::TLisp;
        metadata.main = self.package.main.clone();

        // Convert dependencies
        for (name, spec) in &self.dependencies {
            let version_req = self.dependency_spec_to_version_requirement(spec);
            metadata.dependencies.insert(name.clone(), version_req);
        }

        for (name, spec) in &self.dev_dependencies {
            let version_req = self.dependency_spec_to_version_requirement(spec);
            metadata.dev_dependencies.insert(name.clone(), version_req);
        }

        for (name, spec) in &self.optional_dependencies {
            let mut version_req = self.dependency_spec_to_version_requirement(spec);
            version_req.optional = true;
            metadata.optional_dependencies.insert(name.clone(), version_req);
        }

        // Convert features
        metadata.features = self.features.clone();

        metadata
    }

    /// Convert dependency spec to version requirement
    fn dependency_spec_to_version_requirement(&self, spec: &DependencySpec) -> VersionRequirement {
        match spec {
            DependencySpec::Simple(version) => VersionRequirement::new(version.clone()),
            DependencySpec::Detailed(detailed) => {
                let constraint = detailed.version.clone().unwrap_or("*".to_string());
                let mut req = VersionRequirement::new(constraint);
                req.features = detailed.features.clone();
                req.optional = detailed.optional;
                req.default_features = detailed.default_features;
                req
            }
        }
    }

    /// Add dependency
    pub fn add_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dependencies.insert(name, spec);
    }

    /// Add development dependency
    pub fn add_dev_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dev_dependencies.insert(name, spec);
    }

    /// Add optional dependency
    pub fn add_optional_dependency(&mut self, name: String, spec: DependencySpec) {
        self.optional_dependencies.insert(name, spec);
    }

    /// Add feature
    pub fn add_feature(&mut self, name: String, dependencies: Vec<String>) {
        self.features.insert(name, dependencies);
    }

    /// Add script
    pub fn add_script(&mut self, name: String, command: String) {
        self.scripts.insert(name, command);
    }

    /// Add environment variable
    pub fn add_env(&mut self, name: String, value: String) {
        self.env.insert(name, value);
    }

    /// Get all dependencies (including dev and optional)
    pub fn all_dependencies(&self) -> HashMap<String, &DependencySpec> {
        let mut deps = HashMap::new();

        for (name, spec) in &self.dependencies {
            deps.insert(name.clone(), spec);
        }

        for (name, spec) in &self.dev_dependencies {
            deps.insert(name.clone(), spec);
        }

        for (name, spec) in &self.optional_dependencies {
            deps.insert(name.clone(), spec);
        }

        deps
    }

    /// Check if package has feature
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.contains_key(feature)
    }

    /// Get feature dependencies
    pub fn feature_dependencies(&self, feature: &str) -> Vec<&String> {
        self.features.get(feature).map(|deps| deps.iter().collect()).unwrap_or_default()
    }

    /// Validate configuration
    pub fn validate(&self) -> TlispResult<()> {
        // Validate package name
        if self.package.name.is_empty() {
            return Err(TlispError::Runtime("Package name cannot be empty".to_string()));
        }

        // Validate version
        if self.package.version.is_empty() {
            return Err(TlispError::Runtime("Package version cannot be empty".to_string()));
        }

        // Validate dependencies
        for (name, spec) in &self.dependencies {
            self.validate_dependency(name, spec)?;
        }

        for (name, spec) in &self.dev_dependencies {
            self.validate_dependency(name, spec)?;
        }

        for (name, spec) in &self.optional_dependencies {
            self.validate_dependency(name, spec)?;
        }

        // Validate features
        for (feature_name, feature_deps) in &self.features {
            for dep in feature_deps {
                if !dep.contains('/') && !self.dependencies.contains_key(dep) && !self.optional_dependencies.contains_key(dep) {
                    return Err(TlispError::Runtime(
                        format!("Feature '{}' references unknown dependency '{}'", feature_name, dep)
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate a single dependency
    fn validate_dependency(&self, name: &str, spec: &DependencySpec) -> TlispResult<()> {
        if name.is_empty() {
            return Err(TlispError::Runtime("Dependency name cannot be empty".to_string()));
        }

        match spec {
            DependencySpec::Simple(version) => {
                if version.is_empty() {
                    return Err(TlispError::Runtime(
                        format!("Version for dependency '{}' cannot be empty", name)
                    ));
                }
            }
            DependencySpec::Detailed(detailed) => {
                let source_count = [
                    detailed.version.is_some(),
                    detailed.git.is_some(),
                    detailed.path.is_some(),
                ].iter().filter(|&&x| x).count();

                if source_count == 0 {
                    return Err(TlispError::Runtime(
                        format!("Dependency '{}' must specify version, git, or path", name)
                    ));
                }

                if source_count > 1 {
                    return Err(TlispError::Runtime(
                        format!("Dependency '{}' can only specify one of version, git, or path", name)
                    ));
                }

                // Validate git-specific fields
                if detailed.git.is_some() {
                    let git_ref_count = [
                        detailed.branch.is_some(),
                        detailed.tag.is_some(),
                        detailed.rev.is_some(),
                    ].iter().filter(|&&x| x).count();

                    if git_ref_count > 1 {
                        return Err(TlispError::Runtime(
                            format!("Dependency '{}' can only specify one of branch, tag, or rev", name)
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

impl ProjectConfigManager {
    /// Create new project config manager
    pub fn new(project_root: PathBuf) -> Self {
        ProjectConfigManager {
            config: None,
            config_path: None,
            project_root,
        }
    }

    /// Load project configuration
    pub fn load(&mut self) -> TlispResult<&ProjectConfig> {
        let config_path = self.project_root.join("package.toml");

        if !config_path.exists() {
            return Err(TlispError::Runtime("No package.toml found in project root".to_string()));
        }

        let config = ProjectConfig::from_file(&config_path)?;
        config.validate()?;

        self.config_path = Some(config_path);
        self.config = Some(config);

        Ok(self.config.as_ref().unwrap())
    }

    /// Load or create project configuration
    pub fn load_or_create(&mut self, name: String, version: String) -> TlispResult<&ProjectConfig> {
        let config_path = self.project_root.join("package.toml");

        if config_path.exists() {
            self.load()
        } else {
            let config = ProjectConfig::new(name, version);
            config.to_file(&config_path)?;

            self.config_path = Some(config_path);
            self.config = Some(config);

            Ok(self.config.as_ref().unwrap())
        }
    }

    /// Save current configuration
    pub fn save(&self) -> TlispResult<()> {
        if let (Some(config), Some(path)) = (&self.config, &self.config_path) {
            config.validate()?;
            config.to_file(path)
        } else {
            Err(TlispError::Runtime("No configuration loaded".to_string()))
        }
    }

    /// Get current configuration
    pub fn config(&self) -> Option<&ProjectConfig> {
        self.config.as_ref()
    }

    /// Get mutable configuration
    pub fn config_mut(&mut self) -> Option<&mut ProjectConfig> {
        self.config.as_mut()
    }

    /// Initialize new project
    pub fn init_project(&mut self, name: String, version: String, template: Option<&str>) -> TlispResult<()> {
        let config_path = self.project_root.join("package.toml");

        if config_path.exists() {
            return Err(TlispError::Runtime("Project already initialized (package.toml exists)".to_string()));
        }

        let mut config = ProjectConfig::new(name.clone(), version);

        // Apply template if specified
        if let Some(template_name) = template {
            self.apply_template(&mut config, template_name)?;
        }

        // Create directory structure
        self.create_project_structure(&config)?;

        // Save configuration
        config.to_file(&config_path)?;

        self.config_path = Some(config_path);
        self.config = Some(config);

        Ok(())
    }

    /// Apply project template
    fn apply_template(&self, config: &mut ProjectConfig, template: &str) -> TlispResult<()> {
        match template {
            "lib" => {
                config.package.lib = Some(LibraryConfig {
                    name: Some(config.package.name.clone()),
                    path: Some(PathBuf::from("src/lib.tl")),
                    crate_type: vec!["tlisp".to_string()],
                    required_features: Vec::new(),
                });
            }
            "bin" => {
                config.package.bin.push(BinaryTarget {
                    name: config.package.name.clone(),
                    path: Some(PathBuf::from("src/main.tl")),
                    required_features: Vec::new(),
                });
            }
            "workspace" => {
                config.workspace = Some(WorkspaceConfig {
                    members: vec!["packages/*".to_string()],
                    exclude: Vec::new(),
                    default_members: Vec::new(),
                    dependencies: HashMap::new(),
                    metadata: HashMap::new(),
                });
            }
            _ => {
                return Err(TlispError::Runtime(
                    format!("Unknown template: {}", template)
                ));
            }
        }

        Ok(())
    }

    /// Create project directory structure
    fn create_project_structure(&self, config: &ProjectConfig) -> TlispResult<()> {
        // Create src directory
        let src_dir = self.project_root.join("src");
        fs::create_dir_all(&src_dir)
            .map_err(|e| TlispError::Runtime(format!("Failed to create src directory: {}", e)))?;

        // Create main files based on configuration
        if config.package.lib.is_some() {
            let lib_file = src_dir.join("lib.tl");
            if !lib_file.exists() {
                fs::write(&lib_file, self.generate_lib_template(&config.package.name))
                    .map_err(|e| TlispError::Runtime(format!("Failed to create lib.tl: {}", e)))?;
            }
        }

        if !config.package.bin.is_empty() {
            let main_file = src_dir.join("main.tl");
            if !main_file.exists() {
                fs::write(&main_file, self.generate_main_template(&config.package.name))
                    .map_err(|e| TlispError::Runtime(format!("Failed to create main.tl: {}", e)))?;
            }
        }

        // Create examples directory if there are examples
        if !config.package.examples.is_empty() {
            let examples_dir = self.project_root.join("examples");
            fs::create_dir_all(&examples_dir)
                .map_err(|e| TlispError::Runtime(format!("Failed to create examples directory: {}", e)))?;
        }

        // Create tests directory if there are tests
        if !config.package.tests.is_empty() {
            let tests_dir = self.project_root.join("tests");
            fs::create_dir_all(&tests_dir)
                .map_err(|e| TlispError::Runtime(format!("Failed to create tests directory: {}", e)))?;
        }

        Ok(())
    }

    /// Generate library template
    fn generate_lib_template(&self, name: &str) -> String {
        format!(
            r#";; {} library
;;
;; This is the main library file for the {} package.

(module {}
  "Main library module for {}"

  ;; Export public functions
  (export hello-world)

  ;; Example function
  (defn hello-world []
    "Returns a greeting message"
    "Hello, World from {}!"))
"#,
            name, name, name, name, name
        )
    }

    /// Generate main template
    fn generate_main_template(&self, name: &str) -> String {
        format!(
            r#";; {} main executable
;;
;; This is the main entry point for the {} application.

(module main
  "Main executable module for {}"

  ;; Import dependencies
  ;; (use some-dependency)

  ;; Main function
  (defn main [args]
    "Main entry point"
    (println "Hello, World from {}!")
    0))
"#,
            name, name, name, name
        )
    }

    /// Get project root
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Check if project is initialized
    pub fn is_initialized(&self) -> bool {
        self.project_root.join("package.toml").exists()
    }
}
