//! Error types for spn-client.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when communicating with the spn daemon.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to connect to the daemon socket.
    #[error("Failed to connect to spn daemon at {path}: {source}")]
    ConnectionFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Not connected to the daemon (shouldn't happen in normal use).
    #[error("Not connected to spn daemon")]
    NotConnected,

    /// The requested secret was not found.
    #[error("Secret not found for provider '{provider}': {details}")]
    SecretNotFound { provider: String, details: String },

    /// The daemon returned an error.
    #[error("Daemon error: {0}")]
    DaemonError(String),

    /// Unexpected response from daemon.
    #[error("Unexpected response from daemon")]
    UnexpectedResponse,

    /// Response too large (potential attack or bug).
    #[error("Response too large: {0} bytes")]
    ResponseTooLarge(usize),

    /// IO error during communication.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to serialize request.
    #[error("Failed to serialize request: {0}")]
    SerializationError(#[source] serde_json::Error),

    /// Failed to deserialize response.
    #[error("Failed to deserialize response: {0}")]
    DeserializationError(#[source] serde_json::Error),
}
