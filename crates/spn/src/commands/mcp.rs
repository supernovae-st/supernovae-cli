//! MCP server command implementation.
//!
//! Manages MCP servers via the unified config at ~/.spn/mcp.yaml.
//! Servers are installed via npm but configuration is managed centrally.

use crate::error::{Result, SpnError};
use crate::interop::npm::{mcp_aliases, NpmClient};
use crate::mcp::{config_manager, McpConfigManager, McpScope, McpServer};
use crate::prompts;
use crate::{ApisCommands, McpCommands};

use crate::ux::design_system as ds;

// Re-export spn-mcp config types for the apis subcommand
use spn_mcp::config::{apis_dir, load_all_apis, load_api, validate};

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
            let name = match name {
                Some(n) => n,
                None => prompts::select_mcp_server()?,
            };
            run_add(&npm, &mcp, &name, global, project, no_sync, sync_to).await
        }
        McpCommands::Remove {
            name,
            global,
            project,
        } => {
            let name = match name {
                Some(n) => n,
                None => prompts::select_mcp_server()?,
            };
            run_remove(&mcp, &name, global, project).await
        }
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
        McpCommands::Serve { api } => run_serve(api.as_deref()).await,
        McpCommands::Apis { command } => run_apis(command).await,
        McpCommands::Wrap {
            from_openapi,
            name,
            base_url,
            yes,
        } => run_wrap(from_openapi, name, base_url, yes).await,
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

// ============================================================================
// DYNAMIC REST-TO-MCP SERVER (spn-mcp integration)
// ============================================================================

/// Start the dynamic REST-to-MCP server.
async fn run_serve(api: Option<&str>) -> Result<()> {
    use spn_mcp::server::DynamicHandler;

    println!(
        "{} {}",
        ds::primary("Starting"),
        ds::highlight("spn-mcp dynamic server")
    );

    // Load API configurations
    let configs = if let Some(api_name) = api {
        println!("{} {}", ds::muted("Loading API:"), ds::highlight(api_name));
        let config = load_api(api_name).map_err(|e| {
            SpnError::CommandFailed(format!("Failed to load API '{}': {}", api_name, e))
        })?;

        // Validate the config
        validate(&config).map_err(|e| {
            SpnError::CommandFailed(format!("Invalid config '{}': {}", api_name, e))
        })?;

        vec![config]
    } else {
        println!(
            "{} {}",
            ds::muted("Loading all APIs from:"),
            ds::path(apis_dir().unwrap_or_default().display())
        );
        let configs = load_all_apis()
            .map_err(|e| SpnError::CommandFailed(format!("Failed to load APIs: {}", e)))?;

        if configs.is_empty() {
            println!();
            println!("{}", ds::warning("No API configurations found"));
            println!();
            println!("Create a config file in: {}", ds::path("~/.spn/apis/"));
            println!();
            println!("Example: {}", ds::path("~/.spn/apis/dataforseo.yaml"));
            println!();
            println!("See: {}", ds::primary("spn topic mcp-apis"));
            return Ok(());
        }

        // Validate all configs
        for config in &configs {
            validate(config).map_err(|e| {
                SpnError::CommandFailed(format!("Invalid config '{}': {}", config.name, e))
            })?;
        }

        configs
    };

    // Show loaded APIs
    println!();
    println!("{} {} API(s) loaded:", ds::success("✓"), configs.len());
    for config in &configs {
        let tool_count = config.tools.len();
        println!(
            "  {} {} {}",
            ds::primary("•"),
            ds::highlight(&config.name),
            ds::muted(format!("({} tools)", tool_count))
        );
    }
    println!();

    // Create and run the MCP server
    println!(
        "{} {}",
        ds::primary("→"),
        ds::muted("Starting MCP server on stdio...")
    );

    let handler = DynamicHandler::new(configs)
        .await
        .map_err(|e| SpnError::CommandFailed(format!("Failed to create handler: {}", e)))?;

    handler
        .run()
        .await
        .map_err(|e| SpnError::CommandFailed(format!("MCP server error: {}", e)))?;

    Ok(())
}

/// Wrap a REST API as MCP tools (interactive wizard).
async fn run_wrap(
    from_openapi: Option<std::path::PathBuf>,
    name: Option<String>,
    base_url: Option<String>,
    yes: bool,
) -> Result<()> {
    use dialoguer::{Confirm, Input, Select};
    use spn_mcp::config::{ApiConfig, ApiKeyLocation, AuthConfig, AuthType, ToolDef};

    // OpenAPI import mode
    if let Some(spec_path) = from_openapi {
        return run_openapi_import(&spec_path, name, yes).await;
    }

    // Print banner
    println!();
    println!("╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!(
        "║  {}  MCP WRAPPER WIZARD                                                        ║",
        ds::primary("🛠️")
    );
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Prompt for API name
    let api_name = match name {
        Some(n) => n,
        None => Input::<String>::new()
            .with_prompt("API name")
            .interact_text()
            .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?,
    };

    // Prompt for base URL
    let api_base_url = match base_url {
        Some(u) => u,
        None => Input::<String>::new()
            .with_prompt("Base URL")
            .interact_text()
            .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?,
    };

    // Prompt for auth type
    let auth_types = vec![
        "Bearer Token",
        "API Key (header)",
        "API Key (query)",
        "Basic Auth",
        "None (skip for now)",
    ];
    let auth_selection = Select::new()
        .with_prompt("Authentication")
        .items(&auth_types)
        .default(0)
        .interact()
        .map_err(|e| SpnError::CommandFailed(format!("Select error: {}", e)))?;

    // Build auth config
    let auth = match auth_selection {
        0 => AuthConfig {
            auth_type: AuthType::Bearer,
            credential: api_name.clone(),
            location: None,
            key_name: None,
        },
        1 => {
            let header_name: String = Input::new()
                .with_prompt("Header name")
                .default("X-API-Key".to_string())
                .interact_text()
                .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?;
            AuthConfig {
                auth_type: AuthType::ApiKey,
                credential: api_name.clone(),
                location: Some(ApiKeyLocation::Header),
                key_name: Some(header_name),
            }
        }
        2 => {
            let param_name: String = Input::new()
                .with_prompt("Query param name")
                .default("api_key".to_string())
                .interact_text()
                .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?;
            AuthConfig {
                auth_type: AuthType::ApiKey,
                credential: api_name.clone(),
                location: Some(ApiKeyLocation::Query),
                key_name: Some(param_name),
            }
        }
        3 => AuthConfig {
            auth_type: AuthType::Basic,
            credential: api_name.clone(),
            location: None,
            key_name: None,
        },
        _ => AuthConfig {
            auth_type: AuthType::Bearer,
            credential: "placeholder".to_string(),
            location: None,
            key_name: None,
        },
    };

    println!();
    println!("╭─────────────────────────────────────────────────────────────────────────────────╮");
    println!("│  Adding endpoints...                                                            │");
    println!("╰─────────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Collect tools
    let mut tools: Vec<ToolDef> = Vec::new();
    let mut endpoint_num = 1;

    loop {
        // Method
        let methods = vec!["GET", "POST", "PUT", "PATCH", "DELETE"];
        let method_idx = Select::new()
            .with_prompt(format!("Endpoint {} - Method", endpoint_num))
            .items(&methods)
            .default(0)
            .interact()
            .map_err(|e| SpnError::CommandFailed(format!("Select error: {}", e)))?;
        let method = methods[method_idx].to_string();

        // Path
        let path: String = Input::new()
            .with_prompt(format!("Endpoint {} - Path", endpoint_num))
            .interact_text()
            .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?;

        // Auto-generate tool name
        let default_tool_name = generate_tool_name(&api_name, &method, &path);
        let tool_name: String = Input::new()
            .with_prompt(format!("Endpoint {} - Tool name", endpoint_num))
            .default(default_tool_name)
            .interact_text()
            .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?;

        // Description
        let description: String = Input::new()
            .with_prompt(format!("Endpoint {} - Description", endpoint_num))
            .allow_empty(true)
            .interact_text()
            .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?;

        // Extract path params
        let params = extract_path_params(&path);

        tools.push(ToolDef {
            name: tool_name,
            description: if description.is_empty() {
                None
            } else {
                Some(description)
            },
            method,
            path,
            body_template: None,
            params,
            response: None,
        });

        endpoint_num += 1;

        // Ask to continue
        let add_more = if yes {
            false
        } else {
            Confirm::new()
                .with_prompt("Add another endpoint?")
                .default(true)
                .interact()
                .map_err(|e| SpnError::CommandFailed(format!("Confirm error: {}", e)))?
        };

        if !add_more {
            break;
        }
    }

    // Build config
    let config = ApiConfig {
        name: api_name.clone(),
        version: "1.0".to_string(),
        base_url: api_base_url,
        description: None,
        auth,
        rate_limit: None,
        headers: None,
        tools,
    };

    // Save config
    let apis_dir = apis_dir().map_err(|e| SpnError::CommandFailed(e.to_string()))?;
    std::fs::create_dir_all(&apis_dir)?;
    let config_path = apis_dir.join(format!("{}.yaml", api_name));

    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| SpnError::CommandFailed(format!("YAML serialization error: {}", e)))?;
    std::fs::write(&config_path, &yaml)?;

    println!();
    println!(
        "{} Created {} ({} tools)",
        ds::success("✓"),
        ds::path(config_path.display()),
        config.tools.len()
    );

    // Validate by re-loading
    match spn_mcp::config::load_api(&api_name) {
        Ok(_) => println!("{} Validated configuration", ds::success("✓")),
        Err(e) => println!("{} Validation warning: {}", ds::warning("⚠"), e),
    }

    // Ask to start server (skip in non-interactive mode)
    if !yes {
        println!();
        let start_now = Confirm::new()
            .with_prompt("Start server now?")
            .default(false)
            .interact()
            .map_err(|e| SpnError::CommandFailed(format!("Confirm error: {}", e)))?;

        if start_now {
            println!();
            println!(
                "{}",
                ds::hint_line(format!("Run: spn mcp apis start {}", api_name))
            );
        }
    }

    Ok(())
}

/// Import endpoints from an OpenAPI spec file.
async fn run_openapi_import(
    spec_path: &std::path::Path,
    name: Option<String>,
    yes: bool,
) -> Result<()> {
    use dialoguer::{Input, MultiSelect, Select};

    // Print banner
    println!();
    println!("╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!(
        "║  {}  MCP WRAPPER — OpenAPI Import                                              ║",
        ds::primary("🛠️")
    );
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Parse the OpenAPI spec
    println!("{}...", ds::muted("Parsing OpenAPI spec"));

    let spec = spn_mcp::parse_openapi(spec_path)
        .map_err(|e| SpnError::CommandFailed(format!("Failed to parse OpenAPI spec: {}", e)))?;

    // Show what we found
    println!(
        "Found: {} {}",
        ds::primary(&spec.info.title),
        ds::muted(format!("v{}", spec.info.version))
    );
    if let Some(server) = spec.servers.first() {
        println!("Base URL: {}", ds::path(&server.url));
    }
    println!();

    let endpoint_count = spec.endpoint_count();
    println!(
        "Discovered {} endpoints:",
        ds::primary(endpoint_count.to_string())
    );

    // Show sample endpoints
    let tools = spec.to_api_config(None).tools;
    for tool in tools.iter().take(5) {
        println!(
            "  • {} {} → {}",
            ds::muted(&tool.method),
            ds::path(&tool.path),
            ds::primary(&tool.name)
        );
    }
    if endpoint_count > 5 {
        println!("  ... ({} more)", endpoint_count - 5);
    }
    println!();

    // Selection mode
    let import_options = vec![
        format!("All ({} endpoints)", endpoint_count),
        "Select interactively".to_string(),
        "Filter by tag".to_string(),
    ];

    let selected_tools: Vec<spn_mcp::config::ToolDef>;

    if yes {
        // Non-interactive: import all
        selected_tools = tools;
    } else {
        let selection = Select::new()
            .with_prompt("Import endpoints")
            .items(&import_options)
            .default(0)
            .interact()
            .map_err(|e| SpnError::CommandFailed(format!("Select error: {}", e)))?;

        match selection {
            0 => {
                // All endpoints
                selected_tools = tools;
            }
            1 => {
                // Select interactively
                let tool_labels: Vec<_> = tools
                    .iter()
                    .map(|t| format!("{} {} - {}", t.method, t.path, t.name))
                    .collect();

                let selections = MultiSelect::new()
                    .with_prompt("Select endpoints (space to toggle)")
                    .items(&tool_labels)
                    .interact()
                    .map_err(|e| SpnError::CommandFailed(format!("MultiSelect error: {}", e)))?;

                if selections.is_empty() {
                    println!("{}", ds::warning("No endpoints selected."));
                    return Ok(());
                }

                selected_tools = selections.into_iter().map(|i| tools[i].clone()).collect();
            }
            2 => {
                // Filter by tag
                let tags = spec.tags();
                if tags.is_empty() {
                    println!("{}", ds::warning("No tags found in spec. Importing all."));
                    selected_tools = tools;
                } else {
                    let tag_selection = Select::new()
                        .with_prompt("Select tag")
                        .items(&tags)
                        .interact()
                        .map_err(|e| SpnError::CommandFailed(format!("Select error: {}", e)))?;

                    let selected_tag = &tags[tag_selection];

                    // Filter tools by tag
                    let tag_tools = spec.tools_by_tag(selected_tag);
                    selected_tools = tag_tools
                        .iter()
                        .map(|(path, method, op)| {
                            let tool_name = op
                                .operation_id
                                .clone()
                                .unwrap_or_else(|| generate_tool_name("api", method, path));
                            spn_mcp::config::ToolDef {
                                name: tool_name,
                                description: op.summary.clone().or_else(|| op.description.clone()),
                                method: method.to_string(),
                                path: path.to_string(),
                                body_template: None,
                                params: Vec::new(),
                                response: None,
                            }
                        })
                        .collect();

                    if selected_tools.is_empty() {
                        println!(
                            "{}",
                            ds::warning(format!("No endpoints found with tag '{}'.", selected_tag))
                        );
                        return Ok(());
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    // Get API name
    let api_name = name.unwrap_or_else(|| {
        spec.info
            .title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    });

    // Build config
    let mut config = spec.to_api_config(Some(&api_name));
    config.tools = selected_tools;

    // Prompt for credential name if not in non-interactive mode
    if !yes {
        let credential: String = Input::new()
            .with_prompt("Auth credential name")
            .default(api_name.clone())
            .interact_text()
            .map_err(|e| SpnError::CommandFailed(format!("Input error: {}", e)))?;
        config.auth.credential = credential;
    }

    // Save config
    let apis_dir = apis_dir().map_err(|e| SpnError::CommandFailed(e.to_string()))?;
    std::fs::create_dir_all(&apis_dir)?;
    let config_path = apis_dir.join(format!("{}.yaml", api_name));

    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| SpnError::CommandFailed(format!("YAML serialization error: {}", e)))?;
    std::fs::write(&config_path, &yaml)?;

    println!();
    println!(
        "{} Created {} ({} tools)",
        ds::success("✓"),
        ds::path(config_path.display()),
        config.tools.len()
    );

    // Validate by re-loading
    match spn_mcp::config::load_api(&api_name) {
        Ok(_) => println!("{} Validated configuration", ds::success("✓")),
        Err(e) => println!("{} Validation warning: {}", ds::warning("⚠"), e),
    }

    Ok(())
}

/// Generate a tool name from method and path.
fn generate_tool_name(_api_name: &str, method: &str, path: &str) -> String {
    // Convert /repos/{owner}/{repo}/issues -> repos_repo_issues
    let path_part: String = path
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{'))
        .collect::<Vec<_>>()
        .join("_");

    let method_upper = method.to_uppercase();
    let method_prefix = match method_upper.as_str() {
        "GET" => "get",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "call",
    };

    format!("{}_{}", method_prefix, path_part)
        .trim_matches('_')
        .to_string()
}

/// Extract path parameters from URL template (e.g., /repos/{owner}/{repo}).
fn extract_path_params(path: &str) -> Vec<spn_mcp::config::ParamDef> {
    use spn_mcp::config::{ParamDef, ParamType};

    let mut params = Vec::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            let mut param_name = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    break;
                }
                param_name.push(chars.next().unwrap());
            }
            if !param_name.is_empty() {
                params.push(ParamDef {
                    name: param_name,
                    param_type: ParamType::String,
                    items: None,
                    required: true,
                    default: None,
                    description: None,
                });
            }
        }
    }

    params
}

/// Handle APIs subcommand.
async fn run_apis(command: ApisCommands) -> Result<()> {
    match command {
        ApisCommands::List { json } => run_apis_list(json).await,
        ApisCommands::Validate { name } => run_apis_validate(&name).await,
        ApisCommands::Info { name } => run_apis_info(&name).await,
    }
}

/// List configured REST API wrappers.
async fn run_apis_list(json: bool) -> Result<()> {
    let dir = apis_dir().map_err(|e| SpnError::CommandFailed(e.to_string()))?;
    let configs = load_all_apis()
        .map_err(|e| SpnError::CommandFailed(format!("Failed to load APIs: {}", e)))?;

    if json {
        let json_output: Vec<_> = configs
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "base_url": c.base_url,
                    "description": c.description,
                    "tools": c.tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
        return Ok(());
    }

    if configs.is_empty() {
        println!("{}", ds::warning("No REST API wrappers configured"));
        println!();
        println!("Create YAML configs in: {}", ds::path(dir.display()));
        println!();
        println!("Example:");
        println!("  {}", ds::muted("# ~/.spn/apis/example.yaml"));
        println!("  {}", ds::muted("name: example"));
        println!("  {}", ds::muted("base_url: https://api.example.com"));
        println!("  {}", ds::muted("auth:"));
        println!("  {}", ds::muted("  type: bearer"));
        println!("  {}", ds::muted("  credential: example"));
        println!("  {}", ds::muted("tools:"));
        println!("  {}", ds::muted("  - name: get_data"));
        println!("  {}", ds::muted("    path: /data"));
        return Ok(());
    }

    println!(
        "{} {} {}",
        ds::primary("REST API Wrappers"),
        ds::muted(format!("({})", dir.display())),
        ds::muted(format!("[{} total]", configs.len()))
    );
    println!();

    for config in &configs {
        let tool_names: Vec<_> = config.tools.iter().map(|t| t.name.as_str()).collect();
        println!(
            "  {} {} {}",
            ds::success("•"),
            ds::highlight(&config.name),
            ds::muted(format!("→ {}", config.base_url))
        );

        if let Some(desc) = &config.description {
            println!("    {}", ds::muted(desc));
        }

        println!(
            "    {} {}",
            ds::muted("Tools:"),
            ds::primary(tool_names.join(", "))
        );
        println!();
    }

    println!(
        "{} {}",
        ds::muted("Start server:"),
        ds::command("spn mcp serve")
    );

    Ok(())
}

/// Validate an API configuration.
async fn run_apis_validate(name: &str) -> Result<()> {
    println!("{} {}", ds::primary("Validating:"), ds::highlight(name));

    let config = load_api(name)
        .map_err(|e| SpnError::CommandFailed(format!("Failed to load '{}': {}", name, e)))?;

    match validate(&config) {
        Ok(()) => {
            println!();
            println!(
                "{} {}",
                ds::success("✓"),
                ds::success("Configuration is valid")
            );
            println!();
            println!("  {} {}", ds::muted("Name:"), ds::highlight(&config.name));
            println!("  {} {}", ds::muted("Base URL:"), config.base_url);
            println!(
                "  {} {}",
                ds::muted("Auth:"),
                format!("{:?}", config.auth.auth_type).to_lowercase()
            );
            println!("  {} {}", ds::muted("Tools:"), config.tools.len());

            for tool in &config.tools {
                println!(
                    "    {} {} {}",
                    ds::primary("•"),
                    ds::highlight(&tool.name),
                    ds::muted(format!("{} {}", tool.method, tool.path))
                );
            }
        }
        Err(e) => {
            println!();
            println!("{} {}", ds::error("✗"), ds::error("Validation failed"));
            println!("  {}", ds::error(e.to_string()));
        }
    }

    Ok(())
}

/// Show API configuration details.
async fn run_apis_info(name: &str) -> Result<()> {
    let config = load_api(name)
        .map_err(|e| SpnError::CommandFailed(format!("Failed to load '{}': {}", name, e)))?;

    println!("{}", ds::section(format!("API: {}", config.name)));
    println!();

    // Basic info
    println!("  {} {}", ds::muted("Version:"), config.version);
    println!(
        "  {} {}",
        ds::muted("Base URL:"),
        ds::primary(&config.base_url)
    );

    if let Some(desc) = &config.description {
        println!("  {} {}", ds::muted("Description:"), desc);
    }

    // Auth info
    println!();
    println!("{}", ds::section("Authentication"));
    println!(
        "  {} {}",
        ds::muted("Type:"),
        format!("{:?}", config.auth.auth_type).to_lowercase()
    );
    println!(
        "  {} {}",
        ds::muted("Credential:"),
        ds::highlight(&config.auth.credential)
    );

    // Rate limit info
    if let Some(rate) = &config.rate_limit {
        println!();
        println!("{}", ds::section("Rate Limits"));
        println!(
            "  {} {}/min",
            ds::muted("Requests:"),
            rate.requests_per_minute
        );
        if rate.burst > 1 {
            println!("  {} {}", ds::muted("Burst:"), rate.burst);
        }
    }

    // Tools
    println!();
    println!(
        "{} {}",
        ds::section("Tools"),
        ds::muted(format!("[{}]", config.tools.len()))
    );

    for tool in &config.tools {
        println!();
        println!(
            "  {} {}",
            ds::primary("•"),
            ds::highlight(format!("{}_{}", config.name, tool.name))
        );
        println!(
            "    {} {} {}",
            ds::muted("Endpoint:"),
            ds::primary(&tool.method),
            tool.path
        );

        if let Some(desc) = &tool.description {
            println!("    {} {}", ds::muted("Description:"), desc);
        }

        if !tool.params.is_empty() {
            let param_names: Vec<_> = tool
                .params
                .iter()
                .map(|p| {
                    if p.required {
                        format!("{}*", p.name)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            println!("    {} {}", ds::muted("Params:"), param_names.join(", "));
        }
    }

    Ok(())
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
