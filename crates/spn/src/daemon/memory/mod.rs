//! Memory system for cross-session context persistence.
//!
//! Stores and retrieves context information to enable smarter
//! suggestions and continuity across CLI invocations.

#![allow(dead_code)]

mod store;
mod types;

pub use store::MemoryStore;
pub use types::{MemoryEntry, MemoryKey, MemoryNamespace};
