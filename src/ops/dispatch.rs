//! Operation Registry and Dispatch System
//!
//! This module provides the registry and dispatch system for native operations
//! that can be called from JavaScript via the V8-Rust bridge.

use std::collections::HashMap;
use v8;

/// Operation registry for managing native operations
///
/// This registry stores V8 function callbacks that can be invoked from JavaScript.
/// For the MVP, we use a simple HashMap with function pointers.
///
/// # Note
///
/// V8 callbacks use a low-level raw pointer API. The callback signature is:
/// `extern "C" fn(*const v8::FunctionCallbackInfo)`
///
/// Inside the callback, you can use `v8::FunctionCallbackInfo::from()` to convert
/// the raw pointer to the higher-level API.
///
/// # Example
///
/// ```no_run
/// use ferrum::ops::dispatch::OpRegistry;
/// use v8::FunctionCallbackInfo;
///
/// let mut registry = OpRegistry::new();
///
/// // Register an operation with an extern "C" function
/// extern "C" fn my_op(info: *const FunctionCallbackInfo) {
///     let info = unsafe { &*info };
///     // Use info to access arguments and return values
/// }
///
/// registry.register("my_op".to_string(), my_op);
/// ```
#[derive(Clone)]
pub struct OpRegistry {
    /// Map of operation names to their V8 function callbacks
    ops: HashMap<String, v8::FunctionCallback>,
}

impl Default for OpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OpRegistry {
    /// Create a new empty operation registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            ops: HashMap::new(),
        }
    }

    /// Register a new operation
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the operation (as it will be called from JavaScript)
    /// * `callback` - V8 function callback to handle the operation
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ferrum::ops::dispatch::OpRegistry;
    /// let mut registry = OpRegistry::new();
    /// // Register a callback - see src/ops/bindings.rs for examples
    /// // of proper V8 callback implementations
    /// ```
    pub fn register(&mut self, name: String, callback: v8::FunctionCallback) {
        self.ops.insert(name, callback);
    }

    /// Get a registered operation by name
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the operation to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(&FunctionCallback)` if the operation exists, `None` otherwise.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&v8::FunctionCallback> {
        self.ops.get(name)
    }

    /// Check if an operation is registered
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the operation to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the operation exists, `false` otherwise.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.ops.contains_key(name)
    }

    /// Get the number of registered operations
    #[must_use]
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check if the registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Remove an operation from the registry
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the operation to remove
    ///
    /// # Returns
    ///
    /// Returns `Some(FunctionCallback)` if the operation was removed,
    /// `None` if it didn't exist.
    pub fn unregister(&mut self, name: &str) -> Option<v8::FunctionCallback> {
        self.ops.remove(name)
    }

    /// Clear all operations from the registry
    pub fn clear(&mut self) {
        self.ops.clear();
    }

    /// Get an iterator over all registered operation names
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.ops.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dummy callback for testing with the correct V8 signature
    extern "C" fn dummy_callback(_info: *const v8::FunctionCallbackInfo) {
        // Stub implementation for testing
    }

    // Note: We can't easily test V8 function callbacks without a full V8 setup,
    // so we'll test the registry logic itself

    #[test]
    fn test_registry_new() {
        let registry = OpRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = OpRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_register() {
        let mut registry = OpRegistry::new();

        registry.register("test_op".to_string(), dummy_callback);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_op"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = OpRegistry::new();

        registry.register("get_test".to_string(), dummy_callback);

        assert!(registry.get("get_test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = OpRegistry::new();

        registry.register("to_remove".to_string(), dummy_callback);

        assert!(registry.contains("to_remove"));

        let removed = registry.unregister("to_remove");
        assert!(removed.is_some());
        assert!(!registry.contains("to_remove"));

        // Removing non-existent returns None
        assert!(registry.unregister("to_remove").is_none());
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = OpRegistry::new();

        registry.register("op1".to_string(), dummy_callback);
        registry.register("op2".to_string(), dummy_callback);

        assert_eq!(registry.len(), 2);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_names() {
        let mut registry = OpRegistry::new();

        registry.register("op_a".to_string(), dummy_callback);
        registry.register("op_b".to_string(), dummy_callback);
        registry.register("op_c".to_string(), dummy_callback);

        let names: Vec<&String> = registry.names().collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&&"op_a".to_string()));
        assert!(names.contains(&&"op_b".to_string()));
        assert!(names.contains(&&"op_c".to_string()));
    }

    #[test]
    fn test_registry_multiple_ops() {
        let mut registry = OpRegistry::new();

        for i in 0..10 {
            registry.register(format!("op_{}", i), dummy_callback);
        }

        assert_eq!(registry.len(), 10);

        for i in 0..10 {
            assert!(registry.contains(&format!("op_{}", i)));
        }
    }
}
