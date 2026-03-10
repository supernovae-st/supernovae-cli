//! Google Gemini cloud backend.
//!
//! Google Gemini provides multimodal models including Gemini Pro and Gemini Flash.
//! Uses Google's Generative AI API (different from OpenAI format).

use crate::cloud::{CloudBackend, CloudTokenCallback, DynCloudBackend};
use crate::{BackendKind, BackendsError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use spn_core::{BackendError, ChatMessage, ChatOptions, ChatResponse, EmbeddingResponse};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Default endpoint for Gemini API.
const DEFAULT_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default model.
const DEFAULT_MODEL: &str = "gemini-2.0-flash";

/// Request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Google Gemini cloud backend.
#[derive(Clone)]
pub struct GeminiBackend {
    api_key: String,
    endpoint: String,
    client: Client,
}

impl GeminiBackend {
    /// Create a new Gemini backend with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self, BackendsError> {
        Self::with_endpoint(api_key, DEFAULT_ENDPOINT)
    }

    /// Create a new Gemini backend with a custom endpoint.
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Result<Self, BackendsError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(BackendsError::MissingApiKey(BackendKind::Gemini));
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
        let api_key = std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .map_err(|_| BackendsError::MissingApiKey(BackendKind::Gemini))?;
        Self::new(api_key)
    }

    /// Convert `ChatMessages` to Gemini Content format.
    fn convert_messages(messages: &[ChatMessage]) -> Vec<GeminiContent> {
        use spn_core::ChatRole;
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    ChatRole::Assistant => "model",
                    // Gemini doesn't have system role, treat as user
                    ChatRole::System | ChatRole::User => "user",
                };
                GeminiContent {
                    role: role.to_string(),
                    parts: vec![GeminiPart {
                        text: msg.content.clone(),
                    }],
                }
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
        let contents = Self::convert_messages(messages);

        let request = GeminiRequest {
            contents,
            generation_config: Some(GenerationConfig {
                temperature: options.temperature,
                max_output_tokens: options.max_tokens,
                top_p: options.top_p,
            }),
        };

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.endpoint, model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            // Parse retry-after header BEFORE consuming body
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(BackendsError::RateLimited {
                    backend: BackendKind::Gemini,
                    retry_after,
                });
            }
            return Err(BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: crate::error::sanitize_api_error(&error_text),
                status: Some(status.as_u16()),
            });
        }

        let api_response: GeminiResponse =
            response.json().await.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: format!("Failed to parse response: {e}"),
                status: None,
            })?;

        let content = api_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        // Extract token counts from usage metadata
        let (prompt_tokens, completion_tokens) = api_response
            .usage_metadata
            .as_ref()
            .map_or((None, None), |u| {
                (Some(u.prompt_token_count), Some(u.candidates_token_count))
            });

        Ok(ChatResponse {
            message: ChatMessage::assistant(content),
            done: true,
            total_duration: None,
            eval_count: completion_tokens,
            prompt_eval_count: prompt_tokens,
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
        let contents = Self::convert_messages(messages);

        let request = GeminiRequest {
            contents,
            generation_config: Some(GenerationConfig {
                temperature: options.temperature,
                max_output_tokens: options.max_tokens,
                top_p: options.top_p,
            }),
        };

        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}",
            self.endpoint, model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            // Parse retry-after header BEFORE consuming body
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            let error_text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(BackendsError::RateLimited {
                    backend: BackendKind::Gemini,
                    retry_after,
                });
            }
            return Err(BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: crate::error::sanitize_api_error(&error_text),
                status: Some(status.as_u16()),
            });
        }

        let mut full_content = String::new();
        let body = response.text().await.map_err(|e| BackendsError::ApiError {
            backend: BackendKind::Gemini,
            message: e.to_string(),
            status: None,
        })?;

        // Gemini returns array of responses when streaming
        if let Ok(responses) = serde_json::from_str::<Vec<GeminiResponse>>(&body) {
            for resp in responses {
                if let Some(candidate) = resp.candidates.first() {
                    if let Some(part) = candidate.content.parts.first() {
                        on_token(&part.text);
                        full_content.push_str(&part.text);
                    }
                }
            }
        } else if let Ok(resp) = serde_json::from_str::<GeminiResponse>(&body) {
            // Single response
            if let Some(candidate) = resp.candidates.first() {
                if let Some(part) = candidate.content.parts.first() {
                    on_token(&part.text);
                    full_content.push_str(&part.text);
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
        let request = EmbedRequest {
            model: format!("models/{model}"),
            content: EmbedContent {
                parts: vec![EmbedPart {
                    text: input.to_string(),
                }],
            },
        };

        let url = format!(
            "{}/models/{}:embedContent?key={}",
            self.endpoint, model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: e.to_string(),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: crate::error::sanitize_api_error(&error_text),
                status: Some(status.as_u16()),
            });
        }

        let embed_response: EmbedResponse =
            response.json().await.map_err(|e| BackendsError::ApiError {
                backend: BackendKind::Gemini,
                message: format!("Failed to parse response: {e}"),
                status: None,
            })?;

        Ok(EmbeddingResponse {
            embedding: embed_response.embedding.values,
            total_duration: None,
            prompt_eval_count: None,
        })
    }
}

impl CloudBackend for GeminiBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Gemini
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn available_models(&self) -> impl Future<Output = Result<Vec<String>, BackendError>> + Send {
        async move {
            Ok(vec![
                "gemini-2.0-flash".to_string(),
                "gemini-2.0-flash-lite".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-1.5-flash-8b".to_string(),
                "text-embedding-004".to_string(),
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
            for input in &inputs {
                let response = self
                    .embed_internal(&model, input)
                    .await
                    .map_err(|e| BackendError::BackendSpecific(e.to_string()))?;
                results.push(response);
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

impl DynCloudBackend for GeminiBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Gemini
    }

    fn id(&self) -> &'static str {
        "gemini"
    }

    fn name(&self) -> &'static str {
        "Google Gemini"
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
            for input in &inputs {
                let response = self
                    .embed_internal(&model, input)
                    .await
                    .map_err(|e| BackendError::BackendSpecific(e.to_string()))?;
                results.push(response);
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

// === Request/Response types (Gemini-specific format) ===

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(rename = "topP", skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize)]
struct Candidate {
    content: GeminiContent,
}

#[derive(Deserialize, Clone)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
}

// Embedding types
#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    content: EmbedContent,
}

#[derive(Serialize)]
struct EmbedContent {
    parts: Vec<EmbedPart>,
}

#[derive(Serialize)]
struct EmbedPart {
    text: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: EmbeddingValues,
}

#[derive(Deserialize)]
struct EmbeddingValues {
    values: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_backend_creation() {
        let backend = GeminiBackend::new("AIza_test_key_123").unwrap();
        assert_eq!(CloudBackend::kind(&backend), BackendKind::Gemini);
        assert_eq!(CloudBackend::endpoint(&backend), DEFAULT_ENDPOINT);
        assert!(CloudBackend::supports_embeddings(&backend));
        assert_eq!(CloudBackend::default_model(&backend), DEFAULT_MODEL);
    }

    #[test]
    fn test_gemini_backend_empty_key() {
        let result = GeminiBackend::new("");
        assert!(result.is_err());
        match result {
            Err(BackendsError::MissingApiKey(kind)) => {
                assert_eq!(kind, BackendKind::Gemini);
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
        let gemini_contents = GeminiBackend::convert_messages(&messages);

        assert_eq!(gemini_contents.len(), 2);
        assert_eq!(gemini_contents[0].role, "user");
        assert_eq!(gemini_contents[0].parts[0].text, "Hello");
        assert_eq!(gemini_contents[1].role, "model"); // assistant -> model
        assert_eq!(gemini_contents[1].parts[0].text, "Hi there!");
    }

    #[test]
    fn test_system_message_conversion() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant"),
            ChatMessage::user("Hello"),
        ];
        let gemini_contents = GeminiBackend::convert_messages(&messages);

        // System message becomes user in Gemini (it doesn't support system role)
        assert_eq!(gemini_contents[0].role, "user");
        assert_eq!(
            gemini_contents[0].parts[0].text,
            "You are a helpful assistant"
        );
    }
}
