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
pub mod autonomy;
#[allow(dead_code)] // Phase 1 foundation - used in Phase 2: WatcherService
mod differ;
mod error;
#[allow(dead_code)] // Phase 1 foundation - used in Phase 2: WatcherService
mod foreign;
mod handler;
pub mod jobs;
pub mod mcp;
pub mod memory;
mod model_manager;
#[allow(dead_code)] // Phase 1 foundation - used in Phase 2: WatcherService
mod notifications;
pub mod proactive;
mod recent;
mod secrets;
mod server;
pub mod service;
mod socket;
pub mod traces;
// pub mod watcher;  // Phase 2

#[allow(unused_imports)]
pub use agents::{
    Agent, AgentConfig, AgentId, AgentManager, AgentRole, AgentState, AgentStatus, DelegatedTask,
};
#[allow(unused_imports)]
pub use autonomy::{
    ApprovalLevel, AutonomousTask, AutonomyLevel, AutonomyOrchestrator, AutonomyPolicy, Decision,
    DecisionOutcome, OrchestratorConfig, OrchestratorState, OrchestratorStats, PolicyViolation,
    TaskResult, TaskSource, TaskStatus,
};
pub use error::DaemonError;
#[allow(unused_imports)]
pub use jobs::{Job, JobId, JobScheduler, JobState, JobStatus, JobStore};
#[allow(unused_imports)]
pub use memory::{MemoryEntry, MemoryKey, MemoryNamespace, MemoryStore};
pub use model_manager::ModelManager;
#[allow(unused_imports)]
pub use proactive::{
    ContextTrigger, ProactiveSuggestion, SuggestionAnalyzer, SuggestionCategory, SuggestionId,
    SuggestionPriority, SuggestionSource, TriggerCondition,
};
pub use secrets::SecretManager;
pub use server::{DaemonConfig, DaemonServer};
pub use service::{ServiceError, ServiceManager};
#[allow(unused_imports)]
pub use traces::{ReasoningTrace, TraceId, TraceMetadata, TraceStep, TraceStepKind, TraceStore};
// Reserved for future service management API
#[allow(unused_imports)]
pub use service::{ServiceManagerType, ServiceStatus};

// MCP Auto-Sync (Phase 1)
#[allow(unused_imports)]
pub use differ::{diff_mcp_configs, parse_client_config, McpDiff};
#[allow(unused_imports)]
pub use foreign::{ForeignMcp, ForeignMcpServer, ForeignScope, ForeignSource, ForeignTracker};
#[allow(unused_imports)]
pub use notifications::NotificationService;
#[allow(unused_imports)]
pub use recent::{RecentProject, RecentProjects};

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
