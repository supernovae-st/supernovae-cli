//! Team/project configuration (./mcp.yaml, ./spn.yaml).

use crate::config::types::{Config, McpServerConfig};
use crate::error::{Result, SpnError};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Team MCP configuration file format (./mcp.yaml).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct McpYaml {
    /// MCP servers.
    #[serde(default)]
    servers: FxHashMap<String, McpServerConfig>,
}

/// Get path to team MCP config (./mcp.yaml).
pub fn mcp_config_path(project_root: &Path) -> PathBuf {
    project_root.join("mcp.yaml")
}

/// Get path to team package config (./spn.yaml).
pub fn package_config_path(project_root: &Path) -> PathBuf {
    project_root.join("spn.yaml")
}

/// Load team configuration (merges mcp.yaml + spn.yaml).
///
/// Returns empty config if files don't exist.
pub fn load(project_root: &Path) -> Result<Config> {
    let mut config = Config::default();

    // Load MCP servers from ./mcp.yaml
    let mcp_path = mcp_config_path(project_root);
    if mcp_path.exists() {
        let content = fs::read_to_string(&mcp_path).map_err(|e| {
            SpnError::ConfigError(format!("Failed to read {}: {}", mcp_path.display(), e))
        })?;

        let mcp_yaml: McpYaml = serde_yaml::from_str(&content).map_err(|e| {
            SpnError::ConfigError(format!("Failed to parse {}: {}", mcp_path.display(), e))
        })?;

        config.servers = mcp_yaml.servers;
    }

    // Load team config from ./spn.yaml (if it has config section)
    let spn_path = package_config_path(project_root);
    if spn_path.exists() {
        // For now, spn.yaml only contains package dependencies
        // In the future, we might add a [config] section
    }

    Ok(config)
}

/// Save team MCP configuration to ./mcp.yaml.
pub fn save_mcp(project_root: &Path, servers: &FxHashMap<String, McpServerConfig>) -> Result<()> {
    let path = mcp_config_path(project_root);

    let mcp_yaml = McpYaml {
        servers: servers.clone(),
    };

    let content = serde_yaml::to_string(&mcp_yaml)
        .map_err(|e| SpnError::ConfigError(format!("Failed to serialize MCP config: {}", e)))?;

    fs::write(&path, content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_team_config_paths() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let mcp_path = mcp_config_path(root);
        assert!(mcp_path.ends_with("mcp.yaml"));

        let spn_path = package_config_path(root);
        assert!(spn_path.ends_with("spn.yaml"));
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = TempDir::new().unwrap();
        let config = load(dir.path()).unwrap();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn test_save_and_load_mcp() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let mut servers = FxHashMap::default();
        servers.insert(
            "neo4j".to_string(),
            McpServerConfig {
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@neo4j/mcp-server-neo4j".to_string()],
                env: Default::default(),
                disabled: false,
            },
        );

        save_mcp(root, &servers).unwrap();

        let config = load(root).unwrap();
        assert_eq!(config.servers.len(), 1);
        assert!(config.servers.contains_key("neo4j"));
    }
}
