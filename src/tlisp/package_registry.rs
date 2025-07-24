//! Package Registry and Publishing System for TLISP
//! 
//! Provides a comprehensive package registry system for publishing,
//! downloading, and managing TLISP packages with version control,
//! authentication, and dependency resolution.

use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use crate::error::{TlispError, TlispResult};
use crate::tlisp::package_manager::{PackageMetadata, VersionRequirement};


/// Package registry for managing TLISP packages
pub struct PackageRegistry {
    /// Registry name
    name: String,
    /// Registry URL
    url: String,
    /// Local cache directory
    cache_dir: PathBuf,
    /// Authentication token
    auth_token: Option<String>,
    /// Registry configuration
    config: RegistryConfig,
    /// Package index cache
    index_cache: HashMap<String, PackageIndex>,
    /// Registry statistics
    stats: RegistryStats,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry API version
    pub api_version: String,
    /// Maximum package size in bytes
    pub max_package_size: u64,
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    /// Required metadata fields
    pub required_fields: Vec<String>,
    /// Registry features
    pub features: RegistryFeatures,
    /// Rate limiting configuration
    pub rate_limits: RateLimits,
}

/// Registry features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryFeatures {
    /// Support for private packages
    pub private_packages: bool,
    /// Support for package signing
    pub package_signing: bool,
    /// Support for package mirroring
    pub mirroring: bool,
    /// Support for pre-release versions
    pub pre_releases: bool,
    /// Support for package deprecation
    pub deprecation: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Requests per minute for downloads
    pub downloads_per_minute: u32,
    /// Requests per minute for uploads
    pub uploads_per_minute: u32,
    /// Requests per minute for API calls
    pub api_calls_per_minute: u32,
}

/// Package index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIndex {
    /// Package name
    pub name: String,
    /// Available versions
    pub versions: HashMap<String, PackageVersion>,
    /// Package metadata
    pub metadata: PackageMetadata,
    /// Last updated timestamp
    pub last_updated: SystemTime,
    /// Download statistics
    pub download_stats: DownloadStats,
}

/// Package version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    /// Version string
    pub version: String,
    /// Package metadata for this version
    pub metadata: PackageMetadata,
    /// Download URL
    pub download_url: String,
    /// Package checksum
    pub checksum: String,
    /// Package size in bytes
    pub size: u64,
    /// Publication timestamp
    pub published_at: SystemTime,
    /// Whether this version is yanked
    pub yanked: bool,
    /// Yank reason if applicable
    pub yank_reason: Option<String>,
}

/// Download statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStats {
    /// Total downloads
    pub total: u64,
    /// Downloads in the last 30 days
    pub recent: u64,
    /// Downloads by version
    pub by_version: HashMap<String, u64>,
    /// Downloads by date
    pub by_date: HashMap<String, u64>,
}

/// Registry statistics
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    /// Total packages in registry
    pub total_packages: u64,
    /// Total downloads
    pub total_downloads: u64,
    /// API calls made
    pub api_calls: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Failed operations
    pub failed_operations: u64,
}

/// Package publishing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    /// Package metadata
    pub metadata: PackageMetadata,
    /// Package content (base64 encoded)
    pub content: String,
    /// Package checksum
    pub checksum: String,
    /// Publishing options
    pub options: PublishOptions,
}

/// Publishing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishOptions {
    /// Whether to allow overwriting existing versions
    pub allow_overwrite: bool,
    /// Whether this is a pre-release
    pub pre_release: bool,
    /// Whether to make the package private
    pub private: bool,
    /// Package signing key ID
    pub signing_key: Option<String>,
}

/// Package search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search terms
    pub query: String,
    /// Category filter
    pub category: Option<String>,
    /// Keyword filter
    pub keywords: Vec<String>,
    /// Author filter
    pub author: Option<String>,
    /// License filter
    pub license: Option<String>,
    /// Sort order
    pub sort: SearchSort,
    /// Maximum results
    pub limit: Option<u32>,
    /// Result offset
    pub offset: Option<u32>,
}

/// Search sort options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchSort {
    /// Sort by relevance
    Relevance,
    /// Sort by download count
    Downloads,
    /// Sort by last updated
    Updated,
    /// Sort by name
    Name,
    /// Sort by creation date
    Created,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Found packages
    pub packages: Vec<PackageSearchResult>,
    /// Total number of results
    pub total: u32,
    /// Query that was executed
    pub query: SearchQuery,
    /// Search execution time
    pub execution_time: std::time::Duration,
}

/// Package search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSearchResult {
    /// Package metadata
    pub metadata: PackageMetadata,
    /// Latest version
    pub latest_version: String,
    /// Download statistics
    pub downloads: DownloadStats,
    /// Search relevance score
    pub score: f64,
}

impl PackageRegistry {
    /// Create new package registry
    pub fn new(name: String, url: String, cache_dir: PathBuf) -> Self {
        PackageRegistry {
            name,
            url,
            cache_dir,
            auth_token: None,
            config: RegistryConfig::default(),
            index_cache: HashMap::new(),
            stats: RegistryStats::default(),
        }
    }

    /// Set authentication token
    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    /// Initialize registry cache
    pub fn initialize(&mut self) -> TlispResult<()> {
        // Create cache directory
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| TlispError::Runtime(format!("Failed to create cache directory: {}", e)))?;

        // Load registry configuration
        self.load_config()?;

        // Load package index cache
        self.load_index_cache()?;

        Ok(())
    }

    /// Load registry configuration
    fn load_config(&mut self) -> TlispResult<()> {
        let config_path = self.cache_dir.join("config.json");
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| TlispError::Runtime(format!("Failed to read config: {}", e)))?;
            
            self.config = serde_json::from_str(&content)
                .map_err(|e| TlispError::Runtime(format!("Failed to parse config: {}", e)))?;
        } else {
            // Fetch configuration from registry
            self.fetch_config()?;
        }

        Ok(())
    }

    /// Fetch configuration from registry
    fn fetch_config(&mut self) -> TlispResult<()> {
        // TODO: Implement HTTP client to fetch config from registry
        // For now, use default configuration
        self.config = RegistryConfig::default();
        
        // Save configuration to cache
        let config_path = self.cache_dir.join("config.json");
        let content = serde_json::to_string_pretty(&self.config)
            .map_err(|e| TlispError::Runtime(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(&config_path, content)
            .map_err(|e| TlispError::Runtime(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    /// Load package index cache
    fn load_index_cache(&mut self) -> TlispResult<()> {
        let index_path = self.cache_dir.join("index.json");
        
        if index_path.exists() {
            let content = fs::read_to_string(&index_path)
                .map_err(|e| TlispError::Runtime(format!("Failed to read index: {}", e)))?;
            
            self.index_cache = serde_json::from_str(&content)
                .map_err(|e| TlispError::Runtime(format!("Failed to parse index: {}", e)))?;
        }

        Ok(())
    }

    /// Save package index cache
    fn save_index_cache(&self) -> TlispResult<()> {
        let index_path = self.cache_dir.join("index.json");
        let content = serde_json::to_string_pretty(&self.index_cache)
            .map_err(|e| TlispError::Runtime(format!("Failed to serialize index: {}", e)))?;
        
        fs::write(&index_path, content)
            .map_err(|e| TlispError::Runtime(format!("Failed to write index: {}", e)))?;

        Ok(())
    }

    /// Publish package to registry
    pub fn publish_package(&mut self, request: PublishRequest) -> TlispResult<()> {
        // Validate package metadata
        self.validate_package_metadata(&request.metadata)?;

        // Check package size
        let content_size = request.content.len() as u64;
        if content_size > self.config.max_package_size {
            return Err(TlispError::Runtime(
                format!("Package size {} exceeds maximum allowed size {}", 
                    content_size, self.config.max_package_size)
            ));
        }

        // Verify checksum
        self.verify_checksum(&request.content, &request.checksum)?;

        // Check if version already exists
        if let Some(package_index) = self.index_cache.get(&request.metadata.name) {
            if package_index.versions.contains_key(&request.metadata.version) && !request.options.allow_overwrite {
                return Err(TlispError::Runtime(
                    format!("Version {} of package {} already exists", 
                        request.metadata.version, request.metadata.name)
                ));
            }
        }

        // TODO: Implement HTTP client to upload package to registry
        // For now, simulate successful upload
        
        // Update local index cache
        self.update_index_cache(&request)?;

        // Save updated cache
        self.save_index_cache()?;

        self.stats.total_packages += 1;
        
        Ok(())
    }

    /// Validate package metadata
    fn validate_package_metadata(&self, metadata: &PackageMetadata) -> TlispResult<()> {
        if metadata.name.is_empty() {
            return Err(TlispError::Runtime("Package name cannot be empty".to_string()));
        }

        if metadata.version.is_empty() {
            return Err(TlispError::Runtime("Package version cannot be empty".to_string()));
        }

        // Check required fields
        for field in &self.config.required_fields {
            match field.as_str() {
                "description" => {
                    if metadata.description.is_none() {
                        return Err(TlispError::Runtime("Description is required".to_string()));
                    }
                }
                "license" => {
                    if metadata.license.is_none() {
                        return Err(TlispError::Runtime("License is required".to_string()));
                    }
                }
                "author" => {
                    if metadata.author.is_none() {
                        return Err(TlispError::Runtime("Author is required".to_string()));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Verify package checksum
    fn verify_checksum(&self, content: &str, expected_checksum: &str) -> TlispResult<()> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let actual_checksum = format!("{:x}", hasher.finish());

        if actual_checksum != expected_checksum {
            return Err(TlispError::Runtime(
                format!("Checksum mismatch: expected {}, got {}", expected_checksum, actual_checksum)
            ));
        }

        Ok(())
    }

    /// Update index cache with new package
    fn update_index_cache(&mut self, request: &PublishRequest) -> TlispResult<()> {
        let package_name = &request.metadata.name;
        let version = &request.metadata.version;

        let package_version = PackageVersion {
            version: version.clone(),
            metadata: request.metadata.clone(),
            download_url: format!("{}/packages/{}/{}", self.url, package_name, version),
            checksum: request.checksum.clone(),
            size: request.content.len() as u64,
            published_at: SystemTime::now(),
            yanked: false,
            yank_reason: None,
        };

        if let Some(package_index) = self.index_cache.get_mut(package_name) {
            package_index.versions.insert(version.clone(), package_version);
            package_index.last_updated = SystemTime::now();
        } else {
            let mut versions = HashMap::new();
            versions.insert(version.clone(), package_version);

            let package_index = PackageIndex {
                name: package_name.clone(),
                versions,
                metadata: request.metadata.clone(),
                last_updated: SystemTime::now(),
                download_stats: DownloadStats {
                    total: 0,
                    recent: 0,
                    by_version: HashMap::new(),
                    by_date: HashMap::new(),
                },
            };

            self.index_cache.insert(package_name.clone(), package_index);
        }

        Ok(())
    }

    /// Get package metadata
    pub fn get_package_metadata(&self, name: &str) -> TlispResult<PackageMetadata> {
        if let Some(package_index) = self.index_cache.get(name) {
            // Get the latest version
            if let Some((latest_version, _)) = package_index.versions.iter().next() {
                Ok(PackageMetadata::new(name.to_string(), latest_version.clone()))
            } else {
                Err(TlispError::Runtime(format!("No versions found for package '{}'", name)))
            }
        } else {
            Err(TlispError::Runtime(format!("Package '{}' not found", name)))
        }
    }

    /// Download package from registry
    pub fn download_package(&mut self, name: &str, version_req: &VersionRequirement) -> TlispResult<PathBuf> {
        // Find matching version
        let package_index = self.index_cache.get(name)
            .ok_or_else(|| TlispError::Runtime(format!("Package '{}' not found", name)))?;

        let version = self.find_matching_version(package_index, version_req)?;
        let package_version = package_index.versions.get(&version)
            .ok_or_else(|| TlispError::Runtime(format!("Version '{}' not found", version)))?
            .clone(); // Clone to avoid borrowing issues

        // Check if already cached
        let cache_path = self.cache_dir.join("packages").join(name).join(&version);
        if cache_path.exists() {
            self.stats.cache_hits += 1;
            return Ok(cache_path);
        }

        self.stats.cache_misses += 1;

        // Create cache directory
        fs::create_dir_all(&cache_path)
            .map_err(|e| TlispError::Runtime(format!("Failed to create cache directory: {}", e)))?;

        // Download package using the version information
        let package_file = cache_path.join("package.tar.gz");
        self.download_package_version(&package_version, &package_file)?;

        // Update download statistics
        self.update_download_stats(name, &version);

        self.stats.total_downloads += 1;

        Ok(cache_path)
    }

    /// Find matching version for requirement
    fn find_matching_version(&self, package_index: &PackageIndex, version_req: &VersionRequirement) -> TlispResult<String> {
        // Simple version matching - in a real implementation, this would use semver
        for (version, package_version) in &package_index.versions {
            if !package_version.yanked && self.version_matches(version, &version_req.constraint) {
                return Ok(version.clone());
            }
        }

        Err(TlispError::Runtime(
            format!("No matching version found for constraint '{}'", version_req.constraint)
        ))
    }

    /// Check if version matches constraint
    fn version_matches(&self, version: &str, constraint: &str) -> bool {
        // Simple version matching - in a real implementation, this would use semver
        match constraint {
            "*" => true,
            _ => version == constraint,
        }
    }

    /// Update download statistics
    fn update_download_stats(&mut self, name: &str, version: &str) {
        if let Some(package_index) = self.index_cache.get_mut(name) {
            package_index.download_stats.total += 1;
            package_index.download_stats.recent += 1;

            *package_index.download_stats.by_version.entry(version.to_string()).or_insert(0) += 1;

            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            *package_index.download_stats.by_date.entry(today).or_insert(0) += 1;
        }
    }

    /// Search packages in registry
    pub fn search_packages(&mut self, query: SearchQuery) -> TlispResult<SearchResults> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();

        for (_, package_index) in &self.index_cache {
            let score = self.calculate_search_score(package_index, &query);
            if score > 0.0 {
                let latest_version = self.get_latest_version(package_index);
                results.push(PackageSearchResult {
                    metadata: package_index.metadata.clone(),
                    latest_version,
                    downloads: package_index.download_stats.clone(),
                    score,
                });
            }
        }

        // Sort results by score or specified sort order
        match query.sort {
            SearchSort::Relevance => results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap()),
            SearchSort::Downloads => results.sort_by(|a, b| b.downloads.total.cmp(&a.downloads.total)),
            SearchSort::Name => results.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name)),
            SearchSort::Updated => {
                // Would need to track update times for proper sorting
                results.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
            }
            SearchSort::Created => {
                // Would need to track creation times for proper sorting
                results.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
            }
        }

        // Apply limit and offset
        let total = results.len() as u32;
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(50) as usize;

        if offset < results.len() {
            results = results.into_iter().skip(offset).take(limit).collect();
        } else {
            results.clear();
        }

        let execution_time = start_time.elapsed();

        Ok(SearchResults {
            packages: results,
            total,
            query,
            execution_time,
        })
    }

    /// Calculate search relevance score
    fn calculate_search_score(&self, package_index: &PackageIndex, query: &SearchQuery) -> f64 {
        let mut score = 0.0;
        let query_lower = query.query.to_lowercase();
        let search_terms: Vec<&str> = query_lower.split_whitespace().collect();

        // Score based on name match
        let name_lower = package_index.metadata.name.to_lowercase();
        for term in &search_terms {
            if name_lower.contains(term) {
                score += 2.0;
            }
        }

        // Score based on description match
        if let Some(description) = &package_index.metadata.description {
            let desc_lower = description.to_lowercase();
            for term in &search_terms {
                if desc_lower.contains(term) {
                    score += 1.0;
                }
            }
        }

        // Score based on keywords match
        for keyword in &package_index.metadata.keywords {
            let keyword_lower = keyword.to_lowercase();
            for term in &search_terms {
                if keyword_lower.contains(term) {
                    score += 1.5;
                }
            }
        }

        // Apply filters
        if let Some(category) = &query.category {
            if !package_index.metadata.categories.contains(category) {
                return 0.0;
            }
        }

        if let Some(author) = &query.author {
            if package_index.metadata.author.as_ref() != Some(author) {
                return 0.0;
            }
        }

        if let Some(license) = &query.license {
            if package_index.metadata.license.as_ref() != Some(license) {
                return 0.0;
            }
        }

        if !query.keywords.is_empty() {
            let has_matching_keyword = query.keywords.iter()
                .any(|k| package_index.metadata.keywords.contains(k));
            if !has_matching_keyword {
                return 0.0;
            }
        }

        score
    }

    /// Get latest version of package
    fn get_latest_version(&self, package_index: &PackageIndex) -> String {
        // Simple latest version selection - in a real implementation, this would use semver
        package_index.versions.keys()
            .filter(|v| !package_index.versions[*v].yanked)
            .max()
            .cloned()
            .unwrap_or_else(|| "0.0.0".to_string())
    }

    /// Yank a package version
    pub fn yank_version(&mut self, name: &str, version: &str, reason: Option<String>) -> TlispResult<()> {
        let package_index = self.index_cache.get_mut(name)
            .ok_or_else(|| TlispError::Runtime(format!("Package '{}' not found", name)))?;

        let package_version = package_index.versions.get_mut(version)
            .ok_or_else(|| TlispError::Runtime(format!("Version '{}' not found", version)))?;

        package_version.yanked = true;
        package_version.yank_reason = reason;

        // Save updated cache
        self.save_index_cache()?;

        Ok(())
    }

    /// Unyank a package version
    pub fn unyank_version(&mut self, name: &str, version: &str) -> TlispResult<()> {
        let package_index = self.index_cache.get_mut(name)
            .ok_or_else(|| TlispError::Runtime(format!("Package '{}' not found", name)))?;

        let package_version = package_index.versions.get_mut(version)
            .ok_or_else(|| TlispError::Runtime(format!("Version '{}' not found", version)))?;

        package_version.yanked = false;
        package_version.yank_reason = None;

        // Save updated cache
        self.save_index_cache()?;

        Ok(())
    }

    /// Get package information
    pub fn get_package_info(&self, name: &str) -> Option<&PackageIndex> {
        self.index_cache.get(name)
    }

    /// List all packages
    pub fn list_packages(&self) -> Vec<&PackageIndex> {
        self.index_cache.values().collect()
    }

    /// Get registry statistics
    pub fn stats(&self) -> &RegistryStats {
        &self.stats
    }

    /// Clear cache
    pub fn clear_cache(&mut self) -> TlispResult<()> {
        self.index_cache.clear();

        // Remove cache files
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| TlispError::Runtime(format!("Failed to clear cache: {}", e)))?;
        }

        // Recreate cache directory
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| TlispError::Runtime(format!("Failed to create cache directory: {}", e)))?;

        Ok(())
    }

    /// Refresh package index from registry
    pub fn refresh_index(&mut self) -> TlispResult<()> {
        // TODO: Implement HTTP client to fetch updated index from registry
        // For now, just reload from cache
        self.load_index_cache()?;
        Ok(())
    }

    /// Download a specific package version
    pub fn download_package_version(&mut self, package_version: &PackageVersion, target_file: &std::path::Path) -> TlispResult<()> {
        // For now, create a placeholder implementation
        // In a real implementation, this would use the package_version.download_url
        // to fetch the actual package content via HTTP

        let content = format!(
            "# Package: {}\n# Version: {}\n# Description: {}\n# Placeholder package content",
            package_version.metadata.name,
            package_version.version,
            package_version.metadata.description.as_deref().unwrap_or("No description")
        );

        std::fs::write(target_file, content.as_bytes())
            .map_err(|e| TlispError::Runtime(format!("Failed to write package file: {}", e)))?;

        Ok(())
    }
}

impl RegistryConfig {
    /// Create default registry configuration
    pub fn default() -> Self {
        RegistryConfig {
            api_version: "1.0".to_string(),
            max_package_size: 100 * 1024 * 1024, // 100 MB
            allowed_extensions: vec![
                ".tl".to_string(),
                ".toml".to_string(),
                ".md".to_string(),
                ".txt".to_string(),
                ".json".to_string(),
            ],
            required_fields: vec![
                "name".to_string(),
                "version".to_string(),
                "description".to_string(),
                "license".to_string(),
            ],
            features: RegistryFeatures {
                private_packages: true,
                package_signing: false,
                mirroring: false,
                pre_releases: true,
                deprecation: true,
            },
            rate_limits: RateLimits {
                downloads_per_minute: 100,
                uploads_per_minute: 10,
                api_calls_per_minute: 200,
            },
        }
    }
}

/// Package registry manager for handling multiple registries
pub struct RegistryManager {
    /// Registered package registries
    registries: HashMap<String, PackageRegistry>,
    /// Default registry name
    default_registry: Option<String>,
    /// Global cache directory
    cache_dir: PathBuf,
}

impl RegistryManager {
    /// Create new registry manager
    pub fn new(cache_dir: PathBuf) -> Self {
        RegistryManager {
            registries: HashMap::new(),
            default_registry: None,
            cache_dir,
        }
    }

    /// Add registry
    pub fn add_registry(&mut self, name: String, url: String) -> TlispResult<()> {
        let registry_cache_dir = self.cache_dir.join(&name);
        let mut registry = PackageRegistry::new(name.clone(), url, registry_cache_dir);
        registry.initialize()?;

        self.registries.insert(name.clone(), registry);

        // Set as default if it's the first registry
        if self.default_registry.is_none() {
            self.default_registry = Some(name);
        }

        Ok(())
    }

    /// Remove registry
    pub fn remove_registry(&mut self, name: &str) -> TlispResult<()> {
        if let Some(registry) = self.registries.remove(name) {
            // Clear registry cache
            let _ = registry.cache_dir;

            // Update default registry if needed
            if self.default_registry.as_ref() == Some(&name.to_string()) {
                self.default_registry = self.registries.keys().next().cloned();
            }
        }

        Ok(())
    }

    /// Get registry
    pub fn get_registry(&self, name: &str) -> Option<&PackageRegistry> {
        self.registries.get(name)
    }

    /// Get mutable registry
    pub fn get_registry_mut(&mut self, name: &str) -> Option<&mut PackageRegistry> {
        self.registries.get_mut(name)
    }

    /// Get default registry
    pub fn get_default_registry(&self) -> Option<&PackageRegistry> {
        self.default_registry.as_ref().and_then(|name| self.registries.get(name))
    }

    /// Get mutable default registry
    pub fn get_default_registry_mut(&mut self) -> Option<&mut PackageRegistry> {
        let default_name = self.default_registry.clone()?;
        self.registries.get_mut(&default_name)
    }

    /// Set default registry
    pub fn set_default_registry(&mut self, name: String) -> TlispResult<()> {
        if self.registries.contains_key(&name) {
            self.default_registry = Some(name);
            Ok(())
        } else {
            Err(TlispError::Runtime(format!("Registry '{}' not found", name)))
        }
    }

    /// List all registries
    pub fn list_registries(&self) -> Vec<&str> {
        self.registries.keys().map(|s| s.as_str()).collect()
    }
}
