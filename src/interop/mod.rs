//! Interop module for delegating commands to external binaries.
//!
//! This module handles:
//! - Binary discovery (nika, novanet in PATH or ~/.spn/bin/)
//! - Command execution with proper argument forwarding
//! - Error handling for missing binaries

pub mod binary;
pub mod npm;
pub mod skills;
