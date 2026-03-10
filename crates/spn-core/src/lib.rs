//! spn-core: Core types and validation for the SuperNovae ecosystem.
//!
//! This crate provides:
//! - Provider definitions (20+ LLM and MCP service providers)
//! - Model definitions (curated models for native inference)
//! - Model storage trait (download-only, no inference)
//! - Key format validation with detailed error messages
//! - MCP server configuration types
//! - Package registry types
//!
//! # Design Principles
//!
//! - **Zero dependencies**: Pure Rust, fast compilation, WASM-compatible
//! - **Single source of truth**: All provider and model definitions in one place
//! - **Shared types**: Used by spn-cli, spn-client, spn-keyring, spn-native, and nika
//!
//! # Example
//!
//! ```
//! use spn_core::{
//!     Provider, ProviderCategory, KNOWN_PROVIDERS,
//!     find_provider, provider_to_env_var,
//!     validate_key_format, mask_key, ValidationResult,
//! };
//!
//! // Find a provider
//! let provider = find_provider("anthropic").unwrap();
//! assert_eq!(provider.env_var, "ANTHROPIC_API_KEY");
//!
//! // Validate a key format
//! match validate_key_format("anthropic", "sk-ant-api03-xxx") {
//!     ValidationResult::Valid => println!("Key is valid!"),
//!     ValidationResult::InvalidPrefix { expected, .. } => {
//!         println!("Key should start with: {}", expected);
//!     }
//!     _ => {}
//! }
//!
//! // Mask a key for display
//! let masked = mask_key("sk-ant-secret-key-12345");
//! assert_eq!(masked, "sk-ant-••••••••");
//! ```
//!
//! # Model Resolution
//!
//! ```
//! use spn_core::{find_model, resolve_model, ResolvedModel, Quantization};
//!
//! // Find a curated model
//! let model = find_model("qwen3:8b").unwrap();
//! assert_eq!(model.model_type.name(), "Text");
//!
//! // Resolve model (curated or HuggingFace passthrough)
//! match resolve_model("hf:bartowski/Qwen3-8B-GGUF") {
//!     Some(ResolvedModel::HuggingFace { repo }) => {
//!         println!("HF repo: {}", repo);
//!     }
//!     Some(ResolvedModel::Curated(model)) => {
//!         println!("Curated: {}", model.name);
//!     }
//!     None => {}
//! }
//!
//! // Auto-select quantization based on RAM
//! use spn_core::auto_select_quantization;
//! let model = find_model("qwen3:8b").unwrap();
//! let quant = auto_select_quantization(model, 16); // 16GB RAM
//! assert_eq!(quant, Quantization::Q8_0);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod backend;
mod backup;
mod mcp;
mod model;
mod providers;
mod registry;
mod storage;
mod validation;

// Re-export everything at crate root for ergonomic imports
pub use providers::{
    find_provider, provider_to_env_var, providers_by_category, Provider, ProviderCategory,
    KNOWN_PROVIDERS,
};

pub use validation::{mask_key, validate_key_format, ValidationResult};

pub use mcp::{McpConfig, McpServer, McpServerType, McpSource};

pub use registry::{PackageManifest, PackageRef, PackageType, Source};

pub use backend::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, ChatRole, EmbeddingResponse, GpuInfo,
    LoadConfig, ModelInfo, PullProgress, Quantization, RunningModel,
};

pub use model::{
    auto_select_quantization, detect_available_ram_gb, find_model, models_by_type, resolve_model,
    KnownModel, ModelArchitecture, ModelType, ResolvedModel, KNOWN_MODELS,
};

pub use storage::{
    default_model_dir, DownloadRequest, DownloadResult, ModelStorage, ProgressCallback,
};

pub use backup::{
    BackupContents, BackupError, BackupInfo, BackupManifest, ComponentVersions, NikaContents,
    NovaNetContents, RestoreInfo, SpnContents,
};
