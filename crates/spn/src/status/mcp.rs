//! MCP servers status collector.
//!
//! Collects status of all configured MCP servers including:
//! - Connection status
//! - Transport type
//! - Associated credential

use serde::Serialize;

use crate::mcp::config_manager;

/// MCP server status.
#[derive(Debug, Clone, Serialize)]
pub struct McpServerStatus {
    /// Server name.
    pub name: String,
    /// Current status.
    pub status: ServerStatus,
    /// Transport type.
    pub transport: Transport,
    /// Associated credential name (if any).
    pub credential: Option<String>,
    /// Server command.
    pub command: String,
}

/// Server connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // Future use: Connected/Starting/Error when we implement live status checks
pub enum ServerStatus {
    /// Connected and responding.
    Connected,
    /// Starting up.
    Starting,
    /// Configured but not running.
    Ready,
    /// Disabled by user.
    Disabled,
    /// Error state.
    Error,
}

impl ServerStatus {
    /// Icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            ServerStatus::Connected => "✅",
            ServerStatus::Starting => "⏳",
            ServerStatus::Ready => "○",
            ServerStatus::Disabled => "⏸️",
            ServerStatus::Error => "❌",
        }
    }

    /// Label for display.
    pub fn label(&self) -> &'static str {
        match self {
            ServerStatus::Connected => "connected",
            ServerStatus::Starting => "starting",
            ServerStatus::Ready => "ready",
            ServerStatus::Disabled => "disabled",
            ServerStatus::Error => "error",
        }
    }
}

/// Transport type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // Future use: Websocket when MCP supports it
pub enum Transport {
    /// Standard I/O.
    Stdio,
    /// HTTP/SSE.
    Http,
    /// WebSocket.
    Websocket,
}

/// Map server name to likely credential name.
fn infer_credential(name: &str) -> Option<String> {
    match name {
        "neo4j" | "@neo4j/mcp-neo4j" => Some("neo4j".to_string()),
        "github" | "github-mcp" => Some("github".to_string()),
        "firecrawl" | "firecrawl-mcp" => Some("firecrawl".to_string()),
        "perplexity" | "perplexity-mcp" => Some("perplexity".to_string()),
        "slack" | "slack-mcp" => Some("slack".to_string()),
        "supadata" | "supadata-mcp" => Some("supadata".to_string()),
        // Servers that don't need credentials
        "context7" | "sequential-thinking" | "21st" | "novanet" => None,
        _ => None,
    }
}

/// Collect MCP server statuses.
pub async fn collect() -> Vec<McpServerStatus> {
    let mcp = config_manager();
    let servers = match mcp.list_all_servers() {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut statuses = Vec::new();

    for (name, server) in servers {
        let status = if !server.enabled {
            ServerStatus::Disabled
        } else {
            // For now, mark as Ready since we can't easily check if running
            // In a full implementation, we'd check if the process is running
            ServerStatus::Ready
        };

        // Determine transport (most MCP servers use stdio)
        let transport = if server.command.contains("http") || server.args.iter().any(|a| a.contains("http")) {
            Transport::Http
        } else {
            Transport::Stdio
        };

        let credential = infer_credential(&name);

        statuses.push(McpServerStatus {
            name,
            status,
            transport,
            credential,
            command: server.command,
        });
    }

    statuses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_credential() {
        assert_eq!(infer_credential("neo4j"), Some("neo4j".to_string()));
        assert_eq!(infer_credential("github"), Some("github".to_string()));
        assert_eq!(infer_credential("context7"), None);
    }

    #[test]
    fn test_server_status_icons() {
        assert_eq!(ServerStatus::Connected.icon(), "✅");
        assert_eq!(ServerStatus::Disabled.icon(), "⏸️");
    }
}
