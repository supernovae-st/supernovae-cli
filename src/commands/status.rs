//! Status command implementation.
//!
//! Shows the current state of the SuperNovae ecosystem:
//! - MCP servers (from ~/.spn/mcp.yaml)
//! - Installed packages
//! - Skills
//! - Detected editors

use std::path::PathBuf;

use colored::Colorize;

use crate::error::Result;
use crate::mcp::config_manager;
use crate::storage::LocalStorage;
use crate::sync::adapters::detect_ides;
use crate::sync::config::SyncConfig;
use crate::interop::skills::SkillsClient;

/// Run the status command.
pub async fn run(json: bool) -> Result<()> {
    if json {
        run_json().await
    } else {
        run_display().await
    }
}

/// Display status in human-readable format.
async fn run_display() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    println!("{}", "SuperNovae Status".cyan().bold());
    println!();

    // Section 1: MCP Servers
    println!("{}", "MCP Servers".cyan().bold());
    let mcp = config_manager();
    let servers = mcp.list_all_servers()?;

    if servers.is_empty() {
        println!("  {} No servers configured", "→".dimmed());
        println!("    Add with: {}", "spn mcp add <name>".cyan());
    } else {
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
        }
    }

    // Section 2: Packages
    println!();
    println!("{}", "Packages".cyan().bold());
    let storage = LocalStorage::new()?;
    let packages = storage.list_packages()?;

    if packages.is_empty() {
        println!("  {} No packages installed", "→".dimmed());
        println!("    Install with: {}", "spn add <package>".cyan());
    } else {
        for (name, path) in &packages {
            let version = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?");
            println!("  {} {} {}", "✓".green(), name.bold(), version.dimmed());
        }
    }

    // Section 3: Skills
    println!();
    println!("{}", "Skills".cyan().bold());
    let skills_client = SkillsClient::new();
    let skills = skills_client.list_installed().unwrap_or_default();

    if skills.is_empty() {
        println!("  {} No skills installed", "→".dimmed());
        println!("    Add with: {}", "spn skill add <name>".cyan());
    } else {
        for skill in &skills {
            println!("  {} {}", "✓".green(), skill.bold());
        }
    }

    // Section 4: Editors
    println!();
    println!("{}", "Editors".cyan().bold());
    let detected = detect_ides(&cwd);
    let sync_config = SyncConfig::load().unwrap_or_default();

    if detected.is_empty() {
        println!("  {} No editors detected in current directory", "→".dimmed());
    } else {
        for target in detected {
            let enabled = if sync_config.is_enabled(target) {
                "✓".green()
            } else {
                "○".dimmed()
            };
            println!("  {} {} {}", enabled, target.display_name().bold(), target.config_dir().dimmed());
        }
    }

    // Section 5: Config locations
    println!();
    println!("{}", "Config Files".cyan().bold());
    let global_mcp = crate::mcp::McpConfigManager::default_global_path();
    let exists_badge = |exists: bool| if exists { "✓".green() } else { "○".dimmed() };

    println!(
        "  {} {} {}",
        exists_badge(global_mcp.exists()),
        "MCP (global)".bold(),
        global_mcp.display().to_string().dimmed()
    );

    let project_mcp = cwd.join(".spn").join("mcp.yaml");
    if project_mcp.exists() {
        println!(
            "  {} {} {}",
            "✓".green(),
            "MCP (project)".bold(),
            project_mcp.display().to_string().dimmed()
        );
    }

    let spn_yaml = cwd.join("spn.yaml");
    if spn_yaml.exists() {
        println!(
            "  {} {} {}",
            "✓".green(),
            "spn.yaml".bold(),
            spn_yaml.display().to_string().dimmed()
        );
    }

    // Summary line
    println!();
    let total = servers.len() + packages.len() + skills.len();
    if total == 0 {
        println!("{}", "Get started: spn mcp add neo4j".dimmed());
    } else {
        println!(
            "{} {} MCP servers, {} packages, {} skills",
            "Total:".dimmed(),
            servers.len(),
            packages.len(),
            skills.len()
        );
    }

    Ok(())
}

/// Output status as JSON.
async fn run_json() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mcp = config_manager();
    let servers = mcp.list_all_servers()?;

    let storage = LocalStorage::new()?;
    let packages = storage.list_packages()?;

    let skills_client = SkillsClient::new();
    let skills = skills_client.list_installed().unwrap_or_default();

    let detected = detect_ides(&cwd);
    let sync_config = SyncConfig::load().unwrap_or_default();

    let json = serde_json::json!({
        "mcp_servers": servers.iter().map(|(name, server)| {
            serde_json::json!({
                "name": name,
                "command": server.command,
                "args": server.args,
                "enabled": server.enabled,
                "source": server.source.map(|s| format!("{:?}", s).to_lowercase()),
            })
        }).collect::<Vec<_>>(),
        "packages": packages.iter().map(|(name, path)| {
            serde_json::json!({
                "name": name,
                "path": path.display().to_string(),
            })
        }).collect::<Vec<_>>(),
        "skills": skills,
        "editors": detected.iter().map(|target| {
            serde_json::json!({
                "name": target.display_name(),
                "config_dir": target.config_dir(),
                "sync_enabled": sync_config.is_enabled(*target),
            })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_status_empty() {
        // Just verify it doesn't panic with empty state
        // Full test would need mocked storage
    }
}
