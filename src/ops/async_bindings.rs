//! Async V8-Rust Bridge Bindings
//!
//! This module provides async V8 function callbacks that return proper Promises.
//! These bindings work with the EventLoop to provide true async/await support.

use std::cell::RefCell;
use std::sync::Arc;

use v8;

use crate::ops::async_ops::{create_promise, OpFuture, OpId, OpResult, OpState, PromiseRegistry};
use crate::ops::fs;
use crate::ops::op_import_module;
use crate::permissions::Permissions;

thread_local! {
    static ASYNC_OP_STATE: RefCell<Option<Arc<std::sync::Mutex<OpState>>>> = RefCell::new(None);
    static ASYNC_PROMISE_REGISTRY: RefCell<Option<Arc<std::sync::Mutex<PromiseRegistry>>>> = RefCell::new(None);
    static ASYNC_PERMISSIONS: RefCell<Option<Arc<std::sync::Mutex<Permissions>>>> = RefCell::new(None);
}

/// Set the async context for thread-local storage
pub fn set_async_context(
    op_state: Arc<std::sync::Mutex<OpState>>,
    promise_registry: Arc<std::sync::Mutex<PromiseRegistry>>,
    permissions: Arc<std::sync::Mutex<Permissions>>,
) {
    ASYNC_OP_STATE.with(|state| {
        *state.borrow_mut() = Some(op_state);
    });
    ASYNC_PROMISE_REGISTRY.with(|registry| {
        *registry.borrow_mut() = Some(promise_registry);
    });
    ASYNC_PERMISSIONS.with(|perms| {
        *perms.borrow_mut() = Some(permissions);
    });
}

/// Clear the async context from thread-local storage
pub fn clear_async_context() {
    ASYNC_OP_STATE.with(|state| {
        *state.borrow_mut() = None;
    });
    ASYNC_PROMISE_REGISTRY.with(|registry| {
        *registry.borrow_mut() = None;
    });
    ASYNC_PERMISSIONS.with(|perms| {
        *perms.borrow_mut() = None;
    });
}

fn get_async_state() -> Option<(
    Arc<std::sync::Mutex<OpState>>,
    Arc<std::sync::Mutex<PromiseRegistry>>,
    Arc<std::sync::Mutex<Permissions>>,
)> {
    let op_state = ASYNC_OP_STATE.with(|state| state.borrow().clone())?;
    let promise_registry = ASYNC_PROMISE_REGISTRY.with(|registry| registry.borrow().clone())?;
    let permissions = ASYNC_PERMISSIONS.with(|perms| perms.borrow().clone())?;
    Some((op_state, promise_registry, permissions))
}

fn throw_error(scope: &mut v8::HandleScope, message: &str) {
    let message_str = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::error(scope, message_str);
    scope.throw_exception(error);
}

fn throw_type_error(scope: &mut v8::HandleScope, message: &str) {
    let message_str = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, message_str);
    scope.throw_exception(error);
}

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
        Some(arg.to_rust_string_lossy(scope))
    } else {
        None
    }
}

fn schedule_async_op(
    op_state: &Arc<std::sync::Mutex<OpState>>,
    promise_registry: &Arc<std::sync::Mutex<PromiseRegistry>>,
    resolver: v8::Global<v8::PromiseResolver>,
    future: OpFuture,
) -> OpId {
    let mut state = op_state.lock().unwrap();
    let id = state.next_id();
    let op = crate::ops::async_ops::PendingOp::new(id, future);
    state.add_pending(op);

    let mut registry = promise_registry.lock().unwrap();
    registry.register(id, resolver);

    id
}

pub fn op_async_read_text_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "readTextFile requires a string path argument");
            return;
        }
    };

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_read(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::read_text_file(&path, &permissions) {
            Ok(content) => OpResult::Ok(content),
            Err(e) => OpResult::Err(format!("readTextFile: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_write_text_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "writeTextFile requires a string path argument");
            return;
        }
    };

    let data = match extract_string_arg(scope, &args, 1) {
        Some(d) => d,
        None => {
            throw_type_error(scope, "writeTextFile requires a string data argument");
            return;
        }
    };

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_write(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::write_text_file(&path, &data, &permissions) {
            Ok(_) => OpResult::Ok("undefined".to_string()),
            Err(e) => OpResult::Err(format!("writeTextFile: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_read_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
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

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_read(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::read_file(&path, &permissions) {
            Ok(bytes) => OpResult::OkBytes(bytes),
            Err(e) => OpResult::Err(format!("readFile: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_write_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
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

    let data = extract_bytes_arg_v2(scope, &args, 1);
    if data.is_none() {
        throw_type_error(scope, "writeFile requires Uint8Array or ArrayBuffer data");
        return;
    }
    let data = data.unwrap();

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_write(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::write_file(&path, &data, &permissions) {
            Ok(_) => OpResult::Ok("undefined".to_string()),
            Err(e) => OpResult::Err(format!("writeFile: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

fn extract_bytes_arg_v2(
    scope: &mut v8::HandleScope,
    args: &v8::FunctionCallbackArguments,
    index: i32,
) -> Option<Vec<u8>> {
    use std::cell::Cell;

    if args.length() <= index {
        return None;
    }

    let arg = args.get(index);

    if arg.is_array_buffer() {
        let buffer = v8::Local::<v8::ArrayBuffer>::try_from(arg).ok()?;
        let backing_store = buffer.get_backing_store();
        let bytes: Vec<u8> = backing_store
            .iter()
            .map(|cell: &Cell<u8>| cell.get())
            .collect();
        Some(bytes)
    } else if arg.is_uint8_array() {
        let array = v8::Local::<v8::Uint8Array>::try_from(arg).ok()?;
        let scope2 = &mut v8::HandleScope::new(scope);
        if let Some(buffer) = array.buffer(scope2) {
            let backing_store = buffer.get_backing_store();
            let offset = array.byte_offset() as usize;
            let length = array.byte_length() as usize;
            let bytes: Vec<u8> = backing_store
                .iter()
                .skip(offset)
                .take(length)
                .map(|cell: &Cell<u8>| cell.get())
                .collect();
            Some(bytes)
        } else {
            None
        }
    } else if arg.is_string() {
        let str_val = arg.to_rust_string_lossy(scope);
        Some(str_val.into_bytes())
    } else {
        None
    }
}

pub fn op_async_exists(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
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

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_read(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::exists(&path, &permissions) {
            Ok(result) => OpResult::OkBool(result),
            Err(e) => OpResult::Err(format!("exists: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_stat(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "stat requires a string path argument");
            return;
        }
    };

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_read(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::metadata(&path, &permissions) {
            Ok(meta) => {
                let json = serde_json::json!({
                    "isFile": meta.is_file,
                    "isDirectory": meta.is_directory,
                    "isSymlink": meta.is_symlink,
                    "size": meta.size,
                    "mtime": meta.modified.map(|t| t * 1000),
                    "atime": meta.accessed.map(|t| t * 1000),
                    "birthtime": meta.created.map(|t| t * 1000),
                    "readonly": meta.readonly,
                });
                OpResult::OkJson(json.to_string())
            }
            Err(e) => OpResult::Err(format!("stat: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_mkdir(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
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

    let recursive = if args.length() > 1 {
        let options = args.get(1);
        if options.is_object() {
            if let Ok(obj) = v8::Local::<v8::Object>::try_from(options) {
                let scope2 = &mut v8::HandleScope::new(scope);
                let key = v8::String::new(scope2, "recursive").unwrap();
                if let Some(val) = obj.get(scope2, key.into()) {
                    val.is_true()
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

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_write(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::create_dir(&path, &permissions, recursive) {
            Ok(_) => OpResult::Ok("undefined".to_string()),
            Err(e) => OpResult::Err(format!("mkdir: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_remove(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
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
            if let Ok(obj) = v8::Local::<v8::Object>::try_from(options) {
                let scope2 = &mut v8::HandleScope::new(scope);
                let key = v8::String::new(scope2, "recursive").unwrap();
                if let Some(val) = obj.get(scope2, key.into()) {
                    val.is_true()
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

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_write(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::remove(&path, &permissions, recursive) {
            Ok(_) => OpResult::Ok("undefined".to_string()),
            Err(e) => OpResult::Err(format!("remove: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_copy(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
            return;
        }
    };

    let src = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "copy requires a string source path argument");
            return;
        }
    };

    let dest = match extract_string_arg(scope, &args, 1) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "copy requires a string destination path argument");
            return;
        }
    };

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_write(&dest) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        if let Err(e) = permissions.check_read(&src) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::copy(&src, &dest, &permissions) {
            Ok(bytes) => OpResult::OkNumber(bytes as f64),
            Err(e) => OpResult::Err(format!("copy: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_read_dir(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
            return;
        }
    };

    let path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "readDir requires a string path argument");
            return;
        }
    };

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_read(&path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::read_dir(&path, &permissions) {
            Ok(entries) => {
                let json = serde_json::json!(entries);
                OpResult::OkJson(json.to_string())
            }
            Err(e) => OpResult::Err(format!("readDir: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_rename(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
            return;
        }
    };

    let old_path = match extract_string_arg(scope, &args, 0) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "rename requires a string oldPath argument");
            return;
        }
    };

    let new_path = match extract_string_arg(scope, &args, 1) {
        Some(p) => p,
        None => {
            throw_type_error(scope, "rename requires a string newPath argument");
            return;
        }
    };

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let permissions = permissions_arc.lock().unwrap().clone();
    let global_resolver = v8::Global::new(scope, resolver);

    let future: OpFuture = Box::pin(async move {
        if let Err(e) = permissions.check_write(&old_path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        if let Err(e) = permissions.check_write(&new_path) {
            return OpResult::Err(format!("Permission denied: {}", e));
        }
        match fs::rename(&old_path, &new_path, &permissions) {
            Ok(_) => OpResult::Ok("undefined".to_string()),
            Err(e) => OpResult::Err(format!("rename: {}", e)),
        }
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn op_async_sleep(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (op_state, promise_registry, _permissions_arc) = match get_async_state() {
        Some(state) => state,
        None => {
            throw_error(scope, "Async runtime context not initialized");
            return;
        }
    };

    let ms = if args.length() > 0 {
        let arg = args.get(0);
        if arg.is_number() {
            arg.number_value(scope).unwrap_or(0.0) as u64
        } else {
            0
        }
    } else {
        0
    };

    let (promise, resolver) = match create_promise(scope) {
        Some(p) => p,
        None => {
            throw_error(scope, "Failed to create Promise");
            return;
        }
    };

    let global_resolver = v8::Global::new(scope, resolver);

    let ms_clone = ms;
    let future: OpFuture = Box::pin(async move {
        std::thread::sleep(std::time::Duration::from_millis(ms_clone));
        OpResult::Ok("undefined".to_string())
    });

    schedule_async_op(&op_state, &promise_registry, global_resolver, future);
    rv.set(promise.into());
}

pub fn bootstrap_async_globals(
    scope: &mut v8::HandleScope,
    op_state: Arc<std::sync::Mutex<OpState>>,
    promise_registry: Arc<std::sync::Mutex<PromiseRegistry>>,
    permissions: Arc<std::sync::Mutex<Permissions>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Bootstrapping async global JavaScript APIs");

    set_async_context(op_state, promise_registry, permissions);

    let context = scope.get_current_context();
    let global = context.global(scope);

    let console = v8::Object::new(scope);
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

    {
        let scope2 = &mut v8::HandleScope::new(scope);
        let key = v8::String::new(scope2, "console").unwrap();
        global.set(scope2, key.into(), console.into());
    }

    let deno = v8::Object::new(scope);
    {
        let scope2 = &mut v8::HandleScope::new(scope);

        let name = v8::String::new(scope2, "readTextFile").unwrap();
        let func = v8::Function::new(scope2, op_async_read_text_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "writeTextFile").unwrap();
        let func = v8::Function::new(scope2, op_async_write_text_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "readFile").unwrap();
        let func = v8::Function::new(scope2, op_async_read_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "writeFile").unwrap();
        let func = v8::Function::new(scope2, op_async_write_file).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "exists").unwrap();
        let func = v8::Function::new(scope2, op_async_exists).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "stat").unwrap();
        let func = v8::Function::new(scope2, op_async_stat).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "mkdir").unwrap();
        let func = v8::Function::new(scope2, op_async_mkdir).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "remove").unwrap();
        let func = v8::Function::new(scope2, op_async_remove).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "copy").unwrap();
        let func = v8::Function::new(scope2, op_async_copy).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "readDir").unwrap();
        let func = v8::Function::new(scope2, op_async_read_dir).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "rename").unwrap();
        let func = v8::Function::new(scope2, op_async_rename).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "sleep").unwrap();
        let func = v8::Function::new(scope2, op_async_sleep).unwrap();
        deno.set(scope2, name.into(), func.into());

        let name = v8::String::new(scope2, "importModule").unwrap();
        let func = v8::Function::new(scope2, op_import_module).unwrap();
        deno.set(scope2, name.into(), func.into());
    }

    {
        let scope2 = &mut v8::HandleScope::new(scope);
        let key = v8::String::new(scope2, "Deno").unwrap();
        global.set(scope2, key.into(), deno.into());
    }

    tracing::debug!("Registered async Deno object");
    tracing::info!("Async global JavaScript APIs bootstrapped successfully");

    Ok(())
}

pub fn op_console_log(
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
    println!("{}", output);
    rv.set_undefined();
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_context_set_clear() {
        let op_state = Arc::new(std::sync::Mutex::new(OpState::new()));
        let promise_registry = Arc::new(std::sync::Mutex::new(PromiseRegistry::new()));
        let permissions = Arc::new(std::sync::Mutex::new(Permissions::allow_all()));

        set_async_context(
            op_state.clone(),
            promise_registry.clone(),
            permissions.clone(),
        );

        assert!(get_async_state().is_some());

        clear_async_context();

        assert!(get_async_state().is_none());
    }
}
