//! Native model inference and storage for the SuperNovae ecosystem.
//!
//! This crate provides:
//! - [`HuggingFaceStorage`]: Download models from HuggingFace Hub
//! - [`detect_available_ram_gb`]: Platform-specific RAM detection
//! - [`default_model_dir`]: Default storage location (~/.spn/models)
//! - [`inference::NativeRuntime`]: Local LLM inference via mistral.rs (feature: `inference`)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  spn-native                                                                 │
//! │  ├── HuggingFaceStorage     Download GGUF models from HuggingFace Hub       │
//! │  ├── detect_available_ram_gb()  Platform-specific RAM detection             │
//! │  ├── default_model_dir()        Default storage path (~/.spn/models)        │
//! │  └── NativeRuntime (inference)  mistral.rs inference integration            │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - `progress`: Enable terminal progress bars for downloads
//! - `inference`: Enable local LLM inference via mistral.rs
//! - `native`: Alias for `inference`
//! - `full`: All features
//!
//! # Example: Download
//!
//! ```ignore
//! use spn_native::{HuggingFaceStorage, default_model_dir, detect_available_ram_gb};
//! use spn_core::{find_model, auto_select_quantization, DownloadRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Detect RAM and select quantization
//!     let ram_gb = detect_available_ram_gb();
//!     let model = find_model("qwen3:8b").unwrap();
//!     let quant = auto_select_quantization(model, ram_gb);
//!
//!     // Create storage and download
//!     let storage = HuggingFaceStorage::new(default_model_dir());
//!     let request = DownloadRequest::curated(model).with_quantization(quant);
//!
//!     let result = storage.download(&request, |progress| {
//!         println!("{}: {:.1}%", progress.status, progress.percent());
//!     }).await?;
//!
//!     println!("Downloaded to: {:?}", result.path);
//!     Ok(())
//! }
//! ```
//!
//! # Example: Inference (requires `inference` feature)
//!
//! ```ignore
//! use spn_native::inference::{NativeRuntime, InferenceBackend};
//! use spn_core::{LoadConfig, ChatOptions};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut runtime = NativeRuntime::new();
//!
//!     // Load a downloaded model
//!     runtime.load("~/.spn/models/qwen3-8b-q4_k_m.gguf".into(), LoadConfig::default()).await?;
//!
//!     // Run inference
//!     let response = runtime.infer("What is 2+2?", ChatOptions::default()).await?;
//!     println!("{}", response.message.content);
//!
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
// Allow certain patterns during development
#![allow(clippy::module_inception)]

mod error;
mod platform;
mod storage;

// Feature-gated inference module
pub mod inference;

pub use error::{NativeError, Result};
pub use platform::{default_model_dir, detect_available_ram_gb};
pub use storage::HuggingFaceStorage;

// Re-export inference types at crate root for convenience
pub use inference::{DynInferenceBackend, InferenceBackend, NativeRuntime};

// Re-export core types for convenience
pub use spn_core::{
    auto_select_quantization, find_model, resolve_model, BackendError, ChatOptions, ChatResponse,
    DownloadRequest, DownloadResult, KnownModel, LoadConfig, ModelArchitecture, ModelInfo,
    ModelStorage, ModelType, ProgressCallback, PullProgress, Quantization, ResolvedModel,
};
