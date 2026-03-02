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
//! │  │  keyring   │  │  secrecy   │  │  zeroize   │  │  dotenvy   │  │  libc  ││
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

mod keyring;
pub mod memory;
mod types;

pub use keyring::{
    has_any_keys, mask_api_key, migrate_env_to_keyring, provider_env_var, resolve_api_key,
    security_audit, validate_key_format, KeyringError, MigrationReport, SpnKeyring,
};
pub use memory::{mlock_available, mlock_limit, LockedBuffer, LockedString, MemoryError};
pub use types::{
    mask_key, ProviderKey, SecretSource, SecureBuffer, SecureString, MCP_SECRET_TYPES,
    SUPPORTED_PROVIDERS,
};
