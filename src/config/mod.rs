//! Configuration management with three-level scope hierarchy.
//!
//! # Scope Hierarchy
//!
//! SuperNovae uses a three-level scope system (innermost wins):
//!
//! ```text
//! Local > Team > Global
//! ```
//!
//! - **Global** (`~/.spn/config.toml`): User-wide preferences
//! - **Team** (`./mcp.yaml`): Team/project configuration (committed to git)
//! - **Local** (`./.spn/local.yaml`): Local overrides (gitignored)
//!
//! # Example
//!
//! ```rust
//! use spn::config::ConfigResolver;
//!
//! let config = ConfigResolver::load()?;
//! let model = config.get_provider_model("anthropic");
//! ```

pub mod global;
pub mod local;
pub mod resolver;
pub mod scope;
pub mod team;
pub mod types;

pub use resolver::ConfigResolver;
pub use scope::{ConfigScope, ScopeType};
pub use types::{Config, McpServerConfig, ProviderConfig, SyncConfig};
