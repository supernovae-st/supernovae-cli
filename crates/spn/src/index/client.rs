//! Sparse index client for SuperNovae registry.
//!
//! Fetches package metadata from the sparse index using HTTP or local files.
//!
//! TODO(v0.16): Integrate cache management and local index support

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use thiserror::Error;

use super::types::{IndexEntry, PackageScope};

/// Errors that can occur when accessing the index.
#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("Package not found in index: {0}")]
    PackageNotFound(String),

    #[error("Failed to fetch index: {0}")]
    FetchError(String),

    #[error("Failed to parse index entry: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("No versions available for: {0}")]
    NoVersions(String),
}

/// Registry configuration.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Index URL (HTTPS or file://).
    pub index_url: String,

    /// Download URL for tarballs.
    pub download_url: String,

    /// Local cache directory.
    pub cache_dir: PathBuf,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            index_url:
                "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/index"
                    .to_string(),
            download_url:
                "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/releases"
                    .to_string(),
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("spn")
                .join("registry"),
        }
    }
}

impl RegistryConfig {
    /// Create config for a local file:// registry (for testing).
    pub fn local<P: AsRef<Path>>(index_path: P, releases_path: P) -> Self {
        // Use a cache dir under the index path for isolation
        let cache_dir = index_path
            .as_ref()
            .parent()
            .map(|p| p.join(".spn_cache"))
            .unwrap_or_else(|| PathBuf::from(".spn/cache"));
        Self {
            index_url: format!("file://{}", index_path.as_ref().display()),
            download_url: format!("file://{}", releases_path.as_ref().display()),
            cache_dir,
        }
    }

    /// Check if this is a local file:// URL.
    pub fn is_local(&self) -> bool {
        self.index_url.starts_with("file://")
    }
}

/// Client for fetching package metadata from the sparse index.
pub struct IndexClient {
    config: RegistryConfig,
    http_client: Option<ClientWithMiddleware>,
    /// Cache: package name → Arc<Vec<IndexEntry>> for zero-copy cache hits
    cache: DashMap<String, Arc<Vec<IndexEntry>>>,
}

impl IndexClient {
    /// Create a new index client with default config.
    pub fn new() -> Self {
        Self::with_config(RegistryConfig::default())
    }

    /// Create a new index client with custom config.
    pub fn with_config(config: RegistryConfig) -> Self {
        let http_client = if !config.is_local() {
            // Create retry policy: 3 retries with exponential backoff
            let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
            let retry_middleware = RetryTransientMiddleware::new_with_policy(retry_policy);

            // Build client with retry middleware
            let client = ClientBuilder::new(reqwest::Client::new())
                .with(retry_middleware)
                .build();

            Some(client)
        } else {
            None
        };

        Self {
            config,
            http_client,
            cache: DashMap::new(),
        }
    }

    /// Clear the package cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Fetch all versions of a package from the index.
    ///
    /// Returns `Arc<Vec<IndexEntry>>` for zero-copy cache hits.
    pub async fn fetch_package(&self, name: &str) -> Result<Arc<Vec<IndexEntry>>, IndexError> {
        // Check cache first - Arc clone is O(1)
        if let Some(cached) = self.cache.get(name) {
            return Ok(Arc::clone(&cached));
        }

        let scope = PackageScope::parse(name)
            .ok_or_else(|| IndexError::InvalidPackageName(name.to_string()))?;

        let index_path = scope.index_path();
        let content = self.fetch_index_file(&index_path).await?;

        let entries = Arc::new(self.parse_index_content(&content, name)?);

        // Store in cache
        self.cache.insert(name.to_string(), Arc::clone(&entries));

        Ok(entries)
    }

    /// Fetch the latest non-yanked version of a package.
    pub async fn fetch_latest(&self, name: &str) -> Result<IndexEntry, IndexError> {
        let entries = self.fetch_package(name).await?;

        entries
            .iter()
            .filter(|e| e.is_available())
            .max_by(|a, b| a.semver().ok().cmp(&b.semver().ok()))
            .cloned()
            .ok_or_else(|| IndexError::NoVersions(name.to_string()))
    }

    /// Fetch a specific version of a package.
    pub async fn fetch_version(&self, name: &str, version: &str) -> Result<IndexEntry, IndexError> {
        let entries = self.fetch_package(name).await?;

        entries
            .iter()
            .find(|e| e.version == version)
            .cloned()
            .ok_or_else(|| IndexError::PackageNotFound(format!("{}@{}", name, version)))
    }

    /// Search for packages matching a query (case-insensitive).
    ///
    /// This searches through cached packages only. For comprehensive search,
    /// populate cache by calling fetch_package() on known packages first.
    ///
    /// Returns latest non-yanked version of each matching package.
    pub fn search(&self, query: &str) -> Vec<IndexEntry> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // Search through cached packages
        for entry in self.cache.iter() {
            let package_name = entry.key();

            // Case-insensitive substring match
            if package_name.to_lowercase().contains(&query_lower) {
                // Get latest non-yanked version
                if let Some(latest) = entry
                    .value()
                    .iter()
                    .filter(|e| e.is_available())
                    .max_by(|a, b| a.semver().ok().cmp(&b.semver().ok()))
                {
                    results.push(latest.clone());
                }
            }
        }

        // Sort by relevance: exact match first, then alphabetically
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        results
    }

    /// Get the tarball download URL for a package version.
    pub fn tarball_url(&self, name: &str, version: &str) -> Result<String, IndexError> {
        let scope = PackageScope::parse(name)
            .ok_or_else(|| IndexError::InvalidPackageName(name.to_string()))?;

        let index_path = scope.index_path();
        Ok(format!(
            "{}/{}/{}.tar.gz",
            self.config.download_url, index_path, version
        ))
    }

    /// Fetch the raw index file content.
    async fn fetch_index_file(&self, index_path: &str) -> Result<String, IndexError> {
        if self.config.is_local() {
            self.fetch_local(index_path)
        } else {
            self.fetch_http(index_path).await
        }
    }

    /// Fetch from local file system.
    fn fetch_local(&self, index_path: &str) -> Result<String, IndexError> {
        let base = self
            .config
            .index_url
            .strip_prefix("file://")
            .unwrap_or(&self.config.index_url);
        let path = Path::new(base).join(index_path);

        if !path.exists() {
            return Err(IndexError::PackageNotFound(index_path.to_string()));
        }

        std::fs::read_to_string(&path).map_err(IndexError::IoError)
    }

    /// Fetch from HTTP.
    async fn fetch_http(&self, index_path: &str) -> Result<String, IndexError> {
        let url = format!("{}/{}", self.config.index_url, index_path);

        let client = self
            .http_client
            .as_ref()
            .ok_or_else(|| IndexError::HttpError("HTTP client not initialized".to_string()))?;

        let response = client
            .get(&url)
            .header("User-Agent", format!("spn/{}", env!("CARGO_PKG_VERSION")))
            .send()
            .await
            .map_err(|e| IndexError::HttpError(e.to_string()))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(IndexError::PackageNotFound(index_path.to_string()));
        }

        if !response.status().is_success() {
            return Err(IndexError::HttpError(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        response
            .text()
            .await
            .map_err(|e| IndexError::HttpError(e.to_string()))
    }

    /// Parse NDJSON index content into entries.
    fn parse_index_content(
        &self,
        content: &str,
        name: &str,
    ) -> Result<Vec<IndexEntry>, IndexError> {
        let mut entries = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let entry: IndexEntry = serde_json::from_str(line)?;

            // Validate name matches (paranoia check)
            if entry.name != name {
                continue;
            }

            entries.push(entry);
        }

        if entries.is_empty() {
            return Err(IndexError::PackageNotFound(name.to_string()));
        }

        Ok(entries)
    }

    /// Get the cache path for an index entry.
    pub fn cache_path(&self, name: &str) -> PathBuf {
        let scope = PackageScope::parse(name).unwrap();
        self.config.cache_dir.join("index").join(scope.index_path())
    }
}

impl Default for IndexClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_local_index() -> (TempDir, RegistryConfig) {
        let temp = TempDir::new().unwrap();
        let index_dir = temp.path().join("index");
        let releases_dir = temp.path().join("releases");

        // Create test package index
        let pkg_dir = index_dir.join("@w").join("data");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        let mut file = std::fs::File::create(pkg_dir.join("json-transformer")).unwrap();
        writeln!(file, r#"{{"name":"@workflows/data/json-transformer","vers":"1.0.0","deps":[],"cksum":"sha256:test123","features":{{}},"yanked":false}}"#).unwrap();

        let config = RegistryConfig::local(&index_dir, &releases_dir);
        (temp, config)
    }

    #[tokio::test]
    async fn test_fetch_package_local() {
        let (_temp, config) = setup_local_index();
        let client = IndexClient::with_config(config);

        let entries = client
            .fetch_package("@workflows/data/json-transformer")
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "1.0.0");
    }

    #[tokio::test]
    async fn test_fetch_latest() {
        let (temp, config) = setup_local_index();

        // Add second version
        let pkg_path = temp.path().join("index/@w/data/json-transformer");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&pkg_path)
            .unwrap();
        writeln!(file, r#"{{"name":"@workflows/data/json-transformer","vers":"1.1.0","deps":[],"cksum":"sha256:newer","features":{{}},"yanked":false}}"#).unwrap();

        let client = IndexClient::with_config(config);
        let latest = client
            .fetch_latest("@workflows/data/json-transformer")
            .await
            .unwrap();
        assert_eq!(latest.version, "1.1.0");
    }

    #[tokio::test]
    async fn test_fetch_specific_version() {
        let (temp, config) = setup_local_index();

        // Add second version
        let pkg_path = temp.path().join("index/@w/data/json-transformer");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&pkg_path)
            .unwrap();
        writeln!(file, r#"{{"name":"@workflows/data/json-transformer","vers":"1.1.0","deps":[],"cksum":"sha256:newer","features":{{}},"yanked":false}}"#).unwrap();

        let client = IndexClient::with_config(config);
        let v1 = client
            .fetch_version("@workflows/data/json-transformer", "1.0.0")
            .await
            .unwrap();
        assert_eq!(v1.version, "1.0.0");
        assert_eq!(v1.cksum, "sha256:test123");
    }

    #[tokio::test]
    async fn test_package_not_found() {
        let (_temp, config) = setup_local_index();
        let client = IndexClient::with_config(config);

        let result = client.fetch_package("@workflows/nonexistent/package").await;
        assert!(matches!(result, Err(IndexError::PackageNotFound(_))));
    }

    #[tokio::test]
    async fn test_invalid_package_name() {
        let client = IndexClient::new();
        let result = client.fetch_package("no-at-sign").await;
        assert!(matches!(result, Err(IndexError::InvalidPackageName(_))));
    }

    #[test]
    fn test_tarball_url() {
        let client = IndexClient::new();
        let url = client
            .tarball_url("@workflows/data/json-transformer", "1.0.0")
            .unwrap();
        assert!(url.contains("@w/data/json-transformer/1.0.0.tar.gz"));
    }

    #[tokio::test]
    async fn test_yanked_version_excluded_from_latest() {
        let (temp, config) = setup_local_index();

        // Add yanked version
        let pkg_path = temp.path().join("index/@w/data/json-transformer");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&pkg_path)
            .unwrap();
        writeln!(file, r#"{{"name":"@workflows/data/json-transformer","vers":"2.0.0","deps":[],"cksum":"sha256:yanked","features":{{}},"yanked":true}}"#).unwrap();

        let client = IndexClient::with_config(config);
        let latest = client
            .fetch_latest("@workflows/data/json-transformer")
            .await
            .unwrap();
        // Should get 1.0.0, not yanked 2.0.0
        assert_eq!(latest.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let (_temp, config) = setup_local_index();
        let client = IndexClient::with_config(config);

        // Initial cache should be empty
        assert_eq!(client.cache_size(), 0);

        // Fetch package - should populate cache
        let _ = client
            .fetch_package("@workflows/data/json-transformer")
            .await
            .unwrap();

        // Cache should now have 1 entry
        assert_eq!(client.cache_size(), 1);

        // Fetch same package again - should use cache
        let start = std::time::Instant::now();
        let _ = client
            .fetch_package("@workflows/data/json-transformer")
            .await
            .unwrap();
        let duration = start.elapsed();

        // Second fetch should be much faster (< 1ms for cached)
        assert!(duration.as_millis() < 10);

        // Clear cache
        client.clear_cache();
        assert_eq!(client.cache_size(), 0);
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let (_temp, config) = setup_local_index();
        let client = IndexClient::with_config(config);

        // Search on empty cache returns empty
        let results = client.search("json");
        assert_eq!(results.len(), 0);

        // Populate cache
        let _ = client
            .fetch_package("@workflows/data/json-transformer")
            .await
            .unwrap();

        // Search should find the package
        let results = client.search("json");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "@workflows/data/json-transformer");

        // Partial match should work
        let results = client.search("transformer");
        assert_eq!(results.len(), 1);

        // Case-insensitive search
        let results = client.search("JSON");
        assert_eq!(results.len(), 1);

        // No match returns empty
        let results = client.search("nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_returns_latest_version() {
        let (temp, config) = setup_local_index();
        let client = IndexClient::with_config(config);

        // Add multiple versions
        let pkg_path = temp.path().join("index/@w/data/json-transformer");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&pkg_path)
            .unwrap();
        writeln!(file, r#"{{"name":"@workflows/data/json-transformer","vers":"1.1.0","deps":[],"cksum":"sha256:newer","features":{{}},"yanked":false}}"#).unwrap();
        writeln!(file, r#"{{"name":"@workflows/data/json-transformer","vers":"1.2.0","deps":[],"cksum":"sha256:newest","features":{{}},"yanked":false}}"#).unwrap();

        // Populate cache
        let _ = client
            .fetch_package("@workflows/data/json-transformer")
            .await
            .unwrap();

        // Search should return only latest version
        let results = client.search("json");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].version, "1.2.0");
    }
}
