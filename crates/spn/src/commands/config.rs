//! Config command implementation.
//!
//! Manages configuration across three scopes: Global, Team, Local.

use std::env;

use colored::Colorize;

use crate::config::{global, local, scope::ScopeType, team, ConfigResolver};
use crate::error::{Result, SpnError};
use crate::ConfigCommands;

pub async fn run(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show { section } => show_config(section).await,
        ConfigCommands::Where => show_locations().await,
        ConfigCommands::List { show_origin } => list_config(show_origin).await,
        ConfigCommands::Get { key, show_origin } => get_value(&key, show_origin).await,
        ConfigCommands::Set { key, value, scope } => set_value(&key, &value, &scope).await,
        ConfigCommands::Edit { local, user, mcp } => edit_config(local, user, mcp).await,
        ConfigCommands::Import { file, scope, yes } => import_config(&file, &scope, yes).await,
    }
}

async fn show_config(_section: Option<String>) -> Result<()> {
    let resolver = ConfigResolver::load()?;
    let config = resolver.resolved();
    let scopes = resolver.get_scope_paths()?;

    println!("{}", "⚙️  Resolved Configuration".cyan().bold());
    println!();

    let mut has_config = false;

    // Show providers
    if !config.providers.is_empty() {
        has_config = true;
        println!("{}", "Providers:".bold());
        for (name, provider) in &config.providers {
            if let Some(model) = &provider.model {
                println!("  {} model = {}", name.cyan(), model);
            }
            if let Some(endpoint) = &provider.endpoint {
                println!("  {} endpoint = {}", name.cyan(), endpoint);
            }
        }
        println!();
    }

    // Show sync config
    if !config.sync.enabled_editors.is_empty() || config.sync.auto_sync {
        has_config = true;
        println!("{}", "Sync:".bold());
        println!("  enabled_editors = {:?}", config.sync.enabled_editors);
        println!("  auto_sync = {}", config.sync.auto_sync);
        println!();
    }

    // Show MCP servers
    if !config.servers.is_empty() {
        has_config = true;
        println!("{}", "MCP Servers:".bold());
        for (name, server) in &config.servers {
            let status = if server.disabled { "(disabled)" } else { "" };
            println!("  {} {} {}", name.cyan(), server.command, status.dimmed());
        }
        println!();
    }

    // Show message if no config found
    if !has_config {
        println!("  {}", "No configuration found.".dimmed());
        println!();
        println!("{}", "Config File Locations:".bold());
        for scope in &scopes {
            let status = if scope.exists {
                "✓".green()
            } else {
                "○".dimmed()
            };
            println!("  {} {}", status, scope.display_name());
        }
        println!();
        println!("{}", "Quick Start:".bold());
        println!("  {} Create project manifest", "spn init".cyan());
        println!("  {} Add MCP server", "spn mcp add <name>".cyan());
        println!("  {} Set API key", "spn provider set <name>".cyan());
        println!();
    }

    Ok(())
}

async fn show_locations() -> Result<()> {
    let resolver = ConfigResolver::load()?;
    let scopes = resolver.get_scope_paths()?;

    println!("{}", "📁 Config File Locations".cyan().bold());
    println!();
    println!("   {}", "Precedence: Local > Team > Global".dimmed());
    println!();

    for scope in scopes {
        let status = if scope.exists {
            "✓".green()
        } else {
            "○".dimmed()
        };
        println!("   {} {}", status, scope.display_name());
    }

    println!();
    println!("   {} = exists, {} = not found", "✓".green(), "○".dimmed());

    Ok(())
}

async fn list_config(show_origin: bool) -> Result<()> {
    let resolver = ConfigResolver::load()?;
    let config = resolver.resolved();

    println!("{}", "📋 Configuration Values".cyan().bold());
    println!();

    // List providers
    if !config.providers.is_empty() {
        for (name, provider) in &config.providers {
            if let Some(model) = &provider.model {
                if show_origin {
                    // TODO: Implement proper origin tracking
                    println!("  providers.{}.model = {} (global)", name, model);
                } else {
                    println!("  providers.{}.model = {}", name, model);
                }
            }
        }
    }

    // List sync config
    if !config.sync.enabled_editors.is_empty() {
        println!("  sync.enabled_editors = {:?}", config.sync.enabled_editors);
    }
    if config.sync.auto_sync {
        println!("  sync.auto_sync = true");
    }

    // List servers
    if !config.servers.is_empty() {
        for name in config.servers.keys() {
            println!("  servers.{} = <configured>", name);
        }
    }

    if show_origin {
        println!();
        println!(
            "   {}",
            "Use 'spn config get <key> --show-origin' for detailed origin info".dimmed()
        );
    }

    Ok(())
}

async fn get_value(key: &str, show_origin: bool) -> Result<()> {
    let _resolver = ConfigResolver::load()?;

    // TODO: Implement key path resolution
    // For now, just show a message
    println!("{} Getting value for key: {}", "🔍".cyan(), key.bold());

    if show_origin {
        println!();
        println!("   {} Origin tracking not yet implemented", "⚠️".yellow());
        println!(
            "   {} This will show which scope defined this value",
            "→".dimmed()
        );
    }

    Ok(())
}

async fn set_value(key: &str, value: &str, scope: &str) -> Result<()> {
    let scope_type = ScopeType::from_str(scope).ok_or_else(|| {
        SpnError::ConfigError(format!(
            "Invalid scope: {}. Use: global, team, or local",
            scope
        ))
    })?;

    println!(
        "{} Setting {} = {} in {} scope",
        "✍️".cyan(),
        key.bold(),
        value,
        scope_type
    );

    // TODO: Implement key path resolution and setting
    println!();
    println!(
        "   {} Key path resolution not yet implemented",
        "⚠️".yellow()
    );
    println!("   {} Manual edit with: spn config edit", "→".dimmed());

    Ok(())
}

async fn edit_config(local_flag: bool, user: bool, mcp: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    let path = if local_flag {
        local::config_path(&cwd)
    } else if user {
        global::config_path()?
    } else if mcp {
        team::mcp_config_path(&cwd)
    } else {
        team::package_config_path(&cwd)
    };

    // Determine editor
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    if !path.exists() {
        println!("⚠️  File does not exist: {}", path.display());
        if local_flag {
            println!("   Creating local config...");
            local::save(&cwd, &Default::default())?;
            local::ensure_gitignored(&cwd)?;
        } else if user {
            println!("   Creating global config...");
            global::save(&Default::default())?;
        } else if mcp {
            println!("   Creating MCP config...");
            team::save_mcp(&cwd, &Default::default())?;
        } else {
            println!("   Run 'spn init' to create it.");
            return Ok(());
        }
    }

    println!("✏️  Opening {} with {}...", path.display(), editor);

    // Open editor
    std::process::Command::new(&editor).arg(&path).status()?;

    println!("   Config saved.");

    Ok(())
}

async fn import_config(file: &str, scope: &str, skip_confirm: bool) -> Result<()> {
    use dialoguer::Confirm;
    use rustc_hash::FxHashMap;
    use std::fs;
    use std::path::Path;

    let scope_type = ScopeType::from_str(scope).ok_or_else(|| {
        SpnError::ConfigError(format!(
            "Invalid scope: {}. Use: global, team, or local",
            scope
        ))
    })?;

    println!(
        "{} Importing configuration from {}",
        "📥".cyan(),
        file.bold()
    );
    println!("   Target scope: {}", scope_type);
    println!();

    // Check if file exists
    let path = Path::new(file);
    if !path.exists() {
        return Err(SpnError::ConfigError(format!("File not found: {}", file)));
    }

    // Read and parse file
    let content = fs::read_to_string(path)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to parse JSON: {}", e)))?;

    // Extract MCP servers
    let mcp_servers = if let Some(servers_obj) = parsed.get("mcpServers") {
        if let Some(obj) = servers_obj.as_object() {
            let mut servers = FxHashMap::default();
            for (name, config) in obj {
                if let Some(command) = config.get("command").and_then(|v| v.as_str()) {
                    let args = config
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let env = config
                        .get("env")
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            obj.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect()
                        })
                        .unwrap_or_default();

                    servers.insert(
                        name.clone(),
                        crate::config::types::McpServerConfig {
                            command: command.to_string(),
                            args,
                            env,
                            disabled: false,
                        },
                    );
                }
            }
            servers
        } else {
            FxHashMap::default()
        }
    } else {
        FxHashMap::default()
    };

    if mcp_servers.is_empty() {
        println!("{}", "⚠️  No MCP servers found in file".yellow());
        return Ok(());
    }

    // Show what will be imported
    println!("{}", "MCP Servers to import:".bold());
    for (name, server) in &mcp_servers {
        println!("  {} {} {}", "•".cyan(), name.bold(), server.command);
        if !server.args.is_empty() {
            println!("    args: {:?}", server.args);
        }
        if !server.env.is_empty() {
            println!("    env: {} variables", server.env.len());
        }
    }
    println!();

    // Ask for confirmation
    if !skip_confirm {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Import {} servers into {} scope?",
                mcp_servers.len(),
                scope_type
            ))
            .default(true)
            .interact()
            .unwrap_or(false);

        if !confirmed {
            println!("{}", "❌ Import cancelled".yellow());
            return Ok(());
        }
    }

    // Import based on scope
    let cwd = env::current_dir()?;
    match scope_type {
        ScopeType::Global => {
            let mut config = global::load()?;
            config.servers = mcp_servers;
            global::save(&config)?;
            println!(
                "{} Imported to {}",
                "✅".green(),
                global::config_path()?.display()
            );
        }
        ScopeType::Team => {
            team::save_mcp(&cwd, &mcp_servers)?;
            println!(
                "{} Imported to {}",
                "✅".green(),
                team::mcp_config_path(&cwd).display()
            );
        }
        ScopeType::Local => {
            let mut config = local::load(&cwd)?;
            config.servers = mcp_servers;
            local::save(&cwd, &config)?;
            local::ensure_gitignored(&cwd)?;
            println!(
                "{} Imported to {}",
                "✅".green(),
                local::config_path(&cwd).display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_show_locations_runs() {
        let result = show_locations().await;
        assert!(result.is_ok());
    }
}
