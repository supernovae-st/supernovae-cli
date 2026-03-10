//! Error types for spn-native.

use spn_core::BackendError;
use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for spn-native operations.
pub type Result<T> = std::result::Result<T, NativeError>;

/// Errors that can occur in spn-native operations.
#[derive(Error, Debug)]
pub enum NativeError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Model not found on HuggingFace.
    #[error("Model not found: {repo}/{filename}")]
    ModelNotFound {
        /// HuggingFace repository.
        repo: String,
        /// Requested filename.
        filename: String,
    },

    /// Checksum verification failed.
    #[error("Checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// File path.
        path: PathBuf,
        /// Expected SHA256.
        expected: String,
        /// Actual SHA256.
        actual: String,
    },

    /// Invalid model configuration.
    #[error("Invalid model configuration: {0}")]
    InvalidConfig(String),

    /// Download was interrupted.
    #[error("Download interrupted: {0}")]
    Interrupted(String),

    /// Storage directory error.
    #[error("Storage directory error: {0}")]
    StorageDir(String),

    /// JSON parsing error.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<NativeError> for BackendError {
    fn from(err: NativeError) -> Self {
        match err {
            NativeError::Http(e) => BackendError::NetworkError(e.to_string()),
            NativeError::Io(e) => BackendError::StorageError(e.to_string()),
            NativeError::ModelNotFound { repo, filename } => {
                BackendError::ModelNotFound(format!("{repo}/{filename}"))
            }
            NativeError::ChecksumMismatch {
                expected, actual, ..
            } => BackendError::ChecksumError { expected, actual },
            NativeError::InvalidConfig(msg) => BackendError::InvalidConfig(msg),
            NativeError::Interrupted(msg) => BackendError::DownloadError(msg),
            NativeError::StorageDir(msg) => BackendError::StorageError(msg),
            NativeError::Json(e) => BackendError::ParseError(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = NativeError::ModelNotFound {
            repo: "test/repo".to_string(),
            filename: "model.gguf".to_string(),
        };
        assert!(err.to_string().contains("test/repo"));
        assert!(err.to_string().contains("model.gguf"));
    }

    #[test]
    fn test_checksum_error() {
        let err = NativeError::ChecksumMismatch {
            path: PathBuf::from("/tmp/model.gguf"),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert!(err.to_string().contains("abc123"));
        assert!(err.to_string().contains("def456"));
    }

    #[test]
    fn test_into_backend_error() {
        let err = NativeError::InvalidConfig("bad config".to_string());
        let backend_err: BackendError = err.into();
        assert!(matches!(backend_err, BackendError::InvalidConfig(_)));
    }
}
