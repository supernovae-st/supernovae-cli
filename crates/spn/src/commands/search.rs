//! Search command implementation.
//!
//! Searches for packages in the registry.

use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;
use crate::index::IndexClient;

/// Registry metadata from registry.json
#[derive(Debug, Deserialize)]
struct RegistryMetadata {
    packages: HashMap<String, PackageMetadata>,
}

/// Package metadata in registry.json
#[derive(Debug, Deserialize)]
struct PackageMetadata {
    #[serde(default)]
    description: String,
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    version: String,
}

/// Search result for JSON output.
#[derive(Debug, Serialize)]
struct SearchResult {
    name: String,
    version: String,
    description: String,
    r#type: String,
}

/// Run the search command.
pub async fn run(query: &str, json: bool) -> Result<()> {
    if !json {
        println!("{} Searching SuperNovae Registry...\n", "🔍".cyan());
    }

    let client = IndexClient::new();
    let query_lower = query.to_lowercase();

    // Fetch registry metadata to get package list
    let registry_url =
        "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/registry.json";

    let http_client = reqwest::Client::new();
    let response = http_client
        .get(registry_url)
        .header("User-Agent", "spn/0.6")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(crate::error::SpnError::IndexError(format!(
            "Failed to fetch registry metadata: HTTP {}",
            response.status()
        )));
    }

    let metadata: RegistryMetadata = response.json().await?;

    // Search through packages
    let mut results = Vec::new();

    for (name, pkg_meta) in metadata.packages.iter() {
        let name_lower = name.to_lowercase();
        let desc_lower = pkg_meta.description.to_lowercase();

        // Check if query matches name or description
        if name_lower.contains(&query_lower) || desc_lower.contains(&query_lower) {
            // Try to fetch package details from index
            match client.fetch_latest(name).await {
                Ok(entry) => {
                    results.push((
                        name.clone(),
                        entry.version,
                        pkg_meta.description.clone(),
                        pkg_meta.r#type.clone(),
                    ));
                }
                Err(_) => {
                    // Package in registry.json but not in index (not published yet)
                    results.push((
                        name.clone(),
                        pkg_meta.version.clone(),
                        pkg_meta.description.clone(),
                        pkg_meta.r#type.clone(),
                    ));
                }
            }
        }
    }

    // Sort results: exact match first, then alphabetically
    results.sort_by(|a, b| {
        let a_exact = a.0.to_lowercase() == query_lower;
        let b_exact = b.0.to_lowercase() == query_lower;

        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(&b.0),
        }
    });

    // JSON output
    if json {
        let json_results: Vec<SearchResult> = results
            .iter()
            .map(|(name, version, description, pkg_type)| SearchResult {
                name: name.clone(),
                version: version.clone(),
                description: description.clone(),
                r#type: pkg_type.clone(),
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&json_results)?);
        return Ok(());
    }

    // Human-readable output
    if results.is_empty() {
        println!(
            "   {} No packages found matching '{}'",
            "ℹ️".yellow(),
            query
        );
        println!();
        println!("   Try:");
        println!("   • Check spelling");
        println!("   • Use fewer keywords");
        println!("   • Browse all: https://github.com/supernovae-st/supernovae-registry");
        return Ok(());
    }

    println!("   Found {} package(s):\n", results.len());

    for (name, version, description, pkg_type) in results.iter().take(20) {
        // Get type emoji
        let type_emoji = match pkg_type.as_str() {
            "workflow" => "📋",
            "agent" => "🤖",
            "skill" => "⚡",
            "prompt" => "💬",
            "job" => "⏰",
            "schema" => "📊",
            _ => "📦",
        };

        println!("   {} {}@{}", type_emoji, name.green(), version.dimmed());

        if !description.is_empty() {
            // Truncate description if too long
            let desc = if description.len() > 60 {
                format!("{}...", &description[..57])
            } else {
                description.clone()
            };
            println!("     {}", desc.dimmed());
        }
        println!();
    }

    if results.len() > 20 {
        println!(
            "   {} {} more results not shown",
            "ℹ️".yellow(),
            results.len() - 20
        );
        println!();
    }

    println!("   Use {} to install.", "spn add <package>".cyan());
    println!();

    Ok(())
}
