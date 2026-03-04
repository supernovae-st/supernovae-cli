//! Add command implementation.
//!
//! Adds a package to the project's spn.yaml manifest and installs it.

use colored::Colorize;

use crate::error::{Result, SpnError};
use crate::index::{Downloader, IndexClient};
use crate::manifest::{ResolvedPackage, SpnLockfile, SpnManifest};
use crate::storage::LocalStorage;

/// Package types in the registry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageType {
    Workflow,
    Agent,
    Skill,
    Prompt,
    Job,
    Schema,
}

impl PackageType {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "workflow" => Some(Self::Workflow),
            "agent" => Some(Self::Agent),
            "skill" => Some(Self::Skill),
            "prompt" => Some(Self::Prompt),
            "job" => Some(Self::Job),
            "schema" => Some(Self::Schema),
            _ => None,
        }
    }

    /// Infer type from package scope.
    pub fn from_scope(package: &str) -> Option<Self> {
        if package.starts_with("@workflows/") || package.starts_with("@nika/") {
            Some(Self::Workflow)
        } else if package.starts_with("@agents/") {
            Some(Self::Agent)
        } else if package.starts_with("@skills/") {
            Some(Self::Skill)
        } else if package.starts_with("@prompts/") {
            Some(Self::Prompt)
        } else if package.starts_with("@jobs/") {
            Some(Self::Job)
        } else if package.starts_with("@schemas/") || package.starts_with("@novanet/") {
            Some(Self::Schema)
        } else {
            None
        }
    }
}

/// Options for the add command.
pub struct AddOptions {
    /// Package name (e.g., "@workflows/dev-productivity/code-review").
    pub package: String,

    /// Optional version constraint (e.g., "^0.1", "1.0.0").
    pub version: Option<String>,

    /// Package type (workflow, agent, skill, prompt, job, schema).
    pub package_type: Option<PackageType>,

    /// Add as dev dependency.
    pub dev: bool,

    /// Skip installation (just update manifest).
    pub manifest_only: bool,
}

/// Run the add command.
pub async fn run(package: &str, pkg_type: Option<&str>) -> Result<()> {
    // Parse or infer package type
    let package_type = pkg_type
        .and_then(PackageType::from_str)
        .or_else(|| PackageType::from_scope(package));

    if let Some(ref pt) = package_type {
        println!("   {} Type: {:?}", "→".blue(), pt);
    }

    let options = AddOptions {
        package: package.to_string(),
        version: None,
        package_type,
        dev: false,
        manifest_only: false,
    };

    run_with_options(options).await
}

/// Run the add command with full options.
pub async fn run_with_options(options: AddOptions) -> Result<()> {
    println!(
        "{} Adding package: {}",
        "📦".cyan(),
        options.package.green()
    );

    // 1. Load or create manifest
    let manifest_path = find_manifest()?;
    let mut manifest = if manifest_path.exists() {
        SpnManifest::from_file(&manifest_path).map_err(|_| SpnError::ManifestNotFound)?
    } else {
        println!("   {} Creating new spn.yaml", "→".blue());
        SpnManifest::default()
    };

    // 2. Fetch package info from registry
    let client = IndexClient::new();
    let entry = if let Some(ref version) = options.version {
        client.fetch_version(&options.package, version).await
    } else {
        client.fetch_latest(&options.package).await
    }
    .map_err(|e| SpnError::PackageNotFound(format!("{}: {}", options.package, e)))?;

    let version_constraint = options
        .version
        .clone()
        .unwrap_or_else(|| format!("^{}", entry.version));

    println!(
        "   {} Found {}@{}",
        "✓".green(),
        options.package,
        entry.version
    );

    // 3. Add to manifest
    if options.dev {
        manifest.dev_dependencies.insert(
            options.package.clone(),
            crate::manifest::Dependency::Simple(version_constraint.clone()),
        );
        println!("   {} Added to dev-dependencies", "→".blue());
    } else {
        manifest.add_dependency(&options.package, &version_constraint);
        println!("   {} Added to dependencies", "→".blue());
    }

    // 4. Save manifest
    manifest
        .write_to_file(&manifest_path)
        .map_err(|e| SpnError::ConfigError(format!("Failed to save manifest: {}", e)))?;
    println!("   {} Updated spn.yaml", "✓".green());

    // 5. Install package (unless manifest-only)
    if !options.manifest_only {
        let storage = LocalStorage::new()
            .map_err(|e| SpnError::ConfigError(format!("Storage error: {}", e)))?;

        let downloader = Downloader::new();
        let downloaded = downloader
            .download_entry(&entry)
            .await
            .map_err(|e| SpnError::ConfigError(format!("Download failed: {}", e)))?;

        println!(
            "   {} Downloaded {}",
            "✓".green(),
            downloaded.tarball_path.display()
        );

        let installed = storage
            .install(&downloaded)
            .map_err(|e| SpnError::ConfigError(format!("Install failed: {}", e)))?;

        println!(
            "   {} Installed to {}",
            "✓".green(),
            installed.path.display()
        );

        // 6. Update lockfile
        let lockfile_path = manifest_path
            .parent()
            .map(|p| p.join("spn.lock"))
            .unwrap_or_else(|| std::path::PathBuf::from("spn.lock"));

        let mut lockfile =
            SpnLockfile::find_in_dir(manifest_path.parent().unwrap_or(std::path::Path::new(".")))
                .unwrap_or_else(|_| SpnLockfile::new());

        lockfile.add_package(ResolvedPackage::new(
            &options.package,
            &entry.version,
            &entry.cksum,
        ));

        lockfile
            .write_to_file(&lockfile_path)
            .map_err(|e| SpnError::ConfigError(format!("Failed to save lockfile: {}", e)))?;

        println!("   {} Updated spn.lock", "✓".green());
    }

    println!(
        "{} Successfully added {}",
        "✨".yellow(),
        options.package.green()
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

    #[tokio::test]
    async fn test_add_options_defaults() {
        let options = AddOptions {
            package: "@test/pkg".to_string(),
            version: None,
            package_type: None,
            dev: false,
            manifest_only: false,
        };

        assert_eq!(options.package, "@test/pkg");
        assert!(options.version.is_none());
        assert!(options.package_type.is_none());
        assert!(!options.dev);
    }

    #[test]
    fn test_package_type_from_str() {
        assert_eq!(
            PackageType::from_str("workflow"),
            Some(PackageType::Workflow)
        );
        assert_eq!(PackageType::from_str("agent"), Some(PackageType::Agent));
        assert_eq!(PackageType::from_str("skill"), Some(PackageType::Skill));
        assert_eq!(PackageType::from_str("prompt"), Some(PackageType::Prompt));
        assert_eq!(PackageType::from_str("job"), Some(PackageType::Job));
        assert_eq!(PackageType::from_str("schema"), Some(PackageType::Schema));
        assert_eq!(PackageType::from_str("unknown"), None);
    }

    #[test]
    fn test_package_type_from_scope() {
        assert_eq!(
            PackageType::from_scope("@workflows/test"),
            Some(PackageType::Workflow)
        );
        assert_eq!(
            PackageType::from_scope("@agents/test"),
            Some(PackageType::Agent)
        );
        assert_eq!(
            PackageType::from_scope("@skills/test"),
            Some(PackageType::Skill)
        );
        assert_eq!(
            PackageType::from_scope("@prompts/test"),
            Some(PackageType::Prompt)
        );
        assert_eq!(
            PackageType::from_scope("@jobs/test"),
            Some(PackageType::Job)
        );
        assert_eq!(
            PackageType::from_scope("@schemas/test"),
            Some(PackageType::Schema)
        );
        assert_eq!(
            PackageType::from_scope("@novanet/test"),
            Some(PackageType::Schema)
        );
        assert_eq!(
            PackageType::from_scope("@nika/test"),
            Some(PackageType::Workflow)
        );
        assert_eq!(PackageType::from_scope("unknown"), None);
    }
}
