//! Ollama backend for `SuperNovae` model management.
//!
//! This crate provides the `ModelBackend` trait and an Ollama implementation
//! for managing local LLM models.
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
//! │  │ • is_running(), start(), stop()                                      │   │
//! │  │ • list_models(), pull(), delete()                                    │   │
//! │  │ • load(), unload(), running_models()                                 │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                          ▲                                                  │
//! │                          │ implements                                       │
//! │                          │                                                  │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ OllamaBackend                                                        │   │
//! │  │ • HTTP client for Ollama REST API                                    │   │
//! │  │ • Streaming pull with progress                                       │   │
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
//! # Example
//!
//! ```rust,ignore
//! use spn_ollama::{OllamaBackend, ModelBackend};
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

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod backend;
mod client;
mod ollama;

// Re-export the main types
pub use backend::{
    BoxedBackend, BoxedProgressCallback, DynModelBackend, ModelBackend, ProgressCallback,
};
pub use client::{OllamaClient, DEFAULT_ENDPOINT};
pub use ollama::OllamaBackend;

// Re-export spn-core types for convenience
pub use spn_core::{BackendError, GpuInfo, LoadConfig, ModelInfo, PullProgress, RunningModel};

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
