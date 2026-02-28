//! MCP server command implementation.
//!
//! Manages MCP servers via npm.

use crate::McpCommands;
use crate::error::Result;
use crate::interop::npm::{NpmClient, mcp_aliases};

use colored::Colorize;

/// Run an MCP server management command.
pub async fn run(command: McpCommands) -> Result<()> {
    let client = NpmClient::new();

    if !client.is_available() {
        eprintln!("{}", "Error: npm not found".red());
        eprintln!("Install Node.js from: {}", "https://nodejs.org".cyan());
        return Ok(());
    }

    match command {
        McpCommands::Add { name } => {
            let resolved = client.resolve_alias(&name);
            println!("{} {}", "Installing MCP server:".cyan(), resolved);

            match client.install(&name) {
                Ok(_) => {
                    println!(
                        "{} {}",
                        "✓".green(),
                        "MCP server installed successfully".green()
                    );
                    println!("\nTo use with Claude Code, add to your settings:");
                    println!("  {}", client.npx_command(&name).cyan());
                }
                Err(e) => {
                    eprintln!("{} {}: {}", "✗".red(), "Failed to install".red(), e);
                    std::process::exit(1);
                }
            }
        }
        McpCommands::Remove { name } => {
            let resolved = client.resolve_alias(&name);
            println!("{} {}", "Removing MCP server:".cyan(), resolved);

            match client.uninstall(&name) {
                Ok(_) => {
                    println!("{} {}", "✓".green(), "MCP server removed".green());
                }
                Err(e) => {
                    eprintln!("{} {}: {}", "✗".red(), "Failed to remove".red(), e);
                    std::process::exit(1);
                }
            }
        }
        McpCommands::List => match client.list_mcp_servers() {
            Ok(servers) => {
                if servers.is_empty() {
                    println!("{}", "No MCP servers installed".yellow());
                    println!("\nInstall with: {}", "spn mcp add <name>".cyan());
                    println!("\nAvailable aliases:");
                    for (alias, package) in mcp_aliases() {
                        println!("  {} → {}", alias.cyan(), package.dimmed());
                    }
                } else {
                    println!("{}", "Installed MCP servers:".cyan());
                    for server in &servers {
                        println!("  • {}", server);
                    }
                    println!("\n{} {} server(s)", "Total:".dimmed(), servers.len());
                }
            }
            Err(e) => {
                eprintln!("{} {}: {}", "✗".red(), "Failed to list servers".red(), e);
                std::process::exit(1);
            }
        },
        McpCommands::Test { name } => {
            let resolved = client.resolve_alias(&name);
            println!("{} {}", "Testing MCP server:".cyan(), resolved);

            match client.test_server(&name) {
                Ok(true) => {
                    println!("{} {}", "✓".green(), "Server responds correctly".green());
                }
                Ok(false) => {
                    println!("{} {}", "✗".red(), "Server did not respond".red());
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{} {}: {}", "✗".red(), "Test failed".red(), e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::interop::npm::{NpmClient, mcp_aliases};

    #[test]
    fn test_mcp_aliases() {
        let aliases = mcp_aliases();
        assert!(aliases.contains_key("neo4j"));
        assert!(aliases.contains_key("filesystem"));
    }

    #[test]
    fn test_resolve_alias() {
        let client = NpmClient::new();
        assert_eq!(client.resolve_alias("neo4j"), "@neo4j/mcp-server-neo4j");
        assert_eq!(client.resolve_alias("custom-pkg"), "custom-pkg");
    }
}
