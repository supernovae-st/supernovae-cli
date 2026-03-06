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
        .header("User-Agent", format!("spn/{}", env!("CARGO_PKG_VERSION")))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Sort search results: exact matches first, then alphabetically.
    fn sort_results(results: &mut [(String, String, String, String)], query: &str) {
        let query_lower = query.to_lowercase();
        results.sort_by(|a, b| {
            let a_exact = a.0.to_lowercase() == query_lower;
            let b_exact = b.0.to_lowercase() == query_lower;

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            }
        });
    }

    /// Check if a package name or description matches a query.
    fn matches_query(name: &str, description: &str, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        let name_lower = name.to_lowercase();
        let desc_lower = description.to_lowercase();
        name_lower.contains(&query_lower) || desc_lower.contains(&query_lower)
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            description: "A test package".to_string(),
            r#type: "workflow".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-package"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("workflow"));
    }

    #[test]
    fn test_matches_query_name() {
        assert!(matches_query("my-workflow", "Description", "workflow"));
        assert!(matches_query("MY-WORKFLOW", "Description", "workflow"));
        assert!(!matches_query("my-agent", "Description", "workflow"));
    }

    #[test]
    fn test_matches_query_description() {
        assert!(matches_query(
            "package",
            "A workflow for testing",
            "workflow"
        ));
        assert!(matches_query(
            "package",
            "A WORKFLOW for testing",
            "workflow"
        ));
        assert!(!matches_query(
            "package",
            "An agent for testing",
            "workflow"
        ));
    }

    #[test]
    fn test_sort_results_exact_match_first() {
        let mut results = vec![
            (
                "alpha-workflow".to_string(),
                "1.0".to_string(),
                "".to_string(),
                "".to_string(),
            ),
            (
                "workflow".to_string(),
                "2.0".to_string(),
                "".to_string(),
                "".to_string(),
            ),
            (
                "beta-workflow".to_string(),
                "1.0".to_string(),
                "".to_string(),
                "".to_string(),
            ),
        ];

        sort_results(&mut results, "workflow");

        // Exact match should be first
        assert_eq!(results[0].0, "workflow");
        // Then alphabetically
        assert_eq!(results[1].0, "alpha-workflow");
        assert_eq!(results[2].0, "beta-workflow");
    }

    #[test]
    fn test_sort_results_case_insensitive() {
        let mut results = vec![
            (
                "alpha".to_string(),
                "1.0".to_string(),
                "".to_string(),
                "".to_string(),
            ),
            (
                "WORKFLOW".to_string(),
                "2.0".to_string(),
                "".to_string(),
                "".to_string(),
            ),
        ];

        sort_results(&mut results, "workflow");

        // Case-insensitive exact match should be first
        assert_eq!(results[0].0, "WORKFLOW");
    }

    #[test]
    fn test_sort_results_alphabetical_when_no_exact() {
        let mut results = vec![
            (
                "zebra-workflow".to_string(),
                "1.0".to_string(),
                "".to_string(),
                "".to_string(),
            ),
            (
                "alpha-workflow".to_string(),
                "2.0".to_string(),
                "".to_string(),
                "".to_string(),
            ),
        ];

        sort_results(&mut results, "workflow");

        // Alphabetical order when no exact match
        assert_eq!(results[0].0, "alpha-workflow");
        assert_eq!(results[1].0, "zebra-workflow");
    }
}
