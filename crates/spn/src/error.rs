//! Error types for the SuperNovae CLI.
//!
//! Provides structured error types with helpful suggestions for users.

#![allow(dead_code)]

use colored::Colorize;
use thiserror::Error;

/// Result type alias for CLI operations.
pub type Result<T> = std::result::Result<T, SpnError>;

/// CLI error types.
#[derive(Error, Debug)]
pub enum SpnError {
    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("Manifest not found: spn.yaml")]
    ManifestNotFound,

    #[error("Lockfile not found: spn.lock")]
    LockfileNotFound,

    #[error("Index fetch failed: {0}")]
    IndexError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("MCP server not found: {0}")]
    McpServerNotFound(String),

    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Integrity check failed for {package}: expected {expected}, got {actual}")]
    IntegrityError {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("Version conflict: {0}")]
    VersionConflict(String),

    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(String),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] crate::storage::StorageError),

    #[error("Manifest error: {0}")]
    ManifestError(#[from] crate::manifest::spn_yaml::ManifestError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Daemon already running")]
    DaemonAlreadyRunning,

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("No versions available for package: {0}")]
    NoVersionsAvailable(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl SpnError {
    /// Returns a helpful suggestion for resolving this error.
    pub fn help(&self) -> Option<String> {
        match self {
            SpnError::PackageNotFound(name) => Some(format!(
                "Try: {} {} to find similar packages",
                "spn search".cyan(),
                name
            )),

            SpnError::ManifestNotFound => Some(format!(
                "Run {} to create a new spn.yaml file",
                "spn init".cyan()
            )),

            SpnError::LockfileNotFound => Some(format!(
                "Run {} to generate spn.lock from spn.yaml",
                "spn install".cyan()
            )),

            SpnError::IndexError(_) => Some(
                "Check your network connection and try again.\n   \
                 Registry: https://github.com/supernovae-st/supernovae-registry"
                    .to_string(),
            ),

            SpnError::NetworkError(_) => Some(
                "Check your network connection. If behind a proxy, ensure \
                 HTTPS_PROXY is set."
                    .to_string(),
            ),

            SpnError::McpServerNotFound(name) => Some(format!(
                "Available MCP servers: {}\n   \
                 Install with: {} {}",
                "neo4j, firecrawl, perplexity, supadata, github, slack".dimmed(),
                "spn mcp add".cyan(),
                name
            )),

            SpnError::SkillNotFound(name) => Some(format!(
                "Search for skills: {}\n   \
                 Install with: {} {}",
                "spn skill list".cyan(),
                "spn skill add".cyan(),
                name
            )),

            SpnError::IntegrityError { package, .. } => Some(format!(
                "The package {} may have been corrupted during download.\n   \
                 Try: {} && {}",
                package,
                "spn remove".cyan(),
                "spn add".cyan()
            )),

            SpnError::VersionConflict(msg) => Some(format!(
                "Version conflict: {}\n   \
                 Try relaxing version constraints in spn.yaml or use:\n   \
                 {} to update to compatible versions",
                msg,
                "spn update".cyan()
            )),

            SpnError::DependencyResolution(msg) => Some(format!(
                "Resolution failed: {}\n   \
                 Check for circular dependencies or incompatible version ranges.",
                msg
            )),

            SpnError::CommandNotFound(cmd) => Some(format!(
                "Command '{}' not found.\n   \
                 Run {} for available commands.",
                cmd,
                "spn --help".cyan()
            )),

            SpnError::ConfigError(_) => Some(format!(
                "Configuration may be corrupted.\n   \
                 Check ~/.spn/config.toml or run {} to diagnose.",
                "spn doctor".cyan()
            )),

            SpnError::DaemonNotRunning => Some(format!(
                "Start the daemon with: {}\n   \
                 Or install as service: {}",
                "spn daemon start".cyan(),
                "spn daemon install".cyan()
            )),

            SpnError::DaemonAlreadyRunning => Some(format!(
                "Stop the daemon first: {}\n   \
                 Or check status with: {}",
                "spn daemon stop".cyan(),
                "spn daemon status".cyan()
            )),

            SpnError::ProviderNotFound(name) => Some(format!(
                "Provider '{}' not recognized.\n   \
                 Available providers: {}\n   \
                 List configured: {}",
                name,
                "anthropic, openai, mistral, groq, deepseek, gemini, ollama".dimmed(),
                "spn provider list".cyan()
            )),

            SpnError::NoVersionsAvailable(pkg) => Some(format!(
                "Package '{}' exists but has no available versions.\n   \
                 It may have been yanked. Check: {}",
                pkg,
                format!("spn info {}", pkg).cyan()
            )),

            SpnError::YamlError(_) => Some(
                "Check your YAML syntax. Common issues:\n   \
                 • Incorrect indentation (use 2 spaces)\n   \
                 • Missing colons after keys\n   \
                 • Unquoted special characters"
                    .to_string(),
            ),

            SpnError::TomlError(_) => Some(
                "Check your TOML syntax. Common issues:\n   \
                 • Missing quotes around strings\n   \
                 • Incorrect table headers [section]\n   \
                 • Duplicate keys"
                    .to_string(),
            ),

            _ => None,
        }
    }

    /// Print the error with optional help message to stderr.
    pub fn print(&self) {
        eprintln!("{} {}", "error:".red().bold(), self);
        if let Some(help) = self.help() {
            eprintln!();
            eprintln!("   {} {}", "help:".yellow().bold(), help);
        }
    }
}

/// Alias for backward compatibility and consistency.
pub type CliError = SpnError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_not_found_help() {
        let err = SpnError::PackageNotFound("my-package".to_string());
        let help = err.help();
        assert!(help.is_some());
        assert!(help.unwrap().contains("spn search"));
    }

    #[test]
    fn test_manifest_not_found_help() {
        let err = SpnError::ManifestNotFound;
        let help = err.help();
        assert!(help.is_some());
        assert!(help.unwrap().contains("spn init"));
    }

    #[test]
    fn test_daemon_not_running_help() {
        let err = SpnError::DaemonNotRunning;
        let help = err.help();
        assert!(help.is_some());
        assert!(help.unwrap().contains("spn daemon start"));
    }

    #[test]
    fn test_provider_not_found_help() {
        let err = SpnError::ProviderNotFound("unknown".to_string());
        let help = err.help();
        assert!(help.is_some());
        let msg = help.unwrap();
        assert!(msg.contains("anthropic"));
        assert!(msg.contains("spn provider list"));
    }

    #[test]
    fn test_yaml_error_help() {
        let yaml_err = serde_yaml::from_str::<String>("invalid: [").unwrap_err();
        let err = SpnError::YamlError(yaml_err);
        let help = err.help();
        assert!(help.is_some());
        assert!(help.unwrap().contains("indentation"));
    }

    #[test]
    fn test_error_display() {
        let err = SpnError::PackageNotFound("test-pkg".to_string());
        let display = format!("{}", err);
        assert!(display.contains("test-pkg"));
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_io_error_no_help() {
        // IO errors are too generic for specific help
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = SpnError::IoError(io_err);
        // IoError doesn't have specific help in current implementation
        // It falls through to None in the match
        assert!(err.help().is_none());
    }
}
