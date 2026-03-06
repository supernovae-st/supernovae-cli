//! Publish command implementation.
//!
//! Validates and publishes packages to the SuperNovae registry.

use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::{CliError, Result, SpnError};
use crate::manifest::SpnManifest;

/// Files/directories to always exclude from tarball.
const EXCLUDE_DIRS: &[&str] = &[
    ".git",
    ".spn",
    ".claude",
    ".nika",
    "node_modules",
    "target",
    ".vscode",
    ".idea",
    "__pycache__",
    ".pytest_cache",
];

/// File patterns to exclude from tarball.
const EXCLUDE_PATTERNS: &[&str] = &[
    ".env",
    ".env.local",
    ".local.yaml",
    ".DS_Store",
    "Thumbs.db",
    ".gitignore",
];

/// Created tarball info.
#[derive(Debug)]
pub struct TarballInfo {
    /// Path to the created tarball.
    pub path: PathBuf,
    /// SHA256 checksum (hex encoded).
    pub checksum: String,
    /// Size in bytes.
    pub size: u64,
    /// Number of files included.
    pub file_count: usize,
}

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

async fn publish_package(dir: &Path, manifest: &SpnManifest) -> Result<()> {
    println!("📤 Publishing {}@{}...\n", manifest.name, manifest.version);

    // Validate first
    println!("   Validating package...");

    if manifest.name.is_empty() || manifest.version.is_empty() {
        return Err(CliError::InvalidInput(
            "Package name and version are required".to_string(),
        ));
    }

    // Check if this is a valid scope
    let valid_scopes = ["@nika", "@novanet", "@workflows", "@shared", "@agents", "@skills", "@jobs", "@prompts", "@schemas"];
    let scope = manifest.name.split('/').next().unwrap_or("");

    if !valid_scopes.contains(&scope) {
        println!(
            "   ⚠️  Scope '{}' is not a standard SuperNovae scope",
            scope
        );
        println!("   Standard scopes: @nika, @novanet, @workflows, @shared");
    }

    // Step 1: Create tarball
    println!("   Creating tarball...");
    let tarball = create_tarball(dir, manifest)?;
    println!(
        "   ✓ Tarball created ({} files, {} bytes)",
        tarball.file_count, tarball.size
    );
    println!("   ✓ Checksum: sha256:{}", &tarball.checksum[..16]);

    // Step 2: Git-based publishing workflow
    println!();
    println!("   📋 Git workflow:");

    // Check for gh CLI
    if which::which("gh").is_err() {
        println!();
        println!("   ❌ GitHub CLI not found.");
        println!();
        println!("   Install it with: brew install gh");
        println!("   Then authenticate: gh auth login");
        println!();
        println!("   Manual workflow:");
        println!("   1. Fork supernovae-registry");
        println!("   2. Copy tarball to releases/{}/", get_index_path(&manifest.name));
        println!("   3. Update index/{}", get_index_path(&manifest.name));
        println!("   4. Submit a pull request");
        println!();
        println!("   Tarball location: {}", tarball.path.display());
        return Ok(());
    }

    // Try automated git workflow
    match git_publish_workflow(dir, manifest, &tarball).await {
        Ok(pr_url) => {
            println!();
            println!("   🎉 Pull request created!");
            println!("   🔗 {}", pr_url);
        }
        Err(e) => {
            // Fall back to manual instructions
            println!();
            println!("   ⚠️  Automated workflow failed: {}", e);
            println!();
            println!("   Manual steps:");
            println!(
                "   1. cd ~/path/to/supernovae-registry"
            );
            println!(
                "   2. git checkout -b publish/{}/{}",
                manifest.name.replace('@', ""),
                manifest.version
            );
            println!("   3. mkdir -p releases/{}", get_index_path(&manifest.name));
            println!(
                "   4. cp {} releases/{}/{}.tar.gz",
                tarball.path.display(),
                get_index_path(&manifest.name),
                manifest.version
            );
            println!("   5. Update index/{}", get_index_path(&manifest.name));
            println!(
                "   6. git add . && git commit -m 'feat: publish {}@{}'",
                manifest.name, manifest.version
            );
            println!("   7. git push && gh pr create");
        }
    }
    println!();
    println!("   Tarball: {}", tarball.path.display());

    Ok(())
}

/// Git-based publishing workflow.
/// Clones registry fork, creates branch, adds tarball, updates index, and opens PR.
async fn git_publish_workflow(
    _dir: &Path,
    manifest: &SpnManifest,
    tarball: &TarballInfo,
) -> Result<String> {
    use std::process::Command;

    // Check for local registry clone in common locations
    let registry_paths = [
        dirs::home_dir()
            .unwrap_or_default()
            .join("dev/supernovae/supernovae-registry"),
        dirs::home_dir()
            .unwrap_or_default()
            .join("supernovae-registry"),
        dirs::home_dir()
            .unwrap_or_default()
            .join("projects/supernovae-registry"),
    ];

    let registry_path = registry_paths
        .iter()
        .find(|p| p.exists() && p.join(".git").exists())
        .ok_or_else(|| {
            SpnError::ConfigError(
                "Registry not found. Clone supernovae-registry to ~/dev/supernovae/ or ~/".into(),
            )
        })?;

    println!("   ✓ Found registry at {}", registry_path.display());

    // Ensure we're on main/master and pull latest
    println!("   Updating registry...");
    let status = Command::new("git")
        .args(["checkout", "main"])
        .current_dir(registry_path)
        .output()
        .map_err(|e| SpnError::CommandFailed(format!("git checkout: {}", e)))?;

    if !status.status.success() {
        // Try master if main doesn't exist
        Command::new("git")
            .args(["checkout", "master"])
            .current_dir(registry_path)
            .output()
            .map_err(|e| SpnError::CommandFailed(format!("git checkout: {}", e)))?;
    }

    Command::new("git")
        .args(["pull", "--rebase"])
        .current_dir(registry_path)
        .output()
        .map_err(|e| SpnError::CommandFailed(format!("git pull: {}", e)))?;

    // Create branch
    let branch_name = format!(
        "publish/{}/{}",
        manifest.name.replace('@', "").replace('/', "-"),
        manifest.version
    );
    println!("   Creating branch {}...", branch_name);

    let status = Command::new("git")
        .args(["checkout", "-b", &branch_name])
        .current_dir(registry_path)
        .output()
        .map_err(|e| SpnError::CommandFailed(format!("git checkout -b: {}", e)))?;

    if !status.status.success() {
        // Branch might already exist, try switching
        Command::new("git")
            .args(["checkout", &branch_name])
            .current_dir(registry_path)
            .output()
            .map_err(|e| SpnError::CommandFailed(format!("git checkout: {}", e)))?;
    }

    // Create directories and copy tarball
    let index_path = get_index_path(&manifest.name);
    let releases_dir = registry_path.join("releases").join(&index_path);
    let index_dir = registry_path.join("index").join(&index_path).parent().unwrap().to_path_buf();
    let index_file = registry_path.join("index").join(&index_path);

    println!("   Creating directories...");
    std::fs::create_dir_all(&releases_dir)
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to create releases dir: {}", e)))?;
    std::fs::create_dir_all(&index_dir)
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to create index dir: {}", e)))?;

    // Copy tarball
    let tarball_dest = releases_dir.join(format!("{}.tar.gz", manifest.version));
    println!("   Copying tarball...");
    std::fs::copy(&tarball.path, &tarball_dest)
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to copy tarball: {}", e)))?;

    // Update index file (NDJSON format)
    println!("   Updating index...");
    let index_entry = serde_json::json!({
        "name": manifest.name,
        "version": manifest.version,
        "checksum": format!("sha256:{}", tarball.checksum),
        "yanked": false
    });

    // Append to index file (or create if doesn't exist)
    let mut index_content = if index_file.exists() {
        std::fs::read_to_string(&index_file).unwrap_or_default()
    } else {
        String::new()
    };

    // Add newline if file doesn't end with one
    if !index_content.is_empty() && !index_content.ends_with('\n') {
        index_content.push('\n');
    }

    index_content.push_str(&serde_json::to_string(&index_entry).unwrap());
    index_content.push('\n');

    std::fs::write(&index_file, index_content)
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to write index: {}", e)))?;

    // Git add and commit
    println!("   Committing changes...");
    Command::new("git")
        .args(["add", "."])
        .current_dir(registry_path)
        .output()
        .map_err(|e| SpnError::CommandFailed(format!("git add: {}", e)))?;

    let commit_msg = format!(
        "feat(publish): {}@{}\n\nCo-Authored-By: spn <noreply@supernovae.studio>",
        manifest.name, manifest.version
    );

    Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(registry_path)
        .output()
        .map_err(|e| SpnError::CommandFailed(format!("git commit: {}", e)))?;

    // Push to origin
    println!("   Pushing to origin...");
    let push_status = Command::new("git")
        .args(["push", "-u", "origin", &branch_name])
        .current_dir(registry_path)
        .output()
        .map_err(|e| SpnError::CommandFailed(format!("git push: {}", e)))?;

    if !push_status.status.success() {
        let stderr = String::from_utf8_lossy(&push_status.stderr);
        return Err(SpnError::CommandFailed(format!(
            "git push failed: {}",
            stderr
        )));
    }

    // Create PR via gh CLI
    println!("   Creating pull request...");
    let pr_body = format!(
        "## Package Publish\n\n- **Package:** {}\n- **Version:** {}\n- **Checksum:** sha256:{}\n\n---\nCreated by `spn publish`",
        manifest.name, manifest.version, &tarball.checksum[..16]
    );

    let pr_output = Command::new("gh")
        .args([
            "pr",
            "create",
            "--title",
            &format!("feat: publish {}@{}", manifest.name, manifest.version),
            "--body",
            &pr_body,
            "--base",
            "main",
        ])
        .current_dir(registry_path)
        .output()
        .map_err(|e| SpnError::CommandFailed(format!("gh pr create: {}", e)))?;

    if !pr_output.status.success() {
        let stderr = String::from_utf8_lossy(&pr_output.stderr);
        return Err(SpnError::CommandFailed(format!(
            "gh pr create failed: {}",
            stderr
        )));
    }

    // Extract PR URL from output
    let pr_url = String::from_utf8_lossy(&pr_output.stdout)
        .trim()
        .to_string();

    // Return to main branch
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(registry_path)
        .output()
        .ok(); // Ignore errors

    Ok(pr_url)
}

/// Create a tarball from the package directory.
fn create_tarball(dir: &Path, manifest: &SpnManifest) -> Result<TarballInfo> {
    // Create tarball in temp directory
    let temp_dir = std::env::temp_dir();
    let safe_name = manifest.name.replace('@', "").replace('/', "-");
    let tarball_name = format!("{}-{}.tar.gz", safe_name, manifest.version);
    let tarball_path = temp_dir.join(&tarball_name);

    // Create the tarball
    let file = File::create(&tarball_path)
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to create tarball: {}", e)))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);

    let mut file_count = 0;

    // Walk directory and add files
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !should_exclude(e.path(), dir))
    {
        let entry = entry.map_err(|e| SpnError::Other(anyhow::anyhow!("Walk error: {}", e)))?;
        let path = entry.path();

        // Skip the root directory itself
        if path == dir {
            continue;
        }

        // Get relative path for archive
        let rel_path = path
            .strip_prefix(dir)
            .map_err(|e| SpnError::Other(anyhow::anyhow!("Path error: {}", e)))?;

        if path.is_file() {
            // Add file to archive
            archive
                .append_path_with_name(path, rel_path)
                .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to add file: {}", e)))?;
            file_count += 1;
        } else if path.is_dir() {
            // Add directory entry
            archive
                .append_dir(rel_path, path)
                .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to add dir: {}", e)))?;
        }
    }

    // Finish archive
    let encoder = archive
        .into_inner()
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to finish archive: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to compress: {}", e)))?;

    // Calculate checksum
    let checksum = calculate_sha256(&tarball_path)?;

    // Get file size
    let metadata = std::fs::metadata(&tarball_path)
        .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to get metadata: {}", e)))?;

    Ok(TarballInfo {
        path: tarball_path,
        checksum,
        size: metadata.len(),
        file_count,
    })
}

/// Check if a path should be excluded from the tarball.
fn should_exclude(path: &Path, base: &Path) -> bool {
    // Get the relative path components
    let rel_path = match path.strip_prefix(base) {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Check directory exclusions
    for component in rel_path.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            if EXCLUDE_DIRS.iter().any(|&d| d == name_str) {
                return true;
            }
        }
    }

    // Check file pattern exclusions
    if let Some(file_name) = path.file_name().and_then(OsStr::to_str) {
        for pattern in EXCLUDE_PATTERNS {
            if file_name == *pattern || file_name.ends_with(pattern) {
                return true;
            }
        }
    }

    false
}

/// Calculate SHA256 checksum of a file.
fn calculate_sha256(path: &Path) -> Result<String> {
    let file =
        File::open(path).map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to open file: {}", e)))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| SpnError::Other(anyhow::anyhow!("Failed to read file: {}", e)))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Convert package name to index path.
/// @workflows/dev-productivity/code-review -> @w/dev-productivity/code-review
fn get_index_path(name: &str) -> String {
    // Map full scope names to short prefixes
    let name = name
        .replace("@workflows/", "@w/")
        .replace("@nika/", "@n/")
        .replace("@novanet/", "@nv/")
        .replace("@agents/", "@a/")
        .replace("@skills/", "@s/")
        .replace("@prompts/", "@p/")
        .replace("@jobs/", "@j/")
        .replace("@schemas/", "@sc/")
        .replace("@shared/", "@sh/");

    // Remove the @ prefix for path construction
    name.trim_start_matches('@').to_string()
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

    #[test]
    fn test_create_tarball() {
        let temp = TempDir::new().unwrap();

        // Create test files
        let manifest = SpnManifest {
            name: "@workflows/test/hello".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test workflow".to_string()),
            ..Default::default()
        };
        manifest.write_to_file(temp.path().join("spn.yaml")).unwrap();

        // Create workflow file
        std::fs::write(
            temp.path().join("hello.nika.yaml"),
            "name: hello\nsteps:\n  - infer: Say hello",
        )
        .unwrap();

        // Create README
        std::fs::write(temp.path().join("README.md"), "# Hello workflow").unwrap();

        // Create tarball
        let result = create_tarball(temp.path(), &manifest);
        assert!(result.is_ok());

        let tarball = result.unwrap();
        assert!(tarball.path.exists());
        assert!(tarball.size > 0);
        assert!(tarball.file_count >= 3); // spn.yaml, hello.nika.yaml, README.md
        assert_eq!(tarball.checksum.len(), 64); // SHA256 hex length

        // Verify tarball name
        assert!(tarball
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("workflows-test-hello-1.0.0"));
    }

    #[test]
    fn test_should_exclude() {
        let base = Path::new("/project");

        // Excluded directories
        assert!(should_exclude(Path::new("/project/.git"), base));
        assert!(should_exclude(Path::new("/project/.git/config"), base));
        assert!(should_exclude(Path::new("/project/node_modules/foo"), base));
        assert!(should_exclude(Path::new("/project/.spn/state.json"), base));

        // Excluded files
        assert!(should_exclude(Path::new("/project/.env"), base));
        assert!(should_exclude(Path::new("/project/.DS_Store"), base));
        assert!(should_exclude(Path::new("/project/config.local.yaml"), base));

        // Included files
        assert!(!should_exclude(Path::new("/project/spn.yaml"), base));
        assert!(!should_exclude(Path::new("/project/workflow.nika.yaml"), base));
        assert!(!should_exclude(Path::new("/project/README.md"), base));
        assert!(!should_exclude(Path::new("/project/src/main.rs"), base));
    }

    #[test]
    fn test_get_index_path() {
        assert_eq!(
            get_index_path("@workflows/dev-productivity/code-review"),
            "w/dev-productivity/code-review"
        );
        assert_eq!(get_index_path("@nika/seo-audit"), "n/seo-audit");
        assert_eq!(get_index_path("@agents/researcher"), "a/researcher");
        assert_eq!(get_index_path("@skills/tdd"), "s/tdd");
    }

    #[test]
    fn test_calculate_sha256() {
        let temp = TempDir::new().unwrap();
        let test_file = temp.path().join("test.txt");
        std::fs::write(&test_file, "hello world").unwrap();

        let checksum = calculate_sha256(&test_file).unwrap();
        // SHA256 of "hello world"
        assert_eq!(
            checksum,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
