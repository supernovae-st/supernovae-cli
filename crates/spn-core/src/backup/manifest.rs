//! Backup manifest types.
//!
//! The manifest stores metadata about a backup, including versions,
//! timestamps, and checksums for integrity verification.

use std::collections::HashMap;

/// Backup manifest containing metadata about the backup.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BackupManifest {
    /// Manifest format version (e.g., "1.0.0")
    pub version: String,

    /// When the backup was created (ISO 8601 format)
    pub created_at: String,

    /// Optional user-provided label
    pub label: Option<String>,

    /// Machine hostname
    pub hostname: String,

    /// SuperNovae component versions
    pub versions: ComponentVersions,

    /// SHA-256 checksums for each backed-up file (relative path -> hex checksum)
    pub checksums: HashMap<String, String>,

    /// What was included in the backup
    pub contents: BackupContents,
}

impl BackupManifest {
    /// Create a new manifest with default values.
    pub fn new(hostname: String, spn_version: String) -> Self {
        Self {
            version: "1.0.0".to_string(),
            created_at: String::new(),
            label: None,
            hostname,
            versions: ComponentVersions {
                novanet: None,
                nika: None,
                spn: spn_version,
            },
            checksums: HashMap::new(),
            contents: BackupContents::default(),
        }
    }
}

/// Component version information.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentVersions {
    /// NovaNet version if available
    pub novanet: Option<String>,
    /// Nika version if available
    pub nika: Option<String>,
    /// spn version (always present)
    pub spn: String,
}

/// Summary of what each subsystem contributed to the backup.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BackupContents {
    /// NovaNet backup contents
    pub novanet: NovaNetContents,
    /// Nika backup contents
    pub nika: NikaContents,
    /// spn backup contents
    pub spn: SpnContents,
}

/// NovaNet-specific backup contents.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NovaNetContents {
    /// Number of schema YAML files backed up
    pub schema_files: u32,
    /// Number of seed YAML files backed up
    pub seed_files: u32,
    /// Whether a Neo4j dump was included (optional, v2 feature)
    pub neo4j_dump: bool,
}

/// Nika-specific backup contents.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NikaContents {
    /// Number of workflow YAML files backed up
    pub workflow_files: u32,
    /// Number of chat sessions backed up
    pub session_count: u32,
    /// Number of execution traces backed up
    pub trace_count: u32,
}

/// spn-specific backup contents.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpnContents {
    /// Whether config.toml was backed up
    pub has_config: bool,
    /// Whether mcp.yaml was backed up
    pub has_mcp_yaml: bool,
    /// Whether jobs.json was backed up
    pub has_jobs: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_new() {
        let manifest = BackupManifest::new("test-host".to_string(), "0.15.0".to_string());
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.hostname, "test-host");
        assert_eq!(manifest.versions.spn, "0.15.0");
        assert!(manifest.versions.novanet.is_none());
    }

    #[test]
    fn test_backup_contents_default() {
        let contents = BackupContents::default();
        assert_eq!(contents.novanet.schema_files, 0);
        assert!(!contents.spn.has_config);
    }
}
