//! Error types for the SuperNovae CLI.
//!
//! Provides structured error types with helpful suggestions for users.

#![allow(dead_code)]

use console::style;
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

    #[error("Interactive prompt error: {0}")]
    DialoguerError(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl From<dialoguer::Error> for SpnError {
    fn from(e: dialoguer::Error) -> Self {
        SpnError::DialoguerError(e.to_string())
    }
}

impl SpnError {
    /// Returns a helpful suggestion for resolving this error.
    pub fn help(&self) -> Option<String> {
        match self {
            SpnError::PackageNotFound(name) => Some(format!(
                "Try: {} {} to find similar packages",
                style("spn search").cyan(),
                name
            )),

            SpnError::ManifestNotFound => Some(format!(
                "Run {} to create a new spn.yaml file",
                style("spn init").cyan()
            )),

            SpnError::LockfileNotFound => Some(format!(
                "Run {} to generate spn.lock from spn.yaml",
                style("spn install").cyan()
            )),

            SpnError::IndexError(_) => Some(format!(
                "Check your network connection and try again.\n   \
                 Registry: {}",
                style("https://github.com/supernovae-st/supernovae-registry").dim()
            )),

            SpnError::NetworkError(_) => Some(
                "Check your network connection. If behind a proxy, ensure \
                 HTTPS_PROXY is set."
                    .to_string(),
            ),

            SpnError::McpServerNotFound(name) => Some(format!(
                "Available MCP servers: {}\n   \
                 Install with: {} {}",
                style("neo4j, firecrawl, perplexity, supadata, github, slack").dim(),
                style("spn mcp add").cyan(),
                name
            )),

            SpnError::SkillNotFound(name) => Some(format!(
                "Search for skills: {}\n   \
                 Install with: {} {}",
                style("spn skill list").cyan(),
                style("spn skill add").cyan(),
                name
            )),

            SpnError::IntegrityError { package, .. } => Some(format!(
                "The package {} may have been corrupted during download.\n   \
                 Try: {} && {}",
                style(package).bold(),
                style("spn remove").cyan(),
                style("spn add").cyan()
            )),

            SpnError::VersionConflict(msg) => Some(format!(
                "Version conflict: {}\n   \
                 Try relaxing version constraints in spn.yaml or use:\n   \
                 {} to update to compatible versions",
                msg,
                style("spn update").cyan()
            )),

            SpnError::DependencyResolution(msg) => Some(format!(
                "Resolution failed: {}\n   \
                 Check for circular dependencies or incompatible version ranges.",
                msg
            )),

            SpnError::CommandNotFound(cmd) => Some(format!(
                "Command '{}' not found.\n   \
                 Run {} for available commands.",
                style(cmd).yellow(),
                style("spn --help").cyan()
            )),

            SpnError::ConfigError(_) => Some(format!(
                "Configuration may be corrupted.\n   \
                 Check {} or run {} to diagnose.",
                style("~/.spn/config.toml").dim(),
                style("spn doctor").cyan()
            )),

            SpnError::DaemonNotRunning => Some(format!(
                "Start the daemon with: {}\n   \
                 Or install as service: {}",
                style("spn daemon start").cyan(),
                style("spn daemon install").cyan()
            )),

            SpnError::DaemonAlreadyRunning => Some(format!(
                "Stop the daemon first: {}\n   \
                 Or check status with: {}",
                style("spn daemon stop").cyan(),
                style("spn daemon status").cyan()
            )),

            SpnError::ProviderNotFound(name) => Some(format!(
                "Provider '{}' not recognized.\n   \
                 Available: {}\n   \
                 List configured: {}",
                style(name).yellow(),
                style("anthropic, openai, mistral, groq, deepseek, gemini, ollama").dim(),
                style("spn provider list").cyan()
            )),

            SpnError::NoVersionsAvailable(pkg) => Some(format!(
                "Package '{}' exists but has no available versions.\n   \
                 It may have been yanked. Check: {}",
                style(pkg).yellow(),
                style(format!("spn info {}", pkg)).cyan()
            )),

            SpnError::YamlError(_) => Some(format!(
                "Check your YAML syntax. Common issues:\n   \
                 {} Incorrect indentation (use 2 spaces)\n   \
                 {} Missing colons after keys\n   \
                 {} Unquoted special characters",
                style("•").dim(),
                style("•").dim(),
                style("•").dim()
            )),

            SpnError::TomlError(_) => Some(format!(
                "Check your TOML syntax. Common issues:\n   \
                 {} Missing quotes around strings\n   \
                 {} Incorrect table headers [section]\n   \
                 {} Duplicate keys",
                style("•").dim(),
                style("•").dim(),
                style("•").dim()
            )),

            SpnError::IoError(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Some(
                    "File or directory not found.\n   \
                         Check the path exists and you have access permissions."
                        .to_string(),
                ),
                std::io::ErrorKind::PermissionDenied => Some(format!(
                    "Permission denied.\n   \
                         Try running with {} or check file permissions.",
                    style("sudo").cyan()
                )),
                std::io::ErrorKind::AlreadyExists => {
                    Some("File or directory already exists.".to_string())
                }
                _ => None,
            },

            _ => None,
        }
    }

    /// Print the error with optional help message to stderr.
    pub fn print(&self) {
        eprintln!();
        eprintln!("  {} {}", style("✗").red().bold(), style(self).red());

        if let Some(help) = self.help() {
            eprintln!();
            eprintln!("  {} {}", style("→").cyan(), style(help).dim());
        }
        eprintln!();
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
    fn test_io_error_not_found_has_help() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = SpnError::IoError(io_err);
        let help = err.help();
        assert!(help.is_some());
        assert!(help.unwrap().contains("not found"));
    }

    #[test]
    fn test_io_error_permission_denied_has_help() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = SpnError::IoError(io_err);
        let help = err.help();
        assert!(help.is_some());
        assert!(help.unwrap().contains("Permission denied"));
    }

    #[test]
    fn test_io_error_generic_no_help() {
        // Generic IO errors still don't have specific help
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "something went wrong");
        let err = SpnError::IoError(io_err);
        assert!(err.help().is_none());
    }
}
