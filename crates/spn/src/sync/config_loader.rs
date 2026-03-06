//! JSON configuration loading and manipulation utilities.
//!
//! This module provides common operations for loading, modifying, and saving
//! JSON configuration files used by various IDEs (Claude Code, Cursor, etc.).
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::sync::config_loader::{load_json_config, insert_mcp_server, write_json_config};
//!
//! // Load or create config
//! let mut config = load_json_config(&path, Some("mcpServers"))?;
//!
//! // Insert MCP server
//! insert_mcp_server(&mut config, "server-name", json!({"command": "npx"}));
//!
//! // Save back
//! write_json_config(&path, &config)?;
//! ```

use serde_json::{json, Value};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when loading/saving JSON configs.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to read the config file.
    #[error("Failed to read config file '{path}': {source}")]
    ReadFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to write the config file.
    #[error("Failed to write config file '{path}': {source}")]
    WriteFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to serialize JSON.
    #[error("Failed to serialize JSON: {0}")]
    SerializeFailed(#[from] serde_json::Error),
}

/// Load a JSON config file with graceful fallback to empty object.
///
/// If the file doesn't exist or contains invalid JSON, returns a default
/// object. If `default_key` is provided, the default will be `{ key: {} }`.
///
/// # Arguments
///
/// * `path` - Path to the JSON config file
/// * `default_key` - Optional key to initialize in the default object
///
/// # Example
///
/// ```rust,ignore
/// // Returns {} if file doesn't exist
/// let config = load_json_config(&path, None)?;
///
/// // Returns {"mcpServers": {}} if file doesn't exist
/// let config = load_json_config(&path, Some("mcpServers"))?;
/// ```
pub fn load_json_config(path: &Path, default_key: Option<&str>) -> Result<Value, ConfigError> {
    let default = default_key.map_or_else(|| json!({}), |k| json!({ k: {} }));

    if !path.exists() {
        return Ok(default);
    }

    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFailed {
        path: path.display().to_string(),
        source: e,
    })?;

    // Parse JSON, falling back to default on invalid JSON
    Ok(serde_json::from_str(&content).unwrap_or(default))
}

/// Insert an MCP server into a config's `mcpServers` object.
///
/// Creates the `mcpServers` key if it doesn't exist.
///
/// # Arguments
///
/// * `config` - The JSON config value to modify (mutated in place)
/// * `name` - The server name/key
/// * `server_config` - The server configuration value
///
/// # Example
///
/// ```rust,ignore
/// let mut config = json!({});
/// insert_mcp_server(&mut config, "my-server", json!({"command": "npx"}));
/// // config is now {"mcpServers": {"my-server": {"command": "npx"}}}
/// ```
pub fn insert_mcp_server(config: &mut Value, name: &str, server_config: Value) {
    // Ensure mcpServers exists as an object
    if !config.get("mcpServers").map_or(false, |v| v.is_object()) {
        config["mcpServers"] = json!({});
    }

    // Insert the server
    if let Some(servers) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.insert(name.to_string(), server_config);
    }
}

/// Insert multiple MCP servers into a config.
///
/// Convenience wrapper around [`insert_mcp_server`] for batch operations.
pub fn insert_mcp_servers<I>(config: &mut Value, servers: I)
where
    I: IntoIterator<Item = (String, Value)>,
{
    for (name, server_config) in servers {
        insert_mcp_server(config, &name, server_config);
    }
}

/// Write a JSON config file with pretty formatting.
///
/// Creates parent directories if they don't exist.
///
/// # Arguments
///
/// * `path` - Path to write the JSON file
/// * `value` - The JSON value to write
pub fn write_json_config(path: &Path, value: &Value) -> Result<(), ConfigError> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFailed {
            path: path.display().to_string(),
            source: e,
        })?;
    }

    let content = serde_json::to_string_pretty(value)?;

    std::fs::write(path, content).map_err(|e| ConfigError::WriteFailed {
        path: path.display().to_string(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_nonexistent_returns_default() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nonexistent.json");

        let config = load_json_config(&path, None).unwrap();
        assert_eq!(config, json!({}));
    }

    #[test]
    fn test_load_nonexistent_with_default_key() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nonexistent.json");

        let config = load_json_config(&path, Some("mcpServers")).unwrap();
        assert_eq!(config, json!({"mcpServers": {}}));
    }

    #[test]
    fn test_load_existing_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.json");
        std::fs::write(&path, r#"{"key": "value"}"#).unwrap();

        let config = load_json_config(&path, None).unwrap();
        assert_eq!(config, json!({"key": "value"}));
    }

    #[test]
    fn test_load_invalid_json_returns_default() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("invalid.json");
        std::fs::write(&path, "not valid json").unwrap();

        let config = load_json_config(&path, Some("mcpServers")).unwrap();
        assert_eq!(config, json!({"mcpServers": {}}));
    }

    #[test]
    fn test_insert_mcp_server_creates_key() {
        let mut config = json!({});
        insert_mcp_server(&mut config, "test-server", json!({"command": "npx"}));

        assert_eq!(
            config,
            json!({
                "mcpServers": {
                    "test-server": {"command": "npx"}
                }
            })
        );
    }

    #[test]
    fn test_insert_mcp_server_preserves_existing() {
        let mut config = json!({
            "mcpServers": {
                "existing": {"command": "node"}
            },
            "otherKey": "preserved"
        });

        insert_mcp_server(&mut config, "new-server", json!({"command": "npx"}));

        assert_eq!(config["otherKey"], "preserved");
        assert!(config["mcpServers"]["existing"].is_object());
        assert!(config["mcpServers"]["new-server"].is_object());
    }

    #[test]
    fn test_insert_multiple_servers() {
        let mut config = json!({});

        insert_mcp_servers(
            &mut config,
            vec![
                ("server1".to_string(), json!({"command": "cmd1"})),
                ("server2".to_string(), json!({"command": "cmd2"})),
            ],
        );

        assert!(config["mcpServers"]["server1"].is_object());
        assert!(config["mcpServers"]["server2"].is_object());
    }

    #[test]
    fn test_write_json_config() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("output.json");

        let value = json!({"key": "value"});
        write_json_config(&path, &value).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("key"));
        assert!(content.contains("value"));
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested").join("dir").join("config.json");

        write_json_config(&path, &json!({})).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_roundtrip() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("roundtrip.json");

        // Write
        let original = json!({
            "mcpServers": {
                "test": {"command": "npx", "args": ["arg1"]}
            }
        });
        write_json_config(&path, &original).unwrap();

        // Read back
        let loaded = load_json_config(&path, None).unwrap();
        assert_eq!(loaded, original);
    }
}
