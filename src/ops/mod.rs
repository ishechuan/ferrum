//! Native Operations (Ops)
//!
//! This module contains native operations that can be called from JavaScript.

pub mod async_bindings;
pub mod async_ops;
pub mod bindings;
pub mod dispatch;
pub mod fs;
pub mod net;
pub mod timers;

// Re-export common types (avoid conflicts by not re-exporting async_bindings console ops)
pub use async_ops::*;
pub use bindings::*;
pub use dispatch::*;
pub use fs::*;
pub use net::*;
pub use timers::*;

// Selectively re-export async_bindings
pub use async_bindings::{
    bootstrap_async_globals, clear_async_context, op_async_exists, op_async_mkdir,
    op_async_read_file, op_async_read_text_file, op_async_remove, op_async_sleep, op_async_stat,
    op_async_write_file, op_async_write_text_file, set_async_context,
};
