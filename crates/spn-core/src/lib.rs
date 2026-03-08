//! spn-core: Core types and validation for the SuperNovae ecosystem.
//!
//! This crate provides:
//! - Provider definitions (13+ LLM and MCP service providers)
//! - Key format validation with detailed error messages
//! - MCP server configuration types
//! - Package registry types
//!
//! # Design Principles
//!
//! - **Zero dependencies**: Pure Rust, fast compilation, WASM-compatible
//! - **Single source of truth**: All provider definitions in one place
//! - **Shared types**: Used by spn-cli, spn-client, spn-keyring, and nika
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

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod backend;
mod backup;
mod mcp;
mod providers;
mod registry;
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
    LoadConfig, ModelInfo, PullProgress, RunningModel,
};

pub use backup::{
    BackupContents, BackupError, BackupInfo, BackupManifest, ComponentVersions, NikaContents,
    NovaNetContents, RestoreInfo, SpnContents,
};
