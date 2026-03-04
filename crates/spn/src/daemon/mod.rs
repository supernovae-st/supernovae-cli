//! # spn Daemon
//!
//! The daemon provides a centralized service for:
//! - Secret management (single keychain accessor)
//! - Process management (MCP servers, Ollama)
//! - MCP gateway (future)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                          spn daemon                                      │
//! │ ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
//! │ │   Secret    │  │   Process   │  │    MCP      │  │   Service   │     │
//! │ │   Manager   │  │   Manager   │  │   Gateway   │  │   Registry  │     │
//! │ └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘     │
//! │        │                │                │                │            │
//! │        └────────────────┴────────────────┴────────────────┘            │
//! │                                  │                                      │
//! │                          Unix Socket IPC                                │
//! │                       (~/.spn/daemon.sock)                              │
//! └──────────────────────────────────┬──────────────────────────────────────┘
//!                                    │
//!           ┌────────────────────────┼────────────────────────┐
//!           │                        │                        │
//!      ┌────▼────┐              ┌────▼────┐             ┌────▼────┐
//!      │  Nika   │              │ Claude  │             │   MCP   │
//!      │         │              │  Code   │             │ Servers │
//!      └─────────┘              └─────────┘             └─────────┘
//! ```
//!
//! ## Security
//!
//! - Socket permissions: 0600 (owner only)
//! - SO_PEERCRED validation on all connections
//! - Secrets cached in mlock'd memory
//! - PID file with flock for single instance

mod error;
mod handler;
mod secrets;
mod server;
mod socket;

pub use error::DaemonError;
pub use secrets::SecretManager;
pub use server::{DaemonConfig, DaemonServer};

/// Default daemon configuration paths
pub mod paths {
    use std::path::PathBuf;

    /// Get the daemon socket path (~/.spn/daemon.sock)
    pub fn socket() -> PathBuf {
        spn_client::default_socket_path()
    }

    /// Get the PID file path (~/.spn/daemon.pid)
    pub fn pid_file() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".spn").join("daemon.pid"))
            .unwrap_or_else(|| PathBuf::from("/tmp/spn-daemon.pid"))
    }

    /// Get the spn directory (~/.spn/)
    pub fn spn_dir() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".spn"))
            .unwrap_or_else(|| PathBuf::from("/tmp/spn"))
    }
}
