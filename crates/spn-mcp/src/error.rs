//! Error types for spn-mcp.

use thiserror::Error;

/// Result type alias for spn-mcp operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in spn-mcp.
#[derive(Debug, Error)]
pub enum Error {
    /// Configuration file not found.
    #[error("API configuration not found: {0}")]
    ConfigNotFound(String),

    /// Configuration parsing error.
    #[error("Failed to parse configuration: {0}")]
    ConfigParse(#[from] serde_yaml::Error),

    /// Configuration validation error.
    #[error("Invalid configuration: {0}")]
    ConfigValidation(String),

    /// Credential resolution error.
    #[error("Failed to resolve credential '{0}': {1}")]
    Credential(String, String),

    /// HTTP request error.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Template rendering error.
    #[error("Template rendering failed: {0}")]
    Template(#[from] tera::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// MCP protocol error.
    #[error("MCP error: {0}")]
    Mcp(String),
}
