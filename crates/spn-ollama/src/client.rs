//! HTTP client for Ollama API.
//!
//! Provides low-level HTTP communication with the Ollama REST API.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use spn_core::{BackendError, ModelInfo, PullProgress};
use tracing::{debug, trace};

/// Default Ollama API endpoint.
pub const DEFAULT_ENDPOINT: &str = "http://localhost:11434";

/// Ollama API client.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    endpoint: String,
}

impl OllamaClient {
    /// Create a new Ollama client with the default endpoint.
    #[must_use]
    pub fn new() -> Self {
        Self::with_endpoint(DEFAULT_ENDPOINT)
    }

    /// Create a new Ollama client with a custom endpoint.
    #[must_use]
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();
        debug!(endpoint = %endpoint, "Creating Ollama client");
        Self {
            client: Client::new(),
            endpoint,
        }
    }

    /// Get the endpoint URL.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Check if Ollama is running by pinging the API.
    pub async fn is_running(&self) -> bool {
        let url = format!("{}/api/tags", self.endpoint);
        trace!(url = %url, "Checking if Ollama is running");
        self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .is_ok()
    }

    /// List all installed models.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        let url = format!("{}/api/tags", self.endpoint);
        debug!(url = %url, "Listing models");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BackendError::NetworkError(format!(
                "API returned status {}",
                response.status()
            )));
        }

        let body: ListModelsResponse = response
            .json()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        Ok(body.models.into_iter().map(Into::into).collect())
    }

    /// Get information about a specific model.
    pub async fn model_info(&self, name: &str) -> Result<ModelInfo, BackendError> {
        let url = format!("{}/api/show", self.endpoint);
        debug!(url = %url, model = %name, "Getting model info");

        let response = self
            .client
            .post(&url)
            .json(&ShowRequest { name: name.to_string() })
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(BackendError::ModelNotFound(name.to_string()));
        }

        if !response.status().is_success() {
            return Err(BackendError::NetworkError(format!(
                "API returned status {}",
                response.status()
            )));
        }

        let body: ShowResponse = response
            .json()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        Ok(ModelInfo {
            name: name.to_string(),
            size: body.size.unwrap_or(0),
            quantization: body.details.as_ref().and_then(|d| d.quantization_level.clone()),
            parameters: body.details.as_ref().and_then(|d| d.parameter_size.clone()),
            digest: body.digest,
        })
    }

    /// Pull a model, streaming progress updates.
    pub async fn pull<F>(&self, name: &str, mut on_progress: F) -> Result<(), BackendError>
    where
        F: FnMut(PullProgress),
    {
        let url = format!("{}/api/pull", self.endpoint);
        debug!(url = %url, model = %name, "Pulling model");

        let response = self
            .client
            .post(&url)
            .json(&PullRequest {
                name: name.to_string(),
                stream: true,
            })
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BackendError::NetworkError(format!(
                "API returned status {}",
                response.status()
            )));
        }

        // Stream the response body line by line (NDJSON)
        use futures_util::StreamExt;
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| BackendError::NetworkError(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim();
                if !line.is_empty() {
                    if let Ok(progress) = serde_json::from_str::<PullProgressResponse>(line) {
                        on_progress(PullProgress::new(
                            progress.status.as_deref().unwrap_or(""),
                            progress.completed.unwrap_or(0),
                            progress.total.unwrap_or(0),
                        ));

                        // Check for errors
                        if let Some(error) = progress.error {
                            return Err(BackendError::NetworkError(error));
                        }
                    }
                }
                buffer = buffer[newline_pos + 1..].to_string();
            }
        }

        Ok(())
    }

    /// Delete a model.
    pub async fn delete(&self, name: &str) -> Result<(), BackendError> {
        let url = format!("{}/api/delete", self.endpoint);
        debug!(url = %url, model = %name, "Deleting model");

        let response = self
            .client
            .delete(&url)
            .json(&DeleteRequest { name: name.to_string() })
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(BackendError::ModelNotFound(name.to_string()));
        }

        if !response.status().is_success() {
            return Err(BackendError::NetworkError(format!(
                "API returned status {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// List running models (via /api/ps).
    pub async fn running_models(&self) -> Result<Vec<RunningModelResponse>, BackendError> {
        let url = format!("{}/api/ps", self.endpoint);
        debug!(url = %url, "Listing running models");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BackendError::NetworkError(format!(
                "API returned status {}",
                response.status()
            )));
        }

        let body: PsResponse = response
            .json()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        Ok(body.models)
    }

    /// Generate a completion to warm up/load a model.
    ///
    /// This is the Ollama way to ensure a model is loaded.
    pub async fn generate_warmup(&self, name: &str, keep_alive: Option<&str>) -> Result<(), BackendError> {
        let url = format!("{}/api/generate", self.endpoint);
        debug!(url = %url, model = %name, "Warming up model");

        let mut request = GenerateRequest {
            model: name.to_string(),
            prompt: String::new(),
            stream: false,
            keep_alive: None,
        };

        if let Some(ka) = keep_alive {
            request.keep_alive = Some(ka.to_string());
        }

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(BackendError::ModelNotFound(name.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::NetworkError(format!(
                "API returned status {status}: {text}"
            )));
        }

        Ok(())
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ShowRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ShowResponse {
    #[serde(default)]
    size: Option<u64>,
    digest: Option<String>,
    details: Option<ModelDetails>,
}

#[derive(Debug, Deserialize)]
struct ModelDetails {
    parameter_size: Option<String>,
    quantization_level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListModelsResponse {
    #[serde(default)]
    models: Vec<ListedModel>,
}

#[derive(Debug, Deserialize)]
struct ListedModel {
    name: String,
    size: u64,
    digest: Option<String>,
    details: Option<ModelDetails>,
}

impl From<ListedModel> for ModelInfo {
    fn from(m: ListedModel) -> Self {
        Self {
            name: m.name,
            size: m.size,
            quantization: m.details.as_ref().and_then(|d| d.quantization_level.clone()),
            parameters: m.details.as_ref().and_then(|d| d.parameter_size.clone()),
            digest: m.digest,
        }
    }
}

#[derive(Debug, Serialize)]
struct PullRequest {
    name: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct PullProgressResponse {
    status: Option<String>,
    completed: Option<u64>,
    total: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeleteRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct PsResponse {
    #[serde(default)]
    models: Vec<RunningModelResponse>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunningModelResponse {
    pub name: String,
    pub size: u64,
    pub size_vram: Option<u64>,
    pub digest: Option<String>,
}

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_default_endpoint() {
        let client = OllamaClient::new();
        assert_eq!(client.endpoint(), DEFAULT_ENDPOINT);
    }

    #[test]
    fn test_client_custom_endpoint() {
        let client = OllamaClient::with_endpoint("http://custom:8080");
        assert_eq!(client.endpoint(), "http://custom:8080");
    }

    #[test]
    fn test_listed_model_conversion() {
        let listed = ListedModel {
            name: "llama3.2:7b".to_string(),
            size: 4_000_000_000,
            digest: Some("sha256:abc".to_string()),
            details: Some(ModelDetails {
                parameter_size: Some("7B".to_string()),
                quantization_level: Some("Q4_K_M".to_string()),
            }),
        };

        let info: ModelInfo = listed.into();
        assert_eq!(info.name, "llama3.2:7b");
        assert_eq!(info.size, 4_000_000_000);
        assert_eq!(info.parameters, Some("7B".to_string()));
        assert_eq!(info.quantization, Some("Q4_K_M".to_string()));
    }
}
