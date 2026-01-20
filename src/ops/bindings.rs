//! V8-Rust Bridge Bindings
//!
//! This module provides V8 function callbacks that expose Rust operations to JavaScript.
//! These bridges allow JavaScript code to call native functions like console.log and
//! Deno.readTextFile through the V8 API.
//!
//! # Architecture
//!
//! The bridge works by:
//! 1. Storing RuntimeContext in thread-local storage during execution
//! 2. V8 callbacks access the context through thread-local storage
//! 3. Callbacks extract arguments, check permissions, execute Rust code, and return values
//!
//! # Thread Safety
//!
//! RuntimeContext is wrapped in Arc<Mutex<>> to allow safe access from V8 callbacks
//! which may execute from different threads. We use thread-local storage to pass
//! the context to callbacks.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use v8;

use crate::ops::fs;
use crate::ops::net;
use crate::runtime::RuntimeContext;

// Thread-local storage for the current runtime context
// This is set during script execution and accessed by V8 callbacks
thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<Arc<RuntimeContext>>> = RefCell::new(None);
}

/// Set the current runtime context for this thread
fn set_current_context(context: Arc<RuntimeContext>) {
    CURRENT_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(context);
    });
}

/// Get the current runtime context for this thread
fn get_current_context() -> Option<Arc<RuntimeContext>> {
    CURRENT_CONTEXT.with(|ctx| {
        ctx.borrow().clone()
    })
}

/// Clear the current runtime context for this thread
fn clear_current_context() {
    CURRENT_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = None;
    });
}

/// Extract RuntimeContext from thread-local storage
///
/// # Safety
///
/// This function accesses thread-local storage that should have been
/// set during script execution.
///
/// # Arguments
///
/// * `scope` - The V8 handle scope
///
/// # Returns
///
/// Returns `Some(&RuntimeContext)` if the context was found, `None` otherwise
unsafe fn get_context<'a>(_scope: &mut v8::HandleScope) -> Option<&'a RuntimeContext> {
    // Get the Arc from thread-local storage
    // We extend the lifetime to match the scope - this is safe because:
    // 1. The context is set before script execution
    // 2. The context lives for the duration of the script
    // 3. We only access it during the script execution
    get_current_context().and_then(|arc| {
        // Get a raw pointer to the inner RuntimeContext
        // Safety: The Arc ensures the data is alive
        let ptr = Arc::as_ptr(&arc) as *const RuntimeContext;
        Some(&*ptr)
    })
}

/// Throw a JavaScript error from a Rust callback
///
/// # Arguments
///
/// * `scope` - The V8 handle scope
/// * `message` - The error message to throw
fn throw_error(scope: &mut v8::HandleScope, message: &str) {
    let message_str = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::error(scope, message_str);
    scope.throw_exception(error);
}

/// Throw a JavaScript type error from a Rust callback
fn throw_type_error(scope: &mut v8::HandleScope, message: &str) {
    let message_str = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, message_str);
    scope.throw_exception(error);
}

/// Extract a string argument from V8
///
/// # Arguments
///
/// * `scope` - The V8 handle scope
/// * `args` - The function callback arguments
/// * `index` - The argument index to extract
///
/// # Returns
///
/// Returns `Some(String)` if the argument exists and is a string, `None` otherwise
fn extract_string_arg(
    scope: &mut v8::HandleScope,
    args: &v8::FunctionCallbackArguments,
    index: i32,
) -> Option<String> {
    if args.length() <= index {
        return None;
    }

    let arg = args.get(index);
    if arg.is_string() {
        let str_val = arg.to_rust_string_lossy(scope);
        Some(str_val)
    } else {
        None
    }
}

/// Extract a bytes argument from V8 (ArrayBuffer or Uint8Array)
///
/// # Arguments
///
/// * `scope` - The V8 handle scope
/// * `args` - The function callback arguments
/// * `index` - The argument index to extract
///
/// # Returns
///
/// Returns `Some(Vec<u8>)` if the argument exists and is valid bytes, `None` otherwise
fn extract_bytes_arg(
    scope: &mut v8::HandleScope,
    args: &v8::FunctionCallbackArguments,
    index: i32,
) -> Option<Vec<u8>> {
    if args.length() <= index {
        return None;
    }

    let arg = args.get(index);

    // Handle ArrayBuffer first (more common for binary data)
    if arg.is_array_buffer() {
        let buffer = v8::Local::<v8::ArrayBuffer>::try_from(arg).ok()?;
        let backing_store = buffer.get_backing_store();
        let bytes: Vec<u8> = backing_store.iter().map(|cell: &Cell<u8>| cell.get()).collect();
        Some(bytes)
    }
    // Handle Uint8Array
    else if arg.is_uint8_array() {
        // For Uint8Array, we need to get the underlying buffer and extract bytes
        // The buffer() method returns Option<Local<ArrayBuffer>>, but we can use
        // the DataView or directly work with the Uint8Array
        let array = v8::Local::<v8::Uint8Array>::try_from(arg).ok()?;
        let scope2 = &mut v8::HandleScope::new(scope);

        // Get the buffer - if it fails, we can't process the data
        if let Some(buffer) = array.buffer(scope2) {
            let backing_store = buffer.get_backing_store();
            let offset = array.byte_offset() as usize;
            let length = array.byte_length() as usize;
            let bytes: Vec<u8> = backing_store.iter()
                .skip(offset)
                .take(length)
                .map(|cell: &Cell<u8>| cell.get())
                .collect();
            Some(bytes)
        } else {
            None
        }
    }
    // Handle string (convert to bytes)
    else if arg.is_string() {
        let str_val = arg.to_rust_string_lossy(scope);
        Some(str_val.into_bytes())
    }
    else {
        None
    }
}

// ============================================================================
// Console API Callbacks
// ============================================================================

/// Console.log() implementation
///
/// Outputs all arguments to stdout, separated by spaces.
///
/// # JavaScript Signature
/// ```javascript
/// console.log(...args: any[]): void
/// ```
///
/// # Example
/// ```javascript
/// console.log("Hello, World!");
/// console.log("Value:", 42, "Object:", { key: "value" });
/// ```
pub fn op_console_log(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Collect all arguments as strings
    let mut output = String::new();
    for i in 0..args.length() {
        if i > 0 {
            output.push(' ');
        }

        let arg = args.get(i);
        let str_val = arg.to_rust_string_lossy(scope);
        output.push_str(&str_val);
    }

    // Output to stdout with newline
    println!("{}", output);

    // Return undefined
    rv.set_undefined();
}

/// Console.error() implementation
///
/// Outputs all arguments to stderr, separated by spaces.
///
/// # JavaScript Signature
/// ```javascript
/// console.error(...args: any[]): void
/// ```
///
/// # Example
/// ```javascript
/// console.error("Error:", error_message);
/// ```
pub fn op_console_error(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut output = String::new();
    for i in 0..args.length() {
        if i > 0 {
            output.push(' ');
        }

        let arg = args.get(i);
        let str_val = arg.to_rust_string_lossy(scope);
        output.push_str(&str_val);
    }

    eprintln!("{}", output);

    rv.set_undefined();
}

/// Console.warn() implementation
///
/// Outputs all arguments to stderr with a "Warning:" prefix.
///
/// # JavaScript Signature
/// ```javascript
/// console.warn(...args: any[]): void
/// ```
///
/// # Example
/// ```javascript
/// console.warn("Deprecated function called");
/// ```
pub fn op_console_warn(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut output = String::from("Warning:");
    for i in 0..args.length() {
        output.push(' ');

        let arg = args.get(i);
        let str_val = arg.to_rust_string_lossy(scope);
        output.push_str(&str_val);
    }

    eprintln!("{}", output);

    rv.set_undefined();
}

// ============================================================================
// Deno File System API Callbacks
// ============================================================================

/// Deno.readTextFile() implementation
///
/// Reads a file and returns its contents as a string.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.readTextFile(path: string): Promise<string>
/// ```
///
/// # Example
/// ```javascript
/// const content = await Deno.readTextFile("./hello.txt");
/// console.log(content);
/// ```
///
/// # Errors
///
/// Throws if:
/// - Path argument is missing or not a string
/// - Permission is denied
/// - File does not exist
/// - File cannot be read
pub fn op_read_text_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Extract RuntimeContext from V8
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    // Extract path argument
    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "readTextFile requires a string path argument");
            return;
        }
    };

    // Get permissions
    let permissions = ctx.permissions.lock().unwrap();

    // Execute the file read
    match fs::read_text_file(&path, &permissions) {
        Ok(content) => {
            // Convert result to V8 string
            let result_str = v8::String::new(scope, &content).unwrap();
            rv.set(result_str.into());
        }
        Err(e) => {
            throw_error(scope, &format!("readTextFile: {}", e));
        }
    }
}

/// Deno.writeTextFile() implementation
///
/// Writes a string to a file, creating parent directories if needed.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.writeTextFile(path: string, data: string): Promise<void>
/// ```
///
/// # Example
/// ```javascript
/// await Deno.writeTextFile("./output.txt", "Hello, World!");
/// ```
///
/// # Errors
///
/// Throws if:
/// - Path or data argument is missing
/// - Permission is denied
/// - File cannot be written
pub fn op_write_text_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    // Extract path argument
    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "writeTextFile requires a string path argument");
            return;
        }
    };

    // Extract data argument
    let data = match extract_string_arg(scope, &args, 1) {
        Some(d) => d,
        None => {
            throw_type_error(scope, "writeTextFile requires a string data argument");
            return;
        }
    };

    let permissions = ctx.permissions.lock().unwrap();

    match fs::write_text_file(&path, &data, &permissions) {
        Ok(_) => {
            rv.set_undefined();
        }
        Err(e) => {
            throw_error(scope, &format!("writeTextFile: {}", e));
        }
    }
}

/// Deno.readFile() implementation
///
/// Reads a file and returns its contents as a Uint8Array.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.readFile(path: string): Promise<Uint8Array>
/// ```
///
/// # Example
/// ```javascript
/// const data = await Deno.readFile("./image.png");
/// ```
///
/// # Errors
///
/// Throws if:
/// - Path argument is missing or not a string
/// - Permission is denied
/// - File does not exist or cannot be read
pub fn op_read_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "readFile requires a string path argument");
            return;
        }
    };

    let permissions = ctx.permissions.lock().unwrap();

    match fs::read_file(&path, &permissions) {
        Ok(bytes) => {
            // Create Uint8Array from bytes
            let buffer = v8::ArrayBuffer::new(scope, bytes.len());
            {
                let backing_store = buffer.get_backing_store();
                for (i, byte) in bytes.iter().enumerate() {
                    backing_store[i].set(*byte);
                }
            }
            let uint8_array = v8::Uint8Array::new(scope, buffer, 0, bytes.len()).unwrap();

            rv.set(uint8_array.into());
        }
        Err(e) => {
            throw_error(scope, &format!("readFile: {}", e));
        }
    }
}

/// Deno.writeFile() implementation
///
/// Writes bytes to a file, creating parent directories if needed.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.writeFile(path: string, data: Uint8Array): Promise<void>
/// ```
///
/// # Example
/// ```javascript
/// await Deno.writeFile("./output.bin", new Uint8Array([1, 2, 3]));
/// ```
///
/// # Errors
///
/// Throws if:
/// - Path or data argument is missing
/// - Permission is denied
/// - File cannot be written
pub fn op_write_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "writeFile requires a string path argument");
            return;
        }
    };

    let data = match extract_bytes_arg(scope, &args, 1) {
        Some(d) => d,
        None => {
            throw_type_error(scope, "writeFile requires Uint8Array or ArrayBuffer data");
            return;
        }
    };

    let permissions = ctx.permissions.lock().unwrap();

    match fs::write_file(&path, &data, &permissions) {
        Ok(_) => {
            rv.set_undefined();
        }
        Err(e) => {
            throw_error(scope, &format!("writeFile: {}", e));
        }
    }
}

/// Deno.exists() implementation
///
/// Checks if a path exists.
///
/// # JavaScript Signature
/// ```javascript
/// function Deno.exists(path: string): boolean
/// ```
///
/// # Example
/// ```javascript
/// if (Deno.exists("./config.json")) {
///     console.log("Config file found");
/// }
/// ```
pub fn op_exists(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "exists requires a string path argument");
            return;
        }
    };

    let permissions = ctx.permissions.lock().unwrap();

    match fs::exists(&path, &permissions) {
        Ok(result) => {
            let bool_val = v8::Boolean::new(scope, result);
            rv.set(bool_val.into());
        }
        Err(e) => {
            throw_error(scope, &format!("exists: {}", e));
        }
    }
}

/// Deno.stat() / Deno.metadata() implementation
///
/// Gets metadata for a file or directory.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.stat(path: string): Promise<FileInfo>
/// ```
///
/// # Returns
///
/// An object with properties: isFile, isDirectory, size, mtime, atime, birthtime, readonly
///
/// # Example
/// ```javascript
/// const info = await Deno.stat("./file.txt");
/// console.log(info.size, "bytes");
/// ```
pub fn op_metadata(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "metadata requires a string path argument");
            return;
        }
    };

    let permissions = ctx.permissions.lock().unwrap();

    match fs::metadata(&path, &permissions) {
        Ok(meta) => {
            // Create a JavaScript object with the metadata
            let obj = v8::Object::new(scope);

            // Set isFile
            let key_is_file = v8::String::new(scope, "isFile").unwrap();
            let val_is_file = v8::Boolean::new(scope, meta.is_file);
            obj.set(scope, key_is_file.into(), val_is_file.into());

            // Set isDirectory
            let key_is_dir = v8::String::new(scope, "isDirectory").unwrap();
            let val_is_dir = v8::Boolean::new(scope, meta.is_directory);
            obj.set(scope, key_is_dir.into(), val_is_dir.into());

            // Set isSymlink
            let key_is_symlink = v8::String::new(scope, "isSymlink").unwrap();
            let val_is_symlink = v8::Boolean::new(scope, meta.is_symlink);
            obj.set(scope, key_is_symlink.into(), val_is_symlink.into());

            // Set size
            let key_size = v8::String::new(scope, "size").unwrap();
            let val_size = v8::Number::new(scope, meta.size as f64);
            obj.set(scope, key_size.into(), val_size.into());

            // Set modified (mtime)
            if let Some(mtime) = meta.modified {
                let key_mtime = v8::String::new(scope, "mtime").unwrap();
                let val_mtime = v8::Number::new(scope, mtime as f64 * 1000.0); // Convert to ms
                obj.set(scope, key_mtime.into(), val_mtime.into());
            }

            // Set accessed (atime)
            if let Some(atime) = meta.accessed {
                let key_atime = v8::String::new(scope, "atime").unwrap();
                let val_atime = v8::Number::new(scope, atime as f64 * 1000.0);
                obj.set(scope, key_atime.into(), val_atime.into());
            }

            // Set created (birthtime)
            if let Some(birthtime) = meta.created {
                let key_birthtime = v8::String::new(scope, "birthtime").unwrap();
                let val_birthtime = v8::Number::new(scope, birthtime as f64 * 1000.0);
                obj.set(scope, key_birthtime.into(), val_birthtime.into());
            }

            // Set readonly
            let key_readonly = v8::String::new(scope, "readonly").unwrap();
            let val_readonly = v8::Boolean::new(scope, meta.readonly);
            obj.set(scope, key_readonly.into(), val_readonly.into());

            rv.set(obj.into());
        }
        Err(e) => {
            throw_error(scope, &format!("metadata: {}", e));
        }
    }
}

/// Deno.mkdir() implementation
///
/// Creates a directory.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.mkdir(path: string, options?: { recursive: boolean }): Promise<void>
/// ```
///
/// # Example
/// ```javascript
/// await Deno.mkdir("./dist", { recursive: true });
/// ```
pub fn op_mkdir(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "mkdir requires a string path argument");
            return;
        }
    };

    // Check for recursive option (second argument, if object with recursive property)
    let recursive = if args.length() > 1 {
        let options = args.get(1);
        if options.is_object() {
            let obj = v8::Local::<v8::Object>::try_from(options).ok();
            if let Some(obj) = obj {
                let scope2 = &mut v8::HandleScope::new(scope);
                let key = v8::String::new(scope2, "recursive").unwrap();
                let val = obj.get(scope2, key.into());

                if let Some(val) = val {
                    if val.is_boolean() {
                        val.boolean_value(scope2)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    let permissions = ctx.permissions.lock().unwrap();

    match fs::create_dir(&path, &permissions, recursive) {
        Ok(_) => {
            rv.set_undefined();
        }
        Err(e) => {
            throw_error(scope, &format!("mkdir: {}", e));
        }
    }
}

/// Deno.remove() implementation
///
/// Removes a file or directory.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.remove(path: string, options?: { recursive: boolean }): Promise<void>
/// ```
///
/// # Example
/// ```javascript
/// await Deno.remove("./old-file.txt");
/// await Deno.remove("./dist", { recursive: true });
/// ```
pub fn op_remove(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "remove requires a string path argument");
            return;
        }
    };

    let recursive = if args.length() > 1 {
        let options = args.get(1);
        if options.is_object() {
            let obj = v8::Local::<v8::Object>::try_from(options).ok();
            if let Some(obj) = obj {
                let scope2 = &mut v8::HandleScope::new(scope);
                let key = v8::String::new(scope2, "recursive").unwrap();
                let val = obj.get(scope2, key.into());

                if let Some(val) = val {
                    if val.is_boolean() {
                        val.boolean_value(scope2)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    let permissions = ctx.permissions.lock().unwrap();

    match fs::remove(&path, &permissions, recursive) {
        Ok(_) => {
            rv.set_undefined();
        }
        Err(e) => {
            throw_error(scope, &format!("remove: {}", e));
        }
    }
}

// ============================================================================
// Deno Fetch API Callbacks
// ============================================================================

/// Deno.fetch() implementation
///
/// Fetches a URL using HTTP and returns the response.
///
/// # JavaScript Signature
/// ```javascript
/// async function Deno.fetch(url: string, options?: FetchOptions): Promise<FetchResponse>
/// ```
///
/// # Options
/// - method: HTTP method string (GET, POST, etc.)
/// - headers: Object with string key-value pairs
/// - body: String or Uint8Array for request body
/// - timeout: Timeout in milliseconds
///
/// # Returns
///
/// An object with properties:
/// - ok: boolean (true for 2xx status codes)
/// - status: number (HTTP status code)
/// - statusText: string (HTTP status text)
/// - headers: object (response headers)
/// - url: string (final URL after redirects)
/// - text(): function - returns body as text
/// - json(): function - returns body as JSON
///
/// # Example
/// ```javascript
/// const response = await Deno.fetch("https://example.com");
/// console.log(response.status, response.statusText);
/// const text = await response.text();
/// ```
///
/// # Errors
///
/// Throws if:
/// - URL argument is missing or not a string
/// - Permission is denied
/// - Network request fails
/// - Response cannot be read
pub fn op_fetch(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx = match unsafe { get_context(scope) } {
        Some(ctx) => ctx,
        None => {
            throw_error(scope, "Runtime context not found");
            return;
        }
    };

    // Extract URL argument
    let url = match extract_string_arg(scope, &args, 0) {
        Some(u) => u,
        None => {
            throw_type_error(scope, "fetch requires a string URL argument");
            return;
        }
    };

    // Parse options from second argument if provided
    let mut options = net::FetchOptions::default();

    if args.length() > 1 {
        let options_arg = args.get(1);
        if options_arg.is_object() {
            let obj = match v8::Local::<v8::Object>::try_from(options_arg) {
                Ok(o) => o,
                Err(_) => {
                    throw_type_error(scope, "fetch options must be an object");
                    return;
                }
            };

            let scope2 = &mut v8::HandleScope::new(scope);

            // Extract method
            let key_method = v8::String::new(scope2, "method").unwrap();
            if let Some(val) = obj.get(scope2, key_method.into()) {
                if val.is_string() {
                    let method_str = val.to_rust_string_lossy(scope2);
                    if let Some(method) = net::HttpMethod::from_str(&method_str) {
                        options.method = Some(method);
                    }
                }
            }

            // Extract headers
            let key_headers = v8::String::new(scope2, "headers").unwrap();
            if let Some(val) = obj.get(scope2, key_headers.into()) {
                if val.is_object() {
                    if let Ok(headers_obj) = v8::Local::<v8::Object>::try_from(val) {
                        let mut headers_map = std::collections::HashMap::new();
                        // Get all property names (keys)
                        // Use default GetPropertyNamesArgs for simplicity
                        let key_names = headers_obj.get_own_property_names(
                            scope2,
                            v8::GetPropertyNamesArgs::default(),
                        );
                        if let Some(keys) = key_names {
                            let keys_array = match v8::Local::<v8::Array>::try_from(keys) {
                                Ok(arr) => arr,
                                Err(_) => {
                                    throw_type_error(scope2, "failed to extract headers keys");
                                    return;
                                }
                            };

                            for i in 0..keys_array.length() {
                                let key_val = keys_array.get_index(scope2, i);
                                if let Some(key) = key_val {
                                    let key_str = key.to_rust_string_lossy(scope2);
                                    let value = headers_obj.get(scope2, key);
                                    if let Some(value_val) = value {
                                        let value_str = value_val.to_rust_string_lossy(scope2);
                                        headers_map.insert(key_str, value_str);
                                    }
                                }
                            }
                        }
                        if !headers_map.is_empty() {
                            options.headers = Some(headers_map);
                        }
                    }
                }
            }

            // Extract body (string or bytes)
            let key_body = v8::String::new(scope2, "body").unwrap();
            if let Some(_val) = obj.get(scope2, key_body.into()) {
                // Try to extract as bytes first
                if let Some(_bytes) = extract_bytes_arg(scope2, &args, 1) {
                    // We need to get the body property specifically
                    let scope3 = &mut v8::HandleScope::new(scope2);
                    let body_key = v8::String::new(scope3, "body").unwrap();
                    if let Some(body_val) = obj.get(scope3, body_key.into()) {
                        if body_val.is_string() {
                            options.body = Some(body_val.to_rust_string_lossy(scope3).into_bytes());
                        } else if body_val.is_uint8_array() || body_val.is_array_buffer() {
                            // Re-extract using our helper with the correct index
                            // This is a limitation - we need to handle it differently
                            let scope4 = &mut v8::HandleScope::new(scope3);
                            if let Some(body_bytes) = extract_bytes_arg_from_value(scope4, body_val) {
                                options.body = Some(body_bytes);
                            }
                        }
                    }
                }
            }

            // Extract timeout
            let key_timeout = v8::String::new(scope2, "timeout").unwrap();
            if let Some(val) = obj.get(scope2, key_timeout.into()) {
                if val.is_number() {
                    let timeout_val = val.number_value(scope2).unwrap_or(0.0);
                    if timeout_val > 0.0 {
                        options.timeout = Some(timeout_val as u64);
                    }
                }
            }

            // Extract redirect option
            let key_redirect = v8::String::new(scope2, "redirect").unwrap();
            if let Some(val) = obj.get(scope2, key_redirect.into()) {
                if val.is_boolean() {
                    options.redirect = Some(val.boolean_value(scope2));
                }
            }

            // Extract max_redirects option
            let key_max_redirects = v8::String::new(scope2, "maxRedirects").unwrap();
            if let Some(val) = obj.get(scope2, key_max_redirects.into()) {
                if val.is_number() {
                    let max_val = val.number_value(scope2).unwrap_or(0.0);
                    if max_val > 0.0 {
                        options.max_redirects = Some(max_val as usize);
                    }
                }
            }
        }
    }

    let permissions = ctx.permissions.lock().unwrap();

    match net::fetch(&url, Some(options), &permissions) {
        Ok(response) => {
            // Create a response object with all properties and methods
            let response_obj = v8::Object::new(scope);

            // Set ok
            let key_ok = v8::String::new(scope, "ok").unwrap();
            let val_ok = v8::Boolean::new(scope, response.ok());
            response_obj.set(scope, key_ok.into(), val_ok.into());

            // Set status
            let key_status = v8::String::new(scope, "status").unwrap();
            let val_status = v8::Number::new(scope, response.status as f64);
            response_obj.set(scope, key_status.into(), val_status.into());

            // Set statusText
            let key_status_text = v8::String::new(scope, "statusText").unwrap();
            let val_status_text = v8::String::new(scope, &response.status_text).unwrap();
            response_obj.set(scope, key_status_text.into(), val_status_text.into());

            // Set url
            let key_url = v8::String::new(scope, "url").unwrap();
            let val_url = v8::String::new(scope, &response.url).unwrap();
            response_obj.set(scope, key_url.into(), val_url.into());

            // Set headers object
            let headers_obj = v8::Object::new(scope);
            for (name, value) in &response.headers {
                let key = v8::String::new(scope, name).unwrap();
                let val = v8::String::new(scope, value).unwrap();
                headers_obj.set(scope, key.into(), val.into());
            }
            let key_headers = v8::String::new(scope, "headers").unwrap();
            response_obj.set(scope, key_headers.into(), headers_obj.into());

            // For simplicity, pre-compute and store the body text on the response object
            // This avoids the complexity of using symbols or closures with captured data
            let body_text = String::from_utf8_lossy(&response.body).to_string();
            let body_text_v8 = v8::String::new(scope, &body_text).unwrap();

            // Store the body text as an internal property (using a special key name)
            let scope2 = &mut v8::HandleScope::new(scope);
            let body_key = v8::String::new(scope2, "__ferrum_body__").unwrap();
            response_obj.set(scope2, body_key.into(), body_text_v8.into());

            // Create text() method that returns the pre-computed body text
            let key_text = v8::String::new(scope2, "text").unwrap();
            let func_text = v8::Function::new(scope2, |scope3: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
                let this = args.this();
                let body_key = v8::String::new(scope3, "__ferrum_body__").unwrap();
                if let Some(body_val) = this.get(scope3, body_key.into()) {
                    rv.set(body_val);
                    return;
                }
                throw_error(scope3, "Response body not available");
            }).unwrap();
            response_obj.set(scope2, key_text.into(), func_text.into());

            // Create json() method that parses the body text as JSON
            let scope3 = &mut v8::HandleScope::new(scope2);
            let key_json = v8::String::new(scope3, "json").unwrap();
            let func_json = v8::Function::new(scope3, |scope4: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
                let this = args.this();
                let body_key = v8::String::new(scope4, "__ferrum_body__").unwrap();
                if let Some(body_val) = this.get(scope4, body_key.into()) {
                    if body_val.is_string() {
                        let body_str = v8::Local::<v8::String>::try_from(body_val).unwrap();
                        let body_text = body_str.to_rust_string_lossy(scope4);

                        match serde_json::from_slice::<serde_json::Value>(body_text.as_bytes()) {
                            Ok(_json) => {
                                // Try to parse it using JSON.parse for proper JavaScript objects
                                let context = scope4.get_current_context();
                                let global = context.global(scope4);
                                let json_key = v8::String::new(scope4, "JSON").unwrap();
                                if let Some(json_val) = global.get(scope4, json_key.into()) {
                                    if let Ok(json_obj) = v8::Local::<v8::Object>::try_from(json_val) {
                                        let parse_key = v8::String::new(scope4, "parse").unwrap();
                                        if let Some(parse_val) = json_obj.get(scope4, parse_key.into()) {
                                            if let Ok(parse_func) = v8::Local::<v8::Function>::try_from(parse_val) {
                                                if let Some(v8_str) = v8::String::new(scope4, &body_text) {
                                                    let args = [v8_str.into()];
                                                    if let Some(result) = parse_func.call(scope4, json_obj.into(), &args) {
                                                        rv.set(result);
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // Fallback: return the JSON string
                                rv.set(body_val);
                                return;
                            }
                            Err(e) => {
                                throw_error(scope4, &format!("Invalid JSON: {}", e));
                                return;
                            }
                        }
                    }
                }
                throw_error(scope4, "Response body not available");
            }).unwrap();
            response_obj.set(scope3, key_json.into(), func_json.into());

            rv.set(response_obj.into());
        }
        Err(e) => {
            throw_error(scope, &format!("fetch: {}", e));
        }
    }
}

/// Helper function to extract bytes from a V8 value
/// This is similar to extract_bytes_arg but works with a specific value
/// rather than pulling from arguments
fn extract_bytes_arg_from_value(
    scope: &mut v8::HandleScope,
    value: v8::Local<v8::Value>,
) -> Option<Vec<u8>> {
    // Handle ArrayBuffer
    if value.is_array_buffer() {
        let buffer = v8::Local::<v8::ArrayBuffer>::try_from(value).ok()?;
        let backing_store = buffer.get_backing_store();
        let bytes: Vec<u8> = backing_store.iter().map(|cell| cell.get()).collect();
        Some(bytes)
    }
    // Handle Uint8Array
    else if value.is_uint8_array() {
        let array = v8::Local::<v8::Uint8Array>::try_from(value).ok()?;
        let scope2 = &mut v8::HandleScope::new(scope);
        if let Some(buffer) = array.buffer(scope2) {
            let backing_store = buffer.get_backing_store();
            let offset = array.byte_offset() as usize;
            let length = array.byte_length() as usize;
            let bytes: Vec<u8> = backing_store.iter()
                .skip(offset)
                .take(length)
                .map(|cell| cell.get())
                .collect();
            Some(bytes)
        } else {
            None
        }
    }
    // Handle string
    else if value.is_string() {
        let str_val = value.to_rust_string_lossy(scope);
        Some(str_val.into_bytes())
    }
    else {
        None
    }
}

// ============================================================================
// Global Object Bootstrap
// ============================================================================

/// Bootstrap the global JavaScript APIs
///
/// This function creates the global objects that JavaScript code can access:
/// - `console` object with log, error, warn methods
/// - `Deno` object with file system methods
///
/// # Arguments
///
/// * `scope` - The V8 handle scope (must be a ContextScope)
/// * `context` - The runtime context to store for callbacks
///
/// # Safety
///
/// This function stores the RuntimeContext in thread-local storage.
/// The context must live as long as the V8 isolate.
///
/// # Memory Safety
///
/// We store the Arc<RuntimeContext> in thread-local storage.
/// This prevents the context from being dropped while V8 is using it.
/// The caller is responsible for clearing the context after execution.
pub fn bootstrap_globals(
    scope: &mut v8::HandleScope,
    context: Arc<RuntimeContext>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Bootstrapping global JavaScript APIs");

    // Store the context in thread-local storage
    set_current_context(context.clone());

    let context_scope = scope.get_current_context();
    let global = context_scope.global(scope);

    tracing::trace!("Stored RuntimeContext in thread-local storage");

    // Create console object
    let console = v8::Object::new(scope);

    // Register console methods
    {
        let scope2 = &mut v8::HandleScope::new(scope);

        let name_log = v8::String::new(scope2, "log").unwrap();
        let func_log = v8::Function::new(scope2, op_console_log).unwrap();
        console.set(scope2, name_log.into(), func_log.into());

        let name_error = v8::String::new(scope2, "error").unwrap();
        let func_error = v8::Function::new(scope2, op_console_error).unwrap();
        console.set(scope2, name_error.into(), func_error.into());

        let name_warn = v8::String::new(scope2, "warn").unwrap();
        let func_warn = v8::Function::new(scope2, op_console_warn).unwrap();
        console.set(scope2, name_warn.into(), func_warn.into());
    }

    // Set console on global object
    {
        let scope2 = &mut v8::HandleScope::new(scope);
        let key = v8::String::new(scope2, "console").unwrap();
        global.set(scope2, key.into(), console.into());
    }

    tracing::debug!("Registered console object");

    // Create Deno object
    let deno = v8::Object::new(scope);

    // Register Deno methods
    {
        let scope2 = &mut v8::HandleScope::new(scope);

        // readTextFile
        let name = v8::String::new(scope2, "readTextFile").unwrap();
        let func = v8::Function::new(scope2, op_read_text_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        // writeTextFile
        let name = v8::String::new(scope2, "writeTextFile").unwrap();
        let func = v8::Function::new(scope2, op_write_text_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        // readFile
        let name = v8::String::new(scope2, "readFile").unwrap();
        let func = v8::Function::new(scope2, op_read_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        // writeFile
        let name = v8::String::new(scope2, "writeFile").unwrap();
        let func = v8::Function::new(scope2, op_write_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        // exists
        let name = v8::String::new(scope2, "exists").unwrap();
        let func = v8::Function::new(scope2, op_exists).unwrap();
        deno.set(scope2, name.into(), func.into());

        // stat (metadata)
        let name = v8::String::new(scope2, "stat").unwrap();
        let func = v8::Function::new(scope2, op_metadata).unwrap();
        deno.set(scope2, name.into(), func.into());

        // mkdir
        let name = v8::String::new(scope2, "mkdir").unwrap();
        let func = v8::Function::new(scope2, op_mkdir).unwrap();
        deno.set(scope2, name.into(), func.into());

        // remove
        let name = v8::String::new(scope2, "remove").unwrap();
        let func = v8::Function::new(scope2, op_remove).unwrap();
        deno.set(scope2, name.into(), func.into());

        // fetch
        let name = v8::String::new(scope2, "fetch").unwrap();
        let func = v8::Function::new(scope2, op_fetch).unwrap();
        deno.set(scope2, name.into(), func.into());
    }

    // Set Deno on global object
    {
        let scope2 = &mut v8::HandleScope::new(scope);
        let key = v8::String::new(scope2, "Deno").unwrap();
        global.set(scope2, key.into(), deno.into());
    }

    tracing::debug!("Registered Deno object");
    tracing::info!("Global JavaScript APIs bootstrapped successfully");

    Ok(())
}

/// Clear the current runtime context from thread-local storage
///
/// This should be called after script execution to clean up
pub fn clear_globals() {
    clear_current_context();
}

#[cfg(test)]
mod tests {
    use crate::permissions::Permissions;
    use crate::runtime::{init_v8_platform, JsRuntime, RuntimeConfig};

    /// Helper to create a test runtime with bootstrapped globals
    fn create_test_runtime() -> JsRuntime {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            init_v8_platform();
        });

        let config = RuntimeConfig::default();
        let permissions = Permissions::allow_all();
        JsRuntime::new(config, permissions).unwrap()
    }

    #[test]
    fn test_bootstrap_globals() {
        let rt = create_test_runtime();

        // We can't directly test the V8 bridge without a full V8 context
        // but we can verify the runtime was created
        assert!(!rt.id().is_empty());
    }
}
