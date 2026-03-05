//! Model manager for the spn daemon.
//!
//! Manages model lifecycle (pull, load, unload) using the ModelBackend trait.

use spn_client::{BackendError, LoadConfig, ModelInfo, RunningModel};
use spn_ollama::{DynModelBackend, OllamaBackend};
use std::sync::Arc;
use tracing::{debug, info};

/// Manages model operations via a backend.
pub struct ModelManager {
    /// The model backend (currently Ollama).
    backend: Arc<dyn DynModelBackend>,
}

impl ModelManager {
    /// Create a new model manager with the default Ollama backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            backend: Arc::new(OllamaBackend::new()),
        }
    }

    /// Create a model manager with a custom backend.
    #[must_use]
    pub fn with_backend(backend: Arc<dyn DynModelBackend>) -> Self {
        Self { backend }
    }

    /// Check if the backend is running.
    pub async fn is_backend_running(&self) -> bool {
        self.backend.is_running().await
    }

    /// Start the backend if not running.
    pub async fn ensure_backend_running(&self) -> Result<(), BackendError> {
        if !self.backend.is_running().await {
            info!("Starting {} backend...", self.backend.name());
            self.backend.start().await?;
        }
        Ok(())
    }

    /// List all installed models.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        debug!("Listing models");
        self.backend.list_models().await
    }

    /// Pull a model.
    pub async fn pull(&self, name: &str) -> Result<(), BackendError> {
        info!(model = %name, "Pulling model");
        self.backend.pull(name, None).await
    }

    /// Load a model into memory.
    pub async fn load(&self, name: &str, config: Option<LoadConfig>) -> Result<(), BackendError> {
        let config = config.unwrap_or_default();
        info!(model = %name, "Loading model");
        self.backend.load(name, &config).await
    }

    /// Unload a model from memory.
    pub async fn unload(&self, name: &str) -> Result<(), BackendError> {
        info!(model = %name, "Unloading model");
        self.backend.unload(name).await
    }

    /// Get running models.
    pub async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError> {
        debug!("Getting running models");
        self.backend.running_models().await
    }

    /// Delete a model.
    pub async fn delete(&self, name: &str) -> Result<(), BackendError> {
        info!(model = %name, "Deleting model");
        self.backend.delete(name).await
    }

    /// Get backend identifier.
    #[must_use]
    pub fn backend_id(&self) -> &'static str {
        self.backend.id()
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
        assert_eq!(manager.backend_id(), "ollama");
    }

    #[test]
    fn test_model_manager_default() {
        let manager = ModelManager::default();
        assert_eq!(manager.backend_id(), "ollama");
    }
}
