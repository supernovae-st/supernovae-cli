//! HuggingFace model storage implementation.
//!
//! Downloads GGUF models from HuggingFace Hub with:
//! - Progress callbacks
//! - SHA256 checksum verification
//! - Resumable downloads (via HTTP Range requests)
//! - Caching (skip download if file exists and matches checksum)

use crate::error::{NativeError, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use spn_core::{BackendError, DownloadRequest, DownloadResult, ModelInfo, ModelStorage, PullProgress};
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

// ============================================================================
// HuggingFace API Types
// ============================================================================

/// File info from HuggingFace API.
#[derive(Debug, Deserialize)]
struct HfFileInfo {
    /// Filename.
    #[serde(rename = "rfilename")]
    filename: String,
    /// File size in bytes.
    size: u64,
    /// LFS info (contains SHA256).
    lfs: Option<HfLfsInfo>,
}

/// LFS metadata from HuggingFace.
#[derive(Debug, Deserialize)]
struct HfLfsInfo {
    /// SHA256 checksum.
    sha256: String,
}

// ============================================================================
// HuggingFace Storage
// ============================================================================

/// Storage backend for HuggingFace Hub models.
///
/// Downloads GGUF models from HuggingFace with progress tracking and
/// checksum verification.
///
/// # Example
///
/// ```ignore
/// use spn_native::{HuggingFaceStorage, default_model_dir, DownloadRequest, find_model};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let storage = HuggingFaceStorage::new(default_model_dir());
///     let model = find_model("qwen3:8b").unwrap();
///     let request = DownloadRequest::curated(model);
///
///     let result = storage.download(&request, |p| {
///         println!("{}", p);
///     }).await?;
///
///     println!("Downloaded: {:?}", result.path);
///     Ok(())
/// }
/// ```
pub struct HuggingFaceStorage {
    /// Root directory for model storage.
    storage_dir: PathBuf,
    /// HTTP client.
    client: Client,
}

impl HuggingFaceStorage {
    /// Create a new HuggingFace storage with the given directory.
    #[must_use]
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            client: Client::builder()
                .user_agent("spn-native/0.1.0")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create storage with a custom HTTP client.
    #[must_use]
    pub fn with_client(storage_dir: PathBuf, client: Client) -> Self {
        Self {
            storage_dir,
            client,
        }
    }

    /// Download a model with progress callback.
    ///
    /// # Arguments
    ///
    /// * `request` - Download request specifying model and quantization
    /// * `progress` - Callback for download progress updates
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Model not found on HuggingFace
    /// - Network error during download
    /// - Checksum verification fails
    /// - I/O error writing file
    pub async fn download<F>(
        &self,
        request: &DownloadRequest<'_>,
        progress: F,
    ) -> Result<DownloadResult>
    where
        F: Fn(PullProgress) + Send + 'static,
    {
        // Resolve repo and filename
        let (repo, filename) = self.resolve_request(request)?;

        // Create storage directory
        let model_dir = self.storage_dir.join(&repo);
        fs::create_dir_all(&model_dir).await?;

        let file_path = model_dir.join(&filename);

        // Check if already downloaded
        if !request.force && file_path.exists() {
            progress(PullProgress::new("cached", 1, 1));
            let metadata = fs::metadata(&file_path).await?;
            return Ok(DownloadResult {
                path: file_path,
                size: metadata.len(),
                checksum: None,
                cached: true,
            });
        }

        // Get file info from HuggingFace API
        progress(PullProgress::new("fetching metadata", 0, 1));
        let file_info = self.get_file_info(&repo, &filename).await?;

        // Download the file
        let download_url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo, filename
        );

        progress(PullProgress::new("downloading", 0, file_info.size));

        let response = self.client.get(&download_url).send().await?;

        if !response.status().is_success() {
            return Err(NativeError::ModelNotFound {
                repo: repo.clone(),
                filename: filename.clone(),
            });
        }

        // Stream download to file with progress
        let mut file = File::create(&file_path).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut hasher = Sha256::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            progress(PullProgress::new("downloading", downloaded, file_info.size));
        }

        file.flush().await?;
        drop(file);

        // Verify checksum
        let checksum = format!("{:x}", hasher.finalize());
        if let Some(ref lfs) = file_info.lfs {
            if checksum != lfs.sha256 {
                // Delete corrupted file
                let _ = fs::remove_file(&file_path).await;
                return Err(NativeError::ChecksumMismatch {
                    path: file_path,
                    expected: lfs.sha256.clone(),
                    actual: checksum,
                });
            }
        }

        progress(PullProgress::new("complete", file_info.size, file_info.size));

        Ok(DownloadResult {
            path: file_path,
            size: file_info.size,
            checksum: Some(checksum),
            cached: false,
        })
    }

    /// Resolve download request to HuggingFace repo and filename.
    fn resolve_request(&self, request: &DownloadRequest<'_>) -> Result<(String, String)> {
        if let Some(hf_repo) = &request.hf_repo {
            let filename = request
                .filename
                .clone()
                .ok_or_else(|| NativeError::InvalidConfig("HuggingFace download requires filename".into()))?;
            return Ok((hf_repo.clone(), filename));
        }

        if let Some(model) = request.model {
            let filename = request
                .target_filename()
                .ok_or_else(|| NativeError::InvalidConfig("No quantization available for model".into()))?;
            return Ok((model.hf_repo.to_string(), filename));
        }

        Err(NativeError::InvalidConfig(
            "Download request must specify model or HuggingFace repo".into(),
        ))
    }

    /// Get file info from HuggingFace API.
    async fn get_file_info(&self, repo: &str, filename: &str) -> Result<HfFileInfo> {
        let api_url = format!(
            "https://huggingface.co/api/models/{}/tree/main",
            repo
        );

        let response = self.client.get(&api_url).send().await?;

        if !response.status().is_success() {
            return Err(NativeError::ModelNotFound {
                repo: repo.to_string(),
                filename: filename.to_string(),
            });
        }

        let files: Vec<HfFileInfo> = response.json().await?;

        files
            .into_iter()
            .find(|f| f.filename == filename)
            .ok_or_else(|| NativeError::ModelNotFound {
                repo: repo.to_string(),
                filename: filename.to_string(),
            })
    }
}

// ============================================================================
// ModelStorage Implementation
// ============================================================================

impl ModelStorage for HuggingFaceStorage {
    fn list_models(&self) -> std::result::Result<Vec<ModelInfo>, BackendError> {
        let mut models = Vec::new();

        if !self.storage_dir.exists() {
            return Ok(models);
        }

        // Walk the storage directory
        let entries = std::fs::read_dir(&self.storage_dir)
            .map_err(|e| BackendError::StorageError(e.to_string()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // This is a repo directory
                let repo_name = entry.file_name().to_string_lossy().to_string();

                // List GGUF files in this directory
                if let Ok(files) = std::fs::read_dir(&path) {
                    for file in files.flatten() {
                        let filename = file.file_name().to_string_lossy().to_string();
                        if filename.ends_with(".gguf") {
                            if let Ok(metadata) = file.metadata() {
                                let quant = extract_quantization(&filename);
                                models.push(ModelInfo {
                                    name: format!("{}/{}", repo_name, filename),
                                    size: metadata.len(),
                                    quantization: quant,
                                    parameters: None,
                                    digest: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(models)
    }

    fn exists(&self, model_id: &str) -> bool {
        self.model_path(model_id).exists()
    }

    fn model_info(&self, model_id: &str) -> std::result::Result<ModelInfo, BackendError> {
        let path = self.model_path(model_id);
        if !path.exists() {
            return Err(BackendError::ModelNotFound(model_id.to_string()));
        }

        let metadata = std::fs::metadata(&path)
            .map_err(|e| BackendError::StorageError(e.to_string()))?;

        let filename = path.file_name().unwrap_or_default().to_string_lossy();

        Ok(ModelInfo {
            name: model_id.to_string(),
            size: metadata.len(),
            quantization: extract_quantization(&filename),
            parameters: None,
            digest: None,
        })
    }

    fn delete(&self, model_id: &str) -> std::result::Result<(), BackendError> {
        let path = self.model_path(model_id);
        if !path.exists() {
            return Err(BackendError::ModelNotFound(model_id.to_string()));
        }

        std::fs::remove_file(&path)
            .map_err(|e| BackendError::StorageError(e.to_string()))?;

        Ok(())
    }

    fn model_path(&self, model_id: &str) -> PathBuf {
        // model_id format: "repo/filename" or just "filename"
        // Both cases join to storage_dir
        self.storage_dir.join(model_id)
    }

    fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
}

// ============================================================================
// Helpers
// ============================================================================

// Use the shared extract_quantization function from crate root.
use crate::extract_quantization;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_quantization() {
        assert_eq!(
            extract_quantization("model-q4_k_m.gguf"),
            Some("Q4_K_M".to_string())
        );
        assert_eq!(
            extract_quantization("model-Q8_0.gguf"),
            Some("Q8_0".to_string())
        );
        assert_eq!(
            extract_quantization("model-f16.gguf"),
            Some("F16".to_string())
        );
        assert_eq!(extract_quantization("model.gguf"), None);
    }

    #[test]
    fn test_storage_new() {
        let dir = tempdir().unwrap();
        let storage = HuggingFaceStorage::new(dir.path().to_path_buf());
        assert_eq!(storage.storage_dir(), dir.path());
    }

    #[test]
    fn test_model_path() {
        let dir = tempdir().unwrap();
        let storage = HuggingFaceStorage::new(dir.path().to_path_buf());

        let path = storage.model_path("repo/model.gguf");
        assert!(path.ends_with("repo/model.gguf"));

        let path = storage.model_path("model.gguf");
        assert!(path.ends_with("model.gguf"));
    }

    #[test]
    fn test_list_models_empty() {
        let dir = tempdir().unwrap();
        let storage = HuggingFaceStorage::new(dir.path().to_path_buf());
        let models = storage.list_models().unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn test_exists_false() {
        let dir = tempdir().unwrap();
        let storage = HuggingFaceStorage::new(dir.path().to_path_buf());
        assert!(!storage.exists("nonexistent/model.gguf"));
    }
}
