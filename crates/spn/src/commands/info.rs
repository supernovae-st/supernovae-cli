//! Info command implementation.
//!
//! Displays detailed information about a package from the registry.

use colored::Colorize;
use serde::Serialize;

use crate::error::{Result, SpnError};
use crate::index::IndexClient;
use crate::storage::LocalStorage;

/// Package info for JSON output.
#[derive(Debug, Serialize)]
struct PackageInfo {
    name: String,
    versions: Vec<VersionInfo>,
    installed: Option<String>,
}

/// Version info for JSON output.
#[derive(Debug, Serialize)]
struct VersionInfo {
    version: String,
    yanked: bool,
    latest: bool,
}

/// Run the info command.
pub async fn run(package: &str, json: bool) -> Result<()> {
    if !json {
        println!("{} Package: {}", "ℹ️".cyan(), package.green());
    }

    let client = IndexClient::new();

    // Fetch all versions
    let entries = client
        .fetch_package(package)
        .await
        .map_err(|e| SpnError::PackageNotFound(format!("{}: {}", package, e)))?;

    if entries.is_empty() {
        return Err(SpnError::PackageNotFound(package.to_string()));
    }

    // Get latest version
    let latest = entries
        .iter()
        .filter(|e| e.is_available())
        .max_by(|a, b| a.semver().ok().cmp(&b.semver().ok()));

    // Check if installed locally
    let storage =
        LocalStorage::new().map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;
    let installed = storage.installed_version(package).ok().flatten();

    // JSON output
    if json {
        let info = PackageInfo {
            name: package.to_string(),
            versions: entries
                .iter()
                .map(|e| VersionInfo {
                    version: e.version.clone(),
                    yanked: e.yanked,
                    latest: Some(e) == latest,
                })
                .collect(),
            installed,
        };

        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    // Human-readable output
    println!();
    println!("   {}", "Versions:".bold());
    for entry in entries.iter().rev().take(5) {
        let status = if entry.yanked {
            "(yanked)".red().to_string()
        } else if Some(entry) == latest {
            "(latest)".green().to_string()
        } else {
            String::new()
        };
        println!("   {} {} {}", "•".blue(), entry.version, status);
    }

    if entries.len() > 5 {
        println!("   {} ... and {} more", "•".blue(), entries.len() - 5);
    }

    if let Some(ref version) = installed {
        println!();
        println!("   {} Installed: {}", "✓".green(), version);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_info_serialization() {
        let info = PackageInfo {
            name: "test-package".to_string(),
            versions: vec![
                VersionInfo {
                    version: "1.0.0".to_string(),
                    yanked: false,
                    latest: true,
                },
                VersionInfo {
                    version: "0.9.0".to_string(),
                    yanked: false,
                    latest: false,
                },
            ],
            installed: Some("1.0.0".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-package"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("0.9.0"));
        assert!(json.contains("\"latest\":true"));
        assert!(json.contains("\"installed\":\"1.0.0\""));
    }

    #[test]
    fn test_package_info_no_installed() {
        let info = PackageInfo {
            name: "test-package".to_string(),
            versions: vec![],
            installed: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"installed\":null"));
    }

    #[test]
    fn test_version_info_yanked() {
        let info = VersionInfo {
            version: "1.0.0".to_string(),
            yanked: true,
            latest: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"yanked\":true"));
        assert!(json.contains("\"latest\":false"));
    }
}
