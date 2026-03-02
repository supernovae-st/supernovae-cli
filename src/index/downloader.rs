//! Tarball downloader with integrity verification.
//!
//! Downloads package tarballs from the registry and verifies SHA256 checksums.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::client::{IndexClient, IndexError, RegistryConfig};
use super::types::IndexEntry;

/// Errors that can occur during download.
#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Index error: {0}")]
    Index(#[from] IndexError),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Checksum mismatch for {package}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid checksum format: {0}")]
    InvalidChecksum(String),

    #[error("Failed to extract tarball: {0}")]
    ExtractError(String),
}

/// Downloaded package with verified content.
pub struct DownloadedPackage {
    /// Package name.
    pub name: String,

    /// Package version.
    pub version: String,

    /// Path to the downloaded tarball.
    pub tarball_path: PathBuf,

    /// Verified checksum.
    pub checksum: String,
}

/// Tarball downloader with integrity verification.
pub struct Downloader {
    client: IndexClient,
    cache_dir: PathBuf,
}

impl Downloader {
    /// Create a new downloader with default config.
    pub fn new() -> Self {
        Self::with_config(RegistryConfig::default())
    }

    /// Create a new downloader with custom config.
    pub fn with_config(config: RegistryConfig) -> Self {
        let cache_dir = config.cache_dir.clone();
        Self {
            client: IndexClient::with_config(config),
            cache_dir,
        }
    }

    /// Download a package by name (latest version).
    pub async fn download_latest(&self, name: &str) -> Result<DownloadedPackage, DownloadError> {
        let entry = self.client.fetch_latest(name).await?;
        self.download_entry(&entry).await
    }

    /// Download a specific version of a package.
    pub async fn download_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<DownloadedPackage, DownloadError> {
        let entry = self.client.fetch_version(name, version).await?;
        self.download_entry(&entry).await
    }

    /// Download a package from an index entry.
    pub async fn download_entry(&self, entry: &IndexEntry) -> Result<DownloadedPackage, DownloadError> {
        let tarball_url = self.client.tarball_url(&entry.name, &entry.version)?;

        // Create cache directory structure
        let tarball_path = self.tarball_cache_path(&entry.name, &entry.version);
        if let Some(parent) = tarball_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Download if not cached
        if !tarball_path.exists() {
            self.fetch_tarball(&tarball_url, &tarball_path).await?;
        }

        // Verify checksum
        self.verify_checksum(&tarball_path, &entry.cksum, &entry.name)?;

        Ok(DownloadedPackage {
            name: entry.name.clone(),
            version: entry.version.clone(),
            tarball_path,
            checksum: entry.cksum.clone(),
        })
    }

    /// Get the cache path for a tarball.
    fn tarball_cache_path(&self, name: &str, version: &str) -> PathBuf {
        // Sanitize name for filesystem
        let safe_name = name.replace('@', "").replace('/', "_");
        self.cache_dir
            .join("tarballs")
            .join(format!("{}-{}.tar.gz", safe_name, version))
    }

    /// Fetch tarball from URL (HTTP or file://).
    async fn fetch_tarball(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
        if url.starts_with("file://") {
            self.fetch_local(url, dest)
        } else {
            self.fetch_http(url, dest).await
        }
    }

    /// Fetch from local file system.
    fn fetch_local(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
        let path = url.strip_prefix("file://").unwrap_or(url);
        std::fs::copy(path, dest)?;
        Ok(())
    }

    /// Fetch from HTTP with progress bar.
    async fn fetch_http(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
        let response = reqwest::Client::new()
            .get(url)
            .header("User-Agent", "spn/0.6")
            .send()
            .await
            .map_err(|e| DownloadError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DownloadError::Http(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        // Get content length if available
        let total_size = response.content_length();

        // Create progress bar
        let pb = if let Some(size) = total_size {
            let pb = ProgressBar::new(size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            pb.set_message("📥 Downloading");
            pb
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg} {bytes}")
                    .unwrap(),
            );
            pb.set_message("📥 Downloading");
            pb
        };

        // Stream response and write to file
        let mut file = std::fs::File::create(dest)?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| DownloadError::Http(e.to_string()))?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("✅ Downloaded");

        Ok(())
    }

    /// Verify tarball checksum with progress indicator.
    fn verify_checksum(
        &self,
        path: &Path,
        expected: &str,
        package: &str,
    ) -> Result<(), DownloadError> {
        // Parse expected checksum (format: "sha256:hex")
        let expected_hex = expected
            .strip_prefix("sha256:")
            .ok_or_else(|| DownloadError::InvalidChecksum(expected.to_string()))?;

        // Get file size for progress bar
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();

        // Create progress bar
        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("🔐 Verifying checksum");

        // Compute actual checksum
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        let mut processed: u64 = 0;

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
            processed += n as u64;
            pb.set_position(processed);
        }

        pb.finish_with_message("✅ Checksum verified");

        let actual = hex::encode(hasher.finalize());

        if actual != expected_hex {
            return Err(DownloadError::ChecksumMismatch {
                package: package.to_string(),
                expected: expected_hex.to_string(),
                actual,
            });
        }

        Ok(())
    }

    /// Extract tarball to destination directory with progress.
    pub fn extract(
        &self,
        downloaded: &DownloadedPackage,
        dest: &Path,
    ) -> Result<(), DownloadError> {
        std::fs::create_dir_all(dest)?;

        // Create progress spinner
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("📦 Extracting {}@{}", downloaded.name, downloaded.version));

        let file = std::fs::File::open(&downloaded.tarball_path)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        archive
            .unpack(dest)
            .map_err(|e| DownloadError::ExtractError(e.to_string()))?;

        pb.finish_with_message(format!("✅ Extracted {}@{}", downloaded.name, downloaded.version));

        Ok(())
    }

    /// Clear the download cache.
    pub fn clear_cache(&self) -> Result<(), DownloadError> {
        let tarballs_dir = self.cache_dir.join("tarballs");
        if tarballs_dir.exists() {
            std::fs::remove_dir_all(&tarballs_dir)?;
        }
        Ok(())
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_tarball(dir: &Path, name: &str, version: &str) -> (PathBuf, String) {
        // Create tarball directory structure matching tarball_url() output
        // tarball_url generates: releases/@w/data/json-transformer/1.0.0.tar.gz
        let pkg_name = name.split('/').last().unwrap_or(name);
        let releases_dir = dir.join("releases/@w/data").join(pkg_name);
        std::fs::create_dir_all(&releases_dir).unwrap();

        let tarball_path = releases_dir.join(format!("{}.tar.gz", version));

        // Create a simple tar.gz with a README
        {
            let tar_gz = std::fs::File::create(&tarball_path).unwrap();
            let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);

            // Add a simple file
            let readme_content = format!("# {} v{}", name, version);
            let mut header = tar::Header::new_gnu();
            header.set_size(readme_content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "README.md", readme_content.as_bytes())
                .unwrap();

            // Finish tar and flush gzip by dropping the encoder
            let enc = tar.into_inner().unwrap();
            enc.finish().unwrap();
        }
        // File is now fully written and closed

        // Compute checksum
        let mut file = std::fs::File::open(&tarball_path).unwrap();
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher).unwrap();
        let checksum = format!("sha256:{}", hex::encode(hasher.finalize()));

        (tarball_path, checksum)
    }

    fn setup_test_registry() -> (TempDir, RegistryConfig, String) {
        let temp = TempDir::new().unwrap();
        let index_dir = temp.path().join("index");
        let releases_dir = temp.path().join("releases");

        // Create package tarball and get checksum
        // Use just the package name for the path (json-transformer, not full path)
        let (_, checksum) = create_test_tarball(temp.path(), "json-transformer", "1.0.0");

        // Create index entry with correct checksum
        let pkg_dir = index_dir.join("@w/data");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        let mut file = std::fs::File::create(pkg_dir.join("json-transformer")).unwrap();
        writeln!(
            file,
            r#"{{"name":"@workflows/data/json-transformer","vers":"1.0.0","deps":[],"cksum":"{}","features":{{}},"yanked":false}}"#,
            checksum
        )
        .unwrap();

        let config = RegistryConfig::local(&index_dir, &releases_dir);
        (temp, config, checksum)
    }

    #[tokio::test]
    async fn test_download_and_verify() {
        let (temp, config, _) = setup_test_registry();
        let downloader = Downloader::with_config(config);

        let result = downloader.download_latest("@workflows/data/json-transformer").await;
        assert!(result.is_ok(), "Download failed: {:?}", result.err());

        let pkg = result.unwrap();
        assert_eq!(pkg.name, "@workflows/data/json-transformer");
        assert_eq!(pkg.version, "1.0.0");
        assert!(pkg.tarball_path.exists());
    }

    #[tokio::test]
    async fn test_checksum_mismatch() {
        let temp = TempDir::new().unwrap();
        let index_dir = temp.path().join("index");
        let releases_dir = temp.path().join("releases");

        // Create tarball (creates at releases/@w/data/bad-pkg/1.0.0.tar.gz)
        create_test_tarball(temp.path(), "bad-pkg", "1.0.0");

        // Create index with wrong checksum
        let pkg_dir = index_dir.join("@w/data");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        let mut file = std::fs::File::create(pkg_dir.join("bad-pkg")).unwrap();
        writeln!(
            file,
            r#"{{"name":"@workflows/data/bad-pkg","vers":"1.0.0","deps":[],"cksum":"sha256:0000000000000000000000000000000000000000000000000000000000000000","features":{{}},"yanked":false}}"#
        )
        .unwrap();

        let config = RegistryConfig::local(&index_dir, &releases_dir);
        let downloader = Downloader::with_config(config);

        let result = downloader.download_latest("@workflows/data/bad-pkg").await;
        assert!(matches!(
            result,
            Err(DownloadError::ChecksumMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn test_extract_tarball() {
        let (temp, config, _) = setup_test_registry();
        let downloader = Downloader::with_config(config);

        let pkg = downloader
            .download_latest("@workflows/data/json-transformer")
            .await
            .unwrap();

        let extract_dir = temp.path().join("extracted");
        let result = downloader.extract(&pkg, &extract_dir);
        assert!(result.is_ok());
        assert!(extract_dir.join("README.md").exists());
    }

    #[tokio::test]
    async fn test_cache_reuse() {
        let (temp, config, _) = setup_test_registry();
        let downloader = Downloader::with_config(config);

        // First download
        let pkg1 = downloader
            .download_latest("@workflows/data/json-transformer")
            .await
            .unwrap();

        // Second download should use cache
        let pkg2 = downloader
            .download_latest("@workflows/data/json-transformer")
            .await
            .unwrap();

        assert_eq!(pkg1.tarball_path, pkg2.tarball_path);
    }
}
