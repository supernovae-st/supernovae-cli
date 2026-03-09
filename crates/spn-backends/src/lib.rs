//! spn-backends: Backend abstraction layer for SuperNovae model management.
//!
//! This crate provides:
//! - `ModelBackend` trait for local LLM backends (Ollama, llama.cpp, etc.)
//! - `CloudBackend` trait for cloud LLM providers (Anthropic, OpenAI, etc.)
//! - `BackendKind` enum for backend identification
//! - `BackendRegistry` for managing multiple backends
//! - `ModelOrchestrator` for routing requests via @models/ aliases
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  spn-backends                                                               │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ ModelBackend Trait (local models)                                   │   │
//! │  │ ├── OllamaBackend (via spn-ollama)                                  │   │
//! │  │ └── LlamaCppBackend (planned)                                       │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ CloudBackend Trait (cloud providers)                                │   │
//! │  │ ├── AnthropicBackend                                                │   │
//! │  │ ├── OpenAIBackend                                                   │   │
//! │  │ ├── MistralBackend                                                  │   │
//! │  │ ├── GroqBackend                                                     │   │
//! │  │ ├── DeepSeekBackend                                                 │   │
//! │  │ └── GeminiBackend                                                   │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ BackendRegistry                                                     │   │
//! │  │ ├── register<B: CloudBackend>()                                     │   │
//! │  │ ├── get(kind: BackendKind)                                          │   │
//! │  │ └── list_available()                                                │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────────┐   │
//! │  │ ModelOrchestrator                                                   │   │
//! │  │ ├── resolve("@models/claude-sonnet") → AnthropicBackend             │   │
//! │  │ ├── resolve("@models/llama3.2:8b") → OllamaBackend                  │   │
//! │  │ └── chat(alias, messages) → routes to correct backend               │   │
//! │  └─────────────────────────────────────────────────────────────────────┘   │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use spn_backends::{BackendRegistry, BackendKind, ModelOrchestrator};
//! use spn_backends::cloud::AnthropicBackend;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create registry
//!     let mut registry = BackendRegistry::new();
//!
//!     // Register cloud backends
//!     registry.register(AnthropicBackend::new("sk-ant-...")?);
//!
//!     // Create orchestrator with aliases
//!     let orchestrator = ModelOrchestrator::new(registry);
//!
//!     // Use @models/ alias
//!     let response = orchestrator.chat(
//!         "@models/claude-sonnet",
//!         &[ChatMessage::user("Hello!")],
//!         None,
//!     ).await?;
//!
//!     println!("{}", response.content());
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod backend;
mod cloud;
mod error;
mod kind;
mod orchestrator;
mod registry;
mod traits;

// Re-export main types
pub use backend::{
    BoxedBackend, BoxedProgressCallback, BoxedTokenCallback, DynModelBackend, ModelBackend,
    ProgressCallback,
};
pub use cloud::{CloudBackend, DynCloudBackend};
pub use error::BackendsError;
pub use kind::BackendKind;
pub use orchestrator::{ModelAlias, ModelOrchestrator, ModelRef};
pub use registry::BackendRegistry;
pub use traits::UnifiedBackend;

// Re-export spn-core types for convenience
pub use spn_core::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, ChatRole, EmbeddingResponse, GpuInfo,
    LoadConfig, ModelInfo, PullProgress, RunningModel,
};

// Feature-gated cloud backend implementations
#[cfg(feature = "anthropic")]
pub mod anthropic;

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "mistral")]
pub mod mistral;

#[cfg(feature = "groq")]
pub mod groq;

#[cfg(feature = "deepseek")]
pub mod deepseek;

#[cfg(feature = "gemini")]
pub mod gemini;
