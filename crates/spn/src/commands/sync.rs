//! Sync command implementation.
//!
//! Syncs installed packages and MCP servers to IDE-specific configurations.
//!
//! # MCP Sync
//!
//! Uses `~/.spn/mcp.yaml` as the single source of truth for MCP servers.
//! Syncs to editor-specific configs (Claude Code, Cursor, Windsurf).

use std::path::PathBuf;

use crate::ux::design_system as ds;

use crate::diff::DiffBatch;
use crate::error::{Result, SpnError};
use crate::storage::LocalStorage;
use crate::sync::adapters::{detect_ides, get_adapter};
use crate::sync::config::SyncConfig;
use crate::sync::mcp_sync::sync_mcp_to_editors;
use crate::sync::types::{IdeTarget, PackageManifest, SyncedItem};

/// Run the sync command.
pub async fn run(
    enable: Option<String>,
    disable: Option<String>,
    status: bool,
    target: Option<String>,
    dry_run: bool,
    interactive: bool,
) -> Result<()> {
    let mut config = SyncConfig::load().unwrap_or_default();

    // Handle enable/disable
    if let Some(editor) = enable {
        return enable_target(&mut config, &editor);
    }

    if let Some(editor) = disable {
        return disable_target(&mut config, &editor);
    }

    // Handle status
    if status {
        return show_status(&config);
    }

    // Interactive mode: collect changes, show diffs, confirm
    if interactive && !dry_run {
        return run_interactive_sync(&config, target.as_deref()).await;
    }

    // Main sync operation
    run_sync(&config, target.as_deref(), dry_run).await
}

/// Enable sync for an IDE target.
fn enable_target(config: &mut SyncConfig, editor: &str) -> Result<()> {
    let target = IdeTarget::from_str(editor).ok_or_else(|| {
        SpnError::ConfigError(format!(
            "Unknown editor: {}. Supported: claude-code, cursor, vscode, windsurf",
            editor
        ))
    })?;

    config.enable(target);
    config
        .save()
        .map_err(|e| SpnError::ConfigError(e.to_string()))?;

    println!("✅ Enabled sync for: {}", target.display_name());
    println!("   Config saved to: ~/.spn/sync.json");
    Ok(())
}

/// Disable sync for an IDE target.
fn disable_target(config: &mut SyncConfig, editor: &str) -> Result<()> {
    let target = IdeTarget::from_str(editor).ok_or_else(|| {
        SpnError::ConfigError(format!(
            "Unknown editor: {}. Supported: claude-code, cursor, vscode, windsurf",
            editor
        ))
    })?;

    config.disable(target);
    config
        .save()
        .map_err(|e| SpnError::ConfigError(e.to_string()))?;

    println!("❌ Disabled sync for: {}", target.display_name());
    Ok(())
}

/// Show sync status.
fn show_status(config: &SyncConfig) -> Result<()> {
    println!("📊 Sync Status");
    println!();

    // Show enabled targets
    println!("Enabled targets:");
    if config.enabled_targets.is_empty() {
        println!("  (none)");
    } else {
        for target in &config.enabled_targets {
            println!("  ✅ {}", target.display_name());
        }
    }

    // Show available IDEs in current directory
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let detected = detect_ides(&cwd);

    println!();
    println!("Detected IDEs in current directory:");
    if detected.is_empty() {
        println!("  (none)");
    } else {
        for target in detected {
            let adapter = get_adapter(target);
            let config_path = adapter.config_path(&cwd);
            let status = if config_path.exists() { "✓" } else { "○" };
            println!(
                "  {} {} ({})",
                status,
                target.display_name(),
                config_path.display()
            );
        }
    }

    // Show last sync time
    if let Some(last_sync) = &config.last_sync {
        println!();
        println!("Last sync: {}", last_sync);
    }

    Ok(())
}

/// Run the main sync operation.
async fn run_sync(config: &SyncConfig, target_filter: Option<&str>, dry_run: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| SpnError::ConfigError(e.to_string()))?;

    // Determine which targets to sync
    let targets: Vec<IdeTarget> = if let Some(filter) = target_filter {
        // Specific target requested
        let target = IdeTarget::from_str(filter).ok_or_else(|| {
            SpnError::ConfigError(format!(
                "Unknown target: {}. Supported: claude-code, cursor, vscode, windsurf",
                filter
            ))
        })?;
        vec![target]
    } else if config.enabled_targets.is_empty() {
        // Auto-detect available IDEs
        detect_ides(&cwd)
    } else {
        // Use enabled targets
        config.enabled_targets.iter().copied().collect()
    };

    if targets.is_empty() {
        println!(
            "{}",
            ds::warning("⚠️  No IDE configurations found in current directory.")
        );
        println!("   Create .claude/, .cursor/, .vscode/, or .windsurf/ to enable sync.");
        return Ok(());
    }

    if dry_run {
        println!(
            "{}",
            ds::primary("🔍 Dry run - showing what would be synced:")
        );
        println!();
    } else {
        println!("{}", ds::primary("🔄 Syncing to IDE configurations..."));
        println!();
    }

    // PHASE 1: Sync MCP servers from ~/.spn/mcp.yaml
    println!("{}", ds::primary("MCP Servers"));
    let mcp_results = sync_mcp_to_editors(&targets, Some(&cwd));
    let mut mcp_synced = 0;
    let mut mcp_errors = Vec::new();

    for result in &mcp_results {
        if result.success && result.servers_synced > 0 {
            println!(
                "  {} {} → {} servers",
                ds::success("✓"),
                ds::highlight(result.target.display_name()),
                result.servers_synced
            );
            if !dry_run {
                println!(
                    "    {}",
                    ds::muted(format!("({})", result.config_path.display()))
                );
            }
            mcp_synced += result.servers_synced;
        } else if let Some(err) = &result.error {
            println!(
                "  {} {}: {}",
                ds::error("✗"),
                result.target.display_name(),
                err
            );
            mcp_errors.push(err.clone());
        }
    }

    if mcp_synced == 0 && mcp_errors.is_empty() {
        println!("  {} No MCP servers configured", ds::muted("→"));
        println!("    Add with: {}", ds::primary("spn mcp add <name>"));
    }

    // PHASE 2: Sync packages (skills, hooks)
    let storage = LocalStorage::new()?;
    let packages = storage.list_packages()?;

    let mut total_synced = 0;
    let mut errors = Vec::new();

    if !packages.is_empty() {
        println!();
        println!("{}", ds::primary("Packages"));

        for target in &targets {
            let adapter = get_adapter(*target);

            if !adapter.is_available(&cwd) {
                continue;
            }

            for (name, path) in &packages {
                // Try to load package manifest
                let manifest_path = path.join("spn.json");
                let mut manifest = if manifest_path.exists() {
                    PackageManifest::from_file(&manifest_path).unwrap_or_default()
                } else {
                    PackageManifest {
                        name: name.clone(),
                        version: "0.0.0".to_string(),
                        ..Default::default()
                    }
                };

                // Set name if not in manifest
                if manifest.name.is_empty() {
                    manifest.name = name.clone();
                }

                // Check if this package type requires sync
                if !manifest.requires_sync() {
                    let package_type = manifest.package_type();
                    if dry_run {
                        println!(
                            "  {} Skipping {} (type: {:?}, no sync required)",
                            ds::muted("→"),
                            name,
                            package_type
                        );
                    }
                    continue;
                }

                // Skip packages without skills/hooks (MCP is handled separately)
                if manifest.skills.is_empty() && manifest.hooks.is_empty() {
                    continue;
                }

                if dry_run {
                    if !manifest.skills.is_empty() {
                        println!(
                            "  Would link {} skills from {}",
                            manifest.skills.len(),
                            name
                        );
                    }
                    if !manifest.hooks.is_empty() {
                        println!("  Would link {} hooks from {}", manifest.hooks.len(), name);
                    }
                } else {
                    let result = adapter.sync_package(&cwd, name, path, &manifest);

                    if result.success {
                        for item in &result.synced {
                            match item {
                                SyncedItem::McpServer(_) => {
                                    // MCP handled in phase 1
                                }
                                SyncedItem::Skills(path) => {
                                    println!("  {} Skills: {}", ds::success("✓"), path.display());
                                    total_synced += 1;
                                }
                                SyncedItem::Hooks(path) => {
                                    println!("  {} Hooks: {}", ds::success("✓"), path.display());
                                    total_synced += 1;
                                }
                                SyncedItem::Command(name) => {
                                    println!("  {} Command: {}", ds::success("✓"), name);
                                    total_synced += 1;
                                }
                            }
                        }
                    } else if let Some(err) = result.error {
                        println!("  {} {}: {}", ds::error("✗"), name, err);
                        errors.push((name.clone(), err));
                    }
                }
            }
        }
    }

    // Summary
    println!();
    if dry_run {
        println!("Run without {} to apply changes.", ds::primary("--dry-run"));
    } else if errors.is_empty() && mcp_errors.is_empty() {
        let total = mcp_synced + total_synced;
        if total > 0 {
            println!("{} Synced {} items successfully.", ds::success("✓"), total);
        } else {
            println!("{}", ds::muted("Nothing to sync."));
        }

        // Update last sync time
        let mut config = SyncConfig::load().unwrap_or_default();
        config.last_sync = Some(chrono::Utc::now().to_rfc3339());
        let _ = config.save();
    } else {
        println!(
            "{} Synced with {} errors.",
            ds::warning("⚠"),
            errors.len() + mcp_errors.len()
        );
    }

    Ok(())
}

/// Run sync with interactive diff preview and confirmation.
async fn run_interactive_sync(config: &SyncConfig, target_filter: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| SpnError::ConfigError(e.to_string()))?;

    println!("{}", ds::primary("🔍 Interactive Sync Mode"));
    println!("{}", ds::muted("Analyzing changes..."));
    println!();

    // Determine targets
    let targets: Vec<IdeTarget> = if let Some(filter) = target_filter {
        let target = IdeTarget::from_str(filter)
            .ok_or_else(|| SpnError::ConfigError(format!("Unknown target: {}", filter)))?;
        vec![target]
    } else if config.enabled_targets.is_empty() {
        detect_ides(&cwd)
    } else {
        config.enabled_targets.iter().copied().collect()
    };

    if targets.is_empty() {
        println!(
            "{}",
            ds::warning("⚠️  No IDE configurations found in current directory.")
        );
        return Ok(());
    }

    // Collect all config files that would be modified
    let mut diff_batch = DiffBatch::new();

    // Check MCP config files
    let mcp_manager = crate::mcp::config_manager();
    if let Ok(mcp_config) = mcp_manager.load_resolved() {
        for target in &targets {
            let config_path = match target {
                IdeTarget::ClaudeCode => cwd.join(".claude").join("settings.json"),
                IdeTarget::Cursor => cwd.join(".cursor").join("mcp.json"),
                IdeTarget::Windsurf => cwd.join(".windsurf").join("mcp.json"),
                IdeTarget::VsCode => continue,
            };

            if let Some(parent) = config_path.parent() {
                if !parent.exists() {
                    continue;
                }
            }

            // Read existing content
            let old_content = if config_path.exists() {
                std::fs::read_to_string(&config_path).unwrap_or_default()
            } else {
                String::new()
            };

            // Generate what new content would be
            let new_content = generate_mcp_config_preview(target, &mcp_config);

            // Only add to diff if content changed
            if old_content != new_content {
                diff_batch.add(config_path.display().to_string(), old_content, new_content);
            }
        }
    }

    // If no changes, just inform the user
    if diff_batch.is_empty() {
        println!("{}", ds::success("✓ No changes detected"));
        println!("   All configurations are already up to date.");
        return Ok(());
    }

    // Show diffs and ask for confirmation
    if !diff_batch.confirm() {
        println!("{}", ds::warning("❌ Sync cancelled"));
        return Ok(());
    }

    println!();
    println!("{}", ds::success("✅ Confirmed, applying changes..."));
    println!();

    // Now run the actual sync
    run_sync(config, target_filter, false).await
}

/// Generate MCP config preview for a target.
fn generate_mcp_config_preview(target: &IdeTarget, mcp_config: &crate::mcp::McpConfig) -> String {
    use serde_json::json;

    let mcp_servers: serde_json::Map<String, serde_json::Value> = mcp_config
        .servers
        .iter()
        .map(|(name, server)| {
            let mut server_json = json!({
                "command": server.command,
                "args": server.args,
            });

            if !server.env.is_empty() {
                server_json["env"] = json!(server.env);
            }

            (name.clone(), server_json)
        })
        .collect();

    match target {
        IdeTarget::ClaudeCode => {
            // Claude Code uses settings.json with mcpServers key
            let settings = json!({
                "mcpServers": mcp_servers
            });
            serde_json::to_string_pretty(&settings).unwrap_or_default()
        }
        _ => {
            // Cursor and Windsurf use direct JSON format
            let config = json!({
                "mcpServers": mcp_servers
            });
            serde_json::to_string_pretty(&config).unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ide_target_parsing() {
        assert_eq!(
            IdeTarget::from_str("claude-code"),
            Some(IdeTarget::ClaudeCode)
        );
        assert_eq!(IdeTarget::from_str("cc"), Some(IdeTarget::ClaudeCode));
        assert_eq!(IdeTarget::from_str("cursor"), Some(IdeTarget::Cursor));
        assert_eq!(IdeTarget::from_str("unknown"), None);
    }

    #[test]
    fn test_enable_disable_target() {
        let temp = TempDir::new().unwrap();
        let _config_path = temp.path().join("sync.json");

        let mut config = SyncConfig::default();
        config.enable(IdeTarget::ClaudeCode);
        assert!(config.is_enabled(IdeTarget::ClaudeCode));

        config.disable(IdeTarget::ClaudeCode);
        assert!(!config.is_enabled(IdeTarget::ClaudeCode));
    }
}
