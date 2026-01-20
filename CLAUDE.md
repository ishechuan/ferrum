# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Ferrum is a lightweight JavaScript/TypeScript runtime built with Rust, inspired by Deno. It provides a secure runtime environment with an explicit permission model and ES2020 module support.

**Status:** Alpha (v0.1.0) - Core functionality mostly implemented, many features still in progress.

## Common Commands

### Building
```bash
cargo build                    # Debug build
cargo build --release          # Optimized release build
cargo check                    # Quick compilation check
```

### Testing
```bash
cargo test                     # Run all tests
cargo test -- --nocapture      # Run tests with output
cargo test test_name           # Run specific test
RUST_LOG=debug cargo test      # Run with debug logging
```

### Running
```bash
cargo run -- run script.js              # Run a script
cargo run -- repl                       # Start REPL
cargo run -- eval "1+1"                 # Evaluate expression
cargo run -- --allow-read script.js     # Run with permissions
```

### Code Quality
```bash
cargo fmt                      # Format code
cargo clippy                   # Run linter
cargo clippy -- -D warnings    # Lint with warnings as errors
```

### Installation
```bash
cargo install --path .         # Install ferrum binary locally
```

## Architecture

Ferrum follows a **layered architecture**:

1. **CLI Layer** (`src/main.rs`, `src/cli.rs`) - Command-line parsing, permission management, REPL
2. **JavaScript Runtime** (`src/runtime.rs`) - V8 isolate management, script execution
3. **Module Loader** (`src/module_loader.rs`) - ES2020 module resolution, import maps, caching
4. **Ops Layer** (`src/ops/`) - Native operations bridging Rust to JavaScript
   - `ops/fs.rs` - File system operations
   - `ops/net.rs` - Network operations (DNS, HTTP - partially implemented)
   - `ops/timers.rs` - setTimeout/setInterval

### Key Patterns

**Ops Pattern**: Native Rust functions exposed to JavaScript via V8 bindings. All ops must check permissions before executing sensitive operations.

**Permission System**: Default-deny security model with granular controls (read, write, net, env, run). See `src/permissions.rs`.

**Module Loading**: Custom ES2020 loader with support for file://, import maps, and planned HTTP/HTTPS support.

## Important Files

- `src/main.rs` - CLI entry point with command routing
- `src/runtime.rs` - `JsRuntime` struct wrapping V8 isolate
- `src/module_loader.rs` - `ModuleLoader` for ES2020 modules
- `src/permissions.rs` - `Permissions` and `PermissionFlags` types
- `src/ops/mod.rs` - Op registry and dispatch
- `tests/integration_test.rs` - Integration tests

## Known Limitations

- HTTP/HTTPS fetch API designed but not implemented
- WebSocket support designed but not implemented
- setInterval needs proper FnMut callback handling
- Native ops not yet exposed to JavaScript (V8-Rust bridging incomplete)
- TypeScript support planned for Phase 4
- Source maps, test runner, formatter, debugger not implemented

## Development Notes

- **V8 Platform**: Initialized once per process using `OnceLock` in `runtime.rs`
- **Async Runtime**: Uses Tokio with full features enabled
- **Error Handling**: Custom error types via `thiserror` in each module
- **Release Profile**: Configured with LTO, codegen-units=1, strip=true for optimal binary size
- **Tests**: Use `init_v8_for_tests()` wrapper for V8 initialization in test code
