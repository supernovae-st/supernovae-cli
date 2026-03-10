//! Model orchestrator for routing requests via @models/ aliases.
//!
//! The orchestrator resolves model aliases like `@models/claude-sonnet` to
//! the correct backend and model name, then routes chat and embedding
//! requests accordingly.

use crate::{BackendKind, BackendRegistry, BackendsError};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Model reference with backend and model name.
///
/// Represents a fully-qualified model identifier like:
/// - `anthropic:claude-sonnet-4-20250514`
/// - `openai:gpt-4o`
/// - `ollama:llama3.2:8b`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelRef {
    /// The backend kind.
    pub backend: BackendKind,
    /// The model name/identifier.
    pub model: String,
}

impl ModelRef {
    /// Create a new model reference.
    #[must_use]
    pub fn new(backend: BackendKind, model: impl Into<String>) -> Self {
        Self {
            backend,
            model: model.into(),
        }
    }

    /// Create an Anthropic model reference.
    #[must_use]
    pub fn anthropic(model: impl Into<String>) -> Self {
        Self::new(BackendKind::Anthropic, model)
    }

    /// Create an OpenAI model reference.
    #[must_use]
    pub fn openai(model: impl Into<String>) -> Self {
        Self::new(BackendKind::OpenAI, model)
    }

    /// Create an Ollama model reference.
    #[must_use]
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::new(BackendKind::Ollama, model)
    }

    /// Create a Groq model reference.
    #[must_use]
    pub fn groq(model: impl Into<String>) -> Self {
        Self::new(BackendKind::Groq, model)
    }

    /// Create a Mistral model reference.
    #[must_use]
    pub fn mistral(model: impl Into<String>) -> Self {
        Self::new(BackendKind::Mistral, model)
    }

    /// Create a DeepSeek model reference.
    #[must_use]
    pub fn deepseek(model: impl Into<String>) -> Self {
        Self::new(BackendKind::DeepSeek, model)
    }

    /// Create a Gemini model reference.
    #[must_use]
    pub fn gemini(model: impl Into<String>) -> Self {
        Self::new(BackendKind::Gemini, model)
    }
}

impl std::fmt::Display for ModelRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.backend, self.model)
    }
}

impl FromStr for ModelRef {
    type Err = BackendsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse "backend:model" format
        if let Some((backend_str, model)) = s.split_once(':') {
            // Reject empty model names
            if model.is_empty() {
                return Err(BackendsError::InvalidAlias(s.to_string()));
            }
            let backend = BackendKind::from_str(backend_str)
                .map_err(|_| BackendsError::InvalidAlias(s.to_string()))?;
            Ok(Self::new(backend, model))
        } else {
            Err(BackendsError::InvalidAlias(s.to_string()))
        }
    }
}

/// Model alias like `@models/claude-sonnet`.
///
/// Aliases provide short, memorable names for specific model configurations.
/// They can map to different backends depending on availability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelAlias {
    /// The alias name (without @models/ prefix).
    pub name: String,
    /// The primary model reference.
    pub primary: ModelRef,
    /// Fallback model references (in priority order).
    pub fallbacks: Vec<ModelRef>,
    /// Human-readable description.
    pub description: Option<String>,
}

impl ModelAlias {
    /// Create a new model alias.
    #[must_use]
    pub fn new(name: impl Into<String>, primary: ModelRef) -> Self {
        Self {
            name: name.into(),
            primary,
            fallbacks: Vec::new(),
            description: None,
        }
    }

    /// Add a fallback model reference.
    #[must_use]
    pub fn with_fallback(mut self, fallback: ModelRef) -> Self {
        self.fallbacks.push(fallback);
        self
    }

    /// Add a description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Get the full alias string with @models/ prefix.
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("@models/{}", self.name)
    }
}

/// Model orchestrator for routing requests.
///
/// The orchestrator manages model aliases and routes requests to the
/// appropriate backend based on the alias configuration.
///
/// # Example
///
/// ```rust
/// use spn_providers::{ModelOrchestrator, ModelAlias, ModelRef, BackendKind, BackendRegistry};
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// let registry = Arc::new(RwLock::new(BackendRegistry::new()));
/// let mut orchestrator = ModelOrchestrator::new(registry);
///
/// // Register a custom alias
/// orchestrator.register_alias(
///     ModelAlias::new("my-model", ModelRef::anthropic("claude-sonnet-4-20250514"))
///         .with_fallback(ModelRef::openai("gpt-4o"))
///         .with_description("Best coding model")
/// );
///
/// // Resolve alias to model reference
/// if let Ok(model_ref) = orchestrator.resolve("@models/my-model") {
///     assert_eq!(model_ref.backend, BackendKind::Anthropic);
/// }
/// ```
pub struct ModelOrchestrator {
    /// Registered model aliases.
    aliases: FxHashMap<String, ModelAlias>,
    /// Backend registry (used in Phase 2 for routing requests).
    #[allow(dead_code)]
    registry: Arc<RwLock<BackendRegistry>>,
}

impl ModelOrchestrator {
    /// Create a new orchestrator with the given registry.
    #[must_use]
    pub fn new(registry: Arc<RwLock<BackendRegistry>>) -> Self {
        let mut orchestrator = Self {
            aliases: FxHashMap::default(),
            registry,
        };
        orchestrator.register_default_aliases();
        orchestrator
    }

    /// Register a model alias.
    pub fn register_alias(&mut self, alias: ModelAlias) {
        self.aliases.insert(alias.name.clone(), alias);
    }

    /// Unregister a model alias.
    pub fn unregister_alias(&mut self, name: &str) -> Option<ModelAlias> {
        self.aliases.remove(name)
    }

    /// Get an alias by name.
    #[must_use]
    pub fn get_alias(&self, name: &str) -> Option<&ModelAlias> {
        // Strip @models/ prefix if present
        let name = name.strip_prefix("@models/").unwrap_or(name);
        self.aliases.get(name)
    }

    /// List all registered aliases.
    #[must_use]
    pub fn list_aliases(&self) -> Vec<&ModelAlias> {
        let mut aliases: Vec<_> = self.aliases.values().collect();
        aliases.sort_by_key(|a| &a.name);
        aliases
    }

    /// Resolve a model alias or direct reference to a ModelRef.
    ///
    /// Accepts:
    /// - `@models/claude-sonnet` → resolves alias
    /// - `anthropic:claude-sonnet-4-20250514` → direct reference
    /// - `llama3.2:8b` → assumes Ollama
    pub fn resolve(&self, input: &str) -> Result<ModelRef, BackendsError> {
        // Handle @models/ alias
        if let Some(alias_name) = input.strip_prefix("@models/") {
            if let Some(alias) = self.aliases.get(alias_name) {
                return Ok(alias.primary.clone());
            }
            return Err(BackendsError::ModelNotFound(input.to_string()));
        }

        // Handle direct backend:model reference
        if input.contains(':') {
            // Check if it's a backend prefix (anthropic:, openai:, etc.)
            if let Some((prefix, _)) = input.split_once(':') {
                if BackendKind::from_str(prefix).is_ok() {
                    return ModelRef::from_str(input);
                }
            }

            // Assume it's an Ollama model with tag (e.g., llama3.2:8b)
            return Ok(ModelRef::ollama(input));
        }

        // Bare model name - try to find matching alias first
        if let Some(alias) = self.aliases.get(input) {
            return Ok(alias.primary.clone());
        }

        // Assume it's an Ollama model
        Ok(ModelRef::ollama(input))
    }

    /// Register default model aliases.
    fn register_default_aliases(&mut self) {
        // Anthropic models
        self.register_alias(
            ModelAlias::new("claude-opus", ModelRef::anthropic("claude-opus-4-20250514"))
                .with_description("Most capable Claude model"),
        );
        self.register_alias(
            ModelAlias::new("claude-sonnet", ModelRef::anthropic("claude-sonnet-4-20250514"))
                .with_description("Balanced Claude model"),
        );
        self.register_alias(
            ModelAlias::new("claude-haiku", ModelRef::anthropic("claude-haiku-3-5-20241022"))
                .with_description("Fast, lightweight Claude model"),
        );

        // OpenAI models
        self.register_alias(
            ModelAlias::new("gpt-4o", ModelRef::openai("gpt-4o"))
                .with_description("GPT-4 Omni - multimodal"),
        );
        self.register_alias(
            ModelAlias::new("gpt-4o-mini", ModelRef::openai("gpt-4o-mini"))
                .with_description("Smaller GPT-4o variant"),
        );
        self.register_alias(
            ModelAlias::new("o1", ModelRef::openai("o1"))
                .with_description("OpenAI reasoning model"),
        );
        self.register_alias(
            ModelAlias::new("o3-mini", ModelRef::openai("o3-mini"))
                .with_description("OpenAI small reasoning model"),
        );

        // Groq models (fast inference)
        self.register_alias(
            ModelAlias::new("groq-llama70b", ModelRef::groq("llama-3.3-70b-versatile"))
                .with_description("Llama 70B on Groq - ultra fast"),
        );
        self.register_alias(
            ModelAlias::new("groq-llama8b", ModelRef::groq("llama-3.1-8b-instant"))
                .with_description("Llama 8B on Groq - instant"),
        );
        self.register_alias(
            ModelAlias::new("groq-mixtral", ModelRef::groq("mixtral-8x7b-32768"))
                .with_description("Mixtral on Groq"),
        );

        // Mistral models
        self.register_alias(
            ModelAlias::new("mistral-large", ModelRef::mistral("mistral-large-latest"))
                .with_description("Mistral Large - flagship model"),
        );
        self.register_alias(
            ModelAlias::new("codestral", ModelRef::mistral("codestral-latest"))
                .with_description("Mistral code model"),
        );

        // DeepSeek models
        self.register_alias(
            ModelAlias::new("deepseek-chat", ModelRef::deepseek("deepseek-chat"))
                .with_description("DeepSeek chat model"),
        );
        self.register_alias(
            ModelAlias::new("deepseek-coder", ModelRef::deepseek("deepseek-coder"))
                .with_description("DeepSeek code model"),
        );
        self.register_alias(
            ModelAlias::new("deepseek-reasoner", ModelRef::deepseek("deepseek-reasoner"))
                .with_description("DeepSeek R1 reasoning model"),
        );

        // Gemini models
        self.register_alias(
            ModelAlias::new("gemini-pro", ModelRef::gemini("gemini-1.5-pro"))
                .with_description("Gemini 1.5 Pro"),
        );
        self.register_alias(
            ModelAlias::new("gemini-flash", ModelRef::gemini("gemini-1.5-flash"))
                .with_description("Gemini 1.5 Flash - fast"),
        );
        self.register_alias(
            ModelAlias::new("gemini-2", ModelRef::gemini("gemini-2.0-flash"))
                .with_description("Gemini 2.0 Flash"),
        );

        // Local models (Ollama)
        self.register_alias(
            ModelAlias::new("llama3.2", ModelRef::ollama("llama3.2:latest"))
                .with_description("Llama 3.2 latest"),
        );
        self.register_alias(
            ModelAlias::new("llama3.2-8b", ModelRef::ollama("llama3.2:8b"))
                .with_description("Llama 3.2 8B"),
        );
        self.register_alias(
            ModelAlias::new("llama3.2-70b", ModelRef::ollama("llama3.2:70b"))
                .with_description("Llama 3.2 70B"),
        );
        self.register_alias(
            ModelAlias::new("qwen2.5", ModelRef::ollama("qwen2.5:latest"))
                .with_description("Qwen 2.5"),
        );
        self.register_alias(
            ModelAlias::new("phi4", ModelRef::ollama("phi4:latest"))
                .with_description("Microsoft Phi-4"),
        );
        self.register_alias(
            ModelAlias::new("deepseek-r1", ModelRef::ollama("deepseek-r1:latest"))
                .with_description("DeepSeek R1 local"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::shared_registry;

    #[test]
    fn test_model_ref_new() {
        let model = ModelRef::new(BackendKind::Anthropic, "claude-sonnet-4-20250514");
        assert_eq!(model.backend, BackendKind::Anthropic);
        assert_eq!(model.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_model_ref_convenience_constructors() {
        assert_eq!(
            ModelRef::anthropic("claude-3").backend,
            BackendKind::Anthropic
        );
        assert_eq!(ModelRef::openai("gpt-4").backend, BackendKind::OpenAI);
        assert_eq!(ModelRef::ollama("llama3").backend, BackendKind::Ollama);
        assert_eq!(ModelRef::groq("mixtral").backend, BackendKind::Groq);
    }

    #[test]
    fn test_model_ref_from_str() {
        let model: ModelRef = "anthropic:claude-3".parse().unwrap();
        assert_eq!(model.backend, BackendKind::Anthropic);
        assert_eq!(model.model, "claude-3");

        let model: ModelRef = "openai:gpt-4o".parse().unwrap();
        assert_eq!(model.backend, BackendKind::OpenAI);
        assert_eq!(model.model, "gpt-4o");
    }

    #[test]
    fn test_model_ref_from_str_invalid() {
        assert!("invalid".parse::<ModelRef>().is_err());
    }

    #[test]
    fn test_model_ref_from_str_empty_model() {
        // "anthropic:" should be rejected (empty model name)
        assert!("anthropic:".parse::<ModelRef>().is_err());
        assert!("openai:".parse::<ModelRef>().is_err());
    }

    #[test]
    fn test_model_ref_display() {
        let model = ModelRef::anthropic("claude-3");
        assert_eq!(model.to_string(), "anthropic:claude-3");
    }

    #[test]
    fn test_model_alias_new() {
        let alias = ModelAlias::new("claude-sonnet", ModelRef::anthropic("claude-sonnet-4-20250514"));
        assert_eq!(alias.name, "claude-sonnet");
        assert_eq!(alias.primary.backend, BackendKind::Anthropic);
        assert!(alias.fallbacks.is_empty());
    }

    #[test]
    fn test_model_alias_with_fallback() {
        let alias = ModelAlias::new("fast-coding", ModelRef::groq("llama-3.3-70b-versatile"))
            .with_fallback(ModelRef::anthropic("claude-sonnet-4-20250514"))
            .with_description("Fast coding model");

        assert_eq!(alias.fallbacks.len(), 1);
        assert!(alias.description.is_some());
    }

    #[test]
    fn test_model_alias_full_name() {
        let alias = ModelAlias::new("claude-sonnet", ModelRef::anthropic("claude-3"));
        assert_eq!(alias.full_name(), "@models/claude-sonnet");
    }

    #[test]
    fn test_orchestrator_resolve_alias() {
        let registry = shared_registry();
        let orchestrator = ModelOrchestrator::new(registry);

        // Resolve @models/ alias
        let model = orchestrator.resolve("@models/claude-sonnet").unwrap();
        assert_eq!(model.backend, BackendKind::Anthropic);
        assert!(model.model.contains("claude"));
    }

    #[test]
    fn test_orchestrator_resolve_direct_ref() {
        let registry = shared_registry();
        let orchestrator = ModelOrchestrator::new(registry);

        // Resolve direct reference
        let model = orchestrator.resolve("anthropic:claude-opus-4-20250514").unwrap();
        assert_eq!(model.backend, BackendKind::Anthropic);
        assert_eq!(model.model, "claude-opus-4-20250514");
    }

    #[test]
    fn test_orchestrator_resolve_ollama_tag() {
        let registry = shared_registry();
        let orchestrator = ModelOrchestrator::new(registry);

        // Model with tag should be assumed as Ollama
        let model = orchestrator.resolve("llama3.2:8b").unwrap();
        assert_eq!(model.backend, BackendKind::Ollama);
        assert_eq!(model.model, "llama3.2:8b");
    }

    #[test]
    fn test_orchestrator_resolve_bare_name() {
        let registry = shared_registry();
        let orchestrator = ModelOrchestrator::new(registry);

        // Bare name should match alias first
        let model = orchestrator.resolve("claude-sonnet").unwrap();
        assert_eq!(model.backend, BackendKind::Anthropic);
    }

    #[test]
    fn test_orchestrator_list_aliases() {
        let registry = shared_registry();
        let orchestrator = ModelOrchestrator::new(registry);

        let aliases = orchestrator.list_aliases();
        assert!(!aliases.is_empty());

        // Should have some default aliases
        let names: Vec<_> = aliases.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"claude-sonnet"));
        assert!(names.contains(&"gpt-4o"));
        assert!(names.contains(&"llama3.2"));
    }

    #[test]
    fn test_orchestrator_register_custom_alias() {
        let registry = shared_registry();
        let mut orchestrator = ModelOrchestrator::new(registry);

        orchestrator.register_alias(
            ModelAlias::new("my-model", ModelRef::ollama("custom:latest")),
        );

        let model = orchestrator.resolve("@models/my-model").unwrap();
        assert_eq!(model.backend, BackendKind::Ollama);
        assert_eq!(model.model, "custom:latest");
    }

    #[test]
    fn test_orchestrator_resolve_not_found() {
        let registry = shared_registry();
        let orchestrator = ModelOrchestrator::new(registry);

        let result = orchestrator.resolve("@models/nonexistent");
        assert!(matches!(result, Err(BackendsError::ModelNotFound(_))));
    }
}
