//! MCP servers status collector.
//!
//! Collects status of all configured MCP servers including:
//! - Connection status
//! - Transport type
//! - Associated credential
//! - Client sync status

use serde::Serialize;

use crate::mcp::config_manager;

/// MCP server status.
#[derive(Debug, Clone, Serialize)]
pub struct McpServerStatus {
    /// Server name.
    pub name: String,
    /// Server emoji for display.
    pub emoji: &'static str,
    /// Current status.
    pub status: ServerStatus,
    /// Transport type.
    pub transport: Transport,
    /// Associated credential name (if any).
    pub credential: Option<String>,
    /// Server command.
    pub command: String,
    /// Sync status across clients.
    pub client_sync: ClientSyncStatus,
}

/// Sync status for each supported client.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ClientSyncStatus {
    /// Claude Code sync state.
    pub claude_code: SyncState,
    /// Cursor sync state.
    pub cursor: SyncState,
    /// Windsurf sync state.
    pub windsurf: SyncState,
    /// Nika sync state.
    pub nika: SyncState,
}

/// Sync state for a single client.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncState {
    /// Server is synced to this client.
    Synced,
    /// Sync pending (spn has it, client doesn't).
    #[default]
    Pending,
    /// Client is disabled/not available.
    Disabled,
}

impl SyncState {
    /// Icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            SyncState::Synced => "●",
            SyncState::Pending => "○",
            SyncState::Disabled => "⊘",
        }
    }
}

/// Get emoji for an MCP server by name.
pub fn mcp_emoji(name: &str) -> &'static str {
    match name {
        "neo4j" | "@neo4j/mcp-neo4j" => "🔷",
        "github" | "github-mcp" => "🐙",
        "slack" | "slack-mcp" => "💬",
        "perplexity" | "perplexity-mcp" => "🔮",
        "firecrawl" | "firecrawl-mcp" => "🔥",
        "supadata" | "supadata-mcp" => "📺",
        "dataforseo" => "📊",
        "ahrefs" | "ahrefs-mcp" => "🔗",
        "context7" => "📚",
        "novanet" | "novanet-mcp" => "🌐",
        "sequential-thinking" => "🧠",
        "21st" | "magic" => "🎨",
        "spn-mcp" => "⚡",
        "postgres" | "postgresql" => "🐘",
        "sqlite" => "🗃️",
        "redis" => "🔴",
        "elasticsearch" => "🔍",
        "mongodb" => "🍃",
        _ => "🔌",
    }
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

    // Check client sync status
    let sync_status = check_client_sync(&servers);

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
        let transport =
            if server.command.contains("http") || server.args.iter().any(|a| a.contains("http")) {
                Transport::Http
            } else {
                Transport::Stdio
            };

        let credential = infer_credential(&name);
        let emoji = mcp_emoji(&name);

        // Get client sync for this server
        let client_sync = sync_status.get(&name).cloned().unwrap_or_default();

        statuses.push(McpServerStatus {
            name,
            emoji,
            status,
            transport,
            credential,
            command: server.command,
            client_sync,
        });
    }

    statuses
}

/// Check which clients have each MCP server synced.
fn check_client_sync(
    spn_servers: &[(String, crate::mcp::McpServer)],
) -> rustc_hash::FxHashMap<String, ClientSyncStatus> {
    use crate::sync::adapters::{ClaudeCodeAdapter, CursorAdapter, IdeAdapter, WindsurfAdapter};
    use rustc_hash::FxHashMap;
    use std::path::PathBuf;

    let mut result: FxHashMap<String, ClientSyncStatus> = FxHashMap::default();

    // Initialize all servers with default (pending) status
    for (name, _) in spn_servers {
        result.insert(name.clone(), ClientSyncStatus::default());
    }

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    // Check Claude Code
    let claude = ClaudeCodeAdapter;
    if claude.is_available(&home) {
        if let Ok(content) = std::fs::read_to_string(claude.config_path(&home)) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                    for (name, sync) in result.iter_mut() {
                        if servers.contains_key(name) {
                            sync.claude_code = SyncState::Synced;
                        }
                    }
                }
            }
        }
    } else {
        for sync in result.values_mut() {
            sync.claude_code = SyncState::Disabled;
        }
    }

    // Check Cursor
    let cursor = CursorAdapter;
    if cursor.is_available(&home) {
        if let Ok(content) = std::fs::read_to_string(cursor.config_path(&home)) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                    for (name, sync) in result.iter_mut() {
                        if servers.contains_key(name) {
                            sync.cursor = SyncState::Synced;
                        }
                    }
                }
            }
        }
    } else {
        for sync in result.values_mut() {
            sync.cursor = SyncState::Disabled;
        }
    }

    // Check Windsurf
    let windsurf = WindsurfAdapter;
    if windsurf.is_available(&home) {
        if let Ok(content) = std::fs::read_to_string(windsurf.config_path(&home)) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                    for (name, sync) in result.iter_mut() {
                        if servers.contains_key(name) {
                            sync.windsurf = SyncState::Synced;
                        }
                    }
                }
            }
        }
    } else {
        for sync in result.values_mut() {
            sync.windsurf = SyncState::Disabled;
        }
    }

    // Nika: Check ~/.spn/mcp.yaml (always synced if in spn config)
    for sync in result.values_mut() {
        sync.nika = SyncState::Synced;
    }

    result
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

    #[test]
    fn test_mcp_emoji() {
        assert_eq!(mcp_emoji("neo4j"), "🔷");
        assert_eq!(mcp_emoji("github"), "🐙");
        assert_eq!(mcp_emoji("perplexity"), "🔮");
        assert_eq!(mcp_emoji("unknown-server"), "🔌");
    }

    #[test]
    fn test_sync_state_icons() {
        assert_eq!(SyncState::Synced.icon(), "●");
        assert_eq!(SyncState::Pending.icon(), "○");
        assert_eq!(SyncState::Disabled.icon(), "⊘");
    }

    #[test]
    fn test_client_sync_default() {
        let sync = ClientSyncStatus::default();
        assert_eq!(sync.claude_code, SyncState::Pending);
        assert_eq!(sync.cursor, SyncState::Pending);
        assert_eq!(sync.windsurf, SyncState::Pending);
        assert_eq!(sync.nika, SyncState::Pending);
    }

    #[test]
    fn test_mcp_emoji_aliases() {
        // Test alternative names map to same emoji
        assert_eq!(mcp_emoji("@neo4j/mcp-neo4j"), "🔷");
        assert_eq!(mcp_emoji("github-mcp"), "🐙");
        assert_eq!(mcp_emoji("slack-mcp"), "💬");
        assert_eq!(mcp_emoji("firecrawl-mcp"), "🔥");
    }

    #[test]
    fn test_server_status_labels() {
        assert_eq!(ServerStatus::Connected.label(), "connected");
        assert_eq!(ServerStatus::Ready.label(), "ready");
        assert_eq!(ServerStatus::Disabled.label(), "disabled");
        assert_eq!(ServerStatus::Error.label(), "error");
        assert_eq!(ServerStatus::Starting.label(), "starting");
    }

    #[test]
    fn test_transport_serialization() {
        // Test that transport serializes correctly
        let stdio = Transport::Stdio;
        let json = serde_json::to_string(&stdio).unwrap();
        assert_eq!(json, "\"stdio\"");

        let http = Transport::Http;
        let json = serde_json::to_string(&http).unwrap();
        assert_eq!(json, "\"http\"");
    }
}
