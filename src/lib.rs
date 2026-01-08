//! Ferrum - A simple, secure, and modern JavaScript/TypeScript Runtime
//!
//! This library provides the core functionality for running JavaScript and TypeScript
//! code outside the browser, with a focus on security and developer experience.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

pub mod cli;
pub mod module_loader;
pub mod ops;
pub mod permissions;
pub mod repl;
pub mod runtime;

// Re-exports for convenience
pub use cli::{parse_args, Cli, Commands};
pub use module_loader::{ImportMap, ModuleLoader, ModuleLoaderConfig};
pub use permissions::{Permissions, ReadPermission, WritePermission, NetPermission, EnvPermission, RunPermission};
pub use repl::{Repl, ReplConfig, start_repl};
pub use runtime::{JsRuntime, RuntimeConfig, RuntimeError, RuntimeResult};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration
pub fn default_runtime_config() -> RuntimeConfig {
    RuntimeConfig::default()
}

/// Create a runtime with default permissions
pub fn create_runtime() -> runtime::RuntimeResult<JsRuntime> {
    let config = RuntimeConfig::default();
    let permissions = Permissions::default();
    JsRuntime::new(config, permissions)
}

/// Create a runtime with all permissions granted
pub fn create_unsafe_runtime() -> runtime::RuntimeResult<JsRuntime> {
    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    JsRuntime::new(config, permissions)
}

/// Initialize the V8 platform
///
/// This must be called once before creating any runtime instances.
/// This function is thread-safe and will only initialize V8 once.
pub fn init_v8() {
    runtime::init_v8_platform();
}

/// Shutdown the V8 platform
///
/// # Safety
/// Should only be called when no runtimes are active.
pub unsafe fn shutdown_v8() {
    runtime::shutdown_v8_platform();
}
