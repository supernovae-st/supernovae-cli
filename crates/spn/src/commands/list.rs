//! List command implementation.
//!
//! Lists installed packages from storage and manifest.

use std::env;

use crate::error::Result;
use crate::manifest::SpnManifest;
use crate::storage::LocalStorage;

pub async fn run() -> Result<()> {
    let cwd = env::current_dir()?;

    // Try to load manifest
    let manifest = SpnManifest::find_in_dir(&cwd).ok();

    // Scan filesystem for all packages (includes manually installed ones)
    let storage = LocalStorage::new()?;
    let installed = storage.scan_filesystem()?;

    println!("📦 Installed packages:\n");

    if installed.is_empty() && manifest.as_ref().map_or(true, |m| m.dependencies.is_empty()) {
        println!("   No packages installed.");
        println!();
        println!("   Get started:");
        println!("   • spn add @workflows/dev-productivity/code-review");
        println!("   • spn skill add brainstorming");
        return Ok(());
    }

    // Show manifest dependencies
    if let Some(ref m) = manifest {
        if !m.dependencies.is_empty() {
            println!("   From spn.yaml:");
            for (name, dep) in &m.dependencies {
                let version = dep.version();
                let status = if installed.iter().any(|p| &p.name == name) {
                    "✓"
                } else {
                    "○"
                };
                println!("   {} {} @ {}", status, name, version);
            }
            println!();
        }
    }

    // Show installed packages from storage
    if !installed.is_empty() {
        println!("   In ~/.spn/packages:");
        for pkg in &installed {
            let in_manifest = manifest
                .as_ref()
                .is_some_and(|m| m.dependencies.contains_key(&pkg.name));

            let status = if in_manifest { "✓" } else { "⚡" };
            println!("   {} {} @ {}", status, pkg.name, pkg.version);
        }
        println!();
    }

    // Legend
    println!("   ✓ = in manifest & installed");
    println!("   ○ = in manifest, not installed");
    println!("   ⚡ = installed directly (not in manifest)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::storage::LocalStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_list_storage_empty() {
        let temp = TempDir::new().unwrap();
        let storage = LocalStorage::with_root(temp.path()).unwrap();
        let installed = storage.list_installed().unwrap();
        assert!(installed.is_empty());
    }
}
