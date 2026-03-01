//! Version command implementation.
//!
//! Bumps package version in spn.yaml.

use std::env;

use crate::error::{CliError, Result};
use crate::manifest::SpnManifest;

pub async fn run(bump: &str) -> Result<()> {
    let cwd = env::current_dir()?;

    // Find and load manifest
    let manifest_path = cwd.join("spn.yaml");
    if !manifest_path.exists() {
        let spn_manifest_path = cwd.join(".spn").join("spn.yaml");
        if spn_manifest_path.exists() {
            return bump_version(&spn_manifest_path, bump).await;
        }
        return Err(CliError::NotFound("spn.yaml not found".to_string()));
    }

    bump_version(&manifest_path, bump).await
}

async fn bump_version(manifest_path: &std::path::Path, bump: &str) -> Result<()> {
    let mut manifest = SpnManifest::from_file(manifest_path)?;

    // Parse current version
    let current = semver::Version::parse(&manifest.version).map_err(|e| {
        CliError::InvalidInput(format!("Invalid version in manifest: {}", e))
    })?;

    // Calculate new version
    let new_version = match bump.to_lowercase().as_str() {
        "major" => semver::Version::new(current.major + 1, 0, 0),
        "minor" => semver::Version::new(current.major, current.minor + 1, 0),
        "patch" => semver::Version::new(current.major, current.minor, current.patch + 1),
        "premajor" => {
            let mut v = semver::Version::new(current.major + 1, 0, 0);
            v.pre = semver::Prerelease::new("alpha.0").unwrap();
            v
        }
        "preminor" => {
            let mut v = semver::Version::new(current.major, current.minor + 1, 0);
            v.pre = semver::Prerelease::new("alpha.0").unwrap();
            v
        }
        "prepatch" => {
            let mut v = semver::Version::new(current.major, current.minor, current.patch + 1);
            v.pre = semver::Prerelease::new("alpha.0").unwrap();
            v
        }
        "prerelease" => {
            if current.pre.is_empty() {
                return Err(CliError::InvalidInput(
                    "Cannot bump prerelease on a stable version. Use premajor/preminor/prepatch.".to_string(),
                ));
            }
            // Increment prerelease number
            let pre_str = current.pre.as_str();
            if let Some(pos) = pre_str.rfind('.') {
                let prefix = &pre_str[..pos];
                let num: u64 = pre_str[pos + 1..].parse().unwrap_or(0);
                let mut v = current.clone();
                v.pre = semver::Prerelease::new(&format!("{}.{}", prefix, num + 1)).unwrap();
                v
            } else {
                let mut v = current.clone();
                v.pre = semver::Prerelease::new(&format!("{}.1", pre_str)).unwrap();
                v
            }
        }
        other => {
            // Try to parse as exact version
            match semver::Version::parse(other) {
                Ok(v) => v,
                Err(_) => {
                    return Err(CliError::InvalidInput(format!(
                        "Invalid bump type: {}. Use major, minor, patch, premajor, preminor, prepatch, prerelease, or a valid semver.",
                        other
                    )));
                }
            }
        }
    };

    let old_version = manifest.version.clone();
    manifest.version = new_version.to_string();

    // Write updated manifest
    manifest.write_to_file(manifest_path)?;

    println!("🔢 Version bumped: {} → {}", old_version, manifest.version);
    println!();
    println!("   Next steps:");
    println!("   • git add spn.yaml");
    println!("   • git commit -m \"chore: bump version to {}\"", manifest.version);
    println!("   • git tag v{}", manifest.version);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manifest(dir: &std::path::Path, version: &str) {
        let manifest = SpnManifest {
            name: "test".to_string(),
            version: version.to_string(),
            ..Default::default()
        };
        manifest.write_to_file(dir.join("spn.yaml")).unwrap();
    }

    #[tokio::test]
    async fn test_bump_major() {
        let temp = TempDir::new().unwrap();
        create_test_manifest(temp.path(), "1.2.3");

        let result = bump_version(&temp.path().join("spn.yaml"), "major").await;
        assert!(result.is_ok());

        let manifest = SpnManifest::from_file(temp.path().join("spn.yaml")).unwrap();
        assert_eq!(manifest.version, "2.0.0");
    }

    #[tokio::test]
    async fn test_bump_minor() {
        let temp = TempDir::new().unwrap();
        create_test_manifest(temp.path(), "1.2.3");

        let result = bump_version(&temp.path().join("spn.yaml"), "minor").await;
        assert!(result.is_ok());

        let manifest = SpnManifest::from_file(temp.path().join("spn.yaml")).unwrap();
        assert_eq!(manifest.version, "1.3.0");
    }

    #[tokio::test]
    async fn test_bump_patch() {
        let temp = TempDir::new().unwrap();
        create_test_manifest(temp.path(), "1.2.3");

        let result = bump_version(&temp.path().join("spn.yaml"), "patch").await;
        assert!(result.is_ok());

        let manifest = SpnManifest::from_file(temp.path().join("spn.yaml")).unwrap();
        assert_eq!(manifest.version, "1.2.4");
    }

    #[tokio::test]
    async fn test_bump_exact_version() {
        let temp = TempDir::new().unwrap();
        create_test_manifest(temp.path(), "1.0.0");

        let result = bump_version(&temp.path().join("spn.yaml"), "2.5.0").await;
        assert!(result.is_ok());

        let manifest = SpnManifest::from_file(temp.path().join("spn.yaml")).unwrap();
        assert_eq!(manifest.version, "2.5.0");
    }
}
