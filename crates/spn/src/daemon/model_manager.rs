//! Model manager for the spn daemon.
//!
//! Manages model lifecycle (pull, load, unload, inference) using:
//! - `HuggingFaceStorage` for downloading models from HuggingFace Hub
//! - `NativeRuntime` for local LLM inference via mistral.rs
//!
//! Requires the `inference` feature to be enabled for full functionality.
//! Without the feature, returns "backend not available" errors.

use spn_client::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, LoadConfig, ModelInfo, PullProgress,
    RunningModel,
};

#[cfg(feature = "inference")]
use spn_native::{
    default_model_dir, find_model, resolve_model, DownloadRequest, HuggingFaceStorage,
    InferenceBackend, NativeRuntime,
};

#[cfg(feature = "inference")]
use spn_core::ChatRole;

#[cfg(feature = "inference")]
use std::sync::Arc;

#[cfg(feature = "inference")]
use tokio::sync::RwLock;

#[cfg(feature = "inference")]
use tracing::{debug, info};

#[allow(unused_imports)]
use tracing::warn;

/// Progress callback type for pull operations.
#[allow(dead_code)]
pub type BoxedProgressCallback = Box<dyn Fn(PullProgress) + Send + Sync>;

// ============================================================================
// Implementation WITH inference feature
// ============================================================================

#[cfg(feature = "inference")]
/// Manages model operations using native inference.
///
/// Uses `HuggingFaceStorage` for downloads and `NativeRuntime` for inference.
pub struct ModelManager {
    /// Storage backend for downloading models.
    storage: HuggingFaceStorage,
    /// Native inference runtime.
    runtime: Arc<RwLock<NativeRuntime>>,
    /// Currently loaded model name.
    loaded_model: Arc<RwLock<Option<String>>>,
}

#[cfg(feature = "inference")]
#[allow(dead_code)] // Public API methods - may be used by daemon
impl ModelManager {
    /// Create a new model manager with native inference support.
    #[must_use]
    pub fn new() -> Self {
        let storage_dir = default_model_dir();
        info!(?storage_dir, "ModelManager created with native inference");

        Self {
            storage: HuggingFaceStorage::new(storage_dir),
            runtime: Arc::new(RwLock::new(NativeRuntime::new())),
            loaded_model: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if the backend is running (always true for native).
    pub async fn is_backend_running(&self) -> bool {
        true
    }

    /// Ensure backend is running (no-op for native).
    pub async fn ensure_backend_running(&self) -> Result<(), BackendError> {
        Ok(())
    }

    /// Parse HuggingFace download path into a DownloadRequest.
    ///
    /// Supports formats:
    /// - `namespace/repo/filename.gguf` (3 parts with .gguf extension)
    /// - `namespace/repo` (2 parts, returns error - filename required)
    fn parse_hf_download(path: &str) -> Result<DownloadRequest<'static>, BackendError> {
        // Split by '/' and check format
        let parts: Vec<&str> = path.split('/').collect();

        match parts.len() {
            // Format: namespace/repo/filename.gguf
            3 if parts[2].ends_with(".gguf") => {
                let repo = format!("{}/{}", parts[0], parts[1]);
                let filename = parts[2].to_string();
                debug!(repo = %repo, filename = %filename, "Parsed HuggingFace download");
                Ok(DownloadRequest::huggingface(repo, filename))
            }
            // Format: namespace/repo (missing filename)
            2 => Err(BackendError::ModelNotFound(format!(
                "HuggingFace download requires a filename. Use format: {}/filename.gguf",
                path
            ))),
            // Invalid format
            _ => Err(BackendError::ModelNotFound(format!(
                "Invalid HuggingFace path '{}'. Expected: namespace/repo/filename.gguf",
                path
            ))),
        }
    }

    /// List all installed models from storage.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        use spn_core::ModelStorage;
        self.storage.list_models()
    }

    /// Pull a model from HuggingFace Hub.
    pub async fn pull(&self, name: &str) -> Result<(), BackendError> {
        self.pull_with_progress(name, |_| {}).await
    }

    /// Pull a model with progress callback.
    pub async fn pull_with_progress<F>(&self, name: &str, on_progress: F) -> Result<(), BackendError>
    where
        F: Fn(PullProgress) + Send + Sync + 'static,
    {
        // Try to resolve as a curated model first
        let request = if let Some(model) = find_model(name) {
            info!(model = name, "Pulling curated model");
            DownloadRequest::curated(model)
        } else if let Some(resolved) = resolve_model(name) {
            info!(model = name, "Pulling resolved model");
            match resolved {
                spn_native::ResolvedModel::Curated(model) => DownloadRequest::curated(model),
                spn_native::ResolvedModel::HuggingFace { repo } => {
                    // HuggingFace format: "hf:namespace/repo/filename.gguf"
                    // or just "hf:namespace/repo" (needs filename discovery)
                    Self::parse_hf_download(&repo)?
                }
            }
        } else {
            // Try parsing as direct HuggingFace path: "namespace/repo/filename.gguf"
            Self::parse_hf_download(name)?
        };

        self.storage
            .download(&request, on_progress)
            .await
            .map_err(|e| BackendError::DownloadError(e.to_string()))?;

        Ok(())
    }

    /// Load a model into memory for inference.
    pub async fn load(
        &self,
        name: &str,
        config: Option<LoadConfig>,
    ) -> Result<(), BackendError> {
        use spn_core::ModelStorage;

        // Get the model path
        let model_path = if let Some(model) = find_model(name) {
            // Curated model - construct path from repo/filename
            let filename = model.default_file;
            self.storage.model_path(&format!("{}/{}", model.hf_repo, filename))
        } else {
            // Direct path or repo/filename format
            self.storage.model_path(name)
        };

        if !model_path.exists() {
            return Err(BackendError::ModelNotFound(format!(
                "Model not found at {:?}. Run `spn model pull {}` first.",
                model_path, name
            )));
        }

        let load_config = config.unwrap_or_default();
        debug!(?model_path, ?load_config, "Loading model");

        // Clear loaded_model state BEFORE attempting load
        // This prevents stale state if runtime.load() fails after unloading
        // the previous model internally
        let mut runtime = self.runtime.write().await;
        let was_loaded = runtime.is_loaded();
        if was_loaded {
            // Clear the name now - if load fails, we don't want stale state
            *self.loaded_model.write().await = None;
        }

        // Attempt to load the new model
        match runtime.load(model_path, load_config).await {
            Ok(()) => {
                *self.loaded_model.write().await = Some(name.to_string());
                info!(model = name, "Model loaded successfully");
                Ok(())
            }
            Err(e) => {
                // Load failed - loaded_model is already cleared if we unloaded
                Err(BackendError::BackendSpecific(e.to_string()))
            }
        }
    }

    /// Unload a model from memory.
    pub async fn unload(&self, name: &str) -> Result<(), BackendError> {
        let loaded = self.loaded_model.read().await;
        if loaded.as_deref() != Some(name) {
            return Err(BackendError::ModelNotFound(format!(
                "Model '{}' is not loaded",
                name
            )));
        }
        drop(loaded);

        let mut runtime = self.runtime.write().await;
        runtime
            .unload()
            .await
            .map_err(|e| BackendError::BackendSpecific(e.to_string()))?;

        *self.loaded_model.write().await = None;
        info!(model = name, "Model unloaded");

        Ok(())
    }

    /// Get running models.
    pub async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError> {
        let loaded = self.loaded_model.read().await;
        if let Some(name) = loaded.as_ref() {
            Ok(vec![RunningModel {
                name: name.clone(),
                vram_used: None, // TODO: Get from runtime when available
                gpu_ids: vec![], // TODO: Get actual GPU IDs from runtime when available
            }])
        } else {
            Ok(vec![])
        }
    }

    /// Delete a model from storage.
    pub async fn delete(&self, name: &str) -> Result<(), BackendError> {
        use spn_core::ModelStorage;

        // Unload if currently loaded
        let loaded = self.loaded_model.read().await;
        if loaded.as_deref() == Some(name) {
            drop(loaded);
            self.unload(name).await?;
        }

        self.storage.delete(name)?;
        info!(model = name, "Model deleted");

        Ok(())
    }

    /// Run chat inference on a model.
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Result<ChatResponse, BackendError> {
        // Ensure model is loaded
        let loaded = self.loaded_model.read().await;
        if loaded.as_deref() != Some(model) {
            return Err(BackendError::BackendSpecific(format!(
                "Model '{}' is not loaded. Load it first with `spn model load {}`",
                model, model
            )));
        }
        drop(loaded);

        // Build prompt from messages
        let prompt = messages
            .iter()
            .map(|m| match m.role {
                ChatRole::System => format!("System: {}", m.content),
                ChatRole::User => format!("User: {}", m.content),
                ChatRole::Assistant => format!("Assistant: {}", m.content),
            })
            .collect::<Vec<_>>()
            .join("\n");

        let opts = options.unwrap_or_default();
        let runtime = self.runtime.read().await;

        runtime
            .infer(&prompt, opts)
            .await
            .map_err(|e| BackendError::BackendSpecific(e.to_string()))
    }

    /// Get backend identifier.
    #[must_use]
    pub fn backend_id(&self) -> &'static str {
        "mistral.rs"
    }

    /// Run simple inference with a prompt.
    pub async fn run_inference(
        &self,
        model: &str,
        prompt: &str,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> Result<String, BackendError> {
        // Ensure model is loaded
        let loaded = self.loaded_model.read().await;
        if loaded.as_deref() != Some(model) {
            return Err(BackendError::BackendSpecific(format!(
                "Model '{}' is not loaded. Load it first with `spn model load {}`",
                model, model
            )));
        }
        drop(loaded);

        // Prepend system message to prompt if provided
        let full_prompt = match system {
            Some(sys) => format!("System: {}\n\nUser: {}", sys, prompt),
            None => prompt.to_string(),
        };

        let opts = ChatOptions {
            temperature,
            ..Default::default()
        };

        let runtime = self.runtime.read().await;
        let response = runtime
            .infer(&full_prompt, opts)
            .await
            .map_err(|e| BackendError::BackendSpecific(e.to_string()))?;

        Ok(response.message.content)
    }
}

#[cfg(feature = "inference")]
impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Stub implementation WITHOUT inference feature
// ============================================================================

#[cfg(not(feature = "inference"))]
/// Stub model manager when inference feature is disabled.
///
/// All operations return "backend not available" errors.
pub struct ModelManager {
    _phantom: (),
}

#[cfg(not(feature = "inference"))]
impl ModelManager {
    /// Create a new model manager (stub).
    #[must_use]
    pub fn new() -> Self {
        warn!("ModelManager created without inference feature - model commands unavailable");
        Self { _phantom: () }
    }

    /// Check if the backend is running (always false without feature).
    pub async fn is_backend_running(&self) -> bool {
        false
    }

    /// Ensure backend is running (returns error without feature).
    pub async fn ensure_backend_running(&self) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// List all installed models (returns error without feature).
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Pull a model (returns error without feature).
    pub async fn pull(&self, _name: &str) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Pull a model with progress callback (returns error without feature).
    pub async fn pull_with_progress<F>(
        &self,
        _name: &str,
        _on_progress: F,
    ) -> Result<(), BackendError>
    where
        F: Fn(PullProgress) + Send + Sync + 'static,
    {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Load a model into memory (returns error without feature).
    pub async fn load(
        &self,
        _name: &str,
        _config: Option<LoadConfig>,
    ) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Unload a model from memory (returns error without feature).
    pub async fn unload(&self, _name: &str) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Get running models (returns error without feature).
    pub async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Delete a model (returns error without feature).
    pub async fn delete(&self, _name: &str) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Run chat inference on a model (returns error without feature).
    pub async fn chat(
        &self,
        _model: &str,
        _messages: Vec<ChatMessage>,
        _options: Option<ChatOptions>,
    ) -> Result<ChatResponse, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }

    /// Get backend identifier.
    #[must_use]
    pub fn backend_id(&self) -> &'static str {
        "none"
    }

    /// Run simple inference with a prompt (returns error without feature).
    pub async fn run_inference(
        &self,
        _model: &str,
        _prompt: &str,
        _system: Option<String>,
        _temperature: Option<f32>,
    ) -> Result<String, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native inference not available. Rebuild with --features inference".to_string(),
        ))
    }
}

#[cfg(not(feature = "inference"))]
impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_manager_creation() {
        let manager = ModelManager::new();
        #[cfg(feature = "inference")]
        assert_eq!(manager.backend_id(), "mistral.rs");
        #[cfg(not(feature = "inference"))]
        assert_eq!(manager.backend_id(), "none");
    }

    #[test]
    fn test_model_manager_default() {
        let manager = ModelManager::default();
        #[cfg(feature = "inference")]
        assert_eq!(manager.backend_id(), "mistral.rs");
        #[cfg(not(feature = "inference"))]
        assert_eq!(manager.backend_id(), "none");
    }

    #[tokio::test]
    async fn test_backend_running() {
        let manager = ModelManager::new();
        #[cfg(feature = "inference")]
        assert!(manager.is_backend_running().await);
        #[cfg(not(feature = "inference"))]
        assert!(!manager.is_backend_running().await);
    }

    #[tokio::test]
    #[cfg(not(feature = "inference"))]
    async fn test_operations_return_error_without_feature() {
        let manager = ModelManager::new();

        assert!(manager.ensure_backend_running().await.is_err());
        assert!(manager.list_models().await.is_err());
        assert!(manager.pull("test").await.is_err());
        assert!(manager.load("test", None).await.is_err());
        assert!(manager.unload("test").await.is_err());
        assert!(manager.running_models().await.is_err());
        assert!(manager.delete("test").await.is_err());
    }
}
