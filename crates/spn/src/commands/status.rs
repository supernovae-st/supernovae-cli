//! Status command implementation.
//!
//! Shows the current state of the SuperNovae ecosystem:
//! - MCP servers (from ~/.spn/mcp.yaml)
//! - Installed packages
//! - Skills
//! - Detected editors

use std::path::PathBuf;

use crate::ux::design_system as ds;

use crate::error::Result;
use crate::interop::skills::SkillsClient;
use crate::mcp::config_manager;
use crate::storage::LocalStorage;
use crate::sync::adapters::detect_ides;
use crate::sync::config::SyncConfig;

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

    println!("{}", ds::primary("SuperNovae Status"));
    println!();

    // Section 1: MCP Servers
    println!("{}", ds::primary("MCP Servers"));
    let mcp = config_manager();
    let servers = mcp.list_all_servers()?;

    if servers.is_empty() {
        println!("  {} No servers configured", ds::muted("→"));
        println!("    Add with: {}", ds::primary("spn mcp add <name>"));
    } else {
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
        }
    }

    // Section 2: Packages
    println!();
    println!("{}", ds::primary("Packages"));
    let storage = LocalStorage::new()?;
    let packages = storage.list_packages()?;

    if packages.is_empty() {
        println!("  {} No packages installed", ds::muted("→"));
        println!("    Install with: {}", ds::primary("spn add <package>"));
    } else {
        for (name, path) in &packages {
            let version = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            println!(
                "  {} {} {}",
                ds::success("✓"),
                ds::highlight(name),
                ds::muted(version)
            );
        }
    }

    // Section 3: Skills
    println!();
    println!("{}", ds::primary("Skills"));
    let skills_client = SkillsClient::new();
    let skills = skills_client.list_installed().unwrap_or_default();

    if skills.is_empty() {
        println!("  {} No skills installed", ds::muted("→"));
        println!("    Add with: {}", ds::primary("spn skill add <name>"));
    } else {
        for skill in &skills {
            println!("  {} {}", ds::success("✓"), ds::highlight(skill));
        }
    }

    // Section 4: Editors
    println!();
    println!("{}", ds::primary("Editors"));
    let detected = detect_ides(&cwd);
    let sync_config = SyncConfig::load().unwrap_or_default();

    if detected.is_empty() {
        println!(
            "  {} No editors detected in current directory",
            ds::muted("→")
        );
    } else {
        for target in detected {
            let enabled = if sync_config.is_enabled(target) {
                ds::success("✓")
            } else {
                ds::muted("○")
            };
            println!(
                "  {} {} {}",
                enabled,
                ds::highlight(target.display_name()),
                ds::muted(target.config_dir())
            );
        }
    }

    // Section 5: Config locations
    println!();
    println!("{}", ds::primary("Config Files"));
    let global_mcp = crate::mcp::McpConfigManager::default_global_path();
    let exists_badge = |exists: bool| {
        if exists {
            ds::success("✓")
        } else {
            ds::muted("○")
        }
    };

    println!(
        "  {} {} {}",
        exists_badge(global_mcp.exists()),
        ds::highlight("MCP (global)"),
        ds::muted(global_mcp.display().to_string())
    );

    let project_mcp = cwd.join(".spn").join("mcp.yaml");
    if project_mcp.exists() {
        println!(
            "  {} {} {}",
            ds::success("✓"),
            ds::highlight("MCP (project)"),
            ds::muted(project_mcp.display().to_string())
        );
    }

    let spn_yaml = cwd.join("spn.yaml");
    if spn_yaml.exists() {
        println!(
            "  {} {} {}",
            ds::success("✓"),
            ds::highlight("spn.yaml"),
            ds::muted(spn_yaml.display().to_string())
        );
    }

    // Summary line
    println!();
    let total = servers.len() + packages.len() + skills.len();
    if total == 0 {
        println!("{}", ds::muted("Get started: spn mcp add neo4j"));
    } else {
        println!(
            "{} {} MCP servers, {} packages, {} skills",
            ds::muted("Total:"),
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
