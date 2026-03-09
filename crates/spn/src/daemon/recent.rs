//! Recent projects tracking for Lite C watch scope.
//!
//! Tracks the 5 most recently used project directories to enable
//! file watching on project-level MCP configs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{Result, SpnError};

/// Maximum number of recent projects to track.
const MAX_RECENT_PROJECTS: usize = 5;

/// A project that has been recently accessed via spn commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    /// Absolute path to the project root.
    pub path: PathBuf,
    /// When the project was last accessed.
    pub last_used: DateTime<Utc>,
}

/// Tracks recently used projects for file watching.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecentProjects {
    /// Maximum projects to track (default: 5).
    #[serde(default = "default_max_projects")]
    pub max_projects: usize,
    /// List of recent projects, sorted by last_used (most recent first).
    pub projects: Vec<RecentProject>,
}

fn default_max_projects() -> usize {
    MAX_RECENT_PROJECTS
}

impl RecentProjects {
    /// Create a new empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_projects: MAX_RECENT_PROJECTS,
            projects: Vec::new(),
        }
    }

    /// Get the path to the recent projects file (~/.spn/recent.yaml).
    fn file_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| SpnError::ConfigError("HOME not set".into()))?;
        Ok(home.join(".spn").join("recent.yaml"))
    }

    /// Load recent projects from ~/.spn/recent.yaml.
    ///
    /// Returns empty tracker if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::file_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| SpnError::ConfigError(format!("Failed to read recent.yaml: {e}")))?;

        serde_yaml::from_str(&content)
            .map_err(|e| SpnError::ConfigError(format!("Failed to parse recent.yaml: {e}")))
    }

    /// Save recent projects to ~/.spn/recent.yaml.
    pub fn save(&self) -> Result<()> {
        let path = Self::file_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SpnError::ConfigError(format!("Failed to create .spn dir: {e}")))?;
        }

        let content = serde_yaml::to_string(self)
            .map_err(|e| SpnError::ConfigError(format!("Failed to serialize recent.yaml: {e}")))?;

        std::fs::write(&path, content)
            .map_err(|e| SpnError::ConfigError(format!("Failed to write recent.yaml: {e}")))?;

        Ok(())
    }

    /// Add or update a project (moves it to the top of the list).
    ///
    /// If the project is already tracked, its `last_used` is updated.
    /// If the list exceeds `max_projects`, the oldest project is removed.
    pub fn touch(&mut self, path: PathBuf) {
        let now = Utc::now();

        // Canonicalize path if possible
        let canonical = path.canonicalize().unwrap_or(path);

        // Remove if already exists
        self.projects.retain(|p| p.path != canonical);

        // Add to front
        self.projects.insert(
            0,
            RecentProject {
                path: canonical,
                last_used: now,
            },
        );

        // Enforce max limit
        if self.projects.len() > self.max_projects {
            self.projects.truncate(self.max_projects);
        }
    }

    /// Get all project paths for watching.
    ///
    /// Returns paths of all tracked projects, most recent first.
    #[must_use]
    pub fn watch_paths(&self) -> Vec<PathBuf> {
        self.projects.iter().map(|p| p.path.clone()).collect()
    }

    /// Remove projects whose directories no longer exist.
    pub fn cleanup(&mut self) {
        self.projects.retain(|p| p.path.exists());
    }

    /// Check if a project is tracked.
    #[must_use]
    #[allow(dead_code)] // Used in tests
    pub fn contains(&self, path: &Path) -> bool {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.projects.iter().any(|p| p.path == canonical)
    }

    /// Get the number of tracked projects.
    #[must_use]
    #[allow(dead_code)] // Used in tests and future daemon status API
    pub fn len(&self) -> usize {
        self.projects.len()
    }

    /// Check if there are no tracked projects.
    #[must_use]
    #[allow(dead_code)] // Used in tests and future daemon status API
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_empty() {
        let recent = RecentProjects::new();
        assert!(recent.is_empty());
        assert_eq!(recent.max_projects, MAX_RECENT_PROJECTS);
    }

    #[test]
    fn test_touch_new_project() {
        let mut recent = RecentProjects::new();
        let path = PathBuf::from("/tmp/test-project");

        recent.touch(path.clone());

        assert_eq!(recent.len(), 1);
        assert!(recent.contains(&path));
    }

    #[test]
    fn test_touch_existing_project_moves_to_top() {
        let mut recent = RecentProjects::new();
        let path1 = PathBuf::from("/tmp/project1");
        let path2 = PathBuf::from("/tmp/project2");
        let path3 = PathBuf::from("/tmp/project3");

        recent.touch(path1.clone());
        recent.touch(path2.clone());
        recent.touch(path3.clone());

        // path3 should be first
        assert_eq!(recent.projects[0].path, path3);

        // Touch path1 again - should move to top
        recent.touch(path1.clone());
        assert_eq!(recent.projects[0].path, path1);
        assert_eq!(recent.len(), 3); // No duplicates
    }

    #[test]
    fn test_max_projects_enforced() {
        let mut recent = RecentProjects::new();
        recent.max_projects = 3;

        for i in 0..5 {
            recent.touch(PathBuf::from(format!("/tmp/project{i}")));
        }

        assert_eq!(recent.len(), 3);
        // Most recent (project4) should be first
        assert_eq!(recent.projects[0].path, PathBuf::from("/tmp/project4"));
    }

    #[test]
    fn test_cleanup_removes_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path().to_path_buf();
        let nonexistent = PathBuf::from("/nonexistent/path/12345");

        let mut recent = RecentProjects::new();
        recent.touch(existing_path.clone());
        recent.touch(nonexistent);

        assert_eq!(recent.len(), 2);

        recent.cleanup();

        assert_eq!(recent.len(), 1);
        assert!(recent.contains(&existing_path));
    }

    #[test]
    fn test_watch_paths() {
        let mut recent = RecentProjects::new();
        recent.touch(PathBuf::from("/tmp/a"));
        recent.touch(PathBuf::from("/tmp/b"));

        let paths = recent.watch_paths();

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("/tmp/b"));
        assert_eq!(paths[1], PathBuf::from("/tmp/a"));
    }
}
