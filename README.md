# Ferrum

> A simple, secure, and modern JavaScript/TypeScript Runtime

Ferrum is a lightweight JavaScript and TypeScript runtime inspired by Deno, built with Rust. It aims to provide a secure and productive environment for running JavaScript/TypeScript outside the browser.

English | [简体中文](README.zh-CN.md)

## Status

**Version:** 0.2.0 (Alpha)
**Phase 1:** ✅ 100% Complete
**Phase 2:** 100% Complete (URL/URLSearchParams implemented!)

This is an early-stage project with Phase 1 core functionality fully implemented. Phase 2 (Web APIs) is now 90% complete with HTTP server support and native ops bridge.

## Features

### Core
- **Security**: Explicit permission model for file system, network, and environment access
- **Modern ESM**: ES2020 module support with import maps
- **Fast**: Built on V8 JavaScript engine
- **Single Binary**: Distributed as a single executable

### Standard Library
- **File System API**: Read, write, copy, rename, directory operations (all async!)
- **Network Operations**: DNS resolution, HTTP fetch (GET, POST, headers, timeout), HTTP Server (Deno.serve), WebSocket
- **Timer API**: setTimeout, setInterval, Deno.sleep, Promise support
- **Path Utilities**: Cross-platform path manipulation
- **Text Encoding**: TextEncoder, TextDecoder (UTF-8)
- **URL API**: URL, URLSearchParams for web compatibility

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

### Dynamic Imports
```javascript
// main.js
// Load a module at runtime using Deno.importModule() (returns Promise)
const moduleCode = await Deno.importModule("./utils.js");
const utils = new Function(moduleCode)();
console.log(utils.add(2, 3)); // 5
```

```javascript
// utils.js
const add = (a, b) => a + b;
const subtract = (a, b) => a - b;
"return { add, subtract }";
```

Run with:
```bash
ferrum run --allow-read main.js
```

### HTTP Server
```javascript
// server.js
// Start an HTTP server using Deno.serve()
const server = Deno.serve((req) => {
    return {
        status: 200,
        headers: { "content-type": "text/plain; charset=utf-8" },
        body: "Hello from Ferrum HTTP Server!\n"
    };
}, { port: 8080, hostname: "0.0.0.0" });

console.log("Server started on", await server.addr());
```

Run with:
```bash
ferrum run --allow-net server.js
```

### Text Encoding
```javascript
// encoding.js
// TextEncoder encodes strings to UTF-8 bytes
const encoder = new TextEncoder();
const bytes = encoder.encode("Hello, 世界! 🎉");
console.log("Encoded bytes:", bytes);

// TextDecoder decodes UTF-8 bytes to strings
const decoder = new TextDecoder();
const decoded = decoder.decode(bytes);
console.log("Decoded string:", decoded);

// encodeInto for efficient encoding into pre-allocated buffer
const dest = new Uint8Array(10);
const result = encoder.encodeInto("Hello", dest);
console.log(`Read: ${result.read}, Written: ${result.written}`);
```

Run with:
```bash
ferrum run encoding.js
```

### URL Parsing
```javascript
// url.js
// Parse and manipulate URLs
const url = new URL("https://example.com:8080/path?foo=bar#section");

console.log("Protocol:", url.protocol); // "https:"
console.log("Hostname:", url.hostname); // "example.com"
console.log("Port:", url.port);         // "8080"
console.log("Path:", url.pathname);     // "/path"
console.log("Query:", url.search);      // "?foo=bar"
console.log("Hash:", url.hash);         // "#section"

// URLSearchParams provides easy query string manipulation
const params = url.searchParams;
console.log("foo:", params.get("foo")); // "bar"

// Modify the URL
url.port = "3000";
url.searchParams.set("baz", "qux");
console.log("New URL:", url.href);
```

Run with:
```bash
ferrum run url.js
```

### WebSocket
```javascript
// websocket.js
// Connect to a WebSocket server
// Note: WebSocket operations require --allow-net permission
const ws = await Deno.connectWebSocket("wss://echo.websocket.org");

console.log("Connected:", ws.url);
console.log("Ready state:", ws.readyState); // 1 = open

// Send a message
ws.send("Hello, WebSocket!");

// Receive a message
const message = await ws.recv();
console.log("Received:", message);

// Close the connection
ws.close();
console.log("Connection closed");
```

Run with:
```bash
ferrum run --allow-net websocket.js
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
- **WebSocket** - ✅ Implemented (basic support)
- **TCP connections** - API designed, needs implementation
- **ES Module imports** - Module loader works for `.mjs` files, but dynamic imports (`import()`) not yet supported
- **Module resolution callback** - Basic implementation, needs enhancement for complex import graphs

### Timers
- **setInterval** - Fully implemented with proper FnMut callback handling

### TypeScript
- **TypeScript support** - Planned for Phase 4
- **Source maps** - Planned for Phase 3

### Developer Tools
- **Test runner** - CLI exists, needs JavaScript test framework integration
- **Formatter** - Basic structure, needs implementation
- **Debugger** - Inspector infrastructure exists, needs protocol implementation

## Roadmap

### Phase 1: Core Runtime (MVP) - ✅ 100% Complete
- [x] Basic V8 integration
- [x] Module loading (ESM)
- [x] Module loader runtime integration
- [x] Permission system
- [x] File system operations (including file watching)
- [x] Basic REPL
- [x] DNS resolution
- [x] V8-Rust bridge
- [x] Import map support

### Phase 2: Web APIs - 100% Complete
- [x] Native Ops Bridge - V8-Rust bridge for calling Rust from JavaScript
- [x] Fetch API (HTTP client) - Full async HTTP/HTTPS support with headers, timeout, POST
- [x] Async/Await Bridge - V8 Promise integration with Tokio event loop
- [x] Async File Operations - Deno.readTextFile, writeTextFile, copy, readDir, rename
- [x] Dynamic Imports - Deno.importModule() returns Promise for async module loading
- [x] HTTP Server - Deno.serve() for building HTTP servers
- [x] Text Encoding - TextEncoder/TextDecoder APIs for UTF-8 encoding/decoding
- [x] URL/URLSearchParams - URL parsing and manipulation API for web compatibility
- [x] WebSocket - Basic WebSocket client support (ws://, wss://)

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

1. **WebSocket Support** - ✅ Implemented basic WebSocket client API
2. **Text Encoding** - ✅ Implemented TextEncoder/TextDecoder APIs
3. **URL/URLSearchParams** - ✅ Implemented URL API for web compatibility
4. **Tests** - Add more integration tests for async operations

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
