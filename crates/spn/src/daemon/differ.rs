//! Compare MCP configs to detect foreign MCPs.
//!
//! This module provides utilities to compare spn's MCP configuration with
//! client configurations (Cursor, Claude Code, Windsurf) to detect:
//! - Foreign MCPs: Present in client but not in spn
//! - Missing MCPs: Present in spn but not synced to client
//! - Synced MCPs: Present in both

use rustc_hash::FxHashMap;
use serde::Deserialize;
use spn_core::McpServer;
use std::path::Path;

use crate::error::{Result, SpnError};

/// Result of comparing two MCP configurations.
#[derive(Debug, Default)]
pub struct McpDiff {
    /// MCPs in client but not in spn (foreign).
    pub foreign: Vec<(String, McpServer)>,
    /// MCPs in spn but not in client (need sync).
    pub missing: Vec<String>,
    /// MCPs present in both (synced).
    pub synced: Vec<String>,
}

impl McpDiff {
    /// Check if there are any foreign MCPs.
    #[must_use]
    #[allow(dead_code)] // Reserved for future daemon status API
    pub fn has_foreign(&self) -> bool {
        !self.foreign.is_empty()
    }

    /// Check if there are any missing MCPs.
    #[must_use]
    #[allow(dead_code)] // Reserved for future daemon status API
    pub fn has_missing(&self) -> bool {
        !self.missing.is_empty()
    }

    /// Check if all MCPs are synced (no foreign, no missing).
    #[must_use]
    #[allow(dead_code)] // Reserved for future daemon status API
    pub fn is_synced(&self) -> bool {
        self.foreign.is_empty() && self.missing.is_empty()
    }
}

/// Compare spn's MCPs with a client's MCPs.
///
/// # Arguments
/// * `spn_servers` - List of (name, server) pairs from spn config
/// * `client_servers` - List of (name, server) pairs from client config
///
/// # Returns
/// A `McpDiff` showing what's different between the two configs.
#[must_use]
pub fn diff_mcp_configs(
    spn_servers: &[(String, McpServer)],
    client_servers: &[(String, McpServer)],
) -> McpDiff {
    let spn_names: rustc_hash::FxHashSet<_> = spn_servers.iter().map(|(n, _)| n.as_str()).collect();
    let client_names: rustc_hash::FxHashSet<_> =
        client_servers.iter().map(|(n, _)| n.as_str()).collect();

    let mut diff = McpDiff::default();

    // Foreign: in client but not in spn
    for (name, server) in client_servers {
        if !spn_names.contains(name.as_str()) {
            diff.foreign.push((name.clone(), server.clone()));
        }
    }

    // Missing: in spn but not in client
    for (name, _) in spn_servers {
        if !client_names.contains(name.as_str()) {
            diff.missing.push(name.clone());
        }
    }

    // Synced: in both
    for (name, _) in spn_servers {
        if client_names.contains(name.as_str()) {
            diff.synced.push(name.clone());
        }
    }

    diff
}

/// JSON structure for MCP server in client configs.
///
/// Common format used by Cursor, Claude Code, and Windsurf.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonMcpServer {
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: FxHashMap<String, String>,
    url: Option<String>,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Root structure for Cursor/Windsurf MCP config.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpJsonConfig {
    #[serde(default)]
    mcp_servers: FxHashMap<String, JsonMcpServer>,
    // Cursor uses "mcpServers" but we also check for "servers"
    #[serde(default)]
    servers: FxHashMap<String, JsonMcpServer>,
}

/// Root structure for Claude Code config (~/.claude.json or .mcp.json).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Reserved for Claude Code-specific parsing
struct ClaudeJsonConfig {
    #[serde(default)]
    mcp_servers: FxHashMap<String, JsonMcpServer>,
}

/// Parse a client MCP config file and extract servers (async version).
///
/// Supports:
/// - Cursor format: `{ "mcpServers": { ... } }`
/// - Claude Code format: `{ "mcpServers": { ... } }`
/// - Windsurf format: `{ "mcpServers": { ... } }`
///
/// # Errors
/// Returns error if file cannot be read or parsed.
pub async fn parse_client_config(path: &Path) -> Result<Vec<(String, McpServer)>> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| SpnError::ConfigError(format!("Failed to read {}: {e}", path.display())))?;

    parse_mcp_json(&content)
}

/// Parse MCP JSON content.
fn parse_mcp_json(content: &str) -> Result<Vec<(String, McpServer)>> {
    // Try parsing as generic MCP config
    let config: McpJsonConfig = serde_json::from_str(content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to parse MCP JSON: {e}")))?;

    // Combine mcpServers and servers (mcpServers takes priority)
    let mut all_servers = config.servers;
    all_servers.extend(config.mcp_servers);

    let mut result = Vec::new();
    for (name, json_server) in all_servers {
        let server = json_to_mcp_server(&name, &json_server);
        result.push((name, server));
    }

    Ok(result)
}

/// Convert JSON server config to McpServer.
fn json_to_mcp_server(name: &str, json: &JsonMcpServer) -> McpServer {
    let mut server = if let Some(url) = &json.url {
        McpServer::sse(name, url)
    } else {
        McpServer::stdio(
            name,
            json.command.as_deref().unwrap_or(""),
            json.args.iter().map(String::as_str).collect(),
        )
    };

    // Add environment variables
    for (key, value) in &json.env {
        server = server.with_env(key, value);
    }

    server.with_enabled(json.enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_finds_foreign() {
        let spn = vec![(
            "neo4j".to_string(),
            McpServer::stdio("neo4j", "npx", vec![]),
        )];
        let client = vec![
            (
                "neo4j".to_string(),
                McpServer::stdio("neo4j", "npx", vec![]),
            ),
            (
                "slack".to_string(),
                McpServer::stdio("slack", "npx", vec![]),
            ),
        ];

        let diff = diff_mcp_configs(&spn, &client);

        assert_eq!(diff.foreign.len(), 1);
        assert_eq!(diff.foreign[0].0, "slack");
    }

    #[test]
    fn test_diff_finds_missing() {
        let spn = vec![
            (
                "neo4j".to_string(),
                McpServer::stdio("neo4j", "npx", vec![]),
            ),
            (
                "github".to_string(),
                McpServer::stdio("github", "npx", vec![]),
            ),
        ];
        let client = vec![(
            "neo4j".to_string(),
            McpServer::stdio("neo4j", "npx", vec![]),
        )];

        let diff = diff_mcp_configs(&spn, &client);

        assert_eq!(diff.missing.len(), 1);
        assert_eq!(diff.missing[0], "github");
    }

    #[test]
    fn test_diff_identifies_synced() {
        let spn = vec![
            (
                "neo4j".to_string(),
                McpServer::stdio("neo4j", "npx", vec![]),
            ),
            (
                "github".to_string(),
                McpServer::stdio("github", "npx", vec![]),
            ),
        ];
        let client = vec![
            (
                "neo4j".to_string(),
                McpServer::stdio("neo4j", "npx", vec![]),
            ),
            (
                "github".to_string(),
                McpServer::stdio("github", "npx", vec![]),
            ),
        ];

        let diff = diff_mcp_configs(&spn, &client);

        assert!(diff.is_synced());
        assert_eq!(diff.synced.len(), 2);
    }

    #[test]
    fn test_parse_cursor_format() {
        let json = r#"{
            "mcpServers": {
                "neo4j": {
                    "command": "npx",
                    "args": ["-y", "@neo4j/mcp-neo4j"],
                    "env": {
                        "NEO4J_URI": "bolt://localhost:7687"
                    }
                }
            }
        }"#;

        let servers = parse_mcp_json(json).unwrap();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].0, "neo4j");
        assert_eq!(servers[0].1.command, Some("npx".to_string()));
        assert_eq!(servers[0].1.args, vec!["-y", "@neo4j/mcp-neo4j"]);
    }

    #[test]
    fn test_parse_with_url() {
        let json = r#"{
            "mcpServers": {
                "remote": {
                    "url": "http://localhost:3000/mcp"
                }
            }
        }"#;

        let servers = parse_mcp_json(json).unwrap();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].1.url, Some("http://localhost:3000/mcp".to_string()));
    }

    #[test]
    fn test_parse_disabled_server() {
        let json = r#"{
            "mcpServers": {
                "disabled": {
                    "command": "npx",
                    "args": [],
                    "enabled": false
                }
            }
        }"#;

        let servers = parse_mcp_json(json).unwrap();

        assert!(!servers[0].1.enabled);
    }
}
