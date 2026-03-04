//! Info command implementation.
//!
//! Displays detailed information about a package from the registry.

use colored::Colorize;

use crate::error::{Result, SpnError};
use crate::index::IndexClient;
use crate::storage::LocalStorage;

/// Run the info command.
pub async fn run(package: &str) -> Result<()> {
    println!("{} Package: {}", "ℹ️".cyan(), package.green());

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

    // Check if installed locally
    let storage =
        LocalStorage::new().map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

    if let Ok(Some(version)) = storage.installed_version(package) {
        println!();
        println!("   {} Installed: {}", "✓".green(), version);
    }

    Ok(())
}
