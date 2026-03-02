//! MCP configuration sync module.
//!
//! Syncs MCP servers from ~/.spn/mcp.yaml to editor configurations.
//! This is the single source of truth for MCP servers.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::Result;
use crate::mcp::{config_manager, McpConfig, McpServer};
use super::types::IdeTarget;

/// Result of syncing MCP to an IDE.
#[derive(Debug)]
pub struct McpSyncResult {
    /// Target IDE.
    pub target: IdeTarget,
    /// Number of servers synced.
    pub servers_synced: usize,
    /// Whether the sync was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Path where config was written.
    pub config_path: PathBuf,
}

/// Sync MCP servers to all enabled IDEs.
pub fn sync_mcp_to_editors(
    targets: &[IdeTarget],
    project_root: Option<&Path>,
) -> Vec<McpSyncResult> {
    let mcp_manager = config_manager();

    // Load resolved MCP config (global + project merged)
    let mcp_config = match mcp_manager.load_resolved() {
        Ok(config) => config,
        Err(e) => {
            // Return error for all targets
            return targets
                .iter()
                .map(|target| McpSyncResult {
                    target: *target,
                    servers_synced: 0,
                    success: false,
                    error: Some(format!("Failed to load MCP config: {}", e)),
                    config_path: PathBuf::new(),
                })
                .collect();
        }
    };

    targets
        .iter()
        .map(|target| sync_mcp_to_target(*target, &mcp_config, project_root))
        .collect()
}

/// Sync MCP servers to a specific IDE target.
fn sync_mcp_to_target(
    target: IdeTarget,
    mcp_config: &McpConfig,
    project_root: Option<&Path>,
) -> McpSyncResult {
    match target {
        IdeTarget::ClaudeCode => sync_to_claude_code(mcp_config, project_root),
        IdeTarget::Cursor => sync_to_cursor(mcp_config, project_root),
        IdeTarget::VsCode => McpSyncResult {
            target,
            servers_synced: 0,
            success: true,
            error: None,
            config_path: PathBuf::new(),
        },
        IdeTarget::Windsurf => sync_to_windsurf(mcp_config, project_root),
    }
}

/// Sync MCP servers to Claude Code.
fn sync_to_claude_code(
    mcp_config: &McpConfig,
    project_root: Option<&Path>,
) -> McpSyncResult {
    // Determine target path: user global or project-local
    let config_path = if let Some(root) = project_root {
        root.join(".claude").join("settings.json")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join("settings.json")
    };

    sync_to_json_settings(
        IdeTarget::ClaudeCode,
        &config_path,
        mcp_config,
        "mcpServers",
    )
}

/// Sync MCP servers to Cursor.
fn sync_to_cursor(
    mcp_config: &McpConfig,
    project_root: Option<&Path>,
) -> McpSyncResult {
    // Cursor uses .cursor/mcp.json (project-level only)
    let config_path = if let Some(root) = project_root {
        root.join(".cursor").join("mcp.json")
    } else {
        // For global sync, use user home
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cursor")
            .join("mcp.json")
    };

    sync_to_json_mcp(IdeTarget::Cursor, &config_path, mcp_config)
}

/// Sync MCP servers to Windsurf.
fn sync_to_windsurf(
    mcp_config: &McpConfig,
    project_root: Option<&Path>,
) -> McpSyncResult {
    // Windsurf uses similar format to Claude Code
    let config_path = if let Some(root) = project_root {
        root.join(".windsurf").join("mcp.json")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".windsurf")
            .join("mcp.json")
    };

    sync_to_json_mcp(IdeTarget::Windsurf, &config_path, mcp_config)
}

/// Sync MCP to a JSON settings file with mcpServers key.
fn sync_to_json_settings(
    target: IdeTarget,
    config_path: &Path,
    mcp_config: &McpConfig,
    mcp_key: &str,
) -> McpSyncResult {
    // Load existing settings or create new
    let mut settings = if config_path.exists() {
        match std::fs::read_to_string(config_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| json!({})),
            Err(e) => {
                return McpSyncResult {
                    target,
                    servers_synced: 0,
                    success: false,
                    error: Some(format!("Failed to read config: {}", e)),
                    config_path: config_path.to_path_buf(),
                };
            }
        }
    } else {
        json!({})
    };

    // Build mcpServers object
    let servers_json = build_mcp_servers_json(mcp_config);
    let server_count = mcp_config.servers.len();

    // Merge with existing mcpServers (replace managed servers)
    if let Some(existing) = settings.get_mut(mcp_key) {
        if let Some(obj) = existing.as_object_mut() {
            // Add/update our servers
            if let Some(new_servers) = servers_json.as_object() {
                for (name, config) in new_servers {
                    obj.insert(name.clone(), config.clone());
                }
            }
        }
    } else {
        settings[mcp_key] = servers_json;
    }

    // Write back
    if let Err(e) = write_json_file(config_path, &settings) {
        return McpSyncResult {
            target,
            servers_synced: 0,
            success: false,
            error: Some(format!("Failed to write config: {}", e)),
            config_path: config_path.to_path_buf(),
        };
    }

    McpSyncResult {
        target,
        servers_synced: server_count,
        success: true,
        error: None,
        config_path: config_path.to_path_buf(),
    }
}

/// Sync MCP to a standalone mcp.json file.
fn sync_to_json_mcp(
    target: IdeTarget,
    config_path: &Path,
    mcp_config: &McpConfig,
) -> McpSyncResult {
    // Load existing mcp.json or create new
    let mut mcp_json = if config_path.exists() {
        match std::fs::read_to_string(config_path) {
            Ok(content) => serde_json::from_str(&content)
                .unwrap_or_else(|_| json!({"mcpServers": {}})),
            Err(e) => {
                return McpSyncResult {
                    target,
                    servers_synced: 0,
                    success: false,
                    error: Some(format!("Failed to read config: {}", e)),
                    config_path: config_path.to_path_buf(),
                };
            }
        }
    } else {
        json!({"mcpServers": {}})
    };

    // Build mcpServers object
    let servers_json = build_mcp_servers_json(mcp_config);
    let server_count = mcp_config.servers.len();

    // Merge with existing
    if let Some(existing) = mcp_json.get_mut("mcpServers") {
        if let Some(obj) = existing.as_object_mut() {
            if let Some(new_servers) = servers_json.as_object() {
                for (name, config) in new_servers {
                    obj.insert(name.clone(), config.clone());
                }
            }
        }
    } else {
        mcp_json["mcpServers"] = servers_json;
    }

    // Write back
    if let Err(e) = write_json_file(config_path, &mcp_json) {
        return McpSyncResult {
            target,
            servers_synced: 0,
            success: false,
            error: Some(format!("Failed to write config: {}", e)),
            config_path: config_path.to_path_buf(),
        };
    }

    McpSyncResult {
        target,
        servers_synced: server_count,
        success: true,
        error: None,
        config_path: config_path.to_path_buf(),
    }
}

/// Build JSON object for mcpServers from MCP config.
fn build_mcp_servers_json(mcp_config: &McpConfig) -> Value {
    let mut servers = serde_json::Map::new();

    for (name, server) in &mcp_config.servers {
        if !server.enabled {
            continue;
        }
        servers.insert(name.clone(), server_to_json(server));
    }

    Value::Object(servers)
}

/// Convert McpServer to JSON config.
fn server_to_json(server: &McpServer) -> Value {
    let mut config = json!({
        "command": server.command,
        "args": server.args,
    });

    if !server.env.is_empty() {
        config["env"] = json!(server.env);
    }

    config
}

/// Write JSON to file with pretty formatting.
fn write_json_file(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Get the config path for an IDE target.
pub fn config_path_for_target(target: IdeTarget, project_root: Option<&Path>) -> PathBuf {
    let base = project_root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));

    match target {
        IdeTarget::ClaudeCode => base.join(".claude").join("settings.json"),
        IdeTarget::Cursor => base.join(".cursor").join("mcp.json"),
        IdeTarget::VsCode => base.join(".vscode").join("settings.json"),
        IdeTarget::Windsurf => base.join(".windsurf").join("mcp.json"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::collections::HashMap;

    fn create_test_mcp_config() -> McpConfig {
        let mut config = McpConfig::default();
        config.servers.insert(
            "neo4j".to_string(),
            McpServer {
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@neo4j/mcp-server-neo4j".to_string()],
                env: HashMap::new(),
                description: Some("Neo4j MCP server".to_string()),
                enabled: true,
                source: None,
            },
        );
        config.servers.insert(
            "github".to_string(),
            McpServer {
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-github".to_string()],
                env: HashMap::from([("GITHUB_TOKEN".to_string(), "${GITHUB_TOKEN}".to_string())]),
                description: None,
                enabled: true,
                source: None,
            },
        );
        config
    }

    #[test]
    fn test_build_mcp_servers_json() {
        let config = create_test_mcp_config();
        let json = build_mcp_servers_json(&config);

        assert!(json["neo4j"].is_object());
        assert!(json["github"].is_object());
        assert_eq!(json["neo4j"]["command"], "npx");
        assert!(json["github"]["env"]["GITHUB_TOKEN"].is_string());
    }

    #[test]
    fn test_server_to_json() {
        let server = McpServer {
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
            env: HashMap::new(),
            description: None,
            enabled: true,
            source: None,
        };

        let json = server_to_json(&server);
        assert_eq!(json["command"], "node");
        assert_eq!(json["args"][0], "server.js");
        assert!(json.get("env").is_none());
    }

    #[test]
    fn test_sync_to_json_mcp_creates_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".cursor").join("mcp.json");
        let mcp_config = create_test_mcp_config();

        let result = sync_to_json_mcp(IdeTarget::Cursor, &config_path, &mcp_config);

        assert!(result.success);
        assert_eq!(result.servers_synced, 2);
        assert!(config_path.exists());

        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: Value = serde_json::from_str(&content).unwrap();
        assert!(json["mcpServers"]["neo4j"].is_object());
    }

    #[test]
    fn test_sync_preserves_existing_servers() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".cursor").join("mcp.json");

        // Create existing config with a server
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let existing = json!({
            "mcpServers": {
                "existing-server": {
                    "command": "node",
                    "args": ["existing.js"]
                }
            }
        });
        std::fs::write(&config_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        // Sync with new servers
        let mcp_config = create_test_mcp_config();
        let result = sync_to_json_mcp(IdeTarget::Cursor, &config_path, &mcp_config);

        assert!(result.success);

        // Verify existing server is preserved
        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: Value = serde_json::from_str(&content).unwrap();
        assert!(json["mcpServers"]["existing-server"].is_object());
        assert!(json["mcpServers"]["neo4j"].is_object());
    }

    #[test]
    fn test_disabled_servers_not_synced() {
        let mut config = create_test_mcp_config();
        config.servers.get_mut("neo4j").unwrap().enabled = false;

        let json = build_mcp_servers_json(&config);

        assert!(json.get("neo4j").is_none());
        assert!(json["github"].is_object());
    }

    #[test]
    fn test_config_path_for_target() {
        let temp = TempDir::new().unwrap();

        let claude_path = config_path_for_target(IdeTarget::ClaudeCode, Some(temp.path()));
        assert!(claude_path.ends_with(".claude/settings.json"));

        let cursor_path = config_path_for_target(IdeTarget::Cursor, Some(temp.path()));
        assert!(cursor_path.ends_with(".cursor/mcp.json"));
    }
}
