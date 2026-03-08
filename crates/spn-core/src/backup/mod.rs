//! Unified backup system types for SuperNovae ecosystem.
//!
//! This module provides the core types and traits for the backup system.
//! The actual implementation (file operations, archiving) lives in spn-cli
//! to keep spn-core dependency-free.
//!
//! # Architecture
//!
//! ```text
//! spn-core (types only)      spn-cli (implementation)
//! ├── BackupError            ├── BackupManager
//! ├── BackupManifest         ├── SpnAdapter
//! ├── BackupAdapter trait    ├── NikaAdapter
//! └── Content types          └── NovaNetAdapter
//! ```

mod adapter;
mod error;
mod manifest;

pub use adapter::{AdapterContents, BackupAdapter};
pub use error::BackupError;
pub use manifest::{
    BackupContents, BackupManifest, ComponentVersions, NikaContents, NovaNetContents, SpnContents,
};

use std::path::PathBuf;

/// Information about a backup archive.
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Path to the backup archive.
    pub path: PathBuf,
    /// When the backup was created.
    pub timestamp: String,
    /// Size of the archive in bytes.
    pub size_bytes: u64,
    /// Parsed manifest from the backup.
    pub manifest: BackupManifest,
}

/// Information about a restore operation.
#[derive(Debug, Clone)]
pub struct RestoreInfo {
    /// Path to the backup that was restored.
    pub backup_path: PathBuf,
    /// When the restore was performed.
    pub restored_at: String,
    /// Manifest from the restored backup.
    pub manifest: BackupManifest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_info() {
        let manifest = BackupManifest::new(None);
        let info = BackupInfo {
            path: PathBuf::from("/tmp/backup.tar.gz"),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            size_bytes: 1024,
            manifest,
        };

        assert_eq!(info.path.to_string_lossy(), "/tmp/backup.tar.gz");
        assert_eq!(info.size_bytes, 1024);
    }
}
