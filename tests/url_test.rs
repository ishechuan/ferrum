//! Integration tests for URL/URLSearchParams JavaScript APIs

use ferrum::{create_runtime, create_unsafe_runtime};
use std::sync::Once;

static INIT_V8: Once = Once::new();

fn init_v8_for_tests() {
    INIT_V8.call_once(|| {
        ferrum::init_v8();
    });
}

#[test]
fn test_url_constructor() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com:8080/path?query=value#hash");
        typeof url;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

#[test]
fn test_url_href() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com:8080/path?query=value#hash");
        url.href;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    let href = result.unwrap();
    assert!(href.contains("https://"));
    assert!(href.contains("example.com"));
    assert!(href.contains("8080"));
    assert!(href.contains("/path"));
}

#[test]
fn test_url_protocol() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com");
        url.protocol;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "https:");
}

#[test]
fn test_url_host() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com:8080/path");
        url.host;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "example.com:8080");
}

#[test]
fn test_url_hostname() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com:8080/path");
        url.hostname;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "example.com");
}

#[test]
fn test_url_port() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com:8080/path");
        url.port;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "8080");
}

#[test]
fn test_url_port_empty() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path");
        url.port;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn test_url_pathname() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path/to/file");
        url.pathname;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "/path/to/file");
}

#[test]
fn test_url_search() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path?foo=bar&baz=qux");
        url.search;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "?foo=bar&baz=qux");
}

#[test]
fn test_url_search_empty() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path");
        url.search;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn test_url_hash() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path#section");
        url.hash;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "#section");
}

#[test]
fn test_url_hash_empty() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path");
        url.hash;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn test_url_origin() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com:8080/path?query=value");
        url.origin;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "https://example.com:8080");
}

#[test]
fn test_url_username() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://user:pass@example.com/path");
        url.username;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "user");
}

#[test]
fn test_url_password() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://user:pass@example.com/path");
        url.password;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "pass");
}

#[test]
fn test_url_search_params() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path?foo=bar&foo=baz");
        typeof url.searchParams;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

#[test]
fn test_url_search_params_get() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path?foo=bar&baz=qux");
        url.searchParams.get("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bar");
}

#[test]
fn test_url_search_params_get_all() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path?foo=bar&foo=baz");
        url.searchParams.getAll("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("bar"));
}

#[test]
fn test_url_search_params_has() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path?foo=bar");
        url.searchParams.has("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_url_search_params_has_false() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path?foo=bar");
        url.searchParams.has("baz");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "false");
}

#[test]
fn test_url_to_string() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("https://example.com/path");
        url.toString();
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("https://example.com/path"));
}

#[test]
fn test_url_invalid() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        try {
            new URL("not-a-url");
            "no error";
        } catch (e) {
            "error";
        }
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "error");
}

#[test]
fn test_url_with_base() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("relative/path", "https://example.com/base/");
        url.hostname;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "example.com");
}

#[test]
fn test_url_file_scheme() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("file:///tmp/test.txt");
        url.pathname;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "/tmp/test.txt");
}

#[test]
fn test_urlsearchparams_constructor_string() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams("foo=bar&baz=qux");
        params.get("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bar");
}

#[test]
fn test_urlsearchparams_constructor_object() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams({ foo: "bar", baz: "qux" });
        params.get("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bar");
}

#[test]
fn test_urlsearchparams_constructor_empty() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams();
        params.toString();
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn test_urlsearchparams_set() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams();
        params.set("foo", "bar");
        params.get("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bar");
}

#[test]
fn test_urlsearchparams_append() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams();
        params.append("foo", "bar");
        params.append("foo", "baz");
        const all = params.getAll("foo");
        all.length === 2;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

#[test]
fn test_urlsearchparams_delete() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams("foo=bar&baz=qux");
        params.delete("foo");
        params.has("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "false");
}

#[test]
fn test_urlsearchparams_keys() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams("foo=bar&baz=qux");
        const keys = params.keys();
        keys.length;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "2");
}

#[test]
fn test_urlsearchparams_entries() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams("foo=bar");
        const entries = params.entries();
        const first = entries[0];
        first[0];
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "foo");
}

#[test]
fn test_urlsearchparams_to_string() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams("foo=bar&baz=qux");
        params.toString();
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    let s = result.unwrap();
    assert!(s.contains("foo=bar"));
    assert!(s.contains("baz=qux"));
}

#[test]
fn test_urlsearchparams_encoding() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams("foo=hello%20world");
        params.get("foo");
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello world");
}

#[test]
fn test_urlsearchparams_special_chars() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams();
        params.set("name", "hello world");
        params.toString();
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    let s = result.unwrap();
    assert!(s.contains("name=hello%20world"));
}

#[test]
fn test_url_search_params_size() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const params = new URLSearchParams("foo=bar&baz=qux");
        params.size;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "2");
}

#[test]
fn test_url_relative_path_resolution() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("../other/path.js", "https://example.com/dir/file.js");
        url.pathname;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "/other/path.js");
}

#[test]
fn test_url_query_only() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const url = new URL("?foo=bar", "https://example.com/path");
        url.search;
    "#;
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "?foo=bar");
}

#[test]
fn test_url_fragment_only() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = "const url = new URL(\"#section\", \"https://example.com/path\");\nurl.hash;";
    let result = runtime.execute(code, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "#section");
}
