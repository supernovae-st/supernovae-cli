//! MCP (Model Context Protocol) configuration types.
//!
//! These types are shared between nika (MCP client) and spn (MCP config manager).

/// Type of MCP server transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum McpServerType {
    /// Standard I/O transport (spawned process)
    #[default]
    Stdio,
    /// Server-Sent Events over HTTP
    Sse,
    /// WebSocket transport
    WebSocket,
}

/// MCP server configuration.
///
/// This represents a single MCP server that can be connected to.
#[derive(Debug, Clone, Default)]
pub struct McpServer {
    /// Server name/identifier
    pub name: String,
    /// Transport type
    pub server_type: McpServerType,
    /// Command to run (for stdio transport)
    pub command: Option<String>,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// URL for SSE/WebSocket transports
    pub url: Option<String>,
    /// Whether this server is enabled
    pub enabled: bool,
}

impl McpServer {
    /// Create a new stdio MCP server.
    ///
    /// # Example
    ///
    /// ```
    /// use spn_core::McpServer;
    ///
    /// let server = McpServer::stdio("neo4j", "npx", vec!["-y", "@anthropic/mcp-neo4j"]);
    /// assert_eq!(server.name, "neo4j");
    /// assert!(server.enabled);
    /// ```
    pub fn stdio(name: impl Into<String>, command: impl Into<String>, args: Vec<&str>) -> Self {
        Self {
            name: name.into(),
            server_type: McpServerType::Stdio,
            command: Some(command.into()),
            args: args.into_iter().map(String::from).collect(),
            env: Vec::new(),
            url: None,
            enabled: true,
        }
    }

    /// Create a new SSE MCP server.
    ///
    /// # Example
    ///
    /// ```
    /// use spn_core::McpServer;
    ///
    /// let server = McpServer::sse("remote", "http://localhost:3000/mcp");
    /// assert_eq!(server.name, "remote");
    /// ```
    pub fn sse(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            server_type: McpServerType::Sse,
            command: None,
            args: Vec::new(),
            env: Vec::new(),
            url: Some(url.into()),
            enabled: true,
        }
    }

    /// Add an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Source of an MCP configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpSource {
    /// From spn global config (~/.spn/config.toml)
    SpnGlobal,
    /// From project spn.yaml
    SpnProject,
    /// From Claude Code config
    ClaudeCode,
    /// From Cursor config
    Cursor,
    /// From VS Code config
    VsCode,
    /// Discovered at runtime
    Discovered,
}

/// Complete MCP configuration containing multiple servers.
#[derive(Debug, Clone, Default)]
pub struct McpConfig {
    /// List of configured MCP servers
    pub servers: Vec<McpServer>,
    /// Source of this configuration
    pub source: Option<McpSource>,
}

impl McpConfig {
    /// Create an empty MCP configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a server to the configuration.
    pub fn add_server(&mut self, server: McpServer) {
        self.servers.push(server);
    }

    /// Find a server by name.
    pub fn find_server(&self, name: &str) -> Option<&McpServer> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// Get all enabled servers.
    pub fn enabled_servers(&self) -> impl Iterator<Item = &McpServer> {
        self.servers.iter().filter(|s| s.enabled)
    }

    /// Merge another configuration into this one.
    ///
    /// Servers with the same name are overwritten.
    pub fn merge(&mut self, other: McpConfig) {
        for server in other.servers {
            // Remove existing server with same name
            self.servers.retain(|s| s.name != server.name);
            self.servers.push(server);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_server() {
        let server = McpServer::stdio("test", "node", vec!["server.js"]);
        assert_eq!(server.name, "test");
        assert_eq!(server.server_type, McpServerType::Stdio);
        assert_eq!(server.command, Some("node".to_string()));
        assert_eq!(server.args, vec!["server.js"]);
        assert!(server.enabled);
    }

    #[test]
    fn test_sse_server() {
        let server = McpServer::sse("remote", "http://localhost:3000");
        assert_eq!(server.server_type, McpServerType::Sse);
        assert_eq!(server.url, Some("http://localhost:3000".to_string()));
    }

    #[test]
    fn test_server_with_env() {
        let server = McpServer::stdio("neo4j", "npx", vec!["-y", "@anthropic/mcp-neo4j"])
            .with_env("NEO4J_URI", "bolt://localhost:7687")
            .with_env("NEO4J_PASSWORD", "secret");

        assert_eq!(server.env.len(), 2);
        assert_eq!(server.env[0], ("NEO4J_URI".to_string(), "bolt://localhost:7687".to_string()));
    }

    #[test]
    fn test_config_add_find() {
        let mut config = McpConfig::new();
        config.add_server(McpServer::stdio("neo4j", "npx", vec![]));
        config.add_server(McpServer::stdio("github", "npx", vec![]));

        assert!(config.find_server("neo4j").is_some());
        assert!(config.find_server("github").is_some());
        assert!(config.find_server("unknown").is_none());
    }

    #[test]
    fn test_config_enabled_servers() {
        let mut config = McpConfig::new();
        config.add_server(McpServer::stdio("enabled1", "cmd", vec![]));
        config.add_server(McpServer::stdio("disabled", "cmd", vec![]).with_enabled(false));
        config.add_server(McpServer::stdio("enabled2", "cmd", vec![]));

        let enabled: Vec<_> = config.enabled_servers().collect();
        assert_eq!(enabled.len(), 2);
        assert!(enabled.iter().all(|s| s.enabled));
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = McpConfig::new();
        config1.add_server(McpServer::stdio("neo4j", "old-cmd", vec![]));
        config1.add_server(McpServer::stdio("github", "gh-cmd", vec![]));

        let mut config2 = McpConfig::new();
        config2.add_server(McpServer::stdio("neo4j", "new-cmd", vec![])); // Override
        config2.add_server(McpServer::stdio("slack", "slack-cmd", vec![])); // New

        config1.merge(config2);

        assert_eq!(config1.servers.len(), 3);
        assert_eq!(config1.find_server("neo4j").unwrap().command, Some("new-cmd".to_string()));
    }
}
