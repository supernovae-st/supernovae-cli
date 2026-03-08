//! Backup adapter trait and types.

use std::path::Path;

use super::error::BackupError;
use super::manifest::{NikaContents, NovaNetContents, SpnContents};

/// Adapter contents enum - what each adapter returns.
#[derive(Debug, Clone)]
pub enum AdapterContents {
    NovaNet(NovaNetContents),
    Nika(NikaContents),
    Spn(SpnContents),
}

/// Trait for subsystem backup adapters.
///
/// Each subsystem (NovaNet, Nika, spn) implements this trait
/// to provide backup and restore functionality.
pub trait BackupAdapter {
    /// Name of the subsystem.
    fn name(&self) -> &str;

    /// Whether this subsystem is available for backup.
    fn is_available(&self) -> bool;

    /// Get the current version of this subsystem (if known).
    fn version(&self) -> Option<String>;

    /// Collect data to backup into the staging directory.
    ///
    /// The adapter should copy relevant files to subdirectories
    /// within the staging directory.
    fn collect(&self, staging_dir: &Path) -> Result<AdapterContents, BackupError>;

    /// Restore data from a backup staging directory.
    fn restore(&self, staging_dir: &Path) -> Result<(), BackupError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAdapter {
        name: String,
        available: bool,
    }

    impl BackupAdapter for MockAdapter {
        fn name(&self) -> &str {
            &self.name
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn version(&self) -> Option<String> {
            Some("1.0.0".to_string())
        }

        fn collect(&self, _staging_dir: &Path) -> Result<AdapterContents, BackupError> {
            Ok(AdapterContents::Spn(SpnContents::default()))
        }

        fn restore(&self, _staging_dir: &Path) -> Result<(), BackupError> {
            Ok(())
        }
    }

    #[test]
    fn test_mock_adapter() {
        let adapter = MockAdapter {
            name: "test".to_string(),
            available: true,
        };

        assert_eq!(adapter.name(), "test");
        assert!(adapter.is_available());
        assert_eq!(adapter.version(), Some("1.0.0".to_string()));
    }
}
