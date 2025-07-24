//! Module System for TLISP
//! 
//! Implements TLISP module system with cross-language integration and namespace management.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::tlisp::Value;
use crate::tlisp::types::Type;
use crate::error::{TlispError, TlispResult};

/// Module definition
#[derive(Debug, Clone)]
pub struct Module {
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Module author
    pub author: Option<String>,
    /// Module description
    pub description: Option<String>,
    /// Module license
    pub license: Option<String>,
    /// Module dependencies
    pub dependencies: HashMap<String, ModuleDependency>,
    /// Exported symbols
    pub exports: HashMap<String, Value>,
    /// Exported types
    pub exported_types: HashMap<String, Type>,
    /// Module source language
    pub language: ModuleLanguage,
    /// Module source path
    pub source_path: Option<PathBuf>,
}

/// Module dependency
#[derive(Debug, Clone)]
pub struct ModuleDependency {
    /// Dependency name
    pub name: String,
    /// Version requirement
    pub version: String,
    /// Source language
    pub language: ModuleLanguage,
    /// Optional features
    pub features: Vec<String>,
}

/// Module language
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ModuleLanguage {
    /// TLISP module
    TLisp,
    /// Rust module
    Rust,
    /// Python module
    Python,
    /// JavaScript module
    JavaScript,
    /// C module
    C,
    /// Custom language
    Custom(String),
}

/// Module registry
pub struct ModuleRegistry {
    /// Loaded modules
    modules: HashMap<String, Module>,
    /// Module search paths
    search_paths: Vec<PathBuf>,
    /// Language compilers
    compilers: HashMap<ModuleLanguage, Box<dyn ModuleCompiler>>,
}

/// Module compiler trait
pub trait ModuleCompiler: Send + Sync {
    /// Compile module from source
    fn compile_module(&self, source_path: &PathBuf) -> TlispResult<Module>;
    
    /// Get supported language
    fn language(&self) -> ModuleLanguage;
}

/// Module cache for efficient loading
#[derive(Debug, Clone)]
pub struct ModuleCache {
    /// Cached compiled modules
    compiled_cache: HashMap<String, CachedModule>,
    /// Source file modification times
    file_times: HashMap<PathBuf, std::time::SystemTime>,
    /// Cache statistics
    stats: CacheStats,
}

/// Cached module information
#[derive(Debug, Clone)]
pub struct CachedModule {
    /// Module data
    pub module: Module,
    /// Cache time
    pub cached_at: std::time::SystemTime,
    /// Source file path
    pub source_path: PathBuf,
    /// Source file hash
    pub source_hash: String,
    /// Compilation time
    pub compile_time: std::time::Duration,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Cache invalidations
    pub invalidations: u64,
    /// Total compilation time saved
    pub time_saved: std::time::Duration,
}

/// Dependency graph for module dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Adjacency list representation
    graph: HashMap<String, Vec<String>>,
    /// Reverse dependencies
    reverse_deps: HashMap<String, Vec<String>>,
    /// Topological order cache
    topo_order: Option<Vec<String>>,
}

/// Hot reload watcher for development
pub struct HotReloadWatcher {
    /// Changed files queue
    changed_files: std::sync::mpsc::Receiver<PathBuf>,
    /// Sender for changed files
    _sender: std::sync::mpsc::Sender<PathBuf>,
}

impl ModuleCache {
    /// Create new module cache
    pub fn new() -> Self {
        ModuleCache {
            compiled_cache: HashMap::new(),
            file_times: HashMap::new(),
            stats: CacheStats::default(),
        }
    }

    /// Check if module is cached and up-to-date
    pub fn is_cached(&self, name: &str, source_path: &Path) -> bool {
        if let Some(cached) = self.compiled_cache.get(name) {
            if cached.source_path == source_path {
                // Check if source file has been modified
                if let Ok(metadata) = std::fs::metadata(source_path) {
                    if let Ok(modified) = metadata.modified() {
                        return self.file_times.get(source_path)
                            .map(|&cached_time| cached_time >= modified)
                            .unwrap_or(false);
                    }
                }
            }
        }
        false
    }

    /// Get cached module
    pub fn get(&mut self, name: &str) -> Option<&Module> {
        if let Some(cached) = self.compiled_cache.get(name) {
            self.stats.hits += 1;
            Some(&cached.module)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Cache a compiled module
    pub fn cache(&mut self, name: String, module: Module, source_path: PathBuf, compile_time: std::time::Duration) -> TlispResult<()> {
        // Calculate source hash
        let source_hash = self.calculate_file_hash(&source_path)?;

        // Update file modification time
        if let Ok(metadata) = std::fs::metadata(&source_path) {
            if let Ok(modified) = metadata.modified() {
                self.file_times.insert(source_path.clone(), modified);
            }
        }

        // Cache the module
        let cached_module = CachedModule {
            module,
            cached_at: std::time::SystemTime::now(),
            source_path,
            source_hash,
            compile_time,
        };

        self.compiled_cache.insert(name, cached_module);
        self.stats.time_saved += compile_time;

        Ok(())
    }

    /// Invalidate cache entry
    pub fn invalidate(&mut self, name: &str) {
        if self.compiled_cache.remove(name).is_some() {
            self.stats.invalidations += 1;
        }
    }

    /// Clear all cache
    pub fn clear(&mut self) {
        self.compiled_cache.clear();
        self.file_times.clear();
        self.stats.invalidations += self.stats.hits + self.stats.misses;
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Calculate file hash for cache validation
    fn calculate_file_hash(&self, path: &Path) -> TlispResult<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = std::fs::read_to_string(path)
            .map_err(|e| TlispError::Runtime(format!("Failed to read file for hashing: {}", e)))?;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }
}

impl DependencyGraph {
    /// Create new dependency graph
    pub fn new() -> Self {
        DependencyGraph {
            graph: HashMap::new(),
            reverse_deps: HashMap::new(),
            topo_order: None,
        }
    }

    /// Add dependency edge
    pub fn add_dependency(&mut self, from: String, to: String) {
        self.graph.entry(from.clone()).or_default().push(to.clone());
        self.reverse_deps.entry(to).or_default().push(from);
        self.topo_order = None; // Invalidate cached order
    }

    /// Remove dependency edge
    pub fn remove_dependency(&mut self, from: &str, to: &str) {
        if let Some(deps) = self.graph.get_mut(from) {
            deps.retain(|dep| dep != to);
        }
        if let Some(reverse) = self.reverse_deps.get_mut(to) {
            reverse.retain(|dep| dep != from);
        }
        self.topo_order = None;
    }

    /// Get dependencies of a module
    pub fn get_dependencies(&self, module: &str) -> Vec<&String> {
        self.graph.get(module).map(|deps| deps.iter().collect()).unwrap_or_default()
    }

    /// Get modules that depend on this module
    pub fn get_dependents(&self, module: &str) -> Vec<&String> {
        self.reverse_deps.get(module).map(|deps| deps.iter().collect()).unwrap_or_default()
    }

    /// Get topological order of modules
    pub fn topological_order(&mut self) -> TlispResult<&Vec<String>> {
        if self.topo_order.is_none() {
            self.topo_order = Some(self.compute_topological_order()?);
        }
        Ok(self.topo_order.as_ref().unwrap())
    }

    /// Check for circular dependencies
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in self.graph.keys() {
            if !visited.contains(node) {
                if self.has_cycle_util(node, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        false
    }

    /// Utility function for cycle detection
    fn has_cycle_util(&self, node: &str, visited: &mut HashSet<String>, rec_stack: &mut HashSet<String>) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = self.graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle_util(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Compute topological order using DFS
    fn compute_topological_order(&self) -> TlispResult<Vec<String>> {
        if self.has_cycle() {
            return Err(TlispError::Runtime("Circular dependency detected".to_string()));
        }

        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        for node in self.graph.keys() {
            if !visited.contains(node) {
                self.topological_sort_util(node, &mut visited, &mut stack);
            }
        }

        stack.reverse();
        Ok(stack)
    }

    /// Utility function for topological sort
    fn topological_sort_util(&self, node: &str, visited: &mut HashSet<String>, stack: &mut Vec<String>) {
        visited.insert(node.to_string());

        if let Some(neighbors) = self.graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.topological_sort_util(neighbor, visited, stack);
                }
            }
        }

        stack.push(node.to_string());
    }
}

impl HotReloadWatcher {
    /// Create new hot reload watcher
    pub fn new() -> TlispResult<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();

        Ok(HotReloadWatcher {
            changed_files: receiver,
            _sender: sender,
        })
    }

    /// Check for changed files
    pub fn check_changes(&self) -> Vec<PathBuf> {
        let mut changes = Vec::new();
        while let Ok(path) = self.changed_files.try_recv() {
            changes.push(path);
        }
        changes
    }
}

/// Module loader with caching and dependency management
pub struct ModuleLoader {
    /// Module registry
    registry: ModuleRegistry,
    /// Module search paths
    search_paths: Vec<PathBuf>,
    /// Module cache
    cache: ModuleCache,
    /// Dependency graph
    dependency_graph: DependencyGraph,
    /// Hot reload watcher
    hot_reload: Option<HotReloadWatcher>,
}

impl Module {
    /// Create a new module
    pub fn new(name: String, version: String, language: ModuleLanguage) -> Self {
        Module {
            name,
            version,
            author: None,
            description: None,
            license: None,
            dependencies: HashMap::new(),
            exports: HashMap::new(),
            exported_types: HashMap::new(),
            language,
            source_path: None,
        }
    }

    /// Add dependency
    pub fn add_dependency(&mut self, dependency: ModuleDependency) {
        self.dependencies.insert(dependency.name.clone(), dependency);
    }

    /// Export symbol
    pub fn export_symbol(&mut self, name: String, value: Value) {
        self.exports.insert(name, value);
    }

    /// Export type
    pub fn export_type(&mut self, name: String, type_def: Type) {
        self.exported_types.insert(name, type_def);
    }

    /// Get exported symbol
    pub fn get_export(&self, name: &str) -> Option<&Value> {
        self.exports.get(name)
    }

    /// Get exported type
    pub fn get_exported_type(&self, name: &str) -> Option<&Type> {
        self.exported_types.get(name)
    }

    /// Check if module exports symbol
    pub fn exports_symbol(&self, name: &str) -> bool {
        self.exports.contains_key(name)
    }

    /// List all exports
    pub fn list_exports(&self) -> Vec<&String> {
        self.exports.keys().collect()
    }

    /// List all exported types
    pub fn list_exported_types(&self) -> Vec<&String> {
        self.exported_types.keys().collect()
    }
}

impl ModuleRegistry {
    /// Create a new module registry
    pub fn new() -> Self {
        ModuleRegistry {
            modules: HashMap::new(),
            search_paths: vec![
                PathBuf::from("./"),
                PathBuf::from("./modules/"),
                PathBuf::from("./lib/"),
            ],
            compilers: HashMap::new(),
        }
    }

    /// Register module
    pub fn register_module(&mut self, module: Module) {
        self.modules.insert(module.name.clone(), module);
    }

    /// Get module
    pub fn get_module(&self, name: &str) -> Option<&Module> {
        self.modules.get(name)
    }

    /// Add search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Register compiler
    pub fn register_compiler(&mut self, compiler: Box<dyn ModuleCompiler>) {
        let language = compiler.language();
        self.compilers.insert(language, compiler);
    }

    /// Find module file
    pub fn find_module_file(&self, name: &str, language: &ModuleLanguage) -> Option<PathBuf> {
        let extension = match language {
            ModuleLanguage::TLisp => "tlisp",
            ModuleLanguage::Rust => "rs",
            ModuleLanguage::Python => "py",
            ModuleLanguage::JavaScript => "js",
            ModuleLanguage::C => "c",
            ModuleLanguage::Custom(ext) => ext,
        };

        for search_path in &self.search_paths {
            let module_file = search_path.join(format!("{}.{}", name, extension));
            if module_file.exists() {
                return Some(module_file);
            }
        }

        None
    }

    /// Compile module
    pub fn compile_module(&self, source_path: &PathBuf, language: &ModuleLanguage) -> TlispResult<Module> {
        if let Some(compiler) = self.compilers.get(language) {
            compiler.compile_module(source_path)
        } else {
            Err(TlispError::Runtime(
                format!("No compiler registered for language: {:?}", language)
            ))
        }
    }

    /// List all modules
    pub fn list_modules(&self) -> Vec<&String> {
        self.modules.keys().collect()
    }

    /// Unregister module
    pub fn unregister_module(&mut self, name: &str) -> Option<Module> {
        self.modules.remove(name)
    }
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new() -> Self {
        ModuleLoader {
            registry: ModuleRegistry::new(),
            search_paths: vec![
                PathBuf::from("./"),
                PathBuf::from("./modules/"),
                PathBuf::from("./lib/"),
            ],
            cache: ModuleCache::new(),
            dependency_graph: DependencyGraph::new(),
            hot_reload: None,
        }
    }

    /// Create module loader with custom search paths
    pub fn with_search_paths(search_paths: Vec<PathBuf>) -> Self {
        ModuleLoader {
            registry: ModuleRegistry::new(),
            search_paths,
            cache: ModuleCache::new(),
            dependency_graph: DependencyGraph::new(),
            hot_reload: None,
        }
    }

    /// Enable hot reload for development
    pub fn with_hot_reload(mut self) -> TlispResult<Self> {
        self.hot_reload = Some(HotReloadWatcher::new()?);
        Ok(self)
    }

    /// Load module with caching and dependency management
    pub fn load_module(&mut self, name: &str, language: ModuleLanguage) -> TlispResult<()> {
        // Check if module is already loaded
        if self.registry.get_module(name).is_some() {
            return Ok(());
        }

        // Find module file in search paths
        let module_file = self.find_module_file(name, &language)
            .ok_or_else(|| TlispError::Runtime(
                format!("Module {} not found in search paths", name)
            ))?;

        // Check cache first
        if self.cache.is_cached(name, &module_file) {
            if let Some(cached_module) = self.cache.get(name) {
                // Clone the cached module and register it
                let module = cached_module.clone();
                self.registry.register_module(module);
                return Ok(());
            }
        }

        // Compile module with timing
        let start_time = std::time::Instant::now();
        let module = self.registry.compile_module(&module_file, &language)?;
        let compile_time = start_time.elapsed();

        // Add dependencies to dependency graph
        for dependency in module.dependencies.values() {
            self.dependency_graph.add_dependency(name.to_string(), dependency.name.clone());
        }

        // Check for circular dependencies
        if self.dependency_graph.has_cycle() {
            return Err(TlispError::Runtime(
                format!("Circular dependency detected when loading module {}", name)
            ));
        }

        // Load dependencies in topological order
        let dependencies: Vec<_> = module.dependencies.values().cloned().collect();
        for dependency in dependencies {
            self.load_module(&dependency.name, dependency.language)?;
        }

        // Cache the compiled module
        self.cache.cache(name.to_string(), module.clone(), module_file, compile_time)?;

        // Register module
        self.registry.register_module(module);

        Ok(())
    }

    /// Find module file in search paths
    fn find_module_file(&self, name: &str, language: &ModuleLanguage) -> Option<PathBuf> {
        let extensions = match language {
            ModuleLanguage::TLisp => vec!["tl", "tlisp"],
            ModuleLanguage::Rust => vec!["rs"],
            ModuleLanguage::JavaScript => vec!["js", "mjs"],
            ModuleLanguage::Python => vec!["py"],
            ModuleLanguage::C => vec!["c", "h"],
            ModuleLanguage::Custom(ext) => vec![ext.as_str()],
        };

        for search_path in &self.search_paths {
            for ext in &extensions {
                let file_path = search_path.join(format!("{}.{}", name, ext));
                if file_path.exists() {
                    return Some(file_path);
                }

                // Also check for module directories with main files
                let dir_path = search_path.join(name);
                if dir_path.is_dir() {
                    let main_file = dir_path.join(format!("main.{}", ext));
                    if main_file.exists() {
                        return Some(main_file);
                    }
                    let index_file = dir_path.join(format!("index.{}", ext));
                    if index_file.exists() {
                        return Some(index_file);
                    }
                }
            }
        }

        None
    }

    /// Load module with hot reload support
    pub fn load_module_with_hot_reload(&mut self, name: &str, language: ModuleLanguage) -> TlispResult<()> {
        // Check for file changes if hot reload is enabled
        if let Some(hot_reload) = &self.hot_reload {
            let changes = hot_reload.check_changes();
            for changed_file in changes {
                // Invalidate cache for changed files
                if let Some(file_name) = changed_file.file_stem() {
                    if let Some(name_str) = file_name.to_str() {
                        self.cache.invalidate(name_str);
                        // Also invalidate dependents
                        let dependents = self.dependency_graph.get_dependents(name_str);
                        for dependent in dependents {
                            self.cache.invalidate(dependent);
                        }
                    }
                }
            }
        }

        self.load_module(name, language)
    }

    /// Reload module (force recompilation)
    pub fn reload_module(&mut self, name: &str, language: ModuleLanguage) -> TlispResult<()> {
        // Invalidate cache
        self.cache.invalidate(name);

        // Remove from registry
        self.registry.unregister_module(name);

        // Reload
        self.load_module(name, language)
    }

    /// Unload module and its dependents
    pub fn unload_module(&mut self, name: &str) -> TlispResult<()> {
        // Get all dependents (collect to avoid borrowing issues)
        let dependents: Vec<String> = self.dependency_graph.get_dependents(name)
            .into_iter().cloned().collect();

        // Unload dependents first
        for dependent in dependents {
            self.unload_module(&dependent)?;
        }

        // Remove from cache
        self.cache.invalidate(name);

        // Remove from registry
        self.registry.unregister_module(name);

        // Remove from dependency graph (collect dependencies first)
        let dependencies: Vec<String> = self.dependency_graph.get_dependencies(name)
            .into_iter().cloned().collect();
        for dep in dependencies {
            self.dependency_graph.remove_dependency(name, &dep);
        }

        Ok(())
    }

    /// Get module load order based on dependencies
    pub fn get_load_order(&mut self) -> TlispResult<Vec<String>> {
        self.dependency_graph.topological_order().map(|order| order.clone())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> &CacheStats {
        self.cache.stats()
    }

    /// Add search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Remove search path
    pub fn remove_search_path(&mut self, path: &Path) {
        self.search_paths.retain(|p| p != path);
    }

    /// List search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Get loaded module
    pub fn get_module(&self, name: &str) -> Option<&Module> {
        self.registry.get_module(name)
    }

    /// Import symbols from module
    pub fn import_symbols(&self, module_name: &str, symbols: &[String]) -> TlispResult<HashMap<String, Value>> {
        let module = self.registry.get_module(module_name)
            .ok_or_else(|| TlispError::Runtime(
                format!("Module {} not loaded", module_name)
            ))?;

        let mut imported = HashMap::new();
        for symbol in symbols {
            if let Some(value) = module.get_export(symbol) {
                imported.insert(symbol.clone(), value.clone());
            } else {
                return Err(TlispError::Runtime(
                    format!("Symbol {} not exported by module {}", symbol, module_name)
                ));
            }
        }

        Ok(imported)
    }

    /// Import all symbols from module
    pub fn import_all_symbols(&self, module_name: &str) -> TlispResult<HashMap<String, Value>> {
        let module = self.registry.get_module(module_name)
            .ok_or_else(|| TlispError::Runtime(
                format!("Module {} not loaded", module_name)
            ))?;

        Ok(module.exports.clone())
    }

    /// Get module registry
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    /// Get mutable module registry
    pub fn registry_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.registry
    }
}

/// TLISP module compiler
pub struct TLispModuleCompiler;

impl ModuleCompiler for TLispModuleCompiler {
    fn compile_module(&self, source_path: &PathBuf) -> TlispResult<Module> {
        // Read source file
        let _source = std::fs::read_to_string(source_path)
            .map_err(|e| TlispError::Runtime(format!("Failed to read module: {}", e)))?;

        // Parse module definition
        // For now, create a basic module
        let module_name = source_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut module = Module::new(module_name, "1.0.0".to_string(), ModuleLanguage::TLisp);
        module.source_path = Some(source_path.clone());

        // TODO: Parse actual module definition from source
        // For now, just export a dummy function
        module.export_symbol("hello".to_string(), Value::String("Hello from module!".to_string()));

        Ok(module)
    }

    fn language(&self) -> ModuleLanguage {
        ModuleLanguage::TLisp
    }
}

/// Module system utilities
pub struct ModuleUtils;

impl ModuleUtils {
    /// Create standard library module
    pub fn create_stdlib_module() -> Module {
        let mut module = Module::new("std".to_string(), "1.0.0".to_string(), ModuleLanguage::TLisp);
        
        // Add standard library functions
        module.export_symbol("map".to_string(), Value::Builtin("map".to_string()));
        module.export_symbol("filter".to_string(), Value::Builtin("filter".to_string()));
        module.export_symbol("fold".to_string(), Value::Builtin("fold".to_string()));
        module.export_symbol("length".to_string(), Value::Builtin("length".to_string()));
        
        // Add standard types
        module.export_type("List".to_string(), Type::List(Box::new(Type::TypeVar("a".to_string()))));
        module.export_type("Maybe".to_string(), Type::TypeVar("Maybe".to_string()));
        
        module
    }

    /// Create actor module
    pub fn create_actor_module() -> Module {
        let mut module = Module::new("actor".to_string(), "1.0.0".to_string(), ModuleLanguage::TLisp);
        
        // Add actor functions
        module.export_symbol("spawn".to_string(), Value::Builtin("spawn".to_string()));
        module.export_symbol("send".to_string(), Value::Builtin("send".to_string()));
        module.export_symbol("receive".to_string(), Value::Builtin("receive".to_string()));
        module.export_symbol("link".to_string(), Value::Builtin("link".to_string()));
        module.export_symbol("monitor".to_string(), Value::Builtin("monitor".to_string()));
        
        // Add actor types
        module.export_type("Pid".to_string(), Type::Pid);
        module.export_type("Process".to_string(), Type::TypeVar("Process".to_string()));
        
        module
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}
