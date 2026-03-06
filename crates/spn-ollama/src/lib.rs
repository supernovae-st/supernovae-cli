//! Ollama backend for `SuperNovae` model management.
//!
//! This crate provides the `ModelBackend` trait and an Ollama implementation
//! for managing local LLM models, chat completions, and embeddings.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  spn-ollama                                                                │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ ModelBackend Trait                                                   │   │
//! │  │                                                                      │   │
//! │  │ Lifecycle:                                                           │   │
//! │  │ • is_running(), start(), stop()                                      │   │
//! │  │                                                                      │   │
//! │  │ Model Management:                                                    │   │
//! │  │ • list_models(), pull(), delete()                                    │   │
//! │  │ • load(), unload(), running_models()                                 │   │
//! │  │                                                                      │   │
//! │  │ Inference:                                                           │   │
//! │  │ • chat(), chat_stream() - Chat completions                           │   │
//! │  │ • embed(), embed_batch() - Text embeddings                           │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                          ▲                                                  │
//! │                          │ implements                                       │
//! │                          │                                                  │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ OllamaBackend                                                        │   │
//! │  │ • HTTP client for Ollama REST API                                    │   │
//! │  │ • Streaming chat with token callback                                 │   │
//! │  │ • Batch embeddings support                                           │   │
//! │  │ • Process management (start/stop)                                    │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                                                                             │
//! │  Future:                                                                    │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ LlamaCppBackend (planned)                                            │   │
//! │  │ • Same trait, different implementation                               │   │
//! │  │ • HTTP server mode (OpenAI-compatible)                               │   │
//! │  │ • Or native FFI via llama-cpp-rs                                     │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example: Model Management
//!
//! ```rust,ignore
//! use spn_ollama::{OllamaBackend, ModelBackend, LoadConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = OllamaBackend::new();
//!
//!     // Check if Ollama is running
//!     if !backend.is_running().await {
//!         backend.start().await?;
//!     }
//!
//!     // List installed models
//!     for model in backend.list_models().await? {
//!         println!("{} ({})", model.name, model.size_human());
//!     }
//!
//!     // Pull a model with progress
//!     backend.pull("llama3.2:7b", Some(Box::new(|p| {
//!         println!("{}", p);
//!     }))).await?;
//!
//!     // Load model into memory
//!     backend.load("llama3.2:7b", &LoadConfig::default()).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Example: Chat Completions
//!
//! ```rust,ignore
//! use spn_ollama::{OllamaBackend, ModelBackend, ChatMessage, ChatOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = OllamaBackend::new();
//!
//!     let messages = vec![
//!         ChatMessage::system("You are a helpful assistant."),
//!         ChatMessage::user("What is Rust?"),
//!     ];
//!
//!     let options = ChatOptions::new()
//!         .with_temperature(0.7)
//!         .with_max_tokens(500);
//!
//!     let response = backend.chat("llama3.2", &messages, Some(&options)).await?;
//!     println!("Assistant: {}", response.content());
//!
//!     if let Some(tps) = response.tokens_per_second() {
//!         println!("Speed: {:.1} tokens/sec", tps);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Example: Embeddings
//!
//! ```rust,ignore
//! use spn_ollama::{OllamaBackend, ModelBackend};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = OllamaBackend::new();
//!
//!     // Single embedding
//!     let embedding = backend.embed("nomic-embed-text", "Hello world").await?;
//!     println!("Dimension: {}", embedding.dimension());
//!
//!     // Batch embeddings
//!     let texts = &["Hello", "World", "Rust"];
//!     let embeddings = backend.embed_batch("nomic-embed-text", texts).await?;
//!
//!     // Calculate similarity
//!     let similarity = embeddings[0].cosine_similarity(&embeddings[1]);
//!     println!("Similarity: {:.4}", similarity);
//!
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod backend;
mod client;
mod ollama;

// Re-export the main types
pub use backend::{
    BoxedBackend, BoxedProgressCallback, BoxedTokenCallback, DynModelBackend, ModelBackend,
    ProgressCallback,
};
pub use client::{
    ClientConfig, OllamaClient, DEFAULT_CONNECT_TIMEOUT, DEFAULT_ENDPOINT, DEFAULT_MODEL_TIMEOUT,
    DEFAULT_REQUEST_TIMEOUT,
};
pub use ollama::OllamaBackend;

// Re-export spn-core types for convenience
pub use spn_core::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, ChatRole, EmbeddingResponse, GpuInfo,
    LoadConfig, ModelInfo, PullProgress, RunningModel,
};

/// Create a default Ollama backend.
///
/// Convenience function equivalent to `OllamaBackend::new()`.
#[must_use]
pub fn default_backend() -> OllamaBackend {
    OllamaBackend::new()
}

/// Create a boxed backend for dynamic dispatch.
///
/// Use this when you need to store backends in a collection or
/// pass them through trait objects.
#[must_use]
pub fn boxed_backend() -> BoxedBackend {
    Box::new(OllamaBackend::new())
}
