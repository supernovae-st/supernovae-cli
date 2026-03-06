//! Outdated command implementation.
//!
//! Lists packages with newer versions available.

use colored::Colorize;

use crate::error::{Result, SpnError};
use crate::index::IndexClient;
use crate::storage::LocalStorage;

/// Run the outdated command.
pub async fn run() -> Result<()> {
    println!("{} Checking for outdated packages...", "📋".cyan());

    let storage =
        LocalStorage::new().map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

    let state = storage
        .load_state()
        .map_err(|e| SpnError::ConfigError(format!("Failed to load state: {}", e)))?;

    if state.packages.is_empty() {
        println!("   {} No packages installed", "ℹ️".yellow());
        return Ok(());
    }

    let client = IndexClient::new();
    let mut outdated_count = 0;

    println!();
    for (name, installed) in &state.packages {
        match client.fetch_latest(name).await {
            Ok(latest) => {
                if latest.version != installed.version {
                    println!(
                        "   {} {} {} → {}",
                        "↑".yellow(),
                        name,
                        installed.version.red(),
                        latest.version.green()
                    );
                    outdated_count += 1;
                }
            }
            Err(_) => {
                println!("   {} {} (not in registry)", "?".yellow(), name);
            }
        }
    }

    if outdated_count == 0 {
        println!("   {} All packages up to date!", "✓".green());
    } else {
        println!();
        println!("   {} Run {} to update", "ℹ️".blue(), "spn update".cyan());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    /// Check if a package is outdated (installed version differs from latest).
    #[inline]
    fn is_outdated(installed_version: &str, latest_version: &str) -> bool {
        installed_version != latest_version
    }

    #[test]
    fn test_is_outdated_true() {
        assert!(is_outdated("1.0.0", "2.0.0"));
        assert!(is_outdated("1.0.0", "1.0.1"));
        assert!(is_outdated("0.9.0", "1.0.0"));
    }

    #[test]
    fn test_is_outdated_false() {
        assert!(!is_outdated("1.0.0", "1.0.0"));
        assert!(!is_outdated("2.5.3", "2.5.3"));
    }

    #[test]
    fn test_is_outdated_prerelease() {
        // Pre-release versions are considered different
        assert!(is_outdated("1.0.0-alpha", "1.0.0"));
        assert!(is_outdated("1.0.0", "1.0.0-rc1"));
    }
}
