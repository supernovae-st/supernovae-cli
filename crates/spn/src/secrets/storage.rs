//! Storage backend abstraction for secrets.
//!
//! Provides multiple storage options for API keys:
//! - OS Keychain (most secure, default)
//! - Project .env file
//! - Global ~/.spn/secrets.env
//! - Shell export (print only)

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Storage backend options for secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// OS Keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service).
    /// Most secure option - encrypted by OS, protected by login password.
    #[default]
    Keychain,
    /// Project .env file in current directory.
    /// Good for project-specific keys, should be gitignored.
    Env,
    /// Global ~/.spn/secrets.env file.
    /// Shared across all projects, convenient but less secure than keychain.
    Global,
    /// Shell export command (print only, no storage).
    /// For manual handling or piping to other tools.
    Shell,
}

#[derive(Error, Debug)]
#[error("Invalid storage backend: '{0}'. Valid options: keychain, env, global, shell")]
pub struct InvalidStorageBackend(String);

impl FromStr for StorageBackend {
    type Err = InvalidStorageBackend;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "keychain" => Ok(Self::Keychain),
            "env" => Ok(Self::Env),
            "global" | "global-env" => Ok(Self::Global),
            "shell" | "export" => Ok(Self::Shell),
            _ => Err(InvalidStorageBackend(s.to_string())),
        }
    }
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keychain => write!(f, "keychain"),
            Self::Env => write!(f, "env"),
            Self::Global => write!(f, "global"),
            Self::Shell => write!(f, "shell"),
        }
    }
}

impl StorageBackend {
    /// Human-readable description of this storage backend.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Keychain => "OS Keychain - Most secure, protected by login password",
            Self::Env => "Project .env file - Good for project-specific keys",
            Self::Global => "Global ~/.spn/secrets.env - Shared across all projects",
            Self::Shell => "Shell export - Print command only, you handle storage",
        }
    }

    /// Short description for wizard UI.
    pub fn short_description(&self) -> &'static str {
        match self {
            Self::Keychain => "Most secure. Protected by your login password.",
            Self::Env => "Stored in ./.env (gitignored). Good for project-specific keys.",
            Self::Global => "Stored in ~/.spn/secrets.env. Shared across all projects.",
            Self::Shell => "Key is validated and export command printed. You handle storage.",
        }
    }

    /// Emoji indicator for this storage type.
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Keychain => "🔐",
            Self::Env => "📁",
            Self::Global => "🌍",
            Self::Shell => "📋",
        }
    }

    /// Security level (higher = more secure).
    pub fn security_level(&self) -> u8 {
        match self {
            Self::Keychain => 5,
            Self::Global => 3,
            Self::Env => 2,
            Self::Shell => 1,
        }
    }

    /// Whether this backend requires macOS Developer ID signing to avoid popup.
    pub fn requires_signing(&self) -> bool {
        matches!(self, Self::Keychain)
    }

    /// Get all available storage backends for wizard selection.
    pub fn all() -> &'static [StorageBackend] {
        &[
            StorageBackend::Keychain,
            StorageBackend::Env,
            StorageBackend::Global,
            StorageBackend::Shell,
        ]
    }
}

/// Result of storing a secret.
#[derive(Debug)]
pub enum StoreResult {
    /// Secret was stored successfully.
    Stored {
        backend: StorageBackend,
        location: String,
    },
    /// Shell export command (not stored, just returned).
    ExportCommand { command: String },
}

impl StoreResult {
    /// Get a user-friendly message describing the result.
    pub fn message(&self) -> String {
        match self {
            Self::Stored { backend, location } => {
                format!("API key stored in {} ({})", backend, location)
            }
            Self::ExportCommand { command } => {
                format!("Export command:\n{}", command)
            }
        }
    }
}

/// Get the path to the global secrets file.
///
/// Returns an error if the home directory cannot be determined
/// (e.g., in Docker containers or chroot environments).
pub fn global_secrets_path() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(".spn").join("secrets.env"))
        .context("Could not determine home directory. This can happen in Docker containers or chroot environments.")
}

/// Get the path to the project .env file.
pub fn project_env_path() -> PathBuf {
    PathBuf::from(".env")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_backend_from_str_valid() {
        assert_eq!(
            StorageBackend::from_str("keychain").unwrap(),
            StorageBackend::Keychain
        );
        assert_eq!(
            StorageBackend::from_str("env").unwrap(),
            StorageBackend::Env
        );
        assert_eq!(
            StorageBackend::from_str("global").unwrap(),
            StorageBackend::Global
        );
        assert_eq!(
            StorageBackend::from_str("global-env").unwrap(),
            StorageBackend::Global
        );
        assert_eq!(
            StorageBackend::from_str("shell").unwrap(),
            StorageBackend::Shell
        );
        assert_eq!(
            StorageBackend::from_str("export").unwrap(),
            StorageBackend::Shell
        );
    }

    #[test]
    fn test_storage_backend_from_str_case_insensitive() {
        assert_eq!(
            StorageBackend::from_str("KEYCHAIN").unwrap(),
            StorageBackend::Keychain
        );
        assert_eq!(
            StorageBackend::from_str("Env").unwrap(),
            StorageBackend::Env
        );
        assert_eq!(
            StorageBackend::from_str("GLOBAL").unwrap(),
            StorageBackend::Global
        );
    }

    #[test]
    fn test_storage_backend_from_str_invalid() {
        assert!(StorageBackend::from_str("invalid").is_err());
        assert!(StorageBackend::from_str("").is_err());
        assert!(StorageBackend::from_str("database").is_err());

        let err = StorageBackend::from_str("foo").unwrap_err();
        assert!(err.to_string().contains("foo"));
        assert!(err.to_string().contains("keychain"));
    }

    #[test]
    fn test_storage_backend_display() {
        assert_eq!(StorageBackend::Keychain.to_string(), "keychain");
        assert_eq!(StorageBackend::Env.to_string(), "env");
        assert_eq!(StorageBackend::Global.to_string(), "global");
        assert_eq!(StorageBackend::Shell.to_string(), "shell");
    }

    #[test]
    fn test_storage_backend_description() {
        let desc = StorageBackend::Keychain.description();
        assert!(desc.contains("OS Keychain"));
        assert!(desc.contains("secure"));

        let desc = StorageBackend::Env.description();
        assert!(desc.contains(".env"));

        let desc = StorageBackend::Global.description();
        assert!(desc.contains("~/.spn"));
    }

    #[test]
    fn test_storage_backend_emoji() {
        assert_eq!(StorageBackend::Keychain.emoji(), "🔐");
        assert_eq!(StorageBackend::Env.emoji(), "📁");
        assert_eq!(StorageBackend::Global.emoji(), "🌍");
        assert_eq!(StorageBackend::Shell.emoji(), "📋");
    }

    #[test]
    fn test_storage_backend_security_level() {
        // Keychain should be most secure
        assert!(
            StorageBackend::Keychain.security_level() > StorageBackend::Global.security_level()
        );
        assert!(StorageBackend::Global.security_level() > StorageBackend::Env.security_level());
        assert!(StorageBackend::Env.security_level() > StorageBackend::Shell.security_level());
    }

    #[test]
    fn test_storage_backend_all() {
        let all = StorageBackend::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&StorageBackend::Keychain));
        assert!(all.contains(&StorageBackend::Env));
        assert!(all.contains(&StorageBackend::Global));
        assert!(all.contains(&StorageBackend::Shell));
    }

    #[test]
    fn test_storage_backend_default() {
        assert_eq!(StorageBackend::default(), StorageBackend::Keychain);
    }

    #[test]
    fn test_storage_backend_serde() {
        let backend = StorageBackend::Keychain;
        let json = serde_json::to_string(&backend).unwrap();
        assert_eq!(json, "\"keychain\"");

        let parsed: StorageBackend = serde_json::from_str("\"env\"").unwrap();
        assert_eq!(parsed, StorageBackend::Env);
    }

    #[test]
    fn test_store_result_message() {
        let stored = StoreResult::Stored {
            backend: StorageBackend::Keychain,
            location: "OS Keychain".to_string(),
        };
        assert!(stored.message().contains("Keychain"));

        let export = StoreResult::ExportCommand {
            command: "export FOO=bar".to_string(),
        };
        assert!(export.message().contains("export FOO=bar"));
    }

    #[test]
    fn test_global_secrets_path() {
        let path = global_secrets_path().expect("Should be able to get home directory");
        assert!(path.ends_with(".spn/secrets.env"));
    }

    #[test]
    fn test_project_env_path() {
        let path = project_env_path();
        assert_eq!(path, PathBuf::from(".env"));
    }
}
