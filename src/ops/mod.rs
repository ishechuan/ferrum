//! Native Operations (Ops)
//!
//! This module contains native operations that can be called from JavaScript.

pub mod fs;
pub mod net;
pub mod timers;

// Re-export common types
pub use fs::*;
pub use net::*;
pub use timers::*;
