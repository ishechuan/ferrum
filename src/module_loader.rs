//! ES Module Loader
//!
//! This module implements ES2020 module loading with support for:
//! - File-based modules (.js, .mjs, .ts)
//! - Remote modules (HTTP/HTTPS)
//! - Import maps
//! - Module resolution and caching

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

use crate::permissions::Permissions;

/// Normalize a path by resolving . and .. components
fn normalize_path(path: &str) -> String {
    let path = Path::new(path);
    let mut components = Vec::new();

    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                // Pop the last component if we can
                if !components.is_empty() {
                    components.pop();
                }
            }
            Component::Normal(_) | Component::RootDir => {
                components.push(comp);
            }
            // Skip CurDir (.) and Prefix
            Component::CurDir | Component::Prefix(_) => {}
        }
    }

    let mut result = PathBuf::new();
    for comp in components {
        result.push(comp);
    }

    // Ensure absolute paths start with /
    let path_str = result.to_string_lossy().to_string();
    if path.starts_with("/") && !path_str.starts_with("/") {
        format!("/{}", path_str)
    } else {
        path_str
    }
}

/// Errors that can occur during module loading
#[derive(Error, Debug)]
pub enum ModuleError {
    /// Module was not found at the specified path or URL
    #[error("Module not found: {0}")]
    NotFound(String),

    /// Failed to resolve the module specifier to an absolute path or URL
    #[error("Failed to resolve module: {0}")]
    ResolutionError(String),

    /// Error parsing the module source code
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Permission denied for accessing the module
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Network error occurred while loading a remote module
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Circular dependency detected in module imports
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Invalid module specifier provided
    #[error("Invalid module specifier: {0}")]
    InvalidSpecifier(String),
}

/// Result type for module operations
pub type ModuleResult<T> = Result<T, ModuleError>;

/// Represents the type of module
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleType {
    /// ES Module (.mjs, explicit module)
    ESModule,
    /// CommonJS Module (.cjs, Node.js compatibility)
    CommonJS,
    /// JSON Module
    JSON,
    /// TypeScript Module (.ts)
    TypeScript,
    /// Unknown/Bare specifier
    Unknown,
}

impl ModuleType {
    /// Detect module type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            ".mjs" => ModuleType::ESModule,
            ".cjs" => ModuleType::CommonJS,
            ".json" => ModuleType::JSON,
            ".ts" => ModuleType::TypeScript,
            ".js" => ModuleType::ESModule, // Default to ESM
            _ => ModuleType::Unknown,
        }
    }
}

/// Module source information
#[derive(Debug, Clone)]
pub struct ModuleSource {
    /// The module specifier (URL or path)
    pub specifier: String,
    /// The source code
    pub code: String,
    /// Module type
    pub module_type: ModuleType,
}

/// Resolved module information
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// The resolved specifier (absolute path or URL)
    pub specifier: String,
    /// The module source
    pub source: ModuleSource,
    /// Dependencies (import statements)
    pub dependencies: Vec<String>,
}

/// Import map entry for resolving bare specifiers
#[derive(Debug, Clone)]
pub struct ImportMapEntry {
    /// The prefix to match
    pub prefix: String,
    /// The target to resolve to
    pub target: String,
}

/// Import map for module resolution
#[derive(Debug, Clone, Default)]
pub struct ImportMap {
    entries: Vec<ImportMapEntry>,
    /// Base URL for relative resolution
    #[allow(dead_code)]
    base_url: String,
}

impl ImportMap {
    /// Create a new import map
    pub fn new(base_url: String) -> Self {
        Self {
            entries: Vec::new(),
            base_url,
        }
    }

    /// Add an entry to the import map
    pub fn insert(&mut self, prefix: String, target: String) {
        // Sort by prefix length (longest first) for proper matching
        let entry = ImportMapEntry { prefix, target };
        self.entries.push(entry);
        self.entries.sort_by(|a, b| b.prefix.len().cmp(&a.prefix.len()));
    }

    /// Resolve a specifier using the import map
    pub fn resolve(&self, specifier: &str) -> Option<String> {
        for entry in &self.entries {
            if specifier.starts_with(&entry.prefix) {
                let suffix = &specifier[entry.prefix.len()..];
                return Some(format!("{}{}", entry.target, suffix));
            }
        }
        None
    }

    /// Parse an import map from JSON
    pub fn from_json(json: &str, base_url: String) -> ModuleResult<Self> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| ModuleError::ParseError(format!("Invalid import map JSON: {}", e)))?;

        let mut map = Self::new(base_url);

        if let Some(obj) = value.as_object() {
            if let Some(import_obj) = obj.get("imports").and_then(|v| v.as_object()) {
                for (key, value) in import_obj {
                    if let Some(target) = value.as_str() {
                        map.insert(key.clone(), target.to_string());
                    }
                }
            }
        }

        Ok(map)
    }
}

/// Module cache to avoid reloading the same module
#[derive(Debug, Clone, Default)]
pub struct ModuleCache {
    inner: Arc<tokio::sync::RwLock<HashMap<String, ResolvedModule>>>,
}

impl ModuleCache {
    /// Create a new module cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a module from the cache
    pub async fn get(&self, specifier: &str) -> Option<ResolvedModule> {
        self.inner.read().await.get(specifier).cloned()
    }

    /// Insert a module into the cache
    pub async fn insert(&self, specifier: String, module: ResolvedModule) {
        self.inner.write().await.insert(specifier, module);
    }

    /// Check if a module is cached
    pub async fn contains(&self, specifier: &str) -> bool {
        self.inner.read().await.contains_key(specifier)
    }

    /// Clear the cache
    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

/// Module loader configuration
#[derive(Debug, Clone)]
pub struct ModuleLoaderConfig {
    /// Whether to cache modules
    pub cache_enabled: bool,
    /// Whether to allow remote modules
    pub allow_remote: bool,
    /// Import map for resolution
    pub import_map: Option<ImportMap>,
    /// Base directory for resolving relative paths
    pub base_dir: PathBuf,
}

impl Default for ModuleLoaderConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            allow_remote: true,
            import_map: None,
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
        }
    }
}

/// ES Module Loader
pub struct ModuleLoader {
    /// Runtime permissions
    permissions: Permissions,
    /// Loader configuration
    config: ModuleLoaderConfig,
    /// Module cache
    cache: ModuleCache,
    /// Currently loading modules (for circular dependency detection)
    loading: Arc<tokio::sync::Mutex<Vec<String>>>,
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new(permissions: Permissions, config: ModuleLoaderConfig) -> Self {
        Self {
            permissions,
            config,
            cache: ModuleCache::new(),
            loading: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    /// Resolve a module specifier to an absolute path or URL
    pub fn resolve(&self, specifier: &str, referrer: Option<&str>) -> ModuleResult<String> {
        // Check import map first
        if let Some(import_map) = &self.config.import_map {
            if let Some(resolved) = import_map.resolve(specifier) {
                return Ok(resolved);
            }
        }

        // Handle different specifier types
        if specifier.starts_with("https://") || specifier.starts_with("http://") {
            if !self.config.allow_remote {
                return Err(ModuleError::PermissionDenied(
                    "Remote modules are disabled".to_string(),
                ));
            }
            return Ok(specifier.to_string());
        }

        if specifier.starts_with("/") {
            // Absolute path
            return Ok(specifier.to_string());
        }

        if specifier.starts_with("./") || specifier.starts_with("../") {
            // Relative path
            let base = if let Some(referrer_path) = referrer {
                if referrer_path.starts_with("http") {
                    // For URLs, resolve relative to the URL path
                    let url_path = referrer_path
                        .rsplit('/')
                        .skip(1)
                        .collect::<Vec<_>>()
                        .join("/");
                    format!("{}/{}", url_path, specifier.trim_start_matches("./"))
                } else {
                    // For file paths, resolve relative to the referrer directory
                    let referrer_dir = Path::new(referrer_path).parent()
                        .unwrap_or(Path::new("."));
                    // Join and normalize the path
                    let joined = referrer_dir.join(specifier);
                    normalize_path(joined.to_string_lossy().as_ref())
                }
            } else {
                // Resolve relative to base directory
                let joined = self.config.base_dir.join(specifier);
                normalize_path(joined.to_string_lossy().as_ref())
            };
            return Ok(base);
        }

        // Bare specifier - try node_modules style resolution
        self.resolve_bare_specifier(specifier)
    }

    /// Resolve a bare specifier (e.g., "lodash", "react")
    fn resolve_bare_specifier(&self, specifier: &str) -> ModuleResult<String> {
        // Check node_modules directories
        let mut current = self.config.base_dir.clone();

        loop {
            let node_modules = current.join("node_modules").join(specifier);

            // Try different file extensions
            for ext in &[".js", ".mjs", ".ts", "/index.js", "/index.mjs"] {
                let path = format!("{}{}", node_modules.display(), ext);
                if Path::new(&path).exists() {
                    return Ok(path);
                }
            }

            // Try package.json main field
            let package_json = node_modules.join("package.json");
            if package_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&package_json) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(main) = json.get("main").and_then(|v| v.as_str()) {
                            let main_path = node_modules.join(main);
                            if main_path.exists() {
                                return Ok(main_path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }

            // Move up to parent directory
            if !current.pop() {
                break;
            }
        }

        Err(ModuleError::NotFound(format!(
            "Cannot find module '{}'",
            specifier
        )))
    }

    /// Load a module from a resolved specifier
    pub async fn load(&self, specifier: &str) -> ModuleResult<ModuleSource> {
        // Check cache first
        if self.config.cache_enabled {
            if let Some(cached) = self.cache.get(specifier).await {
                return Ok(cached.source);
            }
        }

        let source = if specifier.starts_with("http") {
            self.load_remote(specifier).await?
        } else {
            self.load_local(specifier)?
        };

        Ok(source)
    }

    /// Load a local file module
    fn load_local(&self, path: &str) -> ModuleResult<ModuleSource> {
        // Check read permission
        self.permissions
            .check_read(path)
            .map_err(|e| ModuleError::PermissionDenied(e.to_string()))?;

        // Read the file
        let code = std::fs::read_to_string(path)
            .map_err(|e| ModuleError::ResolutionError(format!("Failed to read file: {}", e)))?;

        // Detect module type from extension
        let module_type = Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(ModuleType::from_extension)
            .unwrap_or(ModuleType::ESModule);

        Ok(ModuleSource {
            specifier: path.to_string(),
            code,
            module_type,
        })
    }

    /// Load a remote module via HTTP
    async fn load_remote(&self, url: &str) -> ModuleResult<ModuleSource> {
        // Check network permission
        self.permissions
            .check_net(url)
            .map_err(|e| ModuleError::PermissionDenied(e.to_string()))?;

        // Parse URL to get hostname
        let parsed = url::Url::parse(url)
            .map_err(|_| ModuleError::InvalidSpecifier(url.to_string()))?;
        let host = parsed.host_str().unwrap_or("unknown");

        // Check permission for specific host
        self.permissions
            .check_net(host)
            .map_err(|e| ModuleError::PermissionDenied(e.to_string()))?;

        // TODO: Implement HTTP fetch
        // For now, return an error
        Err(ModuleError::NetworkError(format!(
            "Remote module loading not yet implemented: {}",
            url
        )))
    }

    /// Parse module dependencies from source code
    pub fn parse_dependencies(&self, source: &ModuleSource) -> Vec<String> {
        let mut dependencies = Vec::new();

        // Simple regex-based parsing (should use proper parser in production)
        // Match: import ... from "..." and import(...)
        let import_patterns = [
            r#"import\s+.*?from\s+['"]([^'"]+)['"]"#,
            r#"import\s*\(\s*['"]([^'"]+)['"]"#,
            r#"export\s+.*?from\s+['"]([^'"]+)['"]"#,
        ];

        for pattern in &import_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.captures_iter(&source.code) {
                    if let Some(specifier) = cap.get(1) {
                        dependencies.push(specifier.as_str().to_string());
                    }
                }
            }
        }

        dependencies
    }

    /// Load and resolve a complete module
    pub async fn load_module(&self, specifier: &str, referrer: Option<&str>) -> ModuleResult<ResolvedModule> {
        // Check for circular dependency
        let loading = self.loading.lock().await;
        if loading.contains(&specifier.to_string()) {
            return Err(ModuleError::CircularDependency(specifier.to_string()));
        }
        drop(loading);

        // Resolve the specifier
        let resolved_specifier = self.resolve(specifier, referrer)?;

        // Check cache
        if self.config.cache_enabled {
            if let Some(cached) = self.cache.get(&resolved_specifier).await {
                return Ok(cached);
            }
        }

        // Mark as loading
        self.loading.lock().await.push(resolved_specifier.clone());

        // Load the source
        let source = self.load(&resolved_specifier).await?;

        // Parse dependencies
        let dependencies = self.parse_dependencies(&source);

        let module = ResolvedModule {
            specifier: resolved_specifier.clone(),
            source,
            dependencies,
        };

        // Cache the module
        if self.config.cache_enabled {
            self.cache.insert(resolved_specifier.clone(), module.clone()).await;
        }

        // Remove from loading list
        self.loading.lock().await.retain(|s| s != &resolved_specifier);

        Ok(module)
    }

    /// Get the module cache
    pub fn cache(&self) -> &ModuleCache {
        &self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_loader() -> ModuleLoader {
        let permissions = Permissions::allow_all();
        let config = ModuleLoaderConfig::default();
        ModuleLoader::new(permissions, config)
    }

    #[test]
    fn test_module_type_detection() {
        assert_eq!(ModuleType::from_extension(".js"), ModuleType::ESModule);
        assert_eq!(ModuleType::from_extension(".mjs"), ModuleType::ESModule);
        assert_eq!(ModuleType::from_extension(".cjs"), ModuleType::CommonJS);
        assert_eq!(ModuleType::from_extension(".json"), ModuleType::JSON);
        assert_eq!(ModuleType::from_extension(".ts"), ModuleType::TypeScript);
        assert_eq!(ModuleType::from_extension(".txt"), ModuleType::Unknown);
    }

    #[test]
    fn test_import_map() {
        let mut map = ImportMap::new("https://example.com/".to_string());
        map.insert("lodash/".to_string(), "https://cdn.example.com/lodash/".to_string());

        let resolved = map.resolve("lodash/map").unwrap();
        assert_eq!(resolved, "https://cdn.example.com/lodash/map");

        let not_found = map.resolve("react");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_import_map_from_json() {
        let json = r#"{"imports": {"react": "https://cdn.example.com/react.js"}}"#;
        let map = ImportMap::from_json(json, "https://example.com/".to_string()).unwrap();

        let resolved = map.resolve("react").unwrap();
        assert_eq!(resolved, "https://cdn.example.com/react.js");
    }

    #[test]
    fn test_resolve_relative_path() {
        let loader = create_test_loader();

        // Relative path with referrer
        let resolved = loader.resolve("./utils.js", Some("/home/user/main.js")).unwrap();
        assert_eq!(resolved, "/home/user/utils.js");

        // Parent directory
        let resolved = loader.resolve("../shared/lib.js", Some("/home/user/main.js")).unwrap();
        assert_eq!(resolved, "/home/shared/lib.js");
    }

    #[test]
    fn test_resolve_absolute_path() {
        let loader = create_test_loader();
        let resolved = loader.resolve("/usr/local/lib.js", None).unwrap();
        assert_eq!(resolved, "/usr/local/lib.js");
    }

    #[test]
    fn test_resolve_remote() {
        let mut config = ModuleLoaderConfig::default();
        config.allow_remote = true;

        let loader = ModuleLoader::new(Permissions::allow_all(), config);

        let resolved = loader.resolve("https://example.com/module.js", None).unwrap();
        assert_eq!(resolved, "https://example.com/module.js");
    }

    #[test]
    fn test_resolve_remote_denied() {
        let mut config = ModuleLoaderConfig::default();
        config.allow_remote = false;

        let loader = ModuleLoader::new(Permissions::allow_all(), config);

        let result = loader.resolve("https://example.com/module.js", None);
        assert!(matches!(result, Err(ModuleError::PermissionDenied(_))));
    }

    #[test]
    fn test_parse_dependencies() {
        let loader = create_test_loader();

        let source = ModuleSource {
            specifier: "test.js".to_string(),
            code: r#"
                import { foo } from './foo.js';
                import bar from './bar.js';
                import('./dynamic.js');
                export { baz } from './baz.js';
            "#
            .to_string(),
            module_type: ModuleType::ESModule,
        };

        let deps = loader.parse_dependencies(&source);
        assert!(deps.contains(&"./foo.js".to_string()));
        assert!(deps.contains(&"./bar.js".to_string()));
        assert!(deps.contains(&"./dynamic.js".to_string()));
        assert!(deps.contains(&"./baz.js".to_string()));
    }

    #[tokio::test]
    async fn test_module_cache() {
        let cache = ModuleCache::new();

        assert!(!cache.contains("test").await);

        let module = ResolvedModule {
            specifier: "test".to_string(),
            source: ModuleSource {
                specifier: "test".to_string(),
                code: "console.log('test');".to_string(),
                module_type: ModuleType::ESModule,
            },
            dependencies: vec![],
        };

        cache.insert("test".to_string(), module.clone()).await;
        assert!(cache.contains("test").await);

        let retrieved = cache.get("test").await.unwrap();
        assert_eq!(retrieved.specifier, "test");
    }

    #[tokio::test]
    async fn test_module_cache_clear() {
        let cache = ModuleCache::new();

        cache.insert("test".to_string(), ResolvedModule {
            specifier: "test".to_string(),
            source: ModuleSource {
                specifier: "test".to_string(),
                code: "test".to_string(),
                module_type: ModuleType::ESModule,
            },
            dependencies: vec![],
        }).await;

        assert!(cache.contains("test").await);
        cache.clear().await;
        assert!(!cache.contains("test").await);
    }
}
