//! MCP server configuration types.
//!
//! Defines the structure for ~/.spn/mcp.yaml - the single source of truth
//! for all MCP server configurations across the SuperNovae ecosystem.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root configuration for MCP servers.
///
/// Stored at `~/.spn/mcp.yaml` (global) or `.spn/mcp.yaml` (project).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    /// Configuration version for migrations.
    #[serde(default = "default_version")]
    pub version: u32,

    /// MCP server definitions.
    #[serde(default)]
    pub servers: HashMap<String, McpServer>,
}

fn default_version() -> u32 {
    1
}

/// Individual MCP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Command to execute (e.g., "npx", "node", "novanet-mcp").
    pub command: String,

    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables for the server process.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Optional description for documentation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this server is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Source of this server (global, project, or workflow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<McpSource>,
}

fn default_enabled() -> bool {
    true
}

/// Source of an MCP server configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpSource {
    /// Global configuration (~/.spn/mcp.yaml).
    Global,
    /// Project configuration (.spn/mcp.yaml).
    Project,
    /// Workflow-level configuration (inline in workflow.nika.yaml).
    Workflow,
}

impl Default for McpSource {
    fn default() -> Self {
        Self::Global
    }
}

/// Project-level MCP overrides.
///
/// Stored in `spn.yaml` or `.spn/mcp.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMcpConfig {
    /// Servers to use from global config.
    #[serde(default, rename = "use")]
    pub use_servers: Vec<String>,

    /// Servers to disable for this project.
    #[serde(default)]
    pub disable: Vec<String>,

    /// Additional project-specific servers.
    #[serde(default)]
    pub servers: HashMap<String, McpServer>,
}

impl McpConfig {
    /// Create a new empty configuration.
    pub fn new() -> Self {
        Self {
            version: 1,
            servers: HashMap::new(),
        }
    }

    /// Add a server to the configuration.
    pub fn add_server(&mut self, name: String, server: McpServer) {
        self.servers.insert(name, server);
    }

    /// Remove a server from the configuration.
    pub fn remove_server(&mut self, name: &str) -> Option<McpServer> {
        self.servers.remove(name)
    }

    /// Check if a server exists.
    pub fn has_server(&self, name: &str) -> bool {
        self.servers.contains_key(name)
    }

    /// Get a server by name.
    pub fn get_server(&self, name: &str) -> Option<&McpServer> {
        self.servers.get(name)
    }

    /// List all server names.
    pub fn server_names(&self) -> Vec<&String> {
        self.servers.keys().collect()
    }

    /// Merge with project config, applying overrides.
    pub fn merge_with_project(&self, project: &ProjectMcpConfig) -> Self {
        let mut result = McpConfig::new();

        // Start with servers from "use" list or all global servers
        let use_servers: Vec<&String> = if project.use_servers.is_empty() {
            self.servers.keys().collect()
        } else {
            project.use_servers.iter().collect()
        };

        // Add used servers (not disabled)
        for name in use_servers {
            if project.disable.contains(name) {
                continue;
            }
            if let Some(server) = self.servers.get(name) {
                let mut merged_server = server.clone();
                merged_server.source = Some(McpSource::Global);
                result.servers.insert(name.clone(), merged_server);
            }
        }

        // Add project-specific servers
        for (name, server) in &project.servers {
            let mut project_server = server.clone();
            project_server.source = Some(McpSource::Project);
            result.servers.insert(name.clone(), project_server);
        }

        result
    }
}

impl McpServer {
    /// Create a new MCP server configuration.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            description: None,
            enabled: true,
            source: None,
        }
    }

    /// Builder: add arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Builder: add environment variables.
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Builder: add description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder: set enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Builder: set source.
    pub fn with_source(mut self, source: McpSource) -> Self {
        self.source = Some(source);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_new() {
        let config = McpConfig::new();
        assert_eq!(config.version, 1);
        assert!(config.servers.is_empty());
    }

    #[test]
    fn test_add_remove_server() {
        let mut config = McpConfig::new();
        let server = McpServer::new("npx")
            .with_args(vec!["-y".into(), "@neo4j/mcp-server".into()]);

        config.add_server("neo4j".into(), server);
        assert!(config.has_server("neo4j"));

        let removed = config.remove_server("neo4j");
        assert!(removed.is_some());
        assert!(!config.has_server("neo4j"));
    }

    #[test]
    fn test_merge_with_project() {
        let mut global = McpConfig::new();
        global.add_server("neo4j".into(), McpServer::new("npx"));
        global.add_server("linear".into(), McpServer::new("npx"));

        let project = ProjectMcpConfig {
            use_servers: vec!["neo4j".into()],
            disable: vec![],
            servers: {
                let mut s = HashMap::new();
                s.insert("custom".into(), McpServer::new("node"));
                s
            },
        };

        let merged = global.merge_with_project(&project);
        assert!(merged.has_server("neo4j"));
        assert!(!merged.has_server("linear")); // Not in use list
        assert!(merged.has_server("custom")); // Project-specific
    }

    #[test]
    fn test_merge_with_disable() {
        let mut global = McpConfig::new();
        global.add_server("neo4j".into(), McpServer::new("npx"));
        global.add_server("linear".into(), McpServer::new("npx"));

        let project = ProjectMcpConfig {
            use_servers: vec![], // Use all
            disable: vec!["linear".into()],
            servers: HashMap::new(),
        };

        let merged = global.merge_with_project(&project);
        assert!(merged.has_server("neo4j"));
        assert!(!merged.has_server("linear")); // Disabled
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut config = McpConfig::new();
        config.add_server(
            "neo4j".into(),
            McpServer::new("npx")
                .with_args(vec!["-y".into(), "@neo4j/mcp-server".into()])
                .with_description("Neo4j graph database"),
        );

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: McpConfig = serde_yaml::from_str(&yaml).unwrap();

        assert!(parsed.has_server("neo4j"));
        assert_eq!(
            parsed.get_server("neo4j").unwrap().description,
            Some("Neo4j graph database".into())
        );
    }
}
