//! HTTP client for Ollama API.
//!
//! Provides low-level HTTP communication with the Ollama REST API.

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use spn_core::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, ChatRole, EmbeddingResponse, ModelInfo,
    PullProgress,
};
use tracing::{debug, trace};

use std::time::Duration;

/// Default Ollama API endpoint.
pub const DEFAULT_ENDPOINT: &str = "http://localhost:11434";

/// Default timeout for connection checks.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for regular requests.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Default timeout for model operations (pull, chat, etc.).
pub const DEFAULT_MODEL_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Default maximum retry attempts for transient failures.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default delay between retries.
pub const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Configuration options for the Ollama client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Timeout for connection checks.
    pub connect_timeout: Duration,
    /// Timeout for regular requests (list, info, etc.).
    pub request_timeout: Duration,
    /// Timeout for model operations (pull, chat, embeddings).
    pub model_timeout: Duration,
    /// Maximum retry attempts for transient failures.
    pub max_retries: u32,
    /// Delay between retries (exponentially backed off).
    pub retry_delay: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            model_timeout: DEFAULT_MODEL_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_delay: DEFAULT_RETRY_DELAY,
        }
    }
}

impl ClientConfig {
    /// Create a new config with default timeouts.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the connect timeout.
    #[must_use]
    pub const fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the request timeout.
    #[must_use]
    pub const fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set the model operation timeout.
    #[must_use]
    pub const fn with_model_timeout(mut self, timeout: Duration) -> Self {
        self.model_timeout = timeout;
        self
    }

    /// Set the maximum retry attempts.
    #[must_use]
    pub const fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the retry delay (base delay, exponentially backed off).
    #[must_use]
    pub const fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Disable retries.
    #[must_use]
    pub const fn no_retries(mut self) -> Self {
        self.max_retries = 0;
        self
    }
}

/// Ollama API client.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    endpoint: String,
    config: ClientConfig,
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
        Self::with_config(endpoint, ClientConfig::default())
    }

    /// Create a new Ollama client with custom endpoint and config.
    #[must_use]
    pub fn with_config(endpoint: impl Into<String>, config: ClientConfig) -> Self {
        let endpoint = endpoint.into();
        debug!(endpoint = %endpoint, "Creating Ollama client");
        Self {
            client: Client::new(),
            endpoint,
            config,
        }
    }

    /// Get the client configuration.
    #[must_use]
    pub const fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Get the endpoint URL.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Execute a request with automatic retry on transient failures.
    ///
    /// Uses exponential backoff: delay * 2^attempt
    #[allow(dead_code)] // Infrastructure for future use
    async fn with_retry<F, Fut, T>(&self, operation: F) -> Result<T, BackendError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, BackendError>>,
    {
        let mut last_error = BackendError::NetworkError("No attempts made".to_string());

        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Only retry on transient network errors
                    if !Self::is_retryable(&e) {
                        return Err(e);
                    }

                    last_error = e;

                    if attempt < self.config.max_retries {
                        let delay = self.config.retry_delay * 2u32.saturating_pow(attempt);
                        debug!(
                            attempt = attempt + 1,
                            max_retries = self.config.max_retries,
                            delay_ms = delay.as_millis(),
                            "Retrying after transient failure"
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error)
    }

    /// Check if an error is retryable (transient network issue).
    #[allow(dead_code)] // Used by with_retry
    fn is_retryable(error: &BackendError) -> bool {
        matches!(error, BackendError::NetworkError(msg) if
            msg.contains("timeout") ||
            msg.contains("connection") ||
            msg.contains("Connection") ||
            msg.contains("reset") ||
            msg.contains("timed out")
        )
    }

    /// Check if Ollama is running by pinging the API.
    pub async fn is_running(&self) -> bool {
        let url = format!("{}/api/tags", self.endpoint);
        trace!(url = %url, "Checking if Ollama is running");
        self.client
            .get(&url)
            .timeout(self.config.connect_timeout)
            .send()
            .await
            .is_ok()
    }

    /// List all installed models.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        let url = format!("{}/api/tags", self.endpoint);
        debug!(url = %url, "Listing models");

        let response = self
            .client
            .get(&url)
            .timeout(self.config.request_timeout)
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
    ///
    /// # Errors
    ///
    /// Returns `BackendError::ModelNotFound` if model doesn't exist.
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn model_info(&self, name: &str) -> Result<ModelInfo, BackendError> {
        let url = format!("{}/api/show", self.endpoint);
        debug!(url = %url, model = %name, "Getting model info");

        let response = self
            .client
            .post(&url)
            .json(&ShowRequest {
                name: name.to_string(),
            })
            .timeout(self.config.request_timeout)
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
            quantization: body
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone()),
            parameters: body.details.as_ref().and_then(|d| d.parameter_size.clone()),
            digest: body.digest,
        })
    }

    /// Pull a model, streaming progress updates.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NetworkError` if the download fails.
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
    ///
    /// # Errors
    ///
    /// Returns `BackendError::ModelNotFound` if model doesn't exist.
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn delete(&self, name: &str) -> Result<(), BackendError> {
        let url = format!("{}/api/delete", self.endpoint);
        debug!(url = %url, model = %name, "Deleting model");

        let response = self
            .client
            .delete(&url)
            .json(&DeleteRequest {
                name: name.to_string(),
            })
            .timeout(self.config.request_timeout)
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
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn running_models(&self) -> Result<Vec<RunningModelResponse>, BackendError> {
        let url = format!("{}/api/ps", self.endpoint);
        debug!(url = %url, "Listing running models");

        let response = self
            .client
            .get(&url)
            .timeout(self.config.request_timeout)
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
    ///
    /// # Errors
    ///
    /// Returns `BackendError::ModelNotFound` if model doesn't exist.
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn generate_warmup(
        &self,
        name: &str,
        keep_alive: Option<&str>,
    ) -> Result<(), BackendError> {
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
            .timeout(self.config.model_timeout)
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

    /// Send a chat completion request.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::ModelNotFound` if model doesn't exist.
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> Result<ChatResponse, BackendError> {
        let url = format!("{}/api/chat", self.endpoint);
        debug!(url = %url, model = %model, messages = messages.len(), "Sending chat request");

        let request = ChatRequest {
            model: model.to_string(),
            messages: messages.iter().map(|m| ChatMessageRequest::from(m.clone())).collect(),
            stream: false,
            options: options.map(ChatOptionsRequest::from),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.config.model_timeout)
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(BackendError::ModelNotFound(model.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::NetworkError(format!(
                "API returned status {status}: {text}"
            )));
        }

        let body: ChatResponseBody = response
            .json()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: body.message.role.into(),
                content: body.message.content,
            },
            done: body.done,
            total_duration: body.total_duration,
            eval_count: body.eval_count,
            prompt_eval_count: body.prompt_eval_count,
        })
    }

    /// Stream a chat completion request.
    ///
    /// Calls the callback for each token as it's generated.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::ModelNotFound` if model doesn't exist.
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn chat_stream<F>(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        mut on_token: F,
    ) -> Result<ChatResponse, BackendError>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/api/chat", self.endpoint);
        debug!(url = %url, model = %model, "Streaming chat request");

        let request = ChatRequest {
            model: model.to_string(),
            messages: messages.iter().map(|m| ChatMessageRequest::from(m.clone())).collect(),
            stream: true,
            options: options.map(ChatOptionsRequest::from),
        };

        // Timeout applies to initial connection only; streaming continues until done
        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.config.model_timeout)
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(BackendError::ModelNotFound(model.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::NetworkError(format!(
                "API returned status {status}: {text}"
            )));
        }

        // Stream the response
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut final_response: Option<ChatResponseBody> = None;
        let mut full_content = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| BackendError::NetworkError(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim();
                if !line.is_empty() {
                    if let Ok(body) = serde_json::from_str::<ChatResponseBody>(line) {
                        // Emit token
                        on_token(&body.message.content);
                        full_content.push_str(&body.message.content);

                        if body.done {
                            final_response = Some(body);
                        }
                    }
                }
                buffer = buffer[newline_pos + 1..].to_string();
            }
        }

        // Build final response
        let final_body = final_response.ok_or_else(|| {
            BackendError::NetworkError("Stream ended without completion".to_string())
        })?;

        Ok(ChatResponse {
            message: ChatMessage::assistant(full_content),
            done: true,
            total_duration: final_body.total_duration,
            eval_count: final_body.eval_count,
            prompt_eval_count: final_body.prompt_eval_count,
        })
    }

    /// Generate embeddings for a text.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::ModelNotFound` if model doesn't exist.
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> Result<EmbeddingResponse, BackendError> {
        let url = format!("{}/api/embed", self.endpoint);
        debug!(url = %url, model = %model, "Generating embedding");

        let request = EmbedRequest {
            model: model.to_string(),
            input: input.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.config.model_timeout)
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(BackendError::ModelNotFound(model.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::NetworkError(format!(
                "API returned status {status}: {text}"
            )));
        }

        let body: EmbedResponseBody = response
            .json()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        // Ollama returns embeddings as array of arrays, we take the first one
        let embedding = body
            .embeddings
            .into_iter()
            .next()
            .unwrap_or_default();

        Ok(EmbeddingResponse {
            embedding,
            total_duration: body.total_duration,
            prompt_eval_count: body.prompt_eval_count,
        })
    }

    /// Generate embeddings for multiple texts (batch).
    ///
    /// # Errors
    ///
    /// Returns `BackendError::ModelNotFound` if model doesn't exist.
    /// Returns `BackendError::NetworkError` if the API request fails.
    pub async fn embed_batch(
        &self,
        model: &str,
        inputs: &[&str],
    ) -> Result<Vec<EmbeddingResponse>, BackendError> {
        let url = format!("{}/api/embed", self.endpoint);
        debug!(url = %url, model = %model, count = inputs.len(), "Generating batch embeddings");

        let request = EmbedBatchRequest {
            model: model.to_string(),
            input: inputs.iter().map(|s| (*s).to_string()).collect(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.config.model_timeout)
            .send()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(BackendError::ModelNotFound(model.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::NetworkError(format!(
                "API returned status {status}: {text}"
            )));
        }

        let body: EmbedResponseBody = response
            .json()
            .await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        Ok(body
            .embeddings
            .into_iter()
            .map(|embedding| EmbeddingResponse {
                embedding,
                total_duration: body.total_duration,
                prompt_eval_count: body.prompt_eval_count,
            })
            .collect())
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
            quantization: m
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone()),
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

// ============================================================================
// Chat API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessageRequest>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptionsRequest>,
}

#[derive(Debug, Serialize)]
struct ChatMessageRequest {
    role: String,
    content: String,
}

impl From<ChatMessage> for ChatMessageRequest {
    fn from(msg: ChatMessage) -> Self {
        Self {
            role: msg.role.to_string(),
            content: msg.content,
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatOptionsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
}

impl From<&ChatOptions> for ChatOptionsRequest {
    fn from(opts: &ChatOptions) -> Self {
        Self {
            temperature: opts.temperature,
            top_p: opts.top_p,
            top_k: opts.top_k,
            num_predict: opts.max_tokens,
            stop: opts.stop.clone(),
            seed: opts.seed,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponseBody {
    message: ChatMessageResponse,
    done: bool,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    role: ChatRoleResponse,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ChatRoleResponse {
    System,
    User,
    Assistant,
}

impl From<ChatRoleResponse> for ChatRole {
    fn from(role: ChatRoleResponse) -> Self {
        match role {
            ChatRoleResponse::System => Self::System,
            ChatRoleResponse::User => Self::User,
            ChatRoleResponse::Assistant => Self::Assistant,
        }
    }
}

// ============================================================================
// Embedding API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    input: String,
}

#[derive(Debug, Serialize)]
struct EmbedBatchRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbedResponseBody {
    #[serde(default)]
    embeddings: Vec<Vec<f32>>,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
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
