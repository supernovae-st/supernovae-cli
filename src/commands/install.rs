//! Install command implementation.
//!
//! Installs all dependencies from spn.yaml, optionally using spn.lock for frozen versions.

use colored::Colorize;

use crate::error::{Result, SpnError};
use crate::index::{Downloader, IndexClient};
use crate::manifest::{ResolvedPackage, SpnLockfile, SpnManifest};
use crate::storage::LocalStorage;

/// Options for the install command.
#[derive(Default)]
pub struct InstallOptions {
    /// Use exact versions from spn.lock (error if missing).
    pub frozen: bool,

    /// Skip downloading (verify manifest only).
    pub dry_run: bool,

    /// Only install production dependencies (skip dev).
    pub production: bool,
}

/// Run the install command.
pub async fn run(frozen: bool) -> Result<()> {
    let options = InstallOptions {
        frozen,
        ..Default::default()
    };

    run_with_options(options).await
}

/// Run the install command with full options.
pub async fn run_with_options(options: InstallOptions) -> Result<()> {
    println!("{} Installing dependencies...", "📦".cyan());

    // 1. Find and load manifest
    let manifest_path = find_manifest()?;
    if !manifest_path.exists() {
        return Err(SpnError::ManifestNotFound);
    }

    let manifest =
        SpnManifest::from_file(&manifest_path).map_err(|_e| SpnError::ManifestNotFound)?;

    // 2. Collect dependencies to install
    let dependencies: Vec<_> = if options.production {
        manifest.dependencies.iter().collect()
    } else {
        manifest
            .dependencies
            .iter()
            .chain(manifest.dev_dependencies.iter())
            .collect()
    };

    if dependencies.is_empty() {
        println!("   {} No dependencies to install", "✓".green());
        return Ok(());
    }

    println!(
        "   {} Found {} dependencies",
        "→".blue(),
        dependencies.len()
    );

    // 3. Load or create lockfile
    let lockfile_path = manifest_path
        .parent()
        .map(|p| p.join("spn.lock"))
        .unwrap_or_else(|| std::path::PathBuf::from("spn.lock"));

    let mut lockfile = if lockfile_path.exists() {
        SpnLockfile::from_file(&lockfile_path)
            .map_err(|e| SpnError::ConfigError(format!("Failed to read lockfile: {}", e)))?
    } else if options.frozen {
        return Err(SpnError::LockfileNotFound);
    } else {
        SpnLockfile::new()
    };

    // 4. Initialize components
    let client = IndexClient::new();
    let downloader = Downloader::new();
    let storage =
        LocalStorage::new().map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

    // 5. Install each dependency
    let mut installed_count = 0;
    let mut skipped_count = 0;

    for (name, dep) in &dependencies {
        let version_constraint = dep.version();

        // Check if already resolved in lockfile (for frozen mode)
        let entry = if options.frozen {
            // In frozen mode, we must have exact version in lockfile
            let locked = lockfile
                .packages
                .iter()
                .find(|p| &p.name == *name)
                .ok_or_else(|| {
                    SpnError::ConfigError(format!("Package {} not in lockfile (frozen mode)", name))
                })?;

            client
                .fetch_version(name, &locked.version)
                .await
                .map_err(|e| {
                    SpnError::PackageNotFound(format!("{}@{}: {}", name, locked.version, e))
                })?
        } else {
            // Resolve latest matching version
            client
                .fetch_latest(name)
                .await
                .map_err(|e| SpnError::PackageNotFound(format!("{}: {}", name, e)))?
        };

        // Check if already installed at correct version
        let installed_version = storage
            .installed_version(name)
            .map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

        if installed_version.as_ref() == Some(&entry.version) {
            println!(
                "   {} {}@{} (already installed)",
                "✓".green(),
                name,
                entry.version
            );
            skipped_count += 1;
            continue;
        }

        if options.dry_run {
            println!(
                "   {} Would install {}@{} (constraint: {})",
                "→".blue(),
                name,
                entry.version,
                version_constraint
            );
            continue;
        }

        // Download and install
        let downloaded = downloader
            .download_entry(&entry)
            .await
            .map_err(|e| SpnError::ConfigError(format!("Download failed for {}: {}", name, e)))?;

        let installed = storage
            .install(&downloaded)
            .map_err(|e| SpnError::ConfigError(format!("Install failed for {}: {}", name, e)))?;

        println!(
            "   {} {}@{} → {}",
            "✓".green(),
            name,
            entry.version,
            installed.path.display()
        );

        // Update lockfile
        if !options.frozen {
            // Remove old entry if exists
            lockfile.packages.retain(|p| p.name != **name);

            // Add new entry
            lockfile.add_package(ResolvedPackage::new(name, &entry.version, &entry.cksum));
        }

        installed_count += 1;
    }

    // 6. Save lockfile if not frozen
    if !options.frozen && !options.dry_run && installed_count > 0 {
        lockfile
            .write_to_file(&lockfile_path)
            .map_err(|e| SpnError::ConfigError(format!("Failed to save lockfile: {}", e)))?;
        println!("   {} Updated spn.lock", "✓".green());
    }

    // 7. Summary
    if options.dry_run {
        println!(
            "{} Dry run complete: {} would be installed",
            "✨".yellow(),
            dependencies.len()
        );
    } else {
        println!(
            "{} Installed {} packages ({} already up-to-date)",
            "✨".yellow(),
            installed_count,
            skipped_count
        );
    }

    Ok(())
}

/// Find the manifest file (spn.yaml or .spn/spn.yaml).
fn find_manifest() -> Result<std::path::PathBuf> {
    let cwd = std::env::current_dir()
        .map_err(|e| SpnError::ConfigError(format!("Cannot get current dir: {}", e)))?;

    // Check .spn/spn.yaml first
    let spn_dir = cwd.join(".spn").join("spn.yaml");
    if spn_dir.exists() {
        return Ok(spn_dir);
    }

    // Check root spn.yaml
    let root_manifest = cwd.join("spn.yaml");
    Ok(root_manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_options_defaults() {
        let options = InstallOptions::default();

        assert!(!options.frozen);
        assert!(!options.dry_run);
        assert!(!options.production);
    }

    #[tokio::test]
    async fn test_install_no_manifest() {
        // Run from a temp directory with no manifest
        let temp = tempfile::TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = run(false).await;
        assert!(result.is_err());
    }
}
