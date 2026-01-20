//! URL/URLSearchParams V8 Bindings
//!
//! This module provides V8 function callbacks that expose URL and URLSearchParams
//! to JavaScript as global objects.

use v8;

use crate::ops::url::{URLSearchParams, Url};

fn op_url_new(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let url_input = if args.length() > 0 {
        let arg = args.get(0);
        if arg.is_string() {
            Some(arg.to_rust_string_lossy(scope))
        } else {
            None
        }
    } else {
        None
    };

    let base_input = if args.length() > 1 {
        let arg = args.get(1);
        if arg.is_string() {
            Some(arg.to_rust_string_lossy(scope))
        } else {
            None
        }
    } else {
        None
    };

    let url = if let Some(url_str) = url_input {
        if let Some(base_str) = base_input {
            Url::with_base(&url_str, &base_str)
        } else {
            Url::new(&url_str)
        }
    } else {
        return;
    };

    match url {
        Ok(url) => {
            let obj = v8::Object::new(scope);

            let href_key = v8::String::new(scope, "href").unwrap();
            let href_val = v8::String::new(scope, &url.href()).unwrap();
            obj.set(scope, href_key.into(), href_val.into());

            let protocol_key = v8::String::new(scope, "protocol").unwrap();
            let protocol_val = v8::String::new(scope, &format!("{}:", url.scheme())).unwrap();
            obj.set(scope, protocol_key.into(), protocol_val.into());

            let host_key = v8::String::new(scope, "host").unwrap();
            let mut host_val = url.host();
            if let Some(port) = url.port() {
                host_val.push(':');
                host_val.push_str(&port.to_string());
            }
            let host_v8 = v8::String::new(scope, &host_val).unwrap();
            obj.set(scope, host_key.into(), host_v8.into());

            let hostname_key = v8::String::new(scope, "hostname").unwrap();
            let hostname_val = v8::String::new(scope, &url.hostname()).unwrap();
            obj.set(scope, hostname_key.into(), hostname_val.into());

            let port_key = v8::String::new(scope, "port").unwrap();
            let port_val = url.port().map(|p| p.to_string()).unwrap_or_default();
            let port_v8 = v8::String::new(scope, &port_val).unwrap();
            obj.set(scope, port_key.into(), port_v8.into());

            let pathname_key = v8::String::new(scope, "pathname").unwrap();
            let pathname_val = v8::String::new(scope, &url.path()).unwrap();
            obj.set(scope, pathname_key.into(), pathname_val.into());

            let search_key = v8::String::new(scope, "search").unwrap();
            let search_val = v8::String::new(scope, &url.search()).unwrap();
            obj.set(scope, search_key.into(), search_val.into());

            let hash_key = v8::String::new(scope, "hash").unwrap();
            let hash_val = v8::String::new(scope, &url.hash()).unwrap();
            obj.set(scope, hash_key.into(), hash_val.into());

            let origin_key = v8::String::new(scope, "origin").unwrap();
            let origin_val = v8::String::new(scope, &url.origin()).unwrap();
            obj.set(scope, origin_key.into(), origin_val.into());

            let username_key = v8::String::new(scope, "username").unwrap();
            let username_val = v8::String::new(scope, &url.username()).unwrap();
            obj.set(scope, username_key.into(), username_val.into());

            let password_key = v8::String::new(scope, "password").unwrap();
            let password_val = v8::String::new(scope, &url.password()).unwrap();
            obj.set(scope, password_key.into(), password_val.into());

            let search_params_key = v8::String::new(scope, "searchParams").unwrap();
            let search_params = url.search_params();
            let search_params_obj = create_search_params_object(scope, search_params);
            obj.set(scope, search_params_key.into(), search_params_obj.into());

            let to_string_key = v8::String::new(scope, "toString").unwrap();
            let to_string_func = v8::Function::new(scope, op_url_to_string).unwrap();
            obj.set(scope, to_string_key.into(), to_string_func.into());

            rv.set(obj.into());
        }
        Err(e) => {
            let message_str = v8::String::new(scope, &format!("Invalid URL: {}", e)).unwrap();
            let error = v8::Exception::type_error(scope, message_str);
            scope.throw_exception(error);
        }
    }
}

fn op_url_to_string(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();

    let href_key = v8::String::new(scope, "href").unwrap();
    if let Some(href_val) = this.get(scope, href_key.into()) {
        if href_val.is_string() {
            rv.set(href_val);
            return;
        }
    }

    rv.set_undefined();
}

fn create_search_params_object<'a>(
    scope: &mut v8::HandleScope<'a>,
    params: URLSearchParams,
) -> v8::Local<'a, v8::Object> {
    let obj = v8::Object::new(scope);

    let to_string_key = v8::String::new(scope, "toString").unwrap();
    let to_string_func = v8::Function::new(scope, op_search_params_to_string).unwrap();
    obj.set(scope, to_string_key.into(), to_string_func.into());

    let get_key = v8::String::new(scope, "get").unwrap();
    let get_func = v8::Function::new(scope, op_search_params_get).unwrap();
    obj.set(scope, get_key.into(), get_func.into());

    let get_all_key = v8::String::new(scope, "getAll").unwrap();
    let get_all_func = v8::Function::new(scope, op_search_params_get_all).unwrap();
    obj.set(scope, get_all_key.into(), get_all_func.into());

    let has_key = v8::String::new(scope, "has").unwrap();
    let has_func = v8::Function::new(scope, op_search_params_has).unwrap();
    obj.set(scope, has_key.into(), has_func.into());

    let set_key = v8::String::new(scope, "set").unwrap();
    let set_func = v8::Function::new(scope, op_search_params_set).unwrap();
    obj.set(scope, set_key.into(), set_func.into());

    let append_key = v8::String::new(scope, "append").unwrap();
    let append_func = v8::Function::new(scope, op_search_params_append).unwrap();
    obj.set(scope, append_key.into(), append_func.into());

    let delete_key = v8::String::new(scope, "delete").unwrap();
    let delete_func = v8::Function::new(scope, op_search_params_delete).unwrap();
    obj.set(scope, delete_key.into(), delete_func.into());

    let keys_key = v8::String::new(scope, "keys").unwrap();
    let keys_func = v8::Function::new(scope, op_search_params_keys).unwrap();
    obj.set(scope, keys_key.into(), keys_func.into());

    let values_key = v8::String::new(scope, "values").unwrap();
    let values_func = v8::Function::new(scope, op_search_params_values).unwrap();
    obj.set(scope, values_key.into(), values_func.into());

    let entries_key = v8::String::new(scope, "entries").unwrap();
    let entries_func = v8::Function::new(scope, op_search_params_entries).unwrap();
    obj.set(scope, entries_key.into(), entries_func.into());

    let for_each_key = v8::String::new(scope, "forEach").unwrap();
    let for_each_func = v8::Function::new(scope, op_search_params_for_each).unwrap();
    obj.set(scope, for_each_key.into(), for_each_func.into());

    let size_key = v8::String::new(scope, "size").unwrap();
    let size_val = v8::Integer::new(scope, params.len() as i32);
    obj.set(scope, size_key.into(), size_val.into());

    let params_ptr = Box::into_raw(Box::new(params));
    let external = v8::External::new(scope, params_ptr as *mut _);
    let params_key = v8::String::new(scope, "__ferrum_params__").unwrap();
    obj.set(scope, params_key.into(), external.into());

    obj
}

fn op_search_params_to_string(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let result = v8::String::new(scope, &params.to_string()).unwrap();
        rv.set(result.into());
    } else {
        rv.set_undefined();
    }
}

fn op_search_params_get(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let name = if args.length() > 0 {
            let arg = args.get(0);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if let Some(value) = params.get(&name) {
            let result = v8::String::new(scope, &value).unwrap();
            rv.set(result.into());
        } else {
            rv.set_null();
        }
    } else {
        rv.set_null();
    }
}

fn op_search_params_get_all(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let name = if args.length() > 0 {
            let arg = args.get(0);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let values = params.get_all(&name);
        let array = v8::Array::new(scope, values.len() as i32);
        for (i, value) in values.iter().enumerate() {
            let val = v8::String::new(scope, value).unwrap();
            array.set_index(scope, i as u32, val.into());
        }
        rv.set(array.into());
    } else {
        rv.set_null();
    }
}

fn op_search_params_has(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let name = if args.length() > 0 {
            let arg = args.get(0);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let result = v8::Boolean::new(scope, params.has(&name));
        rv.set(result.into());
    } else {
        rv.set_undefined();
    }
}

fn op_search_params_set(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let mut params_opt = get_search_params_from_object_mut(scope, &this);

    if let Some(params) = &mut params_opt {
        let name = if args.length() > 0 {
            let arg = args.get(0);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                return;
            }
        } else {
            return;
        };

        let value = if args.length() > 1 {
            let arg = args.get(1);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        params.set(&name, &value);
    }

    rv.set_undefined();
}

fn op_search_params_append(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let mut params_opt = get_search_params_from_object_mut(scope, &this);

    if let Some(params) = &mut params_opt {
        let name = if args.length() > 0 {
            let arg = args.get(0);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                return;
            }
        } else {
            return;
        };

        let value = if args.length() > 1 {
            let arg = args.get(1);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        params.append(&name, &value);
    }

    rv.set_undefined();
}

fn op_search_params_delete(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let mut params_opt = get_search_params_from_object_mut(scope, &this);

    if let Some(params) = &mut params_opt {
        let name = if args.length() > 0 {
            let arg = args.get(0);
            if arg.is_string() {
                arg.to_rust_string_lossy(scope)
            } else {
                return;
            }
        } else {
            return;
        };

        params.delete(&name);
    }

    rv.set_undefined();
}

fn op_search_params_keys(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let keys = params.keys();
        let array = v8::Array::new(scope, keys.len() as i32);
        for (i, key) in keys.iter().enumerate() {
            let val = v8::String::new(scope, key).unwrap();
            array.set_index(scope, i as u32, val.into());
        }
        rv.set(array.into());
    } else {
        rv.set_null();
    }
}

fn op_search_params_values(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let entries = params.entries();
        let mut values: Vec<String> = entries.into_iter().map(|(_, v)| v).collect();
        let array = v8::Array::new(scope, values.len() as i32);
        for (i, value) in values.iter().enumerate() {
            let val = v8::String::new(scope, value).unwrap();
            array.set_index(scope, i as u32, val.into());
        }
        rv.set(array.into());
    } else {
        rv.set_null();
    }
}

fn op_search_params_entries(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let entries = params.entries();
        let array = v8::Array::new(scope, entries.len() as i32);
        for (i, (key, value)) in entries.iter().enumerate() {
            let entry_array = v8::Array::new(scope, 2);
            let key_val = v8::String::new(scope, key).unwrap();
            let value_val = v8::String::new(scope, value).unwrap();
            entry_array.set_index(scope, 0, key_val.into());
            entry_array.set_index(scope, 1, value_val.into());
            array.set_index(scope, i as u32, entry_array.into());
        }
        rv.set(array.into());
    } else {
        rv.set_null();
    }
}

fn op_search_params_for_each(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let params_ptr = get_search_params_from_object(scope, &this);

    if let Some(params) = params_ptr {
        let callback = if args.length() > 0 {
            let arg = args.get(0);
            if arg.is_function() {
                Some(arg)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(callback) = callback {
            let callback_func = v8::Local::<v8::Function>::try_from(callback).ok();
            if let Some(func) = callback_func {
                let entries = params.entries();
                for (key, value) in entries {
                    let this_val = v8::undefined(scope);
                    let key_v8 = v8::String::new(scope, &key).unwrap();
                    let value_v8 = v8::String::new(scope, &value).unwrap();
                    let args_arr = [value_v8.into(), key_v8.into(), this.into()];

                    let _ = func.call(scope, this_val.into(), &args_arr);
                }
            }
        }
    }

    rv.set_undefined();
}

fn get_search_params_from_object<'a>(
    scope: &mut v8::HandleScope,
    obj: &v8::Local<v8::Object>,
) -> Option<&'a URLSearchParams> {
    let key = v8::String::new(scope, "__ferrum_params__").unwrap();
    if let Some(external_val) = obj.get(scope, key.into()) {
        if external_val.is_external() {
            if let Ok(external) = v8::Local::<v8::External>::try_from(external_val) {
                let ptr = external.value() as *const URLSearchParams;
                if !ptr.is_null() {
                    return Some(unsafe { &*ptr });
                }
            }
        }
    }
    None
}

fn get_search_params_from_object_mut<'a>(
    scope: &mut v8::HandleScope,
    obj: &v8::Local<v8::Object>,
) -> Option<&'a mut URLSearchParams> {
    let key = v8::String::new(scope, "__ferrum_params__").unwrap();
    if let Some(external_val) = obj.get(scope, key.into()) {
        if external_val.is_external() {
            if let Ok(external) = v8::Local::<v8::External>::try_from(external_val) {
                let ptr = external.value() as *mut URLSearchParams;
                if !ptr.is_null() {
                    return Some(unsafe { &mut *ptr });
                }
            }
        }
    }
    None
}

fn op_url_search_params_new(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let input = if args.length() > 0 {
        let arg = args.get(0);
        if arg.is_string() {
            let s = arg.to_rust_string_lossy(scope);
            Some(URLSearchParams::from_query(&s))
        } else if arg.is_object() {
            if let Ok(obj) = v8::Local::<v8::Object>::try_from(arg) {
                let keys = obj.get_own_property_names(scope, v8::GetPropertyNamesArgs::default());
                if let Some(keys) = keys {
                    if let Ok(keys_array) = v8::Local::<v8::Array>::try_from(keys) {
                        let mut map = std::collections::HashMap::new();
                        for i in 0..keys_array.length() {
                            if let Some(key_val) = keys_array.get_index(scope, i) {
                                if let Some(value) = obj.get(scope, key_val) {
                                    if value.is_string() {
                                        let key_str = key_val.to_rust_string_lossy(scope);
                                        let value_str = value.to_rust_string_lossy(scope);
                                        map.insert(key_str, value_str);
                                    }
                                }
                            }
                        }
                        Some(URLSearchParams::from_object(&map))
                    } else {
                        Some(URLSearchParams::new())
                    }
                } else {
                    Some(URLSearchParams::new())
                }
            } else {
                Some(URLSearchParams::new())
            }
        } else {
            Some(URLSearchParams::new())
        }
    } else {
        Some(URLSearchParams::new())
    };

    if let Some(params) = input {
        let obj = create_search_params_object(scope, params);
        rv.set(obj.into());
    } else {
        rv.set_null();
    }
}

/// Create a URL constructor function for V8
pub fn create_url_constructor<'s>(scope: &mut v8::HandleScope<'s>) -> v8::Local<'s, v8::Function> {
    v8::Function::new(scope, op_url_new).unwrap()
}

/// Create a URLSearchParams constructor function for V8
pub fn create_url_search_params_constructor<'s>(
    scope: &mut v8::HandleScope<'s>,
) -> v8::Local<'s, v8::Function> {
    v8::Function::new(scope, op_url_search_params_new).unwrap()
}

/// Bootstrap URL and URLSearchParams as global JavaScript objects
pub fn bootstrap_url_globals(scope: &mut v8::HandleScope) {
    let context = scope.get_current_context();
    let global = context.global(scope);

    {
        let url_key = v8::String::new(scope, "URL").unwrap();
        let url_constructor = create_url_constructor(scope);
        global.set(scope, url_key.into(), url_constructor.into());
    }

    {
        let url_search_params_key = v8::String::new(scope, "URLSearchParams").unwrap();
        let url_search_params_constructor = create_url_search_params_constructor(scope);
        global.set(
            scope,
            url_search_params_key.into(),
            url_search_params_constructor.into(),
        );
    }
}
