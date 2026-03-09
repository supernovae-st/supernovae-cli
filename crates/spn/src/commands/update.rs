//! Update command implementation.
//!
//! Updates installed packages to their latest versions.

use crate::ux::design_system as ds;

use crate::error::{Result, SpnError};
use crate::index::{Downloader, IndexClient};
use crate::storage::LocalStorage;

/// Run the update command.
pub async fn run(package: Option<&str>) -> Result<()> {
    let storage =
        LocalStorage::new().map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

    let state = storage
        .load_state()
        .map_err(|e| SpnError::ConfigError(format!("Failed to load state: {}", e)))?;

    let packages_to_update: Vec<_> = match package {
        Some(name) => {
            if !state.packages.contains_key(name) {
                return Err(SpnError::PackageNotFound(format!(
                    "{} is not installed",
                    name
                )));
            }
            vec![name.to_string()]
        }
        None => state.packages.keys().cloned().collect(),
    };

    if packages_to_update.is_empty() {
        println!("   {} No packages to update", ds::warning("ℹ️"));
        return Ok(());
    }

    println!(
        "{} Updating {} package(s)...",
        ds::primary("🔄"),
        packages_to_update.len()
    );

    let client = IndexClient::new();
    let downloader = Downloader::new();
    let mut updated_count = 0;

    for name in &packages_to_update {
        let installed = state.packages.get(name).ok_or_else(|| {
            SpnError::PackageNotFound(format!("Package {} disappeared from state", name))
        })?;

        match client.fetch_latest(name).await {
            Ok(latest) => {
                if latest.version == installed.version {
                    println!(
                        "   {} {} already at latest ({})",
                        ds::success("✓"),
                        name,
                        installed.version
                    );
                    continue;
                }

                println!(
                    "   {} {} {} → {}",
                    ds::primary("↑"),
                    name,
                    installed.version,
                    latest.version
                );

                // Download and install new version
                let downloaded = downloader
                    .download_entry(&latest)
                    .await
                    .map_err(|e| SpnError::ConfigError(format!("Download failed: {}", e)))?;

                storage
                    .install(&downloaded)
                    .map_err(|e| SpnError::ConfigError(format!("Install failed: {}", e)))?;

                updated_count += 1;
            }
            Err(e) => {
                println!("   {} {} failed: {}", ds::error("✗"), name, e);
            }
        }
    }

    println!();
    println!("{} Updated {} package(s)", ds::warning("✨"), updated_count);

    Ok(())
}

#[cfg(test)]
mod tests {
    /// Check if a version update is needed.
    #[inline]
    fn needs_update(installed_version: &str, latest_version: &str) -> bool {
        installed_version != latest_version
    }

    #[test]
    fn test_needs_update_different_versions() {
        assert!(needs_update("1.0.0", "2.0.0"));
        assert!(needs_update("1.0.0", "1.0.1"));
        assert!(needs_update("1.0.0", "1.1.0"));
    }

    #[test]
    fn test_needs_update_same_version() {
        assert!(!needs_update("1.0.0", "1.0.0"));
        assert!(!needs_update("2.5.3", "2.5.3"));
    }

    #[test]
    fn test_needs_update_handles_prerelease() {
        // Simple string comparison - semantic version comparison
        // would be done by the index client
        assert!(needs_update("1.0.0-alpha", "1.0.0"));
        assert!(needs_update("1.0.0", "1.0.0-beta"));
    }
}
