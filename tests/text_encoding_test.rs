//! Integration tests for TextEncoder/TextDecoder JavaScript APIs

use ferrum::{create_runtime, create_unsafe_runtime};
use std::sync::Once;

static INIT_V8: Once = Once::new();

fn init_v8_for_tests() {
    INIT_V8.call_once(|| {
        ferrum::init_v8();
    });
}

#[test]
fn test_text_encoder_constructor() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        typeof encoder;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

#[test]
fn test_text_encoder_encoding_property() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        encoder.encoding;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "utf-8");
}

#[test]
fn test_text_encoder_encode() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const bytes = encoder.encode("hello");
        bytes.constructor.name;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Uint8Array");
}

#[test]
fn test_text_encoder_encode_ascii_bytes() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const bytes = encoder.encode("hello");
        bytes[0] === 104 && bytes[1] === 101 && bytes[2] === 108 && bytes[3] === 108 && bytes[4] === 111;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_encoder_encode_unicode() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const bytes = encoder.encode("你好");
        bytes.length === 6;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_encoder_encode_emoji() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const bytes = encoder.encode("🎉");
        bytes.length === 4;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_encoder_encode_empty() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const bytes = encoder.encode("");
        bytes.length === 0;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_encoder_encode_into() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const dest = new Uint8Array(10);
        const result = encoder.encodeInto("hello", dest);
        result.read === 5 && result.written === 5;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_decoder_constructor() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const decoder = new TextDecoder();
        typeof decoder;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

#[test]
fn test_text_decoder_encoding_property() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const decoder = new TextDecoder();
        decoder.encoding;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "utf-8");
}

#[test]
fn test_text_decoder_fatal_property() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const decoder = new TextDecoder();
        decoder.fatal;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "false");
}

#[test]
fn test_text_decoder_fatal_option() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const decoder = new TextDecoder("utf-8", { fatal: true });
        decoder.fatal;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_decoder_decode() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const decoder = new TextDecoder();
        const bytes = new Uint8Array([104, 101, 108, 108, 111]);
        decoder.decode(bytes);
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_text_decoder_decode_unicode() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const decoder = new TextDecoder();
        const bytes = encoder.encode("你好世界");
        decoder.decode(bytes);
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "你好世界");
}

#[test]
fn test_text_decoder_decode_emoji() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const decoder = new TextDecoder();
        const bytes = encoder.encode("🎉");
        decoder.decode(bytes);
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "🎉");
}

#[test]
fn test_text_decoder_decode_empty() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const decoder = new TextDecoder();
        const bytes = new Uint8Array([]);
        decoder.decode(bytes) === "";
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_encoder_decoder_roundtrip() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const decoder = new TextDecoder();
        const original = "Hello, 世界! 🎉";
        const encoded = encoder.encode(original);
        const decoded = decoder.decode(encoded);
        original === decoded;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_text_encoder_encode_into_read_written() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const encoder = new TextEncoder();
        const dest = new Uint8Array(3);
        const result = encoder.encodeInto("hello", dest);
        result.read === 5 && result.written === 3;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}
