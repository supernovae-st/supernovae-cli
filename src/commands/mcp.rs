//! MCP server command implementation.
//!
//! Manages MCP servers via the unified config at ~/.spn/mcp.yaml.
//! Servers are installed via npm but configuration is managed centrally.

use crate::error::Result;
use crate::interop::npm::{mcp_aliases, NpmClient};
use crate::mcp::{config_manager, McpConfigManager, McpScope, McpServer};
use crate::McpCommands;

use colored::Colorize;

/// Run an MCP server management command.
pub async fn run(command: McpCommands) -> Result<()> {
    let npm = NpmClient::new();
    let mcp = config_manager();

    match command {
        McpCommands::Add {
            name,
            global,
            project,
            no_sync,
            sync_to,
        } => {
            run_add(&npm, &mcp, &name, global, project, no_sync, sync_to).await
        }
        McpCommands::Remove {
            name,
            global,
            project,
        } => run_remove(&mcp, &name, global, project).await,
        McpCommands::List {
            global,
            project,
            json,
        } => run_list(&mcp, global, project, json).await,
        McpCommands::Test { name } => run_test(&npm, &mcp, &name).await,
    }
}

/// Add an MCP server.
async fn run_add(
    npm: &NpmClient,
    mcp: &McpConfigManager,
    name: &str,
    global: bool,
    project: bool,
    no_sync: bool,
    sync_to: Option<String>,
) -> Result<()> {
    // Check npm availability
    if !npm.is_available() {
        eprintln!("{}", "Error: npm not found".red());
        eprintln!("Install Node.js from: {}", "https://nodejs.org".cyan());
        return Ok(());
    }

    // Determine scope (default to global)
    let scope = determine_scope(global, project);
    let scope_display = match scope {
        McpScope::Global => "~/.spn/mcp.yaml".dimmed(),
        McpScope::Project => ".spn/mcp.yaml".dimmed(),
    };

    // Resolve alias to npm package
    let npm_package = npm.resolve_alias(name);
    println!(
        "{} {} {}",
        "Installing MCP server:".cyan(),
        name.bold(),
        format!("({})", npm_package).dimmed()
    );

    // Install via npm (globally)
    match npm.install(name) {
        Ok(_) => {
            println!("{} {}", "✓".green(), "npm package installed".green());
        }
        Err(e) => {
            eprintln!("{} {}: {}", "✗".red(), "Failed to install npm package".red(), e);
            std::process::exit(1);
        }
    }

    // Create MCP server config
    let server = create_server_from_alias(name, npm);

    // Add to config file
    match mcp.add_server(name, server, scope) {
        Ok(_) => {
            println!(
                "{} {} {} {}",
                "✓".green(),
                "Added to".green(),
                scope.to_string().green().bold(),
                scope_display
            );
        }
        Err(e) => {
            eprintln!("{} {}: {}", "✗".red(), "Failed to add to config".red(), e);
            std::process::exit(1);
        }
    }

    // Sync to editors (unless --no-sync)
    if !no_sync {
        sync_to_editors(name, sync_to.as_deref());
    } else {
        println!(
            "{} {}",
            "→".dimmed(),
            "Skipped editor sync (--no-sync)".dimmed()
        );
    }

    println!();
    println!("{}", "Server ready! Usage:".cyan());
    println!("  • Nika workflows: automatically available via ~/.spn/mcp.yaml");
    println!("  • Editors: synced via spn sync");

    Ok(())
}

/// Remove an MCP server.
async fn run_remove(
    mcp: &McpConfigManager,
    name: &str,
    global: bool,
    project: bool,
) -> Result<()> {
    let scope = determine_scope(global, project);

    println!(
        "{} {} {} {}",
        "Removing MCP server:".cyan(),
        name.bold(),
        "from".dimmed(),
        scope.to_string().dimmed()
    );

    match mcp.remove_server(name, scope) {
        Ok(true) => {
            println!("{} {}", "✓".green(), "Server removed from config".green());

            // Note: we don't uninstall from npm as other projects might use it
            println!(
                "{} {}",
                "→".dimmed(),
                "npm package kept (may be used by other projects)".dimmed()
            );
        }
        Ok(false) => {
            println!(
                "{} {} {}",
                "⚠".yellow(),
                "Server not found in".yellow(),
                scope.to_string().yellow()
            );
        }
        Err(e) => {
            eprintln!("{} {}: {}", "✗".red(), "Failed to remove".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// List MCP servers.
async fn run_list(
    mcp: &McpConfigManager,
    global: bool,
    project: bool,
    json: bool,
) -> Result<()> {
    let servers = if global {
        mcp.list_servers(McpScope::Global)?
    } else if project {
        mcp.list_servers(McpScope::Project)?
    } else {
        mcp.list_all_servers()?
    };

    if json {
        let json_output: Vec<_> = servers
            .iter()
            .map(|(name, server)| {
                serde_json::json!({
                    "name": name,
                    "command": server.command,
                    "args": server.args,
                    "enabled": server.enabled,
                    "source": server.source.map(|s| format!("{:?}", s).to_lowercase()),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
        return Ok(());
    }

    if servers.is_empty() {
        println!("{}", "No MCP servers configured".yellow());
        println!();
        println!("Add servers with:");
        println!("  {} {}", "spn mcp add".cyan(), "<name>".dimmed());
        println!();
        println!("Available aliases:");
        for (alias, package) in mcp_aliases().iter().take(10) {
            println!("  {} → {}", alias.cyan(), package.dimmed());
        }
        println!("  {} more...", "...38".dimmed());
        return Ok(());
    }

    // Header
    let scope_label = if global {
        "Global"
    } else if project {
        "Project"
    } else {
        "All"
    };
    println!(
        "{} {} {}",
        "MCP Servers".cyan().bold(),
        format!("({} scope)", scope_label).dimmed(),
        format!("[{} total]", servers.len()).dimmed()
    );
    println!();

    // Server list
    for (name, server) in &servers {
        let source_badge = match server.source {
            Some(crate::mcp::McpSource::Global) => "[G]".blue(),
            Some(crate::mcp::McpSource::Project) => "[P]".green(),
            Some(crate::mcp::McpSource::Workflow) => "[W]".yellow(),
            None => "[ ]".dimmed(),
        };

        let enabled_badge = if server.enabled {
            "✓".green()
        } else {
            "○".dimmed()
        };

        println!(
            "  {} {} {} {}",
            enabled_badge,
            source_badge,
            name.bold(),
            format!("({})", server.command).dimmed()
        );

        // Show description if available
        if let Some(desc) = &server.description {
            println!("      {}", desc.dimmed());
        }
    }

    println!();
    println!(
        "{} {} {}",
        "Legend:".dimmed(),
        "[G]".blue(),
        "Global".dimmed()
    );
    println!(
        "        {} {}",
        "[P]".green(),
        "Project".dimmed()
    );

    Ok(())
}

/// Test MCP server connection.
async fn run_test(npm: &NpmClient, mcp: &McpConfigManager, name: &str) -> Result<()> {
    if name == "all" {
        let servers = mcp.list_all_servers()?;
        if servers.is_empty() {
            println!("{}", "No servers to test".yellow());
            return Ok(());
        }

        println!("{} {} servers...", "Testing".cyan(), servers.len());
        println!();

        for (server_name, _) in &servers {
            test_single_server(npm, server_name);
        }
    } else {
        // Check if server exists in config
        if !mcp.has_server(name, McpScope::Global)?
            && !mcp.has_server(name, McpScope::Project).unwrap_or(false)
        {
            eprintln!(
                "{} {} {}",
                "✗".red(),
                "Server not found:".red(),
                name.bold()
            );
            eprintln!("  Add with: {}", format!("spn mcp add {}", name).cyan());
            std::process::exit(1);
        }

        test_single_server(npm, name);
    }

    Ok(())
}

/// Test a single server.
fn test_single_server(npm: &NpmClient, name: &str) {
    let _resolved = npm.resolve_alias(name);
    print!("  {} {}... ", "Testing".cyan(), name.bold());

    match npm.test_server(name) {
        Ok(true) => {
            println!("{}", "✓ OK".green());
        }
        Ok(false) => {
            println!("{}", "✗ No response".red());
        }
        Err(e) => {
            println!("{} {}", "✗ Error:".red(), e);
        }
    }
}

/// Determine scope from flags (default to global).
fn determine_scope(_global: bool, project: bool) -> McpScope {
    if project {
        McpScope::Project
    } else {
        // Default to global (--global is implicit default)
        McpScope::Global
    }
}

/// Create an McpServer config from an alias name.
fn create_server_from_alias(alias: &str, npm: &NpmClient) -> McpServer {
    let npm_package = npm.resolve_alias(alias);

    // Build npx command
    let (command, args) = if npm_package.starts_with('@') {
        ("npx".to_string(), vec!["-y".to_string(), npm_package.clone()])
    } else {
        ("npx".to_string(), vec!["-y".to_string(), npm_package.clone()])
    };

    McpServer::new(command)
        .with_args(args)
        .with_description(format!("MCP server: {}", npm_package))
        .with_enabled(true)
}

/// Sync MCP config to editors.
fn sync_to_editors(_name: &str, sync_to: Option<&str>) {
    // For now, just print what would be synced
    // Full sync implementation will come in Phase 2-3
    if let Some(targets) = sync_to {
        println!(
            "{} {} {}",
            "→".dimmed(),
            "Would sync to:".dimmed(),
            targets.cyan()
        );
    } else {
        println!(
            "{} {}",
            "→".dimmed(),
            "Will sync to configured editors on next `spn sync`".dimmed()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::npm::mcp_aliases;

    #[test]
    fn test_mcp_aliases() {
        let aliases = mcp_aliases();
        assert!(aliases.contains_key("neo4j"));
        assert!(aliases.contains_key("filesystem"));
        assert!(aliases.contains_key("github"));
        assert!(aliases.contains_key("perplexity"));
        assert_eq!(aliases.len(), 48);
    }

    #[test]
    fn test_resolve_alias() {
        let client = NpmClient::new();
        assert_eq!(client.resolve_alias("neo4j"), "@neo4j/mcp-server-neo4j");
        assert_eq!(
            client.resolve_alias("filesystem"),
            "@modelcontextprotocol/server-filesystem"
        );
        assert_eq!(client.resolve_alias("custom-pkg"), "custom-pkg");
    }

    #[test]
    fn test_determine_scope() {
        assert_eq!(determine_scope(false, false), McpScope::Global);
        assert_eq!(determine_scope(true, false), McpScope::Global);
        assert_eq!(determine_scope(false, true), McpScope::Project);
        assert_eq!(determine_scope(true, true), McpScope::Project); // project wins
    }

    #[test]
    fn test_create_server_from_alias() {
        let npm = NpmClient::new();
        let server = create_server_from_alias("neo4j", &npm);

        assert_eq!(server.command, "npx");
        assert_eq!(server.args, vec!["-y", "@neo4j/mcp-server-neo4j"]);
        assert!(server.enabled);
        assert!(server.description.is_some());
    }
}
