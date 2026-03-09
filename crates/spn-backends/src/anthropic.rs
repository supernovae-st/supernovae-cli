//! Anthropic (Claude) cloud backend implementation.
//!
//! This module provides the `AnthropicBackend` for interacting with Anthropic's
//! Claude models via their Messages API.
//!
//! # Example
//!
//! ```rust,ignore
//! use spn_backends::anthropic::AnthropicBackend;
//! use spn_backends::{CloudBackend, ChatMessage};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = AnthropicBackend::new("sk-ant-api03-xxx")?;
//!
//!     let messages = vec![
//!         ChatMessage::user("What is Rust?"),
//!     ];
//!
//!     let response = backend.chat("claude-sonnet-4-20250514", &messages, None).await?;
//!     println!("{}", response.content());
//!
//!     Ok(())
//! }
//! ```

use crate::cloud::{CloudBackend, CloudTokenCallback, DynCloudBackend};
use crate::{BackendKind, BackendsError};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use spn_core::{BackendError, ChatMessage, ChatOptions, ChatResponse, ChatRole, EmbeddingResponse};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Anthropic API version.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default endpoint for Anthropic API.
const DEFAULT_ENDPOINT: &str = "https://api.anthropic.com";

/// Default model to use.
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Maximum tokens default.
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Anthropic backend for Claude models.
#[derive(Clone)]
pub struct AnthropicBackend {
    /// API key.
    api_key: String,
    /// API endpoint.
    endpoint: String,
    /// HTTP client.
    client: Client,
}

impl AnthropicBackend {
    /// Create a new Anthropic backend with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self, BackendsError> {
        Self::with_endpoint(api_key, DEFAULT_ENDPOINT)
    }

    /// Create a new Anthropic backend with a custom endpoint.
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Result<Self, BackendsError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(BackendsError::MissingApiKey(BackendKind::Anthropic));
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
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| BackendsError::MissingApiKey(BackendKind::Anthropic))?;
        Self::new(api_key)
    }

    /// Build the messages API URL.
    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.endpoint)
    }

    /// Convert internal messages to Anthropic format.
    fn convert_messages(
        messages: &[ChatMessage],
    ) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system_prompt = None;
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg.role {
                ChatRole::System => {
                    system_prompt = Some(msg.content.clone());
                }
                ChatRole::User => {
                    api_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: msg.content.clone(),
                    });
                }
                ChatRole::Assistant => {
                    api_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: msg.content.clone(),
                    });
                }
            }
        }

        (system_prompt, api_messages)
    }

    /// Build request body.
    fn build_request(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        stream: bool,
    ) -> AnthropicRequest {
        let (system, messages) = Self::convert_messages(messages);
        let options = options.cloned().unwrap_or_default();

        AnthropicRequest {
            model: model.to_string(),
            messages,
            system,
            max_tokens: options.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            temperature: options.temperature,
            top_p: options.top_p,
            top_k: options.top_k,
            stop_sequences: if options.stop.is_empty() {
                None
            } else {
                Some(options.stop.clone())
            },
            stream,
        }
    }

    /// Make a non-streaming chat request.
    async fn chat_internal(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> Result<ChatResponse, BackendsError> {
        let request = self.build_request(model, messages, options, false);

        let response = self
            .client
            .post(&self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Anthropic,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(BackendsError::RateLimited {
                    backend: BackendKind::Anthropic,
                    retry_after: None,
                });
            }

            return Err(BackendsError::ApiError {
                backend: BackendKind::Anthropic,
                message: error_body,
                status: Some(status.as_u16()),
            });
        }

        let api_response: AnthropicResponse =
            response.json().await.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Anthropic,
                message: format!("Failed to parse response: {e}"),
                status: None,
            })?;

        // Extract content from response
        let content = api_response
            .content
            .into_iter()
            .filter_map(|block| {
                if block.r#type == "text" {
                    block.text
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(ChatResponse {
            message: ChatMessage::assistant(content),
            done: api_response.stop_reason.is_some(),
            total_duration: None,
            eval_count: api_response.usage.map(|u| u.output_tokens),
            prompt_eval_count: api_response.usage.map(|u| u.input_tokens),
        })
    }

    /// Make a streaming chat request.
    async fn chat_stream_internal(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        mut on_token: impl FnMut(&str) + Send,
    ) -> Result<ChatResponse, BackendsError> {
        let request = self.build_request(model, messages, options, true);

        let response = self
            .client
            .post(&self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Anthropic,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(BackendsError::RateLimited {
                    backend: BackendKind::Anthropic,
                    retry_after: None,
                });
            }

            return Err(BackendsError::ApiError {
                backend: BackendKind::Anthropic,
                message: error_body,
                status: Some(status.as_u16()),
            });
        }

        let mut stream = response.bytes_stream();
        let mut full_content = String::new();
        let mut input_tokens = 0u32;
        let mut output_tokens = 0u32;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Anthropic,
                message: e.to_string(),
                status: None,
            })?;

            // Parse SSE events
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }

                    if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                        match event.r#type.as_str() {
                            "content_block_delta" => {
                                if let Some(delta) = event.delta {
                                    if let Some(text) = delta.text {
                                        on_token(&text);
                                        full_content.push_str(&text);
                                    }
                                }
                            }
                            "message_delta" => {
                                if let Some(usage) = event.usage {
                                    output_tokens = usage.output_tokens;
                                }
                            }
                            "message_start" => {
                                if let Some(message) = event.message {
                                    if let Some(usage) = message.usage {
                                        input_tokens = usage.input_tokens;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(ChatResponse {
            message: ChatMessage::assistant(full_content),
            done: true,
            total_duration: None,
            eval_count: Some(output_tokens),
            prompt_eval_count: Some(input_tokens),
        })
    }
}

impl CloudBackend for AnthropicBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Anthropic
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn available_models(&self) -> impl Future<Output = Result<Vec<String>, BackendError>> + Send {
        async move {
            // Anthropic doesn't have a models list endpoint, return known models
            Ok(vec![
                "claude-opus-4-20250514".to_string(),
                "claude-sonnet-4-20250514".to_string(),
                "claude-haiku-3-5-20241022".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
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
        _model: &str,
        _input: &str,
    ) -> impl Future<Output = Result<EmbeddingResponse, BackendError>> + Send {
        async move {
            Err(BackendError::BackendSpecific(
                "Anthropic does not support embeddings".to_string(),
            ))
        }
    }

    fn embed_batch(
        &self,
        _model: &str,
        _inputs: &[&str],
    ) -> impl Future<Output = Result<Vec<EmbeddingResponse>, BackendError>> + Send {
        async move {
            Err(BackendError::BackendSpecific(
                "Anthropic does not support embeddings".to_string(),
            ))
        }
    }

    fn supports_embeddings(&self) -> bool {
        false
    }

    fn default_model(&self) -> &str {
        DEFAULT_MODEL
    }
}

// Implement DynCloudBackend for dynamic dispatch
impl DynCloudBackend for AnthropicBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Anthropic
    }

    fn id(&self) -> &'static str {
        "anthropic"
    }

    fn name(&self) -> &'static str {
        "Anthropic"
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
            // Convert boxed callback to regular closure
            let mut callback = on_token;
            self.chat_stream_internal(&model, &messages, options.as_ref(), |t| callback(t))
                .await
                .map_err(|e| BackendError::BackendSpecific(e.to_string()))
        })
    }

    fn embed(
        &self,
        _model: &str,
        _input: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResponse, BackendError>> + Send + '_>> {
        Box::pin(async move {
            Err(BackendError::BackendSpecific(
                "Anthropic does not support embeddings".to_string(),
            ))
        })
    }

    fn embed_batch(
        &self,
        _model: &str,
        _inputs: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResponse>, BackendError>> + Send + '_>>
    {
        Box::pin(async move {
            Err(BackendError::BackendSpecific(
                "Anthropic does not support embeddings".to_string(),
            ))
        })
    }

    fn supports_embeddings(&self) -> bool {
        false
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn default_model(&self) -> &str {
        DEFAULT_MODEL
    }
}

// ============================================================================
// API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    r#type: String,
    text: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct StreamEvent {
    r#type: String,
    #[serde(default)]
    delta: Option<Delta>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    message: Option<MessageStart>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageStart {
    usage: Option<Usage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_backend_creation() {
        let backend = AnthropicBackend::new("sk-ant-test-key");
        assert!(backend.is_ok());

        let backend = backend.unwrap();
        assert_eq!(CloudBackend::kind(&backend), BackendKind::Anthropic);
        assert_eq!(CloudBackend::endpoint(&backend), DEFAULT_ENDPOINT);
    }

    #[test]
    fn test_anthropic_backend_empty_key() {
        let backend = AnthropicBackend::new("");
        assert!(backend.is_err());
    }

    #[test]
    fn test_anthropic_backend_custom_endpoint() {
        let backend = AnthropicBackend::with_endpoint("sk-test", "https://custom.endpoint");
        assert!(backend.is_ok());
        let b = backend.unwrap();
        assert_eq!(CloudBackend::endpoint(&b), "https://custom.endpoint");
    }

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi!"),
        ];

        let (system, api_messages) = AnthropicBackend::convert_messages(&messages);

        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(api_messages.len(), 2);
        assert_eq!(api_messages[0].role, "user");
        assert_eq!(api_messages[1].role, "assistant");
    }

    #[test]
    fn test_default_model() {
        let backend = AnthropicBackend::new("sk-test").unwrap();
        assert_eq!(CloudBackend::default_model(&backend), DEFAULT_MODEL);
    }

    #[test]
    fn test_supports_embeddings() {
        let backend = AnthropicBackend::new("sk-test").unwrap();
        assert!(!CloudBackend::supports_embeddings(&backend));
    }
}
