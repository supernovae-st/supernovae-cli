//! Model backend trait for local LLM management.
//!
//! This trait defines the interface that all local model backends must implement.
//! Currently supports Ollama, with llama.cpp planned for the future.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  ModelBackend Trait                                                        │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  Lifecycle:                                                                 │
//! │  ├── is_running()    Check if backend server is running                    │
//! │  ├── start()         Start the backend server                              │
//! │  └── stop()          Stop the backend server                               │
//! │                                                                             │
//! │  Model Management:                                                          │
//! │  ├── list_models()   List all installed models                             │
//! │  ├── pull()          Download a model from registry                        │
//! │  ├── delete()        Remove a model                                        │
//! │  └── model_info()    Get detailed model information                        │
//! │                                                                             │
//! │  Runtime:                                                                   │
//! │  ├── load()          Load a model into memory (GPU)                        │
//! │  ├── unload()        Unload a model from memory                            │
//! │  └── running_models() List currently loaded models                         │
//! │                                                                             │
//! │  System:                                                                    │
//! │  ├── gpu_info()      Get GPU information                                   │
//! │  └── endpoint_url()  Get the API endpoint URL                              │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use spn_core::{BackendError, GpuInfo, LoadConfig, ModelInfo, PullProgress, RunningModel};
use std::future::Future;
use std::pin::Pin;

/// Progress callback type for pull operations.
pub type ProgressCallback = Box<dyn Fn(PullProgress) + Send + Sync>;

/// Model backend trait for local LLM management.
///
/// All methods are async to support non-blocking I/O.
/// Implementations must be `Send + Sync` for use across threads.
///
/// # Example Implementation
///
/// ```rust,ignore
/// use spn_ollama::{ModelBackend, ProgressCallback};
/// use spn_core::{BackendError, ModelInfo, LoadConfig};
///
/// struct MyBackend;
///
/// impl ModelBackend for MyBackend {
///     fn id(&self) -> &'static str { "my-backend" }
///
///     async fn is_running(&self) -> bool {
///         // Check if backend is running
///         true
///     }
///
///     // ... implement other methods
/// }
/// ```
pub trait ModelBackend: Send + Sync {
    /// Unique identifier for this backend (e.g., "ollama", "llama-cpp").
    fn id(&self) -> &'static str;

    /// Human-readable name of the backend.
    fn name(&self) -> &'static str {
        self.id()
    }

    /// Check if the backend server is running.
    fn is_running(&self) -> impl Future<Output = bool> + Send;

    /// Start the backend server.
    ///
    /// Returns `Ok(())` if already running or successfully started.
    fn start(&self) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Stop the backend server.
    ///
    /// Returns `Ok(())` if already stopped or successfully stopped.
    fn stop(&self) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// List all installed models.
    fn list_models(&self) -> impl Future<Output = Result<Vec<ModelInfo>, BackendError>> + Send;

    /// Get information about a specific model.
    fn model_info(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<ModelInfo, BackendError>> + Send;

    /// Pull/download a model from the registry.
    ///
    /// The progress callback is called with updates during download.
    fn pull(
        &self,
        name: &str,
        progress: Option<ProgressCallback>,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Delete a model.
    fn delete(&self, name: &str) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Load a model into memory (with optional GPU configuration).
    fn load(
        &self,
        name: &str,
        config: &LoadConfig,
    ) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// Unload a model from memory.
    fn unload(&self, name: &str) -> impl Future<Output = Result<(), BackendError>> + Send;

    /// List currently loaded/running models.
    fn running_models(
        &self,
    ) -> impl Future<Output = Result<Vec<RunningModel>, BackendError>> + Send;

    /// Get GPU information (if available).
    fn gpu_info(&self) -> impl Future<Output = Result<Vec<GpuInfo>, BackendError>> + Send;

    /// Get the API endpoint URL.
    fn endpoint_url(&self) -> &str;

    /// Check if a model is installed.
    fn is_installed(&self, name: &str) -> impl Future<Output = bool> + Send {
        async move { self.model_info(name).await.is_ok() }
    }

    /// Check if a model is currently loaded.
    fn is_loaded(&self, name: &str) -> impl Future<Output = bool> + Send {
        async move {
            self.running_models()
                .await
                .map(|models| models.iter().any(|m| m.name == name))
                .unwrap_or(false)
        }
    }
}

/// Boxed version of the progress callback for trait objects.
pub type BoxedProgressCallback = Box<dyn Fn(PullProgress) + Send + Sync + 'static>;

/// Type alias for a boxed `ModelBackend` trait object.
pub type BoxedBackend = Box<dyn DynModelBackend>;

/// Object-safe version of `ModelBackend` for dynamic dispatch.
///
/// This trait mirrors `ModelBackend` but uses boxed futures for
/// compatibility with trait objects.
pub trait DynModelBackend: Send + Sync {
    /// Unique identifier for this backend.
    fn id(&self) -> &'static str;

    /// Human-readable name of the backend.
    fn name(&self) -> &'static str;

    /// Check if the backend server is running.
    fn is_running(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;

    /// Start the backend server.
    fn start(&self) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Stop the backend server.
    fn stop(&self) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// List all installed models.
    fn list_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, BackendError>> + Send + '_>>;

    /// Get information about a specific model.
    fn model_info(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ModelInfo, BackendError>> + Send + '_>>;

    /// Pull/download a model from the registry.
    fn pull(
        &self,
        name: &str,
        progress: Option<BoxedProgressCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Delete a model.
    fn delete(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Load a model into memory.
    fn load(
        &self,
        name: &str,
        config: &LoadConfig,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// Unload a model from memory.
    fn unload(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), BackendError>> + Send + '_>>;

    /// List currently loaded models.
    fn running_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RunningModel>, BackendError>> + Send + '_>>;

    /// Get GPU information.
    fn gpu_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<GpuInfo>, BackendError>> + Send + '_>>;

    /// Get the API endpoint URL.
    fn endpoint_url(&self) -> &str;
}
