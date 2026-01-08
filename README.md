# Ferrum

> A simple, secure, and modern JavaScript/TypeScript Runtime

Ferrum is a lightweight JavaScript and TypeScript runtime inspired by Deno, built with Rust. It aims to provide a secure and productive environment for running JavaScript/TypeScript outside the browser.

English | [简体中文](README.zh-CN.md)

## Status

**Version:** 0.1.0 (Alpha)

This is an early-stage project. Core functionality is implemented, but many features are still in development. See [Current Limitations](#current-limitations) for details.

## Features

### Core
- **Security**: Explicit permission model for file system, network, and environment access
- **Modern ESM**: ES2020 module support with import maps
- **Fast**: Built on V8 JavaScript engine
- **Single Binary**: Distributed as a single executable

### Standard Library
- **File System API**: Read, write, copy, rename, directory operations
- **Network Operations**: DNS resolution (HTTP/TCP planned)
- **Timer API**: setTimeout, promises (setInterval in progress)
- **Path Utilities**: Cross-platform path manipulation

### Developer Experience
- **REPL**: Interactive shell with multi-line support
- **CLI**: Rich command-line interface with permission flags
- **Testing**: Built-in test framework

## Installation

### From Source
```bash
# Clone the repository
git clone https://github.com/yourusername/ferrum.git
cd ferrum

# Build and install
cargo install --path .
```

### Pre-built Binaries
Coming soon...

## Quick Start

### Running a Script
```bash
ferrum run main.js
```

### REPL Mode
```bash
ferrum repl
> 1 + 1
2
> console.log("Hello")
Hello
```

### With Permissions
```bash
ferrum run --allow-read --allow-net script.js
```

## Usage Examples

### Hello World
```javascript
// hello.js
console.log("Hello, Ferrum!");
```

Run with:
```bash
ferrum run hello.js
```

### File Operations
```javascript
// files.js
const data = "Hello, Ferrum!";
await Deno.writeTextFile("./output.txt", data);

const content = await Deno.readTextFile("./output.txt");
console.log(content);
```

Run with:
```bash
ferrum run --allow-read --allow-write files.js
```

### DNS Lookup
```javascript
// dns.js
// Note: DNS operations require --allow-net permission
const ips = await Deno.resolveDns("example.com");
console.log(ips);
```

Run with:
```bash
ferrum run --allow-net dns.js
```

## Permission System

Ferrum provides a secure permission system. By default, scripts run with NO access to:

- File system
- Network
- Environment variables
- Subprocesses

### Grant Permissions

```bash
# Allow all (use with caution)
ferrum run --allow-all script.js

# Allow specific permissions
ferrum run --allow-read --allow-net script.js

# Allow specific paths
ferrum run --allow-read-path=/tmp --allow-write-path=/tmp script.js

# Allow specific network domains
ferrum run --allow-net-domain=github.com,api.github.com script.js

# Allow environment variable access
ferrum run --allow-env script.js

# Allow subprocess execution
ferrum run --allow-run script.js
```

## Architecture

Ferrum is built with several key components:

```
┌─────────────────────────────────────────────────────────┐
│                      CLI Layer                          │
│  (arg parsing, permission management, REPL)             │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                   JavaScript Runtime                    │
│  (module loading, execution, inspector)                 │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                      V8 Engine                          │
│  (JavaScript execution, JIT compilation, garbage         │
│   collection)                                           │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                    Op Layer (Ops)                       │
│  (File I/O, Network, Timers, etc.)                      │
└─────────────────────────────────────────────────────────┘
```

### Key Technologies

- **Rust**: Core runtime implementation
- **V8**: JavaScript execution engine
- **Tokio**: Async runtime
- **Clap**: Command-line argument parsing
- **Tracing**: Structured logging

## Project Structure

```
ferrum/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library entry point
│   ├── cli.rs               # Command-line argument parsing
│   ├── runtime.rs           # JavaScript runtime setup
│   ├── module_loader.rs     # Module resolution and loading
│   ├── permissions.rs       # Permission system
│   ├── repl.rs              # REPL implementation
│   ├── ops/                 # Native operations
│   │   ├── mod.rs
│   │   ├── fs.rs           # File system operations
│   │   ├── net.rs          # Network operations
│   │   └── timers.rs       # Timer operations
│   └── js/                  # Built-in JavaScript files
│       └── core.js         # Core utilities (pending integration)
├── tests/                   # Integration tests
├── examples/                # Example scripts
└── Cargo.toml
```

## Current Limitations

This is an alpha release. The following features are **not yet implemented**:

### Network
- **HTTP/HTTPS fetch** - API structure exists, needs HTTP client integration (reqwest/hyper)
- **WebSocket** - Designed but not implemented
- **TCP connections** - Designed but not implemented

### Timers
- **setInterval** - Timer infrastructure works, but callback execution needs proper `FnMut` handling

### File System
- **File watching** - Placeholder only, needs `notify` crate integration

### JavaScript Integration
- **V8-Rust bridging** - Native operations not yet exposed to JavaScript
- **Core JavaScript API** - `js/core.js` references unimplemented native functions

### TypeScript
- **TypeScript support** - Planned for Phase 4
- **Source maps** - Planned for Phase 3

### Developer Tools
- **Test runner** - CLI exists, needs JavaScript test framework integration
- **Formatter** - Basic structure, needs implementation
- **Debugger** - Inspector infrastructure exists, needs protocol implementation

## Roadmap

### Phase 1: Core Runtime (MVP) - 85% Complete
- [x] Basic V8 integration
- [x] Module loading (ESM)
- [x] Permission system
- [x] File system operations (file watching pending)
- [x] Basic REPL
- [x] DNS resolution

### Phase 2: Web APIs - 20% Complete
- [ ] Fetch API (HTTP client) - API designed, needs implementation
- [ ] WebSocket - API designed, needs implementation
- [ ] Text encoding/decoding
- [ ] URL/URLSearchParams
- [ ] HTTP Server

### Phase 3: Developer Tools - 30% Complete
- [x] Test runner CLI - needs JavaScript integration
- [ ] Code formatter - structure only
- [ ] Linter
- [ ] Source map support
- [ ] Debugger integration

### Phase 4: Advanced Features
- [ ] TypeScript compiler integration
- [ ] Package management
- [ ] Worker threads
- [ ] Plugin system
- [ ] Snapshot-based startup

## Comparison

| Feature | Ferrum | Deno | Node.js |
|---------|--------|------|---------|
| Language | Rust | Rust | C++ |
| TypeScript | Planned | Native | Requires compilation |
| Security | Permissions | Permissions | No built-in security |
| ESM | Default | Default | Opt-in |
| Centralized Package | No | No | npm |
| Single Binary | Yes | Yes | No |

## Contributing

Contributions are welcome! This is an early-stage project and there's plenty to work on.

### Priority Areas

1. **HTTP Client Integration** - Integrate reqwest or hyper for fetch API
2. **V8-Rust Bridge** - Expose native operations to JavaScript
3. **setInterval Fix** - Proper FnMut callback handling
4. **File Watching** - Integrate notify crate
5. **Tests** - Add more integration tests

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines (coming soon).

### Development

```bash
# Clone the repository
git clone https://github.com/yourusername/ferrum.git
cd ferrum

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- run script.js

# Format code
cargo fmt

# Lint code
cargo clippy

# Build release version
cargo build --release
```

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Inspired by [Deno](https://deno.land)
- Built with [V8](https://v8.dev)
- Uses [Rust](https://www.rust-lang.org)

## Name

Ferrum is Latin for "iron", representing strength and reliability as a runtime foundation.
