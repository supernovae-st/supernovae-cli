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

/// Extract quantization type from a filename.
///
/// Supports: Q2_K, Q3_K_S, Q3_K_M, Q3_K_L, Q4_K_S, Q4_K_M, Q5_K_S, Q5_K_M, Q6_K, Q8_0, F16, F32
///
/// # Example
///
/// ```
/// use spn_native::extract_quantization;
///
/// assert_eq!(extract_quantization("model-q4_k_m.gguf"), Some("Q4_K_M".to_string()));
/// assert_eq!(extract_quantization("model-Q8_0.gguf"), Some("Q8_0".to_string()));
/// assert_eq!(extract_quantization("model.gguf"), None);
/// ```
#[must_use]
pub fn extract_quantization(filename: &str) -> Option<String> {
    let lower = filename.to_lowercase();
    // Order from most specific to least specific (longer patterns first)
    for quant in [
        "q3_k_s", "q3_k_m", "q3_k_l", // Q3 variants
        "q4_k_s", "q4_k_m", // Q4 variants
        "q5_k_s", "q5_k_m", // Q5 variants
        "q2_k", "q6_k", "q8_0", // Single variants
        "f16", "f32", // Float variants
    ] {
        if lower.contains(quant) {
            return Some(quant.to_uppercase());
        }
    }
    None
}
