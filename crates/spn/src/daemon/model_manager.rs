//! Model manager for the spn daemon.
//!
//! Manages model lifecycle (pull, load, unload).
//!
//! Currently returns "not available" errors as the Ollama backend has been
//! removed. Native inference via mistral.rs will be added in Phase 5.
//!
//! See: spn-native crate for model storage (HuggingFaceStorage)

#![allow(dead_code)]

use spn_client::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, LoadConfig, ModelInfo, PullProgress,
    RunningModel,
};
use tracing::warn;

/// Progress callback type for pull operations.
pub type BoxedProgressCallback = Box<dyn Fn(PullProgress) + Send + Sync>;

/// Manages model operations.
///
/// Currently returns "backend not available" errors.
/// Native inference will be implemented in Phase 5 (spn-native + mistral.rs).
pub struct ModelManager {
    _phantom: (),
}

impl ModelManager {
    /// Create a new model manager.
    ///
    /// Note: Currently creates a stub manager as the native backend
    /// is not yet implemented.
    #[must_use]
    pub fn new() -> Self {
        warn!("ModelManager created without backend - model commands unavailable");
        Self { _phantom: () }
    }

    /// Check if the backend is running.
    ///
    /// Always returns false until native backend is implemented.
    pub async fn is_backend_running(&self) -> bool {
        false
    }

    /// Start the backend if not running.
    ///
    /// Returns error until native backend is implemented.
    pub async fn ensure_backend_running(&self) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// List all installed models.
    ///
    /// Returns error until native backend is implemented.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Pull a model.
    ///
    /// Returns error until native backend is implemented.
    pub async fn pull(&self, _name: &str) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Pull a model with progress callback.
    ///
    /// Returns error until native backend is implemented.
    pub async fn pull_with_progress<F>(
        &self,
        _name: &str,
        _on_progress: F,
    ) -> Result<(), BackendError>
    where
        F: Fn(PullProgress) + Send + Sync + 'static,
    {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Load a model into memory.
    ///
    /// Returns error until native backend is implemented.
    pub async fn load(
        &self,
        _name: &str,
        _config: Option<LoadConfig>,
    ) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Unload a model from memory.
    ///
    /// Returns error until native backend is implemented.
    pub async fn unload(&self, _name: &str) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Get running models.
    ///
    /// Returns error until native backend is implemented.
    pub async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Delete a model.
    ///
    /// Returns error until native backend is implemented.
    pub async fn delete(&self, _name: &str) -> Result<(), BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Run chat inference on a model.
    ///
    /// Returns error until native backend is implemented.
    pub async fn chat(
        &self,
        _model: &str,
        _messages: Vec<ChatMessage>,
        _options: Option<ChatOptions>,
    ) -> Result<ChatResponse, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }

    /// Get backend identifier.
    #[must_use]
    pub fn backend_id(&self) -> &'static str {
        "none"
    }

    /// Run simple inference with a prompt.
    ///
    /// Returns error until native backend is implemented.
    pub async fn run_inference(
        &self,
        _model: &str,
        _prompt: &str,
        _system: Option<String>,
        _temperature: Option<f32>,
    ) -> Result<String, BackendError> {
        Err(BackendError::BackendSpecific(
            "Native model backend not yet implemented. Coming in Phase 5.".to_string(),
        ))
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_manager_creation() {
        let manager = ModelManager::new();
        assert_eq!(manager.backend_id(), "none");
    }

    #[test]
    fn test_model_manager_default() {
        let manager = ModelManager::default();
        assert_eq!(manager.backend_id(), "none");
    }

    #[tokio::test]
    async fn test_backend_not_running() {
        let manager = ModelManager::new();
        assert!(!manager.is_backend_running().await);
    }

    #[tokio::test]
    async fn test_operations_return_error() {
        let manager = ModelManager::new();

        // All operations should return NotRunning error
        assert!(manager.ensure_backend_running().await.is_err());
        assert!(manager.list_models().await.is_err());
        assert!(manager.pull("test").await.is_err());
        assert!(manager.load("test", None).await.is_err());
        assert!(manager.unload("test").await.is_err());
        assert!(manager.running_models().await.is_err());
        assert!(manager.delete("test").await.is_err());
    }
}
