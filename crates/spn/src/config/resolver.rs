//! Configuration resolver - merges all three scopes.
//!
//! Supports key path resolution for accessing nested values like:
//! - `providers.anthropic.model`
//! - `sync.auto_sync`
//! - `servers.neo4j.command`

#![allow(dead_code)]

use crate::config::{global, local, team, types::Config, ConfigScope, ScopeType};
use crate::error::Result;
use serde_json::Value;
use std::path::Path;

/// Configuration resolver that merges all scopes.
pub struct ConfigResolver {
    /// Resolved final configuration (merged).
    pub config: Config,
    /// Individual scope configurations.
    pub scopes: Vec<(ScopeType, Config)>,
}

impl ConfigResolver {
    /// Load configuration from all scopes.
    ///
    /// Precedence: Local > Team > Global (innermost wins).
    pub fn load() -> Result<Self> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        Self::load_from(&cwd)
    }

    /// Load configuration from specific project root.
    pub fn load_from(project_root: &Path) -> Result<Self> {
        let mut config = Config::default();
        let mut scopes = Vec::new();

        // Load Global (~/.spn/config.toml)
        let global_config = global::load()?;
        scopes.push((ScopeType::Global, global_config.clone()));
        config.merge(global_config);

        // Load Team (./mcp.yaml, ./spn.yaml)
        let team_config = team::load(project_root)?;
        scopes.push((ScopeType::Team, team_config.clone()));
        config.merge(team_config);

        // Load Local (./.spn/local.yaml)
        let local_config = local::load(project_root)?;
        scopes.push((ScopeType::Local, local_config.clone()));
        config.merge(local_config);

        Ok(Self { config, scopes })
    }

    /// Get final resolved configuration.
    pub fn resolved(&self) -> &Config {
        &self.config
    }

    /// Get configuration for a specific scope.
    pub fn get_scope(&self, scope_type: ScopeType) -> Option<&Config> {
        self.scopes
            .iter()
            .find(|(st, _)| *st == scope_type)
            .map(|(_, cfg)| cfg)
    }

    /// Show which scope defined a specific value.
    ///
    /// Checks scopes in reverse order (Local -> Team -> Global) and returns
    /// the first scope that has the key path defined.
    pub fn get_origin(&self, key: &str) -> Option<ScopeType> {
        // Check scopes in reverse order (Local -> Team -> Global)
        for (scope_type, config) in self.scopes.iter().rev() {
            if get_value_at_path(config, key).is_some() {
                return Some(*scope_type);
            }
        }
        None
    }

    /// Get a value at a key path from the resolved configuration.
    ///
    /// Key paths are dot-separated: `providers.anthropic.model`, `sync.auto_sync`
    pub fn get_value(&self, key: &str) -> Option<Value> {
        get_value_at_path(&self.config, key)
    }

    /// Get a value at a key path from a specific scope.
    pub fn get_value_from_scope(&self, scope_type: ScopeType, key: &str) -> Option<Value> {
        self.get_scope(scope_type)
            .and_then(|config| get_value_at_path(config, key))
    }

    /// Get all scope paths.
    pub fn get_scope_paths(&self) -> Result<Vec<ConfigScope>> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        Ok(vec![
            ConfigScope::new(ScopeType::Global, global::config_path()?),
            ConfigScope::new(ScopeType::Team, team::mcp_config_path(&cwd)),
            ConfigScope::new(ScopeType::Local, local::config_path(&cwd)),
        ])
    }
}

/// Get a value at a dot-separated key path from a Config.
///
/// Examples:
/// - `providers.anthropic.model`
/// - `sync.auto_sync`
/// - `servers.neo4j.command`
fn get_value_at_path(config: &Config, key: &str) -> Option<Value> {
    // Convert config to JSON Value for traversal
    let json = serde_json::to_value(config).ok()?;

    // Split key path and traverse
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = &json;

    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }

    // Don't return null values
    if current.is_null() {
        return None;
    }

    Some(current.clone())
}

/// Format a JSON value for display.
pub fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(_) => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
        }
        Value::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::McpServerConfig;
    use rustc_hash::FxHashMap;
    use tempfile::TempDir;

    #[test]
    fn test_resolver_load() {
        let dir = TempDir::new().unwrap();
        let resolver = ConfigResolver::load_from(dir.path()).unwrap();
        assert!(resolver.config.providers.is_empty());
    }

    #[test]
    fn test_scope_precedence() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Create team config with neo4j
        let mut team_servers = FxHashMap::default();
        team_servers.insert(
            "neo4j".to_string(),
            McpServerConfig {
                command: "npx".to_string(),
                args: vec![],
                env: Default::default(),
                disabled: false,
            },
        );
        team::save_mcp(root, &team_servers).unwrap();

        // Create local config that overrides neo4j command
        let mut local_config = Config::default();
        local_config.servers.insert(
            "neo4j".to_string(),
            McpServerConfig {
                command: "node".to_string(), // Override
                args: vec![],
                env: Default::default(),
                disabled: false,
            },
        );
        local::save(root, &local_config).unwrap();

        // Resolve
        let resolver = ConfigResolver::load_from(root).unwrap();

        // Local should win
        let neo4j = resolver.config.servers.get("neo4j").unwrap();
        assert_eq!(neo4j.command, "node");
    }

    #[test]
    fn test_get_value_at_path() {
        use crate::config::types::{ProviderConfig, SyncConfig};

        let mut config = Config::default();
        config.providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                model: Some("claude-opus-4-5".to_string()),
                endpoint: None,
                extra: FxHashMap::default(),
            },
        );
        config.sync = SyncConfig {
            enabled_editors: vec!["claude-code".to_string()],
            auto_sync: true,
        };
        config.servers.insert(
            "neo4j".to_string(),
            McpServerConfig {
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@neo4j/mcp".to_string()],
                env: FxHashMap::default(),
                disabled: false,
            },
        );

        // Test provider access
        let value = get_value_at_path(&config, "providers.anthropic.model");
        assert_eq!(value.unwrap().as_str().unwrap(), "claude-opus-4-5");

        // Test sync access
        let value = get_value_at_path(&config, "sync.auto_sync");
        assert_eq!(value.unwrap().as_bool().unwrap(), true);

        // Test server access
        let value = get_value_at_path(&config, "servers.neo4j.command");
        assert_eq!(value.unwrap().as_str().unwrap(), "npx");

        // Test nested array
        let value = get_value_at_path(&config, "servers.neo4j.args");
        assert!(value.unwrap().is_array());

        // Test non-existent path
        let value = get_value_at_path(&config, "providers.openai.model");
        assert!(value.is_none());

        // Test partial path
        let value = get_value_at_path(&config, "providers.anthropic");
        assert!(value.unwrap().is_object());
    }

    #[test]
    fn test_format_value() {
        assert_eq!(format_value(&Value::String("test".to_string())), "test");
        assert_eq!(format_value(&Value::Bool(true)), "true");
        assert_eq!(format_value(&Value::Number(42.into())), "42");
        assert_eq!(
            format_value(&Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string())
            ])),
            "[a, b]"
        );
    }
}
