//! Native LLM inference module.
//!
//! This module provides local model inference via mistral.rs when the
//! `inference` feature is enabled.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  Inference Module                                                           │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  InferenceBackend (trait)                                                   │
//! │  ├── load(path, config)      Load GGUF model into memory                    │
//! │  ├── unload()                Unload model from memory                       │
//! │  ├── is_loaded()             Check if model is loaded                       │
//! │  ├── model_info()            Get metadata about loaded model                │
//! │  ├── infer(prompt, opts)     Generate response (non-streaming)              │
//! │  └── infer_stream(...)       Generate response (streaming)                  │
//! │                                                                             │
//! │  NativeRuntime (struct)                                                     │
//! │  └── Implements InferenceBackend using mistral.rs                           │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use spn_native::inference::{NativeRuntime, InferenceBackend};
//! use spn_core::{LoadConfig, ChatOptions};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut runtime = NativeRuntime::new();
//!
//!     // Load a GGUF model
//!     let model_path = PathBuf::from("~/.spn/models/qwen3-8b-q4_k_m.gguf");
//!     runtime.load(model_path, LoadConfig::default()).await?;
//!
//!     // Run inference
//!     let response = runtime.infer(
//!         "What is 2+2?",
//!         ChatOptions::default().with_temperature(0.7)
//!     ).await?;
//!
//!     println!("{}", response.content);
//!     Ok(())
//! }
//! ```

mod runtime;
mod traits;

pub use runtime::NativeRuntime;
pub use traits::{DynInferenceBackend, InferenceBackend};

// Re-export types commonly used with inference
pub use spn_core::{ChatOptions, ChatResponse, LoadConfig, ModelInfo};
