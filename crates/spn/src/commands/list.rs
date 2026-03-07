//! List command implementation.
//!
//! Lists installed packages from storage and manifest.

use std::env;

use serde::Serialize;

use crate::error::Result;
use crate::manifest::SpnManifest;
use crate::storage::LocalStorage;
use crate::ux::design_system as ds;

/// Package list for JSON output.
#[derive(Debug, Serialize)]
struct PackageList {
    manifest_packages: Vec<ManifestPackage>,
    installed_packages: Vec<InstalledPackage>,
}

/// Manifest package for JSON output.
#[derive(Debug, Serialize)]
struct ManifestPackage {
    name: String,
    version: String,
    installed: bool,
}

/// Installed package for JSON output.
#[derive(Debug, Serialize)]
struct InstalledPackage {
    name: String,
    version: String,
    in_manifest: bool,
}

pub async fn run(json: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    // Try to load manifest
    let manifest = SpnManifest::find_in_dir(&cwd).ok();

    // Scan filesystem for all packages (includes manually installed ones)
    let storage = LocalStorage::new()?;
    let installed = storage.scan_filesystem()?;

    // JSON output
    if json {
        let manifest_packages: Vec<ManifestPackage> = manifest
            .as_ref()
            .map(|m| {
                m.dependencies
                    .iter()
                    .map(|(name, dep)| ManifestPackage {
                        name: name.clone(),
                        version: dep.version().to_string(),
                        installed: installed.iter().any(|p| &p.name == name),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let installed_packages: Vec<InstalledPackage> = installed
            .iter()
            .map(|pkg| InstalledPackage {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                in_manifest: manifest
                    .as_ref()
                    .is_some_and(|m| m.dependencies.contains_key(&pkg.name)),
            })
            .collect();

        let list = PackageList {
            manifest_packages,
            installed_packages,
        };

        println!("{}", serde_json::to_string_pretty(&list)?);
        return Ok(());
    }

    // Human-readable output
    println!("{}", ds::primary("Installed packages:"));
    println!();

    if installed.is_empty() && manifest.as_ref().is_none_or(|m| m.dependencies.is_empty()) {
        println!("  {}", ds::muted("No packages installed."));
        println!();
        println!("  {}", ds::highlight("Get started:"));
        println!(
            "  {} {}",
            ds::bullet_icon(),
            ds::command("spn add @workflows/dev-productivity/code-review")
        );
        println!(
            "  {} {}",
            ds::bullet_icon(),
            ds::command("spn skill add brainstorming")
        );
        return Ok(());
    }

    // Show manifest dependencies
    if let Some(ref m) = manifest {
        if !m.dependencies.is_empty() {
            println!("  {}", ds::muted("From spn.yaml:"));
            for (name, dep) in &m.dependencies {
                let version = dep.version();
                let status = if installed.iter().any(|p| &p.name == name) {
                    ds::success(ds::icon::SUCCESS)
                } else {
                    ds::muted("○")
                };
                println!(
                    "  {} {} @ {}",
                    status,
                    ds::package(name),
                    ds::version(version)
                );
            }
            println!();
        }
    }

    // Show installed packages from storage
    if !installed.is_empty() {
        println!("  {}", ds::muted("In ~/.spn/packages:"));
        for pkg in &installed {
            let in_manifest = manifest
                .as_ref()
                .is_some_and(|m| m.dependencies.contains_key(&pkg.name));

            let status = if in_manifest {
                ds::success(ds::icon::SUCCESS)
            } else {
                ds::warning("⚡")
            };
            println!(
                "  {} {} @ {}",
                status,
                ds::package(&pkg.name),
                ds::version(&pkg.version)
            );
        }
        println!();
    }

    // Legend
    println!(
        "  {} {} = in manifest & installed",
        ds::success(ds::icon::SUCCESS),
        ds::muted("")
    );
    println!("  {} = in manifest, not installed", ds::muted("○"));
    println!(
        "  {} = installed directly (not in manifest)",
        ds::warning("⚡")
    );

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
