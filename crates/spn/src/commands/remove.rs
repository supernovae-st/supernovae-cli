//! Remove command implementation.
//!
//! Removes a package from the project's spn.yaml manifest and local storage.

use crate::ux::design_system as ds;

use crate::error::{Result, SpnError};
use crate::manifest::{SpnLockfile, SpnManifest};
use crate::storage::LocalStorage;

/// Options for the remove command.
pub struct RemoveOptions {
    /// Package name to remove.
    pub package: String,

    /// Only update manifest (don't remove from disk).
    pub manifest_only: bool,
}

/// Run the remove command.
pub async fn run(package: &str) -> Result<()> {
    let options = RemoveOptions {
        package: package.to_string(),
        manifest_only: false,
    };

    run_with_options(options).await
}

/// Run the remove command with full options.
pub async fn run_with_options(options: RemoveOptions) -> Result<()> {
    println!(
        "{} Removing package: {}",
        ds::primary("🗑️"),
        ds::warning(&options.package)
    );

    // 1. Find and load manifest
    let manifest_path = find_manifest()?;
    if !manifest_path.exists() {
        return Err(SpnError::ManifestNotFound);
    }

    let mut manifest =
        SpnManifest::from_file(&manifest_path).map_err(|_e| SpnError::ManifestNotFound)?;

    // 2. Check if package exists in dependencies
    let was_dep = manifest.dependencies.remove(&options.package).is_some();
    let was_dev_dep = manifest.dev_dependencies.remove(&options.package).is_some();

    if !was_dep && !was_dev_dep {
        println!(
            "   {} Package {} not found in manifest",
            ds::warning("⚠"),
            options.package
        );
        return Ok(());
    }

    // 3. Save manifest
    manifest
        .write_to_file(&manifest_path)
        .map_err(|e| SpnError::ConfigError(format!("Failed to save manifest: {}", e)))?;

    let dep_type = if was_dep {
        "dependencies"
    } else {
        "dev-dependencies"
    };
    println!("   {} Removed from {}", ds::success("✓"), dep_type);

    // 4. Remove from local storage (unless manifest-only)
    if !options.manifest_only {
        let storage = LocalStorage::new()
            .map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

        match storage.uninstall(&options.package) {
            Ok(()) => {
                println!("   {} Removed from local storage", ds::success("✓"));
            }
            Err(e) => {
                println!("   {} Could not remove from storage: {}", ds::warning("⚠"), e);
            }
        }
    }

    // 5. Update lockfile
    let lockfile_path = manifest_path
        .parent()
        .map(|p| p.join("spn.lock"))
        .unwrap_or_else(|| std::path::PathBuf::from("spn.lock"));

    if lockfile_path.exists() {
        let mut lockfile = SpnLockfile::from_file(&lockfile_path)
            .map_err(|e| SpnError::ConfigError(format!("Failed to read lockfile: {}", e)))?;

        let original_len = lockfile.packages.len();
        lockfile.packages.retain(|p| p.name != options.package);

        if lockfile.packages.len() < original_len {
            lockfile
                .write_to_file(&lockfile_path)
                .map_err(|e| SpnError::ConfigError(format!("Failed to save lockfile: {}", e)))?;
            println!("   {} Updated spn.lock", ds::success("✓"));
        }
    }

    println!(
        "{} Successfully removed {}",
        ds::warning("✨"),
        ds::success(&options.package)
    );
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
    fn test_remove_options_defaults() {
        let options = RemoveOptions {
            package: "@test/pkg".to_string(),
            manifest_only: false,
        };

        assert_eq!(options.package, "@test/pkg");
        assert!(!options.manifest_only);
    }

    #[tokio::test]
    async fn test_remove_no_manifest() {
        // Run from a temp directory with no manifest
        let temp = tempfile::TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = run("@test/pkg").await;
        assert!(result.is_err());
    }
}
