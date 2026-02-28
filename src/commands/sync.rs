//! Sync command implementation.
//!
//! Syncs installed packages to IDE-specific configurations.

use std::path::PathBuf;

use crate::error::{Result, SpnError};
use crate::storage::LocalStorage;
use crate::sync::adapters::{detect_ides, get_adapter};
use crate::sync::config::SyncConfig;
use crate::sync::types::{IdeTarget, PackageManifest, SyncedItem};

/// Run the sync command.
pub async fn run(
    enable: Option<String>,
    disable: Option<String>,
    status: bool,
    target: Option<String>,
    dry_run: bool,
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
        println!("⚠️  No IDE configurations found in current directory.");
        println!("   Create .claude/, .cursor/, .vscode/, or .windsurf/ to enable sync.");
        return Ok(());
    }

    // Get installed packages
    let storage = LocalStorage::new()?;
    let packages = storage.list_packages()?;

    if packages.is_empty() {
        println!("⚠️  No packages installed. Run `spn install` first.");
        return Ok(());
    }

    if dry_run {
        println!("🔍 Dry run - showing what would be synced:");
        println!();
    } else {
        println!("🔄 Syncing packages to IDE configurations...");
        println!();
    }

    let mut total_synced = 0;
    let mut errors = Vec::new();

    for target in targets {
        let adapter = get_adapter(target);

        if !adapter.is_available(&cwd) {
            continue;
        }

        println!("📁 {}", target.display_name());

        for (name, path) in &packages {
            // Try to load package manifest
            let manifest_path = path.join("spn.json");
            let manifest = if manifest_path.exists() {
                PackageManifest::from_file(&manifest_path).unwrap_or_default()
            } else {
                PackageManifest {
                    name: name.clone(),
                    version: "0.0.0".to_string(),
                    ..Default::default()
                }
            };

            if !manifest.has_integrations() {
                continue;
            }

            if dry_run {
                // Show what would be synced
                if let Some(mcp) = &manifest.mcp {
                    println!("   Would add MCP server: {} ({})", name, mcp.command);
                }
                if !manifest.skills.is_empty() {
                    println!("   Would link {} skills", manifest.skills.len());
                }
                if !manifest.hooks.is_empty() {
                    println!("   Would link {} hooks", manifest.hooks.len());
                }
            } else {
                // Actually sync
                let result = adapter.sync_package(&cwd, name, path, &manifest);

                if result.success {
                    for item in &result.synced {
                        match item {
                            SyncedItem::McpServer(name) => {
                                println!("   ✅ MCP: {}", name);
                            }
                            SyncedItem::Skills(path) => {
                                println!("   ✅ Skills: {}", path.display());
                            }
                            SyncedItem::Hooks(path) => {
                                println!("   ✅ Hooks: {}", path.display());
                            }
                            SyncedItem::Command(name) => {
                                println!("   ✅ Command: {}", name);
                            }
                        }
                        total_synced += 1;
                    }
                } else if let Some(err) = result.error {
                    println!("   ❌ {}: {}", name, err);
                    errors.push((name.clone(), err));
                }
            }
        }
    }

    println!();
    if dry_run {
        println!("Run without --dry-run to apply changes.");
    } else if errors.is_empty() {
        println!("✅ Synced {} items successfully.", total_synced);

        // Update last sync time
        let mut config = SyncConfig::load().unwrap_or_default();
        config.last_sync = Some(chrono::Utc::now().to_rfc3339());
        let _ = config.save();
    } else {
        println!(
            "⚠️  Synced {} items with {} errors.",
            total_synced,
            errors.len()
        );
    }

    Ok(())
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
        let config_path = temp.path().join("sync.json");

        let mut config = SyncConfig::default();
        config.enable(IdeTarget::ClaudeCode);
        assert!(config.is_enabled(IdeTarget::ClaudeCode));

        config.disable(IdeTarget::ClaudeCode);
        assert!(!config.is_enabled(IdeTarget::ClaudeCode));
    }
}
