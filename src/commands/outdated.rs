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

    let storage = LocalStorage::new()
        .map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

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
