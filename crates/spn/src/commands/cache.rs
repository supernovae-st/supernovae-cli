//! Cache command implementation.
//!
//! Manages the local package cache (downloaded tarballs).

use crate::error::Result;
use crate::storage::LocalStorage;
use crate::ux::design_system as ds;

/// Cache subcommand.
#[derive(Clone)]
pub enum CacheCommand {
    /// Clear the package cache.
    Clear,
    /// Show cache info.
    Info { json: bool },
}

/// Run a cache command.
pub async fn run(command: CacheCommand) -> Result<()> {
    match command {
        CacheCommand::Clear => clear().await,
        CacheCommand::Info { json } => info(json).await,
    }
}

/// Clear the package cache.
async fn clear() -> Result<()> {
    let storage = LocalStorage::new()?;

    println!("{} Clearing package cache...", ds::primary("🗑️"));

    storage.clear_cache()?;

    println!("{} Cache cleared", ds::success("✓"));

    Ok(())
}

/// Show cache info.
async fn info(json: bool) -> Result<()> {
    let paths =
        spn_client::SpnPaths::new().map_err(|e| anyhow::anyhow!("Failed to get paths: {}", e))?;
    let cache_dir = paths.cache_dir();
    let tarballs_dir = cache_dir.join("tarballs");

    // Calculate cache size
    let (file_count, total_size) = if tarballs_dir.exists() {
        let mut count = 0usize;
        let mut size = 0u64;

        if let Ok(entries) = std::fs::read_dir(&tarballs_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        count += 1;
                        size += metadata.len();
                    }
                }
            }
        }

        (count, size)
    } else {
        (0, 0)
    };

    if json {
        let info = serde_json::json!({
            "path": cache_dir.to_string_lossy(),
            "tarballs_path": tarballs_dir.to_string_lossy(),
            "file_count": file_count,
            "total_bytes": total_size,
            "total_size_human": format_size(total_size),
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("{}", ds::primary("Package Cache"));
        println!();
        println!("  Path:     {}", cache_dir.display());
        println!("  Files:    {}", file_count);
        println!("  Size:     {}", format_size(total_size));

        if file_count > 0 {
            println!();
            println!(
                "  {} Run '{}' to free space",
                ds::primary("→"),
                ds::command("spn cache clear")
            );
        }
    }

    Ok(())
}

/// Format bytes as human-readable size.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 bytes");
        assert_eq!(format_size(512), "512 bytes");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }
}
