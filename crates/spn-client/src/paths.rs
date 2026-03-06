//! Centralized path management for the ~/.spn directory structure.
//!
//! This module provides a single source of truth for all paths used by the
//! SuperNovae ecosystem, eliminating scattered `dirs::home_dir().join(".spn")`
//! calls throughout the codebase.
//!
//! # Example
//!
//! ```rust,no_run
//! use spn_client::SpnPaths;
//!
//! // Create paths rooted at ~/.spn
//! let paths = SpnPaths::new().expect("HOME directory must be set");
//!
//! // Access specific paths
//! println!("Config: {:?}", paths.config_file());
//! println!("Socket: {:?}", paths.socket_file());
//! println!("Packages: {:?}", paths.packages_dir());
//!
//! // For testing, use a custom root
//! let test_paths = SpnPaths::with_root("/tmp/spn-test".into());
//! ```
//!
//! # Directory Structure
//!
//! ```text
//! ~/.spn/
//! ├── config.toml          # Global user configuration
//! ├── daemon.sock          # Unix socket for IPC
//! ├── daemon.pid           # PID file with flock
//! ├── secrets.env          # API keys (fallback to keychain)
//! ├── state.json           # Package installation state
//! ├── bin/                  # Binary stubs (nika, novanet)
//! ├── packages/             # Installed packages
//! │   └── @scope/name/version/
//! ├── cache/                # Download cache
//! │   └── tarballs/
//! └── registry/             # Registry index cache
//! ```

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error type for path operations.
#[derive(Debug, Error)]
pub enum PathError {
    /// HOME directory is not set or unavailable.
    #[error("HOME directory not found. Set HOME environment variable.")]
    HomeNotFound,

    /// Failed to create a required directory.
    #[error("Failed to create directory {path}: {source}")]
    CreateDirFailed {
        /// The path that could not be created.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
}

/// Centralized path management for the ~/.spn directory structure.
///
/// Provides type-safe access to all paths used by spn-cli, spn-daemon,
/// and other tools in the SuperNovae ecosystem.
#[derive(Debug, Clone)]
pub struct SpnPaths {
    root: PathBuf,
}

impl SpnPaths {
    /// Create paths rooted at the default location (~/.spn).
    ///
    /// Returns an error if the HOME directory is not available.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use spn_client::SpnPaths;
    ///
    /// let paths = SpnPaths::new()?;
    /// println!("Root: {:?}", paths.root());
    /// # Ok::<(), spn_client::PathError>(())
    /// ```
    pub fn new() -> Result<Self, PathError> {
        let home = dirs::home_dir().ok_or(PathError::HomeNotFound)?;
        Ok(Self {
            root: home.join(".spn"),
        })
    }

    /// Create paths with a custom root directory.
    ///
    /// Useful for testing or custom installations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use spn_client::SpnPaths;
    /// use std::path::PathBuf;
    ///
    /// let paths = SpnPaths::with_root(PathBuf::from("/tmp/spn-test"));
    /// assert_eq!(paths.root().to_str().unwrap(), "/tmp/spn-test");
    /// ```
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    // =========================================================================
    // Directory Paths
    // =========================================================================

    /// Root directory (~/.spn).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Binary directory (~/.spn/bin).
    ///
    /// Contains symlinks or stubs for nika, novanet, etc.
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// Packages directory (~/.spn/packages).
    ///
    /// Structure: packages/@scope/name/version/
    pub fn packages_dir(&self) -> PathBuf {
        self.root.join("packages")
    }

    /// Cache directory (~/.spn/cache).
    ///
    /// Contains downloaded tarballs and temporary files.
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// Tarballs cache directory (~/.spn/cache/tarballs).
    pub fn tarballs_dir(&self) -> PathBuf {
        self.cache_dir().join("tarballs")
    }

    /// Registry cache directory (~/.spn/registry).
    ///
    /// Contains cached package index data.
    pub fn registry_dir(&self) -> PathBuf {
        self.root.join("registry")
    }

    // =========================================================================
    // File Paths
    // =========================================================================

    /// Global configuration file (~/.spn/config.toml).
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// Secrets file (~/.spn/secrets.env).
    ///
    /// Alternative to OS keychain for storing API keys.
    pub fn secrets_file(&self) -> PathBuf {
        self.root.join("secrets.env")
    }

    /// Daemon socket file (~/.spn/daemon.sock).
    pub fn socket_file(&self) -> PathBuf {
        self.root.join("daemon.sock")
    }

    /// Daemon PID file (~/.spn/daemon.pid).
    pub fn pid_file(&self) -> PathBuf {
        self.root.join("daemon.pid")
    }

    /// State file (~/.spn/state.json).
    ///
    /// Tracks installed packages and their versions.
    pub fn state_file(&self) -> PathBuf {
        self.root.join("state.json")
    }

    // =========================================================================
    // Package Paths
    // =========================================================================

    /// Get the path for a specific package version.
    ///
    /// # Arguments
    ///
    /// * `name` - Package name (e.g., "@workflows/code-review")
    /// * `version` - Package version (e.g., "1.0.0")
    ///
    /// # Example
    ///
    /// ```rust
    /// use spn_client::SpnPaths;
    /// use std::path::PathBuf;
    ///
    /// let paths = SpnPaths::with_root(PathBuf::from("/home/user/.spn"));
    /// let pkg_path = paths.package_dir("@workflows/code-review", "1.0.0");
    /// assert!(pkg_path.to_string_lossy().contains("@workflows"));
    /// ```
    pub fn package_dir(&self, name: &str, version: &str) -> PathBuf {
        self.packages_dir().join(name).join(version)
    }

    /// Get the path for a binary stub.
    ///
    /// # Arguments
    ///
    /// * `name` - Binary name (e.g., "nika", "novanet")
    pub fn binary(&self, name: &str) -> PathBuf {
        self.bin_dir().join(name)
    }

    // =========================================================================
    // Directory Management
    // =========================================================================

    /// Ensure all required directories exist.
    ///
    /// Creates the following directories if they don't exist:
    /// - ~/.spn/
    /// - ~/.spn/bin/
    /// - ~/.spn/packages/
    /// - ~/.spn/cache/
    /// - ~/.spn/cache/tarballs/
    /// - ~/.spn/registry/
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use spn_client::SpnPaths;
    ///
    /// let paths = SpnPaths::new()?;
    /// paths.ensure_dirs()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn ensure_dirs(&self) -> Result<(), PathError> {
        let dirs = [
            self.root.clone(),
            self.bin_dir(),
            self.packages_dir(),
            self.cache_dir(),
            self.tarballs_dir(),
            self.registry_dir(),
        ];

        for dir in dirs {
            std::fs::create_dir_all(&dir).map_err(|e| PathError::CreateDirFailed {
                path: dir,
                source: e,
            })?;
        }

        Ok(())
    }

    /// Check if the root directory exists.
    pub fn exists(&self) -> bool {
        self.root.exists()
    }
}

impl Default for SpnPaths {
    /// Creates SpnPaths with the default root, panicking if HOME is unavailable.
    ///
    /// **Note:** Prefer `SpnPaths::new()` which returns a Result.
    fn default() -> Self {
        Self::new().expect("HOME directory must be set for SpnPaths::default()")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_with_root() {
        let paths = SpnPaths::with_root(PathBuf::from("/custom/root"));
        assert_eq!(paths.root(), Path::new("/custom/root"));
    }

    #[test]
    fn test_directory_paths() {
        let paths = SpnPaths::with_root(PathBuf::from("/home/user/.spn"));

        assert_eq!(paths.bin_dir(), PathBuf::from("/home/user/.spn/bin"));
        assert_eq!(
            paths.packages_dir(),
            PathBuf::from("/home/user/.spn/packages")
        );
        assert_eq!(paths.cache_dir(), PathBuf::from("/home/user/.spn/cache"));
        assert_eq!(
            paths.tarballs_dir(),
            PathBuf::from("/home/user/.spn/cache/tarballs")
        );
        assert_eq!(
            paths.registry_dir(),
            PathBuf::from("/home/user/.spn/registry")
        );
    }

    #[test]
    fn test_file_paths() {
        let paths = SpnPaths::with_root(PathBuf::from("/home/user/.spn"));

        assert_eq!(
            paths.config_file(),
            PathBuf::from("/home/user/.spn/config.toml")
        );
        assert_eq!(
            paths.secrets_file(),
            PathBuf::from("/home/user/.spn/secrets.env")
        );
        assert_eq!(
            paths.socket_file(),
            PathBuf::from("/home/user/.spn/daemon.sock")
        );
        assert_eq!(
            paths.pid_file(),
            PathBuf::from("/home/user/.spn/daemon.pid")
        );
        assert_eq!(
            paths.state_file(),
            PathBuf::from("/home/user/.spn/state.json")
        );
    }

    #[test]
    fn test_package_dir() {
        let paths = SpnPaths::with_root(PathBuf::from("/home/user/.spn"));

        let pkg = paths.package_dir("@workflows/code-review", "1.0.0");
        assert_eq!(
            pkg,
            PathBuf::from("/home/user/.spn/packages/@workflows/code-review/1.0.0")
        );
    }

    #[test]
    fn test_binary_path() {
        let paths = SpnPaths::with_root(PathBuf::from("/home/user/.spn"));

        assert_eq!(
            paths.binary("nika"),
            PathBuf::from("/home/user/.spn/bin/nika")
        );
        assert_eq!(
            paths.binary("novanet"),
            PathBuf::from("/home/user/.spn/bin/novanet")
        );
    }

    #[test]
    fn test_ensure_dirs() {
        let temp = TempDir::new().unwrap();
        let paths = SpnPaths::with_root(temp.path().to_path_buf());

        // Directories should not exist initially
        assert!(!paths.bin_dir().exists());
        assert!(!paths.packages_dir().exists());

        // Create them
        paths.ensure_dirs().unwrap();

        // Now they should exist
        assert!(paths.bin_dir().exists());
        assert!(paths.packages_dir().exists());
        assert!(paths.cache_dir().exists());
        assert!(paths.tarballs_dir().exists());
        assert!(paths.registry_dir().exists());
    }

    #[test]
    fn test_exists() {
        let temp = TempDir::new().unwrap();
        let paths = SpnPaths::with_root(temp.path().join("nonexistent"));

        assert!(!paths.exists());

        std::fs::create_dir_all(paths.root()).unwrap();
        assert!(paths.exists());
    }

    #[test]
    fn test_new_returns_home_based_path() {
        // This test only works if HOME is set
        if let Ok(paths) = SpnPaths::new() {
            let root_str = paths.root().to_string_lossy();
            assert!(root_str.ends_with(".spn"));
        }
    }

    #[test]
    fn test_clone() {
        let paths = SpnPaths::with_root(PathBuf::from("/test"));
        let cloned = paths.clone();
        assert_eq!(paths.root(), cloned.root());
    }

    #[test]
    fn test_debug() {
        let paths = SpnPaths::with_root(PathBuf::from("/test"));
        let debug_str = format!("{:?}", paths);
        assert!(debug_str.contains("SpnPaths"));
        assert!(debug_str.contains("/test"));
    }
}
