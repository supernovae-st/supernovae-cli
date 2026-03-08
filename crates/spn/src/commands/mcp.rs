//! MCP server command implementation.
//!
//! Manages MCP servers via the unified config at ~/.spn/mcp.yaml.
//! Servers are installed via npm but configuration is managed centrally.

use crate::error::{Result, SpnError};
use crate::interop::npm::{mcp_aliases, NpmClient};
use crate::mcp::{config_manager, McpConfigManager, McpScope, McpServer};
use crate::McpCommands;

use crate::ux::design_system as ds;

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
        } => run_add(&npm, &mcp, &name, global, project, no_sync, sync_to).await,
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
        McpCommands::Logs {
            name,
            follow,
            lines,
            level,
        } => run_logs(&mcp, name.as_deref(), follow, lines, level.as_deref()).await,
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
        eprintln!("{}", ds::error("Error: npm not found"));
        eprintln!(
            "Install Node.js from: {}",
            ds::primary("https://nodejs.org")
        );
        return Ok(());
    }

    // Determine scope (default to global)
    let scope = determine_scope(global, project);
    let scope_display = match scope {
        McpScope::Global => ds::muted("~/.spn/mcp.yaml"),
        McpScope::Project => ds::muted(".spn/mcp.yaml"),
    };

    // Resolve alias to npm package
    let npm_package = npm.resolve_alias(name);
    println!(
        "{} {} {}",
        ds::primary("Installing MCP server:"),
        ds::highlight(name),
        ds::muted(format!("({})", npm_package))
    );

    // Install via npm (globally)
    npm.install(name)
        .map_err(|e| SpnError::CommandFailed(format!("Failed to install npm package: {}", e)))?;
    println!(
        "{} {}",
        ds::success("✓"),
        ds::success("npm package installed")
    );

    // Create MCP server config
    let server = create_server_from_alias(name, npm);

    // Add to config file
    mcp.add_server(name, server, scope)
        .map_err(|e| SpnError::CommandFailed(format!("Failed to add to config: {}", e)))?;
    println!(
        "{} {} {} {}",
        ds::success("✓"),
        ds::success("Added to"),
        ds::success(scope.to_string()).bold(),
        scope_display
    );

    // Sync to editors (unless --no-sync)
    if !no_sync {
        sync_to_editors(name, sync_to.as_deref());
    } else {
        println!(
            "{} {}",
            ds::muted("→"),
            ds::muted("Skipped editor sync (--no-sync)")
        );
    }

    println!();
    println!("{}", ds::primary("Server ready! Usage:"));
    println!("  • Nika workflows: automatically available via ~/.spn/mcp.yaml");
    println!("  • Editors: synced via spn sync");

    Ok(())
}

/// Remove an MCP server.
async fn run_remove(mcp: &McpConfigManager, name: &str, global: bool, project: bool) -> Result<()> {
    let scope = determine_scope(global, project);

    println!(
        "{} {} {} {}",
        ds::primary("Removing MCP server:"),
        ds::highlight(name),
        ds::muted("from"),
        ds::muted(scope.to_string())
    );

    match mcp.remove_server(name, scope) {
        Ok(true) => {
            println!(
                "{} {}",
                ds::success("✓"),
                ds::success("Server removed from config")
            );

            // Note: we don't uninstall from npm as other projects might use it
            println!(
                "{} {}",
                ds::muted("→"),
                ds::muted("npm package kept (may be used by other projects)")
            );
        }
        Ok(false) => {
            println!(
                "{} {} {}",
                ds::warning("⚠"),
                ds::warning("Server not found in"),
                ds::warning(scope.to_string())
            );
        }
        Err(e) => {
            return Err(SpnError::CommandFailed(format!("Failed to remove: {}", e)));
        }
    }

    Ok(())
}

/// List MCP servers.
async fn run_list(mcp: &McpConfigManager, global: bool, project: bool, json: bool) -> Result<()> {
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
        println!("{}", ds::warning("No MCP servers configured"));
        println!();
        println!("Add servers with:");
        println!("  {} {}", ds::primary("spn mcp add"), ds::muted("<name>"));
        println!();
        println!("Available aliases:");
        for (alias, package) in mcp_aliases().iter().take(10) {
            println!("  {} → {}", ds::primary(alias), ds::muted(package));
        }
        println!("  {} more...", ds::muted("...38"));
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
        ds::primary("MCP Servers"),
        ds::muted(format!("({} scope)", scope_label)),
        ds::muted(format!("[{} total]", servers.len()))
    );
    println!();

    // Server list
    for (name, server) in &servers {
        let source_badge = match server.source {
            Some(crate::mcp::McpSource::Global) => ds::primary("[G]"),
            Some(crate::mcp::McpSource::Project) => ds::success("[P]"),
            Some(crate::mcp::McpSource::Workflow) => ds::warning("[W]"),
            None => ds::muted("[ ]"),
        };

        let enabled_badge = if server.enabled {
            ds::success("✓")
        } else {
            ds::muted("○")
        };

        println!(
            "  {} {} {} {}",
            enabled_badge,
            source_badge,
            ds::highlight(name),
            ds::muted(format!("({})", server.command))
        );

        // Show description if available
        if let Some(desc) = &server.description {
            println!("      {}", ds::muted(desc));
        }
    }

    println!();
    println!(
        "{} {} {}",
        ds::muted("Legend:"),
        ds::primary("[G]"),
        ds::muted("Global")
    );
    println!("        {} {}", ds::success("[P]"), ds::muted("Project"));

    Ok(())
}

/// Test MCP server connection.
async fn run_test(npm: &NpmClient, mcp: &McpConfigManager, name: &str) -> Result<()> {
    if name == "all" {
        let servers = mcp.list_all_servers()?;
        if servers.is_empty() {
            println!("{}", ds::warning("No servers to test"));
            return Ok(());
        }

        println!("{} {} servers...", ds::primary("Testing"), servers.len());
        println!();

        for (server_name, _) in &servers {
            test_single_server(npm, server_name);
        }
    } else {
        // Check if server exists in config
        if !mcp.has_server(name, McpScope::Global)?
            && !mcp.has_server(name, McpScope::Project).unwrap_or(false)
        {
            return Err(SpnError::CommandFailed(format!(
                "Server not found: {}\n  Add with: spn mcp add {}",
                name, name
            )));
        }

        test_single_server(npm, name);
    }

    Ok(())
}

/// Test a single server.
fn test_single_server(npm: &NpmClient, name: &str) {
    let _resolved = npm.resolve_alias(name);
    print!("  {} {}... ", ds::primary("Testing"), ds::highlight(name));

    match npm.test_server(name) {
        Ok(true) => {
            println!("{}", ds::success("✓ OK"));
        }
        Ok(false) => {
            println!("{}", ds::error("✗ No response"));
        }
        Err(e) => {
            println!("{} {}", ds::error("✗ Error:"), e);
        }
    }
}

/// View MCP server logs.
async fn run_logs(
    mcp: &McpConfigManager,
    name: Option<&str>,
    follow: bool,
    lines: usize,
    level: Option<&str>,
) -> Result<()> {
    // Get logs directory
    let logs_dir = dirs::home_dir()
        .ok_or_else(|| SpnError::ConfigError("Could not find home directory".into()))?
        .join(".spn/logs/mcp");

    // Validate level filter if provided
    if let Some(lvl) = level {
        match lvl.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" | "trace" => {}
            _ => {
                return Err(SpnError::InvalidInput(format!(
                    "Invalid log level '{}'. Use: debug, info, warn, error, trace",
                    lvl
                )));
            }
        }
    }

    // Determine which servers to show logs for
    let servers: Vec<String> = if let Some(server_name) = name {
        if server_name == "all" {
            mcp.list_all_servers()?
                .into_iter()
                .map(|(n, _)| n)
                .collect()
        } else {
            // Verify server exists
            if !mcp.has_server(server_name, McpScope::Global)?
                && !mcp
                    .has_server(server_name, McpScope::Project)
                    .unwrap_or(false)
            {
                return Err(SpnError::CommandFailed(format!(
                    "Server not found: {}",
                    server_name
                )));
            }
            vec![server_name.to_string()]
        }
    } else {
        // Default to all servers
        mcp.list_all_servers()?
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    };

    if servers.is_empty() {
        println!("{}", ds::info_line("No MCP servers configured"));
        println!();
        println!("Add servers with: {}", ds::command("spn mcp add <name>"));
        return Ok(());
    }

    // Check if logs directory exists
    if !logs_dir.exists() {
        println!("{}", ds::section("MCP Logs"));
        println!();
        println!("{}", ds::info_line("No log files found"));
        println!();
        println!("  Log directory: {}", ds::path(logs_dir.display()));
        println!();
        println!(
            "  {}",
            ds::muted("MCP servers log to stderr when started by Claude Code or Nika.")
        );
        println!(
            "  {}",
            ds::muted("To capture logs, run with: spn daemon start --capture-mcp-logs")
        );
        return Ok(());
    }

    // Show logs header
    let server_display = if servers.len() == 1 {
        servers[0].clone()
    } else {
        format!("{} servers", servers.len())
    };

    println!("{}", ds::section(format!("MCP Logs: {}", server_display)));

    if follow {
        println!(
            "{}",
            ds::hint_line("Following logs... Press Ctrl+C to stop")
        );
    }

    // Display logs for each server
    for server in &servers {
        let log_file = logs_dir.join(format!("{}.log", server));

        if !log_file.exists() {
            println!(
                "  {} {} {}",
                ds::muted("["),
                ds::highlight(server),
                ds::muted("] No log file")
            );
            continue;
        }

        // Read and display log lines
        let content = std::fs::read_to_string(&log_file)?;
        let all_lines: Vec<&str> = content.lines().collect();

        // Apply level filter if specified
        let filtered_lines: Vec<&str> = if let Some(lvl) = level {
            let level_upper = lvl.to_uppercase();
            all_lines
                .into_iter()
                .filter(|line| line.contains(&level_upper))
                .collect()
        } else {
            all_lines
        };

        // Get last N lines
        let start = if filtered_lines.len() > lines {
            filtered_lines.len() - lines
        } else {
            0
        };
        let display_lines = &filtered_lines[start..];

        if display_lines.is_empty() {
            println!(
                "  {} {} {}",
                ds::muted("["),
                ds::highlight(server),
                ds::muted("] Empty log file")
            );
            continue;
        }

        // Print server header
        println!("  {} {}", ds::primary("━━━"), ds::highlight(server));

        // Print log lines with syntax coloring
        for line in display_lines {
            print_colored_log_line(line);
        }

        println!();
    }

    if follow {
        // For follow mode, we'd need to implement tail -f behavior
        // This is a placeholder for now - real implementation would use notify or similar
        println!(
            "{}",
            ds::info_line("Follow mode not yet implemented. Use: tail -f ~/.spn/logs/mcp/*.log")
        );
    }

    Ok(())
}

/// Print a log line with level-based coloring.
fn print_colored_log_line(line: &str) {
    let line_upper = line.to_uppercase();

    if line_upper.contains("ERROR") || line_upper.contains("ERR]") {
        println!("    {}", ds::error(line));
    } else if line_upper.contains("WARN") {
        println!("    {}", ds::warning(line));
    } else if line_upper.contains("DEBUG") || line_upper.contains("TRACE") {
        println!("    {}", ds::muted(line));
    } else {
        println!("    {}", line);
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

    // Build npx command (works with scoped @org/pkg and regular packages)
    let (command, args) = (
        "npx".to_string(),
        vec!["-y".to_string(), npm_package.clone()],
    );

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
            ds::muted("→"),
            ds::muted("Would sync to:"),
            ds::primary(targets)
        );
    } else {
        println!(
            "{} {}",
            ds::muted("→"),
            ds::muted("Will sync to configured editors on next `spn sync`")
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

    // =========================================================================
    // Log coloring tests
    // =========================================================================

    #[test]
    fn test_print_colored_log_line_error() {
        // Just verify it doesn't panic - output is to stdout
        print_colored_log_line("2024-01-15 10:30:00 ERROR Failed to connect");
        print_colored_log_line("[ERR] Connection refused");
    }

    #[test]
    fn test_print_colored_log_line_warn() {
        print_colored_log_line("2024-01-15 10:30:00 WARN Retry attempt 2");
        print_colored_log_line("WARNING: deprecated config");
    }

    #[test]
    fn test_print_colored_log_line_debug() {
        print_colored_log_line("DEBUG: entering function");
        print_colored_log_line("2024-01-15 10:30:00 DEBUG variable = 42");
    }

    #[test]
    fn test_print_colored_log_line_info() {
        print_colored_log_line("2024-01-15 10:30:00 INFO Server started");
        print_colored_log_line("Just a normal log line");
    }
}
