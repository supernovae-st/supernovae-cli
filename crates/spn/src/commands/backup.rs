//! Backup and restore commands for SuperNovae ecosystem.
//!
//! Creates unified backups of NovaNet schema/seeds, Nika workflows/sessions,
//! and spn configuration. Backups are stored in ~/.spn/backups/ as tar.gz archives.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, Utc};

use crate::error::{Result, SpnError};
use clap::Subcommand;
use console::style;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use tar::{Archive, Builder};
use walkdir::WalkDir;

/// Backup subcommands.
#[derive(Subcommand)]
pub enum BackupCommands {
    /// Create a new backup of all SuperNovae data
    Create {
        /// Optional label for the backup (e.g., "before-refactor")
        #[arg(short, long)]
        label: Option<String>,

        /// Include only specific subsystems (novanet, nika, spn)
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,
    },

    /// Restore from a backup
    Restore {
        /// Backup file path, or "latest" for most recent
        #[arg(default_value = "latest")]
        backup: String,

        /// Force restore without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List available backups
    #[command(visible_alias = "l", visible_alias = "ls")]
    List {
        /// Show detailed information (manifest contents)
        #[arg(short, long)]
        detailed: bool,

        /// Maximum number of backups to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

/// Run backup command.
pub async fn run(command: BackupCommands) -> Result<()> {
    match command {
        BackupCommands::Create { label, only } => create_backup(label, only).await,
        BackupCommands::Restore { backup, force } => restore_backup(&backup, force).await,
        BackupCommands::List {
            detailed,
            limit,
            json,
        } => list_backups(detailed, limit, json).await,
        BackupCommands::Prune { keep, execute } => prune_backups(keep, execute).await,
    }
}

/// Get the backup directory (~/.spn/backups/).
fn backup_dir() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| SpnError::Other(anyhow::anyhow!("$HOME environment variable not set")))?
        .join(".spn")
        .join("backups"))
}

/// Generate a backup filename with timestamp and optional label.
fn generate_backup_name(timestamp: &DateTime<Utc>, label: Option<&str>) -> String {
    let ts = timestamp.format("%Y-%m-%dT%H-%M-%S");
    match label {
        Some(l) => format!("backup-{}-{}.tar.gz", ts, sanitize_label(l)),
        None => format!("backup-{}.tar.gz", ts),
    }
}

/// Sanitize label for use in filename.
fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

/// Create a new backup.
async fn create_backup(label: Option<String>, only: Option<Vec<String>>) -> Result<()> {
    let backup_path = backup_dir()?;
    fs::create_dir_all(&backup_path).context("Failed to create backup directory")?;

    let timestamp = Utc::now();
    let name = generate_backup_name(&timestamp, label.as_deref());
    let archive_path = backup_path.join(&name);

    println!("{} Creating backup...", style("").cyan());

    // Create staging directory
    let staging = tempfile::tempdir().context("Failed to create staging directory")?;

    // Collect data from each subsystem
    let mut manifest = BackupManifest::new(&timestamp, label.clone());
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );

    let subsystems = only.unwrap_or_else(|| vec!["novanet".into(), "nika".into(), "spn".into()]);

    // NovaNet
    if subsystems.iter().any(|s| s == "novanet") {
        pb.set_message("Collecting NovaNet data...");
        manifest.novanet = collect_novanet(staging.path())?;
    }

    // Nika
    if subsystems.iter().any(|s| s == "nika") {
        pb.set_message("Collecting Nika data...");
        manifest.nika = collect_nika(staging.path())?;
    }

    // spn
    if subsystems.iter().any(|s| s == "spn") {
        pb.set_message("Collecting spn data...");
        manifest.spn = collect_spn(staging.path())?;
    }

    // Calculate checksums
    pb.set_message("Calculating checksums...");
    manifest.checksums = calculate_checksums(staging.path())?;

    // Write manifest
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(staging.path().join("manifest.json"), &manifest_json)?;

    // Create tar.gz archive
    pb.set_message("Compressing archive...");
    create_tar_gz(staging.path(), &archive_path)?;

    pb.finish_and_clear();

    // Get final size
    let size = fs::metadata(&archive_path)?.len();

    println!("{} Backup created successfully!", style("").green());
    println!();
    println!("   {} {}", style("File:").dim(), archive_path.display());
    println!("   {} {}", style("Size:").dim(), format_bytes(size));
    println!(
        "   {} {}",
        style("Time:").dim(),
        timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );

    if let Some(l) = &label {
        println!("   {} {}", style("Label:").dim(), l);
    }

    // Summary
    println!();
    println!("   {}", style("Contents:").dim());
    if manifest.novanet.schema_files > 0 || manifest.novanet.seed_files > 0 {
        println!(
            "     NovaNet: {} schema, {} seed files",
            manifest.novanet.schema_files, manifest.novanet.seed_files
        );
    }
    if manifest.nika.workflow_files > 0 || manifest.nika.session_count > 0 {
        println!(
            "     Nika: {} workflows, {} sessions",
            manifest.nika.workflow_files, manifest.nika.session_count
        );
    }
    if manifest.spn.has_config || manifest.spn.has_mcp_yaml {
        let mut items = Vec::new();
        if manifest.spn.has_config {
            items.push("config");
        }
        if manifest.spn.has_mcp_yaml {
            items.push("mcp.yaml");
        }
        if manifest.spn.has_jobs {
            items.push("jobs");
        }
        println!("     spn: {}", items.join(", "));
    }

    Ok(())
}

/// Restore from a backup.
async fn restore_backup(backup: &str, force: bool) -> Result<()> {
    let backup_path = if backup == "latest" {
        find_latest_backup()?
    } else {
        PathBuf::from(backup)
    };

    if !backup_path.exists() {
        return Err(SpnError::NotFound(format!(
            "Backup not found: {}",
            backup_path.display()
        )));
    }

    // Read manifest first to show what will be restored
    let staging = tempfile::tempdir()?;
    extract_tar_gz(&backup_path, staging.path())?;

    let manifest_path = staging.path().join("manifest.json");
    let manifest: BackupManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;

    if !force {
        println!("{} This will overwrite existing data!", style("").yellow());
        println!();
        println!("   Backup: {}", backup_path.display());
        println!("   Created: {}", manifest.created_at);
        if let Some(label) = &manifest.label {
            println!("   Label: {}", label);
        }
        println!();
        print!("   Continue? [y/N] ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );

    // Restore NovaNet
    if staging.path().join("novanet").exists() {
        pb.set_message("Restoring NovaNet data...");
        restore_novanet(staging.path())?;
    }

    // Restore Nika
    if staging.path().join("nika").exists() {
        pb.set_message("Restoring Nika data...");
        restore_nika(staging.path())?;
    }

    // Restore spn
    if staging.path().join("spn").exists() {
        pb.set_message("Restoring spn data...");
        restore_spn(staging.path())?;
    }

    pb.finish_and_clear();

    println!("{} Restore completed successfully!", style("").green());
    println!();
    println!("   From: {}", backup_path.display());
    println!("   Original backup: {}", manifest.created_at);

    Ok(())
}

/// List available backups.
async fn list_backups(detailed: bool, limit: usize, json: bool) -> Result<()> {
    let backup_path = backup_dir()?;

    if !backup_path.exists() {
        if json {
            println!("[]");
        } else {
            println!("No backups found in {}", backup_path.display());
        }
        return Ok(());
    }

    let mut backups: Vec<BackupInfo> = Vec::new();

    for entry in fs::read_dir(&backup_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "gz") {
            if let Ok(info) = read_backup_info(&path) {
                backups.push(info);
            }
        }
    }

    // Sort by timestamp, newest first
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    if json {
        let limited: Vec<_> = backups.into_iter().take(limit).collect();
        println!("{}", serde_json::to_string_pretty(&limited)?);
        return Ok(());
    }

    if backups.is_empty() {
        println!("No backups found in {}", backup_path.display());
        return Ok(());
    }

    println!(
        "{} Available backups ({} total):",
        style("").cyan(),
        backups.len()
    );
    println!();

    for (i, backup) in backups.iter().take(limit).enumerate() {
        let marker = if i == 0 { "" } else { " " };
        let name = backup
            .path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();

        println!(
            " {} {} ({}, {})",
            marker,
            style(&name).bold(),
            format_bytes(backup.size_bytes),
            backup.timestamp
        );

        if detailed {
            if let Some(manifest) = &backup.manifest {
                if manifest.novanet.schema_files > 0 || manifest.novanet.seed_files > 0 {
                    println!(
                        "      NovaNet: {} schema, {} seed",
                        manifest.novanet.schema_files, manifest.novanet.seed_files
                    );
                }
                if manifest.nika.workflow_files > 0 || manifest.nika.session_count > 0 {
                    println!(
                        "      Nika: {} workflows, {} sessions",
                        manifest.nika.workflow_files, manifest.nika.session_count
                    );
                }
                if manifest.spn.has_mcp_yaml || manifest.spn.has_config {
                    println!(
                        "      spn: {}{}",
                        if manifest.spn.has_mcp_yaml {
                            "mcp.yaml"
                        } else {
                            ""
                        },
                        if manifest.spn.has_config {
                            ", config"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
    }

    if backups.len() > limit {
        println!();
        println!(
            "   {} more backups not shown (use -n to show more)",
            backups.len() - limit
        );
    }

    Ok(())
}

/// Prune old backups.
async fn prune_backups(keep: usize, execute: bool) -> Result<()> {
    let backup_path = backup_dir()?;

    if !backup_path.exists() {
        println!("{} No backups to prune", style("").green());
        return Ok(());
    }

    let mut backups: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(&backup_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "gz") {
            backups.push(path);
        }
    }

    // Sort by name (which includes timestamp)
    backups.sort();
    backups.reverse(); // Newest first

    if backups.len() <= keep {
        println!(
            "{} Nothing to prune ({} backups, keeping {})",
            style("").green(),
            backups.len(),
            keep
        );
        return Ok(());
    }

    let to_delete = &backups[keep..];

    if execute {
        println!(
            "{} Deleting {} old backups...",
            style("").cyan(),
            to_delete.len()
        );
        for path in to_delete {
            fs::remove_file(path)?;
            println!(
                "   Deleted: {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        println!("{} Pruned {} backups", style("").green(), to_delete.len());
    } else {
        println!(
            "{} Dry run - would delete {} backups:",
            style("").cyan(),
            to_delete.len()
        );
        for path in to_delete {
            println!(
                "   {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        println!();
        println!("Run with --execute to actually delete");
    }

    Ok(())
}

// === Subsystem collectors ===

fn collect_novanet(staging: &Path) -> Result<NovaNetContents> {
    let mut contents = NovaNetContents::default();

    // Look for private-data/ in common locations
    let private_data_paths = [
        dirs::home_dir().map(|h| h.join("dev/supernovae/private-data")),
        std::env::current_dir()
            .ok()
            .map(|p| p.join("../private-data")),
    ];

    let private_data_path = private_data_paths
        .into_iter()
        .flatten()
        .find(|p| p.join("models").exists());

    if let Some(private_data) = private_data_path {
        let novanet_staging = staging.join("novanet");
        fs::create_dir_all(&novanet_staging)?;

        // Copy models/
        let models_src = private_data.join("models");
        if models_src.exists() {
            let models_dst = novanet_staging.join("private-data/models");
            copy_dir_recursive(&models_src, &models_dst)?;
            contents.schema_files = count_yaml_files(&models_dst);
        }

        // Copy seed/
        let seed_src = private_data.join("seed");
        if seed_src.exists() {
            let seed_dst = novanet_staging.join("private-data/seed");
            copy_dir_recursive(&seed_src, &seed_dst)?;
            contents.seed_files = count_yaml_files(&seed_dst);
        }
    }

    Ok(contents)
}

fn collect_nika(staging: &Path) -> Result<NikaContents> {
    let mut contents = NikaContents::default();

    // ~/.nika/ directory
    if let Some(nika_dir) = dirs::home_dir().map(|h| h.join(".nika")) {
        if nika_dir.exists() {
            let nika_staging = staging.join("nika");
            fs::create_dir_all(&nika_staging)?;

            // Copy sessions/
            let sessions_src = nika_dir.join("sessions");
            if sessions_src.exists() {
                let sessions_dst = nika_staging.join("sessions");
                copy_dir_recursive(&sessions_src, &sessions_dst)?;
                contents.session_count = count_files(&sessions_dst);
            }

            // Copy traces/
            let traces_src = nika_dir.join("traces");
            if traces_src.exists() {
                let traces_dst = nika_staging.join("traces");
                copy_dir_recursive(&traces_src, &traces_dst)?;
                contents.trace_count = count_files(&traces_dst);
            }
        }
    }

    // Look for workflows in nika project
    let nika_paths = [dirs::home_dir().map(|h| h.join("dev/supernovae/nika/workflows"))];

    for path in nika_paths.into_iter().flatten() {
        if path.exists() {
            let workflows_dst = staging.join("nika/workflows");
            fs::create_dir_all(&workflows_dst)?;
            copy_dir_recursive(&path, &workflows_dst)?;
            contents.workflow_files = count_yaml_files(&workflows_dst);
            break;
        }
    }

    Ok(contents)
}

fn collect_spn(staging: &Path) -> Result<SpnContents> {
    let mut contents = SpnContents::default();

    let spn_dir = dirs::home_dir()
        .ok_or_else(|| SpnError::Other(anyhow::anyhow!("$HOME environment variable not set")))?
        .join(".spn");

    if !spn_dir.exists() {
        return Ok(contents);
    }

    let spn_staging = staging.join("spn");
    fs::create_dir_all(&spn_staging)?;

    // Copy config.toml (sanitized - no secrets)
    let config_path = spn_dir.join("config.toml");
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let sanitized = sanitize_config(&content);
        fs::write(spn_staging.join("config.toml"), sanitized)?;
        contents.has_config = true;
    }

    // Copy mcp.yaml
    let mcp_path = spn_dir.join("mcp.yaml");
    if mcp_path.exists() {
        fs::copy(&mcp_path, spn_staging.join("mcp.yaml"))?;
        contents.has_mcp_yaml = true;
    }

    // Copy jobs.json
    let jobs_path = spn_dir.join("jobs.json");
    if jobs_path.exists() {
        fs::copy(&jobs_path, spn_staging.join("jobs.json"))?;
        contents.has_jobs = true;
    }

    Ok(contents)
}

// === Subsystem restorers ===

fn restore_novanet(staging: &Path) -> Result<()> {
    let novanet_staging = staging.join("novanet");
    if !novanet_staging.exists() {
        return Ok(());
    }

    // Find private-data/ destination
    let private_data_paths = [dirs::home_dir().map(|h| h.join("dev/supernovae/private-data"))];

    let private_data_path = private_data_paths
        .into_iter()
        .flatten()
        .find(|p| p.exists());

    if let Some(private_data) = private_data_path {
        // Restore models/
        let models_src = novanet_staging.join("private-data/models");
        if models_src.exists() {
            let models_dst = private_data.join("models");
            copy_dir_recursive(&models_src, &models_dst)?;
        }

        // Restore seed/
        let seed_src = novanet_staging.join("private-data/seed");
        if seed_src.exists() {
            let seed_dst = private_data.join("seed");
            copy_dir_recursive(&seed_src, &seed_dst)?;
        }
    }

    Ok(())
}

fn restore_nika(staging: &Path) -> Result<()> {
    let nika_staging = staging.join("nika");
    if !nika_staging.exists() {
        return Ok(());
    }

    if let Some(nika_dir) = dirs::home_dir().map(|h| h.join(".nika")) {
        fs::create_dir_all(&nika_dir)?;

        // Restore sessions/
        let sessions_src = nika_staging.join("sessions");
        if sessions_src.exists() {
            copy_dir_recursive(&sessions_src, &nika_dir.join("sessions"))?;
        }

        // Restore traces/
        let traces_src = nika_staging.join("traces");
        if traces_src.exists() {
            copy_dir_recursive(&traces_src, &nika_dir.join("traces"))?;
        }
    }

    Ok(())
}

fn restore_spn(staging: &Path) -> Result<()> {
    let spn_staging = staging.join("spn");
    if !spn_staging.exists() {
        return Ok(());
    }

    let spn_dir = dirs::home_dir()
        .ok_or_else(|| SpnError::Other(anyhow::anyhow!("$HOME environment variable not set")))?
        .join(".spn");
    fs::create_dir_all(&spn_dir)?;

    // Restore config.toml
    let config_src = spn_staging.join("config.toml");
    if config_src.exists() {
        fs::copy(&config_src, spn_dir.join("config.toml"))?;
    }

    // Restore mcp.yaml
    let mcp_src = spn_staging.join("mcp.yaml");
    if mcp_src.exists() {
        fs::copy(&mcp_src, spn_dir.join("mcp.yaml"))?;
    }

    // Restore jobs.json
    let jobs_src = spn_staging.join("jobs.json");
    if jobs_src.exists() {
        fs::copy(&jobs_src, spn_dir.join("jobs.json"))?;
    }

    Ok(())
}

// === Utilities ===

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    // Security: Don't follow symlinks to prevent infinite loops and data exfiltration
    for entry in WalkDir::new(src).min_depth(1).follow_links(false) {
        let entry = entry.map_err(|e| SpnError::Other(e.into()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(src)
            .map_err(|e| SpnError::Other(e.into()))?;
        let target = dst.join(relative);

        if path.is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }

    Ok(())
}

fn count_yaml_files(dir: &Path) -> u32 {
    if !dir.exists() {
        return 0;
    }

    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        })
        .count() as u32
}

fn count_files(dir: &Path) -> u32 {
    if !dir.exists() {
        return 0;
    }

    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u32
}

fn calculate_checksums(dir: &Path) -> Result<HashMap<String, String>> {
    let mut checksums = HashMap::new();

    for entry in WalkDir::new(dir).min_depth(1) {
        let entry = entry.map_err(|e| SpnError::Other(e.into()))?;
        if entry.file_type().is_file() {
            let path = entry.path();
            let relative = path
                .strip_prefix(dir)
                .map_err(|e| SpnError::Other(e.into()))?
                .to_string_lossy()
                .to_string();

            let mut file = File::open(path)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 8192];

            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }

            let hash = hex::encode(hasher.finalize());
            checksums.insert(relative, hash);
        }
    }

    Ok(checksums)
}

fn create_tar_gz(source: &Path, archive_path: &Path) -> Result<()> {
    let file = File::create(archive_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(encoder);

    for entry in WalkDir::new(source).min_depth(1) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;

        if path.is_file() {
            tar.append_path_with_name(path, relative)?;
        } else if path.is_dir() {
            tar.append_dir(relative, path)?;
        }
    }

    tar.into_inner()?.finish()?;
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, dest: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Security: Validate each entry before extraction to prevent path traversal
    for entry in archive.entries().map_err(|e| SpnError::Other(e.into()))? {
        let mut entry = entry.map_err(|e| SpnError::Other(e.into()))?;
        let path = entry.path().map_err(|e| SpnError::Other(e.into()))?;

        // Reject absolute paths
        if path.is_absolute() {
            return Err(SpnError::Other(anyhow::anyhow!(
                "Security: Absolute path in archive rejected: {}",
                path.display()
            )));
        }

        // Reject path traversal attempts (../)
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(SpnError::Other(anyhow::anyhow!(
                "Security: Path traversal attempt rejected: {}",
                path.display()
            )));
        }

        // Safe to extract
        entry
            .unpack_in(dest)
            .map_err(|e| SpnError::Other(e.into()))?;
    }
    Ok(())
}

fn find_latest_backup() -> Result<PathBuf> {
    let backup_path = backup_dir()?;

    if !backup_path.exists() {
        return Err(SpnError::NotFound("No backups found".to_string()));
    }

    let mut backups: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(&backup_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "gz") {
            backups.push(path);
        }
    }

    backups.sort();
    backups
        .pop()
        .ok_or_else(|| SpnError::NotFound("No backups found".to_string()))
}

fn read_backup_info(path: &Path) -> Result<BackupInfo> {
    let staging = tempfile::tempdir()?;
    extract_tar_gz(path, staging.path())?;

    let manifest_path = staging.path().join("manifest.json");
    let manifest: Option<BackupManifest> = if manifest_path.exists() {
        Some(serde_json::from_str(&fs::read_to_string(&manifest_path)?)?)
    } else {
        None
    };

    let timestamp = manifest
        .as_ref()
        .map(|m| m.created_at.clone())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(BackupInfo {
        path: path.to_path_buf(),
        timestamp,
        size_bytes: fs::metadata(path)?.len(),
        manifest,
    })
}

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
        format!("{} B", bytes)
    }
}

// === Types (local, serializable versions) ===

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BackupManifest {
    version: String,
    created_at: String,
    label: Option<String>,
    hostname: String,
    versions: ComponentVersions,
    checksums: HashMap<String, String>,
    novanet: NovaNetContents,
    nika: NikaContents,
    spn: SpnContents,
}

impl BackupManifest {
    fn new(timestamp: &DateTime<Utc>, label: Option<String>) -> Self {
        Self {
            version: "1.0.0".to_string(),
            created_at: timestamp.to_rfc3339(),
            label,
            hostname: gethostname::gethostname().to_string_lossy().to_string(),
            versions: ComponentVersions {
                novanet: None,
                nika: None,
                spn: env!("CARGO_PKG_VERSION").to_string(),
            },
            checksums: HashMap::new(),
            novanet: NovaNetContents::default(),
            nika: NikaContents::default(),
            spn: SpnContents::default(),
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct ComponentVersions {
    novanet: Option<String>,
    nika: Option<String>,
    spn: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct NovaNetContents {
    schema_files: u32,
    seed_files: u32,
    neo4j_dump: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct NikaContents {
    workflow_files: u32,
    session_count: u32,
    trace_count: u32,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct SpnContents {
    has_config: bool,
    has_mcp_yaml: bool,
    has_jobs: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BackupInfo {
    path: PathBuf,
    timestamp: String,
    size_bytes: u64,
    manifest: Option<BackupManifest>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_backup_name() {
        let ts = chrono::Utc::now();
        let name = generate_backup_name(&ts, None);
        assert!(name.starts_with("backup-"));
        assert!(name.ends_with(".tar.gz"));

        let name_with_label = generate_backup_name(&ts, Some("pre-refactor"));
        assert!(name_with_label.contains("pre-refactor"));
    }

    #[test]
    fn test_sanitize_label() {
        assert_eq!(sanitize_label("Hello World"), "hello-world");
        assert_eq!(sanitize_label("pre-refactor"), "pre-refactor");
        assert_eq!(sanitize_label("v1.0.0"), "v1-0-0");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(100), "100 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_sanitize_config() {
        let config = r#"
[neo4j]
uri = "bolt://localhost:7687"
password = "secret123"
api_key = "sk-xxx"
"#;
        let sanitized = sanitize_config(config);
        assert!(!sanitized.contains("password"));
        assert!(!sanitized.contains("api_key"));
        assert!(sanitized.contains("uri"));
    }
}
