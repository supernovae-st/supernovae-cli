//! IDE synchronization module.
//!
//! Syncs installed SuperNovae packages to IDE-specific configurations.
//!
//! Supported IDEs:
//! - Claude Code (.claude/settings.json)
//! - Cursor (.cursor/mcp.json)
//! - VS Code (.vscode/settings.json)
//! - Windsurf (.windsurf/settings.json)

pub mod adapters;
pub mod config;
pub mod types;

pub use adapters::IdeAdapter;
pub use config::SyncConfig;
pub use types::{IdeTarget, PackageManifest, SyncResult};
