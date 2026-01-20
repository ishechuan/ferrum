//! TextEncoder/TextDecoder V8 Bindings
//!
//! This module provides V8 function callbacks that expose TextEncoder and TextDecoder
//! to JavaScript as global objects.

use std::cell::Cell;

use v8;

use crate::ops::text_encoding::TextDecoder;

fn op_text_encoder_new(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let obj = v8::Object::new(scope);

    let encoding_key = v8::String::new(scope, "encoding").unwrap();
    let encoding_val = v8::String::new(scope, "utf-8").unwrap();
    obj.set(scope, encoding_key.into(), encoding_val.into());

    let encode_key = v8::String::new(scope, "encode").unwrap();
    let encode_func = v8::Function::new(scope, op_text_encoder_encode).unwrap();
    obj.set(scope, encode_key.into(), encode_func.into());

    let encode_into_key = v8::String::new(scope, "encodeInto").unwrap();
    let encode_into_func = v8::Function::new(scope, op_text_encoder_encode_into).unwrap();
    obj.set(scope, encode_into_key.into(), encode_into_func.into());

    rv.set(obj.into());
}

fn op_text_encoder_encode(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let input = if args.length() > 0 {
        let arg = args.get(0);
        if arg.is_string() {
            arg.to_rust_string_lossy(scope)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let bytes = input.as_bytes().to_vec();

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

fn op_text_encoder_encode_into(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let src = if args.length() > 0 {
        let arg = args.get(0);
        if arg.is_string() {
            arg.to_rust_string_lossy(scope)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut dest_vec = if args.length() > 1 {
        let arg = args.get(1);
        if arg.is_uint8_array() {
            let array = v8::Local::<v8::Uint8Array>::try_from(arg).ok();
            if let Some(array) = array {
                let buffer = array.buffer(scope).unwrap();
                let backing_store = buffer.get_backing_store();
                let offset = array.byte_offset() as usize;
                let length = array.byte_length() as usize;
                let mut v = Vec::with_capacity(length);
                for i in 0..length {
                    v.push(backing_store[offset + i].get());
                }
                v
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let src_bytes = src.as_bytes();
    let src_len = src_bytes.len();
    let dest_len = dest_vec.len();
    let written = std::cmp::min(src_len, dest_len);
    dest_vec[..written].copy_from_slice(&src_bytes[..written]);

    let result_obj = v8::Object::new(scope);
    let read_key = v8::String::new(scope, "read").unwrap();
    let written_key = v8::String::new(scope, "written").unwrap();
    let read_val = v8::Integer::new(scope, src_len as i32);
    let written_val = v8::Integer::new(scope, written as i32);
    result_obj.set(scope, read_key.into(), read_val.into());
    result_obj.set(scope, written_key.into(), written_val.into());

    rv.set(result_obj.into());
}

fn op_text_decoder_new(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let label = if args.length() > 0 {
        let arg = args.get(0);
        if arg.is_string() {
            Some(arg.to_rust_string_lossy(scope))
        } else {
            None
        }
    } else {
        None
    };

    let fatal = if args.length() > 1 {
        let arg = args.get(1);
        if arg.is_object() {
            if let Ok(obj) = v8::Local::<v8::Object>::try_from(arg) {
                let key = v8::String::new(scope, "fatal").unwrap();
                if let Some(val) = obj.get(scope, key.into()) {
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

    let ignore_bom = if args.length() > 1 {
        let arg = args.get(1);
        if arg.is_object() {
            if let Ok(obj) = v8::Local::<v8::Object>::try_from(arg) {
                let key = v8::String::new(scope, "ignoreBOM").unwrap();
                if let Some(val) = obj.get(scope, key.into()) {
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

    let decoder = TextDecoder::with_options(label, Some(fatal), Some(ignore_bom));

    let obj = v8::Object::new(scope);

    let encoding_key = v8::String::new(scope, "encoding").unwrap();
    let encoding_val = v8::String::new(scope, decoder.encoding()).unwrap();
    obj.set(scope, encoding_key.into(), encoding_val.into());

    let fatal_key = v8::String::new(scope, "fatal").unwrap();
    let fatal_val = v8::Boolean::new(scope, decoder.fatal());
    obj.set(scope, fatal_key.into(), fatal_val.into());

    let ignore_bom_key = v8::String::new(scope, "ignoreBOM").unwrap();
    let ignore_bom_val = v8::Boolean::new(scope, decoder.ignore_bom());
    obj.set(scope, ignore_bom_key.into(), ignore_bom_val.into());

    let decode_key = v8::String::new(scope, "decode").unwrap();
    let decode_func = v8::Function::new(scope, op_text_decoder_decode).unwrap();
    obj.set(scope, decode_key.into(), decode_func.into());

    rv.set(obj.into());
}

fn op_text_decoder_decode(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let bytes = if args.length() > 0 {
        let arg = args.get(0);

        if arg.is_array_buffer() {
            let buffer = v8::Local::<v8::ArrayBuffer>::try_from(arg).ok();
            if let Some(buffer) = buffer {
                let backing_store = buffer.get_backing_store();
                backing_store
                    .iter()
                    .map(|cell: &Cell<u8>| cell.get())
                    .collect()
            } else {
                vec![]
            }
        } else if arg.is_uint8_array() {
            let array = v8::Local::<v8::Uint8Array>::try_from(arg).ok();
            if let Some(array) = array {
                let buffer = array.buffer(scope).unwrap();
                let backing_store = buffer.get_backing_store();
                let offset = array.byte_offset() as usize;
                let length = array.byte_length() as usize;
                backing_store
                    .iter()
                    .skip(offset)
                    .take(length)
                    .map(|cell: &Cell<u8>| cell.get())
                    .collect()
            } else {
                vec![]
            }
        } else if arg.is_string() {
            let str_val = arg.to_rust_string_lossy(scope);
            str_val.into_bytes()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let decoder = TextDecoder::with_options(Some("utf-8".to_string()), Some(false), Some(false));
    let result = TextDecoder::decode(&decoder, &bytes, false);

    let result_str = v8::String::new(scope, &result).unwrap();
    rv.set(result_str.into());
}

/// Create a TextEncoder constructor function
pub fn create_text_encoder_constructor<'s>(
    scope: &mut v8::HandleScope<'s>,
) -> v8::Local<'s, v8::Function> {
    v8::Function::new(scope, op_text_encoder_new).unwrap()
}

/// Create a TextDecoder constructor function
pub fn create_text_decoder_constructor<'s>(
    scope: &mut v8::HandleScope<'s>,
) -> v8::Local<'s, v8::Function> {
    v8::Function::new(scope, op_text_decoder_new).unwrap()
}

/// Bootstrap TextEncoder and TextDecoder globals
pub fn bootstrap_text_encoding_globals(scope: &mut v8::HandleScope) {
    let context = scope.get_current_context();
    let global = context.global(scope);

    {
        let text_encoder_key = v8::String::new(scope, "TextEncoder").unwrap();
        let text_encoder_constructor = create_text_encoder_constructor(scope);
        global.set(
            scope,
            text_encoder_key.into(),
            text_encoder_constructor.into(),
        );
    }

    {
        let text_decoder_key = v8::String::new(scope, "TextDecoder").unwrap();
        let text_decoder_constructor = create_text_decoder_constructor(scope);
        global.set(
            scope,
            text_decoder_key.into(),
            text_decoder_constructor.into(),
        );
    }
}
