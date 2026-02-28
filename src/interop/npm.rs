//! npm integration for MCP servers.
//!
//! Proxies MCP server installation via npm/npx.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

use thiserror::Error;

/// Known MCP server aliases.
/// Maps short names to npm packages.
pub fn mcp_aliases() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("neo4j", "@neo4j/mcp-server-neo4j"),
        ("filesystem", "@anthropic/mcp-server-filesystem"),
        ("github", "@anthropic/mcp-server-github"),
        ("postgres", "@anthropic/mcp-server-postgres"),
        ("sqlite", "@anthropic/mcp-server-sqlite"),
        ("memory", "@anthropic/mcp-server-memory"),
        ("puppeteer", "@anthropic/mcp-server-puppeteer"),
        ("brave-search", "@anthropic/mcp-server-brave-search"),
        ("google-maps", "@anthropic/mcp-server-google-maps"),
        ("fetch", "@anthropic/mcp-server-fetch"),
        (
            "sequential-thinking",
            "@anthropic/mcp-server-sequential-thinking",
        ),
    ])
}

/// Errors that can occur with npm operations.
#[derive(Error, Debug)]
pub enum NpmError {
    #[error("npm not found. Install Node.js: https://nodejs.org")]
    NpmNotFound,

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Installation failed: {0}")]
    InstallFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for npm operations.
pub type Result<T> = std::result::Result<T, NpmError>;

/// npm client for MCP server operations.
pub struct NpmClient {
    /// Global npm directory.
    global_dir: Option<PathBuf>,
}

impl NpmClient {
    /// Create a new npm client.
    pub fn new() -> Self {
        let global_dir = Self::find_global_dir();
        Self { global_dir }
    }

    /// Find the global npm directory.
    fn find_global_dir() -> Option<PathBuf> {
        let output = Command::new("npm").args(["root", "-g"]).output().ok()?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            Some(PathBuf::from(path.trim()))
        } else {
            None
        }
    }

    /// Check if npm is available.
    pub fn is_available(&self) -> bool {
        which::which("npm").is_ok()
    }

    /// Resolve an alias to the full package name.
    pub fn resolve_alias(&self, name: &str) -> String {
        mcp_aliases()
            .get(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.to_string())
    }

    /// Install an MCP server package globally.
    pub fn install(&self, name: &str) -> Result<ExitStatus> {
        if !self.is_available() {
            return Err(NpmError::NpmNotFound);
        }

        let package = self.resolve_alias(name);

        let status = Command::new("npm")
            .args(["install", "-g", &package])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(NpmError::InstallFailed(package));
        }

        Ok(status)
    }

    /// Uninstall an MCP server package.
    pub fn uninstall(&self, name: &str) -> Result<ExitStatus> {
        if !self.is_available() {
            return Err(NpmError::NpmNotFound);
        }

        let package = self.resolve_alias(name);

        let status = Command::new("npm")
            .args(["uninstall", "-g", &package])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Ok(status)
    }

    /// List globally installed MCP servers.
    pub fn list_mcp_servers(&self) -> Result<Vec<String>> {
        if !self.is_available() {
            return Err(NpmError::NpmNotFound);
        }

        let output = Command::new("npm")
            .args(["list", "-g", "--depth=0", "--json"])
            .output()?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        // Parse JSON output
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);

        let mut servers = Vec::new();

        if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
            for (name, _) in deps {
                // Filter for MCP servers
                if name.contains("mcp-server") || name.contains("mcp_server") {
                    servers.push(name.clone());
                }
            }
        }

        servers.sort();
        Ok(servers)
    }

    /// Test an MCP server connection.
    pub fn test_server(&self, name: &str) -> Result<bool> {
        let package = self.resolve_alias(name);

        // Try to run the server with --help or --version
        let output = Command::new("npx").args([&package, "--help"]).output()?;

        Ok(output.status.success())
    }

    /// Get the npx command for running an MCP server.
    pub fn npx_command(&self, name: &str) -> String {
        let package = self.resolve_alias(name);
        format!("npx {}", package)
    }
}

impl Default for NpmClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Install an MCP server via npm.
pub fn install_mcp(name: &str) -> Result<ExitStatus> {
    NpmClient::new().install(name)
}

/// Uninstall an MCP server.
pub fn uninstall_mcp(name: &str) -> Result<ExitStatus> {
    NpmClient::new().uninstall(name)
}

/// List installed MCP servers.
pub fn list_mcp_servers() -> Result<Vec<String>> {
    NpmClient::new().list_mcp_servers()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_aliases() {
        let aliases = mcp_aliases();
        assert_eq!(aliases.get("neo4j"), Some(&"@neo4j/mcp-server-neo4j"));
        assert_eq!(
            aliases.get("filesystem"),
            Some(&"@anthropic/mcp-server-filesystem")
        );
    }

    #[test]
    fn test_resolve_alias() {
        let client = NpmClient::new();

        // Known alias
        assert_eq!(client.resolve_alias("neo4j"), "@neo4j/mcp-server-neo4j");

        // Unknown name (pass through)
        assert_eq!(
            client.resolve_alias("@custom/mcp-server"),
            "@custom/mcp-server"
        );
    }

    #[test]
    fn test_npx_command() {
        let client = NpmClient::new();

        assert_eq!(client.npx_command("neo4j"), "npx @neo4j/mcp-server-neo4j");
    }

    #[test]
    fn test_client_creation() {
        let client = NpmClient::new();
        // npm may or may not be available
        let _ = client.is_available();
    }
}
