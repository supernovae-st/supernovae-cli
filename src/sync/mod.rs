//! IDE synchronization module.
//!
//! Syncs installed SuperNovae packages to IDE-specific configurations.
//!
//! Supported IDEs:
//! - Claude Code (.claude/settings.json)
//! - Cursor (.cursor/mcp.json)
//! - VS Code (.vscode/settings.json)
//! - Windsurf (.windsurf/settings.json)
//!
//! # MCP Sync
//!
//! The `mcp_sync` module provides sync from `~/.spn/mcp.yaml` (single source)
//! to editor-specific configuration files.

pub mod adapters;
pub mod config;
pub mod mcp_sync;
pub mod types;
