//! Backup manifest types.
//!
//! The manifest is stored as JSON inside each backup archive
//! and describes the contents and metadata of the backup.

use std::collections::HashMap;

/// Backup manifest stored inside each backup archive.
#[derive(Debug, Clone)]
pub struct BackupManifest {
    /// Manifest format version (for future compatibility).
    pub version: String,
    /// Timestamp when backup was created (RFC 3339).
    pub created_at: String,
    /// Optional user-provided label.
    pub label: Option<String>,
    /// Hostname where backup was created.
    pub hostname: String,
    /// Component versions at time of backup.
    pub versions: ComponentVersions,
    /// File checksums (path -> SHA256).
    pub checksums: HashMap<String, String>,
    /// What was backed up.
    pub contents: BackupContents,
}

impl BackupManifest {
    /// Create a new manifest with current timestamp.
    pub fn new(label: Option<String>) -> Self {
        Self {
            version: "1.0".to_string(),
            created_at: chrono_lite_now(),
            label,
            hostname: String::new(),
            versions: ComponentVersions::default(),
            checksums: HashMap::new(),
            contents: BackupContents::default(),
        }
    }

    /// Set the hostname.
    pub fn set_hostname(&mut self, hostname: String) {
        self.hostname = hostname;
    }

    /// Serialize to JSON string (manual implementation to avoid serde in spn-core).
    pub fn to_json(&self) -> String {
        let checksums_json = self
            .checksums
            .iter()
            .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            r#"{{
  "version": "{}",
  "created_at": "{}",
  "label": {},
  "hostname": "{}",
  "versions": {{
    "novanet": {},
    "nika": {},
    "spn": "{}"
  }},
  "checksums": {{
{}
  }},
  "contents": {{
    "novanet": {{
      "schema_files": {},
      "seed_files": {},
      "neo4j_dump": {}
    }},
    "nika": {{
      "workflow_files": {},
      "session_count": {},
      "trace_count": {}
    }},
    "spn": {{
      "has_config": {},
      "has_mcp_yaml": {},
      "has_jobs": {}
    }}
  }}
}}"#,
            self.version,
            self.created_at,
            self.label
                .as_ref()
                .map(|l| format!("\"{}\"", l))
                .unwrap_or_else(|| "null".to_string()),
            self.hostname,
            self.versions
                .novanet
                .as_ref()
                .map(|v| format!("\"{}\"", v))
                .unwrap_or_else(|| "null".to_string()),
            self.versions
                .nika
                .as_ref()
                .map(|v| format!("\"{}\"", v))
                .unwrap_or_else(|| "null".to_string()),
            env!("CARGO_PKG_VERSION"),
            checksums_json,
            self.contents.novanet.schema_files,
            self.contents.novanet.seed_files,
            self.contents.novanet.neo4j_dump,
            self.contents.nika.workflow_files,
            self.contents.nika.session_count,
            self.contents.nika.trace_count,
            self.contents.spn.has_config,
            self.contents.spn.has_mcp_yaml,
            self.contents.spn.has_jobs,
        )
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self, String> {
        // Simple JSON parser for the manifest format
        let version = extract_string(json, "version").unwrap_or_else(|| "1.0".to_string());
        let created_at =
            extract_string(json, "created_at").ok_or("Missing created_at")?;
        let label = extract_string(json, "label");
        let hostname = extract_string(json, "hostname").unwrap_or_default();

        // Parse versions block
        let novanet_version = extract_nested_string(json, "versions", "novanet");
        let nika_version = extract_nested_string(json, "versions", "nika");

        // Parse contents
        let schema_files = extract_nested_u32(json, "novanet", "schema_files").unwrap_or(0);
        let seed_files = extract_nested_u32(json, "novanet", "seed_files").unwrap_or(0);
        let neo4j_dump = extract_nested_bool(json, "novanet", "neo4j_dump").unwrap_or(false);

        let workflow_files = extract_nested_u32(json, "nika", "workflow_files").unwrap_or(0);
        let session_count = extract_nested_u32(json, "nika", "session_count").unwrap_or(0);
        let trace_count = extract_nested_u32(json, "nika", "trace_count").unwrap_or(0);

        let has_config = extract_nested_bool(json, "spn", "has_config").unwrap_or(false);
        let has_mcp_yaml = extract_nested_bool(json, "spn", "has_mcp_yaml").unwrap_or(false);
        let has_jobs = extract_nested_bool(json, "spn", "has_jobs").unwrap_or(false);

        Ok(Self {
            version,
            created_at,
            label,
            hostname,
            versions: ComponentVersions {
                novanet: novanet_version,
                nika: nika_version,
            },
            checksums: HashMap::new(),
            contents: BackupContents {
                novanet: NovaNetContents {
                    schema_files,
                    seed_files,
                    neo4j_dump,
                },
                nika: NikaContents {
                    workflow_files,
                    session_count,
                    trace_count,
                },
                spn: SpnContents {
                    has_config,
                    has_mcp_yaml,
                    has_jobs,
                },
            },
        })
    }
}

/// Component versions captured at backup time.
#[derive(Debug, Clone, Default)]
pub struct ComponentVersions {
    pub novanet: Option<String>,
    pub nika: Option<String>,
}

/// Describes what was backed up.
#[derive(Debug, Clone, Default)]
pub struct BackupContents {
    pub novanet: NovaNetContents,
    pub nika: NikaContents,
    pub spn: SpnContents,
}

/// NovaNet backup contents.
#[derive(Debug, Clone, Default)]
pub struct NovaNetContents {
    pub schema_files: u32,
    pub seed_files: u32,
    pub neo4j_dump: bool,
}

/// Nika backup contents.
#[derive(Debug, Clone, Default)]
pub struct NikaContents {
    pub workflow_files: u32,
    pub session_count: u32,
    pub trace_count: u32,
}

/// spn backup contents.
#[derive(Debug, Clone, Default)]
pub struct SpnContents {
    pub has_config: bool,
    pub has_mcp_yaml: bool,
    pub has_jobs: bool,
}

// Helper functions for simple JSON parsing

fn extract_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\": \"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_nested_string(json: &str, block: &str, key: &str) -> Option<String> {
    // Find the block first
    let block_pattern = format!("\"{}\": {{", block);
    let block_start = json.find(&block_pattern)?;
    let rest = &json[block_start..];
    let block_end = rest.find('}')?;
    let block_content = &rest[..block_end];

    // Now find the key within the block
    let pattern = format!("\"{}\": \"", key);
    let start = block_content.find(&pattern)? + pattern.len();
    let key_rest = &block_content[start..];
    let end = key_rest.find('"')?;
    Some(key_rest[..end].to_string())
}

fn extract_nested_u32(json: &str, block: &str, key: &str) -> Option<u32> {
    let block_pattern = format!("\"{}\": {{", block);
    let block_start = json.find(&block_pattern)?;
    let rest = &json[block_start..];
    let block_end = rest.find('}')?;
    let block_content = &rest[..block_end];

    let pattern = format!("\"{}\": ", key);
    let start = block_content.find(&pattern)? + pattern.len();
    let key_rest = &block_content[start..];

    // Parse until comma, newline, or }
    let end = key_rest
        .find(|c: char| c == ',' || c == '\n' || c == '}')
        .unwrap_or(key_rest.len());
    key_rest[..end].trim().parse().ok()
}

fn extract_nested_bool(json: &str, block: &str, key: &str) -> Option<bool> {
    let block_pattern = format!("\"{}\": {{", block);
    let block_start = json.find(&block_pattern)?;
    let rest = &json[block_start..];
    let block_end = rest.find('}')?;
    let block_content = &rest[..block_end];

    let pattern = format!("\"{}\": ", key);
    let start = block_content.find(&pattern)? + pattern.len();
    let key_rest = &block_content[start..];

    let end = key_rest
        .find(|c: char| c == ',' || c == '\n' || c == '}')
        .unwrap_or(key_rest.len());
    let value = key_rest[..end].trim();
    Some(value == "true")
}

/// Get current timestamp in RFC 3339 format without chrono dependency.
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = now.as_secs();

    // Calculate date components (simplified, doesn't handle all edge cases)
    let days_since_epoch = secs / 86400;
    let remaining_secs = secs % 86400;

    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let seconds = remaining_secs % 60;

    // Simplified date calculation
    let mut year = 1970;
    let mut remaining_days = days_since_epoch as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days in days_in_months {
        if remaining_days < days {
            break;
        }
        remaining_days -= days;
        month += 1;
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_new() {
        let manifest = BackupManifest::new(Some("test".to_string()));
        assert_eq!(manifest.version, "1.0");
        assert_eq!(manifest.label, Some("test".to_string()));
        assert!(!manifest.created_at.is_empty());
    }

    #[test]
    fn test_manifest_to_json() {
        let manifest = BackupManifest::new(Some("test".to_string()));
        let json = manifest.to_json();
        assert!(json.contains("\"version\": \"1.0\""));
        assert!(json.contains("\"label\": \"test\""));
    }

    #[test]
    fn test_manifest_roundtrip() {
        let mut manifest = BackupManifest::new(Some("roundtrip".to_string()));
        manifest.set_hostname("test-host".to_string());
        manifest.contents.novanet.schema_files = 10;
        manifest.contents.nika.workflow_files = 5;

        let json = manifest.to_json();
        let parsed = BackupManifest::from_json(&json).unwrap();

        assert_eq!(parsed.label, Some("roundtrip".to_string()));
        assert_eq!(parsed.hostname, "test-host");
        assert_eq!(parsed.contents.novanet.schema_files, 10);
        assert_eq!(parsed.contents.nika.workflow_files, 5);
    }

    #[test]
    fn test_chrono_lite_now() {
        let now = chrono_lite_now();
        // Should be in RFC 3339 format
        assert!(now.contains("T"));
        assert!(now.ends_with("Z"));
        assert_eq!(now.len(), 20); // YYYY-MM-DDTHH:MM:SSZ
    }
}
