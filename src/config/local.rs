//! Local overrides configuration (./.spn/local.yaml).

use crate::config::types::Config;
use crate::error::{Result, SpnError};
use std::fs;
use std::path::{Path, PathBuf};

/// Get path to local config file (./.spn/local.yaml).
pub fn config_path(project_root: &Path) -> PathBuf {
    project_root.join(".spn").join("local.yaml")
}

/// Load local configuration.
///
/// Returns empty config if file doesn't exist.
pub fn load(project_root: &Path) -> Result<Config> {
    let path = config_path(project_root);

    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| SpnError::ConfigError(format!("Failed to read {}: {}", path.display(), e)))?;

    serde_yaml::from_str(&content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to parse {}: {}", path.display(), e)))
}

/// Save local configuration.
pub fn save(project_root: &Path, config: &Config) -> Result<()> {
    let path = config_path(project_root);

    // Ensure .spn/ directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| SpnError::ConfigError(format!("Failed to create directory: {}", e)))?;
    }

    let content = serde_yaml::to_string(config)
        .map_err(|e| SpnError::ConfigError(format!("Failed to serialize config: {}", e)))?;

    fs::write(&path, content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(())
}

/// Ensure .spn/local.yaml is in .gitignore.
pub fn ensure_gitignored(project_root: &Path) -> Result<()> {
    let gitignore_path = project_root.join(".gitignore");

    let pattern = ".spn/local.yaml";

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)
            .map_err(|e| SpnError::ConfigError(format!("Failed to read .gitignore: {}", e)))?;

        if content.lines().any(|line| line.trim() == pattern) {
            return Ok(());
        }

        // Append to .gitignore
        let mut new_content = content;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(pattern);
        new_content.push('\n');

        fs::write(&gitignore_path, new_content)
            .map_err(|e| SpnError::ConfigError(format!("Failed to update .gitignore: {}", e)))?;
    } else {
        // Create new .gitignore
        fs::write(&gitignore_path, format!("{}\n", pattern))
            .map_err(|e| SpnError::ConfigError(format!("Failed to create .gitignore: {}", e)))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_local_config_path() {
        let dir = TempDir::new().unwrap();
        let path = config_path(dir.path());
        assert!(path.ends_with(".spn/local.yaml"));
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = TempDir::new().unwrap();
        let config = load(dir.path()).unwrap();
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let mut config = Config::default();
        config.sync.auto_sync = true;

        save(root, &config).unwrap();

        let loaded = load(root).unwrap();
        assert!(loaded.sync.auto_sync);
    }

    #[test]
    fn test_ensure_gitignored() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        ensure_gitignored(root).unwrap();

        let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(gitignore.contains(".spn/local.yaml"));

        // Call again - should be idempotent
        ensure_gitignored(root).unwrap();

        let gitignore2 = fs::read_to_string(root.join(".gitignore")).unwrap();
        assert_eq!(
            gitignore2.matches(".spn/local.yaml").count(),
            1,
            "Pattern should appear only once"
        );
    }
}
