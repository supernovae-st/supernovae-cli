//! Error types for the SuperNovae CLI.

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

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Alias for backward compatibility and consistency.
pub type CliError = SpnError;
