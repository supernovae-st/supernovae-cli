//! Secure API key storage via system keychain.
//!
//! Uses keyring-rs for cross-platform credential storage:
//! - macOS: Keychain Access
//! - Windows: Credential Manager
//! - Linux: Secret Service (GNOME Keyring, KWallet)
//!
//! # Security Design
//!
//! All keys are:
//! 1. Validated before storage
//! 2. Stored encrypted in OS keychain
//! 3. Retrieved into SecretString (auto-zeroize)
//! 4. Never logged or printed in full

use colored::Colorize;
use keyring::Entry;
use secrecy::SecretString;
use thiserror::Error;
use zeroize::Zeroizing;

use super::types::SecretSource;

/// Service name for keyring entries.
const SERVICE_NAME: &str = "spn";

/// Keyring error types.
#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("Failed to access keyring: {0}")]
    AccessError(String),
    #[error("Key not found for provider: {0}")]
    NotFound(String),
    #[error("Failed to store key: {0}")]
    StoreError(String),
    #[error("Failed to delete key: {0}")]
    DeleteError(String),
    #[error("Invalid key format: {0}")]
    ValidationError(String),
    #[error("Keychain locked or inaccessible")]
    Locked,
}

/// Keyring wrapper for spn API keys.
///
/// All methods that return keys use `Zeroizing<String>` or `SecretString`
/// to ensure automatic memory clearing.
pub struct SpnKeyring;

impl SpnKeyring {
    /// Get API key for a provider as zeroizing string.
    ///
    /// The returned string will be automatically zeroized when dropped.
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

    /// Get API key wrapped in SecretString for maximum safety.
    ///
    /// Use this when passing keys to external APIs.
    pub fn get_secret(provider: &str) -> Result<SecretString, KeyringError> {
        let key = Self::get(provider)?;
        Ok(SecretString::from((*key).clone()))
    }

    /// Store API key for a provider.
    ///
    /// The key is validated before storage.
    pub fn set(provider: &str, key: &str) -> Result<(), KeyringError> {
        // Validate key format first
        validate_key_format(provider, key).map_err(KeyringError::ValidationError)?;

        let entry = Entry::new(SERVICE_NAME, provider)
            .map_err(|e| KeyringError::AccessError(e.to_string()))?;

        entry
            .set_password(key)
            .map_err(|e| match e {
                keyring::Error::NoStorageAccess(_) => KeyringError::Locked,
                _ => KeyringError::StoreError(e.to_string()),
            })
    }

    /// Store API key from SecretString (safer input).
    pub fn set_secret(provider: &str, key: &SecretString) -> Result<(), KeyringError> {
        use secrecy::ExposeSecret;
        Self::set(provider, key.expose_secret())
    }

    /// Delete API key for a provider.
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
        Self::get(provider).ok().map(|k| mask_api_key(&k))
    }

    /// List all providers with stored keys.
    pub fn list_stored() -> Vec<String> {
        crate::secrets::SUPPORTED_PROVIDERS
            .iter()
            .chain(super::types::MCP_SECRET_TYPES.iter())
            .filter(|p| Self::exists(p))
            .map(|p| p.to_string())
            .collect()
    }

    /// Verify keychain is accessible.
    pub fn is_accessible() -> bool {
        // Try to create an entry (doesn't store anything)
        Entry::new(SERVICE_NAME, "__test__").is_ok()
    }
}

/// Mask API key for display (show first 6 and last 1 char).
pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 10 {
        return "****".to_string();
    }
    let prefix = &key[..6.min(key.len())];
    let suffix = &key[key.len().saturating_sub(1)..];
    format!("{}...{}", prefix, suffix)
}

/// Validate API key format (basic checks).
pub fn validate_key_format(provider: &str, key: &str) -> Result<(), String> {
    // Universal empty check
    if key.trim().is_empty() {
        return Err("API key cannot be empty".into());
    }

    // Check for common mistakes
    if key.contains(' ') && !key.starts_with("http") {
        return Err("API key should not contain spaces".into());
    }

    match provider {
        "anthropic" => {
            if !key.starts_with("sk-ant-") {
                return Err("Anthropic keys start with 'sk-ant-'".into());
            }
            if key.len() < 40 {
                return Err("Key seems too short".into());
            }
        }
        "openai" => {
            if !key.starts_with("sk-") {
                return Err("OpenAI keys start with 'sk-'".into());
            }
            if key.len() < 20 {
                return Err("Key seems too short".into());
            }
        }
        "gemini" => {
            // Gemini keys start with "AIza" typically
            if key.len() < 30 {
                return Err("Gemini key seems too short".into());
            }
        }
        "mistral" | "groq" | "deepseek" => {
            if key.len() < 32 {
                return Err("Key seems too short".into());
            }
        }
        "ollama" => {
            // Ollama uses base URL, not API key
            if !key.starts_with("http") {
                return Err("Ollama requires a base URL (http://...)".into());
            }
        }
        "github" => {
            // GitHub tokens start with ghp_, gho_, ghu_, ghs_, or ghr_
            let valid_prefixes = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_"];
            if !valid_prefixes.iter().any(|p| key.starts_with(p)) {
                return Err("GitHub tokens start with 'ghp_', 'gho_', etc.".into());
            }
        }
        "perplexity" | "firecrawl" | "supadata" => {
            if key.len() < 20 {
                return Err("API key seems too short".into());
            }
        }
        _ => {
            // Unknown provider: basic length check
            if key.len() < 10 {
                return Err("Key seems too short".into());
            }
        }
    }
    Ok(())
}

/// Get environment variable name for provider.
pub fn provider_env_var(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "groq" => "GROQ_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "gemini" => "GEMINI_API_KEY",
        "ollama" => "OLLAMA_API_BASE_URL",
        // MCP-related
        "neo4j" => "NEO4J_PASSWORD",
        "github" => "GITHUB_TOKEN",
        "slack" => "SLACK_TOKEN",
        "perplexity" => "PERPLEXITY_API_KEY",
        "firecrawl" => "FIRECRAWL_API_KEY",
        "supadata" => "SUPADATA_API_KEY",
        _ => "UNKNOWN_API_KEY",
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MIGRATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Providers to migrate (excludes ollama - uses URL not key).
const MIGRATABLE_PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    "mistral",
    "groq",
    "deepseek",
    "gemini",
    "perplexity",
    "github",
    "firecrawl",
    "supadata",
];

/// Report of migration results.
#[derive(Debug, Default)]
pub struct MigrationReport {
    /// Number of keys migrated to keychain.
    pub migrated: usize,
    /// Number skipped (already in keychain).
    pub skipped: usize,
    /// Providers with no env var set.
    pub not_found: Vec<String>,
    /// Providers that failed (provider, error).
    pub errors: Vec<(String, String)>,
}

impl MigrationReport {
    /// Generate summary string.
    pub fn summary(&self) -> String {
        format!(
            "Migration complete: {} migrated, {} skipped, {} not found",
            self.migrated,
            self.skipped,
            self.not_found.len()
        )
    }

    /// Check if migration was successful (no errors).
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Migrate API keys from environment variables to system keychain.
pub fn migrate_env_to_keyring() -> MigrationReport {
    let mut report = MigrationReport::default();

    for provider in MIGRATABLE_PROVIDERS {
        let env_var = provider_env_var(provider);

        match std::env::var(env_var) {
            Ok(key) if !key.is_empty() => {
                // Use Zeroizing to clear the key from memory after migration
                let key = Zeroizing::new(key);

                // Check if already in keyring
                if SpnKeyring::exists(provider) {
                    println!(
                        "  {} {}: Found {} {}",
                        "├──".dimmed(),
                        env_var,
                        "→".dimmed(),
                        "Already in keychain (skipped)".yellow()
                    );
                    report.skipped += 1;
                    continue;
                }

                // Migrate to keyring
                print!(
                    "  {} {}: Found {} Migrating... ",
                    "├──".dimmed(),
                    env_var,
                    "→".dimmed()
                );
                match SpnKeyring::set(provider, &key) {
                    Ok(()) => {
                        println!("{}", "✓".green());
                        report.migrated += 1;
                    }
                    Err(e) => {
                        println!("{} ({})", "✗".red(), e);
                        report.errors.push((provider.to_string(), e.to_string()));
                    }
                }
                // key is automatically zeroized when it goes out of scope
            }
            _ => {
                println!(
                    "  {} {}: {}",
                    "├──".dimmed(),
                    env_var,
                    "Not found".dimmed()
                );
                report.not_found.push(provider.to_string());
            }
        }
    }

    report
}

/// Resolve API key from multiple sources with priority:
/// 1. OS Keychain (most secure)
/// 2. Environment variable
/// 3. .env file (via dotenvy)
///
/// Returns the key wrapped in Zeroizing for automatic memory clearing.
pub fn resolve_api_key(provider: &str) -> Option<(Zeroizing<String>, SecretSource)> {
    // Try keychain first
    if let Ok(key) = SpnKeyring::get(provider) {
        return Some((key, SecretSource::Keychain));
    }

    // Try environment variable
    let env_var = provider_env_var(provider);
    if let Ok(key) = std::env::var(env_var) {
        if !key.is_empty() {
            return Some((Zeroizing::new(key), SecretSource::Environment));
        }
    }

    // Try .env file
    if dotenvy::dotenv().is_ok() {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Some((Zeroizing::new(key), SecretSource::DotEnv));
            }
        }
    }

    None
}

/// Check if any provider keys are configured.
pub fn has_any_keys() -> bool {
    crate::secrets::SUPPORTED_PROVIDERS
        .iter()
        .any(|p| resolve_api_key(p).is_some())
}

/// Get security status for all providers.
pub fn security_audit() -> Vec<(String, Option<SecretSource>, String)> {
    let mut results = Vec::new();

    for provider in crate::secrets::SUPPORTED_PROVIDERS.iter()
        .chain(super::types::MCP_SECRET_TYPES.iter())
    {
        let (source, recommendation) = match resolve_api_key(provider) {
            Some((_, SecretSource::Keychain)) => {
                (Some(SecretSource::Keychain), "✓ Secure".to_string())
            }
            Some((_, SecretSource::Environment)) => {
                (Some(SecretSource::Environment), "Consider migrating to keychain".to_string())
            }
            Some((_, SecretSource::DotEnv)) => {
                (Some(SecretSource::DotEnv), "⚠ Migrate to keychain with `spn provider migrate`".to_string())
            }
            Some((_, SecretSource::Inline)) => {
                (Some(SecretSource::Inline), "⚠ INSECURE - remove from config!".to_string())
            }
            None => {
                (None, "Not configured".to_string())
            }
        };
        results.push((provider.to_string(), source, recommendation));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key_standard() {
        let key = "sk-ant-api03-abc123xyz789def456ghi";
        assert_eq!(mask_api_key(key), "sk-ant...i");
    }

    #[test]
    fn test_mask_api_key_short() {
        assert_eq!(mask_api_key("short"), "****");
        assert_eq!(mask_api_key("1234567890"), "****");
    }

    #[test]
    fn test_mask_api_key_empty() {
        assert_eq!(mask_api_key(""), "****");
    }

    #[test]
    fn test_validate_anthropic_key_valid() {
        let result =
            validate_key_format("anthropic", "sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_anthropic_key_wrong_prefix() {
        let result = validate_key_format("anthropic", "sk-wrong-prefix");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sk-ant-"));
    }

    #[test]
    fn test_validate_openai_key_valid() {
        let result = validate_key_format("openai", "sk-proj-abcdefghijklmnop");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_openai_key_wrong_prefix() {
        let result = validate_key_format("openai", "wrong-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_gemini_key_valid() {
        let result = validate_key_format("gemini", "AIzaSyBabcdefghijklmnopqrstuvwxyz123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_ollama_requires_url() {
        let result = validate_key_format("ollama", "http://localhost:11434");
        assert!(result.is_ok());

        let result = validate_key_format("ollama", "not-a-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_github_token() {
        // Valid GitHub tokens
        assert!(validate_key_format("github", "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxx").is_ok());
        assert!(validate_key_format("github", "github_pat_xxxxxxxxxxxxx").is_ok());

        // Invalid
        assert!(validate_key_format("github", "invalid_token").is_err());
    }

    #[test]
    fn test_validate_empty_key_rejected() {
        let result = validate_key_format("anthropic", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_whitespace_only_key_rejected() {
        let result = validate_key_format("openai", "   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_key_with_spaces_rejected() {
        let result = validate_key_format("anthropic", "sk-ant-api key with spaces");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("spaces"));
    }

    #[test]
    fn test_provider_env_var() {
        assert_eq!(provider_env_var("anthropic"), "ANTHROPIC_API_KEY");
        assert_eq!(provider_env_var("openai"), "OPENAI_API_KEY");
        assert_eq!(provider_env_var("gemini"), "GEMINI_API_KEY");
        assert_eq!(provider_env_var("ollama"), "OLLAMA_API_BASE_URL");
        assert_eq!(provider_env_var("github"), "GITHUB_TOKEN");
        assert_eq!(provider_env_var("firecrawl"), "FIRECRAWL_API_KEY");
    }

    #[test]
    fn test_keyring_accessibility() {
        // This test just ensures the function doesn't panic
        let _ = SpnKeyring::is_accessible();
    }

    #[test]
    fn test_migration_report() {
        let mut report = MigrationReport::default();
        report.migrated = 2;
        report.skipped = 1;
        report.not_found = vec!["test".to_string()];

        assert!(report.is_success());
        assert!(report.summary().contains("2 migrated"));
    }
}
