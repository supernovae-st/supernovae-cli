//! MCP Registry client for package metadata.
//!
//! Fetches MCP server metadata from the SuperNovae registry.json.
//! Falls back to hardcoded aliases for offline/fast access.
//!
//! TODO(v0.16): Integrate with `spn mcp` commands

#![allow(dead_code)]

use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

// Re-exported via spn-client
use spn_client::Source;

/// Errors for MCP registry operations.
#[derive(Error, Debug)]
pub enum McpRegistryError {
    #[error("Failed to fetch registry: {0}")]
    FetchError(String),

    #[error("Failed to parse registry: {0}")]
    ParseError(String),

    #[error("Package not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for MCP registry operations.
pub type Result<T> = std::result::Result<T, McpRegistryError>;

/// Registry.json v3 format (partial - just what we need for MCP).
#[derive(Debug, Deserialize)]
struct Registry {
    packages: FxHashMap<String, RegistryPackage>,
}

/// Package entry in registry.json.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryPackage {
    /// Package version
    pub version: String,
    /// Package type (mcp, model, workflow, etc.)
    #[serde(rename = "type")]
    pub package_type: String,
    /// Package description
    pub description: Option<String>,
    /// Source definition
    pub source: Option<PackageSource>,
}

/// Source definition from registry.json.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PackageSource {
    /// NPM package
    Npm {
        package: String,
        version: Option<String>,
    },
    /// PyPI package
    Pypi {
        package: String,
        version: Option<String>,
    },
    /// Ollama model
    Ollama { model: String },
    /// Binary download
    Binary {
        platforms: FxHashMap<String, String>,
    },
}

impl PackageSource {
    /// Convert to spn-core Source type.
    pub fn to_core_source(&self) -> Source {
        match self {
            PackageSource::Npm { package, version } => Source::Npm {
                package: package.clone(),
                version: version.clone(),
            },
            PackageSource::Pypi { package, version } => Source::PyPi {
                package: package.clone(),
                version: version.clone(),
            },
            PackageSource::Ollama { model } => Source::Ollama {
                model: model.clone(),
            },
            PackageSource::Binary { platforms } => Source::Binary {
                platforms: platforms
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            },
        }
    }
}

/// MCP package information.
#[derive(Debug, Clone)]
pub struct McpPackage {
    /// Package name (e.g., "@mcp/neo4j")
    pub name: String,
    /// NPM package name (e.g., "@neo4j/mcp-server-neo4j")
    pub npm_package: String,
    /// Version constraint
    pub version: Option<String>,
    /// Description
    pub description: Option<String>,
}

/// Registry configuration.
#[derive(Debug, Clone)]
pub struct McpRegistryConfig {
    /// Registry URL
    pub registry_url: String,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
}

impl Default for McpRegistryConfig {
    fn default() -> Self {
        Self {
            registry_url: "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/registry.json".to_string(),
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("spn")
                .join("registry"),
            cache_ttl: 3600, // 1 hour
        }
    }
}

/// MCP Registry client.
///
/// Fetches MCP package metadata from registry.json with caching.
/// Falls back to hardcoded aliases for offline/fast access.
pub struct McpRegistry {
    config: McpRegistryConfig,
    /// Cached MCP packages: short name → McpPackage
    cache: Arc<RwLock<Option<FxHashMap<String, McpPackage>>>>,
    /// HTTP client
    http_client: reqwest::Client,
}

impl McpRegistry {
    /// Create a new MCP registry client.
    pub fn new() -> Self {
        Self::with_config(McpRegistryConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: McpRegistryConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(None)),
            http_client: reqwest::Client::new(),
        }
    }

    /// Resolve a short name to NPM package.
    ///
    /// Returns the NPM package name for installation.
    /// Tries registry first, falls back to hardcoded aliases.
    pub async fn resolve(&self, name: &str) -> String {
        // Try registry first
        if let Some(pkg) = self.get_package(name).await {
            return pkg.npm_package;
        }

        // Fall back to hardcoded aliases
        super::npm::mcp_aliases()
            .get(name)
            .map(|s: &&str| s.to_string())
            .unwrap_or_else(|| name.to_string())
    }

    /// Get full package information.
    pub async fn get_package(&self, name: &str) -> Option<McpPackage> {
        // Ensure cache is loaded
        self.ensure_cache().await;

        let cache = self.cache.read().await;
        if let Some(packages) = cache.as_ref() {
            // Try exact match first (for @mcp/name format)
            if let Some(pkg) = packages.get(name) {
                return Some(pkg.clone());
            }

            // Try short name (e.g., "neo4j" → "@mcp/neo4j")
            let full_name = format!("@mcp/{}", name);
            if let Some(pkg) = packages.get(&full_name) {
                return Some(pkg.clone());
            }
        }

        None
    }

    /// List all available MCP packages.
    pub async fn list(&self) -> Vec<McpPackage> {
        self.ensure_cache().await;

        let cache = self.cache.read().await;
        cache
            .as_ref()
            .map(|packages| packages.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Search for MCP packages by query.
    pub async fn search(&self, query: &str) -> Vec<McpPackage> {
        let query_lower = query.to_lowercase();

        self.list()
            .await
            .into_iter()
            .filter(|pkg| {
                pkg.name.to_lowercase().contains(&query_lower)
                    || pkg.npm_package.to_lowercase().contains(&query_lower)
                    || pkg
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Ensure cache is loaded.
    async fn ensure_cache(&self) {
        let cache = self.cache.read().await;
        if cache.is_some() {
            return;
        }
        drop(cache);

        // Try to load from cache file first
        if let Ok(packages) = self.load_from_cache_file() {
            let mut cache = self.cache.write().await;
            *cache = Some(packages);
            return;
        }

        // Fetch from network
        if let Ok(packages) = self.fetch_from_network().await {
            // Save to cache file
            let _ = self.save_to_cache_file(&packages);

            let mut cache = self.cache.write().await;
            *cache = Some(packages);
        }
    }

    /// Fetch from network.
    async fn fetch_from_network(&self) -> Result<FxHashMap<String, McpPackage>> {
        let response = self
            .http_client
            .get(&self.config.registry_url)
            .header("User-Agent", format!("spn/{}", env!("CARGO_PKG_VERSION")))
            .send()
            .await
            .map_err(|e| McpRegistryError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(McpRegistryError::FetchError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| McpRegistryError::FetchError(e.to_string()))?;

        self.parse_registry(&text)
    }

    /// Parse registry.json content.
    fn parse_registry(&self, content: &str) -> Result<FxHashMap<String, McpPackage>> {
        let registry: Registry = serde_json::from_str(content)
            .map_err(|e| McpRegistryError::ParseError(e.to_string()))?;

        let mut packages = FxHashMap::default();

        for (name, pkg) in registry.packages {
            // Only include MCP packages
            if pkg.package_type != "mcp" {
                continue;
            }

            // Extract NPM package from source
            if let Some(PackageSource::Npm { package, version }) = &pkg.source {
                packages.insert(
                    name.clone(),
                    McpPackage {
                        name: name.clone(),
                        npm_package: package.clone(),
                        version: version.clone(),
                        description: pkg.description.clone(),
                    },
                );
            }
        }

        Ok(packages)
    }

    /// Load from cache file.
    fn load_from_cache_file(&self) -> Result<FxHashMap<String, McpPackage>> {
        let cache_path = self.config.cache_dir.join("mcp-registry.json");

        // Check if cache exists and is fresh
        if !cache_path.exists() {
            return Err(McpRegistryError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache file not found",
            )));
        }

        let metadata = std::fs::metadata(&cache_path)?;
        if let Ok(modified) = metadata.modified() {
            let age = modified.elapsed().unwrap_or_default();
            if age.as_secs() > self.config.cache_ttl {
                return Err(McpRegistryError::Io(std::io::Error::other("Cache expired")));
            }
        }

        let content = std::fs::read_to_string(&cache_path)?;
        self.parse_registry(&content)
    }

    /// Save to cache file.
    fn save_to_cache_file(&self, packages: &FxHashMap<String, McpPackage>) -> Result<()> {
        std::fs::create_dir_all(&self.config.cache_dir)?;

        let cache_path = self.config.cache_dir.join("mcp-registry.json");

        // We save the parsed packages as JSON for quick loading
        let content = serde_json::to_string_pretty(
            &packages
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        serde_json::json!({
                            "name": v.name,
                            "npm_package": v.npm_package,
                            "version": v.version,
                            "description": v.description,
                        }),
                    )
                })
                .collect::<FxHashMap<_, _>>(),
        )
        .map_err(|e| McpRegistryError::ParseError(e.to_string()))?;

        std::fs::write(&cache_path, content)?;
        Ok(())
    }

    /// Clear the cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;

        // Also remove cache file
        let cache_path = self.config.cache_dir.join("mcp-registry.json");
        let _ = std::fs::remove_file(cache_path);
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_registry() {
        let registry = McpRegistry::new();

        let content = r#"{
            "version": 3,
            "packages": {
                "@mcp/neo4j": {
                    "version": "1.0.0",
                    "type": "mcp",
                    "description": "Neo4j MCP server",
                    "source": {
                        "type": "npm",
                        "package": "@neo4j/mcp-server-neo4j",
                        "version": "^1.0.0"
                    }
                },
                "@workflows/test": {
                    "version": "1.0.0",
                    "type": "workflow",
                    "description": "Not an MCP package"
                }
            }
        }"#;

        let packages = registry.parse_registry(content).unwrap();

        // Should only include MCP packages
        assert_eq!(packages.len(), 1);

        let neo4j = packages.get("@mcp/neo4j").unwrap();
        assert_eq!(neo4j.npm_package, "@neo4j/mcp-server-neo4j");
        assert_eq!(neo4j.version, Some("^1.0.0".to_string()));
    }

    #[test]
    fn test_package_source_to_core() {
        let npm = PackageSource::Npm {
            package: "@neo4j/mcp-server-neo4j".to_string(),
            version: Some("^1.0.0".to_string()),
        };

        let core = npm.to_core_source();
        assert!(matches!(core, Source::Npm { .. }));

        let ollama = PackageSource::Ollama {
            model: "deepseek-coder:6.7b".to_string(),
        };

        let core = ollama.to_core_source();
        assert!(matches!(core, Source::Ollama { .. }));
    }
}
