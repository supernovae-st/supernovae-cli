//! Model backend trait for local LLM management.
//!
//! This trait defines the interface that all local model backends must implement.
//! Currently supports Ollama, with llama.cpp planned for the future.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  ModelBackend Trait                                                        │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  Lifecycle:                                                                 │
//! │  ├── is_running()    Check if backend server is running                    │
//! │  ├── start()         Start the backend server                              │
//! │  └── stop()          Stop the backend server                               │
//! │                                                                             │
//! │  Model Management:                                                          │
//! │  ├── list_models()   List all installed models                             │
//! │  ├── pull()          Download a model from registry                        │
//! │  ├── delete()        Remove a model                                        │
//! │  └── model_info()    Get detailed model information                        │
//! │                                                                             │
//! │  Runtime:                                                                   │
//! │  ├── load()          Load a model into memory (GPU)                        │
//! │  ├── unload()        Unload a model from memory                            │
//! │  └── running_models() List currently loaded models                         │
//! │                                                                             │
//! │  Inference:                                                                 │
//! │  ├── chat()          Send chat completion request                          │
//! │  ├── chat_stream()   Stream chat with token callback                       │
//! │  ├── embed()         Generate text embedding                               │
//! │  └── embed_batch()   Batch embedding generation                            │
//! │                                                                             │
//! │  System:                                                                    │
//! │  ├── gpu_info()      Get GPU information                                   │
//! │  └── endpoint_url()  Get the API endpoint URL                              │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use spn_core::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, EmbeddingResponse, GpuInfo, LoadConfig,
    ModelInfo, PullProgress, RunningModel,
};
use std::future::Future;
use std::pin::Pin;

/// Progress callback type for pull operations.
pub type ProgressCallback = Box<dyn Fn(PullProgress) + Send + Sync>;

/// Model backend trait for local LLM management.
///
/// All methods are async to support non-blocking I/O.
/// Implementations must be `Send + Sync` for use across threads.
///
/// # Example Implementation
///
/// See the `OllamaBackend` implementation for a complete reference.
/// The trait requires implementing async methods for model lifecycle management.
pub trait ModelBackend: Send + Sync {
    /// Unique identifier for this backend (e.g., "ollama", "llama-cpp").
    fn id(&self) -> &'static str;

    /// Human-readable name of the backend.
    fn name(&self) -> &'static str {
        self.id()
    }

    /// Check if the backend server is running.
    fn is_running(&self) -> impl Future<Output = bool> + Send;

    /// Start the backend server.
    ///
    /// Returns `Ok(())` if already running or successfully started.
    fn start(&self) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Stop the backend server.
    ///
    /// Returns `Ok(())` if already stopped or successfully stopped.
    fn stop(&self) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// List all installed models.
    fn list_models(&self) -> impl Future<Output = Result<Vec<ModelInfo>, BackendError>> + Send;

    /// Get information about a specific model.
    fn model_info(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<ModelInfo, BackendError>> + Send;

    /// Pull/download a model from the registry.
    ///
    /// The progress callback is called with updates during download.
    fn pull(
        &self,
        name: &str,
        progress: Option<ProgressCallback>,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Delete a model.
    fn delete(&self, name: &str) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Load a model into memory (with optional GPU configuration).
    fn load(
        &self,
        name: &str,
        config: &LoadConfig,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Unload a model from memory.
    fn unload(&self, name: &str) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// List currently loaded/running models.
    fn running_models(
        &self,
    ) -> impl Future<Output = Result<Vec<RunningModel>, BackendError>> + Send;

    /// Get GPU information (if available).
    fn gpu_info(&self) -> impl Future<Output = Result<Vec<GpuInfo>, BackendError>> + Send;

    /// Get the API endpoint URL.
    fn endpoint_url(&self) -> &str;

    /// Check if a model is installed.
    fn is_installed(&self, name: &str) -> impl Future<Output = bool> + Send {
        async move { self.model_info(name).await.is_ok() }
    }

    /// Check if a model is currently loaded.
    fn is_loaded(&self, name: &str) -> impl Future<Output = bool> + Send {
        async move {
            self.running_models()
                .await
                .map(|models| models.iter().any(|m| m.name == name))
                .unwrap_or(false)
        }
    }

    // =========================================================================
    // Inference Methods
    // =========================================================================

    /// Send a chat completion request.
    fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> impl Future<Output = Result<ChatResponse, BackendError>> + Send;

    /// Generate an embedding for text.
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

    /// Stream a chat completion request.
    ///
    /// Calls the callback for each token as it's generated.
    /// Returns the final complete response when done.
    fn chat_stream<F>(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        on_token: F,
    ) -> impl Future<Output = Result<ChatResponse, BackendError>> + Send
    where
        F: FnMut(&str) + Send;
}

/// Boxed version of the progress callback for trait objects.
pub type BoxedProgressCallback = Box<dyn Fn(PullProgress) + Send + Sync + 'static>;

/// Boxed token callback for streaming chat in trait objects.
pub type BoxedTokenCallback = Box<dyn FnMut(&str) + Send + 'static>;

/// Type alias for a boxed `ModelBackend` trait object.
pub type BoxedBackend = Box<dyn DynModelBackend>;

/// Object-safe version of `ModelBackend` for dynamic dispatch.
///
/// This trait mirrors `ModelBackend` but uses boxed futures for
/// compatibility with trait objects (`Box<dyn DynModelBackend>`).
///
/// # API Signature Differences
///
/// Some method signatures differ from `ModelBackend` for object safety:
///
/// | Method | `ModelBackend` | `DynModelBackend` | Reason |
/// |--------|---------------|-------------------|--------|
/// | `chat` | `&[ChatMessage]` | `Vec<ChatMessage>` | References can't outlive the future |
/// | `chat` | `Option<&ChatOptions>` | `Option<ChatOptions>` | Same - owned for lifetime safety |
/// | `embed_batch` | `&[&str]` | `Vec<String>` | Double references are not object-safe |
/// | `chat_stream` | `F: FnMut` | `BoxedTokenCallback` | Generic callbacks aren't object-safe |
///
/// When using `DynModelBackend`, callers must allocate owned data.
/// For performance-critical code with known backend types, prefer `ModelBackend`.
pub trait DynModelBackend: Send + Sync {
    /// Unique identifier for this backend.
    fn id(&self) -> &'static str;

    /// Human-readable name of the backend.
    fn name(&self) -> &'static str;

    /// Check if the backend server is running.
    fn is_running(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;

    /// Start the backend server.
    fn start(&self) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Stop the backend server.
    fn stop(&self) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// List all installed models.
    fn list_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, BackendError>> + Send + '_>>;

    /// Get information about a specific model.
    fn model_info(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ModelInfo, BackendError>> + Send + '_>>;

    /// Pull/download a model from the registry.
    fn pull(
        &self,
        name: &str,
        progress: Option<BoxedProgressCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Delete a model.
    fn delete(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Load a model into memory.
    fn load(
        &self,
        name: &str,
        config: &LoadConfig,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Unload a model from memory.
    fn unload(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// List currently loaded models.
    fn running_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RunningModel>, BackendError>> + Send + '_>>;

    /// Get GPU information.
    fn gpu_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<GpuInfo>, BackendError>> + Send + '_>>;

    /// Get the API endpoint URL.
    fn endpoint_url(&self) -> &str;

    /// Send a chat completion request.
    fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
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

    /// Stream a chat completion request.
    ///
    /// Calls the callback for each token as it's generated.
    fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
        on_token: BoxedTokenCallback,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, BackendError>> + Send + '_>>;
}
