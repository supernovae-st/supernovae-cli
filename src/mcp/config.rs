//! MCP configuration file management.
//!
//! Handles loading and saving MCP configurations from:
//! - Global: `~/.spn/mcp.yaml`
//! - Project: `.spn/mcp.yaml` or `spn.yaml` (mcp section)

use crate::error::Result;
use crate::mcp::types::{McpConfig, McpServer, McpSource, ProjectMcpConfig};
use std::path::{Path, PathBuf};

/// Location scope for MCP configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpScope {
    /// Global configuration (~/.spn/mcp.yaml).
    Global,
    /// Project configuration (.spn/mcp.yaml).
    Project,
}

impl std::fmt::Display for McpScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Project => write!(f, "project"),
        }
    }
}

/// Manager for MCP configuration files.
pub struct McpConfigManager {
    global_path: PathBuf,
    project_root: Option<PathBuf>,
}

impl McpConfigManager {
    /// Create a new config manager.
    pub fn new() -> Self {
        Self {
            global_path: Self::default_global_path(),
            project_root: None,
        }
    }

    /// Create a config manager with a project root.
    pub fn with_project(project_root: PathBuf) -> Self {
        Self {
            global_path: Self::default_global_path(),
            project_root: Some(project_root),
        }
    }

    /// Get the default global config path.
    pub fn default_global_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".spn")
            .join("mcp.yaml")
    }

    /// Get the project config path.
    pub fn project_path(&self) -> Option<PathBuf> {
        self.project_root
            .as_ref()
            .map(|root| root.join(".spn").join("mcp.yaml"))
    }

    /// Get the path for a given scope.
    pub fn path_for_scope(&self, scope: McpScope) -> PathBuf {
        match scope {
            McpScope::Global => self.global_path.clone(),
            McpScope::Project => self
                .project_path()
                .unwrap_or_else(|| PathBuf::from(".spn/mcp.yaml")),
        }
    }

    /// Load global MCP configuration.
    pub fn load_global(&self) -> Result<McpConfig> {
        Self::load_from_path(&self.global_path)
    }

    /// Load project MCP configuration.
    pub fn load_project(&self) -> Result<Option<ProjectMcpConfig>> {
        let Some(path) = self.project_path() else {
            return Ok(None);
        };

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)?;
        let config: ProjectMcpConfig = serde_yaml::from_str(&content)?;

        Ok(Some(config))
    }

    /// Load resolved MCP configuration (global + project merged).
    pub fn load_resolved(&self) -> Result<McpConfig> {
        let global = self.load_global()?;

        if let Some(project) = self.load_project()? {
            Ok(global.merge_with_project(&project))
        } else {
            Ok(global)
        }
    }

    /// Save MCP configuration to a specific scope.
    pub fn save(&self, config: &McpConfig, scope: McpScope) -> Result<()> {
        let path = self.path_for_scope(scope);
        Self::save_to_path(config, &path)
    }

    /// Add a server to a specific scope.
    pub fn add_server(
        &self,
        name: &str,
        server: McpServer,
        scope: McpScope,
    ) -> Result<()> {
        let path = self.path_for_scope(scope);
        let mut config = Self::load_from_path(&path).unwrap_or_default();

        config.add_server(name.to_string(), server);
        Self::save_to_path(&config, &path)?;

        Ok(())
    }

    /// Remove a server from a specific scope.
    pub fn remove_server(&self, name: &str, scope: McpScope) -> Result<bool> {
        let path = self.path_for_scope(scope);
        let mut config = Self::load_from_path(&path)?;

        let removed = config.remove_server(name).is_some();
        if removed {
            Self::save_to_path(&config, &path)?;
        }

        Ok(removed)
    }

    /// List servers from a specific scope.
    pub fn list_servers(&self, scope: McpScope) -> Result<Vec<(String, McpServer)>> {
        let path = self.path_for_scope(scope);
        let config = Self::load_from_path(&path)?;

        Ok(config
            .servers
            .into_iter()
            .map(|(name, mut server)| {
                server.source = Some(match scope {
                    McpScope::Global => McpSource::Global,
                    McpScope::Project => McpSource::Project,
                });
                (name, server)
            })
            .collect())
    }

    /// List all servers (global + project merged).
    pub fn list_all_servers(&self) -> Result<Vec<(String, McpServer)>> {
        let config = self.load_resolved()?;
        Ok(config.servers.into_iter().collect())
    }

    /// Check if a server exists in a specific scope.
    pub fn has_server(&self, name: &str, scope: McpScope) -> Result<bool> {
        let path = self.path_for_scope(scope);
        let config = Self::load_from_path(&path)?;
        Ok(config.has_server(name))
    }

    /// Load configuration from a specific path.
    fn load_from_path(path: &Path) -> Result<McpConfig> {
        if !path.exists() {
            return Ok(McpConfig::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: McpConfig = serde_yaml::from_str(&content)?;

        Ok(config)
    }

    /// Save configuration to a specific path.
    fn save_to_path(config: &McpConfig, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(config)?;

        // Add header comment
        let content_with_header = format!(
            "# SuperNovae MCP Server Configuration\n\
             # This file is the single source of truth for MCP servers.\n\
             # Managed by: spn mcp add/remove\n\
             # Documentation: https://supernovae.studio/docs/mcp\n\
             \n\
             {content}"
        );

        std::fs::write(path, content_with_header)?;

        Ok(())
    }
}

impl Default for McpConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect the current project root by looking for spn.yaml.
pub fn find_project_root() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;

    for ancestor in current.ancestors() {
        if ancestor.join("spn.yaml").exists() {
            return Some(ancestor.to_path_buf());
        }
    }

    None
}

/// Create an McpConfigManager with auto-detected project root.
pub fn config_manager() -> McpConfigManager {
    match find_project_root() {
        Some(root) => McpConfigManager::with_project(root),
        None => McpConfigManager::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, McpConfigManager) {
        let temp = TempDir::new().unwrap();
        let manager = McpConfigManager {
            global_path: temp.path().join("global").join("mcp.yaml"),
            project_root: Some(temp.path().join("project")),
        };

        // Create directories
        std::fs::create_dir_all(temp.path().join("global")).unwrap();
        std::fs::create_dir_all(temp.path().join("project").join(".spn")).unwrap();

        (temp, manager)
    }

    #[test]
    fn test_load_empty_global() {
        let (_temp, manager) = setup_test_env();
        let config = manager.load_global().unwrap();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn test_add_and_load_server() {
        let (_temp, manager) = setup_test_env();

        let server = McpServer::new("npx")
            .with_args(vec!["-y".into(), "@neo4j/mcp-server".into()]);

        manager
            .add_server("neo4j", server, McpScope::Global)
            .unwrap();

        let config = manager.load_global().unwrap();
        assert!(config.has_server("neo4j"));
    }

    #[test]
    fn test_remove_server() {
        let (_temp, manager) = setup_test_env();

        let server = McpServer::new("npx");
        manager
            .add_server("test", server, McpScope::Global)
            .unwrap();

        assert!(manager.has_server("test", McpScope::Global).unwrap());

        manager.remove_server("test", McpScope::Global).unwrap();

        assert!(!manager.has_server("test", McpScope::Global).unwrap());
    }

    #[test]
    fn test_list_servers() {
        let (_temp, manager) = setup_test_env();

        manager
            .add_server("server1", McpServer::new("cmd1"), McpScope::Global)
            .unwrap();
        manager
            .add_server("server2", McpServer::new("cmd2"), McpScope::Global)
            .unwrap();

        let servers = manager.list_servers(McpScope::Global).unwrap();
        assert_eq!(servers.len(), 2);
    }

    #[test]
    fn test_scope_display() {
        assert_eq!(format!("{}", McpScope::Global), "global");
        assert_eq!(format!("{}", McpScope::Project), "project");
    }
}
