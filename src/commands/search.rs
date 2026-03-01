//! Search command implementation.
//!
//! Searches for packages in the registry.

use colored::Colorize;

use crate::error::Result;

/// Run the search command.
pub async fn run(query: &str) -> Result<()> {
    println!("{} Searching for: {}", "🔍".cyan(), query.green());

    // For now, direct to registry website
    let search_url = format!(
        "https://github.com/SuperNovae-studio/supernovae-registry/search?q={}",
        urlencoding::encode(query)
    );

    println!();
    println!("   {} Registry search:", "→".blue());
    println!("   {}", search_url);
    println!();
    println!("   {} Full-text search coming in v0.4.0", "ℹ️".yellow());

    Ok(())
}
