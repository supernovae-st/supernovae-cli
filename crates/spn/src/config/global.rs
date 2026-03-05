//! Global user configuration (~/.spn/config.toml).

use crate::config::types::Config;
use crate::error::{Result, SpnError};
use std::fs;
use std::path::PathBuf;

/// Get path to global config file (~/.spn/config.toml).
pub fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| SpnError::ConfigError("Home directory not found".to_string()))?;
    Ok(home.join(".spn").join("config.toml"))
}

/// Load global configuration.
///
/// Returns empty config if file doesn't exist.
pub fn load() -> Result<Config> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| SpnError::ConfigError(format!("Failed to read {}: {}", path.display(), e)))?;

    toml::from_str(&content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to parse {}: {}", path.display(), e)))
}

/// Save global configuration.
pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;

    // Ensure ~/.spn/ directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| SpnError::ConfigError(format!("Failed to create directory: {}", e)))?;
    }

    let content = toml::to_string_pretty(config)
        .map_err(|e| SpnError::ConfigError(format!("Failed to serialize config: {}", e)))?;

    fs::write(&path, content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(())
}

/// Get a specific value from global config.
pub fn get(_key: &str) -> Result<Option<serde_json::Value>> {
    let _config = load()?;

    // Simple key resolution for now
    // TODO: Support nested keys like "providers.anthropic.model"
    Ok(None)
}

/// Set a specific value in global config.
pub fn set(_key: &str, _value: serde_json::Value) -> Result<()> {
    let config = load()?;

    // Simple key resolution for now
    // TODO: Support nested keys like "providers.anthropic.model"

    save(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_path() {
        let path = config_path().unwrap();
        assert!(path.ends_with(".spn/config.toml"));
    }

    #[test]
    fn test_load_nonexistent() {
        // Loading nonexistent file should return default config
        let config = load().unwrap_or_default();
        assert!(config.providers.is_empty());
    }
}
