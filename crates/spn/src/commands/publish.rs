//! Publish command implementation.
//!
//! Validates and publishes packages to the SuperNovae registry.

use std::env;
use std::path::Path;

use crate::error::{CliError, Result};
use crate::manifest::SpnManifest;

pub async fn run(dry_run: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    // Find manifest
    let manifest = SpnManifest::find_in_dir(&cwd)
        .map_err(|_| CliError::NotFound("spn.yaml not found".to_string()))?;

    if dry_run {
        validate_package(&cwd, &manifest).await
    } else {
        publish_package(&cwd, &manifest).await
    }
}

async fn validate_package(dir: &Path, manifest: &SpnManifest) -> Result<()> {
    println!("🔍 Validating package (dry run)...\n");

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check required fields
    if manifest.name.is_empty() {
        errors.push("Package name is required");
    }

    if manifest.version.is_empty() {
        errors.push("Package version is required");
    } else if semver::Version::parse(&manifest.version).is_err() {
        errors.push("Version must be valid semver (e.g., 1.0.0)");
    }

    if manifest.description.is_none() {
        warnings.push("Description is recommended for discoverability");
    }

    if manifest.license.is_none() {
        warnings.push("License is recommended");
    }

    // Check for required files
    let readme_exists = dir.join("README.md").exists() || dir.join("readme.md").exists();
    if !readme_exists {
        warnings.push("README.md is recommended");
    }

    // Check package scope
    if !manifest.name.starts_with('@') {
        warnings.push("Package name should be scoped (e.g., @nika/my-workflow)");
    }

    // Display results
    println!("   Package: {}", manifest.name);
    println!("   Version: {}", manifest.version);
    if let Some(ref desc) = manifest.description {
        println!("   Description: {}", desc);
    }
    println!();

    if !errors.is_empty() {
        println!("   ❌ Errors:");
        for err in &errors {
            println!("      • {}", err);
        }
        println!();
    }

    if !warnings.is_empty() {
        println!("   ⚠️  Warnings:");
        for warn in &warnings {
            println!("      • {}", warn);
        }
        println!();
    }

    if errors.is_empty() {
        println!("   ✅ Package is valid for publishing");
        println!();
        println!("   To publish: spn publish");
    } else {
        return Err(CliError::InvalidInput(format!(
            "Package validation failed with {} error(s)",
            errors.len()
        )));
    }

    Ok(())
}

async fn publish_package(_dir: &Path, manifest: &SpnManifest) -> Result<()> {
    println!("📤 Publishing {}@{}...\n", manifest.name, manifest.version);

    // Validate first
    println!("   Validating package...");

    if manifest.name.is_empty() || manifest.version.is_empty() {
        return Err(CliError::InvalidInput(
            "Package name and version are required".to_string(),
        ));
    }

    // Check if this is a valid scope
    let valid_scopes = ["@nika", "@novanet", "@workflows", "@shared"];
    let scope = manifest.name.split('/').next().unwrap_or("");

    if !valid_scopes.contains(&scope) {
        println!(
            "   ⚠️  Scope '{}' is not a standard SuperNovae scope",
            scope
        );
        println!("   Standard scopes: @nika, @novanet, @workflows, @shared");
    }

    // TODO: Implement actual publishing
    // 1. Create tarball
    // 2. Calculate checksum
    // 3. Upload to registry
    // 4. Update index

    println!();
    println!("   ⚠️  Publishing is not yet implemented.");
    println!();
    println!("   Current workflow:");
    println!("   1. Fork supernovae-registry");
    println!("   2. Add your package to packages/<scope>/<name>/");
    println!("   3. Update registry.json");
    println!("   4. Submit a pull request");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manifest(dir: &Path) -> SpnManifest {
        let manifest = SpnManifest {
            name: "@test/my-package".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test package".to_string()),
            license: Some("MIT".to_string()),
            ..Default::default()
        };
        manifest.write_to_file(dir.join("spn.yaml")).unwrap();
        manifest
    }

    #[tokio::test]
    async fn test_validate_valid_package() {
        let temp = TempDir::new().unwrap();
        let manifest = create_test_manifest(temp.path());

        let result = validate_package(temp.path(), &manifest).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_missing_name() {
        let temp = TempDir::new().unwrap();
        let manifest = SpnManifest {
            name: "".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };

        let result = validate_package(temp.path(), &manifest).await;
        assert!(result.is_err());
    }
}
