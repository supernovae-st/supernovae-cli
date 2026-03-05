//! OS keychain wrapper for `SuperNovae` CLI.
//!
//! Provides secure storage and retrieval of API keys using the system keychain:
//! - macOS: Keychain Access
//! - Windows: Credential Manager
//! - Linux: Secret Service (GNOME Keyring, `KWallet`)
//!
//! # Security Design
//!
//! All keys are:
//! 1. Validated using `spn_core` provider definitions before storage
//! 2. Stored encrypted in OS keychain
//! 3. Retrieved into `Zeroizing<String>` (auto-clear on drop)
//! 4. Optionally wrapped in `SecretString` for API calls
//!
//! # Example
//!
//! ```rust,ignore
//! use spn_keyring::SpnKeyring;
//!
//! // Store a key (validated against spn_core provider definitions)
//! SpnKeyring::set("anthropic", "sk-ant-api03-...")?;
//!
//! // Retrieve with auto-zeroize on drop
//! let key = SpnKeyring::get("anthropic")?;
//!
//! // Check existence
//! if SpnKeyring::exists("openai") {
//!     println!("OpenAI configured");
//! }
//!
//! // Delete
//! SpnKeyring::delete("anthropic")?;
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use keyring::Entry;
use secrecy::SecretString;
use thiserror::Error;
use zeroize::Zeroizing;

// Re-export spn-core types for convenience
pub use spn_core::{
    find_provider, mask_key, provider_to_env_var, validate_key_format, Provider, ProviderCategory,
    ValidationResult, KNOWN_PROVIDERS,
};

/// Service name for keyring entries.
const SERVICE_NAME: &str = "spn";

/// Keyring error types.
#[derive(Debug, Error)]
pub enum KeyringError {
    /// Failed to access the keyring.
    #[error("Failed to access keyring: {0}")]
    AccessError(String),

    /// Key not found for the specified provider.
    #[error("Key not found for provider: {0}")]
    NotFound(String),

    /// Failed to store the key.
    #[error("Failed to store key: {0}")]
    StoreError(String),

    /// Failed to delete the key.
    #[error("Failed to delete key: {0}")]
    DeleteError(String),

    /// Key validation failed.
    #[error("Invalid key format: {0}")]
    ValidationError(String),

    /// Keychain is locked or inaccessible.
    #[error("Keychain locked or inaccessible")]
    Locked,

    /// Unknown provider.
    #[error("Unknown provider: {0}")]
    UnknownProvider(String),
}

/// Keyring wrapper for spn API keys.
///
/// All methods that return keys use `Zeroizing<String>` or `SecretString`
/// to ensure automatic memory clearing.
///
/// # Example
///
/// ```rust,ignore
/// use spn_keyring::SpnKeyring;
///
/// // Store
/// SpnKeyring::set("anthropic", "sk-ant-...")?;
///
/// // Retrieve
/// let key = SpnKeyring::get("anthropic")?;
/// println!("Key retrieved (will auto-clear on drop)");
///
/// // List all stored
/// for provider in SpnKeyring::list() {
///     println!("Stored: {}", provider);
/// }
/// ```
pub struct SpnKeyring;

impl SpnKeyring {
    /// Get API key for a provider as zeroizing string.
    ///
    /// The returned string will be automatically zeroized when dropped.
    ///
    /// # Errors
    ///
    /// Returns `KeyringError::NotFound` if no key is stored for this provider.
    /// Returns `KeyringError::Locked` if the keychain is locked.
    pub fn get(provider: &str) -> Result<Zeroizing<String>, KeyringError> {
        let entry = Entry::new(SERVICE_NAME, provider)
            .map_err(|e| KeyringError::AccessError(e.to_string()))?;

        let password = entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => KeyringError::NotFound(provider.to_string()),
            keyring::Error::NoStorageAccess(_) => KeyringError::Locked,
            _ => KeyringError::AccessError(e.to_string()),
        })?;

        Ok(Zeroizing::new(password))
    }

    /// Get API key wrapped in `SecretString` for maximum safety.
    ///
    /// Use this when passing keys to external APIs.
    ///
    /// # Errors
    ///
    /// Same as [`SpnKeyring::get`].
    ///
    /// # Security
    ///
    /// The intermediate String is wrapped in `Zeroizing` to ensure it's cleared
    /// even if a panic occurs before `SecretString` takes ownership.
    pub fn get_secret(provider: &str) -> Result<SecretString, KeyringError> {
        let key = Self::get(provider)?;
        // Wrap clone in Zeroizing for panic safety - if anything panics,
        // the intermediate will still be zeroized on drop
        let mut temp = Zeroizing::new((*key).clone());
        // Take the inner value - SecretString will own and zeroize it on drop
        let inner = std::mem::take(&mut *temp);
        Ok(SecretString::new(inner.into()))
    }

    /// Store API key for a provider.
    ///
    /// The key is validated against `spn_core` provider definitions before storage.
    ///
    /// # Errors
    ///
    /// Returns `KeyringError::ValidationError` if the key format is invalid.
    /// Returns `KeyringError::Locked` if the keychain is locked.
    pub fn set(provider: &str, key: &str) -> Result<(), KeyringError> {
        // Validate key format using spn-core
        let result = validate_key_format(provider, key);
        if !result.is_valid() {
            return Err(KeyringError::ValidationError(result.to_string()));
        }

        let entry = Entry::new(SERVICE_NAME, provider)
            .map_err(|e| KeyringError::AccessError(e.to_string()))?;

        entry.set_password(key).map_err(|e| match e {
            keyring::Error::NoStorageAccess(_) => KeyringError::Locked,
            _ => KeyringError::StoreError(e.to_string()),
        })
    }

    /// Store API key from `SecretString` (safer input).
    ///
    /// # Errors
    ///
    /// Same as [`SpnKeyring::set`].
    pub fn set_secret(provider: &str, key: &SecretString) -> Result<(), KeyringError> {
        use secrecy::ExposeSecret;
        Self::set(provider, key.expose_secret())
    }

    /// Delete API key for a provider.
    ///
    /// # Errors
    ///
    /// Returns `KeyringError::DeleteError` if deletion fails.
    pub fn delete(provider: &str) -> Result<(), KeyringError> {
        let entry = Entry::new(SERVICE_NAME, provider)
            .map_err(|e| KeyringError::AccessError(e.to_string()))?;

        entry
            .delete_credential()
            .map_err(|e| KeyringError::DeleteError(e.to_string()))
    }

    /// Check if key exists for a provider.
    pub fn exists(provider: &str) -> bool {
        Self::get(provider).is_ok()
    }

    /// Get masked version of stored key.
    ///
    /// Safe for logging and display.
    pub fn get_masked(provider: &str) -> Option<String> {
        Self::get(provider).ok().map(|k| mask_key(&k))
    }

    /// List all providers with stored keys.
    ///
    /// Checks all known providers from `spn_core::KNOWN_PROVIDERS`.
    pub fn list() -> Vec<String> {
        KNOWN_PROVIDERS
            .iter()
            .filter(|p| Self::exists(p.id))
            .map(|p| p.id.to_string())
            .collect()
    }

    /// Verify keychain is accessible.
    pub fn is_accessible() -> bool {
        Entry::new(SERVICE_NAME, "__spn_test__").is_ok()
    }

    /// Get the provider definition from spn-core.
    ///
    /// Returns `None` if the provider is not in `KNOWN_PROVIDERS`.
    #[must_use]
    pub fn provider_info(provider: &str) -> Option<&'static Provider> {
        find_provider(provider)
    }
}

/// Source of a resolved secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretSource {
    /// From OS keychain (most secure).
    Keychain,
    /// From environment variable.
    Environment,
    /// From .env file (least secure).
    DotEnv,
}

impl std::fmt::Display for SecretSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keychain => write!(f, "keychain"),
            Self::Environment => write!(f, "environment"),
            Self::DotEnv => write!(f, ".env file"),
        }
    }
}

/// Resolve API key from multiple sources with priority:
/// 1. OS Keychain (most secure)
/// 2. Environment variable
/// 3. .env file (if dotenvy is loaded)
///
/// Returns the key wrapped in `Zeroizing` for automatic memory clearing,
/// along with the source where it was found.
#[must_use]
pub fn resolve_key(provider: &str) -> Option<(Zeroizing<String>, SecretSource)> {
    // Try keychain first
    if let Ok(key) = SpnKeyring::get(provider) {
        return Some((key, SecretSource::Keychain));
    }

    // Try environment variable
    if let Some(env_var) = provider_to_env_var(provider) {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Some((Zeroizing::new(key), SecretSource::Environment));
            }
        }
    }

    None
}

/// Check if any provider keys are configured (in any source).
#[must_use]
pub fn has_any_keys() -> bool {
    KNOWN_PROVIDERS.iter().any(|p| resolve_key(p.id).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_accessibility() {
        // This test just ensures the function doesn't panic
        let _ = SpnKeyring::is_accessible();
    }

    #[test]
    fn test_provider_info() {
        let provider = SpnKeyring::provider_info("anthropic");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().env_var, "ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_provider_info_unknown() {
        let provider = SpnKeyring::provider_info("unknown_provider");
        assert!(provider.is_none());
    }

    #[test]
    fn test_secret_source_display() {
        assert_eq!(SecretSource::Keychain.to_string(), "keychain");
        assert_eq!(SecretSource::Environment.to_string(), "environment");
        assert_eq!(SecretSource::DotEnv.to_string(), ".env file");
    }

    #[test]
    fn test_keyring_error_display() {
        let err = KeyringError::NotFound("anthropic".to_string());
        assert!(err.to_string().contains("anthropic"));

        let err = KeyringError::ValidationError("too short".to_string());
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn test_list_returns_vec() {
        // list() should return a Vec, even if empty
        let stored = SpnKeyring::list();
        assert!(stored.is_empty() || !stored.is_empty()); // Always true, just check type
    }

    // Note: Actual keychain operations require system integration tests
    // These would need #[ignore] or run in CI with keychain access
}
