//! Mistral AI cloud backend.
//!
//! Mistral AI provides open-weight models with strong European presence.
//! Uses OpenAI-compatible API format.

use crate::cloud::{CloudBackend, CloudTokenCallback, DynCloudBackend};
use crate::{BackendKind, BackendsError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use spn_core::{BackendError, ChatMessage, ChatOptions, ChatResponse, EmbeddingResponse};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Default endpoint for Mistral API.
const DEFAULT_ENDPOINT: &str = "https://api.mistral.ai/v1";

/// Default model.
const DEFAULT_MODEL: &str = "mistral-large-latest";

/// Request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Mistral AI cloud backend.
#[derive(Clone)]
pub struct MistralBackend {
    api_key: String,
    endpoint: String,
    client: Client,
}

impl MistralBackend {
    /// Create a new Mistral backend with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self, BackendsError> {
        Self::with_endpoint(api_key, DEFAULT_ENDPOINT)
    }

    /// Create a new Mistral backend with a custom endpoint.
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Result<Self, BackendsError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(BackendsError::MissingApiKey(BackendKind::Mistral));
        }

        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| BackendsError::Config(e.to_string()))?;

        Ok(Self {
            api_key,
            endpoint: endpoint.into(),
            client,
        })
    }

    /// Create a new backend from environment variable.
    pub fn from_env() -> Result<Self, BackendsError> {
        let api_key = std::env::var("MISTRAL_API_KEY")
            .map_err(|_| BackendsError::MissingApiKey(BackendKind::Mistral))?;
        Self::new(api_key)
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<MistralMessage> {
        messages.iter().map(MistralMessage::from).collect()
    }

    async fn chat_internal(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> Result<ChatResponse, BackendsError> {
        let options = options.cloned().unwrap_or_default();
        let mistral_messages = Self::convert_messages(messages);

        let request = MistralRequest {
            model: model.to_string(),
            messages: mistral_messages,
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(BackendsError::RateLimited {
                    backend: BackendKind::Mistral,
                    retry_after: None,
                });
            }
            return Err(BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: error_text,
                status: Some(status.as_u16()),
            });
        }

        let api_response: MistralResponse =
            response.json().await.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: format!("Failed to parse response: {e}"),
                status: None,
            })?;

        let content = api_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            message: ChatMessage::assistant(content),
            done: true,
            total_duration: None,
            eval_count: api_response.usage.as_ref().map(|u| u.completion_tokens),
            prompt_eval_count: api_response.usage.as_ref().map(|u| u.prompt_tokens),
        })
    }

    async fn chat_stream_internal(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        mut on_token: impl FnMut(&str) + Send,
    ) -> Result<ChatResponse, BackendsError> {
        let options = options.cloned().unwrap_or_default();
        let mistral_messages = Self::convert_messages(messages);

        let request = MistralRequest {
            model: model.to_string(),
            messages: mistral_messages,
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            stream: true,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(BackendsError::RateLimited {
                    backend: BackendKind::Mistral,
                    retry_after: None,
                });
            }
            return Err(BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: error_text,
                status: Some(status.as_u16()),
            });
        }

        let mut full_content = String::new();
        let body = response.text().await.map_err(|e| BackendsError::ApiError {
            backend: BackendKind::Mistral,
            message: e.to_string(),
            status: None,
        })?;

        for line in body.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            on_token(content);
                            full_content.push_str(content);
                        }
                    }
                }
            }
        }

        Ok(ChatResponse {
            message: ChatMessage::assistant(full_content),
            done: true,
            total_duration: None,
            eval_count: None,
            prompt_eval_count: None,
        })
    }

    async fn embed_internal(
        &self,
        model: &str,
        input: &str,
    ) -> Result<EmbeddingResponse, BackendsError> {
        let request = EmbeddingRequest {
            model: model.to_string(),
            input: vec![input.to_string()],
        };

        let response = self
            .client
            .post(format!("{}/embeddings", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: error_text,
                status: Some(status.as_u16()),
            });
        }

        let api_response: EmbeddingApiResponse =
            response.json().await.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Mistral,
                message: format!("Failed to parse response: {e}"),
                status: None,
            })?;

        // Return first embedding (EmbeddingResponse is per-input)
        let embedding = api_response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .unwrap_or_default();

        Ok(EmbeddingResponse {
            embedding,
            total_duration: None,
            prompt_eval_count: None,
        })
    }
}

impl CloudBackend for MistralBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Mistral
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn available_models(&self) -> impl Future<Output = Result<Vec<String>, BackendError>> + Send {
        async move {
            Ok(vec![
                "mistral-large-latest".to_string(),
                "mistral-small-latest".to_string(),
                "codestral-latest".to_string(),
                "ministral-8b-latest".to_string(),
                "ministral-3b-latest".to_string(),
                "mistral-embed".to_string(),
            ])
        }
    }

    fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> impl Future<Output = Result<ChatResponse, BackendError>> + Send {
        let model = model.to_string();
        let messages = messages.to_vec();
        let options = options.cloned();
        async move {
            self.chat_internal(&model, &messages, options.as_ref())
                .await
                .map_err(|e| BackendError::BackendSpecific(e.to_string()))
        }
    }

    fn chat_stream<F>(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        on_token: F,
    ) -> impl Future<Output = Result<ChatResponse, BackendError>> + Send
    where
        F: FnMut(&str) + Send,
    {
        let model = model.to_string();
        let messages = messages.to_vec();
        let options = options.cloned();
        async move {
            self.chat_stream_internal(&model, &messages, options.as_ref(), on_token)
                .await
                .map_err(|e| BackendError::BackendSpecific(e.to_string()))
        }
    }

    fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> impl Future<Output = Result<EmbeddingResponse, BackendError>> + Send {
        let model = model.to_string();
        let input = input.to_string();
        async move {
            self.embed_internal(&model, &input)
                .await
                .map_err(|e| BackendError::BackendSpecific(e.to_string()))
        }
    }

    fn embed_batch(
        &self,
        model: &str,
        inputs: &[&str],
    ) -> impl Future<Output = Result<Vec<EmbeddingResponse>, BackendError>> + Send {
        let model = model.to_string();
        let inputs: Vec<String> = inputs.iter().map(|s| (*s).to_string()).collect();
        async move {
            let mut results = Vec::new();
            for input in inputs {
                let resp = self
                    .embed_internal(&model, &input)
                    .await
                    .map_err(|e| BackendError::BackendSpecific(e.to_string()))?;
                results.push(resp);
            }
            Ok(results)
        }
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    fn default_model(&self) -> &str {
        DEFAULT_MODEL
    }
}

impl DynCloudBackend for MistralBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Mistral
    }

    fn id(&self) -> &'static str {
        "mistral"
    }

    fn name(&self) -> &'static str {
        "Mistral AI"
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn available_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, BackendError>> + Send + '_>> {
        Box::pin(CloudBackend::available_models(self))
    }

    fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, BackendError>> + Send + '_>> {
        let model = model.to_string();
        Box::pin(async move {
            self.chat_internal(&model, &messages, options.as_ref())
                .await
                .map_err(|e| BackendError::BackendSpecific(e.to_string()))
        })
    }

    fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
        on_token: CloudTokenCallback,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, BackendError>> + Send + '_>> {
        let model = model.to_string();
        Box::pin(async move {
            let mut callback = on_token;
            self.chat_stream_internal(&model, &messages, options.as_ref(), |t| callback(t))
                .await
                .map_err(|e| BackendError::BackendSpecific(e.to_string()))
        })
    }

    fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResponse, BackendError>> + Send + '_>> {
        let model = model.to_string();
        let input = input.to_string();
        Box::pin(async move {
            self.embed_internal(&model, &input)
                .await
                .map_err(|e| BackendError::BackendSpecific(e.to_string()))
        })
    }

    fn embed_batch(
        &self,
        model: &str,
        inputs: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResponse>, BackendError>> + Send + '_>>
    {
        let model = model.to_string();
        Box::pin(async move {
            let mut results = Vec::new();
            for input in inputs {
                let resp = self
                    .embed_internal(&model, &input)
                    .await
                    .map_err(|e| BackendError::BackendSpecific(e.to_string()))?;
                results.push(resp);
            }
            Ok(results)
        })
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn default_model(&self) -> &str {
        DEFAULT_MODEL
    }
}

// === Request/Response types ===

#[derive(Serialize)]
struct MistralRequest {
    model: String,
    messages: Vec<MistralMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct MistralMessage {
    role: String,
    content: String,
}

impl From<&ChatMessage> for MistralMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: msg.role.to_string(),
            content: msg.content.clone(),
        }
    }
}

#[derive(Deserialize)]
struct MistralResponse {
    choices: Vec<Choice>,
    usage: Option<MistralUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: MistralMessage,
}

#[derive(Deserialize, Clone)]
struct MistralUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbeddingApiResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mistral_backend_creation() {
        let backend = MistralBackend::new("test_key_123").unwrap();
        assert_eq!(CloudBackend::kind(&backend), BackendKind::Mistral);
        assert_eq!(CloudBackend::endpoint(&backend), DEFAULT_ENDPOINT);
        assert!(CloudBackend::supports_embeddings(&backend));
        assert_eq!(CloudBackend::default_model(&backend), DEFAULT_MODEL);
    }

    #[test]
    fn test_mistral_backend_empty_key() {
        let result = MistralBackend::new("");
        assert!(result.is_err());
        match result {
            Err(BackendsError::MissingApiKey(kind)) => {
                assert_eq!(kind, BackendKind::Mistral);
            }
            _ => panic!("Expected MissingApiKey error"),
        }
    }

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];
        let mistral_messages = MistralBackend::convert_messages(&messages);

        assert_eq!(mistral_messages.len(), 2);
        assert_eq!(mistral_messages[0].role, "user");
        assert_eq!(mistral_messages[0].content, "Hello");
        assert_eq!(mistral_messages[1].role, "assistant");
        assert_eq!(mistral_messages[1].content, "Hi there!");
    }
}
