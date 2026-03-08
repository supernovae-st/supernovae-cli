//! Ollama status collector.
//!
//! Queries the Ollama API to get:
//! - Server running status
//! - Available models
//! - Loaded models and memory usage

use serde::{Deserialize, Serialize};

/// Default Ollama endpoint.
const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

/// Ollama system status.
#[derive(Debug, Clone, Serialize)]
pub struct OllamaStatus {
    /// Whether Ollama server is running.
    pub running: bool,
    /// Endpoint URL.
    pub endpoint: String,
    /// Available models.
    pub models: Vec<ModelInfo>,
    /// Currently loaded model (if any).
    pub loaded_model: Option<String>,
    /// Memory usage in bytes.
    pub memory_used: u64,
    /// Total available memory (estimated).
    pub memory_total: u64,
}

/// Information about a model.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    /// Model name (e.g., "llama3.2:7b").
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// Whether currently loaded in memory.
    pub loaded: bool,
}

/// Response from /api/tags endpoint.
#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<TagModel>,
}

#[derive(Debug, Deserialize)]
struct TagModel {
    name: String,
    size: u64,
}

/// Response from /api/ps endpoint (running models).
#[derive(Debug, Deserialize)]
struct PsResponse {
    models: Vec<PsModel>,
}

#[derive(Debug, Deserialize)]
struct PsModel {
    name: String,
    size: u64,
    size_vram: Option<u64>,
}

/// Collect Ollama status.
pub async fn collect() -> OllamaStatus {
    let endpoint = OLLAMA_ENDPOINT.to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    // Check if server is running
    let running = client
        .get(format!("{endpoint}/api/tags"))
        .send()
        .await
        .is_ok();

    if !running {
        return OllamaStatus {
            running: false,
            endpoint,
            models: vec![],
            loaded_model: None,
            memory_used: 0,
            memory_total: 0,
        };
    }

    // Get available models
    let models_result = client
        .get(format!("{endpoint}/api/tags"))
        .send()
        .await
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                Some(r)
            } else {
                None
            }
        });

    let tags: Option<TagsResponse> = match models_result {
        Some(resp) => resp.json().await.ok(),
        None => None,
    };

    // Get running models
    let ps_result = client
        .get(format!("{endpoint}/api/ps"))
        .send()
        .await
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                Some(r)
            } else {
                None
            }
        });

    let ps: Option<PsResponse> = match ps_result {
        Some(resp) => resp.json().await.ok(),
        None => None,
    };

    let running_models: Vec<String> = ps
        .as_ref()
        .map(|p| p.models.iter().map(|m| m.name.clone()).collect())
        .unwrap_or_default();

    let memory_used: u64 = ps
        .as_ref()
        .map(|p| p.models.iter().map(|m| m.size_vram.unwrap_or(m.size)).sum())
        .unwrap_or(0);

    let loaded_model = running_models.first().cloned();

    let models: Vec<ModelInfo> = tags
        .map(|t| {
            t.models
                .into_iter()
                .map(|m| ModelInfo {
                    loaded: running_models.contains(&m.name),
                    name: m.name,
                    size: m.size,
                })
                .collect()
        })
        .unwrap_or_default();

    // Estimate total memory (16GB default, could query system)
    let memory_total = 16 * 1024 * 1024 * 1024; // 16 GB

    OllamaStatus {
        running: true,
        endpoint,
        models,
        loaded_model,
        memory_used,
        memory_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_collect_when_offline() {
        // When Ollama is not running, should return empty status
        let status = collect().await;
        // Can't assert running status as it depends on actual Ollama
        assert!(status.endpoint.contains("11434"));
    }
}
