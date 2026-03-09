//! Info command implementation.
//!
//! Displays detailed information about a package from the registry.

use crate::ux::design_system as ds;
use serde::Serialize;

use crate::error::{Result, SpnError};
use crate::index::IndexClient;
use crate::storage::LocalStorage;

/// Package info for JSON output.
#[derive(Debug, Serialize)]
struct PackageInfo {
    name: String,
    versions: Vec<VersionInfo>,
    installed: Option<InstalledInfo>,
}

/// Installed package info for JSON output.
#[derive(Debug, Serialize)]
struct InstalledInfo {
    version: String,
    path: String,
    checksum: String,
    installed_at: String,
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
        println!("{} Package: {}", ds::primary("ℹ️"), ds::success(package));
    }

    let client = IndexClient::new();

    // Fetch all versions
    let entries = client
        .fetch_package(package)
        .await
        .map_err(|_| SpnError::PackageNotFound(package.to_string()))?;

    if entries.is_empty() {
        return Err(SpnError::PackageNotFound(package.to_string()));
    }

    // Get latest version
    let latest = entries
        .iter()
        .filter(|e| e.is_available())
        .max_by(|a, b| a.semver().ok().cmp(&b.semver().ok()));

    // Check if installed locally (with full details)
    let storage =
        LocalStorage::new().map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;
    let installed_pkg = storage.get_installed(package).ok().flatten();

    // JSON output
    if json {
        let installed = installed_pkg.map(|p| InstalledInfo {
            version: p.version,
            path: p.path.to_string_lossy().to_string(),
            checksum: p.checksum,
            installed_at: p.installed_at,
        });

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
    println!("   {}", ds::highlight("Versions:"));
    for entry in entries.iter().rev().take(5) {
        let status = if entry.yanked {
            ds::error("(yanked)").to_string()
        } else if Some(entry) == latest {
            ds::success("(latest)").to_string()
        } else {
            String::new()
        };
        println!("   {} {} {}", ds::primary("•"), entry.version, status);
    }

    if entries.len() > 5 {
        println!("   {} ... and {} more", ds::primary("•"), entries.len() - 5);
    }

    // Show detailed installation info
    if let Some(ref pkg) = installed_pkg {
        println!();
        println!("   {}", ds::highlight("Installed:"));
        println!("   {} Version:  {}", ds::success("✓"), pkg.version);
        println!("   {} Path:     {}", ds::muted("•"), pkg.path.display());
        println!("   {} Checksum: {}", ds::muted("•"), &pkg.checksum[..20.min(pkg.checksum.len())]);
        println!("   {} Date:     {}", ds::muted("•"), &pkg.installed_at[..10.min(pkg.installed_at.len())]);
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
            installed: Some(InstalledInfo {
                version: "1.0.0".to_string(),
                path: "/home/user/.spn/packages/test-package/1.0.0".to_string(),
                checksum: "sha256:abc123".to_string(),
                installed_at: "2024-01-01T00:00:00Z".to_string(),
            }),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-package"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("0.9.0"));
        assert!(json.contains("\"latest\":true"));
        assert!(json.contains("\"version\":\"1.0.0\"")); // In installed
        assert!(json.contains("\"path\":\"/home/user/.spn/packages"));
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

    #[test]
    fn test_installed_info_serialization() {
        let info = InstalledInfo {
            version: "2.0.0".to_string(),
            path: "/tmp/test".to_string(),
            checksum: "sha256:xyz".to_string(),
            installed_at: "2024-06-15T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"version\":\"2.0.0\""));
        assert!(json.contains("\"path\":\"/tmp/test\""));
        assert!(json.contains("\"checksum\":\"sha256:xyz\""));
        assert!(json.contains("\"installed_at\":\"2024-06-15"));
    }
}
