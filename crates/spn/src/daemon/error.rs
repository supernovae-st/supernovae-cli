//! Daemon-specific error types.
//!
//! TODO(v0.16): Add more granular error variants

#![allow(dead_code)]

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur in the daemon.
#[derive(Debug, Error)]
pub enum DaemonError {
    /// Failed to create socket directory
    #[error("Failed to create socket directory {path}: {source}")]
    CreateDirFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to bind to socket
    #[error("Failed to bind to socket {path}: {source}")]
    BindFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to set socket permissions
    #[error("Failed to set socket permissions on {path}: {source}")]
    SetPermissionsFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Another daemon instance is already running
    #[error("Daemon already running (PID file: {pid_file})")]
    AlreadyRunning { pid_file: PathBuf },

    /// Failed to create PID file
    #[error("Failed to create PID file {path}: {source}")]
    PidFileFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to acquire PID file lock
    #[error("Failed to acquire lock on PID file {path}: {source}")]
    LockFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Peer credential verification failed
    #[error("Peer credential verification failed: {reason}")]
    PeerCredentialsFailed { reason: String },

    /// Unauthorized connection attempt
    #[error("Unauthorized connection: process UID {peer_uid} != daemon UID {daemon_uid}")]
    Unauthorized { peer_uid: u32, daemon_uid: u32 },

    /// Secret not found
    #[error("Secret not found for provider: {provider}")]
    SecretNotFound { provider: String },

    /// Keychain access error
    #[error("Keychain error: {0}")]
    KeychainError(String),

    /// Memory lock failed
    #[error("Failed to lock memory: {0}")]
    MemoryLockFailed(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Server shutdown requested
    #[error("Server shutdown requested")]
    Shutdown,

    /// Stale socket file exists
    #[error("Stale socket file exists at {path}, removing")]
    StaleSocket { path: PathBuf },

    /// Configuration error (e.g., HOME not set)
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
