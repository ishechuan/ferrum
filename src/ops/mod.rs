//! Native Operations (Ops)
//!
//! This module contains native operations that can be called from JavaScript.

pub mod async_bindings;
pub mod async_ops;
pub mod bindings;
pub mod dispatch;
pub mod fs;
pub mod net;
pub mod text_encoding;
pub mod text_encoding_bindings;
pub mod timer_bindings;
pub mod timers;
pub mod url;
pub mod url_bindings;

// Re-export common types (avoid conflicts by not re-exporting async_bindings console ops)
pub use async_ops::*;
pub use bindings::*;
pub use dispatch::*;
pub use fs::*;
pub use net::*;
pub use text_encoding::*;
pub use timers::*;
pub use url::*;
pub use url_bindings::*;

// Selectively re-export async_bindings
pub use async_bindings::{
    bootstrap_async_globals, clear_async_context, op_async_copy, op_async_exists, op_async_mkdir,
    op_async_read_dir, op_async_read_text_file, op_async_remove, op_async_rename, op_async_sleep,
    op_async_stat, op_async_write_file, op_async_write_text_file, set_async_context,
};

// Also export op_import_module from bindings
pub use bindings::op_import_module;

// Also export server operations
pub use bindings::op_serve;
pub use bindings::op_server_addr;
pub use bindings::op_server_close;
pub use bindings::op_server_listening;

// Export WebSocket operations
pub use bindings::op_connect_websocket;
pub use net::{WebSocketConnection, WebSocketMessage, WebSocketOptions, WebSocketReadyState};

// Export HTTP server types
pub use net::{serve, serve_json, HttpServer, Request, Response, ServerOptions, ServerState};
