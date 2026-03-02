//! Local storage for installed SuperNovae packages.
//!
//! Directory structure:
//! ```text
//! ~/.spn/
//! ├── packages/                 # Installed packages
//! │   ├── @workflows/
//! │   │   └── dev-productivity/
//! │   │       └── code-review/
//! │   │           └── 1.0.0/    # Version directory
//! │   │               ├── workflow.nika.yaml
//! │   │               └── README.md
//! │   └── @nika/
//! │       └── seo-audit/
//! │           └── 0.1.5/
//! ├── cache/                    # Downloaded tarballs
//! │   └── tarballs/
//! └── state.json                # Installed packages state
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::index::DownloadedPackage;

/// Errors that can occur with local storage.
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse state file: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Package not installed: {0}")]
    NotInstalled(String),

    #[error("Failed to extract package: {0}")]
    ExtractError(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Directory walk error: {0}")]
    WalkDirError(#[from] walkdir::Error),
}

/// Record of an installed package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    /// Package name.
    pub name: String,

    /// Installed version.
    pub version: String,

    /// SHA256 checksum.
    pub checksum: String,

    /// Installation path.
    pub path: PathBuf,

    /// Installation timestamp (ISO 8601).
    pub installed_at: String,
}

/// State of installed packages.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageState {
    /// Version of the state file format.
    pub version: u32,

    /// Installed packages (name -> info).
    pub packages: HashMap<String, InstalledPackage>,
}

impl StorageState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self {
            version: 1,
            packages: HashMap::new(),
        }
    }
}

/// Local storage manager.
pub struct LocalStorage {
    /// Root directory (~/.spn/).
    root: PathBuf,

    /// Packages directory.
    packages_dir: PathBuf,

    /// Cache directory.
    cache_dir: PathBuf,

    /// State file path.
    state_file: PathBuf,
}

impl LocalStorage {
    /// Create a new local storage manager with default paths.
    pub fn new() -> Result<Self, StorageError> {
        let root = dirs::home_dir()
            .ok_or_else(|| {
                StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine home directory",
                ))
            })?
            .join(".spn");

        Self::with_root(root)
    }

    /// Create a local storage manager with a custom root directory.
    pub fn with_root<P: AsRef<Path>>(root: P) -> Result<Self, StorageError> {
        let root = root.as_ref().to_path_buf();
        let packages_dir = root.join("packages");
        let cache_dir = root.join("cache");
        let state_file = root.join("state.json");

        // Ensure directories exist
        std::fs::create_dir_all(&packages_dir)?;
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            root,
            packages_dir,
            cache_dir,
            state_file,
        })
    }

    /// Get the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the packages directory.
    pub fn packages_dir(&self) -> &Path {
        &self.packages_dir
    }

    /// Get the cache directory.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Load the storage state.
    pub fn load_state(&self) -> Result<StorageState, StorageError> {
        if !self.state_file.exists() {
            return Ok(StorageState::new());
        }

        let content = std::fs::read_to_string(&self.state_file)?;
        let state: StorageState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Save the storage state.
    pub fn save_state(&self, state: &StorageState) -> Result<(), StorageError> {
        let content = serde_json::to_string_pretty(state)?;
        std::fs::write(&self.state_file, content)?;
        Ok(())
    }

    /// Validate that a path component doesn't contain traversal sequences.
    pub fn validate_path_component(&self, component: &str) -> Result<(), StorageError> {
        if component.contains("..") {
            return Err(StorageError::InvalidPath(
                "Path traversal detected".to_string(),
            ));
        }
        Ok(())
    }

    /// Get the installation path for a package (validated).
    pub fn safe_package_path(&self, name: &str, version: &str) -> Result<PathBuf, StorageError> {
        // Validate no path traversal
        self.validate_path_component(name)?;
        self.validate_path_component(version)?;

        let path = name
            .replace('@', "")
            .replace('/', std::path::MAIN_SEPARATOR_STR);
        let full_path = self.packages_dir.join(&path).join(version);

        // Ensure path stays within packages_dir
        if !full_path.starts_with(&self.packages_dir) {
            return Err(StorageError::InvalidPath(
                "Path escapes packages directory".to_string(),
            ));
        }

        Ok(full_path)
    }

    /// Get the installation path for a package (legacy, unchecked).
    #[allow(dead_code)]
    pub fn package_path(&self, name: &str, version: &str) -> PathBuf {
        // Convert @scope/path to directory structure
        let path = name
            .replace('@', "")
            .replace('/', std::path::MAIN_SEPARATOR_STR);
        self.packages_dir.join(path).join(version)
    }

    /// Install a downloaded package.
    pub fn install(
        &self,
        downloaded: &DownloadedPackage,
    ) -> Result<InstalledPackage, StorageError> {
        // Use validated path
        let install_path = self.safe_package_path(&downloaded.name, &downloaded.version)?;

        // Create installation directory
        std::fs::create_dir_all(&install_path)?;

        // Extract tarball with path validation
        let file = std::fs::File::open(&downloaded.tarball_path)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        // Disable potentially dangerous features
        archive.set_preserve_permissions(false);
        archive.set_unpack_xattrs(false);

        // Validate each entry before extraction
        for entry in archive
            .entries()
            .map_err(|e| StorageError::ExtractError(e.to_string()))?
        {
            let mut entry = entry.map_err(|e| StorageError::ExtractError(e.to_string()))?;
            let path = entry
                .path()
                .map_err(|e| StorageError::ExtractError(e.to_string()))?;

            // Check for path traversal in tarball
            if path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(StorageError::InvalidPath(format!(
                    "Tarball contains path traversal: {}",
                    path.display()
                )));
            }

            entry
                .unpack_in(&install_path)
                .map_err(|e| StorageError::ExtractError(e.to_string()))?;
        }

        let installed = InstalledPackage {
            name: downloaded.name.clone(),
            version: downloaded.version.clone(),
            checksum: downloaded.checksum.clone(),
            path: install_path,
            installed_at: chrono::Utc::now().to_rfc3339(),
        };

        // Update state
        let mut state = self.load_state()?;
        state
            .packages
            .insert(downloaded.name.clone(), installed.clone());
        self.save_state(&state)?;

        Ok(installed)
    }

    /// Uninstall a package.
    pub fn uninstall(&self, name: &str) -> Result<(), StorageError> {
        let mut state = self.load_state()?;

        let installed = state
            .packages
            .remove(name)
            .ok_or_else(|| StorageError::NotInstalled(name.to_string()))?;

        // Remove package directory
        if installed.path.exists() {
            std::fs::remove_dir_all(&installed.path)?;

            // Clean up empty parent directories
            let mut parent = installed.path.parent();
            while let Some(dir) = parent {
                if dir == self.packages_dir {
                    break;
                }
                if dir.read_dir()?.next().is_none() {
                    std::fs::remove_dir(dir)?;
                    parent = dir.parent();
                } else {
                    break;
                }
            }
        }

        self.save_state(&state)?;
        Ok(())
    }

    /// Check if a package is installed.
    pub fn is_installed(&self, name: &str) -> Result<bool, StorageError> {
        let state = self.load_state()?;
        Ok(state.packages.contains_key(name))
    }

    /// Get installed package info.
    pub fn get_installed(&self, name: &str) -> Result<Option<InstalledPackage>, StorageError> {
        let state = self.load_state()?;
        Ok(state.packages.get(name).cloned())
    }

    /// List all installed packages.
    pub fn list_installed(&self) -> Result<Vec<InstalledPackage>, StorageError> {
        let state = self.load_state()?;
        Ok(state.packages.values().cloned().collect())
    }

    /// Scan filesystem for packages (includes manually installed ones).
    pub fn scan_filesystem(&self) -> Result<Vec<InstalledPackage>, StorageError> {
        use walkdir::WalkDir;

        let mut packages = Vec::new();

        // Walk through packages directory
        for entry in WalkDir::new(&self.packages_dir)
            .min_depth(1)
            .max_depth(5)
            .follow_links(false)
        {
            let entry = entry?;
            let path = entry.path();

            // Look for manifest.yaml files
            if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new("manifest.yaml")) {
                // Read manifest
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(manifest) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                        // Extract package info
                        let name = manifest.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let version = manifest.get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0.0.0")
                            .to_string();

                        // Package directory is parent of manifest.yaml
                        let package_path = path.parent().unwrap().to_path_buf();

                        packages.push(InstalledPackage {
                            name,
                            version,
                            checksum: "filesystem".to_string(),
                            path: package_path,
                            installed_at: chrono::Utc::now().to_rfc3339(),
                        });
                    }
                }
            }
        }

        Ok(packages)
    }

    /// List all installed packages as (name, path) pairs.
    pub fn list_packages(&self) -> Result<Vec<(String, PathBuf)>, StorageError> {
        let state = self.load_state()?;
        Ok(state
            .packages
            .values()
            .map(|p| (p.name.clone(), p.path.clone()))
            .collect())
    }

    /// Get installed package version.
    pub fn installed_version(&self, name: &str) -> Result<Option<String>, StorageError> {
        let state = self.load_state()?;
        Ok(state.packages.get(name).map(|p| p.version.clone()))
    }

    /// Clear the package cache.
    pub fn clear_cache(&self) -> Result<(), StorageError> {
        let tarballs_dir = self.cache_dir.join("tarballs");
        if tarballs_dir.exists() {
            std::fs::remove_dir_all(&tarballs_dir)?;
        }
        Ok(())
    }
}

impl Default for LocalStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create default local storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (TempDir, LocalStorage) {
        let temp = TempDir::new().unwrap();
        let storage = LocalStorage::with_root(temp.path()).unwrap();
        (temp, storage)
    }

    fn create_mock_tarball(dir: &Path, name: &str, version: &str) -> DownloadedPackage {
        // Create a simple tar.gz with sanitized filename
        let safe_name = name.replace('@', "").replace('/', "_");
        let tarball_path = dir.join(format!("{}-{}.tar.gz", safe_name, version));

        {
            let file = std::fs::File::create(&tarball_path).unwrap();
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);

            // Add a workflow file
            let content = format!("name: {}\nversion: {}", name, version);
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "workflow.nika.yaml", content.as_bytes())
                .unwrap();

            let enc = tar.into_inner().unwrap();
            enc.finish().unwrap();
        }

        DownloadedPackage {
            name: name.to_string(),
            version: version.to_string(),
            tarball_path,
            checksum: "sha256:test".to_string(),
        }
    }

    #[test]
    fn test_directory_structure() {
        let (temp, storage) = create_test_storage();

        assert!(storage.packages_dir().exists());
        assert!(storage.cache_dir().exists());
        assert_eq!(storage.root(), temp.path());
    }

    #[test]
    fn test_package_path() {
        let (_temp, storage) = create_test_storage();

        let path = storage.package_path("@workflows/dev-productivity/code-review", "1.0.0");
        assert!(path.to_string_lossy().contains("workflows"));
        assert!(path.to_string_lossy().contains("code-review"));
        assert!(path.to_string_lossy().contains("1.0.0"));
    }

    #[test]
    fn test_install_package() {
        let (temp, storage) = create_test_storage();

        let downloaded = create_mock_tarball(temp.path(), "@test/pkg", "1.0.0");
        let installed = storage.install(&downloaded).unwrap();

        assert_eq!(installed.name, "@test/pkg");
        assert_eq!(installed.version, "1.0.0");
        assert!(installed.path.exists());
        assert!(installed.path.join("workflow.nika.yaml").exists());
    }

    #[test]
    fn test_is_installed() {
        let (temp, storage) = create_test_storage();

        assert!(!storage.is_installed("@test/pkg").unwrap());

        let downloaded = create_mock_tarball(temp.path(), "@test/pkg", "1.0.0");
        storage.install(&downloaded).unwrap();

        assert!(storage.is_installed("@test/pkg").unwrap());
    }

    #[test]
    fn test_uninstall_package() {
        let (temp, storage) = create_test_storage();

        let downloaded = create_mock_tarball(temp.path(), "@test/pkg", "1.0.0");
        let installed = storage.install(&downloaded).unwrap();
        let install_path = installed.path.clone();

        assert!(install_path.exists());

        storage.uninstall("@test/pkg").unwrap();

        assert!(!install_path.exists());
        assert!(!storage.is_installed("@test/pkg").unwrap());
    }

    #[test]
    fn test_list_installed() {
        let (temp, storage) = create_test_storage();

        let pkg1 = create_mock_tarball(temp.path(), "@test/pkg1", "1.0.0");
        let pkg2 = create_mock_tarball(temp.path(), "@test/pkg2", "2.0.0");

        storage.install(&pkg1).unwrap();
        storage.install(&pkg2).unwrap();

        let installed = storage.list_installed().unwrap();
        assert_eq!(installed.len(), 2);
    }

    #[test]
    fn test_installed_version() {
        let (temp, storage) = create_test_storage();

        let downloaded = create_mock_tarball(temp.path(), "@test/pkg", "1.2.3");
        storage.install(&downloaded).unwrap();

        let version = storage.installed_version("@test/pkg").unwrap();
        assert_eq!(version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_state_persistence() {
        let (temp, storage) = create_test_storage();

        let downloaded = create_mock_tarball(temp.path(), "@test/pkg", "1.0.0");
        storage.install(&downloaded).unwrap();

        // Create new storage instance pointing to same directory
        let storage2 = LocalStorage::with_root(temp.path()).unwrap();
        assert!(storage2.is_installed("@test/pkg").unwrap());
    }
}
