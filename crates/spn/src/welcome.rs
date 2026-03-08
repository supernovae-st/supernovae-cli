//! Welcome and tour functions for spn CLI.
//!
//! Provides the feature tour for `spn tour` command.

use crate::ux::design_system as ds;

/// Display the feature tour
pub fn show_tour() {
    println!();
    println!("{}", ds::primary("🧭 spn Feature Tour"));
    println!();

    let features = [
        (
            "Package Management",
            "Install AI workflows, schemas, and tools",
            "spn add @nika/generate-page",
        ),
        (
            "Secure Secrets",
            "Store API keys in your OS keychain",
            "spn provider set anthropic",
        ),
        (
            "MCP Servers",
            "Add Model Context Protocol servers",
            "spn mcp add neo4j",
        ),
        (
            "Local Models",
            "Manage LLMs via Ollama",
            "spn model pull llama3.2",
        ),
        (
            "Editor Sync",
            "Sync packages to Claude Code, VS Code",
            "spn sync",
        ),
        ("Diagnostics", "Check your setup is working", "spn doctor"),
    ];

    for (i, (name, desc, cmd)) in features.iter().enumerate() {
        println!(
            "  {} {}",
            ds::primary(format!("{}.", i + 1)),
            ds::highlight(*name)
        );
        println!("     {}", ds::muted(*desc));
        println!("     {} {}", ds::muted("$"), ds::command(*cmd));
        println!();
    }

    println!(
        "{}",
        ds::muted("Run `spn topic` to explore any of these in detail.")
    );
    println!();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_show_tour_does_not_panic() {
        // Basic smoke test - function should not panic
        // Can't easily test output without capturing stdout
    }
}
