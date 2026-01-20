//! Timer V8-Rust Bridge Bindings
//!
//! This module provides V8 function callbacks for timer operations (setTimeout, setInterval, etc.).
//! These bindings allow JavaScript code to use the global setTimeout/setInterval functions.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use v8;

use crate::ops::timers::{TimerId, TimerRegistry};

thread_local! {
    static TIMER_REGISTRY: RefCell<Option<Arc<TimerRegistry>>> = RefCell::new(None);
    static TIMER_CALLBACK_TX: RefCell<Option<std::sync::mpsc::Sender<(TimerId, String)>>> = RefCell::new(None);
}

/// Counter for generating unique timer IDs at the JavaScript binding layer
static TIMER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Set the timer registry and callback channel for thread-local storage
pub fn set_timer_context(
    registry: Arc<TimerRegistry>,
    callback_tx: std::sync::mpsc::Sender<(TimerId, String)>,
) {
    TIMER_REGISTRY.with(|r| {
        *r.borrow_mut() = Some(registry);
    });
    TIMER_CALLBACK_TX.with(|t| {
        *t.borrow_mut() = Some(callback_tx);
    });
}

/// Clear the timer context from thread-local storage
pub fn clear_timer_context() {
    TIMER_REGISTRY.with(|r| {
        *r.borrow_mut() = None;
    });
    TIMER_CALLBACK_TX.with(|t| {
        *t.borrow_mut() = None;
    });
}

/// Get the timer registry from thread-local storage
fn get_timer_registry() -> Option<Arc<TimerRegistry>> {
    TIMER_REGISTRY.with(|r| r.borrow().clone())
}

/// Get the callback channel from thread-local storage
fn get_callback_tx() -> Option<std::sync::mpsc::Sender<(TimerId, String)>> {
    TIMER_CALLBACK_TX.with(|t| t.borrow().clone())
}

fn throw_error(scope: &mut v8::HandleScope, message: &str) {
    let message_str = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::error(scope, message_str);
    scope.throw_exception(error);
}

/// Generate a unique timer ID for JavaScript
fn generate_js_timer_id() -> u64 {
    TIMER_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// setTimeout implementation
///
/// Executes a callback after a specified delay.
///
/// # JavaScript Signature
/// ```javascript
/// function setTimeout(callback: Function, delay: number, ...args: any[]): number
/// ```
///
/// # Example
/// ```javascript
/// setTimeout(() => console.log("Hello"), 1000);
/// ```
pub fn op_set_timeout(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let registry = match get_timer_registry() {
        Some(r) => r,
        None => {
            throw_error(scope, "Timer registry not initialized");
            return;
        }
    };

    let callback_tx = match get_callback_tx() {
        Some(t) => t,
        None => {
            throw_error(scope, "Timer callback channel not initialized");
            return;
        }
    };

    if args.length() < 2 {
        throw_error(
            scope,
            "setTimeout requires at least 2 arguments (callback, delay)",
        );
        return;
    }

    let callback = args.get(0);
    if !callback.is_function() {
        throw_error(scope, "First argument to setTimeout must be a function");
        return;
    }

    let delay = args.get(1);
    if !delay.is_number() {
        throw_error(scope, "Second argument to setTimeout must be a number");
        return;
    }

    let delay_ms = delay.number_value(scope).unwrap_or(0.0) as u64;

    let callback_fn = callback.to_rust_string_lossy(scope);
    let registry_clone = registry.clone();
    let callback_tx_clone = callback_tx.clone();
    let js_timer_id = generate_js_timer_id();

    let _ = tokio::spawn(async move {
        registry_clone
            .set_timeout(
                delay_ms,
                Box::new(move || {
                    let _ = callback_tx_clone.send((js_timer_id, callback_fn.clone()));
                }),
            )
            .await;
    });

    rv.set_uint32(js_timer_id as u32);
}

/// setInterval implementation
///
/// Executes a callback repeatedly at a specified interval.
///
/// # JavaScript Signature
/// ```javascript
/// function setInterval(callback: Function, delay: number, ...args: any[]): number
/// ```
///
/// # Example
/// ```javascript
/// setInterval(() => console.log("Tick"), 1000);
/// ```
pub fn op_set_interval(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let registry = match get_timer_registry() {
        Some(r) => r,
        None => {
            throw_error(scope, "Timer registry not initialized");
            return;
        }
    };

    let callback_tx = match get_callback_tx() {
        Some(t) => t,
        None => {
            throw_error(scope, "Timer callback channel not initialized");
            return;
        }
    };

    if args.length() < 2 {
        throw_error(
            scope,
            "setInterval requires at least 2 arguments (callback, delay)",
        );
        return;
    }

    let callback = args.get(0);
    if !callback.is_function() {
        throw_error(scope, "First argument to setInterval must be a function");
        return;
    }

    let delay = args.get(1);
    if !delay.is_number() {
        throw_error(scope, "Second argument to setInterval must be a number");
        return;
    }

    let delay_ms = delay.number_value(scope).unwrap_or(0.0) as u64;

    let callback_fn = callback.to_rust_string_lossy(scope);
    let registry_clone = registry.clone();
    let callback_tx_clone = callback_tx.clone();
    let js_timer_id = generate_js_timer_id();

    let _ = tokio::spawn(async move {
        registry_clone
            .set_interval(
                delay_ms,
                Box::new(move || {
                    let _ = callback_tx_clone.send((js_timer_id, callback_fn.clone()));
                }),
            )
            .await;
    });

    rv.set_uint32(js_timer_id as u32);
}

/// clearTimeout implementation
///
/// Cancels a timeout created by setTimeout.
///
/// # JavaScript Signature
/// ```javascript
/// function clearTimeout(id: number): void
/// ```
///
/// # Example
/// ```javascript
/// const id = setTimeout(() => console.log("Hello"), 1000);
/// clearTimeout(id);
/// ```
pub fn op_clear_timeout(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let registry = match get_timer_registry() {
        Some(r) => r,
        None => {
            throw_error(scope, "Timer registry not initialized");
            return;
        }
    };

    if args.length() < 1 {
        throw_error(scope, "clearTimeout requires at least 1 argument (id)");
        return;
    }

    let id_arg = args.get(0);
    if !id_arg.is_number() {
        throw_error(scope, "Argument to clearTimeout must be a number");
        return;
    }

    let id = id_arg.number_value(scope).unwrap_or(0.0) as u64;

    let registry_clone = registry.clone();
    let _ = tokio::spawn(async move {
        let _ = registry_clone.clear(id).await;
    });

    rv.set_undefined();
}

/// clearInterval implementation
///
/// Cancels an interval created by setInterval.
///
/// # JavaScript Signature
/// ```javascript
/// function clearInterval(id: number): void
/// ```
///
/// # Example
/// ```javascript
/// const id = setInterval(() => console.log("Tick"), 1000);
/// clearInterval(id);
/// ```
pub fn op_clear_interval(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let registry = match get_timer_registry() {
        Some(r) => r,
        None => {
            throw_error(scope, "Timer registry not initialized");
            return;
        }
    };

    if args.length() < 1 {
        throw_error(scope, "clearInterval requires at least 1 argument (id)");
        return;
    }

    let id_arg = args.get(0);
    if !id_arg.is_number() {
        throw_error(scope, "Argument to clearInterval must be a number");
        return;
    }

    let id = id_arg.number_value(scope).unwrap_or(0.0) as u64;

    let registry_clone = registry.clone();
    let _ = tokio::spawn(async move {
        let _ = registry_clone.clear(id).await;
    });

    rv.set_undefined();
}

/// Execute pending timer callbacks
///
/// This function should be called during the event loop to process
/// timer callbacks that were triggered by the timer system.
///
/// # Arguments
///
/// * `scope` - The V8 handle scope
/// * `receiver` - Receiver for timer callback events
///
/// # Returns
///
/// Number of callbacks executed
pub fn execute_timer_callbacks(
    scope: &mut v8::HandleScope,
    receiver: &std::sync::mpsc::Receiver<(TimerId, String)>,
) -> usize {
    let mut count = 0;
    while let Ok((_timer_id, callback_source)) = receiver.try_recv() {
        let source = v8::String::new(scope, &callback_source).unwrap();
        if let Some(script) = v8::Script::compile(scope, source, None) {
            let _ = script.run(scope);
        }
        count += 1;
    }
    count
}

/// Bootstrap global timer functions (setTimeout, setInterval, clearTimeout, clearInterval)
///
/// This function adds the global timer functions to the JavaScript context.
///
/// # Arguments
///
/// * `scope` - The V8 handle scope (must be a ContextScope)
/// * `registry` - The timer registry to use for timer operations
/// * `callback_tx` - Channel sender for timer callbacks
pub fn bootstrap_timer_globals(
    scope: &mut v8::HandleScope,
    registry: Arc<TimerRegistry>,
    callback_tx: std::sync::mpsc::Sender<(TimerId, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Bootstrapping global timer functions");

    set_timer_context(registry, callback_tx);

    let context = scope.get_current_context();
    let global = context.global(scope);

    {
        let scope2 = &mut v8::HandleScope::new(scope);

        // setTimeout
        let name = v8::String::new(scope2, "setTimeout").unwrap();
        let func = v8::Function::new(scope2, op_set_timeout).unwrap();
        global.set(scope2, name.into(), func.into());

        // setInterval
        let name = v8::String::new(scope2, "setInterval").unwrap();
        let func = v8::Function::new(scope2, op_set_interval).unwrap();
        global.set(scope2, name.into(), func.into());

        // clearTimeout
        let name = v8::String::new(scope2, "clearTimeout").unwrap();
        let func = v8::Function::new(scope2, op_clear_timeout).unwrap();
        global.set(scope2, name.into(), func.into());

        // clearInterval
        let name = v8::String::new(scope2, "clearInterval").unwrap();
        let func = v8::Function::new(scope2, op_clear_interval).unwrap();
        global.set(scope2, name.into(), func.into());
    }

    tracing::debug!("Registered global timer functions");

    Ok(())
}

/// Clear timer globals and cleanup
pub fn clear_timer_globals() {
    clear_timer_context();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_timer_context_set_clear() {
        let registry = Arc::new(TimerRegistry::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        set_timer_context(registry.clone(), tx.clone());

        assert!(get_timer_registry().is_some());
        assert!(get_callback_tx().is_some());

        clear_timer_globals();

        assert!(get_timer_registry().is_none());
        assert!(get_callback_tx().is_none());
    }

    #[test]
    fn test_timer_id_generation() {
        let id1 = generate_js_timer_id();
        let id2 = generate_js_timer_id();
        assert_ne!(id1, id2);
        assert!(id2 > id1);
    }
}
