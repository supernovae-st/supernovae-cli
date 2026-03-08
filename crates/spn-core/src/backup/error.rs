//! Backup error types.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Errors that can occur during backup operations.
#[derive(Debug)]
pub enum BackupError {
    /// IO operation failed.
    Io(io::Error),
    /// System is not available for backup.
    NotAvailable(String),
    /// Archive is invalid or corrupted.
    InvalidArchive(String),
    /// Manifest parsing or validation error.
    ManifestError(String),
    /// No backups found.
    NoBackupsFound,
    /// Specific backup not found.
    BackupNotFound(PathBuf),
    /// Version mismatch between backup and current version.
    VersionMismatch {
        backup_version: String,
        current_version: String,
    },
    /// Checksum mismatch.
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    /// Operation cancelled by user.
    Cancelled,
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::NotAvailable(msg) => write!(f, "Not available: {}", msg),
            Self::InvalidArchive(msg) => write!(f, "Invalid archive: {}", msg),
            Self::ManifestError(msg) => write!(f, "Manifest error: {}", msg),
            Self::NoBackupsFound => write!(f, "No backups found"),
            Self::BackupNotFound(path) => write!(f, "Backup not found: {}", path.display()),
            Self::VersionMismatch {
                backup_version,
                current_version,
            } => write!(
                f,
                "Version mismatch: backup={}, current={}",
                backup_version, current_version
            ),
            Self::ChecksumMismatch {
                file,
                expected,
                actual,
            } => write!(
                f,
                "Checksum mismatch for {}: expected={}, actual={}",
                file, expected, actual
            ),
            Self::Cancelled => write!(f, "Operation cancelled"),
        }
    }
}

impl std::error::Error for BackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for BackupError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = BackupError::Io(io_err);
        assert!(format!("{}", err).contains("IO error"));
    }

    #[test]
    fn test_version_mismatch_display() {
        let err = BackupError::VersionMismatch {
            backup_version: "0.15.0".to_string(),
            current_version: "0.16.0".to_string(),
        };
        assert!(format!("{}", err).contains("0.15.0"));
        assert!(format!("{}", err).contains("0.16.0"));
    }

    #[test]
    fn test_checksum_mismatch_display() {
        let err = BackupError::ChecksumMismatch {
            file: "test.txt".to_string(),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test.txt"));
        assert!(msg.contains("abc123"));
        assert!(msg.contains("def456"));
    }
}
