//! Sync configuration management.
//!
//! Provides persistent sync configuration stored in ~/.spn/sync.json.
//! Used by `spn sync` command and daemon auto-sync.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::types::IdeTarget;

/// Sync configuration stored in ~/.spn/sync.json.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enabled IDE targets for automatic sync.
    #[serde(default)]
    pub enabled_targets: HashSet<IdeTarget>,

    /// Project-specific sync settings.
    #[serde(default)]
    pub projects: Vec<ProjectSyncConfig>,

    /// Last sync timestamp (ISO 8601).
    #[serde(default)]
    pub last_sync: Option<String>,
}

/// Project-specific sync configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSyncConfig {
    /// Project root path.
    pub path: PathBuf,

    /// IDEs to sync for this project.
    pub targets: HashSet<IdeTarget>,

    /// Last sync timestamp.
    pub last_sync: Option<String>,
}

impl SyncConfig {
    /// Default config file path.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".spn")
            .join("sync.json")
    }

    /// Load sync config from file.
    pub fn load() -> Result<Self, std::io::Error> {
        let path = Self::default_path();
        Self::from_file(&path)
    }

    /// Load from a specific file.
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Save sync config to file.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::default_path();
        self.write_to_file(&path)
    }

    /// Write to a specific file.
    pub fn write_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    /// Enable sync for an IDE target.
    pub fn enable(&mut self, target: IdeTarget) {
        self.enabled_targets.insert(target);
    }

    /// Disable sync for an IDE target.
    pub fn disable(&mut self, target: IdeTarget) {
        self.enabled_targets.remove(&target);
    }

    /// Check if an IDE target is enabled.
    #[allow(dead_code)] // Phase 2: auto-sync uses this
    pub fn is_enabled(&self, target: IdeTarget) -> bool {
        self.enabled_targets.contains(&target)
    }

    /// Get or create project config.
    #[allow(dead_code)] // Phase 2: project-specific sync
    pub fn project_config(&mut self, path: &Path) -> &mut ProjectSyncConfig {
        let path = path.to_path_buf();

        // Find existing
        if let Some(idx) = self.projects.iter().position(|p| p.path == path) {
            return &mut self.projects[idx];
        }

        // Create new
        self.projects.push(ProjectSyncConfig {
            path,
            targets: HashSet::new(),
            last_sync: None,
        });

        // SAFETY: We just pushed to the vector, so last_mut() is guaranteed to return Some
        self.projects
            .last_mut()
            .expect("just pushed; vec cannot be empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sync_config_defaults() {
        let config = SyncConfig::default();
        assert!(config.enabled_targets.is_empty());
        assert!(config.projects.is_empty());
        assert!(config.last_sync.is_none());
    }

    #[test]
    fn test_enable_disable_target() {
        let mut config = SyncConfig::default();

        config.enable(IdeTarget::ClaudeCode);
        assert!(config.is_enabled(IdeTarget::ClaudeCode));
        assert!(!config.is_enabled(IdeTarget::Cursor));

        config.disable(IdeTarget::ClaudeCode);
        assert!(!config.is_enabled(IdeTarget::ClaudeCode));
    }

    #[test]
    fn test_save_load_config() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("sync.json");

        let mut config = SyncConfig::default();
        config.enable(IdeTarget::ClaudeCode);
        config.enable(IdeTarget::Cursor);
        config.last_sync = Some("2026-02-28T12:00:00Z".to_string());

        config.write_to_file(&path).unwrap();

        let loaded = SyncConfig::from_file(&path).unwrap();
        assert!(loaded.is_enabled(IdeTarget::ClaudeCode));
        assert!(loaded.is_enabled(IdeTarget::Cursor));
        assert!(!loaded.is_enabled(IdeTarget::VsCode));
        assert_eq!(loaded.last_sync, Some("2026-02-28T12:00:00Z".to_string()));
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = PathBuf::from("/nonexistent/sync.json");
        let config = SyncConfig::from_file(&path).unwrap();
        assert!(config.enabled_targets.is_empty());
    }
}
