//! Configuration data types.
//!
//! TODO(v0.14): Integrate sync configuration with `spn sync`

#![allow(dead_code)]

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// Root configuration structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Provider configurations (LLM APIs).
    #[serde(default)]
    pub providers: FxHashMap<String, ProviderConfig>,

    /// Sync configuration.
    #[serde(default)]
    pub sync: SyncConfig,

    /// Secrets configuration.
    #[serde(default)]
    pub secrets: SecretsConfig,

    /// MCP servers (only in team/local configs).
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    pub servers: FxHashMap<String, McpServerConfig>,
}

/// Provider configuration (Anthropic, OpenAI, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Default model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// API endpoint override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Additional provider-specific settings.
    #[serde(flatten)]
    pub extra: FxHashMap<String, serde_json::Value>,
}

/// Sync configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enabled editor targets.
    #[serde(default)]
    pub enabled_editors: Vec<String>,

    /// Auto-sync on install/add.
    #[serde(default)]
    pub auto_sync: bool,
}

/// Secrets configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Default storage backend for new keys.
    /// Options: "keychain", "env", "global"
    #[serde(default = "default_storage")]
    pub default_storage: String,

    /// Automatically migrate env vars to keychain.
    #[serde(default)]
    pub auto_migrate: bool,
}

fn default_storage() -> String {
    "keychain".to_string()
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            default_storage: default_storage(),
            auto_migrate: false,
        }
    }
}

/// MCP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to execute.
    pub command: String,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    pub env: FxHashMap<String, String>,

    /// Whether this is disabled.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
}

impl Config {
    /// Create an empty config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another config into this one (other takes precedence).
    pub fn merge(&mut self, other: Config) {
        // Merge providers
        for (name, provider) in other.providers {
            self.providers.insert(name, provider);
        }

        // Merge sync (replace)
        if !other.sync.enabled_editors.is_empty() || other.sync.auto_sync {
            self.sync = other.sync;
        }

        // Merge secrets (replace if non-default)
        if other.secrets.default_storage != "keychain" || other.secrets.auto_migrate {
            self.secrets = other.secrets;
        }

        // Merge servers
        for (name, server) in other.servers {
            self.servers.insert(name, server);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_merge() {
        let mut base = Config {
            providers: FxHashMap::from_iter([(
                "anthropic".to_string(),
                ProviderConfig {
                    model: Some("claude-sonnet-4-5".to_string()),
                    endpoint: None,
                    extra: FxHashMap::default(),
                },
            )]),
            sync: SyncConfig {
                enabled_editors: vec!["claude-code".to_string()],
                auto_sync: false,
            },
            secrets: SecretsConfig::default(),
            servers: FxHashMap::default(),
        };

        let override_config = Config {
            providers: FxHashMap::from_iter([(
                "anthropic".to_string(),
                ProviderConfig {
                    model: Some("claude-opus-4-5".to_string()),
                    endpoint: None,
                    extra: FxHashMap::default(),
                },
            )]),
            sync: SyncConfig {
                enabled_editors: vec![],
                auto_sync: true,
            },
            secrets: SecretsConfig::default(),
            servers: FxHashMap::default(),
        };

        base.merge(override_config);

        assert_eq!(
            base.providers.get("anthropic").unwrap().model,
            Some("claude-opus-4-5".to_string())
        );
        assert!(base.sync.auto_sync);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.providers.is_empty());
        assert!(config.servers.is_empty());
        assert!(!config.sync.auto_sync);
        assert_eq!(config.secrets.default_storage, "keychain");
        assert!(!config.secrets.auto_migrate);
    }

    #[test]
    fn test_secrets_config_default() {
        let secrets = SecretsConfig::default();
        assert_eq!(secrets.default_storage, "keychain");
        assert!(!secrets.auto_migrate);
    }

    #[test]
    fn test_secrets_config_merge() {
        let mut base = Config {
            providers: FxHashMap::default(),
            sync: SyncConfig::default(),
            secrets: SecretsConfig {
                default_storage: "keychain".to_string(),
                auto_migrate: false,
            },
            servers: FxHashMap::default(),
        };

        // Override with different storage backend
        let override_config = Config {
            providers: FxHashMap::default(),
            sync: SyncConfig::default(),
            secrets: SecretsConfig {
                default_storage: "env".to_string(),
                auto_migrate: true,
            },
            servers: FxHashMap::default(),
        };

        base.merge(override_config);

        // Secrets should be replaced when non-default
        assert_eq!(base.secrets.default_storage, "env");
        assert!(base.secrets.auto_migrate);
    }

    #[test]
    fn test_secrets_config_merge_keeps_base_when_override_is_default() {
        let mut base = Config {
            providers: FxHashMap::default(),
            sync: SyncConfig::default(),
            secrets: SecretsConfig {
                default_storage: "global".to_string(),
                auto_migrate: true,
            },
            servers: FxHashMap::default(),
        };

        // Override with default values should not replace
        let override_config = Config {
            providers: FxHashMap::default(),
            sync: SyncConfig::default(),
            secrets: SecretsConfig::default(), // default: keychain, auto_migrate: false
            servers: FxHashMap::default(),
        };

        base.merge(override_config);

        // Base secrets should be preserved (override was default)
        assert_eq!(base.secrets.default_storage, "global");
        assert!(base.secrets.auto_migrate);
    }
}
