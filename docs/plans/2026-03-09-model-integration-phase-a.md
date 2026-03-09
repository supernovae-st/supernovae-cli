# Master Plan Phase A: Unified Backend Architecture

**Version:** 0.16.0
**Status:** Draft
**Author:** Claude + Thibaut
**Date:** 2026-03-09

---

## Executive Summary

Phase A unifies the existing model management (Ollama + Cloud Providers) into a single
`ModelOrchestrator` with the `@models/` alias system in `spn.yaml`. This phase requires
**no new ML dependencies** — it's pure architecture refactoring.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PHASE A SCOPE                                                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ✅ IN SCOPE                              ❌ OUT OF SCOPE                       │
│  ─────────────────────────────────────    ─────────────────────────────────     │
│  • Backend registry system                • Candle integration (Phase B)        │
│  • @models/ aliases in spn.yaml           • mistral.rs integration (Phase B)    │
│  • Cloud providers as backends            • llmfit integration (Phase C)        │
│  • ModelOrchestrator routing              • Image/audio generation              │
│  • Intent-based model selection           • Model explorer TUI                  │
│  • Nika workflow integration              • Hardware-aware recommendations      │
│  • spn-mcp model tools                                                          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Architecture

### Current State

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  CURRENT (v0.15.x)                                                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn-core                    spn-ollama                    spn-keyring          │
│  ├── Provider enum           ├── ModelBackend trait        ├── API keys         │
│  ├── ChatMessage             ├── DynModelBackend trait     └── resolve_key()    │
│  ├── ChatResponse            ├── OllamaBackend                                  │
│  └── BackendError            └── OllamaClient                                   │
│                                                                                 │
│  PROBLEM: Cloud providers (Anthropic, OpenAI) are in spn-keyring/Provider,     │
│           not unified with ModelBackend. No @models/ aliases. No orchestrator. │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Target State

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  TARGET (v0.16.0)                                                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐    │
│  │  spn.yaml                                                               │    │
│  │  ────────                                                               │    │
│  │  models:                                                                │    │
│  │    - @models/llama3.2:8b          # → OllamaBackend                     │    │
│  │    - @models/claude-sonnet        # → AnthropicBackend                  │    │
│  │    - @models/gpt-4o               # → OpenAIBackend                     │    │
│  │    - @models/codestral            # → MistralBackend (cloud)            │    │
│  └────────────────────────────────────────────────────────────────────────┘    │
│                                       │                                         │
│                                       ▼                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐    │
│  │  ModelOrchestrator                                                      │    │
│  │  ├── resolve_model("@models/llama3.2:8b") → (OllamaBackend, "llama3.2")│    │
│  │  ├── resolve_intent("deep-reasoning") → claude-sonnet or gpt-4o        │    │
│  │  └── route_request(model_ref, messages) → ChatResponse                 │    │
│  └────────────────────────────────────────────────────────────────────────┘    │
│                                       │                                         │
│           ┌───────────────────────────┼───────────────────────────┐             │
│           ▼                           ▼                           ▼             │
│  ┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐   │
│  │  OllamaBackend  │         │ AnthropicBackend│         │  OpenAIBackend  │   │
│  │  (local, free)  │         │  (cloud, paid)  │         │  (cloud, paid)  │   │
│  └─────────────────┘         └─────────────────┘         └─────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Task 1: Create `spn-backends` Crate

**Purpose:** Extract backend traits to shared location, add cloud backends.

**New crate structure:**
```
crates/spn-backends/
├── Cargo.toml
└── src/
    ├── lib.rs              # Re-exports
    ├── traits.rs           # ModelBackend, DynModelBackend (moved from spn-ollama)
    ├── registry.rs         # BackendRegistry, BackendKind
    ├── orchestrator.rs     # ModelOrchestrator, IntentRouter
    ├── capabilities.rs     # BackendCapabilities
    ├── model_ref.rs        # ModelRef, ModelAlias
    │
    ├── local/
    │   └── mod.rs          # Re-export OllamaBackend from spn-ollama
    │
    └── cloud/
        ├── mod.rs
        ├── anthropic.rs    # AnthropicBackend
        ├── openai.rs       # OpenAIBackend
        ├── mistral.rs      # MistralBackend (cloud API)
        ├── groq.rs         # GroqBackend
        ├── deepseek.rs     # DeepSeekBackend
        └── gemini.rs       # GeminiBackend
```

**Cargo.toml:**
```toml
[package]
name = "spn-backends"
version = "0.1.0"
edition = "2021"

[features]
default = ["ollama", "cloud-anthropic", "cloud-openai"]

# Local backends
ollama = ["dep:spn-ollama"]

# Cloud backends
cloud-anthropic = ["dep:reqwest"]
cloud-openai = ["dep:reqwest"]
cloud-mistral = ["dep:reqwest"]
cloud-groq = ["dep:reqwest"]
cloud-deepseek = ["dep:reqwest"]
cloud-gemini = ["dep:reqwest"]

# All cloud
cloud-all = ["cloud-anthropic", "cloud-openai", "cloud-mistral", "cloud-groq", "cloud-deepseek", "cloud-gemini"]

[dependencies]
spn-core = { path = "../spn-core" }
spn-ollama = { path = "../spn-ollama", optional = true }
spn-keyring = { path = "../spn-keyring" }

reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls-webpki-roots", "stream"], optional = true }
tokio = { version = "1.36", features = ["sync"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2"
tracing = "0.1"
async-trait = "0.1"  # For object-safe async traits if needed
futures-util = "0.3"
```

### Task 2: Implement Cloud Backends

**Example: AnthropicBackend**

```rust
// crates/spn-backends/src/cloud/anthropic.rs

use crate::traits::{ModelBackend, DynModelBackend, BoxFuture};
use spn_core::{
    BackendError, BackendResult, ChatMessage, ChatOptions, ChatResponse,
    ChatRole, EmbeddingResponse, LoadConfig, ModelInfo, RunningModel,
    GpuInfo, PullProgress,
};
use spn_keyring::resolve_api_key;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1";

/// Cloud backend for Anthropic Claude models
#[derive(Clone)]
pub struct AnthropicBackend {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl AnthropicBackend {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: None,
        }
    }

    pub async fn with_api_key(mut self) -> Result<Self, BackendError> {
        self.api_key = Some(
            resolve_api_key("anthropic")
                .await
                .map_err(|e| BackendError::Auth(e.to_string()))?
        );
        Ok(self)
    }

    /// Convert spn ChatMessage to Anthropic format
    fn convert_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages.iter().map(|m| {
            serde_json::json!({
                "role": match m.role {
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::System => "user", // Anthropic handles system separately
                },
                "content": m.content
            })
        }).collect()
    }

    /// Extract system message if present
    fn extract_system(&self, messages: &[ChatMessage]) -> Option<String> {
        messages.iter()
            .find(|m| m.role == ChatRole::System)
            .map(|m| m.content.clone())
    }
}

impl ModelBackend for AnthropicBackend {
    fn id(&self) -> &'static str { "anthropic" }
    fn name(&self) -> &'static str { "Anthropic Claude" }

    // Cloud backends are always "running"
    async fn is_running(&self) -> bool {
        self.api_key.is_some()
    }

    async fn start(&self) -> BackendResult<()> {
        // No-op for cloud
        Ok(())
    }

    async fn stop(&self) -> BackendResult<()> {
        // No-op for cloud
        Ok(())
    }

    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        // Return known Claude models
        Ok(vec![
            ModelInfo::new("claude-sonnet", "claude-3-5-sonnet-20241022")
                .with_family("claude")
                .with_parameters(175_000_000_000), // Estimated
            ModelInfo::new("claude-opus", "claude-3-opus-20240229")
                .with_family("claude")
                .with_parameters(200_000_000_000),
            ModelInfo::new("claude-haiku", "claude-3-5-haiku-20241022")
                .with_family("claude")
                .with_parameters(20_000_000_000),
        ])
    }

    async fn model_info(&self, name: &str) -> BackendResult<ModelInfo> {
        self.list_models().await?
            .into_iter()
            .find(|m| m.name == name || m.digest.as_deref() == Some(name))
            .ok_or_else(|| BackendError::ModelNotFound(name.to_string()))
    }

    async fn pull(&self, _name: &str, _progress: Option<ProgressCallback>) -> BackendResult<()> {
        // Cloud models don't need to be pulled
        Ok(())
    }

    async fn delete(&self, _name: &str) -> BackendResult<()> {
        // Cloud models can't be deleted
        Err(BackendError::NotSupported("delete not supported for cloud backends".into()))
    }

    async fn load(&self, _name: &str, _config: &LoadConfig) -> BackendResult<()> {
        // Cloud models are always loaded
        Ok(())
    }

    async fn unload(&self, _name: &str) -> BackendResult<()> {
        // Cloud models can't be unloaded
        Ok(())
    }

    async fn running_models(&self) -> BackendResult<Vec<RunningModel>> {
        // Cloud models don't have running state
        Ok(vec![])
    }

    async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| BackendError::Auth("API key not configured".into()))?;

        let model_id = match model {
            "claude-sonnet" => "claude-3-5-sonnet-20241022",
            "claude-opus" => "claude-3-opus-20240229",
            "claude-haiku" => "claude-3-5-haiku-20241022",
            other => other,
        };

        let system = self.extract_system(messages);
        let messages: Vec<_> = messages.iter()
            .filter(|m| m.role != ChatRole::System)
            .collect();

        let mut body = serde_json::json!({
            "model": model_id,
            "messages": self.convert_messages(&messages.iter().map(|m| (*m).clone()).collect::<Vec<_>>()),
            "max_tokens": options.and_then(|o| o.max_tokens).unwrap_or(4096),
        });

        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        if let Some(opts) = options {
            if let Some(temp) = opts.temperature {
                body["temperature"] = serde_json::Value::from(temp);
            }
        }

        let response = self.client
            .post(format!("{}/messages", ANTHROPIC_API_URL))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::Api(format!("{}: {}", status, text)));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| BackendError::Parse(e.to_string()))?;

        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            message: ChatMessage::assistant(content),
            done: true,
            eval_count: data["usage"]["output_tokens"].as_u64().map(|n| n as u32),
            prompt_eval_count: data["usage"]["input_tokens"].as_u64().map(|n| n as u32),
            total_duration: None,
            load_duration: None,
            prompt_eval_duration: None,
            eval_duration: None,
        })
    }

    async fn chat_stream<F>(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        mut on_token: F,
    ) -> BackendResult<ChatResponse>
    where
        F: FnMut(ChatChunk) + Send,
    {
        // Implement SSE streaming for Anthropic
        // ... (similar to chat but with stream=true and SSE parsing)
        todo!("Implement streaming")
    }

    async fn embed(&self, _model: &str, _input: &str) -> BackendResult<EmbeddingResponse> {
        Err(BackendError::NotSupported("Anthropic does not support embeddings".into()))
    }

    async fn embed_batch(&self, _model: &str, _inputs: &[&str]) -> BackendResult<Vec<EmbeddingResponse>> {
        Err(BackendError::NotSupported("Anthropic does not support embeddings".into()))
    }

    async fn gpu_info(&self) -> BackendResult<Vec<GpuInfo>> {
        // Cloud backends don't expose GPU info
        Ok(vec![])
    }

    fn endpoint_url(&self) -> &str {
        ANTHROPIC_API_URL
    }
}

// Implement DynModelBackend via blanket impl in traits.rs
```

### Task 3: Create Backend Registry

```rust
// crates/spn-backends/src/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use crate::traits::DynModelBackend;

/// Backend identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    // Local backends
    Ollama,

    // Cloud backends
    Anthropic,
    OpenAI,
    Mistral,
    Groq,
    DeepSeek,
    Gemini,

    // Future: Phase B
    // Candle,
    // MistralRs,
}

impl BackendKind {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Mistral => "mistral",
            Self::Groq => "groq",
            Self::DeepSeek => "deepseek",
            Self::Gemini => "gemini",
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Ollama)
    }

    pub fn is_cloud(&self) -> bool {
        !self.is_local()
    }

    /// Parse from string (for config files)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ollama" => Some(Self::Ollama),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "openai" | "gpt" => Some(Self::OpenAI),
            "mistral" => Some(Self::Mistral),
            "groq" => Some(Self::Groq),
            "deepseek" => Some(Self::DeepSeek),
            "gemini" | "google" => Some(Self::Gemini),
            _ => None,
        }
    }
}

/// Factory function type for creating backends
pub type BackendFactory = Box<dyn Fn() -> Arc<dyn DynModelBackend> + Send + Sync>;

/// Runtime registry of available backends
pub struct BackendRegistry {
    factories: HashMap<BackendKind, BackendFactory>,
    instances: HashMap<BackendKind, Arc<dyn DynModelBackend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            instances: HashMap::new(),
        };

        // Register default backends based on features
        #[cfg(feature = "ollama")]
        registry.register(BackendKind::Ollama, || {
            Arc::new(spn_ollama::OllamaBackend::new())
        });

        #[cfg(feature = "cloud-anthropic")]
        registry.register(BackendKind::Anthropic, || {
            Arc::new(crate::cloud::AnthropicBackend::new())
        });

        #[cfg(feature = "cloud-openai")]
        registry.register(BackendKind::OpenAI, || {
            Arc::new(crate::cloud::OpenAIBackend::new())
        });

        // ... register other backends

        registry
    }

    pub fn register<F>(&mut self, kind: BackendKind, factory: F)
    where
        F: Fn() -> Arc<dyn DynModelBackend> + Send + Sync + 'static,
    {
        self.factories.insert(kind, Box::new(factory));
    }

    /// Get or create backend instance (singleton per kind)
    pub fn get(&mut self, kind: BackendKind) -> Option<Arc<dyn DynModelBackend>> {
        if let Some(instance) = self.instances.get(&kind) {
            return Some(Arc::clone(instance));
        }

        if let Some(factory) = self.factories.get(&kind) {
            let instance = factory();
            self.instances.insert(kind, Arc::clone(&instance));
            return Some(instance);
        }

        None
    }

    /// List available backend kinds
    pub fn available(&self) -> Vec<BackendKind> {
        self.factories.keys().copied().collect()
    }

    /// Check if a backend kind is registered
    pub fn has(&self, kind: BackendKind) -> bool {
        self.factories.contains_key(&kind)
    }
}
```

### Task 4: Create Model Alias System

```rust
// crates/spn-backends/src/model_ref.rs

use crate::registry::BackendKind;
use serde::{Deserialize, Serialize};

/// Model reference that can be resolved to a backend + model name
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelRef {
    /// Alias format: @models/llama3.2:8b
    Alias(ModelAlias),

    /// Direct backend reference: ollama:llama3.2:8b
    Direct { backend: String, model: String },

    /// Just the model name (requires default backend)
    Name(String),
}

/// Parsed model alias
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAlias {
    pub name: String,      // e.g., "llama3.2"
    pub variant: Option<String>,  // e.g., "8b"
    pub raw: String,       // Original string
}

impl ModelAlias {
    /// Parse from @models/name:variant format
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix("@models/")?;
        let (name, variant) = if let Some((n, v)) = s.split_once(':') {
            (n.to_string(), Some(v.to_string()))
        } else {
            (s.to_string(), None)
        };

        Some(Self {
            name,
            variant,
            raw: format!("@models/{}", s),
        })
    }

    /// Resolve to backend kind and model name
    pub fn resolve(&self) -> (BackendKind, String) {
        let full_name = if let Some(ref v) = self.variant {
            format!("{}:{}", self.name, v)
        } else {
            self.name.clone()
        };

        // Cloud model aliases
        match self.name.as_str() {
            // Anthropic
            "claude-sonnet" | "claude-opus" | "claude-haiku" => {
                (BackendKind::Anthropic, self.name.clone())
            }

            // OpenAI
            "gpt-4o" | "gpt-4-turbo" | "gpt-4" | "gpt-3.5-turbo" | "o1" | "o1-mini" => {
                (BackendKind::OpenAI, self.name.clone())
            }

            // OpenAI Image
            "dall-e-3" | "dall-e-2" => {
                (BackendKind::OpenAI, self.name.clone())
            }

            // Mistral Cloud
            "codestral" | "mistral-large" | "mistral-medium" => {
                (BackendKind::Mistral, self.name.clone())
            }

            // Groq
            "llama3-groq" | "mixtral-groq" => {
                (BackendKind::Groq, full_name)
            }

            // DeepSeek
            "deepseek-chat" | "deepseek-coder" | "deepseek-r1" => {
                (BackendKind::DeepSeek, self.name.clone())
            }

            // Gemini
            "gemini-pro" | "gemini-flash" | "gemini-ultra" => {
                (BackendKind::Gemini, self.name.clone())
            }

            // Default: Ollama (local)
            _ => (BackendKind::Ollama, full_name)
        }
    }
}

impl ModelRef {
    /// Parse from string (supports multiple formats)
    pub fn parse(s: &str) -> Self {
        if s.starts_with("@models/") {
            if let Some(alias) = ModelAlias::parse(s) {
                return Self::Alias(alias);
            }
        }

        if let Some((backend, model)) = s.split_once(':') {
            if BackendKind::from_str(backend).is_some() {
                return Self::Direct {
                    backend: backend.to_string(),
                    model: model.to_string(),
                };
            }
        }

        Self::Name(s.to_string())
    }

    /// Resolve to backend kind and model name
    pub fn resolve(&self, default_backend: BackendKind) -> (BackendKind, String) {
        match self {
            Self::Alias(alias) => alias.resolve(),
            Self::Direct { backend, model } => {
                let kind = BackendKind::from_str(backend).unwrap_or(default_backend);
                (kind, model.clone())
            }
            Self::Name(name) => (default_backend, name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alias_parse() {
        let alias = ModelAlias::parse("@models/llama3.2:8b").unwrap();
        assert_eq!(alias.name, "llama3.2");
        assert_eq!(alias.variant, Some("8b".to_string()));

        let (backend, model) = alias.resolve();
        assert_eq!(backend, BackendKind::Ollama);
        assert_eq!(model, "llama3.2:8b");
    }

    #[test]
    fn test_cloud_alias() {
        let alias = ModelAlias::parse("@models/claude-sonnet").unwrap();
        let (backend, model) = alias.resolve();
        assert_eq!(backend, BackendKind::Anthropic);
        assert_eq!(model, "claude-sonnet");
    }
}
```

### Task 5: Create ModelOrchestrator

```rust
// crates/spn-backends/src/orchestrator.rs

use crate::{
    registry::{BackendKind, BackendRegistry},
    model_ref::ModelRef,
    traits::DynModelBackend,
    capabilities::BackendCapabilities,
};
use spn_core::{
    BackendError, BackendResult, ChatMessage, ChatOptions, ChatResponse,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Intent for model selection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelIntent {
    FastGeneration,
    DeepReasoning,
    CreativeWriting,
    CodeGeneration,
    Translation,
    ImageGeneration,    // Phase B
    ImageAnalysis,      // Phase B
    SpeechToText,       // Phase B
    TextToSpeech,       // Phase B
}

/// Constraints for model selection
#[derive(Debug, Clone, Default)]
pub struct ModelConstraints {
    /// Force local-only (privacy)
    pub local_only: bool,

    /// Force free models only
    pub free_only: bool,

    /// Prefer speed over quality
    pub prefer_speed: bool,

    /// Prefer quality over speed
    pub prefer_quality: bool,

    /// Maximum cost per 1M tokens (cloud)
    pub max_cost: Option<f64>,
}

/// Configuration for orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Default backend for unqualified model names
    pub default_backend: BackendKind,

    /// Default model for text generation
    pub default_text_model: String,

    /// Default model for code generation
    pub default_code_model: Option<String>,

    /// Fallback chain: try backends in order
    pub fallback_chain: Vec<BackendKind>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_backend: BackendKind::Ollama,
            default_text_model: "llama3.2:8b".to_string(),
            default_code_model: None,
            fallback_chain: vec![
                BackendKind::Ollama,
                BackendKind::Anthropic,
                BackendKind::OpenAI,
            ],
        }
    }
}

/// Orchestrates model requests across multiple backends
pub struct ModelOrchestrator {
    registry: RwLock<BackendRegistry>,
    config: OrchestratorConfig,
}

impl ModelOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            registry: RwLock::new(BackendRegistry::new()),
            config,
        }
    }

    /// Get backend for a model reference
    pub async fn get_backend(&self, model_ref: &ModelRef) -> BackendResult<(Arc<dyn DynModelBackend>, String)> {
        let (kind, model_name) = model_ref.resolve(self.config.default_backend);

        let mut registry = self.registry.write().await;
        let backend = registry.get(kind)
            .ok_or_else(|| BackendError::NotAvailable(kind.id().to_string()))?;

        Ok((backend, model_name))
    }

    /// Resolve intent to a model reference
    pub async fn resolve_intent(
        &self,
        intent: ModelIntent,
        constraints: &ModelConstraints,
    ) -> BackendResult<ModelRef> {
        // Simple intent resolution (Phase C will use llmfit for this)
        let model = match intent {
            ModelIntent::FastGeneration => {
                if constraints.local_only {
                    "@models/llama3.2:8b"
                } else if constraints.prefer_speed {
                    "@models/claude-haiku"
                } else {
                    "@models/llama3.2:8b"
                }
            }
            ModelIntent::DeepReasoning => {
                if constraints.local_only {
                    "@models/llama3.2:70b"
                } else {
                    "@models/claude-sonnet"
                }
            }
            ModelIntent::CreativeWriting => {
                if constraints.local_only {
                    "@models/llama3.2:8b"
                } else {
                    "@models/gpt-4o"
                }
            }
            ModelIntent::CodeGeneration => {
                if constraints.local_only {
                    "@models/codellama:7b"
                } else {
                    "@models/codestral"
                }
            }
            ModelIntent::Translation => {
                "@models/llama3.2:8b"
            }
            // Phase B intents
            ModelIntent::ImageGeneration => {
                return Err(BackendError::NotSupported("Image generation requires Phase B".into()));
            }
            ModelIntent::ImageAnalysis => {
                return Err(BackendError::NotSupported("Image analysis requires Phase B".into()));
            }
            ModelIntent::SpeechToText => {
                return Err(BackendError::NotSupported("Speech-to-text requires Phase B".into()));
            }
            ModelIntent::TextToSpeech => {
                return Err(BackendError::NotSupported("Text-to-speech requires Phase B".into()));
            }
        };

        Ok(ModelRef::parse(model))
    }

    /// Execute chat request with automatic routing
    pub async fn chat(
        &self,
        model_ref: &ModelRef,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse> {
        let (backend, model_name) = self.get_backend(model_ref).await?;

        // Convert to owned types for DynModelBackend
        backend.chat(
            model_name,
            messages.to_vec(),
            options.cloned(),
        ).await
    }

    /// Execute chat with intent-based model selection
    pub async fn chat_with_intent(
        &self,
        intent: ModelIntent,
        constraints: &ModelConstraints,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse> {
        let model_ref = self.resolve_intent(intent, constraints).await?;
        self.chat(&model_ref, messages, options).await
    }

    /// Execute chat with fallback chain
    pub async fn chat_with_fallback(
        &self,
        model_refs: &[ModelRef],
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse> {
        let mut last_error = None;

        for model_ref in model_refs {
            match self.chat(model_ref, messages, options).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::warn!("Backend {} failed: {}", model_ref.resolve(self.config.default_backend).0.id(), e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| BackendError::NotAvailable("No backends available".into())))
    }

    /// List all available models across all backends
    pub async fn list_all_models(&self) -> BackendResult<Vec<(BackendKind, Vec<spn_core::ModelInfo>)>> {
        let mut results = Vec::new();
        let registry = self.registry.read().await;

        for kind in registry.available() {
            // Get backend and list models
            // ... implementation
        }

        Ok(results)
    }
}
```

### Task 6: Update spn.yaml Schema

```rust
// crates/spn/src/manifest/mod.rs

use serde::{Deserialize, Serialize};
use spn_backends::model_ref::ModelRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpnManifest {
    pub name: String,
    pub version: String,

    #[serde(default)]
    pub packages: Vec<String>,

    /// Model dependencies (NEW)
    #[serde(default)]
    pub models: Vec<ModelDependency>,

    /// Model defaults for workflows (NEW)
    #[serde(default)]
    pub model_defaults: ModelDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelDependency {
    /// Simple alias: "@models/llama3.2:8b"
    Alias(String),

    /// Detailed specification
    Detailed {
        name: String,
        #[serde(default)]
        backend: Option<String>,
        #[serde(default)]
        constraints: Option<ModelConstraints>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelDefaults {
    /// Default model for `infer:` steps
    pub text: Option<String>,

    /// Default model for code-related tasks
    pub code: Option<String>,

    /// Default model for image generation (Phase B)
    pub image: Option<String>,

    /// Default model for embeddings
    pub embedding: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelConstraints {
    pub local_only: Option<bool>,
    pub free_only: Option<bool>,
}
```

**Example spn.yaml:**
```yaml
name: my-ai-project
version: 1.0.0

packages:
  - @workflows/seo/content-generator
  - @skills/code-review

models:
  # Simple aliases (auto-resolve backend)
  - @models/llama3.2:8b
  - @models/claude-sonnet
  - @models/gpt-4o

  # Detailed specification
  - name: codellama
    backend: ollama
    constraints:
      local_only: true

model_defaults:
  text: @models/llama3.2:8b
  code: @models/codestral
```

### Task 7: Add MCP Tools for Models

```rust
// crates/spn-mcp/src/tools/models.rs

use rmcp::{Tool, ToolResult};
use spn_backends::{ModelOrchestrator, ModelRef, ModelIntent, ModelConstraints};

/// MCP tool: spn_model_chat
pub struct ModelChatTool {
    orchestrator: Arc<ModelOrchestrator>,
}

impl Tool for ModelChatTool {
    fn name(&self) -> &str { "spn_model_chat" }

    fn description(&self) -> &str {
        "Chat with a model via spn's unified backend system"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "model": {
                    "type": "string",
                    "description": "Model reference (@models/llama3.2:8b, @models/claude-sonnet, etc.)"
                },
                "messages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "role": { "type": "string", "enum": ["user", "assistant", "system"] },
                            "content": { "type": "string" }
                        }
                    }
                },
                "intent": {
                    "type": "string",
                    "enum": ["fast-generation", "deep-reasoning", "creative-writing", "code-generation"],
                    "description": "Optional: let orchestrator choose model based on intent"
                },
                "constraints": {
                    "type": "object",
                    "properties": {
                        "local_only": { "type": "boolean" },
                        "prefer_speed": { "type": "boolean" }
                    }
                }
            },
            "required": ["messages"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let model = params.get("model").and_then(|v| v.as_str());
        let intent = params.get("intent").and_then(|v| v.as_str());
        let messages = parse_messages(&params["messages"])?;
        let constraints = parse_constraints(&params.get("constraints"))?;

        let response = if let Some(model_str) = model {
            let model_ref = ModelRef::parse(model_str);
            self.orchestrator.chat(&model_ref, &messages, None).await?
        } else if let Some(intent_str) = intent {
            let intent = parse_intent(intent_str)?;
            self.orchestrator.chat_with_intent(intent, &constraints, &messages, None).await?
        } else {
            return Err("Either 'model' or 'intent' must be provided".into());
        };

        Ok(serde_json::json!({
            "content": response.message.content,
            "done": response.done,
            "usage": {
                "prompt_tokens": response.prompt_eval_count,
                "completion_tokens": response.eval_count
            }
        }))
    }
}

/// MCP tool: spn_model_list
pub struct ModelListTool {
    orchestrator: Arc<ModelOrchestrator>,
}

impl Tool for ModelListTool {
    fn name(&self) -> &str { "spn_model_list" }

    fn description(&self) -> &str {
        "List available models across all backends"
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        let models = self.orchestrator.list_all_models().await?;

        Ok(serde_json::json!({
            "backends": models.iter().map(|(kind, models)| {
                serde_json::json!({
                    "backend": kind.id(),
                    "models": models.iter().map(|m| {
                        serde_json::json!({
                            "name": m.name,
                            "size_gb": m.size_gb(),
                            "parameters": m.parameter_size
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        }))
    }
}
```

### Task 8: Nika Workflow Integration

**Example workflow using new system:**

```yaml
# generate-content.nika.yaml
workflow: smart-content-pipeline

defaults:
  model: @models/llama3.2:8b

steps:
  # Uses default model
  - infer: "Generate SEO keywords for QR codes"
    use.ctx: keywords

  # Override with cloud model
  - infer: "Analyze competitor strategy and create differentiation plan"
    model: @models/claude-sonnet
    use.ctx: strategy

  # Intent-based selection
  - infer: "Write landing page copy"
    intent: creative-writing
    use.ctx: copy

  # With constraints
  - infer: "Process user data"
    intent: fast-generation
    constraints:
      local_only: true
    use.ctx: processed

  # Invoke MCP tool directly
  - invoke: spn_model_chat
    params:
      model: "@models/gpt-4o"
      messages:
        - role: user
          content: "Summarize: $strategy"
    use.ctx: summary
```

---

## File Changes Summary

| File | Action | LOC |
|------|--------|-----|
| `crates/spn-backends/Cargo.toml` | Create | ~50 |
| `crates/spn-backends/src/lib.rs` | Create | ~30 |
| `crates/spn-backends/src/traits.rs` | Move from spn-ollama | ~200 |
| `crates/spn-backends/src/registry.rs` | Create | ~100 |
| `crates/spn-backends/src/orchestrator.rs` | Create | ~250 |
| `crates/spn-backends/src/model_ref.rs` | Create | ~150 |
| `crates/spn-backends/src/capabilities.rs` | Create | ~50 |
| `crates/spn-backends/src/cloud/mod.rs` | Create | ~20 |
| `crates/spn-backends/src/cloud/anthropic.rs` | Create | ~300 |
| `crates/spn-backends/src/cloud/openai.rs` | Create | ~350 |
| `crates/spn-backends/src/cloud/mistral.rs` | Create | ~250 |
| `crates/spn-backends/src/cloud/groq.rs` | Create | ~200 |
| `crates/spn-backends/src/cloud/deepseek.rs` | Create | ~200 |
| `crates/spn-backends/src/cloud/gemini.rs` | Create | ~250 |
| `crates/spn/src/manifest/mod.rs` | Update | +100 |
| `crates/spn/src/daemon/model_manager.rs` | Update | +50 |
| `crates/spn-mcp/src/tools/models.rs` | Create | ~200 |

**Total:** ~2,750 LOC

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_alias_parsing() {
        let alias = ModelAlias::parse("@models/llama3.2:8b").unwrap();
        assert_eq!(alias.name, "llama3.2");
        assert_eq!(alias.variant, Some("8b".to_string()));
    }

    #[test]
    fn test_cloud_model_resolution() {
        let alias = ModelAlias::parse("@models/claude-sonnet").unwrap();
        let (backend, _) = alias.resolve();
        assert_eq!(backend, BackendKind::Anthropic);
    }

    #[tokio::test]
    async fn test_orchestrator_routing() {
        let orchestrator = ModelOrchestrator::new(Default::default());
        let model_ref = ModelRef::parse("@models/claude-sonnet");
        let (backend, model) = orchestrator.get_backend(&model_ref).await.unwrap();
        assert_eq!(backend.id(), "anthropic");
    }
}
```

### Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires API keys
async fn test_anthropic_backend_chat() {
    let backend = AnthropicBackend::new().with_api_key().await.unwrap();
    let messages = vec![ChatMessage::user("Say hello")];
    let response = backend.chat("claude-sonnet", &messages, None).await.unwrap();
    assert!(!response.message.content.is_empty());
}
```

---

## Verification Checklist

- [ ] `cargo build --workspace` passes
- [ ] `cargo test --workspace` passes (1300+ tests)
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `spn model list` shows models from all backends
- [ ] `spn.yaml` with `models:` section parses correctly
- [ ] `@models/claude-sonnet` routes to Anthropic
- [ ] `@models/llama3.2:8b` routes to Ollama
- [ ] MCP tools `spn_model_chat` and `spn_model_list` work
- [ ] Nika workflow with `model:` field works
- [ ] Streaming works for cloud backends

---

## Commit Strategy

```bash
# Commit 1: Create spn-backends crate structure
feat(backends): create spn-backends crate with trait extraction

# Commit 2: Add registry and model reference system
feat(backends): add BackendRegistry and ModelRef alias system

# Commit 3: Implement cloud backends
feat(backends): add Anthropic, OpenAI, Mistral cloud backends

# Commit 4: Add orchestrator
feat(backends): add ModelOrchestrator with intent routing

# Commit 5: Update spn.yaml schema
feat(manifest): add models section to spn.yaml

# Commit 6: Add MCP tools
feat(mcp): add spn_model_chat and spn_model_list tools

# Commit 7: Update daemon
refactor(daemon): use ModelOrchestrator instead of direct backend
```

---

## Dependencies

**New dependencies:**
- None (uses existing reqwest, tokio, serde)

**Crate changes:**
- `spn-ollama` → becomes optional dep of `spn-backends`
- `spn-cli` → depends on `spn-backends` instead of `spn-ollama`

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Cloud API changes | Abstract behind trait, easy to update |
| API key management | Uses existing spn-keyring |
| Rate limiting | Add retry logic with backoff |
| Cost overruns | Add `free_only` constraint, cost tracking |

---

## Success Criteria

1. **Functional:** All 7 providers work (Ollama + 6 cloud)
2. **Performance:** No regression in Ollama performance
3. **UX:** `@models/` aliases feel natural
4. **Integration:** Nika workflows can use any backend
5. **Extensibility:** Adding Phase B backends is straightforward
