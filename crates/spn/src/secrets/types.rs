//! Secret types with zeroize support.
//!
//! TODO(v0.14): Integrate additional provider methods
//!
//! # Security Design
//!
//! All sensitive data in this module follows defense-in-depth:
//! 1. `Zeroize` trait - explicit memory clearing
//! 2. `ZeroizeOnDrop` - automatic clearing when dropped
//! 3. `SecretString` - prevents accidental exposure (Debug, Display)
//! 4. `Zeroizing<T>` wrapper - auto-zeroize for any type

#![allow(dead_code)]

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

// Re-export core types from spn-keyring (which re-exports from spn-core)
pub use spn_keyring::{
    find_provider, mask_key, validate_key_format, Provider, ProviderCategory, ValidationResult,
    KNOWN_PROVIDERS,
};

/// Alias for backward compatibility: `mask_api_key` -> `mask_key`
pub fn mask_api_key(key: &str) -> String {
    mask_key(key)
}

/// Alias for backward compatibility: `provider_env_var` -> `provider_to_env_var`
pub fn provider_env_var(provider: &str) -> &'static str {
    spn_keyring::provider_to_env_var(provider).unwrap_or("UNKNOWN_API_KEY")
}

/// Get LLM provider IDs (for backward compatibility with SUPPORTED_PROVIDERS constant).
///
/// Returns provider IDs for LLM and Local categories.
pub fn llm_provider_ids() -> impl Iterator<Item = &'static str> {
    KNOWN_PROVIDERS.iter().filter_map(|p| {
        if matches!(
            p.category,
            ProviderCategory::Llm | ProviderCategory::Local
        ) {
            Some(p.id)
        } else {
            None
        }
    })
}

/// Get MCP provider IDs (for backward compatibility with MCP_SECRET_TYPES constant).
///
/// Returns provider IDs for MCP category.
pub fn mcp_provider_ids() -> impl Iterator<Item = &'static str> {
    KNOWN_PROVIDERS
        .iter()
        .filter_map(|p| {
            if p.category == ProviderCategory::Mcp {
                Some(p.id)
            } else {
                None
            }
        })
}

/// Supported LLM providers with their key formats.
///
/// DEPRECATED: Use `llm_provider_ids()` or `KNOWN_PROVIDERS` from spn_core instead.
/// This constant is kept for backward compatibility.
pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "anthropic", // ANTHROPIC_API_KEY (sk-ant-...)
    "openai",    // OPENAI_API_KEY (sk-...)
    "mistral",   // MISTRAL_API_KEY
    "groq",      // GROQ_API_KEY
    "deepseek",  // DEEPSEEK_API_KEY
    "gemini",    // GEMINI_API_KEY
    "ollama",    // OLLAMA_API_BASE_URL (URL, not key)
];

/// MCP-related secret types.
///
/// DEPRECATED: Use `mcp_provider_ids()` or `KNOWN_PROVIDERS` from spn_core instead.
/// This constant is kept for backward compatibility.
pub const MCP_SECRET_TYPES: &[&str] = &[
    "neo4j",      // NEO4J_PASSWORD
    "github",     // GITHUB_TOKEN
    "slack",      // SLACK_BOT_TOKEN
    "perplexity", // PERPLEXITY_API_KEY
    "firecrawl",  // FIRECRAWL_API_KEY
    "supadata",   // SUPADATA_API_KEY
];

/// Provider API key with maximum secure handling.
///
/// This struct provides multiple layers of protection:
/// - Inner key is wrapped in `Zeroizing<String>` for auto-clear on drop
/// - Manual `Zeroize` and `ZeroizeOnDrop` implementations
/// - Debug trait redacts the key value
/// - No Clone to prevent accidental copies
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ProviderKey {
    /// Provider name (e.g., "anthropic", "openai").
    #[zeroize(skip)] // Provider name is not sensitive
    pub provider: String,
    /// The actual API key (auto-zeroized on drop).
    key: Zeroizing<String>,
}

impl ProviderKey {
    /// Create a new provider key.
    ///
    /// The key is immediately wrapped in `Zeroizing` for protection.
    pub fn new(provider: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            key: Zeroizing::new(key.into()),
        }
    }

    /// Create from SecretString (transfers ownership securely).
    pub fn from_secret(provider: impl Into<String>, secret: SecretString) -> Self {
        Self {
            provider: provider.into(),
            key: Zeroizing::new(secret.expose_secret().to_string()),
        }
    }

    /// Get the key value (raw access - use sparingly).
    ///
    /// Prefer `to_secret()` for passing to APIs.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Convert to SecretString for safer handling.
    ///
    /// The returned SecretString will also be zeroized on drop.
    pub fn to_secret(&self) -> SecretString {
        SecretString::from((*self.key).clone())
    }

    /// Get masked version for display.
    ///
    /// Shows first 6 and last 1 character only.
    pub fn masked(&self) -> String {
        mask_key(&self.key)
    }

    /// Validate key format for this provider.
    pub fn validate(&self) -> Result<(), String> {
        let result = validate_key_format(&self.provider, &self.key);
        if result.is_valid() {
            Ok(())
        } else {
            Err(result.to_string())
        }
    }

    /// Explicitly zeroize the key (called automatically on drop).
    pub fn clear(&mut self) {
        self.key.zeroize();
    }
}

impl std::fmt::Debug for ProviderKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderKey")
            .field("provider", &self.provider)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

// Prevent Display from exposing the key
impl std::fmt::Display for ProviderKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProviderKey({}: {})", self.provider, self.masked())
    }
}

/// Environment source for a secret.
///
/// Extended version with `Inline` variant for CLI-specific use cases.
/// For the base version, see `spn_keyring::SecretSource`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretSource {
    /// Stored in OS keychain (most secure).
    Keychain,
    /// From environment variable.
    Environment,
    /// From .env file.
    DotEnv,
    /// Inline in config (NOT recommended).
    Inline,
}

impl SecretSource {
    /// Security level (higher = more secure).
    pub fn security_level(&self) -> u8 {
        match self {
            SecretSource::Keychain => 3,    // Most secure
            SecretSource::Environment => 2, // OK for CI
            SecretSource::DotEnv => 1,      // Acceptable for dev
            SecretSource::Inline => 0,      // Never do this
        }
    }

    /// Human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            SecretSource::Keychain => "OS Keychain (secure)",
            SecretSource::Environment => "Environment variable",
            SecretSource::DotEnv => ".env file",
            SecretSource::Inline => "Inline in config (INSECURE!)",
        }
    }

    /// Emoji indicator for source type.
    pub fn icon(&self) -> &'static str {
        match self {
            SecretSource::Keychain => "🔐",
            SecretSource::Environment => "📦",
            SecretSource::DotEnv => "📄",
            SecretSource::Inline => "⚠️",
        }
    }
}

/// Convert from spn_keyring::SecretSource to our extended version.
impl From<spn_keyring::SecretSource> for SecretSource {
    fn from(source: spn_keyring::SecretSource) -> Self {
        match source {
            spn_keyring::SecretSource::Keychain => SecretSource::Keychain,
            spn_keyring::SecretSource::Environment => SecretSource::Environment,
            spn_keyring::SecretSource::DotEnv => SecretSource::DotEnv,
        }
    }
}

/// Secure buffer for temporarily holding sensitive data.
///
/// Use this when you need to temporarily hold sensitive data
/// that will be zeroized when the buffer is dropped.
pub type SecureBuffer = Zeroizing<Vec<u8>>;

/// Secure string for temporarily holding sensitive text.
///
/// Use this when you need to temporarily hold sensitive strings
/// that will be zeroized when dropped.
pub type SecureString = Zeroizing<String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_key_masked() {
        let key = ProviderKey::new("anthropic", "sk-ant-api03-abc123xyz789");
        // mask_key from spn-core shows first 7 chars + bullets
        assert!(key.masked().starts_with("sk-ant-"));
        assert!(key.masked().contains("••••••••"));
    }

    #[test]
    fn test_provider_key_masked_short() {
        let key = ProviderKey::new("test", "short");
        // mask_key from spn-core shows all chars (up to 7) + bullets
        assert_eq!(key.masked(), "short••••••••");
    }

    #[test]
    fn test_provider_key_debug_redacted() {
        let key = ProviderKey::new("openai", "sk-secret");
        let debug = format!("{:?}", key);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("sk-secret"));
    }

    #[test]
    fn test_provider_key_display_masked() {
        let key = ProviderKey::new("anthropic", "sk-ant-api03-abc123xyz789");
        let display = format!("{}", key);
        assert!(display.contains("ProviderKey"));
        assert!(!display.contains("abc123"));
    }

    #[test]
    fn test_provider_key_zeroize() {
        let mut key = ProviderKey::new("test", "secret-value");
        assert_eq!(key.key(), "secret-value");
        key.clear();
        // After zeroize, the string should be empty or zeroed
        assert!(key.key().is_empty() || key.key().chars().all(|c| c == '\0'));
    }

    #[test]
    fn test_secret_source_security_level() {
        assert!(
            SecretSource::Keychain.security_level() > SecretSource::Environment.security_level()
        );
        assert!(SecretSource::Environment.security_level() > SecretSource::DotEnv.security_level());
        assert!(SecretSource::DotEnv.security_level() > SecretSource::Inline.security_level());
    }

    #[test]
    fn test_mask_api_key_alias() {
        assert_eq!(mask_api_key("sk-ant-api03-abc123xyz789"), mask_key("sk-ant-api03-abc123xyz789"));
    }

    #[test]
    fn test_secure_string_zeroize() {
        let mut secure: SecureString = Zeroizing::new("secret".to_string());
        assert_eq!(*secure, "secret");
        secure.zeroize();
        assert!(secure.is_empty());
    }

    #[test]
    fn test_llm_provider_ids() {
        let providers: Vec<_> = llm_provider_ids().collect();
        assert!(providers.contains(&"anthropic"));
        assert!(providers.contains(&"openai"));
        assert!(providers.contains(&"ollama"));
        assert!(!providers.contains(&"github")); // MCP, not LLM
    }

    #[test]
    fn test_mcp_provider_ids() {
        let providers: Vec<_> = mcp_provider_ids().collect();
        assert!(providers.contains(&"github"));
        assert!(providers.contains(&"neo4j"));
        assert!(!providers.contains(&"anthropic")); // LLM, not MCP
    }

    #[test]
    fn test_provider_env_var_alias() {
        assert_eq!(provider_env_var("anthropic"), "ANTHROPIC_API_KEY");
        assert_eq!(provider_env_var("github"), "GITHUB_TOKEN");
        assert_eq!(provider_env_var("unknown"), "UNKNOWN_API_KEY");
    }

    #[test]
    fn test_secret_source_from_spn_keyring() {
        let keychain: SecretSource = spn_keyring::SecretSource::Keychain.into();
        assert_eq!(keychain, SecretSource::Keychain);

        let env: SecretSource = spn_keyring::SecretSource::Environment.into();
        assert_eq!(env, SecretSource::Environment);
    }
}
