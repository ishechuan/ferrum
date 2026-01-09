//! V8 JavaScript Runtime Core
//!
//! This module provides the main runtime implementation using the V8 engine.
//! It handles V8 initialization, isolate management, and JavaScript execution.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use thiserror::Error;

use v8::{CreateParams, Module, OwnedIsolate, Platform, Script};

use crate::module_loader::{ModuleLoader, ModuleLoaderConfig};
use crate::ops::bindings::bootstrap_globals;
use crate::ops::dispatch::OpRegistry;
use crate::permissions::Permissions;

/// Errors that can occur during runtime operations
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// V8 JavaScript execution error
    #[error("V8 execution error: {0}")]
    ExecutionError(String),

    /// Script compilation error
    #[error("Script compilation error: {0}")]
    CompilationError(String),

    /// Runtime initialization error
    #[error("Runtime initialization error: {0}")]
    InitializationError(String),

    /// Permission denied for operation
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Operation timeout
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Module loading error
    #[error("Module error: {0}")]
    ModuleError(String),

    /// Unknown error occurred
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type for runtime operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Global V8 platform - must be initialized once per process
/// Using OnceLock for thread-safe one-time initialization
static PLATFORM: std::sync::OnceLock<v8::SharedRef<Platform>> = std::sync::OnceLock::new();

/// Initialize the V8 platform (must be called before any runtime operations)
///
/// This function is thread-safe and will only initialize the platform once.
/// Subsequent calls will return the existing platform reference.
pub fn init_v8_platform() {
    PLATFORM.get_or_init(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        // Initialize V8 with the platform
        v8::V8::initialize_platform(platform.clone());
        v8::V8::initialize();
        platform
    });
}

/// Check if the V8 platform has been initialized
pub fn is_v8_platform_initialized() -> bool {
    PLATFORM.get().is_some()
}

/// Shutdown the V8 platform
///
/// # Safety
/// Should only be called when no runtimes are active.
/// This is primarily used for testing and cleanup.
pub unsafe fn shutdown_v8_platform() {
    if PLATFORM.get().is_some() {
        v8::V8::dispose();
        // Note: We don't remove the platform from OnceLock as it's not designed for that
        // This function is mainly for test cleanup
    }
}

/// Configuration for the JavaScript runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Whether to allow native modules
    pub allow_native: bool,
    /// Maximum stack size in bytes (0 = no limit)
    pub max_stack_size: usize,
    /// Execution timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u64,
    /// Whether to enable inspector for debugging
    pub enable_inspector: bool,
    /// Initial heap size in MB
    pub initial_heap_size: usize,
    /// Maximum heap size in MB (0 = no limit)
    pub max_heap_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            allow_native: false,
            max_stack_size: 0,
            timeout_ms: 0,
            enable_inspector: false,
            initial_heap_size: 8,
            max_heap_size: 0,
        }
    }
}

/// JavaScript execution statistics
#[derive(Debug, Default, Clone)]
pub struct RuntimeStats {
    /// Number of scripts executed
    pub scripts_executed: usize,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Number of errors encountered
    pub error_count: usize,
    /// Current memory usage in bytes
    pub memory_usage_bytes: usize,
}

/// Shared context passed to V8 callbacks via External
///
/// This struct contains references to the runtime's state that
/// ops need access to (permissions, registry, etc.).
///
/// # Note
///
/// The context is stored in Arc<Mutex<>> to allow safe sharing across
/// V8 callbacks which may execute from different threads.
pub struct RuntimeContext {
    /// Runtime permissions (checked by ops)
    pub permissions: Arc<Mutex<Permissions>>,
    /// Operation registry (for dispatching native ops)
    pub registry: Arc<Mutex<OpRegistry>>,
}

impl RuntimeContext {
    /// Create a new runtime context
    #[must_use]
    pub fn new(permissions: Permissions, registry: OpRegistry) -> Self {
        Self {
            permissions: Arc::new(Mutex::new(permissions)),
            registry: Arc::new(Mutex::new(registry)),
        }
    }
}

/// Main JavaScript runtime
///
/// Each runtime instance has its own V8 isolate and context,
/// providing complete isolation between instances.
pub struct JsRuntime {
    /// V8 isolate (owns the JavaScript heap and manages execution)
    isolate: OwnedIsolate,
    /// Shared runtime context (passed to V8 callbacks)
    rt_context: Arc<RuntimeContext>,
    /// Runtime permissions
    permissions: Permissions,
    /// Runtime configuration
    config: RuntimeConfig,
    /// Execution statistics
    stats: Rc<RefCell<RuntimeStats>>,
    /// Unique identifier for this runtime
    id: String,
    /// Module loader for ES module support
    module_loader: Option<ModuleLoader>,
    /// V8 module cache (maps specifier to v8::Module)
    module_cache: HashMap<String, v8::Global<Module>>,
}

impl JsRuntime {
    /// Create a new JavaScript runtime instance
    ///
    /// # Arguments
    /// * `config` - Runtime configuration options
    /// * `permissions` - Permission set for this runtime
    ///
    /// # Returns
    /// A new runtime instance or an error if initialization fails
    pub fn new(config: RuntimeConfig, permissions: Permissions) -> RuntimeResult<Self> {
        // Create V8 isolate with configured parameters
        let params = CreateParams::default();

        // Create isolate using v8 API
        let isolate = v8::Isolate::new(params);
        let id = uuid::Uuid::new_v4().to_string();

        tracing::debug!("Created new runtime instance: {}", id);

        // Create operation registry for potential future use
        let registry = OpRegistry::new();

        // Create shared runtime context
        // Note: We pass the registry to RuntimeContext for potential future use
        let rt_context = Arc::new(RuntimeContext::new(permissions.clone(), registry));

        Ok(Self {
            isolate,
            rt_context,
            permissions,
            config,
            stats: Rc::new(RefCell::new(RuntimeStats::default())),
            id,
            module_loader: None,
            module_cache: HashMap::new(),
        })
    }

    /// Create a new JavaScript runtime instance with module loading support
    ///
    /// # Arguments
    /// * `config` - Runtime configuration options
    /// * `permissions` - Permission set for this runtime
    /// * `module_config` - Module loader configuration
    ///
    /// # Returns
    /// A new runtime instance with module loading enabled
    pub fn with_module_loader(
        config: RuntimeConfig,
        permissions: Permissions,
        module_config: ModuleLoaderConfig,
    ) -> RuntimeResult<Self> {
        // Create V8 isolate with configured parameters
        let params = CreateParams::default();

        // Create isolate using v8 API
        let isolate = v8::Isolate::new(params);
        let id = uuid::Uuid::new_v4().to_string();

        tracing::debug!("Created new runtime instance with module loader: {}", id);

        // Create operation registry for potential future use
        let registry = OpRegistry::new();

        // Create shared runtime context
        let rt_context = Arc::new(RuntimeContext::new(permissions.clone(), registry));

        // Create module loader
        let module_loader = ModuleLoader::new(permissions.clone(), module_config);

        Ok(Self {
            isolate,
            rt_context,
            permissions,
            config,
            stats: Rc::new(RefCell::new(RuntimeStats::default())),
            id,
            module_loader: Some(module_loader),
            module_cache: HashMap::new(),
        })
    }

    /// Set up module loader for this runtime
    ///
    /// This can be used to add module loading support to an existing runtime.
    ///
    /// # Arguments
    /// * `module_config` - Module loader configuration
    pub fn setup_module_loader(&mut self, module_config: ModuleLoaderConfig) {
        let module_loader = ModuleLoader::new(self.permissions.clone(), module_config);
        self.module_loader = Some(module_loader);
        tracing::debug!("Module loader set up for runtime: {}", self.id);
    }

    /// Check if module loader is available
    pub fn has_module_loader(&self) -> bool {
        self.module_loader.is_some()
    }

    /// Get the runtime ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the current permissions
    pub fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    /// Get mutable reference to permissions
    pub fn permissions_mut(&mut self) -> &mut Permissions {
        &mut self.permissions
    }

    /// Get the runtime configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Get execution statistics
    pub fn stats(&self) -> RuntimeStats {
        self.stats.borrow().clone()
    }

    /// Execute JavaScript code
    ///
    /// # Arguments
    /// * `code` - JavaScript source code to execute
    /// * `filename` - Optional filename for error reporting
    ///
    /// # Returns
    /// The result of the last expression evaluated
    pub fn execute(&mut self, code: &str, _filename: Option<&str>) -> RuntimeResult<String> {
        // Clone the runtime context before creating the scope
        // This avoids borrow checker issues with the mutable borrow of self.isolate
        let rt_context = self.rt_context.clone();

        let scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Context::new(scope);
        let scope = &mut v8::ContextScope::new(scope, context);

        // Bootstrap global APIs (console, Deno, etc.)
        // This must happen after context creation and before script execution
        if let Err(e) = bootstrap_globals(scope, rt_context) {
            tracing::error!("Failed to bootstrap globals: {}", e);
            return Err(RuntimeError::InitializationError(format!(
                "Failed to bootstrap globals: {}",
                e
            )));
        }

        // Compile the script
        let source = v8::String::new(scope, code)
            .ok_or_else(|| RuntimeError::CompilationError("Failed to create source string".into()))?;

        let script = Script::compile(scope, source, None)
            .ok_or_else(|| RuntimeError::CompilationError("Script compilation failed".into()))?;

        // Run the script
        let result = script.run(scope)
            .ok_or_else(|| RuntimeError::ExecutionError("Script execution failed".into()))?;

        // Convert result to string
        let result_str = result.to_rust_string_lossy(scope);

        // Update stats
        self.stats.borrow_mut().scripts_executed += 1;

        Ok(result_str)
    }

    /// Execute a script from a file
    ///
    /// # Arguments
    /// * `path` - Path to the JavaScript file
    ///
    /// # Returns
    /// The result of the last expression evaluated
    pub fn execute_file(&mut self, path: &str) -> RuntimeResult<String> {
        // Check read permission
        self.permissions
            .check_read(path)
            .map_err(|e| RuntimeError::PermissionDenied(e.to_string()))?;

        // Read the file
        let code = std::fs::read_to_string(path)
            .map_err(|e| RuntimeError::ExecutionError(format!("Failed to read file: {}", e)))?;

        self.execute(&code, Some(path))
    }

    /// Execute an ES module from a file path
    ///
    /// This method uses V8's Module API to compile and execute ES modules.
    /// It handles module resolution, dependency loading, and instantiation.
    ///
    /// # Arguments
    /// * `specifier` - Module specifier (file path or URL)
    ///
    /// # Returns
    /// The result of module evaluation
    pub fn execute_module(&mut self, specifier: &str) -> RuntimeResult<String> {
        // Check if module loader is available
        let module_loader = self.module_loader.as_ref()
            .ok_or_else(|| RuntimeError::ModuleError("Module loader not initialized".to_string()))?;

        // Clone the runtime context before creating the scope
        let rt_context = self.rt_context.clone();

        // Get a mutable reference to the module cache
        let module_cache = &mut self.module_cache;

        let scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Context::new(scope);
        let scope = &mut v8::ContextScope::new(scope, context);

        // Bootstrap global APIs (console, Deno, etc.)
        if let Err(e) = bootstrap_globals(scope, rt_context) {
            tracing::error!("Failed to bootstrap globals: {}", e);
            return Err(RuntimeError::InitializationError(format!(
                "Failed to bootstrap globals: {}",
                e
            )));
        }

        // Load and compile the module
        let (module, _resolved_specifier) = Self::compile_module_impl(
            scope,
            module_loader,
            module_cache,
            specifier,
            None,
        )?;

        // Check for compilation errors
        if module.get_status() == v8::ModuleStatus::Errored {
            let exception = module.get_exception();
            let error_msg = exception.to_rust_string_lossy(scope);
            return Err(RuntimeError::CompilationError(format!("Module compilation error: {}", error_msg)));
        }

        // Instantiate the module (this resolves all dependencies)
        // For simple modules without imports, this should succeed
        // The callback will be called for each import, but we return null for all imports
        // which means modules with imports will fail during instantiation
        let instantiate_result = module.instantiate_module(scope, Self::module_resolve_callback);

        // instantiate_module returns Option<bool>
        // None means success (no imports), Some(true) means success with imports,
        // Some(false) means failure
        if instantiate_result == Some(false) {
            // Check for instantiation errors
            if module.get_status() == v8::ModuleStatus::Errored {
                let exception = module.get_exception();
                let error_msg = exception.to_rust_string_lossy(scope);
                return Err(RuntimeError::ModuleError(format!("Module instantiation error: {}", error_msg)));
            }
            return Err(RuntimeError::ModuleError("Module instantiation failed - modules with imports are not yet supported".to_string()));
        }

        // Evaluate the module
        let result = module.evaluate(scope);
        let result = match result {
            Some(r) => r,
            None => {
                // Check for evaluation errors
                if module.get_status() == v8::ModuleStatus::Errored {
                    let exception = module.get_exception();
                    let error_msg = exception.to_rust_string_lossy(scope);
                    return Err(RuntimeError::ExecutionError(format!("Module evaluation error: {}", error_msg)));
                }
                return Err(RuntimeError::ExecutionError("Module evaluation failed".to_string()));
            }
        };

        // Check if the result is a Promise (top-level await)
        // ES modules with async operations will return a Promise
        if result.is_promise() {
            // For now, we'll return the Promise object as a string
            // In a production system, we'd want to wait for the promise to resolve
            // and then return the actual result
            let result_str = result.to_rust_string_lossy(scope);

            // Update stats
            self.stats.borrow_mut().scripts_executed += 1;

            Ok(result_str)
        } else {
            // Convert result to string
            let result_str = result.to_rust_string_lossy(scope);

            // Update stats
            self.stats.borrow_mut().scripts_executed += 1;

            Ok(result_str)
        }
    }

    /// Compile a V8 module from a specifier (implementation)
    ///
    /// This is a static helper method that compiles a module using the ModuleLoader.
    /// It uses the module loader to resolve and load module source code.
    ///
    /// # Arguments
    /// * `scope` - V8 handle scope
    /// * `module_loader` - Module loader instance
    /// * `module_cache` - Module cache for storing compiled modules
    /// * `specifier` - Module specifier
    /// * `referrer` - Referrer module (for relative imports)
    ///
    /// # Returns
    /// Compiled V8 module and its resolved specifier
    fn compile_module_impl<'s>(
        scope: &mut v8::ContextScope<'s, v8::HandleScope>,
        module_loader: &ModuleLoader,
        module_cache: &mut HashMap<String, v8::Global<Module>>,
        specifier: &str,
        referrer: Option<&str>,
    ) -> RuntimeResult<(v8::Local<'s, Module>, String)> {
        // Resolve the specifier
        let resolved_specifier = module_loader.resolve(specifier, referrer)
            .map_err(|e| RuntimeError::ModuleError(format!("Failed to resolve '{}': {}", specifier, e)))?;

        // Check if module is already cached
        if let Some(cached_module) = module_cache.get(&resolved_specifier) {
            let local = v8::Local::new(scope, cached_module);
            return Ok((local, resolved_specifier));
        }

        // Load the module source (block on async)
        let resolved_module = tokio::runtime::Runtime::new()
            .map_err(|e| RuntimeError::InitializationError(format!("Failed to create tokio runtime: {}", e)))?
            .block_on(module_loader.load_module(&resolved_specifier, referrer))
            .map_err(|e| RuntimeError::ModuleError(format!("Failed to load module '{}': {}", specifier, e)))?;

        // Create V8 source string
        let source_str = v8::String::new(scope, &resolved_module.source.code)
            .ok_or_else(|| RuntimeError::CompilationError(
                format!("Failed to create source string for '{}'", specifier)
            ))?;

        // Create resource name for error reporting
        let resource_name = v8::String::new(scope, &resolved_specifier)
            .ok_or_else(|| RuntimeError::CompilationError(
                format!("Failed to create resource name for '{}'", specifier)
            ))?;

        // Create undefined value before creating ScriptOrigin to avoid borrow issues
        let undefined_value = v8::undefined(scope);

        // Create ScriptOrigin for the module
        // The ScriptOrigin constructor has a specific signature:
        // new(scope, resource_name, resource_line_offset, resource_column_offset,
        //     resource_is_shared_cross_origin, script_id, source_map_url,
        //     is_opaque, is_wam, is_module)
        let origin = v8::ScriptOrigin::new(
            scope,
            resource_name.into(),
            0,
            0,
            false,
            -1,
            undefined_value.into(),
            false,
            false,
            true,
        );

        // Create ScriptCompiler source
        let source = v8::script_compiler::Source::new(source_str, Some(&origin));

        // Compile the module using ScriptCompiler
        let module = v8::script_compiler::compile_module(scope, source)
            .ok_or_else(|| RuntimeError::CompilationError(
                format!("Failed to compile module '{}'", specifier)
            ))?;

        // Cache the compiled module (convert to Global for storage)
        let global_module = v8::Global::new(scope, module);
        module_cache.insert(resolved_specifier.clone(), global_module);

        Ok((module, resolved_specifier))
    }

    /// Module resolution callback for V8
    ///
    /// This callback is invoked by V8 when it needs to resolve a module import.
    ///
    /// # Note
    /// This is a simplified implementation that always returns null.
    /// A production implementation would:
    /// 1. Get the isolate from the context
    /// 2. Convert the specifier to a Rust string
    /// 3. Look up the module in our cache
    /// 4. Return the module if found
    /// 5. Otherwise, compile and cache it
    ///
    /// For now, modules with imports will fail during instantiation.
    fn module_resolve_callback<'a>(
        _context: v8::Local<'a, v8::Context>,
        _specifier: v8::Local<'a, v8::String>,
        _import_attributes: v8::Local<'a, v8::FixedArray>,
        _module_request: v8::Local<'a, Module>,
    ) -> Option<v8::Local<'a, Module>> {
        // Return None to indicate module not found
        // This means modules with imports will fail during instantiation
        None
    }

    /// Get memory usage information
    pub fn get_memory_usage(&self) -> RuntimeResult<(usize, usize)> {
        // V8 API for heap statistics may vary by version
        // Return a placeholder for now
        Ok((0, 0))
    }

    /// Perform garbage collection
    pub fn gc(&mut self) {
        // Low memory notification triggers GC in V8
        self.isolate.low_memory_notification();
    }
}

impl Drop for JsRuntime {
    fn drop(&mut self) {
        tracing::debug!("Dropping runtime instance: {}", self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Initialize V8 platform for tests (thread-safe, one-time initialization)
    fn init_v8_for_tests() {
        INIT.call_once(|| {
            init_v8_platform();
        });
    }

    fn init_test_runtime() -> JsRuntime {
        init_v8_for_tests();

        let config = RuntimeConfig::default();
        let permissions = Permissions::allow_all();
        JsRuntime::new(config, permissions).unwrap()
    }

    #[test]
    fn test_runtime_creation() {
        let rt = init_test_runtime();
        assert!(!rt.id().is_empty());
    }

    #[test]
    fn test_simple_execution() {
        let mut rt = init_test_runtime();
        let result = rt.execute("1 + 1", None).unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_syntax_error() {
        let mut rt = init_test_runtime();
        let result = rt.execute("syntax error", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_error() {
        let mut rt = init_test_runtime();
        let result = rt.execute("throw new Error('test error')", None);
        assert!(matches!(result, Err(RuntimeError::ExecutionError(_))));
    }

    #[test]
    fn test_permission_denied() {
        init_v8_for_tests(); // Ensure V8 is initialized
        let config = RuntimeConfig::default();
        let permissions = Permissions::default(); // All denied

        let mut rt = JsRuntime::new(config, permissions).unwrap();
        let result = rt.execute_file("/etc/passwd");
        assert!(matches!(result, Err(RuntimeError::PermissionDenied(_))));
    }

    #[test]
    fn test_stats_tracking() {
        let mut rt = init_test_runtime();

        let stats_before = rt.stats();
        assert_eq!(stats_before.scripts_executed, 0);

        rt.execute("1 + 1", None).unwrap();
        rt.execute("2 + 2", None).unwrap();

        let stats_after = rt.stats();
        assert_eq!(stats_after.scripts_executed, 2);
    }
}
