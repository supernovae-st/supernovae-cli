//! Backup error and info types.

use std::path::PathBuf;

use super::BackupManifest;

/// Backup operation errors.
#[derive(Debug)]
pub enum BackupError {
    /// I/O error during backup/restore
    Io(std::io::Error),
    /// Backup directory not found or not accessible
    BackupDirNotFound(PathBuf),
    /// Backup file not found
    BackupNotFound(PathBuf),
    /// No backups available
    NoBackupsFound,
    /// Subsystem not available for backup
    NotAvailable(String),
    /// Invalid backup archive (corrupted or wrong format)
    InvalidArchive(String),
    /// Manifest parsing error
    ManifestError(String),
    /// Checksum mismatch during verification
    ChecksumMismatch {
        /// File that failed verification
        file: String,
        /// Expected checksum
        expected: String,
        /// Actual computed checksum
        actual: String,
    },
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::BackupDirNotFound(path) => {
                write!(f, "Backup directory not found: {}", path.display())
            }
            Self::BackupNotFound(path) => write!(f, "Backup not found: {}", path.display()),
            Self::NoBackupsFound => write!(f, "No backups found"),
            Self::NotAvailable(msg) => write!(f, "Subsystem not available: {}", msg),
            Self::InvalidArchive(msg) => write!(f, "Invalid backup archive: {}", msg),
            Self::ManifestError(msg) => write!(f, "Manifest error: {}", msg),
            Self::ChecksumMismatch {
                file,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Checksum mismatch for {}: expected {}, got {}",
                    file, expected, actual
                )
            }
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

impl From<std::io::Error> for BackupError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Information about a completed backup.
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Path to the backup archive
    pub path: PathBuf,
    /// When the backup was created (ISO 8601)
    pub timestamp: String,
    /// Size of the backup archive in bytes
    pub size_bytes: u64,
    /// Backup manifest
    pub manifest: BackupManifest,
}

/// Information about a completed restore.
#[derive(Debug, Clone)]
pub struct RestoreInfo {
    /// Path to the backup that was restored
    pub backup_path: PathBuf,
    /// When the restore was performed (ISO 8601)
    pub restored_at: String,
    /// Manifest from the restored backup
    pub manifest: BackupManifest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BackupError::NoBackupsFound;
        assert_eq!(err.to_string(), "No backups found");

        let err = BackupError::NotAvailable("Neo4j not running".to_string());
        assert!(err.to_string().contains("Neo4j not running"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let backup_err: BackupError = io_err.into();
        assert!(matches!(backup_err, BackupError::Io(_)));
    }
}
