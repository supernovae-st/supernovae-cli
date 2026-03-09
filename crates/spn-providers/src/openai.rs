//! OpenAI cloud backend implementation.
//!
//! This module provides the `OpenAIBackend` for interacting with OpenAI's
//! GPT models via their Chat Completions API.

use crate::cloud::{CloudBackend, CloudTokenCallback, DynCloudBackend};
use crate::{BackendKind, BackendsError};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use spn_core::{BackendError, ChatMessage, ChatOptions, ChatResponse, ChatRole, EmbeddingResponse};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://api.openai.com";
const DEFAULT_MODEL: &str = "gpt-4o";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// OpenAI backend for GPT models.
#[derive(Clone)]
pub struct OpenAIBackend {
    api_key: String,
    endpoint: String,
    client: Client,
}

impl OpenAIBackend {
    /// Create a new OpenAI backend.
    pub fn new(api_key: impl Into<String>) -> Result<Self, BackendsError> {
        Self::with_endpoint(api_key, DEFAULT_ENDPOINT)
    }

    /// Create with custom endpoint.
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Result<Self, BackendsError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(BackendsError::MissingApiKey(BackendKind::OpenAI));
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

    /// Create from environment.
    pub fn from_env() -> Result<Self, BackendsError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| BackendsError::MissingApiKey(BackendKind::OpenAI))?;
        Self::new(api_key)
    }

    fn chat_url(&self) -> String {
        format!("{}/v1/chat/completions", self.endpoint)
    }

    fn embeddings_url(&self) -> String {
        format!("{}/v1/embeddings", self.endpoint)
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<OpenAIMessage> {
        messages
            .iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    ChatRole::System => "system".to_string(),
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect()
    }

    async fn chat_internal(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> Result<ChatResponse, BackendsError> {
        let options = options.cloned().unwrap_or_default();
        let request = OpenAIRequest {
            model: model.to_string(),
            messages: Self::convert_messages(messages),
            temperature: options.temperature,
            top_p: options.top_p,
            max_tokens: options.max_tokens,
            stop: if options.stop.is_empty() {
                None
            } else {
                Some(options.stop.clone())
            },
            stream: false,
        };

        let response = self
            .client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(BackendsError::RateLimited {
                    backend: BackendKind::OpenAI,
                    retry_after: None,
                });
            }
            return Err(BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: error_body,
                status: Some(status.as_u16()),
            });
        }

        let api_response: OpenAIResponse =
            response.json().await.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::OpenAI,
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
        let request = OpenAIRequest {
            model: model.to_string(),
            messages: Self::convert_messages(messages),
            temperature: options.temperature,
            top_p: options.top_p,
            max_tokens: options.max_tokens,
            stop: if options.stop.is_empty() {
                None
            } else {
                Some(options.stop.clone())
            },
            stream: true,
        };

        let response = self
            .client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: error_body,
                status: Some(status.as_u16()),
            });
        }

        let mut stream = response.bytes_stream();
        let mut full_content = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: e.to_string(),
                status: None,
            })?;

            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(event) = serde_json::from_str::<StreamChunk>(data) {
                        if let Some(choice) = event.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                on_token(content);
                                full_content.push_str(content);
                            }
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
            input: input.to_string(),
        };

        let response = self
            .client
            .post(self.embeddings_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: error_body,
                status: Some(status.as_u16()),
            });
        }

        let api_response: EmbeddingApiResponse =
            response.json().await.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::OpenAI,
                message: format!("Failed to parse response: {e}"),
                status: None,
            })?;

        let embedding = api_response
            .data
            .first()
            .map(|d| d.embedding.clone())
            .unwrap_or_default();

        Ok(EmbeddingResponse {
            embedding,
            total_duration: None,
            prompt_eval_count: api_response.usage.map(|u| u.total_tokens),
        })
    }
}

impl CloudBackend for OpenAIBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::OpenAI
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn available_models(&self) -> impl Future<Output = Result<Vec<String>, BackendError>> + Send {
        async move {
            Ok(vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
                "o3-mini".to_string(),
                "gpt-4-turbo".to_string(),
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
            let mut results = Vec::with_capacity(inputs.len());
            for input in inputs {
                let result = self.embed_internal(&model, &input).await.map_err(|e| {
                    BackendError::BackendSpecific(e.to_string())
                })?;
                results.push(result);
            }
            Ok(results)
        }
    }

    fn default_model(&self) -> &str {
        DEFAULT_MODEL
    }
}

impl DynCloudBackend for OpenAIBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::OpenAI
    }

    fn id(&self) -> &'static str {
        "openai"
    }

    fn name(&self) -> &'static str {
        "OpenAI"
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
            let mut results = Vec::with_capacity(inputs.len());
            for input in inputs {
                let result = self.embed_internal(&model, &input).await.map_err(|e| {
                    BackendError::BackendSpecific(e.to_string())
                })?;
                results.push(result);
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

// API Types
#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: OpenAIMessage,
}

#[derive(Deserialize, Clone)]
struct OpenAIUsage {
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
    input: String,
}

#[derive(Deserialize)]
struct EmbeddingApiResponse {
    data: Vec<EmbeddingData>,
    usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingUsage {
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_backend_creation() {
        let backend = OpenAIBackend::new("sk-test-key");
        assert!(backend.is_ok());
        let backend = backend.unwrap();
        assert_eq!(CloudBackend::kind(&backend), BackendKind::OpenAI);
    }

    #[test]
    fn test_openai_backend_empty_key() {
        let backend = OpenAIBackend::new("");
        assert!(backend.is_err());
    }

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::system("System"),
            ChatMessage::user("User"),
            ChatMessage::assistant("Assistant"),
        ];
        let api_messages = OpenAIBackend::convert_messages(&messages);
        assert_eq!(api_messages.len(), 3);
        assert_eq!(api_messages[0].role, "system");
        assert_eq!(api_messages[1].role, "user");
        assert_eq!(api_messages[2].role, "assistant");
    }
}
