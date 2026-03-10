//! MCP (Model Context Protocol) server management.
//!
//! This module provides the single source of truth for MCP server
//! configurations across the SuperNovae ecosystem.
//!
//! # File Locations
//!
//! - Global: `~/.spn/mcp.yaml` - shared by all projects
//! - Project: `.spn/mcp.yaml` - project-specific overrides
//!
//! # Inheritance Model
//!
//! ```text
//! LEVEL 1: GLOBAL (~/.spn/mcp.yaml)
//!     │
//!     │ inherits
//!     ▼
//! LEVEL 2: PROJECT (.spn/mcp.yaml)
//!     │
//!     │ inherits
//!     ▼
//! LEVEL 3: WORKFLOW (workflow.nika.yaml)
//! ```
//!
//! # Usage
//!
//! ```text
//! use supernovae_cli::mcp::{config_manager, McpScope, McpServer};
//!
//! // Add a server
//! let manager = config_manager();
//! let server = McpServer::new("npx")
//!     .with_args(vec!["-y".into(), "@neo4j/mcp-server".into()]);
//! manager.add_server("neo4j", server, McpScope::Global)?;
//!
//! // List all servers (global + project merged)
//! let servers = manager.list_all_servers()?;
//! ```

mod config;
mod types;

#[allow(unused_imports)]
pub use config::{config_manager, find_project_root, McpConfigManager, McpScope};
#[allow(unused_imports)]
pub use types::{McpConfig, McpServer, McpSource, ProjectMcpConfig};
