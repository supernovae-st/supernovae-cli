//! Ollama backend implementation.
//!
//! Implements the `ModelBackend` trait for Ollama.

use crate::backend::{DynModelBackend, ModelBackend, ProgressCallback};
use crate::client::OllamaClient;
use spn_core::{BackendError, GpuInfo, LoadConfig, ModelInfo, PullProgress, RunningModel};
use std::future::Future;
use std::pin::Pin;
use tracing::{debug, info, warn};

/// Ollama backend for local model management.
///
/// # Example
///
/// ```rust,ignore
/// use spn_ollama::OllamaBackend;
/// use spn_ollama::ModelBackend;
///
/// let backend = OllamaBackend::new();
///
/// if backend.is_running().await {
///     let models = backend.list_models().await?;
///     for model in models {
///         println!("{}", model.name);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OllamaBackend {
    client: OllamaClient,
}

impl OllamaBackend {
    /// Create a new Ollama backend with default endpoint.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: OllamaClient::new(),
        }
    }

    /// Create a new Ollama backend with custom endpoint.
    #[must_use]
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            client: OllamaClient::with_endpoint(endpoint),
        }
    }

    /// Get the underlying client.
    #[must_use]
    pub const fn client(&self) -> &OllamaClient {
        &self.client
    }

    // Private implementation methods to avoid trait ambiguity
    async fn impl_is_running(&self) -> bool {
        self.client.is_running().await
    }

    async fn impl_start(&self) -> Result<(), BackendError> {
        if self.impl_is_running().await {
            debug!("Ollama is already running");
            return Ok(());
        }

        info!("Starting Ollama server...");

        #[cfg(unix)]
        {
            use tokio::process::Command;

            #[cfg(target_os = "macos")]
            {
                let output = Command::new("launchctl")
                    .args(["start", "com.ollama.ollama"])
                    .output()
                    .await;

                if output.is_ok() {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if self.impl_is_running().await {
                        info!("Ollama started via launchctl");
                        return Ok(());
                    }
                }
            }

            let result = Command::new("ollama").arg("serve").spawn();

            match result {
                Ok(_child) => {
                    for _ in 0..10 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        if self.impl_is_running().await {
                            info!("Ollama server started");
                            return Ok(());
                        }
                    }
                    Err(BackendError::ProcessError(
                        "Ollama started but not responding".into(),
                    ))
                }
                Err(e) => Err(BackendError::ProcessError(format!(
                    "Failed to start Ollama: {e}"
                ))),
            }
        }

        #[cfg(not(unix))]
        {
            Err(BackendError::ProcessError(
                "Auto-start not supported on this platform".into(),
            ))
        }
    }

    #[allow(clippy::unused_async)] // Async used on macOS but not other platforms
    async fn impl_stop(&self) -> Result<(), BackendError> {
        warn!("Stopping Ollama server is not directly supported");

        #[cfg(target_os = "macos")]
        {
            use tokio::process::Command;
            let _ = Command::new("launchctl")
                .args(["stop", "com.ollama.ollama"])
                .output()
                .await;
        }

        Ok(())
    }

    async fn impl_list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        if !self.impl_is_running().await {
            return Err(BackendError::NotRunning);
        }
        self.client.list_models().await
    }

    async fn impl_model_info(&self, name: &str) -> Result<ModelInfo, BackendError> {
        if !self.impl_is_running().await {
            return Err(BackendError::NotRunning);
        }
        self.client.model_info(name).await
    }

    async fn impl_pull(
        &self,
        name: &str,
        progress: Option<ProgressCallback>,
    ) -> Result<(), BackendError> {
        if !self.impl_is_running().await {
            return Err(BackendError::NotRunning);
        }

        info!(model = %name, "Pulling model");

        let callback = progress.unwrap_or_else(|| Box::new(|_: PullProgress| {}));

        self.client.pull(name, callback).await
    }

    async fn impl_delete(&self, name: &str) -> Result<(), BackendError> {
        if !self.impl_is_running().await {
            return Err(BackendError::NotRunning);
        }

        info!(model = %name, "Deleting model");
        self.client.delete(name).await
    }

    async fn impl_load(&self, name: &str, config: &LoadConfig) -> Result<(), BackendError> {
        if !self.impl_is_running().await {
            return Err(BackendError::NotRunning);
        }

        info!(model = %name, ?config, "Loading model");

        let keep_alive = if config.keep_alive { Some("-1") } else { None };

        self.client.generate_warmup(name, keep_alive).await
    }

    async fn impl_unload(&self, name: &str) -> Result<(), BackendError> {
        if !self.impl_is_running().await {
            return Err(BackendError::NotRunning);
        }

        info!(model = %name, "Unloading model");
        self.client.generate_warmup(name, Some("0")).await
    }

    async fn impl_running_models(&self) -> Result<Vec<RunningModel>, BackendError> {
        if !self.impl_is_running().await {
            return Err(BackendError::NotRunning);
        }

        let models = self.client.running_models().await?;

        Ok(models
            .into_iter()
            .map(|m| RunningModel {
                name: m.name,
                vram_used: m.size_vram,
                gpu_ids: vec![],
            })
            .collect())
    }

    #[allow(clippy::unnecessary_wraps, clippy::unused_self)] // Required to match trait signature
    fn impl_gpu_info(&self) -> Result<Vec<GpuInfo>, BackendError> {
        debug!("GPU info not directly available from Ollama API");
        Ok(vec![])
    }
}

impl Default for OllamaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelBackend for OllamaBackend {
    fn id(&self) -> &'static str {
        "ollama"
    }

    fn name(&self) -> &'static str {
        "Ollama"
    }

    async fn is_running(&self) -> bool {
        self.impl_is_running().await
    }

    async fn start(&self) -> Result<(), BackendError> {
        self.impl_start().await
    }

    async fn stop(&self) -> Result<(), BackendError> {
        self.impl_stop().await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        self.impl_list_models().await
    }

    async fn model_info(&self, name: &str) -> Result<ModelInfo, BackendError> {
        self.impl_model_info(name).await
    }

    async fn pull(
        &self,
        name: &str,
        progress: Option<ProgressCallback>,
    ) -> Result<(), BackendError> {
        self.impl_pull(name, progress).await
    }

    async fn delete(&self, name: &str) -> Result<(), BackendError> {
        self.impl_delete(name).await
    }

    async fn load(&self, name: &str, config: &LoadConfig) -> Result<(), BackendError> {
        self.impl_load(name, config).await
    }

    async fn unload(&self, name: &str) -> Result<(), BackendError> {
        self.impl_unload(name).await
    }

    async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError> {
        self.impl_running_models().await
    }

    async fn gpu_info(&self) -> Result<Vec<GpuInfo>, BackendError> {
        self.impl_gpu_info()
    }

    fn endpoint_url(&self) -> &str {
        self.client.endpoint()
    }
}

// ============================================================================
// DynModelBackend implementation for trait objects
// ============================================================================

impl DynModelBackend for OllamaBackend {
    fn id(&self) -> &'static str {
        "ollama"
    }

    fn name(&self) -> &'static str {
        "Ollama"
    }

    fn is_running(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(self.impl_is_running())
    }

    fn start(&self) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>> {
        Box::pin(self.impl_start())
    }

    fn stop(&self) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>> {
        Box::pin(self.impl_stop())
    }

    fn list_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, BackendError>> + Send + '_>> {
        Box::pin(self.impl_list_models())
    }

    fn model_info(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ModelInfo, BackendError>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move { self.impl_model_info(&name).await })
    }

    fn pull(
        &self,
        name: &str,
        progress: Option<crate::backend::BoxedProgressCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move { self.impl_pull(&name, progress).await })
    }

    fn delete(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move { self.impl_delete(&name).await })
    }

    fn load(
        &self,
        name: &str,
        config: &LoadConfig,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>> {
        let name = name.to_string();
        let config = config.clone();
        Box::pin(async move { self.impl_load(&name, &config).await })
    }

    fn unload(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move { self.impl_unload(&name).await })
    }

    fn running_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RunningModel>, BackendError>> + Send + '_>> {
        Box::pin(self.impl_running_models())
    }

    fn gpu_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<GpuInfo>, BackendError>> + Send + '_>> {
        Box::pin(async move { self.impl_gpu_info() })
    }

    fn endpoint_url(&self) -> &str {
        self.client.endpoint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_id() {
        let backend = OllamaBackend::new();
        assert_eq!(ModelBackend::id(&backend), "ollama");
        assert_eq!(ModelBackend::name(&backend), "Ollama");
    }

    #[test]
    fn test_backend_endpoint() {
        let backend = OllamaBackend::new();
        assert_eq!(
            ModelBackend::endpoint_url(&backend),
            "http://localhost:11434"
        );

        let custom = OllamaBackend::with_endpoint("http://custom:8080");
        assert_eq!(ModelBackend::endpoint_url(&custom), "http://custom:8080");
    }

    #[tokio::test]
    async fn test_is_running_offline() {
        let backend = OllamaBackend::with_endpoint("http://localhost:99999");
        assert!(!backend.impl_is_running().await);
    }
}
