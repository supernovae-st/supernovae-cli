//! Interop module for delegating commands to external binaries.
//!
//! This module handles:
//! - Binary discovery (nika, novanet in PATH or ~/.spn/bin/)
//! - Command execution with proper argument forwarding
//! - MCP registry for package metadata
//! - Model registry for package metadata
//! - Error handling for missing binaries

pub mod binary;
pub mod mcp_registry;
pub mod model_registry;
pub mod npm;
pub mod skills;
