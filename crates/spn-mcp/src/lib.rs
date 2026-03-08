//! spn-mcp: Dynamic REST-to-MCP wrapper library
//!
//! This crate provides the core functionality for wrapping REST APIs as MCP tools.
//!
//! # Architecture
//!
//! ```text
//! ~/.spn/apis/*.yaml → config::load_all_apis() → ApiConfig
//!                                                    ↓
//!                                            server::DynamicHandler
//!                                                    ↓
//!                                               MCP Server (stdio)
//! ```
//!
//! # Configuration Format
//!
//! API configurations are YAML files in `~/.spn/apis/`:
//!
//! ```yaml
//! name: example
//! base_url: https://api.example.com/v1
//! auth:
//!   type: bearer
//!   credential: example
//! tools:
//!   - name: get_data
//!     method: GET
//!     path: /data
//! ```

pub mod config;
pub mod error;
pub mod openapi;
pub mod server;

pub use config::{ApiConfig, AuthConfig, AuthType, ToolDef};
pub use error::{Error, Result};
pub use openapi::{parse_openapi, OpenApiError, OpenApiSpec};
