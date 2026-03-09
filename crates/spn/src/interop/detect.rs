//! Ecosystem tool detection and auto-install support.
//!
//! Detects Nika and NovaNet installations, provides version checking,
//! and supports on-demand installation prompts.
//!
//! # Usage
//!
//! ```ignore
//! let tools = EcosystemTools::detect();
//!
//! if !tools.nika.is_installed() {
//!     // Prompt user to install
//! }
//! ```

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use indicatif::{ProgressBar, ProgressStyle};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use thiserror::Error;

/// Install status for an ecosystem tool.
#[derive(Debug, Clone, PartialEq)]
pub enum InstallStatus {
    /// Tool is installed with version and path.
    Installed { version: String, path: PathBuf },
    /// Tool is not installed.
    NotInstalled,
    /// Tool is outdated (current version < latest).
    #[allow(dead_code)]
    Outdated { current: String, latest: String },
}

impl InstallStatus {
    /// Returns true if the tool is installed (any version).
    pub fn is_installed(&self) -> bool {
        matches!(
            self,
            InstallStatus::Installed { .. } | InstallStatus::Outdated { .. }
        )
    }

    /// Returns the version if installed.
    #[allow(dead_code)]
    pub fn version(&self) -> Option<&str> {
        match self {
            InstallStatus::Installed { version, .. } => Some(version),
            InstallStatus::Outdated { current, .. } => Some(current),
            InstallStatus::NotInstalled => None,
        }
    }

    /// Returns the path if installed.
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            InstallStatus::Installed { path, .. } => Some(path),
            InstallStatus::Outdated { .. } => None,
            InstallStatus::NotInstalled => None,
        }
    }
}

/// Detected state of ecosystem tools.
#[derive(Debug, Clone)]
pub struct EcosystemTools {
    /// Nika workflow engine status.
    pub nika: InstallStatus,
    /// NovaNet knowledge graph CLI status.
    pub novanet: InstallStatus,
}

impl EcosystemTools {
    /// Detect installed ecosystem tools.
    pub fn detect() -> Self {
        Self {
            nika: detect_nika(),
            novanet: detect_novanet(),
        }
    }

    /// Check if all tools are installed.
    pub fn all_installed(&self) -> bool {
        self.nika.is_installed() && self.novanet.is_installed()
    }

    /// Get list of missing tools.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.nika.is_installed() {
            missing.push("nika");
        }
        if !self.novanet.is_installed() {
            missing.push("novanet");
        }
        missing
    }
}

/// Detect Nika installation.
fn detect_nika() -> InstallStatus {
    detect_binary("nika")
}

/// Detect NovaNet installation.
fn detect_novanet() -> InstallStatus {
    detect_binary("novanet")
}

/// Generic binary detection.
fn detect_binary(name: &str) -> InstallStatus {
    // Try to find the binary in PATH
    match which::which(name) {
        Ok(path) => {
            // Get version
            let version = get_binary_version(&path);
            let canonical_path = path.canonicalize().unwrap_or(path);

            InstallStatus::Installed {
                version,
                path: canonical_path,
            }
        }
        Err(_) => {
            // Check ~/.spn/bin/ as fallback
            if let Ok(paths) = spn_client::SpnPaths::new() {
                let spn_bin = paths.binary(name);
                if spn_bin.exists() {
                    let version = get_binary_version(&spn_bin);
                    let canonical_path = spn_bin.canonicalize().unwrap_or(spn_bin);
                    return InstallStatus::Installed {
                        version,
                        path: canonical_path,
                    };
                }
            }

            // Check Homebrew paths on macOS
            #[cfg(target_os = "macos")]
            {
                let homebrew_paths = [
                    PathBuf::from("/opt/homebrew/bin").join(name),
                    PathBuf::from("/usr/local/bin").join(name),
                ];
                for path in homebrew_paths {
                    if path.exists() {
                        let version = get_binary_version(&path);
                        let canonical_path = path.canonicalize().unwrap_or(path);
                        return InstallStatus::Installed {
                            version,
                            path: canonical_path,
                        };
                    }
                }
            }

            InstallStatus::NotInstalled
        }
    }
}

/// Get version string from binary.
fn get_binary_version(path: &PathBuf) -> String {
    Command::new(path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| {
            // Parse version from output like "nika 0.21.1" or "novanet 0.17.2"
            s.split_whitespace().nth(1).unwrap_or(s.trim()).to_string()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Errors that can occur during installation.
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum InstallError {
    #[error("Installation failed for {tool}: {message}")]
    InstallFailed { tool: String, message: String },

    #[error("Missing tool: {0}. Run `spn setup {0}` to install.")]
    MissingTool(String),

    #[error("No suitable installation method found (cargo or brew required)")]
    NoInstallMethod,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Failed to extract archive: {0}")]
    ExtractError(String),

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("User cancelled installation")]
    Cancelled,
}

/// Install method preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    /// Use cargo install.
    Cargo,
    /// Use Homebrew.
    Brew,
    /// Download pre-built binary.
    Binary,
}

impl InstallMethod {
    /// Get the best available install method.
    pub fn best_available() -> Option<Self> {
        // Prefer cargo if available
        if Command::new("cargo").arg("--version").output().is_ok() {
            return Some(InstallMethod::Cargo);
        }

        // Fall back to brew on macOS
        if Command::new("brew").arg("--version").output().is_ok() {
            return Some(InstallMethod::Brew);
        }

        // Binary download is always available
        Some(InstallMethod::Binary)
    }

    /// Display name for the method.
    pub fn display_name(&self) -> &'static str {
        match self {
            InstallMethod::Cargo => "cargo install",
            InstallMethod::Brew => "Homebrew",
            InstallMethod::Binary => "Direct download",
        }
    }
}

/// Get the target triple for the current platform.
fn get_target_triple() -> Result<&'static str, InstallError> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok("aarch64-apple-darwin");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok("x86_64-apple-darwin");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok("x86_64-unknown-linux-gnu");

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Ok("aarch64-unknown-linux-gnu");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok("x86_64-pc-windows-msvc");

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return Err(InstallError::UnsupportedPlatform(format!(
        "{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )));
}

/// Build HTTP client with retry middleware.
fn build_http_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let retry_middleware = RetryTransientMiddleware::new_with_policy(retry_policy);

    ClientBuilder::new(reqwest::Client::new())
        .with(retry_middleware)
        .build()
}

/// Download and install a binary from GitHub releases.
///
/// Downloads from `https://github.com/{org}/{repo}/releases/latest/download/{binary}-{target}.tar.gz`
/// and installs to `~/.spn/bin/{binary}`.
async fn download_and_install_binary(
    tool_name: &str,
    github_org: &str,
    github_repo: &str,
) -> Result<(), InstallError> {
    use futures_util::StreamExt;

    let target = get_target_triple()?;

    // Get install directory
    let paths = spn_client::SpnPaths::new().map_err(|e| InstallError::InstallFailed {
        tool: tool_name.to_string(),
        message: format!("Failed to get spn paths: {}", e),
    })?;

    let bin_dir = paths.bin_dir();
    std::fs::create_dir_all(&bin_dir)?;

    // Build download URL
    let tarball_name = format!("{}-{}.tar.gz", tool_name, target);
    let url = format!(
        "https://github.com/{}/{}/releases/latest/download/{}",
        github_org, github_repo, tarball_name
    );

    // Create temp file for download
    let temp_dir = tempfile::tempdir()?;
    let tarball_path = temp_dir.path().join(&tarball_name);

    // Download with progress bar
    let client = build_http_client();
    let response = client
        .get(&url)
        .header(
            "User-Agent",
            format!("spn/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|e| InstallError::Http(e.to_string()))?;

    if !response.status().is_success() {
        return Err(InstallError::Http(format!(
            "HTTP {}: {}",
            response.status(),
            url
        )));
    }

    let total_size = response.content_length();
    let pb = if let Some(size) = total_size {
        let pb = ProgressBar::new(size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "📥 Downloading {msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(tool_name.to_string());
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} 📥 Downloading {msg} {bytes}")
                .unwrap(),
        );
        pb.set_message(tool_name.to_string());
        pb
    };

    // Stream download to file
    let mut file = std::fs::File::create(&tarball_path)?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| InstallError::Http(e.to_string()))?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!("✅ Downloaded {}", tool_name));

    // Extract tarball
    let extract_pb = ProgressBar::new_spinner();
    extract_pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} 📦 Extracting {msg}")
            .unwrap(),
    );
    extract_pb.set_message(tool_name.to_string());
    extract_pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let tarball_file = std::fs::File::open(&tarball_path)?;
    let decoder = flate2::read::GzDecoder::new(tarball_file);
    let mut archive = tar::Archive::new(decoder);

    // Extract to temp directory first
    let extract_dir = temp_dir.path().join("extract");
    std::fs::create_dir_all(&extract_dir)?;
    archive
        .unpack(&extract_dir)
        .map_err(|e| InstallError::ExtractError(e.to_string()))?;

    extract_pb.finish_with_message(format!("✅ Extracted {}", tool_name));

    // Find the binary in extracted files (could be at root or in a directory)
    let binary_name = if cfg!(windows) {
        format!("{}.exe", tool_name)
    } else {
        tool_name.to_string()
    };

    let binary_src = find_binary_in_dir(&extract_dir, &binary_name).ok_or_else(|| {
        InstallError::ExtractError(format!("Binary '{}' not found in archive", binary_name))
    })?;

    // Copy to bin directory
    let binary_dest = bin_dir.join(&binary_name);
    std::fs::copy(&binary_src, &binary_dest)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_dest, perms)?;
    }

    println!("✅ Installed {} to {}", tool_name, binary_dest.display());

    Ok(())
}

/// Find a binary in a directory (recursively, but shallow).
fn find_binary_in_dir(dir: &std::path::Path, binary_name: &str) -> Option<PathBuf> {
    // Check root first
    let root_path = dir.join(binary_name);
    if root_path.exists() && root_path.is_file() {
        return Some(root_path);
    }

    // Check one level deep
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested = path.join(binary_name);
                if nested.exists() && nested.is_file() {
                    return Some(nested);
                }
            }
        }
    }

    None
}

/// Install Nika workflow engine.
pub async fn install_nika(method: InstallMethod) -> Result<(), InstallError> {
    match method {
        InstallMethod::Cargo => {
            let status = Command::new("cargo")
                .args(["install", "nika-cli", "--locked"])
                .status()?;

            if !status.success() {
                return Err(InstallError::InstallFailed {
                    tool: "nika".into(),
                    message: format!("cargo install failed with exit code {:?}", status.code()),
                });
            }
        }
        InstallMethod::Brew => {
            let status = Command::new("brew")
                .args(["install", "supernovae-st/tap/nika"])
                .status()?;

            if !status.success() {
                return Err(InstallError::InstallFailed {
                    tool: "nika".into(),
                    message: format!("brew install failed with exit code {:?}", status.code()),
                });
            }
        }
        InstallMethod::Binary => {
            download_and_install_binary("nika", "supernovae-st", "nika").await?;
        }
    }

    Ok(())
}

/// Install NovaNet CLI.
pub async fn install_novanet(method: InstallMethod) -> Result<(), InstallError> {
    match method {
        InstallMethod::Cargo => {
            let status = Command::new("cargo")
                .args(["install", "novanet-cli", "--locked"])
                .status()?;

            if !status.success() {
                return Err(InstallError::InstallFailed {
                    tool: "novanet".into(),
                    message: format!("cargo install failed with exit code {:?}", status.code()),
                });
            }
        }
        InstallMethod::Brew => {
            let status = Command::new("brew")
                .args(["install", "supernovae-st/tap/novanet"])
                .status()?;

            if !status.success() {
                return Err(InstallError::InstallFailed {
                    tool: "novanet".into(),
                    message: format!("brew install failed with exit code {:?}", status.code()),
                });
            }
        }
        InstallMethod::Binary => {
            download_and_install_binary("novanet", "supernovae-st", "novanet").await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_status_is_installed() {
        let installed = InstallStatus::Installed {
            version: "1.0.0".into(),
            path: PathBuf::from("/usr/bin/test"),
        };
        assert!(installed.is_installed());

        let not_installed = InstallStatus::NotInstalled;
        assert!(!not_installed.is_installed());

        let outdated = InstallStatus::Outdated {
            current: "0.9.0".into(),
            latest: "1.0.0".into(),
        };
        assert!(outdated.is_installed());
    }

    #[test]
    fn test_install_status_version() {
        let installed = InstallStatus::Installed {
            version: "1.0.0".into(),
            path: PathBuf::from("/usr/bin/test"),
        };
        assert_eq!(installed.version(), Some("1.0.0"));

        let not_installed = InstallStatus::NotInstalled;
        assert_eq!(not_installed.version(), None);
    }

    #[test]
    fn test_ecosystem_tools_detect() {
        // Should not panic
        let tools = EcosystemTools::detect();
        // Missing list should be consistent with individual checks
        let missing = tools.missing();
        if !tools.nika.is_installed() {
            assert!(missing.contains(&"nika"));
        }
        if !tools.novanet.is_installed() {
            assert!(missing.contains(&"novanet"));
        }
    }

    #[test]
    fn test_install_method_best_available() {
        // Should always return Some (at least Binary is available)
        let method = InstallMethod::best_available();
        assert!(method.is_some());
    }

    #[test]
    fn test_install_method_display_name() {
        assert_eq!(InstallMethod::Cargo.display_name(), "cargo install");
        assert_eq!(InstallMethod::Brew.display_name(), "Homebrew");
        assert_eq!(InstallMethod::Binary.display_name(), "Direct download");
    }

    #[test]
    fn test_install_error_display() {
        let err = InstallError::MissingTool("nika".into());
        assert!(err.to_string().contains("nika"));
        assert!(err.to_string().contains("spn setup"));
    }
}
