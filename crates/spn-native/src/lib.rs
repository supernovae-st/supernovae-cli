//! Native model inference and storage for the SuperNovae ecosystem.
//!
//! This crate provides:
//! - [`HuggingFaceStorage`]: Download models from HuggingFace Hub
//! - [`detect_available_ram_gb`]: Platform-specific RAM detection
//! - [`default_model_dir`]: Default storage location (~/.spn/models)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  spn-native                                                                 │
//! │  ├── HuggingFaceStorage   Download GGUF models from HuggingFace Hub         │
//! │  ├── detect_available_ram_gb()  Platform-specific RAM detection             │
//! │  ├── default_model_dir()        Default storage path (~/.spn/models)        │
//! │  └── (future) NativeRuntime     mistral.rs inference integration            │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
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

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod error;
mod platform;
mod storage;

pub use error::{NativeError, Result};
pub use platform::{default_model_dir, detect_available_ram_gb};
pub use storage::HuggingFaceStorage;

// Re-export core types for convenience
pub use spn_core::{
    auto_select_quantization, find_model, resolve_model, BackendError, DownloadRequest,
    DownloadResult, KnownModel, ModelArchitecture, ModelInfo, ModelStorage, ModelType,
    ProgressCallback, PullProgress, Quantization, ResolvedModel,
};
