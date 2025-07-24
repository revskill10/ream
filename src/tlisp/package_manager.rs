//! Package Manager for TLISP
//! 
//! Comprehensive package management system with dependency resolution,
//! version management, and package registry support.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use crate::error::{TlispError, TlispResult};
use crate::tlisp::ModuleLanguage;
use crate::tlisp::package_registry::PackageRegistry as RegistryImpl;

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Package author
    pub author: Option<String>,
    /// Package license
    pub license: Option<String>,
    /// Package homepage
    pub homepage: Option<String>,
    /// Package repository
    pub repository: Option<String>,
    /// Package keywords
    pub keywords: Vec<String>,
    /// Package categories
    pub categories: Vec<String>,
    /// Package dependencies
    pub dependencies: HashMap<String, VersionRequirement>,
    /// Development dependencies
    pub dev_dependencies: HashMap<String, VersionRequirement>,
    /// Optional dependencies
    pub optional_dependencies: HashMap<String, VersionRequirement>,
    /// Package features
    pub features: HashMap<String, Vec<String>>,
    /// Build configuration
    pub build: Option<BuildConfig>,
    /// Package language
    pub language: ModuleLanguage,
    /// Entry point
    pub main: Option<String>,
    /// Exported modules
    pub exports: Vec<String>,
}

/// Version requirement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRequirement {
    /// Version constraint (e.g., "^1.0.0", ">=2.0.0", "~1.2.3")
    pub constraint: String,
    /// Optional features to enable
    pub features: Vec<String>,
    /// Whether this dependency is optional
    pub optional: bool,
    /// Default features enabled
    pub default_features: bool,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Build script path
    pub script: Option<String>,
    /// Build dependencies
    pub dependencies: HashMap<String, VersionRequirement>,
    /// Build features
    pub features: Vec<String>,
    /// Target directory
    pub target_dir: Option<String>,
}

/// Package registry
#[derive(Debug, Clone)]
pub struct PackageRegistry {
    /// Registry URL
    pub url: String,
    /// Registry name
    pub name: String,
    /// Authentication token
    pub token: Option<String>,
    /// Local cache directory
    pub cache_dir: PathBuf,
}

/// Package manager
pub struct PackageManager {
    /// Package registries
    registries: Vec<PackageRegistry>,
    /// Local package cache
    cache: HashMap<String, PackageMetadata>,
    /// Installed packages
    installed: HashMap<String, InstalledPackage>,
    /// Dependency resolver
    resolver: DependencyResolver,
    /// Package cache directory
    cache_dir: PathBuf,
    /// Global packages directory
    global_dir: PathBuf,
}

/// Installed package information
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    /// Package metadata
    pub metadata: PackageMetadata,
    /// Installation path
    pub path: PathBuf,
    /// Installation time
    pub installed_at: std::time::SystemTime,
    /// Enabled features
    pub features: HashSet<String>,
    /// Direct dependency flag
    pub is_direct: bool,
}

/// Dependency resolver
pub struct DependencyResolver {
    /// Resolution cache
    cache: HashMap<String, Vec<ResolvedDependency>>,
    /// Conflict resolution strategy
    strategy: ConflictResolution,
}

/// Resolved dependency
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// Package name
    pub name: String,
    /// Resolved version
    pub version: String,
    /// Source registry
    pub registry: String,
    /// Enabled features
    pub features: HashSet<String>,
    /// Dependency path
    pub path: Vec<String>,
}

/// Conflict resolution strategy
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    /// Use the highest compatible version
    Highest,
    /// Use the lowest compatible version
    Lowest,
    /// Fail on conflicts
    Strict,
    /// Use user-specified resolution
    Manual(HashMap<String, String>),
}

/// Package installation options
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Features to enable
    pub features: Vec<String>,
    /// Whether to install as development dependency
    pub dev: bool,
    /// Whether to install globally
    pub global: bool,
    /// Whether to force reinstall
    pub force: bool,
    /// Whether to install optional dependencies
    pub optional: bool,
}

impl PackageMetadata {
    /// Create new package metadata
    pub fn new(name: String, version: String) -> Self {
        PackageMetadata {
            name,
            version,
            description: None,
            author: None,
            license: None,
            homepage: None,
            repository: None,
            keywords: Vec::new(),
            categories: Vec::new(),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            optional_dependencies: HashMap::new(),
            features: HashMap::new(),
            build: None,
            language: ModuleLanguage::TLisp,
            main: None,
            exports: Vec::new(),
        }
    }

    /// Load package metadata from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> TlispResult<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| TlispError::Runtime(format!("Failed to read package file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| TlispError::Runtime(format!("Failed to parse package file: {}", e)))
    }

    /// Save package metadata to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> TlispResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| TlispError::Runtime(format!("Failed to serialize package: {}", e)))?;
        
        fs::write(path, content)
            .map_err(|e| TlispError::Runtime(format!("Failed to write package file: {}", e)))
    }

    /// Get all dependencies (including dev and optional)
    pub fn all_dependencies(&self) -> HashMap<String, &VersionRequirement> {
        let mut deps = HashMap::new();
        
        for (name, req) in &self.dependencies {
            deps.insert(name.clone(), req);
        }
        
        for (name, req) in &self.dev_dependencies {
            deps.insert(name.clone(), req);
        }
        
        for (name, req) in &self.optional_dependencies {
            deps.insert(name.clone(), req);
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
}

impl VersionRequirement {
    /// Create new version requirement
    pub fn new(constraint: String) -> Self {
        VersionRequirement {
            constraint,
            features: Vec::new(),
            optional: false,
            default_features: true,
        }
    }

    /// Create exact version requirement
    pub fn exact(version: String) -> Self {
        VersionRequirement::new(format!("={}", version))
    }

    /// Create compatible version requirement (^)
    pub fn compatible(version: String) -> Self {
        VersionRequirement::new(format!("^{}", version))
    }

    /// Create tilde version requirement (~)
    pub fn tilde(version: String) -> Self {
        VersionRequirement::new(format!("~{}", version))
    }

    /// Check if version satisfies requirement
    pub fn satisfies(&self, version: &str) -> bool {
        // Simplified version matching - in a real implementation,
        // this would use a proper semver library
        if self.constraint.starts_with('=') {
            let required = &self.constraint[1..];
            version == required
        } else if self.constraint.starts_with('^') {
            let required = &self.constraint[1..];
            // Compatible version (same major version)
            version.starts_with(&required.split('.').next().unwrap_or(""))
        } else if self.constraint.starts_with('~') {
            let required = &self.constraint[1..];
            // Tilde version (same major.minor)
            let parts: Vec<&str> = required.split('.').collect();
            if parts.len() >= 2 {
                let prefix = format!("{}.{}", parts[0], parts[1]);
                version.starts_with(&prefix)
            } else {
                version.starts_with(required)
            }
        } else if self.constraint.starts_with(">=") {
            // Greater than or equal (simplified)
            true // TODO: Implement proper version comparison
        } else {
            // Default to exact match
            version == self.constraint
        }
    }
}

impl PackageRegistry {
    /// Create new package registry
    pub fn new(name: String, url: String, cache_dir: PathBuf) -> Self {
        PackageRegistry {
            url,
            name,
            token: None,
            cache_dir,
        }
    }

    /// Create default registry
    pub fn default(cache_dir: PathBuf) -> Self {
        PackageRegistry::new(
            "default".to_string(),
            "https://packages.tlisp.org".to_string(),
            cache_dir,
        )
    }

    /// Set authentication token
    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }
}

impl PackageManager {
    /// Create new package manager
    pub fn new(cache_dir: PathBuf, global_dir: PathBuf) -> Self {
        let mut registries = Vec::new();
        registries.push(PackageRegistry::default(cache_dir.clone()));

        PackageManager {
            registries,
            cache: HashMap::new(),
            installed: HashMap::new(),
            resolver: DependencyResolver::new(),
            cache_dir,
            global_dir,
        }
    }

    /// Add package registry
    pub fn add_registry(&mut self, registry: PackageRegistry) {
        self.registries.push(registry);
    }

    /// Install package
    pub fn install(&mut self, name: &str, options: InstallOptions) -> TlispResult<()> {
        // Resolve dependencies
        let resolved = self.resolver.resolve(name, &options, &self.registries)?;

        // Install packages in dependency order
        for dep in resolved {
            self.install_package(&dep, &options)?;
        }

        Ok(())
    }

    /// Install specific package
    fn install_package(&mut self, dep: &ResolvedDependency, options: &InstallOptions) -> TlispResult<()> {
        // Check if already installed
        if let Some(installed) = self.installed.get(&dep.name) {
            if installed.metadata.version == dep.version && !options.force {
                return Ok(());
            }
        }

        // Download package
        let package_path = self.download_package(dep)?;

        // Load package metadata
        let metadata_path = package_path.join("package.toml");
        let metadata = PackageMetadata::from_file(metadata_path)?;

        // Install package
        let install_path = if options.global {
            self.global_dir.join(&dep.name)
        } else {
            self.cache_dir.join("packages").join(&dep.name).join(&dep.version)
        };

        // Copy package files
        self.copy_package(&package_path, &install_path)?;

        // Register installed package
        let installed_package = InstalledPackage {
            metadata,
            path: install_path,
            installed_at: std::time::SystemTime::now(),
            features: dep.features.clone(),
            is_direct: dep.path.len() == 1,
        };

        self.installed.insert(dep.name.clone(), installed_package);

        Ok(())
    }

    /// Download package from registry
    fn download_package(&self, dep: &ResolvedDependency) -> TlispResult<PathBuf> {
        // Find registry
        let registry = self.registries.iter()
            .find(|r| r.name == dep.registry)
            .ok_or_else(|| TlispError::Runtime(format!("Registry {} not found", dep.registry)))?;

        // Create download path
        let download_path = registry.cache_dir
            .join("downloads")
            .join(&dep.name)
            .join(&dep.version);

        // Check if already downloaded
        if download_path.exists() {
            return Ok(download_path);
        }

        // Create directories
        fs::create_dir_all(&download_path)
            .map_err(|e| TlispError::Runtime(format!("Failed to create download directory: {}", e)))?;

        // TODO: Implement actual download from registry
        // For now, create a placeholder
        let package_toml = download_path.join("package.toml");
        let placeholder_metadata = PackageMetadata::new(dep.name.clone(), dep.version.clone());
        placeholder_metadata.to_file(package_toml)?;

        Ok(download_path)
    }

    /// Copy package files
    fn copy_package(&self, src: &Path, dst: &Path) -> TlispResult<()> {
        fs::create_dir_all(dst)
            .map_err(|e| TlispError::Runtime(format!("Failed to create install directory: {}", e)))?;

        // TODO: Implement recursive copy
        // For now, just copy package.toml
        let src_toml = src.join("package.toml");
        let dst_toml = dst.join("package.toml");

        if src_toml.exists() {
            fs::copy(src_toml, dst_toml)
                .map_err(|e| TlispError::Runtime(format!("Failed to copy package file: {}", e)))?;
        }

        Ok(())
    }

    /// Uninstall package
    pub fn uninstall(&mut self, name: &str) -> TlispResult<()> {
        if let Some(installed) = self.installed.remove(name) {
            // Remove package directory
            if installed.path.exists() {
                fs::remove_dir_all(&installed.path)
                    .map_err(|e| TlispError::Runtime(format!("Failed to remove package: {}", e)))?;
            }
            Ok(())
        } else {
            Err(TlispError::Runtime(format!("Package '{}' is not installed", name)))
        }
    }

    /// List installed packages
    pub fn list_installed(&self) -> Vec<&InstalledPackage> {
        self.installed.values().collect()
    }

    /// Search packages in registries
    pub fn search(&self, query: &str) -> TlispResult<Vec<PackageMetadata>> {
        let mut results = Vec::new();

        // Search in cache first
        for (name, metadata) in &self.cache {
            if name.contains(query) ||
               metadata.description.as_ref().map_or(false, |d| d.contains(query)) ||
               metadata.keywords.iter().any(|k| k.contains(query)) {
                results.push(metadata.clone());
            }
        }

        // TODO: Search in remote registries

        Ok(results)
    }

    /// Update package cache
    pub fn update_cache(&mut self) -> TlispResult<()> {
        // TODO: Fetch latest package information from registries
        Ok(())
    }

    /// Get package information
    pub fn get_package_info(&self, name: &str) -> Option<&PackageMetadata> {
        self.cache.get(name)
    }

    /// Check if package is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed.contains_key(name)
    }
}

impl DependencyResolver {
    /// Create new dependency resolver
    pub fn new() -> Self {
        DependencyResolver {
            cache: HashMap::new(),
            strategy: ConflictResolution::Highest,
        }
    }

    /// Set conflict resolution strategy
    pub fn with_strategy(mut self, strategy: ConflictResolution) -> Self {
        self.strategy = strategy;
        self
    }

    /// Resolve dependencies for a package
    pub fn resolve(
        &mut self,
        package_name: &str,
        options: &InstallOptions,
        registries: &[PackageRegistry],
    ) -> TlispResult<Vec<ResolvedDependency>> {
        let mut resolved = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with the root package
        queue.push_back((package_name.to_string(), Vec::new()));

        while let Some((name, path)) = queue.pop_front() {
            if visited.contains(&name) {
                continue;
            }
            visited.insert(name.clone());

            // Find package in registries
            let package_metadata = self.find_package(&name, registries)?;

            // Create resolved dependency
            let resolved_dep = ResolvedDependency {
                name: name.clone(),
                version: package_metadata.version.clone(),
                registry: "default".to_string(), // TODO: Track actual registry
                features: HashSet::new(), // TODO: Handle features
                path: path.clone(),
            };

            resolved.push(resolved_dep);

            // Add dependencies to queue
            for (dep_name, _req) in &package_metadata.dependencies {
                let mut new_path = path.clone();
                new_path.push(name.clone());
                queue.push_back((dep_name.clone(), new_path));
            }

            // Add dev dependencies if requested
            if options.dev {
                for (dep_name, _req) in &package_metadata.dev_dependencies {
                    let mut new_path = path.clone();
                    new_path.push(name.clone());
                    queue.push_back((dep_name.clone(), new_path));
                }
            }

            // Add optional dependencies if requested
            if options.optional {
                for (dep_name, _req) in &package_metadata.optional_dependencies {
                    let mut new_path = path.clone();
                    new_path.push(name.clone());
                    queue.push_back((dep_name.clone(), new_path));
                }
            }
        }

        // Sort by dependency order (dependencies first)
        resolved.sort_by_key(|dep| dep.path.len());

        Ok(resolved)
    }

    /// Find package in registries
    fn find_package(&self, name: &str, registries: &[PackageRegistry]) -> TlispResult<PackageMetadata> {
        // Search through all registries for the package
        for registry in registries {
            // Create a registry implementation for each registry config
            let registry_impl = RegistryImpl::new(
                registry.name.clone(),
                registry.url.clone(),
                registry.cache_dir.clone()
            );

            if let Ok(metadata) = registry_impl.get_package_metadata(name) {
                return Ok(metadata);
            }
        }

        // If not found in any registry, try to create a default package
        // This could be a local package or a fallback
        if self.package_exists_locally(name) {
            Ok(PackageMetadata::new(name.to_string(), "local".to_string()))
        } else {
            Err(TlispError::Runtime(format!("Package '{}' not found in any registry", name)))
        }
    }

    /// Check if package exists locally
    fn package_exists_locally(&self, name: &str) -> bool {
        // Check if package is in the cache
        self.cache.contains_key(name)
    }

    /// Resolve version conflicts
    fn resolve_conflicts(&self, packages: &[ResolvedDependency]) -> TlispResult<Vec<ResolvedDependency>> {
        // Group by package name
        let mut groups: HashMap<String, Vec<&ResolvedDependency>> = HashMap::new();
        for pkg in packages {
            groups.entry(pkg.name.clone()).or_default().push(pkg);
        }

        let mut resolved = Vec::new();
        for (name, versions) in groups {
            if versions.len() == 1 {
                resolved.push(versions[0].clone());
            } else {
                // Handle version conflicts based on strategy
                let selected = match &self.strategy {
                    ConflictResolution::Highest => {
                        // Select highest version (simplified)
                        versions.iter().max_by_key(|v| &v.version).unwrap()
                    }
                    ConflictResolution::Lowest => {
                        // Select lowest version (simplified)
                        versions.iter().min_by_key(|v| &v.version).unwrap()
                    }
                    ConflictResolution::Strict => {
                        return Err(TlispError::Runtime(
                            format!("Version conflict for package {}", name)
                        ));
                    }
                    ConflictResolution::Manual(resolutions) => {
                        if let Some(target_version) = resolutions.get(&name) {
                            versions.iter()
                                .find(|v| &v.version == target_version)
                                .ok_or_else(|| TlispError::Runtime(
                                    format!("Manual resolution version {} not found for {}", target_version, name)
                                ))?
                        } else {
                            return Err(TlispError::Runtime(
                                format!("No manual resolution specified for {}", name)
                            ));
                        }
                    }
                };
                resolved.push((*selected).clone());
            }
        }

        Ok(resolved)
    }
}

impl InstallOptions {
    /// Create default install options
    pub fn new() -> Self {
        InstallOptions {
            features: Vec::new(),
            dev: false,
            global: false,
            force: false,
            optional: false,
        }
    }

    /// Enable development dependencies
    pub fn with_dev(mut self) -> Self {
        self.dev = true;
        self
    }

    /// Install globally
    pub fn global(mut self) -> Self {
        self.global = true;
        self
    }

    /// Force reinstall
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Include optional dependencies
    pub fn with_optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Add feature
    pub fn with_feature(mut self, feature: String) -> Self {
        self.features.push(feature);
        self
    }
}
