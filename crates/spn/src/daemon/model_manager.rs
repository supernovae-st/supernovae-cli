//! Model manager for the spn daemon.
//!
//! Manages model lifecycle (pull, load, unload) using the ModelBackend trait.
//!
//! TODO(v0.14): Integrate with `spn model` commands and daemon IPC

#![allow(dead_code)]

use spn_client::{
    BackendError, ChatMessage, ChatOptions, ChatResponse, LoadConfig, ModelInfo, PullProgress,
    RunningModel,
};
use spn_ollama::{BoxedProgressCallback, DynModelBackend, OllamaBackend};
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

    /// Pull a model with progress callback.
    ///
    /// The callback is invoked for each progress update during the download.
    pub async fn pull_with_progress<F>(&self, name: &str, on_progress: F) -> Result<(), BackendError>
    where
        F: Fn(PullProgress) + Send + Sync + 'static,
    {
        info!(model = %name, "Pulling model with progress");
        let callback: BoxedProgressCallback = Box::new(on_progress);
        self.backend.pull(name, Some(callback)).await
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

    /// Run chat inference on a model.
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Result<ChatResponse, BackendError> {
        info!(model = %model, "Running chat inference");
        self.backend.chat(model, messages, options).await
    }

    /// Get backend identifier.
    #[must_use]
    pub fn backend_id(&self) -> &'static str {
        self.backend.id()
    }

    /// Run simple inference with a prompt.
    ///
    /// Convenience wrapper around `chat` for single-turn inference.
    pub async fn run_inference(
        &self,
        model: &str,
        prompt: &str,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> Result<String, BackendError> {
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(ChatMessage::system(sys));
        }

        messages.push(ChatMessage::user(prompt));

        let options = temperature.map(|t| ChatOptions {
            temperature: Some(t),
            ..Default::default()
        });

        let response = self.chat(model, messages, options).await?;
        Ok(response.message.content)
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
    use std::sync::atomic::{AtomicU32, Ordering};

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

    #[test]
    fn test_pull_with_progress_callback_signature() {
        // Test that the callback signature compiles correctly
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        // Create a callback that increments a counter
        let callback = move |_progress: PullProgress| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        };

        // Verify the callback type is correct by checking it can be boxed
        let _boxed: BoxedProgressCallback = Box::new(callback);

        // If this compiles, the signature is correct
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }
}
