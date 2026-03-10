//! Native runtime implementation using mistral.rs.
//!
//! This module provides the `NativeRuntime` struct which implements
//! the `InferenceBackend` trait using the mistral.rs library.

use crate::inference::traits::InferenceBackend;
use crate::NativeError;
use futures_util::stream::Stream;
use spn_core::{ChatOptions, ChatResponse, LoadConfig, ModelInfo};
use std::path::{Path, PathBuf};

#[cfg(feature = "inference")]
use spn_core::ChatRole;
#[cfg(feature = "inference")]
use std::sync::Arc;
#[cfg(feature = "inference")]
use tracing::{debug, info};

#[cfg(feature = "inference")]
use mistralrs::{GgufModelBuilder, Model, RequestBuilder, TextMessageRole, TextMessages};
#[cfg(feature = "inference")]
use tokio::sync::RwLock;

/// Native runtime for local LLM inference.
///
/// Uses mistral.rs for high-performance inference on GGUF models.
/// Supports CPU and GPU (Metal on macOS, CUDA on Linux) acceleration.
///
/// # Example
///
/// ```ignore
/// use spn_native::inference::NativeRuntime;
/// use spn_core::LoadConfig;
///
/// let mut runtime = NativeRuntime::new()?;
/// runtime.load("model.gguf".into(), LoadConfig::default()).await?;
/// let response = runtime.infer("Hello!", Default::default()).await?;
/// ```
#[allow(dead_code)] // Fields used only with inference feature
pub struct NativeRuntime {
    /// The loaded model (None if no model is loaded).
    #[cfg(feature = "inference")]
    model: Option<Arc<RwLock<Model>>>,

    /// Metadata about the loaded model.
    model_info: Option<ModelInfo>,

    /// Path to the currently loaded model.
    model_path: Option<PathBuf>,

    /// Load configuration used for the current model.
    config: Option<LoadConfig>,
}

impl NativeRuntime {
    /// Create a new native runtime.
    ///
    /// The runtime is created without a model loaded. Call `load()` to
    /// load a model before running inference.
    #[must_use]
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "inference")]
            model: None,
            model_info: None,
            model_path: None,
            config: None,
        }
    }

    /// Get the path to the currently loaded model.
    #[must_use]
    pub fn model_path(&self) -> Option<&PathBuf> {
        self.model_path.as_ref()
    }

    /// Get the load configuration for the current model.
    #[must_use]
    pub fn config(&self) -> Option<&LoadConfig> {
        self.config.as_ref()
    }

    /// Convert spn-core ChatRole to mistral.rs TextMessageRole.
    #[cfg(feature = "inference")]
    #[allow(dead_code)] // Will be used for streaming support
    fn convert_role(role: ChatRole) -> TextMessageRole {
        match role {
            ChatRole::System => TextMessageRole::System,
            ChatRole::User => TextMessageRole::User,
            ChatRole::Assistant => TextMessageRole::Assistant,
        }
    }
}

impl Default for NativeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "inference")]
impl InferenceBackend for NativeRuntime {
    async fn load(&mut self, model_path: PathBuf, config: LoadConfig) -> Result<(), NativeError> {
        info!(?model_path, "Loading GGUF model");

        // Unload any existing model
        if self.model.is_some() {
            self.unload().await?;
        }

        // Validate path exists
        if !model_path.exists() {
            return Err(NativeError::ModelNotFound {
                repo: "local".to_string(),
                filename: model_path.to_string_lossy().to_string(),
            });
        }

        // Build the model using GgufModelBuilder
        let model_path_str = model_path.to_string_lossy().to_string();

        debug!(gpu_layers = config.gpu_layers, "Building model");

        // Build model - simplified configuration
        let model = GgufModelBuilder::new(model_path_str.clone(), vec![model_path_str.clone()])
            .with_logging()
            .build()
            .await
            .map_err(|e| NativeError::InvalidConfig(format!("Failed to build model: {e}")))?;

        // Extract model info from the loaded model
        let info = ModelInfo {
            name: model_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            size: tokio::fs::metadata(&model_path)
                .await
                .map(|m| m.len())
                .unwrap_or(0),
            quantization: extract_quantization_from_path(&model_path),
            parameters: None,
            digest: None,
        };

        self.model = Some(Arc::new(RwLock::new(model)));
        self.model_info = Some(info);
        self.model_path = Some(model_path);
        self.config = Some(config);

        info!("Model loaded successfully");
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), NativeError> {
        if self.model.is_some() {
            info!("Unloading model");
            self.model = None;
            self.model_info = None;
            self.model_path = None;
            self.config = None;
        }
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    fn model_info(&self) -> Option<&ModelInfo> {
        self.model_info.as_ref()
    }

    async fn infer(&self, prompt: &str, options: ChatOptions) -> Result<ChatResponse, NativeError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| NativeError::InvalidConfig("No model loaded".to_string()))?;

        let model = model.read().await;

        // Build messages - just user prompt for now
        // System messages should be passed as part of the prompt or via messages API
        let messages = TextMessages::new().add_message(TextMessageRole::User, prompt);

        debug!(
            temperature = options.temperature,
            max_tokens = options.max_tokens,
            "Running inference"
        );

        // Build request with sampling parameters
        let mut request = RequestBuilder::from(messages);

        // Apply temperature if provided (convert f32 to f64)
        if let Some(temp) = options.temperature {
            request = request.set_sampler_temperature(f64::from(temp));
        }

        // Apply max_tokens if provided
        if let Some(max_tokens) = options.max_tokens {
            request = request.set_sampler_max_len(max_tokens as usize);
        }

        // Send request with sampling parameters
        let response = model
            .send_chat_request(request)
            .await
            .map_err(|e| NativeError::InvalidConfig(format!("Inference failed: {e}")))?;

        // Extract response content - fail if no content returned
        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| {
                NativeError::InvalidConfig("Model returned empty response (no choices)".to_string())
            })?;

        Ok(ChatResponse {
            message: spn_core::ChatMessage {
                role: ChatRole::Assistant,
                content,
            },
            done: true,
            total_duration: None,
            prompt_eval_count: Some(response.usage.prompt_tokens as u32),
            eval_count: Some(response.usage.completion_tokens as u32),
        })
    }

    async fn infer_stream(
        &self,
        _prompt: &str,
        _options: ChatOptions,
    ) -> Result<impl Stream<Item = Result<String, NativeError>> + Send, NativeError> {
        // Streaming requires complex lifetime management with the model lock.
        // For now, use the non-streaming `infer` method instead.
        // TODO: Implement streaming by cloning the Arc and managing lifetimes properly.
        Err::<futures_util::stream::Empty<Result<String, NativeError>>, _>(
            NativeError::InvalidConfig(
                "Streaming not yet implemented for native runtime. Use infer() instead.".to_string(),
            ),
        )
    }
}

/// Extract quantization from file path.
#[cfg(feature = "inference")]
fn extract_quantization_from_path(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_string_lossy().to_lowercase();
    for quant in ["q4_k_s", "q4_k_m", "q5_k_s", "q5_k_m", "q6_k", "q8_0", "f16", "f32"] {
        if filename.contains(quant) {
            return Some(quant.to_uppercase());
        }
    }
    None
}

// Stub implementation when inference feature is not enabled
#[cfg(not(feature = "inference"))]
impl InferenceBackend for NativeRuntime {
    async fn load(&mut self, _model_path: PathBuf, _config: LoadConfig) -> Result<(), NativeError> {
        Err(NativeError::InvalidConfig(
            "Inference feature not enabled. Rebuild with --features inference".to_string(),
        ))
    }

    async fn unload(&mut self) -> Result<(), NativeError> {
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        false
    }

    fn model_info(&self) -> Option<&ModelInfo> {
        None
    }

    async fn infer(
        &self,
        _prompt: &str,
        _options: ChatOptions,
    ) -> Result<ChatResponse, NativeError> {
        Err(NativeError::InvalidConfig(
            "Inference feature not enabled. Rebuild with --features inference".to_string(),
        ))
    }

    async fn infer_stream(
        &self,
        _prompt: &str,
        _options: ChatOptions,
    ) -> Result<impl Stream<Item = Result<String, NativeError>> + Send, NativeError> {
        Err::<futures_util::stream::Empty<Result<String, NativeError>>, _>(
            NativeError::InvalidConfig(
                "Inference feature not enabled. Rebuild with --features inference".to_string(),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = NativeRuntime::new();
        assert!(!runtime.is_loaded());
        assert!(runtime.model_info().is_none());
        assert!(runtime.model_path().is_none());
    }

    #[test]
    fn test_runtime_default() {
        let runtime = NativeRuntime::default();
        assert!(!runtime.is_loaded());
    }

    #[tokio::test]
    #[cfg(not(feature = "inference"))]
    async fn test_load_without_feature() {
        let mut runtime = NativeRuntime::new();
        let result = runtime
            .load(PathBuf::from("test.gguf"), LoadConfig::default())
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Inference feature not enabled"));
    }

    #[tokio::test]
    #[cfg(not(feature = "inference"))]
    async fn test_infer_without_feature() {
        let runtime = NativeRuntime::new();
        let result = runtime.infer("test", ChatOptions::default()).await;
        assert!(result.is_err());
    }
}
