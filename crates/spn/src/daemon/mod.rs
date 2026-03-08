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

pub mod agents;
mod error;
mod handler;
pub mod jobs;
pub mod mcp;
pub mod memory;
mod model_manager;
pub mod proactive;
mod secrets;
mod server;
pub mod service;
mod socket;
pub mod traces;

pub use error::DaemonError;
#[allow(unused_imports)]
pub use agents::{
    Agent, AgentConfig, AgentId, AgentManager, AgentRole, AgentState, AgentStatus, DelegatedTask,
};
#[allow(unused_imports)]
pub use jobs::{Job, JobId, JobScheduler, JobState, JobStatus, JobStore};
#[allow(unused_imports)]
pub use memory::{MemoryEntry, MemoryKey, MemoryNamespace, MemoryStore};
#[allow(unused_imports)]
pub use proactive::{
    ContextTrigger, ProactiveSuggestion, SuggestionAnalyzer, SuggestionCategory, SuggestionId,
    SuggestionPriority, SuggestionSource, TriggerCondition,
};
#[allow(unused_imports)]
pub use traces::{ReasoningTrace, TraceId, TraceMetadata, TraceStep, TraceStepKind, TraceStore};
pub use model_manager::ModelManager;
pub use secrets::SecretManager;
pub use server::{DaemonConfig, DaemonServer};
pub use service::{ServiceError, ServiceManager};
// Reserved for future service management API
#[allow(unused_imports)]
pub use service::{ServiceManagerType, ServiceStatus};

/// Default daemon configuration paths.
///
/// Uses [`spn_client::SpnPaths`] as the single source of truth.
pub mod paths {
    use crate::error::{Result, SpnError};
    use spn_client::SpnPaths;
    use std::path::PathBuf;

    /// Helper to convert PathError to SpnError.
    fn paths() -> Result<SpnPaths> {
        SpnPaths::new().map_err(|e| SpnError::ConfigError(e.to_string()))
    }

    /// Get the daemon socket path (~/.spn/daemon.sock)
    ///
    /// Returns error if HOME is unavailable.
    pub fn socket() -> Result<PathBuf> {
        paths().map(|p| p.socket_file())
    }

    /// Get the PID file path (~/.spn/daemon.pid)
    ///
    /// Returns error if HOME is unavailable.
    pub fn pid_file() -> Result<PathBuf> {
        paths().map(|p| p.pid_file())
    }

    /// Get the spn directory (~/.spn/)
    ///
    /// Returns error if HOME is unavailable.
    pub fn spn_dir() -> Result<PathBuf> {
        paths().map(|p| p.root().to_path_buf())
    }
}
