//! Backend types for model management.
//!
//! These types are used by spn-ollama (and future backends like llama.cpp)
//! to provide a unified interface for local model management.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  spn-core (this module)                                                    │
//! │  ├── PullProgress       Progress updates during model download              │
//! │  ├── ModelInfo          Information about an installed model                │
//! │  ├── RunningModel       Currently loaded model with GPU allocation          │
//! │  ├── GpuInfo            GPU device information                              │
//! │  ├── LoadConfig         Configuration for loading a model                   │
//! │  └── BackendError       Error types for backend operations                  │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```
//! use spn_core::{LoadConfig, ModelInfo, PullProgress};
//!
//! // Create a load configuration
//! let config = LoadConfig::default()
//!     .with_gpu_layers(-1)  // Use all GPU layers
//!     .with_context_size(4096);
//!
//! // Model info from backend
//! let info = ModelInfo {
//!     name: "llama3.2:7b".to_string(),
//!     size: 4_000_000_000,
//!     quantization: Some("Q4_K_M".to_string()),
//!     parameters: Some("7B".to_string()),
//!     digest: Some("sha256:abc123".to_string()),
//! };
//!
//! assert!(info.size_gb() > 3.0);
//! ```

use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Progress information during model pull/download.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PullProgress {
    /// Current status message (e.g., "pulling manifest", "downloading").
    pub status: String,
    /// Bytes completed.
    pub completed: u64,
    /// Total bytes to download.
    pub total: u64,
}

impl PullProgress {
    /// Create a new progress update.
    #[must_use]
    pub fn new(status: impl Into<String>, completed: u64, total: u64) -> Self {
        Self {
            status: status.into(),
            completed,
            total,
        }
    }

    /// Get progress as a percentage (0.0 to 100.0).
    #[must_use]
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed as f64 / self.total as f64) * 100.0
        }
    }

    /// Check if download is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.completed >= self.total
    }
}

impl fmt::Display for PullProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:.1}%", self.status, self.percent())
    }
}

/// Information about an installed model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModelInfo {
    /// Model name (e.g., "llama3.2:7b").
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// Quantization level (e.g., "Q4_K_M", "Q8_0").
    pub quantization: Option<String>,
    /// Parameter count (e.g., "7B", "70B").
    pub parameters: Option<String>,
    /// Model digest/hash.
    pub digest: Option<String>,
}

impl ModelInfo {
    /// Get size in gigabytes.
    #[must_use]
    pub fn size_gb(&self) -> f64 {
        self.size as f64 / 1_000_000_000.0
    }

    /// Get size as human-readable string.
    #[must_use]
    pub fn size_human(&self) -> String {
        let gb = self.size_gb();
        if gb >= 1.0 {
            format!("{gb:.1} GB")
        } else {
            format!("{:.0} MB", self.size as f64 / 1_000_000.0)
        }
    }
}

impl fmt::Display for ModelInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.size_human())
    }
}

/// Information about a currently running/loaded model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RunningModel {
    /// Model name.
    pub name: String,
    /// VRAM used in bytes (if available).
    pub vram_used: Option<u64>,
    /// GPU IDs this model is loaded on.
    pub gpu_ids: Vec<u32>,
}

impl RunningModel {
    /// Get VRAM used in gigabytes.
    #[must_use]
    pub fn vram_gb(&self) -> Option<f64> {
        self.vram_used.map(|v| v as f64 / 1_000_000_000.0)
    }
}

impl fmt::Display for RunningModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.gpu_ids.is_empty() {
            write!(f, " [GPU: {:?}]", self.gpu_ids)?;
        }
        if let Some(vram) = self.vram_gb() {
            write!(f, " ({vram:.1} GB VRAM)")?;
        }
        Ok(())
    }
}

/// GPU device information.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GpuInfo {
    /// GPU device ID.
    pub id: u32,
    /// GPU name (e.g., "NVIDIA RTX 4090").
    pub name: String,
    /// Total memory in bytes.
    pub memory_total: u64,
    /// Free memory in bytes.
    pub memory_free: u64,
}

impl GpuInfo {
    /// Get total memory in gigabytes.
    #[must_use]
    pub fn memory_total_gb(&self) -> f64 {
        self.memory_total as f64 / 1_000_000_000.0
    }

    /// Get free memory in gigabytes.
    #[must_use]
    pub fn memory_free_gb(&self) -> f64 {
        self.memory_free as f64 / 1_000_000_000.0
    }

    /// Get memory usage percentage.
    #[must_use]
    pub fn memory_used_percent(&self) -> f64 {
        if self.memory_total == 0 {
            0.0
        } else {
            let used = self.memory_total.saturating_sub(self.memory_free);
            (used as f64 / self.memory_total as f64) * 100.0
        }
    }
}

impl fmt::Display for GpuInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GPU {}: {} ({:.1}/{:.1} GB free)",
            self.id,
            self.name,
            self.memory_free_gb(),
            self.memory_total_gb()
        )
    }
}

/// Error types for backend operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BackendError {
    /// Backend server is not running.
    NotRunning,
    /// Model not found in registry or locally.
    ModelNotFound(String),
    /// Model is already loaded.
    AlreadyLoaded(String),
    /// Insufficient GPU/system memory.
    InsufficientMemory,
    /// Network error during pull/API call.
    NetworkError(String),
    /// Process management error.
    ProcessError(String),
    /// Backend-specific error.
    BackendSpecific(String),
}

impl std::error::Error for BackendError {}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRunning => write!(f, "Backend server is not running"),
            Self::ModelNotFound(name) => write!(f, "Model not found: {name}"),
            Self::AlreadyLoaded(name) => write!(f, "Model already loaded: {name}"),
            Self::InsufficientMemory => write!(f, "Insufficient memory to load model"),
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
            Self::ProcessError(msg) => write!(f, "Process error: {msg}"),
            Self::BackendSpecific(msg) => write!(f, "Backend error: {msg}"),
        }
    }
}

impl BackendError {
    /// Returns `true` if this error is transient and the operation should be retried.
    ///
    /// Retryable errors include network failures and temporary backend unavailability.
    /// Non-retryable errors include model not found, insufficient memory, etc.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::NetworkError(_) | Self::NotRunning)
    }
}

/// Configuration for loading a model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LoadConfig {
    /// GPU IDs to use for this model (empty = auto).
    pub gpu_ids: Vec<u32>,
    /// Number of layers to offload to GPU (-1 = all, 0 = none).
    pub gpu_layers: i32,
    /// Context size (token window).
    pub context_size: Option<u32>,
    /// Keep model loaded in memory (prevent unload).
    pub keep_alive: bool,
}

// ============================================================================
// Chat Types
// ============================================================================

/// Role in a chat conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ChatRole {
    /// System message (instructions).
    System,
    /// User message.
    User,
    /// Assistant response.
    Assistant,
}

impl fmt::Display for ChatRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
        }
    }
}

/// A message in a chat conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ChatMessage {
    /// Role of the message sender.
    pub role: ChatRole,
    /// Content of the message.
    pub content: String,
}

impl ChatMessage {
    /// Create a new system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
        }
    }

    /// Create a new user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }

    /// Create a new assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
        }
    }
}

/// Options for chat completion.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ChatOptions {
    /// Temperature for sampling (0.0 to 2.0).
    pub temperature: Option<f32>,
    /// Top-p (nucleus) sampling.
    pub top_p: Option<f32>,
    /// Top-k sampling.
    pub top_k: Option<u32>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Stop sequences.
    pub stop: Vec<String>,
    /// Seed for reproducibility.
    pub seed: Option<u64>,
}

impl ChatOptions {
    /// Create new chat options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set temperature.
    #[must_use]
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set top-p sampling.
    #[must_use]
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set top-k sampling.
    #[must_use]
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set maximum tokens.
    #[must_use]
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Add a stop sequence.
    #[must_use]
    pub fn with_stop(mut self, stop: impl Into<String>) -> Self {
        self.stop.push(stop.into());
        self
    }

    /// Set seed for reproducibility.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

/// Response from a chat completion.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ChatResponse {
    /// The assistant's response message.
    pub message: ChatMessage,
    /// Whether the response is complete (not streaming).
    pub done: bool,
    /// Total duration in nanoseconds.
    pub total_duration: Option<u64>,
    /// Tokens generated.
    pub eval_count: Option<u32>,
    /// Prompt tokens.
    pub prompt_eval_count: Option<u32>,
}

impl ChatResponse {
    /// Get the response content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.message.content
    }

    /// Get tokens per second (if metrics available).
    #[must_use]
    pub fn tokens_per_second(&self) -> Option<f64> {
        match (self.eval_count, self.total_duration) {
            (Some(count), Some(duration)) if duration > 0 => {
                Some(count as f64 / (duration as f64 / 1_000_000_000.0))
            }
            _ => None,
        }
    }
}

// ============================================================================
// Embedding Types
// ============================================================================

/// Response from an embedding request.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmbeddingResponse {
    /// The embedding vector.
    pub embedding: Vec<f32>,
    /// Total duration in nanoseconds.
    pub total_duration: Option<u64>,
    /// Number of tokens in the input.
    pub prompt_eval_count: Option<u32>,
}

impl EmbeddingResponse {
    /// Get the dimension of the embedding.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.embedding.len()
    }

    /// Calculate cosine similarity with another embedding.
    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        if self.embedding.len() != other.embedding.len() {
            return 0.0;
        }

        let dot_product: f32 = self
            .embedding
            .iter()
            .zip(&other.embedding)
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            gpu_ids: Vec::new(),
            gpu_layers: -1, // All layers on GPU by default
            context_size: None,
            keep_alive: false,
        }
    }
}

impl LoadConfig {
    /// Create a new load configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set specific GPU IDs.
    #[must_use]
    pub fn with_gpus(mut self, gpu_ids: Vec<u32>) -> Self {
        self.gpu_ids = gpu_ids;
        self
    }

    /// Set GPU layers (-1 = all, 0 = CPU only).
    #[must_use]
    pub fn with_gpu_layers(mut self, layers: i32) -> Self {
        self.gpu_layers = layers;
        self
    }

    /// Set context size.
    #[must_use]
    pub fn with_context_size(mut self, size: u32) -> Self {
        self.context_size = Some(size);
        self
    }

    /// Set keep alive.
    #[must_use]
    pub fn with_keep_alive(mut self, keep: bool) -> Self {
        self.keep_alive = keep;
        self
    }

    /// Check if this is a CPU-only configuration.
    #[must_use]
    pub fn is_cpu_only(&self) -> bool {
        self.gpu_layers == 0
    }

    /// Check if using all GPU layers.
    #[must_use]
    pub fn is_full_gpu(&self) -> bool {
        self.gpu_layers < 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pull_progress() {
        let progress = PullProgress::new("downloading", 500, 1000);
        assert_eq!(progress.percent(), 50.0);
        assert!(!progress.is_complete());

        let complete = PullProgress::new("complete", 1000, 1000);
        assert!(complete.is_complete());
    }

    #[test]
    fn test_pull_progress_display() {
        let progress = PullProgress::new("pulling", 750, 1000);
        assert_eq!(progress.to_string(), "pulling: 75.0%");
    }

    #[test]
    fn test_pull_progress_zero_total() {
        let progress = PullProgress::new("starting", 0, 0);
        assert_eq!(progress.percent(), 0.0);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_model_info_size() {
        let info = ModelInfo {
            name: "llama3.2:7b".to_string(),
            size: 4_500_000_000,
            quantization: Some("Q4_K_M".to_string()),
            parameters: Some("7B".to_string()),
            digest: None,
        };

        assert!((info.size_gb() - 4.5).abs() < 0.01);
        assert_eq!(info.size_human(), "4.5 GB");
    }

    #[test]
    fn test_model_info_display() {
        let info = ModelInfo {
            name: "test:latest".to_string(),
            size: 500_000_000,
            quantization: None,
            parameters: None,
            digest: None,
        };

        assert!(info.to_string().contains("test:latest"));
        assert!(info.to_string().contains("500 MB"));
    }

    #[test]
    fn test_running_model() {
        let model = RunningModel {
            name: "llama3.2".to_string(),
            vram_used: Some(4_000_000_000),
            gpu_ids: vec![0],
        };

        assert!((model.vram_gb().unwrap() - 4.0).abs() < 0.01);
        assert!(model.to_string().contains("llama3.2"));
        assert!(model.to_string().contains("GPU"));
    }

    #[test]
    fn test_gpu_info() {
        let gpu = GpuInfo {
            id: 0,
            name: "RTX 4090".to_string(),
            memory_total: 24_000_000_000,
            memory_free: 20_000_000_000,
        };

        assert!((gpu.memory_total_gb() - 24.0).abs() < 0.01);
        assert!((gpu.memory_free_gb() - 20.0).abs() < 0.01);
        assert!((gpu.memory_used_percent() - 16.67).abs() < 0.5);
    }

    #[test]
    fn test_backend_error_display() {
        let err = BackendError::NotRunning;
        assert!(err.to_string().contains("not running"));

        let err = BackendError::ModelNotFound("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_load_config_default() {
        let config = LoadConfig::default();
        assert!(config.gpu_ids.is_empty());
        assert_eq!(config.gpu_layers, -1);
        assert!(config.is_full_gpu());
        assert!(!config.is_cpu_only());
    }

    #[test]
    fn test_load_config_builder() {
        let config = LoadConfig::new()
            .with_gpus(vec![0, 1])
            .with_gpu_layers(32)
            .with_context_size(8192)
            .with_keep_alive(true);

        assert_eq!(config.gpu_ids, vec![0, 1]);
        assert_eq!(config.gpu_layers, 32);
        assert_eq!(config.context_size, Some(8192));
        assert!(config.keep_alive);
        assert!(!config.is_cpu_only());
        assert!(!config.is_full_gpu());
    }

    #[test]
    fn test_load_config_cpu_only() {
        let config = LoadConfig::new().with_gpu_layers(0);
        assert!(config.is_cpu_only());
        assert!(!config.is_full_gpu());
    }

    #[test]
    fn test_chat_role_display() {
        assert_eq!(ChatRole::System.to_string(), "system");
        assert_eq!(ChatRole::User.to_string(), "user");
        assert_eq!(ChatRole::Assistant.to_string(), "assistant");
    }

    #[test]
    fn test_chat_message_constructors() {
        let system = ChatMessage::system("You are helpful");
        assert_eq!(system.role, ChatRole::System);
        assert_eq!(system.content, "You are helpful");

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, ChatRole::User);

        let assistant = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant.role, ChatRole::Assistant);
    }

    #[test]
    fn test_chat_options_builder() {
        let options = ChatOptions::new()
            .with_temperature(0.7)
            .with_top_p(0.9)
            .with_top_k(40)
            .with_max_tokens(100)
            .with_stop("END")
            .with_seed(42);

        assert_eq!(options.temperature, Some(0.7));
        assert_eq!(options.top_p, Some(0.9));
        assert_eq!(options.top_k, Some(40));
        assert_eq!(options.max_tokens, Some(100));
        assert_eq!(options.stop, vec!["END"]);
        assert_eq!(options.seed, Some(42));
    }

    #[test]
    fn test_chat_response_content() {
        let response = ChatResponse {
            message: ChatMessage::assistant("Hello!"),
            done: true,
            total_duration: Some(1_000_000_000),
            eval_count: Some(10),
            prompt_eval_count: Some(5),
        };

        assert_eq!(response.content(), "Hello!");
        assert!(response.done);
    }

    #[test]
    fn test_chat_response_tokens_per_second() {
        let response = ChatResponse {
            message: ChatMessage::assistant("Test"),
            done: true,
            total_duration: Some(2_000_000_000), // 2 seconds
            eval_count: Some(100),
            prompt_eval_count: None,
        };

        let tps = response.tokens_per_second().unwrap();
        assert!((tps - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_embedding_response_dimension() {
        let response = EmbeddingResponse {
            embedding: vec![0.1, 0.2, 0.3, 0.4],
            total_duration: None,
            prompt_eval_count: None,
        };

        assert_eq!(response.dimension(), 4);
    }

    #[test]
    fn test_embedding_cosine_similarity() {
        let a = EmbeddingResponse {
            embedding: vec![1.0, 0.0, 0.0],
            total_duration: None,
            prompt_eval_count: None,
        };

        let b = EmbeddingResponse {
            embedding: vec![1.0, 0.0, 0.0],
            total_duration: None,
            prompt_eval_count: None,
        };

        // Identical vectors should have similarity of 1.0
        assert!((a.cosine_similarity(&b) - 1.0).abs() < 0.001);

        let c = EmbeddingResponse {
            embedding: vec![0.0, 1.0, 0.0],
            total_duration: None,
            prompt_eval_count: None,
        };

        // Orthogonal vectors should have similarity of 0.0
        assert!((a.cosine_similarity(&c)).abs() < 0.001);
    }

    #[test]
    fn test_embedding_cosine_similarity_different_dimensions() {
        let a = EmbeddingResponse {
            embedding: vec![1.0, 0.0],
            total_duration: None,
            prompt_eval_count: None,
        };

        let b = EmbeddingResponse {
            embedding: vec![1.0, 0.0, 0.0],
            total_duration: None,
            prompt_eval_count: None,
        };

        // Different dimensions should return 0.0
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }

    #[test]
    fn test_backend_error_is_retryable() {
        // Retryable errors (transient failures)
        assert!(BackendError::NetworkError("timeout".to_string()).is_retryable());
        assert!(BackendError::NotRunning.is_retryable());

        // Non-retryable errors (permanent failures)
        assert!(!BackendError::ModelNotFound("model".to_string()).is_retryable());
        assert!(!BackendError::AlreadyLoaded("model".to_string()).is_retryable());
        assert!(!BackendError::InsufficientMemory.is_retryable());
        assert!(!BackendError::ProcessError("error".to_string()).is_retryable());
        assert!(!BackendError::BackendSpecific("error".to_string()).is_retryable());
    }
}
