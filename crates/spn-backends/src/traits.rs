//! Unified backend trait combining local and cloud backends.
//!
//! This module provides a unified interface that works for both local model
//! backends (Ollama, llama.cpp) and cloud providers (Anthropic, OpenAI, etc.).

use crate::{BackendKind, BackendsError};
use spn_core::{BackendError, ChatMessage, ChatOptions, ChatResponse, EmbeddingResponse};
use std::future::Future;
use std::pin::Pin;

/// Unified backend trait for both local and cloud LLM backends.
///
/// This trait provides a common interface for chat and embedding operations
/// that works regardless of whether the backend is local or cloud-based.
///
/// For local-only operations (pull, load, unload), use `ModelBackend` directly.
/// For cloud-specific operations, use `CloudBackend` directly.
pub trait UnifiedBackend: Send + Sync {
    /// Get the backend kind.
    fn kind(&self) -> BackendKind;

    /// Get the backend identifier.
    fn id(&self) -> &'static str {
        self.kind().id()
    }

    /// Get the backend name.
    fn name(&self) -> &'static str {
        self.kind().name()
    }

    /// Check if the backend is available and ready to use.
    fn is_available(&self) -> impl Future<Output = bool> + Send;

    /// Send a chat completion request.
    fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> impl Future<Output = Result<ChatResponse, BackendsError>> + Send;

    /// Generate an embedding for text.
    fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> impl Future<Output = Result<EmbeddingResponse, BackendsError>> + Send;

    /// Check if this backend supports embeddings.
    fn supports_embeddings(&self) -> bool {
        true
    }

    /// Check if this backend supports streaming.
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Get the default model for this backend.
    fn default_model(&self) -> &str;
}

/// Object-safe version of `UnifiedBackend`.
pub trait DynUnifiedBackend: Send + Sync {
    /// Get the backend kind.
    fn kind(&self) -> BackendKind;

    /// Get the backend identifier.
    fn id(&self) -> &'static str;

    /// Get the backend name.
    fn name(&self) -> &'static str;

    /// Check if the backend is available.
    fn is_available(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;

    /// Send a chat completion request.
    fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, BackendsError>> + Send + '_>>;

    /// Generate an embedding.
    fn embed(
        &self,
        model: &str,
        input: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResponse, BackendsError>> + Send + '_>>;

    /// Check if this backend supports embeddings.
    fn supports_embeddings(&self) -> bool;

    /// Check if this backend supports streaming.
    fn supports_streaming(&self) -> bool;

    /// Get the default model.
    fn default_model(&self) -> &str;
}

/// Type alias for a boxed unified backend.
pub type BoxedUnifiedBackend = Box<dyn DynUnifiedBackend>;
