//! Model storage trait (download-only, no inference).
//!
//! This module defines the [`ModelStorage`] trait for downloading and managing
//! local models. The actual implementation lives in `spn-native`.
//!
//! **Note:** This trait is for spn (package manager) - download only, no inference.
//! For inference, see `NativeRuntime` in Nika.

use crate::backend::{BackendError, ModelInfo, PullProgress, Quantization};
use crate::model::KnownModel;
use std::path::PathBuf;

// ============================================================================
// Storage Location
// ============================================================================

/// Default model storage directory.
///
/// Models are stored in `~/.spn/models/` by default.
///
/// Requires the `dirs` feature to be enabled.
#[cfg(feature = "dirs")]
#[must_use]
pub fn default_model_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".spn").join("models"))
        .unwrap_or_else(|| PathBuf::from(".spn/models"))
}

/// Default model storage directory (fallback when `dirs` feature is disabled).
///
/// Returns a relative path `.spn/models` when running without home directory support.
#[cfg(not(feature = "dirs"))]
#[must_use]
pub fn default_model_dir() -> PathBuf {
    PathBuf::from(".spn/models")
}

// ============================================================================
// Sync Storage Trait (zero-dep)
// ============================================================================

/// Model storage backend (sync version).
///
/// This trait defines operations for downloading and managing local models.
/// Implementations include `HuggingFaceStorage` in spn-native.
///
/// For the async version, enable the `async-storage` feature.
pub trait ModelStorage: Send + Sync {
    /// List downloaded models.
    ///
    /// # Errors
    ///
    /// Returns error if the storage directory cannot be read.
    fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError>;

    /// Check if a model exists locally.
    fn exists(&self, model_id: &str) -> bool;

    /// Get model info for a specific model.
    ///
    /// # Errors
    ///
    /// Returns error if the model is not found.
    fn model_info(&self, model_id: &str) -> Result<ModelInfo, BackendError>;

    /// Delete a model.
    ///
    /// # Errors
    ///
    /// Returns error if the model cannot be deleted.
    fn delete(&self, model_id: &str) -> Result<(), BackendError>;

    /// Get the local path for a model.
    fn model_path(&self, model_id: &str) -> PathBuf;

    /// Get the storage root directory.
    fn storage_dir(&self) -> &PathBuf;
}

// ============================================================================
// Download Progress Callback
// ============================================================================

/// Type alias for download progress callbacks.
pub type ProgressCallback = Box<dyn Fn(PullProgress) + Send + 'static>;

// ============================================================================
// Download Request
// ============================================================================

/// Request to download a model.
#[derive(Debug, Clone)]
pub struct DownloadRequest<'a> {
    /// The model to download (curated).
    pub model: Option<&'a KnownModel>,

    /// HuggingFace repo (for passthrough).
    pub hf_repo: Option<String>,

    /// Specific filename to download.
    pub filename: Option<String>,

    /// Quantization level (for curated models).
    pub quantization: Option<Quantization>,

    /// Force re-download even if exists.
    pub force: bool,
}

impl<'a> DownloadRequest<'a> {
    /// Create a request for a curated model.
    #[must_use]
    pub fn curated(model: &'a KnownModel) -> Self {
        Self {
            model: Some(model),
            hf_repo: None,
            filename: None,
            quantization: None,
            force: false,
        }
    }

    /// Create a request for a HuggingFace model.
    #[must_use]
    pub fn huggingface(repo: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            model: None,
            hf_repo: Some(repo.into()),
            filename: Some(filename.into()),
            quantization: None,
            force: false,
        }
    }

    /// Set the quantization level.
    #[must_use]
    pub fn with_quantization(mut self, quant: Quantization) -> Self {
        self.quantization = Some(quant);
        self
    }

    /// Force re-download.
    #[must_use]
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Get the target filename for this download.
    #[must_use]
    pub fn target_filename(&self) -> Option<String> {
        if let Some(filename) = &self.filename {
            return Some(filename.clone());
        }

        if let Some(model) = self.model {
            let quant = self.quantization.unwrap_or(Quantization::Q4_K_M);
            return model.filename_for_quant(quant).map(String::from);
        }

        None
    }
}

// ============================================================================
// Download Result
// ============================================================================

/// Result of a model download.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Local path to the downloaded model.
    pub path: PathBuf,

    /// Size of the downloaded file in bytes.
    pub size: u64,

    /// SHA256 checksum of the file.
    pub checksum: Option<String>,

    /// Whether the file was already cached.
    pub cached: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_dir() {
        let dir = default_model_dir();
        // Without dirs feature, just check it contains expected components
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.contains(".spn") || dir_str.contains("spn"));
        assert!(dir_str.contains("models"));
    }

    #[test]
    fn test_download_request_curated() {
        use crate::model::find_model;

        let model = find_model("qwen3:8b").unwrap();
        let request = DownloadRequest::curated(model).with_quantization(Quantization::Q4_K_M);

        assert!(request.model.is_some());
        assert!(request.hf_repo.is_none());
        assert_eq!(request.quantization, Some(Quantization::Q4_K_M));

        let filename = request.target_filename();
        assert!(filename.is_some());
        assert!(filename.unwrap().contains("q4_k_m"));
    }

    #[test]
    fn test_download_request_huggingface() {
        let request =
            DownloadRequest::huggingface("bartowski/Model", "model-q4_k_m.gguf").force();

        assert!(request.model.is_none());
        assert_eq!(request.hf_repo.as_deref(), Some("bartowski/Model"));
        assert!(request.force);

        let filename = request.target_filename();
        assert_eq!(filename.as_deref(), Some("model-q4_k_m.gguf"));
    }
}
