//! Configuration resolver - merges all three scopes.

use crate::config::{global, local, team, types::Config, ConfigScope, ScopeType};
use crate::error::Result;
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
    pub fn get_origin(&self, _key: &str) -> Option<ScopeType> {
        // Check scopes in reverse order (Local -> Team -> Global)
        // Return the first one that has the key defined
        for (scope_type, config) in self.scopes.iter().rev() {
            // TODO: Implement proper key path resolution
            // For now, just check if config is non-empty
            if !config.providers.is_empty() || !config.servers.is_empty() {
                return Some(*scope_type);
            }
        }
        None
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
}
