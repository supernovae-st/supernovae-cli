//! Interop module for delegating commands to external binaries.
//!
//! This module handles:
//! - Binary discovery (nika, novanet in PATH or ~/.spn/bin/)
//! - Command execution with proper argument forwarding
//! - MCP registry for package metadata
//! - Model registry for package metadata
//! - Ecosystem tool detection and auto-install
//! - Error handling for missing binaries

pub mod binary;
pub mod detect;
pub mod mcp_registry;
pub mod model_registry;
pub mod npm;
pub mod skills;

#[allow(unused_imports)]
pub use detect::{EcosystemTools, InstallError, InstallMethod, InstallStatus};
