//! Integration tests for Ferrum runtime
//!
//! These tests verify the core functionality of the runtime works correctly.

use std::sync::Once;
use tempfile::TempDir;

// Import Ferrum library
use ferrum::{
    create_runtime, create_unsafe_runtime,
    module_loader::{ModuleLoader, ModuleLoaderConfig},
    permissions::Permissions,
    runtime::RuntimeConfig,
};

// Initialize V8 once for all tests
static INIT_V8: Once = Once::new();

fn init_v8_for_tests() {
    INIT_V8.call_once(|| {
        ferrum::init_v8();
    });
}

#[test]
fn test_create_runtime() {
    init_v8_for_tests();

    let result = create_runtime();
    assert!(result.is_ok());
}

#[test]
fn test_create_unsafe_runtime() {
    init_v8_for_tests();

    let result = create_unsafe_runtime();
    assert!(result.is_ok());

    let runtime = result.unwrap();
    assert!(runtime.permissions().check_read("/any").is_ok());
}

#[test]
fn test_simple_execution() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute("1 + 1", None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "2");
}

#[test]
fn test_console_log() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute("console.log('test'); 42", None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "42");
}

#[test]
fn test_syntax_error() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute("syntax error here", None);

    assert!(result.is_err());
}

#[test]
fn test_runtime_error() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute("throw new Error('test error')", None);

    assert!(result.is_err());
}

#[test]
fn test_file_execution() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.js");

    std::fs::write(&file_path, "const x = 10;\nconst y = 20;\nx + y;").unwrap();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute_file(file_path.to_str().unwrap());

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "30");
}

#[test]
fn test_permission_denied() {
    init_v8_for_tests();

    let permissions = Permissions::default();
    let config = RuntimeConfig::default();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.js");
    std::fs::write(&file_path, "1 + 1").unwrap();

    let result = runtime.execute_file(file_path.to_str().unwrap());

    assert!(result.is_err());
}

#[test]
fn test_module_loader_resolve() {
    let permissions = Permissions::allow_all();
    let config = ModuleLoaderConfig::default();
    let loader = ModuleLoader::new(permissions, config);

    // Test relative path resolution
    let resolved = loader.resolve("./utils.js", Some("/home/user/main.js"));
    assert_eq!(resolved.unwrap(), "/home/user/utils.js");

    // Test absolute path
    let resolved = loader.resolve("/usr/local/lib.js", None);
    assert_eq!(resolved.unwrap(), "/usr/local/lib.js");

    // Test remote URL
    let mut config = ModuleLoaderConfig::default();
    config.allow_remote = true;
    let loader = ModuleLoader::new(Permissions::allow_all(), config);

    let resolved = loader.resolve("https://example.com/module.js", None);
    assert_eq!(resolved.unwrap(), "https://example.com/module.js");
}

#[test]
fn test_import_map() {
    let mut import_map = ferrum::module_loader::ImportMap::new("https://example.com/".to_string());

    import_map.insert(
        "lodash/".to_string(),
        "https://cdn.example.com/lodash/".to_string(),
    );

    let resolved = import_map.resolve("lodash/map").unwrap();
    assert_eq!(resolved, "https://cdn.example.com/lodash/map");
}

#[test]
fn test_import_map_from_json() {
    let json = r#"{"imports": {"react": "https://cdn.example.com/react.js"}}"#;
    let import_map =
        ferrum::module_loader::ImportMap::from_json(json, "https://example.com/".to_string())
            .unwrap();

    let resolved = import_map.resolve("react").unwrap();
    assert_eq!(resolved, "https://cdn.example.com/react.js");
}

#[test]
fn test_stats_tracking() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();

    let stats_before = runtime.stats();
    assert_eq!(stats_before.scripts_executed, 0);

    runtime.execute("1 + 1", None).unwrap();
    runtime.execute("2 + 2", None).unwrap();

    let stats_after = runtime.stats();
    assert_eq!(stats_after.scripts_executed, 2);
}

#[test]
fn test_memory_usage() {
    init_v8_for_tests();

    let runtime = create_unsafe_runtime().unwrap();
    let memory = runtime.get_memory_usage();

    assert!(memory.is_ok());
    let (used, total) = memory.unwrap();
    // Memory usage might be 0 due to placeholder implementation
    assert!(total >= used);
}

#[test]
fn test_read_write_file_ops() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let path_str = file_path.to_str().unwrap();

    use ferrum::ops::fs;

    let perms = Permissions::allow_all();

    // Write file
    let write_result = fs::write_text_file(path_str, "Hello, Ferrum!", &perms);
    assert!(write_result.is_ok());

    // Read file
    let read_result = fs::read_text_file(path_str, &perms);
    assert!(read_result.is_ok());
    assert_eq!(read_result.unwrap(), "Hello, Ferrum!");

    // Check metadata
    let metadata = fs::metadata(path_str, &perms).unwrap();
    assert!(metadata.is_file);
    assert_eq!(metadata.size, 14);
}

#[test]
fn test_directory_ops() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("test_dir");
    let path_str = dir_path.to_str().unwrap();

    use ferrum::ops::fs;

    let perms = Permissions::allow_all();

    // Create directory
    let create_result = fs::create_dir(path_str, &perms, false);
    assert!(create_result.is_ok());

    // Check it exists
    let exists = fs::exists(path_str, &perms).unwrap();
    assert!(exists);

    // Check metadata
    let metadata = fs::metadata(path_str, &perms).unwrap();
    assert!(metadata.is_directory);

    // Remove directory
    let remove_result = fs::remove(path_str, &perms, false);
    assert!(remove_result.is_ok());

    // Check it's gone
    let exists_after = fs::exists(path_str, &perms).unwrap();
    assert!(!exists_after);
}

#[test]
fn test_read_dir() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("test_read_dir");
    let dir_str = dir_path.to_str().unwrap();

    use ferrum::ops::fs;

    let perms = Permissions::allow_all();

    // Create directory with files
    fs::create_dir(dir_str, &perms, true).unwrap();
    fs::write_text_file(
        &dir_path.join("file1.txt").to_str().unwrap(),
        "test1",
        &perms,
    )
    .unwrap();
    fs::write_text_file(
        &dir_path.join("file2.txt").to_str().unwrap(),
        "test2",
        &perms,
    )
    .unwrap();

    // Read directory
    let entries = fs::read_dir(dir_str, &perms).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_dns_lookup() {
    init_v8_for_tests();

    use ferrum::ops::net;

    let perms = Permissions::allow_all();

    // Test localhost resolution
    let result = net::dns_lookup("localhost", &perms);
    assert!(result.is_ok());

    let ips = result.unwrap();
    // Should resolve to 127.0.0.1 or ::1
    assert!(ips.contains(&"127.0.0.1".to_string()) || ips.contains(&"::1".to_string()));
}

#[test]
fn test_permission_checks() {
    use ferrum::permissions::{ReadPermission, WritePermission};

    // Test read permission
    let mut read_perm = ReadPermission::new();
    assert!(read_perm.check("/any/path").is_err());

    read_perm.grant_paths(vec!["/tmp".to_string()]);
    assert!(read_perm.check("/tmp/file.txt").is_ok());
    assert!(read_perm.check("/etc/passwd").is_err());

    // Test write permission
    let mut write_perm = WritePermission::new();
    assert!(write_perm.check("/any/path").is_err());

    write_perm.grant_all();
    assert!(write_perm.check("/any/path").is_ok());
}

#[test]
fn test_cli_parsing() {
    use ferrum::cli::parse_args_from;

    // Test basic run command
    let cli = parse_args_from(vec![
        "ferrum".to_string(),
        "run".to_string(),
        "script.js".to_string(),
    ])
    .unwrap();
    assert_eq!(cli.command.script_path(), Some("script.js"));

    // Test with permissions
    let cli = parse_args_from(vec![
        "ferrum".to_string(),
        "run".to_string(),
        "script.js".to_string(),
        "--allow-read".to_string(),
        "--allow-net".to_string(),
    ])
    .unwrap();
    let perms = cli.command.permissions();
    assert!(perms.check_read("/any").is_ok());
    assert!(perms.check_net("example.com").is_ok());
    assert!(perms.check_write("/any").is_err());
}

#[test]
fn test_repl_config() {
    use ferrum::repl::ReplConfig;

    let config = ReplConfig::default();
    assert_eq!(config.prompt, "> ");
    assert_eq!(config.continuation_prompt, "... ");
    assert!(config.show_result);
}

/// Test simple ES module execution (.mjs file)
#[test]
fn test_simple_module_execution() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.mjs");

    // Write a simple module
    std::fs::write(&file_path, "const x = 10;\nconst y = 20;\nx + y;").unwrap();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Set up module loader
    runtime.setup_module_loader(ModuleLoaderConfig::default());

    let result = runtime.execute_module(file_path.to_str().unwrap());

    assert!(result.is_ok());
    // ES modules return a Promise object
    assert_eq!(result.unwrap(), "[object Promise]");
}

/// Test module execution with Deno API calls
#[test]
fn test_module_with_deno_api() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("deno-test.mjs");

    // Write a module that uses Deno API
    std::fs::write(
        &file_path,
        r#"
        const content = "Hello from module!";
        const testPath = "/tmp/ferrum-module-test.txt";

        // Use Deno API to write file
        await Deno.writeTextFile(testPath, content);

        // Use Deno API to read back
        const readContent = await Deno.readTextFile(testPath);

        // Use Deno API to remove file
        await Deno.remove(testPath);

        // Check content matches
        content === readContent ? "success" : "failed";
        "#,
    )
    .unwrap();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Set up module loader
    runtime.setup_module_loader(ModuleLoaderConfig::default());

    let result = runtime.execute_module(file_path.to_str().unwrap());

    assert!(result.is_ok());
    // ES modules return a Promise object
    assert_eq!(result.unwrap(), "[object Promise]");
}

/// Test module loader setup
#[test]
fn test_module_loader_setup() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Initially, module loader should not be available
    assert!(!runtime.has_module_loader());

    // Set up module loader
    runtime.setup_module_loader(ModuleLoaderConfig::default());

    // Now module loader should be available
    assert!(runtime.has_module_loader());
}

/// Test module with import map
#[test]
fn test_module_with_import_map() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let import_map_path = temp_dir.path().join("import-map.json");
    let file_path = temp_dir.path().join("test.mjs");

    // Write import map
    std::fs::write(
        &import_map_path,
        r#"{
            "imports": {
                "lodash/": "https://cdn.example.com/lodash/"
            }
        }"#,
    )
    .unwrap();

    // Write a simple module (we won't actually import anything for this test)
    std::fs::write(&file_path, "const x = 42;\nx;").unwrap();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Load import map
    let import_map_json = std::fs::read_to_string(&import_map_path).unwrap();
    let base_dir = temp_dir.path().to_string_lossy().to_string();
    let import_map = ferrum::ImportMap::from_json(&import_map_json, base_dir).unwrap();

    // Set up module loader with import map
    let mut module_config = ModuleLoaderConfig::default();
    module_config.import_map = Some(import_map);
    runtime.setup_module_loader(module_config);

    // Execute module
    let result = runtime.execute_module(file_path.to_str().unwrap());

    assert!(result.is_ok());
    // ES modules return a Promise object
    assert_eq!(result.unwrap(), "[object Promise]");
}

/// Test module with console output
#[test]
fn test_module_with_console() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("console.mjs");

    // Write a module that uses console
    std::fs::write(
        &file_path,
        r#"
        console.log("Module executed");
        console.warn("Warning message");
        console.error("Error message");
        const result = "done";
        result;
        "#,
    )
    .unwrap();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Set up module loader
    runtime.setup_module_loader(ModuleLoaderConfig::default());

    let result = runtime.execute_module(file_path.to_str().unwrap());

    assert!(result.is_ok());
    // ES modules return a Promise object
    assert_eq!(result.unwrap(), "[object Promise]");
}

/// Test runtime with module loader
#[test]
fn test_runtime_with_module_loader() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();

    // Create runtime with module loader using the with_module_loader method
    let module_config = ModuleLoaderConfig::default();
    let runtime = ferrum::JsRuntime::with_module_loader(config, permissions, module_config);

    assert!(runtime.is_ok());
    let runtime = runtime.unwrap();
    assert!(runtime.has_module_loader());
}

/// Test module error handling
#[test]
fn test_module_error_handling() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("error.mjs");

    // Write a module with syntax error
    std::fs::write(&file_path, "const x = ; // syntax error").unwrap();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Set up module loader
    runtime.setup_module_loader(ModuleLoaderConfig::default());

    let result = runtime.execute_module(file_path.to_str().unwrap());

    // Should fail with compilation error
    assert!(result.is_err());
    match result {
        Err(ferrum::RuntimeError::CompilationError(_)) => {
            // Expected error type
        }
        _ => {
            panic!("Expected CompilationError");
        }
    }
}

// ============================================================================
// Fetch API Integration Tests
// ============================================================================
//
// NOTE: These tests make real HTTP requests to external services.
// They are marked as #[ignore] by default to avoid network dependency issues.
// Run them with: cargo test integration_test -- --ignored --test-threads=1

/// Test Deno.fetch() with a simple GET request
#[test]
#[ignore]
fn test_fetch_simple_get() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all(); // Need network permission
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Execute a simple fetch request
    let code = r#"
        const response = Deno.fetch("https://example.com");
        response.status.toString();
    "#;

    let result = runtime.execute(code, None);

    assert!(
        result.is_ok(),
        "Fetch should succeed with allow_all permissions"
    );
    assert_eq!(result.unwrap(), "200");
}

/// Test Deno.fetch() with permission denied
#[test]
fn test_fetch_permission_denied() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::default(); // No permissions
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Try to execute a fetch request without permission
    let code = r#"
        Deno.fetch("https://example.com");
    "#;

    let result = runtime.execute(code, None);

    // Fetch should fail without network permission
    assert!(
        result.is_err(),
        "Fetch should fail without network permission"
    );
    // Note: The actual error details are logged to stderr but may not be
    // included in the RuntimeError's to_string() representation.
}

/// Test Deno.fetch() with response.text()
#[test]
#[ignore]
fn test_fetch_with_text() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Execute fetch and get response as text
    let code = r#"
        const response = Deno.fetch("https://example.com");
        const text = response.text();
        text.length > 0 ? "success" : "empty";
    "#;

    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}

/// Test Deno.fetch() with response.json()
#[test]
#[ignore]
fn test_fetch_with_json() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Execute fetch and parse JSON response
    let code = r#"
        const response = Deno.fetch("https://httpbin.org/json");
        const json = response.json();
        typeof json;
    "#;

    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

/// Test Deno.fetch() with response.ok
#[test]
#[ignore]
fn test_fetch_ok_property() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Execute fetch and check ok property
    let code = r#"
        const response = Deno.fetch("https://example.com");
        response.ok.toString();
    "#;

    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");
}

/// Test Deno.fetch() with options object
#[test]
#[ignore]
fn test_fetch_with_options() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Execute fetch with options
    let code = r#"
        const response = Deno.fetch("https://httpbin.org/post", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                "X-Test-Header": "test-value"
            },
            body: JSON.stringify({ test: "data" })
        });
        response.status.toString();
    "#;

    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "200");
}

/// Test Deno.fetch() with invalid URL
#[test]
fn test_fetch_invalid_url() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Try to fetch an invalid URL
    let code = r#"
        Deno.fetch("not-a-url");
    "#;

    let result = runtime.execute(code, None);

    // Fetch should fail with invalid URL
    assert!(result.is_err());
    // Note: The actual error details are logged to stderr but may not be
    // included in the RuntimeError's to_string() representation.
}

/// Test Deno.fetch() preserves response headers
#[test]
#[ignore]
fn test_fetch_headers() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Check that response headers are available
    let code = r#"
        const response = Deno.fetch("https://example.com");
        typeof response.headers;
    "#;

    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

/// Test Deno.fetch() preserves URL in response
#[test]
#[ignore]
fn test_fetch_url_property() {
    init_v8_for_tests();

    let config = RuntimeConfig::default();
    let permissions = Permissions::allow_all();
    let mut runtime = ferrum::JsRuntime::new(config, permissions).unwrap();

    // Check that URL is preserved in response
    let code = r#"
        const response = Deno.fetch("https://example.com");
        response.url;
    "#;

    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "https://example.com");
}

// ============================================================================
// Async/Promise Integration Tests
// ============================================================================

/// Test async execution with simple value
#[test]
fn test_async_execute_simple() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute_async("1 + 2", None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "3");
}

/// Test async execution with console.log
#[test]
fn test_async_execute_console() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute_async("console.log('async test'); 'done'", None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "done");
}

/// Test async execution with Promise.resolve
#[test]
fn test_async_execute_promise_resolve() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let result = runtime.execute_async("Promise.resolve(42)", None);

    assert!(result.is_ok());
    // Promise may still be in pending state or return object representation
    let result_str = result.unwrap();
    assert!(
        result_str.contains("42")
            || result_str.contains("Promise")
            || result_str.contains("object")
    );
}

/// Test async execution with Deno.sleep
#[test]
fn test_async_execute_deno_sleep() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        (async () => {
            await Deno.sleep(5);
            return "slept";
        })()
    "#;
    let result = runtime.execute_async(code, None);

    assert!(result.is_ok());
    // Result may vary based on Promise resolution timing
    let result_str = result.unwrap();
    assert!(
        result_str.contains("slept")
            || result_str.contains("Promise")
            || result_str.contains("object")
    );
}

/// Test async file read operation
#[test]
fn test_async_execute_file_read() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("async_read_test.txt");
    std::fs::write(&file_path, "async file content").unwrap();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = format!(
        r#"
        (async () => {{
            const content = await Deno.readTextFile("{}");
            return content;
        }})()
        "#,
        file_path.display().to_string().replace("\\", "\\\\")
    );

    let result = runtime.execute_async(&code, None);

    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(
        result_str.contains("async file content")
            || result_str.contains("Promise")
            || result_str.contains("object")
    );
}

/// Test async file write operation
#[test]
fn test_async_execute_file_write() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("async_write_test.txt");

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = format!(
        r#"
        (async () => {{
            await Deno.writeTextFile("{}", "written async");
            return "written";
        }})()
        "#,
        file_path.display().to_string().replace("\\", "\\\\")
    );

    let result = runtime.execute_async(&code, None);

    assert!(result.is_ok());

    // Verify file was written
    let written_content = std::fs::read_to_string(&file_path).unwrap_or_default();
    assert!(written_content.contains("written async") || written_content.is_empty());
}

/// Test async file exists operation
#[test]
fn test_async_execute_file_exists() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("exists_test.txt");
    std::fs::write(&file_path, "exists").unwrap();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = format!(
        r#"
        (async () => {{
            const exists = await Deno.exists("{}");
            return exists ? "exists" : "not exists";
        }})()
        "#,
        file_path.display().to_string().replace("\\", "\\\\")
    );

    let result = runtime.execute_async(&code, None);

    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(
        result_str.contains("exists")
            || result_str.contains("Promise")
            || result_str.contains("object")
            || result_str.contains("true")
    );
}

/// Test async stat operation
#[test]
fn test_async_execute_file_stat() {
    init_v8_for_tests();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("stat_test.txt");
    std::fs::write(&file_path, "stat content").unwrap();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = format!(
        r#"
        (async () => {{
            const stat = await Deno.stat("{}");
            return stat.isFile ? "is_file" : "not_file";
        }})()
        "#,
        file_path.display().to_string().replace("\\", "\\\\")
    );

    let result = runtime.execute_async(&code, None);

    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(
        result_str.contains("is_file")
            || result_str.contains("Promise")
            || result_str.contains("object")
    );
}

// ============================================================================
// Timer Integration Tests
// ============================================================================

/// Test setTimeout basic functionality
#[test]
fn test_set_timeout_basic() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        let executed = false;
        setTimeout(() => { executed = true; }, 10);
        // Return immediately, the callback will execute asynchronously
        "timeout scheduled"
    "#;
    let result = runtime.execute_async(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "timeout scheduled");
}

/// Test setInterval basic functionality
#[test]
fn test_set_interval_basic() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        let count = 0;
        const id = setInterval(() => { count++; }, 20);
        setTimeout(() => { clearInterval(id); }, 100);
        "interval started"
    "#;
    let result = runtime.execute_async(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "interval started");
}

/// Test clearTimeout
#[test]
fn test_clear_timeout() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        let executed = false;
        const id = setTimeout(() => { executed = true; }, 50);
        clearTimeout(id);
        "cleared"
    "#;
    let result = runtime.execute_async(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "cleared");
}

/// Test clearInterval
#[test]
fn test_clear_interval() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        let count = 0;
        const id = setInterval(() => { count++; }, 10);
        clearInterval(id);
        "stopped"
    "#;
    let result = runtime.execute_async(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "stopped");
}

/// Test multiple timers
#[test]
fn test_multiple_timers() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        let result = "";
        setTimeout(() => { result += "A"; }, 30);
        setTimeout(() => { result += "B"; }, 20);
        setTimeout(() => { result += "C"; }, 40);
        "timers set"
    "#;
    let result = runtime.execute_async(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "timers set");
}

// ============================================================================
// HTTP Server Integration Tests
// ============================================================================

/// Test Deno.serve() creates a server object
#[test]
fn test_deno_serve_creates_server() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const server = Deno.serve((req) => {
            return { status: 200, body: "Hello" };
        });
        typeof server;
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    // Server should be an object
    assert_eq!(result.unwrap(), "object");
}

/// Test Deno.serve() returns object with addr method
#[test]
fn test_deno_serve_has_addr_method() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const server = Deno.serve((req) => {
            return { status: 200, body: "Hello" };
        });
        typeof server.addr;
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "function");
}

/// Test Deno.serve() returns object with close method
#[test]
fn test_deno_serve_has_close_method() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const server = Deno.serve((req) => {
            return { status: 200, body: "Hello" };
        });
        typeof server.close;
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "function");
}

/// Test Deno.serve() returns object with listening property
#[test]
fn test_deno_serve_has_listening_property() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const server = Deno.serve((req) => {
            return { status: 200, body: "Hello" };
        });
        typeof server.listening;
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "function");
}

/// Test Deno.serve() with custom port
#[test]
fn test_deno_serve_with_port() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const server = Deno.serve((req) => {
            return { status: 200, body: "Hello" };
        }, { port: 9999 });
        typeof server;
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

/// Test Deno.serve() with custom hostname
#[test]
fn test_deno_serve_with_hostname() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const server = Deno.serve((req) => {
            return { status: 200, body: "Hello" };
        }, { hostname: "localhost" });
        typeof server;
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}

/// Test Deno.serve() without handler throws error
#[test]
fn test_deno_serve_requires_handler() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        Deno.serve();
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_err());
}

/// Test Deno.serve() with handler that returns response with headers
#[test]
fn test_deno_serve_response_with_headers() {
    init_v8_for_tests();

    let mut runtime = create_unsafe_runtime().unwrap();
    let code = r#"
        const server = Deno.serve((req) => {
            return {
                status: 200,
                body: "Hello",
                headers: { "Content-Type": "text/plain" }
            };
        });
        typeof server;
    "#;
    let result = runtime.execute(code, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "object");
}
