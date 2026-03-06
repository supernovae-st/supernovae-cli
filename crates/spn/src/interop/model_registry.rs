//! Model Registry client for package metadata.
//!
//! Fetches model metadata from the SuperNovae registry.json.
//!
//! TODO(v0.14): Integrate with `spn model` commands

#![allow(dead_code)]

use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors for model registry operations.
#[derive(Error, Debug)]
pub enum ModelRegistryError {
    #[error("Failed to fetch registry: {0}")]
    FetchError(String),

    #[error("Failed to parse registry: {0}")]
    ParseError(String),

    #[error("Model not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for model registry operations.
pub type Result<T> = std::result::Result<T, ModelRegistryError>;

/// Registry.json v3 format (partial - just what we need for models).
#[derive(Debug, Deserialize)]
struct Registry {
    packages: FxHashMap<String, RegistryPackage>,
}

/// Package entry in registry.json.
#[derive(Debug, Clone, Deserialize)]
struct RegistryPackage {
    version: String,
    #[serde(rename = "type")]
    package_type: String,
    description: Option<String>,
    source: Option<PackageSource>,
}

/// Source definition from registry.json.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum PackageSource {
    Ollama { model: String },
    #[serde(other)]
    Other,
}

/// Model variant information.
#[derive(Debug, Clone, Default)]
pub struct ModelVariant {
    /// Variant name (e.g., "7b", "32b")
    pub name: String,
    /// Ollama model name with tag
    pub ollama: String,
    /// Download size
    pub size: String,
    /// VRAM requirement
    pub vram: String,
    /// Best use case
    pub best_for: String,
}

/// Model package information.
#[derive(Debug, Clone, Default)]
pub struct ModelPackage {
    /// Package name (e.g., "@models/code/deepseek-coder")
    pub name: String,
    /// Ollama model name (e.g., "deepseek-coder")
    pub ollama_model: String,
    /// Description
    pub description: Option<String>,
    /// Category (code, chat, embed, vision, reasoning)
    pub category: String,
    /// Available variants
    pub variants: Vec<ModelVariant>,
    /// Benchmark scores
    pub benchmarks: FxHashMap<String, f64>,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Recommended use cases
    pub recommended_for: Vec<String>,
}

/// Registry configuration.
#[derive(Debug, Clone)]
pub struct ModelRegistryConfig {
    /// Registry URL
    pub registry_url: String,
    /// Package metadata URL (for package.yaml files)
    pub packages_url: String,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
}

impl Default for ModelRegistryConfig {
    fn default() -> Self {
        Self {
            registry_url: "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/registry.json".to_string(),
            packages_url: "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/packages".to_string(),
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("spn")
                .join("registry"),
            cache_ttl: 3600, // 1 hour
        }
    }
}

/// Model Registry client.
///
/// Fetches model package metadata from registry.json with caching.
pub struct ModelRegistry {
    config: ModelRegistryConfig,
    /// Cached model packages: package name → ModelPackage
    cache: Arc<RwLock<Option<FxHashMap<String, ModelPackage>>>>,
    /// HTTP client
    http_client: reqwest::Client,
}

impl ModelRegistry {
    /// Create a new model registry client.
    pub fn new() -> Self {
        Self::with_config(ModelRegistryConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: ModelRegistryConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(None)),
            http_client: reqwest::Client::new(),
        }
    }

    /// Get full package information.
    pub async fn get(&self, name: &str) -> Option<ModelPackage> {
        self.ensure_cache().await;

        let cache = self.cache.read().await;
        if let Some(packages) = cache.as_ref() {
            // Try exact match first
            if let Some(pkg) = packages.get(name) {
                return Some(pkg.clone());
            }

            // Try short name (e.g., "deepseek-coder" → "@models/code/deepseek-coder")
            for (full_name, pkg) in packages.iter() {
                if full_name.ends_with(&format!("/{}", name)) {
                    return Some(pkg.clone());
                }
            }
        }

        None
    }

    /// List all available models.
    pub async fn list(&self) -> Vec<ModelPackage> {
        self.ensure_cache().await;

        let cache = self.cache.read().await;
        cache
            .as_ref()
            .map(|packages| packages.values().cloned().collect())
            .unwrap_or_default()
    }

    /// List models by category.
    pub async fn list_by_category(&self, category: &str) -> Vec<ModelPackage> {
        self.list()
            .await
            .into_iter()
            .filter(|pkg| pkg.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// Search for models by query.
    pub async fn search(&self, query: &str) -> Vec<ModelPackage> {
        let query_lower = query.to_lowercase();

        self.list()
            .await
            .into_iter()
            .filter(|pkg| {
                pkg.name.to_lowercase().contains(&query_lower)
                    || pkg.ollama_model.to_lowercase().contains(&query_lower)
                    || pkg.category.to_lowercase().contains(&query_lower)
                    || pkg
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || pkg
                        .capabilities
                        .iter()
                        .any(|c| c.to_lowercase().contains(&query_lower))
                    || pkg
                        .recommended_for
                        .iter()
                        .any(|r| r.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Recommend models for a use case.
    pub async fn recommend(&self, use_case: Option<&str>) -> Vec<ModelPackage> {
        let models = self.list().await;

        if let Some(case) = use_case {
            let case_lower = case.to_lowercase();

            // Map common use cases to categories/capabilities
            let (category, keywords): (Option<&str>, Vec<&str>) = match case_lower.as_str() {
                "coding" | "code" | "programming" => (Some("code"), vec!["code", "programming"]),
                "chat" | "conversation" | "assistant" => (Some("chat"), vec!["chat", "general"]),
                "embedding" | "embeddings" | "search" | "rag" => {
                    (Some("embed"), vec!["embedding", "search", "rag"])
                }
                "vision" | "image" | "multimodal" => (Some("vision"), vec!["vision", "image"]),
                "reasoning" | "math" | "logic" => {
                    (Some("reasoning"), vec!["reasoning", "math", "logic"])
                }
                _ => (None, vec![case_lower.as_str()]),
            };

            let mut results: Vec<_> = models
                .into_iter()
                .filter(|pkg| {
                    // Check category match
                    if let Some(cat) = category {
                        if pkg.category.eq_ignore_ascii_case(cat) {
                            return true;
                        }
                    }

                    // Check keywords in capabilities and recommended_for
                    for keyword in &keywords {
                        if pkg.capabilities.iter().any(|c| c.to_lowercase().contains(keyword))
                            || pkg
                                .recommended_for
                                .iter()
                                .any(|r| r.to_lowercase().contains(keyword))
                        {
                            return true;
                        }
                    }

                    false
                })
                .collect();

            // Sort by number of matching keywords (relevance)
            results.sort_by(|a, b| {
                let a_score: usize = keywords
                    .iter()
                    .filter(|k| {
                        a.capabilities.iter().any(|c| c.to_lowercase().contains(*k))
                            || a.recommended_for
                                .iter()
                                .any(|r| r.to_lowercase().contains(*k))
                    })
                    .count();
                let b_score: usize = keywords
                    .iter()
                    .filter(|k| {
                        b.capabilities.iter().any(|c| c.to_lowercase().contains(*k))
                            || b.recommended_for
                                .iter()
                                .any(|r| r.to_lowercase().contains(*k))
                    })
                    .count();
                b_score.cmp(&a_score)
            });

            results
        } else {
            // No specific use case - return popular models by category
            let mut by_category: FxHashMap<String, Vec<ModelPackage>> = FxHashMap::default();
            for model in models {
                by_category
                    .entry(model.category.clone())
                    .or_default()
                    .push(model);
            }

            // Return first model from each category
            by_category
                .into_values()
                .filter_map(|mut models| {
                    models.sort_by(|a, b| a.name.cmp(&b.name));
                    models.into_iter().next()
                })
                .collect()
        }
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
    async fn fetch_from_network(&self) -> Result<FxHashMap<String, ModelPackage>> {
        let response = self
            .http_client
            .get(&self.config.registry_url)
            .header("User-Agent", format!("spn/{}", env!("CARGO_PKG_VERSION")))
            .send()
            .await
            .map_err(|e| ModelRegistryError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ModelRegistryError::FetchError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| ModelRegistryError::FetchError(e.to_string()))?;

        self.parse_registry(&text)
    }

    /// Parse registry.json content.
    fn parse_registry(&self, content: &str) -> Result<FxHashMap<String, ModelPackage>> {
        let registry: Registry = serde_json::from_str(content)
            .map_err(|e| ModelRegistryError::ParseError(e.to_string()))?;

        let mut packages = FxHashMap::default();

        for (name, pkg) in registry.packages {
            // Only include model packages
            if pkg.package_type != "model" {
                continue;
            }

            // Extract Ollama model from source
            if let Some(PackageSource::Ollama { model }) = &pkg.source {
                // Extract category from name (e.g., "@models/code/deepseek-coder" → "code")
                let category = name
                    .strip_prefix("@models/")
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("other")
                    .to_string();

                packages.insert(
                    name.clone(),
                    ModelPackage {
                        name: name.clone(),
                        ollama_model: model.clone(),
                        description: pkg.description.clone(),
                        category,
                        // These will be populated from package.yaml if needed
                        variants: vec![],
                        benchmarks: FxHashMap::default(),
                        capabilities: vec![],
                        recommended_for: vec![],
                    },
                );
            }
        }

        Ok(packages)
    }

    /// Load from cache file.
    fn load_from_cache_file(&self) -> Result<FxHashMap<String, ModelPackage>> {
        let cache_path = self.config.cache_dir.join("model-registry.json");

        if !cache_path.exists() {
            return Err(ModelRegistryError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache file not found",
            )));
        }

        let metadata = std::fs::metadata(&cache_path)?;
        if let Ok(modified) = metadata.modified() {
            let age = modified.elapsed().unwrap_or_default();
            if age.as_secs() > self.config.cache_ttl {
                return Err(ModelRegistryError::Io(std::io::Error::other(
                    "Cache expired",
                )));
            }
        }

        let content = std::fs::read_to_string(&cache_path)?;
        self.parse_registry(&content)
    }

    /// Save to cache file.
    fn save_to_cache_file(&self, packages: &FxHashMap<String, ModelPackage>) -> Result<()> {
        std::fs::create_dir_all(&self.config.cache_dir)?;

        let cache_path = self.config.cache_dir.join("model-registry.json");

        // We save a simplified JSON for quick loading
        let content = serde_json::to_string_pretty(
            &packages
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        serde_json::json!({
                            "name": v.name,
                            "ollama_model": v.ollama_model,
                            "description": v.description,
                            "category": v.category,
                        }),
                    )
                })
                .collect::<FxHashMap<_, _>>(),
        )
        .map_err(|e| ModelRegistryError::ParseError(e.to_string()))?;

        std::fs::write(&cache_path, content)?;
        Ok(())
    }

    /// Clear the cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;

        let cache_path = self.config.cache_dir.join("model-registry.json");
        let _ = std::fs::remove_file(cache_path);
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_registry() {
        let registry = ModelRegistry::new();

        let content = r#"{
            "version": 3,
            "packages": {
                "@models/code/deepseek-coder": {
                    "version": "1.0.0",
                    "type": "model",
                    "description": "Best coding model",
                    "source": {
                        "type": "ollama",
                        "model": "deepseek-coder"
                    }
                },
                "@mcp/neo4j": {
                    "version": "1.0.0",
                    "type": "mcp",
                    "description": "Not a model"
                }
            }
        }"#;

        let packages = registry.parse_registry(content).unwrap();

        // Should only include model packages
        assert_eq!(packages.len(), 1);

        let deepseek = packages.get("@models/code/deepseek-coder").unwrap();
        assert_eq!(deepseek.ollama_model, "deepseek-coder");
        assert_eq!(deepseek.category, "code");
    }

    #[test]
    fn test_category_extraction() {
        let registry = ModelRegistry::new();

        let content = r#"{
            "version": 3,
            "packages": {
                "@models/chat/llama3.2": {
                    "version": "1.0.0",
                    "type": "model",
                    "source": { "type": "ollama", "model": "llama3.2" }
                },
                "@models/vision/llava": {
                    "version": "1.0.0",
                    "type": "model",
                    "source": { "type": "ollama", "model": "llava" }
                }
            }
        }"#;

        let packages = registry.parse_registry(content).unwrap();

        assert_eq!(packages.get("@models/chat/llama3.2").unwrap().category, "chat");
        assert_eq!(packages.get("@models/vision/llava").unwrap().category, "vision");
    }
}
