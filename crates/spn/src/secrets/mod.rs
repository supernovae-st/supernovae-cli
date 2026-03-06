//! Secure secret management for spn CLI.
//!
//! Provides secure storage and handling of API keys and credentials.
//!
//! # Security Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                     SECRETS MANAGEMENT                                      │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────┐│
//! │  │ spn-keyring│  │  secrecy   │  │  zeroize   │  │  dotenvy   │  │  libc  ││
//! │  │ (storage)  │  │ (wrapping) │  │ (mem wipe) │  │ (.env)     │  │ (mlock)││
//! │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └───┬────┘│
//! │        │               │               │               │              │     │
//! │        ▼               ▼               ▼               ▼              ▼     │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐│
//! │  │                         SpnKeyring                                      ││
//! │  │  • OS keychain storage (macOS/Windows/Linux)                            ││
//! │  │  • Zeroizing<String> returns (auto-clear on drop)                       ││
//! │  │  • SecretString wrapping (prevent accidental exposure)                  ││
//! │  │  • Validation per provider format                                       ││
//! │  │  • Migration from env vars                                              ││
//! │  └─────────────────────────────────────────────────────────────────────────┘│
//! │                                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐│
//! │  │                        LockedBuffer / LockedString                      ││
//! │  │  • mlock() - prevents memory from being swapped to disk                 ││
//! │  │  • MADV_DONTDUMP - excludes from core dumps (Linux)                     ││
//! │  │  • Automatic zeroize on drop                                            ││
//! │  │  • Debug/Display redaction                                              ││
//! │  └─────────────────────────────────────────────────────────────────────────┘│
//! │                                                                             │
//! │  Key Resolution Priority:                                                   │
//! │  1. OS Keychain (most secure)                                               │
//! │  2. Environment variable                                                    │
//! │  3. .env file (via dotenvy)                                                 │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use spn::secrets::{SpnKeyring, resolve_api_key};
//!
//! // Store a key in OS keychain
//! SpnKeyring::set("anthropic", "sk-ant-...")?;
//!
//! // Retrieve with auto-zeroize on drop
//! let key = SpnKeyring::get("anthropic")?;
//! // key is Zeroizing<String> - cleared when dropped
//!
//! // Or get as SecretString for API calls
//! let secret = SpnKeyring::get_secret("anthropic")?;
//! use secrecy::ExposeSecret;
//! let exposed = secret.expose_secret();
//! // Both are automatically zeroized when dropped
//!
//! // Resolve from any source (keychain, env, .env)
//! if let Some((key, source)) = resolve_api_key("anthropic") {
//!     println!("Found key from {:?}", source);
//! }
//! ```
//!
//! # Security Best Practices
//!
//! 1. **Prefer keychain** - Use `spn provider migrate` to move keys from .env
//! 2. **Use SecretString** - Pass to external APIs via `get_secret()`
//! 3. **Don't log keys** - Use `mask_api_key()` for display
//! 4. **Validate early** - Keys are validated before storage

mod env_storage;
mod keyring;
pub mod memory;
mod storage;
mod types;
mod wizard;

// Core keyring exports (from spn_keyring via keyring.rs)
pub use keyring::{
    has_any_keys, mask_api_key, migrate_env_to_keyring, resolve_api_key, security_audit,
    validate_key_format, KeyringError, MigrationReport, SpnKeyring,
};

// Storage exports
pub use env_storage::{is_gitignored, store_in_dotenv, store_in_global};
pub use storage::{global_secrets_path, project_env_path, StorageBackend};

// Type exports (extended SecretSource with Inline variant, backward-compatible constants)
pub use types::{
    provider_env_var, SecretSource, MCP_SECRET_TYPES, SUPPORTED_PROVIDERS,
};

// Memory protection exports
pub use memory::{mlock_available, mlock_limit};

// Wizard exports
pub use wizard::{run_quick_setup, run_wizard};

// Re-export core types from spn_keyring/spn_core for convenience
pub use types::{
    find_provider, llm_provider_ids, mask_key, mcp_provider_ids, Provider, ProviderCategory,
    ValidationResult, KNOWN_PROVIDERS,
};
