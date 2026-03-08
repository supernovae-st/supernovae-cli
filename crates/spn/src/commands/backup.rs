//! Unified backup system for SuperNovae ecosystem.
//!
//! Orchestrates backups across NovaNet, Nika, and spn subsystems.

use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use console::style;

use spn_core::backup::{
    AdapterContents, BackupAdapter, BackupError, BackupInfo, BackupManifest, NikaContents,
    NovaNetContents, RestoreInfo, SpnContents,
};

use crate::error::{Result, SpnError};

// ============================================================================
// CLI SUBCOMMANDS
// ============================================================================

#[derive(Subcommand, Clone, Debug)]
pub enum BackupCommands {
    /// Create a new backup of all SuperNovae data
    Create {
        /// Optional label for the backup (e.g., "pre-refacto")
        #[arg(short, long)]
        label: Option<String>,
    },

    /// Restore from a backup archive
    Restore {
        /// Path to backup file, or "latest" for most recent
        #[arg(default_value = "latest")]
        backup: String,

        /// Force restore without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List available backups
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,

        /// Maximum number of backups to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Delete old backups
    Prune {
        /// Keep only the N most recent backups
        #[arg(short, long, default_value = "5")]
        keep: usize,

        /// Actually delete (dry-run by default)
        #[arg(long)]
        execute: bool,
    },
}

// ============================================================================
// BACKUP MANAGER
// ============================================================================

/// Backup manager orchestrating all backup operations.
pub struct BackupManager {
    /// Directory where backups are stored.
    backup_dir: PathBuf,
    /// Registered adapters for each subsystem.
    adapters: Vec<Box<dyn BackupAdapter>>,
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BackupManager {
    /// Create a new backup manager with default backup directory.
    pub fn new() -> Self {
        let backup_dir = dirs::home_dir()
            .expect("home directory should exist")
            .join(".spn")
            .join("backups");

        Self {
            backup_dir,
            adapters: Vec::new(),
        }
    }

    /// Register a backup adapter for a subsystem.
    pub fn register_adapter(&mut self, adapter: Box<dyn BackupAdapter>) {
        self.adapters.push(adapter);
    }

    /// Create a new backup of all registered systems.
    pub fn create_backup(&self, label: Option<&str>) -> Result<BackupInfo> {
        // Ensure backup directory exists
        fs::create_dir_all(&self.backup_dir)?;

        // Create manifest
        let mut manifest = BackupManifest::new(label.map(String::from));
        manifest.set_hostname(gethostname::gethostname().to_string_lossy().to_string());

        // Generate backup filename
        let timestamp = &manifest.created_at;
        let safe_timestamp = timestamp.replace(':', "-");
        let name = if let Some(l) = label {
            format!("backup-{}-{}.tar.gz", l, safe_timestamp)
        } else {
            format!("backup-{}.tar.gz", safe_timestamp)
        };
        let backup_path = self.backup_dir.join(&name);

        // Create staging directory
        let staging_dir = tempfile::tempdir()?;

        // Collect data from each adapter
        for adapter in &self.adapters {
            if !adapter.is_available() {
                continue;
            }

            // Update versions in manifest
            match adapter.name() {
                "novanet" => manifest.versions.novanet = adapter.version(),
                "nika" => manifest.versions.nika = adapter.version(),
                _ => {}
            }

            // Collect data
            let contents = adapter.collect(staging_dir.path())?;

            // Update manifest contents
            match contents {
                AdapterContents::NovaNet(c) => manifest.contents.novanet = c,
                AdapterContents::Nika(c) => manifest.contents.nika = c,
                AdapterContents::Spn(c) => manifest.contents.spn = c,
            }
        }

        // Write manifest to staging directory
        let manifest_path = staging_dir.path().join("manifest.json");
        let manifest_json = manifest.to_json();
        fs::write(&manifest_path, &manifest_json)?;

        // Create tar.gz archive
        create_archive(staging_dir.path(), &backup_path)?;

        // Get final size
        let size_bytes = fs::metadata(&backup_path)?.len();

        Ok(BackupInfo {
            path: backup_path,
            timestamp: manifest.created_at.clone(),
            size_bytes,
            manifest,
        })
    }

    /// Restore from a backup archive.
    pub fn restore_backup(&self, backup_path: &Path) -> Result<RestoreInfo> {
        // Validate backup exists
        if !backup_path.exists() {
            return Err(SpnError::NotFound(format!(
                "Backup not found: {}",
                backup_path.display()
            )));
        }

        // Create staging directory
        let staging_dir = tempfile::tempdir()?;

        // Extract archive
        extract_archive(backup_path, staging_dir.path())?;

        // Read manifest
        let manifest_path = staging_dir.path().join("manifest.json");
        let manifest_content = fs::read_to_string(&manifest_path)
            .map_err(|_| SpnError::InvalidInput("Missing manifest.json in backup".to_string()))?;
        let manifest = BackupManifest::from_json(&manifest_content)
            .map_err(|e| SpnError::InvalidInput(format!("Invalid manifest: {}", e)))?;

        // Restore each subsystem
        for adapter in &self.adapters {
            if !adapter.is_available() {
                continue;
            }
            adapter.restore(staging_dir.path())?;
        }

        // Get current timestamp for restore info
        let restored_at = BackupManifest::new(None).created_at;

        Ok(RestoreInfo {
            backup_path: backup_path.to_path_buf(),
            restored_at,
            manifest,
        })
    }

    /// List all available backups, sorted by timestamp (newest first).
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Check if it's a .tar.gz file
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".tar.gz") {
                    if let Ok(info) = self.read_backup_info(&path) {
                        backups.push(info);
                    }
                }
            }
        }

        // Sort by timestamp, newest first
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(backups)
    }

    /// Delete a backup file.
    pub fn delete_backup(&self, backup_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Err(SpnError::NotFound(format!(
                "Backup not found: {}",
                backup_path.display()
            )));
        }
        fs::remove_file(backup_path)?;
        Ok(())
    }

    /// Read backup info from an archive.
    fn read_backup_info(&self, backup_path: &Path) -> Result<BackupInfo> {
        // Create temp dir and extract
        let staging_dir = tempfile::tempdir()?;
        extract_archive(backup_path, staging_dir.path())?;

        // Read manifest
        let manifest_path = staging_dir.path().join("manifest.json");
        let manifest_content = fs::read_to_string(&manifest_path)
            .map_err(|_| SpnError::InvalidInput("Missing manifest.json".to_string()))?;
        let manifest = BackupManifest::from_json(&manifest_content)
            .map_err(|e| SpnError::InvalidInput(format!("Invalid manifest: {}", e)))?;

        let size_bytes = fs::metadata(backup_path)?.len();

        Ok(BackupInfo {
            path: backup_path.to_path_buf(),
            timestamp: manifest.created_at.clone(),
            size_bytes,
            manifest,
        })
    }
}

// ============================================================================
// ADAPTERS
// ============================================================================

/// spn adapter - backs up ~/.spn/ configuration.
pub struct SpnAdapter {
    spn_dir: PathBuf,
}

impl Default for SpnAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SpnAdapter {
    pub fn new() -> Self {
        Self {
            spn_dir: dirs::home_dir().expect("home dir").join(".spn"),
        }
    }
}

impl BackupAdapter for SpnAdapter {
    fn name(&self) -> &str {
        "spn"
    }

    fn is_available(&self) -> bool {
        self.spn_dir.exists()
    }

    fn version(&self) -> Option<String> {
        Some(env!("CARGO_PKG_VERSION").to_string())
    }

    fn collect(&self, staging_dir: &Path) -> std::result::Result<AdapterContents, BackupError> {
        let spn_staging = staging_dir.join("spn");
        fs::create_dir_all(&spn_staging)?;

        // Copy config.toml (sanitize secrets)
        let config_path = self.spn_dir.join("config.toml");
        let has_config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let sanitized = sanitize_config(&content);
            fs::write(spn_staging.join("config.toml"), sanitized)?;
            true
        } else {
            false
        };

        // Copy mcp.yaml
        let mcp_path = self.spn_dir.join("mcp.yaml");
        let has_mcp_yaml = if mcp_path.exists() {
            fs::copy(&mcp_path, spn_staging.join("mcp.yaml"))?;
            true
        } else {
            false
        };

        // Copy jobs.json
        let jobs_path = self.spn_dir.join("jobs.json");
        let has_jobs = if jobs_path.exists() {
            fs::copy(&jobs_path, spn_staging.join("jobs.json"))?;
            true
        } else {
            false
        };

        Ok(AdapterContents::Spn(SpnContents {
            has_config,
            has_mcp_yaml,
            has_jobs,
        }))
    }

    fn restore(&self, staging_dir: &Path) -> std::result::Result<(), BackupError> {
        let spn_staging = staging_dir.join("spn");

        if !spn_staging.exists() {
            return Ok(());
        }

        fs::create_dir_all(&self.spn_dir)?;

        // Restore config.toml
        let config_src = spn_staging.join("config.toml");
        if config_src.exists() {
            fs::copy(&config_src, self.spn_dir.join("config.toml"))?;
        }

        // Restore mcp.yaml
        let mcp_src = spn_staging.join("mcp.yaml");
        if mcp_src.exists() {
            fs::copy(&mcp_src, self.spn_dir.join("mcp.yaml"))?;
        }

        // Restore jobs.json
        let jobs_src = spn_staging.join("jobs.json");
        if jobs_src.exists() {
            fs::copy(&jobs_src, self.spn_dir.join("jobs.json"))?;
        }

        Ok(())
    }
}

/// Nika adapter - backs up ~/.nika/ directory.
pub struct NikaAdapter {
    nika_dir: PathBuf,
}

impl Default for NikaAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NikaAdapter {
    pub fn new() -> Self {
        Self {
            nika_dir: dirs::home_dir().expect("home dir").join(".nika"),
        }
    }
}

impl BackupAdapter for NikaAdapter {
    fn name(&self) -> &str {
        "nika"
    }

    fn is_available(&self) -> bool {
        self.nika_dir.exists()
    }

    fn version(&self) -> Option<String> {
        // Try to read from nika binary version
        None
    }

    fn collect(&self, staging_dir: &Path) -> std::result::Result<AdapterContents, BackupError> {
        let nika_staging = staging_dir.join("nika");
        fs::create_dir_all(&nika_staging)?;

        let mut session_count = 0;
        let mut trace_count = 0;
        let mut workflow_files = 0;

        // Copy sessions/
        let sessions_src = self.nika_dir.join("sessions");
        if sessions_src.exists() {
            let sessions_dst = nika_staging.join("sessions");
            copy_dir_recursive(&sessions_src, &sessions_dst)?;
            session_count = count_files(&sessions_dst);
        }

        // Copy traces/
        let traces_src = self.nika_dir.join("traces");
        if traces_src.exists() {
            let traces_dst = nika_staging.join("traces");
            copy_dir_recursive(&traces_src, &traces_dst)?;
            trace_count = count_files(&traces_dst);
        }

        // Copy workflows/ (*.nika.yaml files)
        let workflows_src = self.nika_dir.join("workflows");
        if workflows_src.exists() {
            let workflows_dst = nika_staging.join("workflows");
            copy_dir_recursive(&workflows_src, &workflows_dst)?;
            workflow_files = count_yaml_files(&workflows_dst);
        }

        Ok(AdapterContents::Nika(NikaContents {
            workflow_files,
            session_count,
            trace_count,
        }))
    }

    fn restore(&self, staging_dir: &Path) -> std::result::Result<(), BackupError> {
        let nika_staging = staging_dir.join("nika");

        if !nika_staging.exists() {
            return Ok(());
        }

        fs::create_dir_all(&self.nika_dir)?;

        // Restore sessions/
        let sessions_src = nika_staging.join("sessions");
        if sessions_src.exists() {
            copy_dir_recursive(&sessions_src, &self.nika_dir.join("sessions"))?;
        }

        // Restore traces/
        let traces_src = nika_staging.join("traces");
        if traces_src.exists() {
            copy_dir_recursive(&traces_src, &self.nika_dir.join("traces"))?;
        }

        // Restore workflows/
        let workflows_src = nika_staging.join("workflows");
        if workflows_src.exists() {
            copy_dir_recursive(&workflows_src, &self.nika_dir.join("workflows"))?;
        }

        Ok(())
    }
}

/// NovaNet adapter - backs up brain/ YAML source of truth.
pub struct NovaNetAdapter {
    brain_path: Option<PathBuf>,
}

impl Default for NovaNetAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NovaNetAdapter {
    pub fn new() -> Self {
        // Look for brain/ in common locations
        let brain_path = Self::find_brain_path();
        Self { brain_path }
    }

    fn find_brain_path() -> Option<PathBuf> {
        let candidates = [
            dirs::home_dir().map(|h| h.join("dev/supernovae/brain")),
            std::env::current_dir().ok().map(|c| c.join("brain")),
        ];

        for candidate in candidates.into_iter().flatten() {
            if candidate.join("models").exists() {
                return Some(candidate);
            }
        }
        None
    }
}

impl BackupAdapter for NovaNetAdapter {
    fn name(&self) -> &str {
        "novanet"
    }

    fn is_available(&self) -> bool {
        self.brain_path.is_some()
    }

    fn version(&self) -> Option<String> {
        None
    }

    fn collect(&self, staging_dir: &Path) -> std::result::Result<AdapterContents, BackupError> {
        let brain_path = self
            .brain_path
            .as_ref()
            .ok_or(BackupError::NotAvailable("NovaNet brain/ not found".into()))?;

        let novanet_staging = staging_dir.join("novanet");
        fs::create_dir_all(&novanet_staging)?;

        let mut schema_files = 0;
        let mut seed_files = 0;

        // Copy brain/models/
        let models_src = brain_path.join("models");
        if models_src.exists() {
            let models_dst = novanet_staging.join("brain/models");
            copy_dir_recursive(&models_src, &models_dst)?;
            schema_files = count_yaml_files(&models_dst);
        }

        // Copy brain/seed/
        let seed_src = brain_path.join("seed");
        if seed_src.exists() {
            let seed_dst = novanet_staging.join("brain/seed");
            copy_dir_recursive(&seed_src, &seed_dst)?;
            seed_files = count_yaml_files(&seed_dst);
        }

        Ok(AdapterContents::NovaNet(NovaNetContents {
            schema_files,
            seed_files,
            neo4j_dump: false,
        }))
    }

    fn restore(&self, staging_dir: &Path) -> std::result::Result<(), BackupError> {
        let brain_path = self
            .brain_path
            .as_ref()
            .ok_or(BackupError::NotAvailable("NovaNet brain/ not found".into()))?;

        let novanet_staging = staging_dir.join("novanet");

        if !novanet_staging.exists() {
            return Ok(());
        }

        // Restore brain/models/
        let models_src = novanet_staging.join("brain/models");
        if models_src.exists() {
            copy_dir_recursive(&models_src, &brain_path.join("models"))?;
        }

        // Restore brain/seed/
        let seed_src = novanet_staging.join("brain/seed");
        if seed_src.exists() {
            copy_dir_recursive(&seed_src, &brain_path.join("seed"))?;
        }

        Ok(())
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Create a tar.gz archive from a directory.
fn create_archive(source_dir: &Path, archive_path: &Path) -> Result<()> {
    let tar_gz = File::create(archive_path)?;
    let encoder = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);

    // Add all files from source directory
    for entry in walkdir::WalkDir::new(source_dir) {
        let entry = entry.map_err(|e| SpnError::Other(e.into()))?;
        let path = entry.path();

        if path == source_dir {
            continue;
        }

        let relative = path
            .strip_prefix(source_dir)
            .map_err(|e| SpnError::Other(e.into()))?;

        if path.is_file() {
            archive.append_path_with_name(path, relative)?;
        } else if path.is_dir() {
            archive.append_dir(relative, path)?;
        }
    }

    archive.finish()?;
    Ok(())
}

/// Extract a tar.gz archive to a directory.
fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let tar_gz = File::open(archive_path)?;
    let decoder = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(decoder);

    archive.unpack(dest_dir)?;
    Ok(())
}

/// Remove password/secret fields from config content.
fn sanitize_config(content: &str) -> String {
    content
        .lines()
        .filter(|line| {
            let lower = line.to_lowercase();
            !lower.contains("password")
                && !lower.contains("secret")
                && !lower.contains("token")
                && !lower.contains("api_key")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Copy a directory recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::result::Result<(), BackupError> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Count files in a directory.
fn count_files(dir: &Path) -> u32 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count() as u32
}

/// Count YAML files in a directory.
fn count_yaml_files(dir: &Path) -> u32 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .map_or(false, |ext| ext == "yaml" || ext == "yml")
        })
        .count() as u32
}

/// Format bytes as human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

// ============================================================================
// COMMAND EXECUTION
// ============================================================================

/// Execute a backup command.
pub async fn run(command: BackupCommands) -> Result<()> {
    // Initialize backup manager with all adapters
    let mut manager = BackupManager::new();
    manager.register_adapter(Box::new(SpnAdapter::new()));
    manager.register_adapter(Box::new(NikaAdapter::new()));
    manager.register_adapter(Box::new(NovaNetAdapter::new()));

    match command {
        BackupCommands::Create { label } => {
            println!("{} Creating backup...", style("🔄").cyan());

            let info = manager.create_backup(label.as_deref())?;

            println!();
            println!("{} Backup created successfully!", style("✅").green());
            println!();
            println!("   {} {}", style("📦 File:").bold(), info.path.display());
            println!(
                "   {} {}",
                style("📊 Size:").bold(),
                format_bytes(info.size_bytes)
            );
            println!("   {} {}", style("🕐 Time:").bold(), &info.timestamp[..19]);

            if let Some(label) = &info.manifest.label {
                println!("   {} {}", style("🏷️  Label:").bold(), label);
            }

            // Show what was backed up
            println!();
            println!("   {}", style("Contents:").dim());
            if info.manifest.contents.spn.has_config
                || info.manifest.contents.spn.has_mcp_yaml
                || info.manifest.contents.spn.has_jobs
            {
                println!("   └── spn: config, MCP servers");
            }
            if info.manifest.contents.nika.session_count > 0
                || info.manifest.contents.nika.workflow_files > 0
            {
                println!(
                    "   └── nika: {} sessions, {} workflows",
                    info.manifest.contents.nika.session_count,
                    info.manifest.contents.nika.workflow_files
                );
            }
            if info.manifest.contents.novanet.schema_files > 0 {
                println!(
                    "   └── novanet: {} schema, {} seed files",
                    info.manifest.contents.novanet.schema_files,
                    info.manifest.contents.novanet.seed_files
                );
            }

            Ok(())
        }

        BackupCommands::Restore { backup, force } => {
            let backup_path = if backup == "latest" {
                let backups = manager.list_backups()?;
                backups
                    .first()
                    .map(|b| b.path.clone())
                    .ok_or_else(|| SpnError::NotFound("No backups found".to_string()))?
            } else {
                PathBuf::from(&backup)
            };

            if !force {
                println!(
                    "{} This will overwrite existing data!",
                    style("⚠️").yellow()
                );
                println!(
                    "   Backup: {}",
                    backup_path.file_name().unwrap().to_string_lossy()
                );
                print!("   Continue? [y/N] ");
                std::io::stdout().flush()?;

                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            println!("{} Restoring from backup...", style("🔄").cyan());

            let info = manager.restore_backup(&backup_path)?;

            println!();
            println!("{} Restore completed successfully!", style("✅").green());
            println!();
            println!(
                "   {} {}",
                style("📦 From:").bold(),
                backup_path.file_name().unwrap().to_string_lossy()
            );
            println!(
                "   {} {}",
                style("🕐 Original:").bold(),
                &info.manifest.created_at[..19]
            );

            Ok(())
        }

        BackupCommands::List { verbose, limit } => {
            let backups = manager.list_backups()?;

            if backups.is_empty() {
                println!("No backups found in ~/.spn/backups/");
                return Ok(());
            }

            println!(
                "{} Available backups ({} total):",
                style("📦").cyan(),
                backups.len()
            );
            println!();

            for (i, backup) in backups.iter().take(limit).enumerate() {
                let marker = if i == 0 {
                    style("→").green()
                } else {
                    style(" ").dim()
                };
                let name = backup.path.file_name().unwrap().to_string_lossy();

                println!(
                    "{} {} ({}, {})",
                    marker,
                    name,
                    format_bytes(backup.size_bytes),
                    &backup.timestamp[..16]
                );

                if verbose {
                    let m = &backup.manifest;
                    if m.contents.novanet.schema_files > 0 {
                        println!(
                            "     NovaNet: {} schema, {} seed files",
                            m.contents.novanet.schema_files, m.contents.novanet.seed_files
                        );
                    }
                    if m.contents.nika.workflow_files > 0 || m.contents.nika.session_count > 0 {
                        println!(
                            "     Nika: {} workflows, {} sessions",
                            m.contents.nika.workflow_files, m.contents.nika.session_count
                        );
                    }
                    if m.contents.spn.has_config {
                        println!("     spn: config, MCP servers");
                    }
                }
            }

            if backups.len() > limit {
                println!();
                println!(
                    "   {} {} more backups not shown",
                    style("...").dim(),
                    backups.len() - limit
                );
            }

            Ok(())
        }

        BackupCommands::Prune { keep, execute } => {
            let backups = manager.list_backups()?;

            if backups.len() <= keep {
                println!(
                    "{} Nothing to prune ({} backups, keeping {})",
                    style("✅").green(),
                    backups.len(),
                    keep
                );
                return Ok(());
            }

            let to_delete = &backups[keep..];

            if execute {
                println!(
                    "{} Deleting {} old backups...",
                    style("🗑️").red(),
                    to_delete.len()
                );
                for backup in to_delete {
                    manager.delete_backup(&backup.path)?;
                    println!(
                        "   Deleted: {}",
                        backup.path.file_name().unwrap().to_string_lossy()
                    );
                }
                println!("{} Pruned {} backups", style("✅").green(), to_delete.len());
            } else {
                println!(
                    "{} Dry run - would delete {} backups:",
                    style("🔍").cyan(),
                    to_delete.len()
                );
                for backup in to_delete {
                    println!("   {}", backup.path.file_name().unwrap().to_string_lossy());
                }
                println!();
                println!("Run with {} to actually delete", style("--execute").bold());
            }

            Ok(())
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_manager_new() {
        let manager = BackupManager::new();
        assert!(manager.backup_dir.ends_with("backups"));
    }

    #[test]
    fn test_spn_adapter_is_available() {
        let adapter = SpnAdapter::new();
        // May or may not be available depending on system
        let _ = adapter.is_available();
    }

    #[test]
    fn test_nika_adapter_is_available() {
        let adapter = NikaAdapter::new();
        let _ = adapter.is_available();
    }

    #[test]
    fn test_novanet_adapter_is_available() {
        let adapter = NovaNetAdapter::new();
        let _ = adapter.is_available();
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_sanitize_config() {
        let config = r#"
[neo4j]
uri = "bolt://localhost:7687"
user = "neo4j"
password = "secret123"

[general]
verbose = true
"#;
        let sanitized = sanitize_config(config);
        assert!(!sanitized.contains("password"));
        assert!(sanitized.contains("uri"));
        assert!(sanitized.contains("verbose"));
    }

    #[test]
    fn test_backup_without_adapters() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = BackupManager::new();
        manager.backup_dir = temp_dir.path().to_path_buf();

        // Create backup with no adapters
        let info = manager.create_backup(Some("empty")).unwrap();
        assert!(info.path.exists());
        assert!(info.path.to_string_lossy().contains("empty"));
    }

    #[test]
    fn test_list_backups_empty() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = BackupManager::new();
        manager.backup_dir = temp_dir.path().to_path_buf();

        let backups = manager.list_backups().unwrap();
        assert!(backups.is_empty());
    }
}
