//! npm integration for MCP servers.
//!
//! Proxies MCP server installation via npm/npx.
//! Uses the MCP registry for package resolution, with hardcoded fallbacks.
//!
//! TODO(v0.16): Integrate with `spn mcp` commands

#![allow(dead_code)]

use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

use thiserror::Error;

use super::mcp_registry::McpRegistry;

/// Known MCP server aliases (fallback for offline/fast access).
/// Maps short names to npm packages.
/// NOTE: Registry is the source of truth; this is for offline fallback.
pub fn mcp_aliases() -> FxHashMap<&'static str, &'static str> {
    FxHashMap::from_iter([
        // Anthropic official
        ("filesystem", "@modelcontextprotocol/server-filesystem"),
        ("github", "@modelcontextprotocol/server-github"),
        ("postgres", "@modelcontextprotocol/server-postgres"),
        ("sqlite", "@modelcontextprotocol/server-sqlite"),
        ("memory", "@modelcontextprotocol/server-memory"),
        ("puppeteer", "@modelcontextprotocol/server-puppeteer"),
        ("brave-search", "@modelcontextprotocol/server-brave-search"),
        ("google-maps", "@modelcontextprotocol/server-google-maps"),
        ("fetch", "@modelcontextprotocol/server-fetch"),
        ("slack", "@modelcontextprotocol/server-slack"),
        ("gdrive", "@modelcontextprotocol/server-gdrive"),
        ("sentry", "@modelcontextprotocol/server-sentry"),
        ("gitlab", "@modelcontextprotocol/server-gitlab"),
        ("git", "@modelcontextprotocol/server-git"),
        ("everart", "@modelcontextprotocol/server-everart"),
        (
            "aws-kb-retrieval",
            "@modelcontextprotocol/server-aws-kb-retrieval",
        ),
        (
            "sequential-thinking",
            "@modelcontextprotocol/server-sequential-thinking",
        ),
        // Third-party popular
        ("neo4j", "@neo4j/mcp-server-neo4j"),
        ("perplexity", "perplexity-mcp"),
        ("firecrawl", "firecrawl-mcp"),
        ("browserbase", "@browserbasehq/mcp-server-browserbase"),
        ("cloudflare", "@cloudflare/mcp-server-cloudflare"),
        ("stripe", "@stripe/mcp-server-stripe"),
        ("supabase", "@supabase/mcp-server-supabase"),
        ("linear", "@linear/mcp-server-linear"),
        ("notion", "@notionhq/mcp-server-notion"),
        ("airtable", "@airtable/mcp-server-airtable"),
        ("vercel", "@vercel/mcp-server-vercel"),
        ("neon", "@neondatabase/mcp-server-neon"),
        ("planetscale", "@planetscale/mcp-server-planetscale"),
        ("axiom", "@axiomhq/mcp-server-axiom"),
        ("e2b", "@e2b/mcp-server-e2b"),
        ("context7", "context7-mcp"),
        ("exa", "exa-mcp-server"),
        ("tavily", "tavily-mcp"),
        ("qdrant", "@qdrant/mcp-server-qdrant"),
        ("milvus", "@milvus/mcp-server-milvus"),
        ("pinecone", "@pinecone-database/mcp-server-pinecone"),
        ("weaviate", "@weaviate/mcp-server-weaviate"),
        // Developer tools
        ("docker", "mcp-server-docker"),
        ("kubernetes", "mcp-server-kubernetes"),
        ("raygun", "@raygun/mcp-server-raygun"),
        ("saucelabs", "@saucelabs/mcp-server-saucelabs"),
        ("circleci", "@circleci/mcp-server-circleci"),
        // Analytics & monitoring
        ("grafana", "@grafana/mcp-server-grafana"),
        ("datadog", "@datadog/mcp-server-datadog"),
        ("splunk", "@splunk/mcp-server-splunk"),
        ("21st", "@21st-dev/magic-mcp"),
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
    /// MCP registry for package resolution.
    registry: McpRegistry,
}

impl NpmClient {
    /// Create a new npm client.
    pub fn new() -> Self {
        let global_dir = Self::find_global_dir();
        Self {
            global_dir,
            registry: McpRegistry::new(),
        }
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

    /// Resolve an alias to the full package name (sync, uses fallback).
    pub fn resolve_alias(&self, name: &str) -> String {
        mcp_aliases()
            .get(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.to_string())
    }

    /// Resolve an alias using the registry (async, preferred).
    pub async fn resolve_alias_async(&self, name: &str) -> String {
        self.registry.resolve(name).await
    }

    /// Get the MCP registry.
    pub fn registry(&self) -> &McpRegistry {
        &self.registry
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

    /// Get the npx command using registry (async, preferred).
    pub async fn npx_command_async(&self, name: &str) -> String {
        let package = self.resolve_alias_async(name).await;
        format!("npx {}", package)
    }

    /// Install an MCP server using registry resolution (async).
    pub async fn install_async(&self, name: &str) -> Result<ExitStatus> {
        if !self.is_available() {
            return Err(NpmError::NpmNotFound);
        }

        let package = self.resolve_alias_async(name).await;

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

    /// Search MCP servers in registry.
    pub async fn search_mcp(&self, query: &str) -> Vec<super::mcp_registry::McpPackage> {
        self.registry.search(query).await
    }

    /// List all available MCP servers from registry.
    pub async fn list_available_mcp(&self) -> Vec<super::mcp_registry::McpPackage> {
        self.registry.list().await
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
            Some(&"@modelcontextprotocol/server-filesystem")
        );
        // Verify we have 48 aliases
        assert_eq!(aliases.len(), 48);
    }

    #[test]
    fn test_resolve_alias() {
        let client = NpmClient::new();

        // Known alias
        assert_eq!(client.resolve_alias("neo4j"), "@neo4j/mcp-server-neo4j");
        assert_eq!(
            client.resolve_alias("github"),
            "@modelcontextprotocol/server-github"
        );

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
