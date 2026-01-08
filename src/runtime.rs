//! V8 JavaScript Runtime Core
//!
//! This module provides the main runtime implementation using the V8 engine.
//! It handles V8 initialization, isolate management, and JavaScript execution.

use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;

use v8::{CreateParams, OwnedIsolate, Platform, Script};

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

/// Main JavaScript runtime
///
/// Each runtime instance has its own V8 isolate and context,
/// providing complete isolation between instances.
pub struct JsRuntime {
    /// V8 isolate (owns the JavaScript heap and manages execution)
    isolate: OwnedIsolate,
    /// Runtime permissions
    permissions: Permissions,
    /// Runtime configuration
    config: RuntimeConfig,
    /// Execution statistics
    stats: Rc<RefCell<RuntimeStats>>,
    /// Unique identifier for this runtime
    id: String,
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

        Ok(Self {
            isolate,
            permissions,
            config,
            stats: Rc::new(RefCell::new(RuntimeStats::default())),
            id,
        })
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
        let scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Context::new(scope);
        let scope = &mut v8::ContextScope::new(scope, context);

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
