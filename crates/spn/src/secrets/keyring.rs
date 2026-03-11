//! Secure API key storage via system keychain.
//!
//! This module provides CLI-specific extensions on top of `spn_keyring`.
//!
//! TODO(v0.16): Integrate additional keyring methods
//!
//! # Core keyring functionality
//!
//! For basic keyring operations, use `SpnKeyring` from spn_keyring:
//! - `SpnKeyring::get()` / `SpnKeyring::set()` / `SpnKeyring::delete()`
//! - `SpnKeyring::exists()` / `SpnKeyring::list()`
//!
//! # CLI-specific extensions
//!
//! This module adds:
//! - `resolve_api_key()` - Resolution with .env file support
//! - `migrate_env_to_keyring()` - Interactive migration with colored output
//! - `security_audit()` - Provider security status check

#![allow(dead_code)]

use crate::ux::design_system as ds;
use zeroize::Zeroizing;

use super::types::{provider_env_var, SecretSource, MCP_SECRET_TYPES, SUPPORTED_PROVIDERS};

// Re-export SpnKeyring from spn_keyring for convenience
pub use spn_keyring::{KeyringError, SpnKeyring};

// Re-export validation functions from types (which re-exports from spn_keyring)
pub use super::types::{mask_api_key, validate_key_format};

// ═══════════════════════════════════════════════════════════════════════════════
// RESOLVE API KEY (Extended with .env support)
// ═══════════════════════════════════════════════════════════════════════════════

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
    SUPPORTED_PROVIDERS
        .iter()
        .any(|p| resolve_api_key(p).is_some())
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECURITY AUDIT
// ═══════════════════════════════════════════════════════════════════════════════

/// Get security status for all providers.
pub fn security_audit() -> Vec<(String, Option<SecretSource>, String)> {
    let mut results = Vec::new();

    for provider in SUPPORTED_PROVIDERS.iter().chain(MCP_SECRET_TYPES.iter()) {
        let (source, recommendation) = match resolve_api_key(provider) {
            Some((_, SecretSource::Keychain)) => {
                (Some(SecretSource::Keychain), "✓ Secure".to_string())
            }
            Some((_, SecretSource::Environment)) => (
                Some(SecretSource::Environment),
                "Consider migrating to keychain".to_string(),
            ),
            Some((_, SecretSource::DotEnv)) => (
                Some(SecretSource::DotEnv),
                "⚠ Migrate to keychain with `spn provider migrate`".to_string(),
            ),
            Some((_, SecretSource::Inline)) => (
                Some(SecretSource::Inline),
                "⚠ INSECURE - remove from config!".to_string(),
            ),
            None => (None, "Not configured".to_string()),
        };
        results.push((provider.to_string(), source, recommendation));
    }

    results
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
///
/// On macOS, this uses `set_with_acl()` to pre-authorize the spn binary,
/// preventing repeated keychain popup prompts.
///
/// # Feature: `os-keychain`
///
/// When the `os-keychain` feature is disabled, this function returns an error
/// indicating that keychain is unavailable (use env vars instead).
#[cfg(feature = "os-keychain")]
pub fn migrate_env_to_keyring() -> MigrationReport {
    use std::path::PathBuf;

    let mut report = MigrationReport::default();

    // Get the path to the spn binary for ACL pre-authorization
    let spn_path: PathBuf = std::env::current_exe().unwrap_or_else(|_| {
        // Fallback to ~/.cargo/bin/spn
        dirs::home_dir()
            .map(|h| h.join(".cargo/bin/spn"))
            .unwrap_or_else(|| PathBuf::from("/usr/local/bin/spn"))
    });

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
                        ds::muted("├──"),
                        env_var,
                        ds::muted(ds::icon::ARROW),
                        ds::warning("Already in keychain (skipped)")
                    );
                    report.skipped += 1;
                    continue;
                }

                // Migrate to keyring with ACL pre-authorization
                print!(
                    "  {} {}: Found {} Migrating... ",
                    ds::muted("├──"),
                    env_var,
                    ds::muted(ds::icon::ARROW)
                );
                match SpnKeyring::set_with_acl(provider, &key, &spn_path) {
                    Ok(()) => {
                        println!("{}", ds::success(ds::icon::SUCCESS));
                        report.migrated += 1;
                    }
                    Err(e) => {
                        println!("{} ({})", ds::error(ds::icon::ERROR), e);
                        report.errors.push((provider.to_string(), e.to_string()));
                    }
                }
                // key is automatically zeroized when it goes out of scope
            }
            _ => {
                println!(
                    "  {} {}: {}",
                    ds::muted("├──"),
                    env_var,
                    ds::muted("Not found")
                );
                report.not_found.push(provider.to_string());
            }
        }
    }

    report
}

/// Migrate API keys - stub for Docker builds (keychain unavailable).
#[cfg(not(feature = "os-keychain"))]
pub fn migrate_env_to_keyring() -> MigrationReport {
    println!(
        "  {} {}",
        ds::warning(ds::icon::WARNING),
        ds::warning("Keychain unavailable (Docker build). Use environment variables.")
    );
    let mut report = MigrationReport::default();
    report.errors.push((
        "all".to_string(),
        "Keychain not available in this build".to_string(),
    ));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_report() {
        let report = MigrationReport {
            migrated: 2,
            skipped: 1,
            not_found: vec!["test".to_string()],
            ..Default::default()
        };

        assert!(report.is_success());
        assert!(report.summary().contains("2 migrated"));
    }

    #[test]
    fn test_keyring_accessibility() {
        // This test just ensures the function doesn't panic
        let _ = SpnKeyring::is_accessible();
    }

    #[test]
    fn test_security_audit_covers_all_providers() {
        let audit = security_audit();
        let audit_providers: Vec<&str> = audit.iter().map(|(p, _, _)| p.as_str()).collect();

        // All LLM providers should be in audit
        for provider in SUPPORTED_PROVIDERS {
            assert!(
                audit_providers.contains(provider),
                "LLM provider '{}' should be in security_audit",
                provider
            );
        }

        // All MCP types should be in audit
        for mcp_type in MCP_SECRET_TYPES {
            assert!(
                audit_providers.contains(mcp_type),
                "MCP type '{}' should be in security_audit",
                mcp_type
            );
        }
    }

    #[test]
    fn test_security_audit_returns_recommendations() {
        let audit = security_audit();

        // Each entry should have a recommendation string
        for (provider, _source, recommendation) in &audit {
            assert!(
                !recommendation.is_empty(),
                "Provider {} should have a recommendation",
                provider
            );
        }
    }

    #[test]
    fn test_migratable_providers_excludes_ollama() {
        // Ollama uses URL, not API key, so it shouldn't be migrated
        assert!(!MIGRATABLE_PROVIDERS.contains(&"ollama"));
    }

    #[test]
    fn test_migratable_providers_includes_mcp() {
        // MCP providers should be migratable
        assert!(MIGRATABLE_PROVIDERS.contains(&"github"));
        assert!(MIGRATABLE_PROVIDERS.contains(&"firecrawl"));
    }
}
