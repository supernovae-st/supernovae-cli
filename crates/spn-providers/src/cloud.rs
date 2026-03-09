//! Cloud backend trait for remote LLM providers.
//!
//! This trait defines the interface for cloud-based LLM providers like
//! Anthropic, OpenAI, Mistral, Groq, DeepSeek, and Gemini.
//!
//! # Differences from ModelBackend
//!
//! Cloud backends don't manage local models, so they have a simpler interface:
//! - No `pull()`, `delete()`, `load()`, `unload()` methods
//! - No `running_models()` or `gpu_info()` methods
//! - Just chat and embedding operations

use crate::BackendKind;
use spn_core::{BackendError, ChatMessage, ChatOptions, ChatResponse, EmbeddingResponse};
use std::future::Future;
use std::pin::Pin;

/// Boxed token callback for streaming chat.
pub type CloudTokenCallback = Box<dyn FnMut(&str) + Send + 'static>;

/// Cloud backend trait for remote LLM providers.
///
/// This is a simpler interface than `ModelBackend` since cloud providers
/// don't require local model management (pull, load, unload, etc.).
///
/// # Example Implementation
///
/// ```rust,ignore
/// use spn_backends::{CloudBackend, BackendKind};
/// use spn_core::{ChatMessage, ChatOptions, ChatResponse, BackendError};
///
/// struct MyCloudBackend {
///     api_key: String,
///     endpoint: String,
/// }
///
/// impl CloudBackend for MyCloudBackend {
///     fn kind(&self) -> BackendKind { BackendKind::OpenAI }
///
///     async fn chat(
///         &self,
///         model: &str,
///         messages: &[ChatMessage],
///         options: Option<&ChatOptions>,
///     ) -> Result<ChatResponse, BackendError> {
///         // Make API request...
///         todo!()
///     }
///
///     // ... implement other methods
/// }
/// ```
pub trait CloudBackend: Send + Sync {
    /// Get the backend kind.
    fn kind(&self) -> BackendKind;

    /// Unique identifier for this backend.
    fn id(&self) -> &'static str {
        self.kind().id()
    }

    /// Human-readable name of the backend.
    fn name(&self) -> &'static str {
        self.kind().name()
    }

    /// Get the API endpoint URL.
    fn endpoint(&self) -> &str;

    /// List available models for this provider.
    fn available_models(&self) -> impl Future<Output = Result<Vec<String>, BackendError>> + Send;

    /// Send a chat completion request.
    fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> impl Future<Output = Result<ChatResponse, BackendError>> + Send;

    /// Stream a chat completion request.
    ///
    /// Calls the callback for each token as it's generated.
    fn chat_stream<F>(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        on_token: F,
    ) -> impl Future<Output = Result<ChatResponse, BackendError>> + Send
    where
        F: FnMut(&str) + Send;

    /// Generate an embedding for text.
    ///
    /// Not all cloud providers support embeddings. Returns an error if unsupported.
    fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> impl Future<Output = Result<EmbeddingResponse, BackendError>> + Send;

    /// Generate embeddings for multiple texts (batch).
    fn embed_batch(
        &self,
        model: &str,
        inputs: &[&str],
    ) -> impl Future<Output = Result<Vec<EmbeddingResponse>, BackendError>> + Send;

    /// Check if this provider supports embeddings.
    fn supports_embeddings(&self) -> bool {
        true
    }

    /// Check if this provider supports streaming.
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Get the default model for this provider.
    fn default_model(&self) -> &str;
}

/// Object-safe version of `CloudBackend` for dynamic dispatch.
pub trait DynCloudBackend: Send + Sync {
    /// Get the backend kind.
    fn kind(&self) -> BackendKind;

    /// Unique identifier for this backend.
    fn id(&self) -> &'static str;

    /// Human-readable name of the backend.
    fn name(&self) -> &'static str;

    /// Get the API endpoint URL.
    fn endpoint(&self) -> &str;

    /// List available models for this provider.
    fn available_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, BackendError>> + Send + '_>>;

    /// Send a chat completion request.
    fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, BackendError>> + Send + '_>>;

    /// Stream a chat completion request.
    fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
        on_token: CloudTokenCallback,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, BackendError>> + Send + '_>>;

    /// Generate an embedding for text.
    fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResponse, BackendError>> + Send + '_>>;

    /// Generate embeddings for multiple texts (batch).
    fn embed_batch(
        &self,
        model: &str,
        inputs: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResponse>, BackendError>> + Send + '_>>;

    /// Check if this provider supports embeddings.
    fn supports_embeddings(&self) -> bool;

    /// Check if this provider supports streaming.
    fn supports_streaming(&self) -> bool;

    /// Get the default model for this provider.
    fn default_model(&self) -> &str;
}

/// Type alias for a boxed cloud backend.
pub type BoxedCloudBackend = Box<dyn DynCloudBackend>;

#[cfg(test)]
mod tests {
    use super::*;

    // Test that the traits have the expected methods
    #[test]
    fn test_cloud_backend_methods() {
        // This test just ensures the trait compiles with all required methods
        fn _assert_cloud_backend<T: CloudBackend>() {}
        fn _assert_dyn_cloud_backend<T: DynCloudBackend>() {}
    }
}
