//! Status collectors for the unified dashboard.
//!
//! Provides comprehensive system status including:
//! - Local models (Ollama)
//! - Credentials (LLM providers + MCP services)
//! - MCP servers
//! - Daemon status

pub mod credentials;
pub mod daemon;
pub mod mcp;
pub mod ollama;
pub mod render;

use serde::Serialize;

/// Complete system status.
#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub ollama: ollama::OllamaStatus,
    pub credentials: Vec<credentials::CredentialStatus>,
    pub mcp_servers: Vec<mcp::McpServerStatus>,
    pub daemon: daemon::DaemonStatus,
}

impl SystemStatus {
    /// Collect all system status.
    pub async fn collect() -> Self {
        let (ollama, credentials, mcp_servers, daemon) = tokio::join!(
            ollama::collect(),
            credentials::collect(),
            mcp::collect(),
            daemon::collect(),
        );

        Self {
            ollama,
            credentials,
            mcp_servers,
            daemon,
        }
    }

    /// Summary counts for quick display.
    pub fn summary(&self) -> StatusSummary {
        let credentials_configured = self
            .credentials
            .iter()
            .filter(|c| c.status == credentials::Status::Ready)
            .count();
        let credentials_total = self.credentials.len();

        let mcp_active = self
            .mcp_servers
            .iter()
            .filter(|s| matches!(s.status, mcp::ServerStatus::Connected))
            .count();
        let mcp_total = self.mcp_servers.len();

        let models_count = self.ollama.models.len();

        StatusSummary {
            credentials_configured,
            credentials_total,
            mcp_active,
            mcp_total,
            models_count,
            daemon_running: self.daemon.running,
        }
    }
}

/// Quick summary for footer display.
#[derive(Debug, Clone)]
pub struct StatusSummary {
    pub credentials_configured: usize,
    pub credentials_total: usize,
    pub mcp_active: usize,
    pub mcp_total: usize,
    pub models_count: usize,
    pub daemon_running: bool,
}
