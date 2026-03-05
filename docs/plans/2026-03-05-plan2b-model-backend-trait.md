# Plan 2B: ModelBackend Trait + Nika Multi-Model Schema

**Date:** 2026-03-05
**Version:** v1.0
**Status:** DRAFT
**Depends On:** Plan 2 (Model Management)
**Priority:** HIGH (do NOW, not "later")

---

## Executive Summary

Ce plan ajoute:
1. **ModelBackend trait** → Abstraction pour backends locaux (Ollama, llama.cpp futur)
2. **Nika multi-model schema** → `model:` field at workflow + task level
3. **AST + LSP integration** → Parsing, validation, autocomplete
4. **GPU allocation advanced** → Multi-GPU, CPU-only options

---

## 1. ModelBackend Trait (spn-core)

### 1.1 Trait Definition

```rust
// crates/spn-core/src/backend.rs

/// Progress callback for model downloads
#[derive(Debug, Clone)]
pub struct PullProgress {
    pub status: String,
    pub completed: u64,
    pub total: u64,
    pub digest: Option<String>,
}

impl PullProgress {
    pub fn percent(&self) -> Option<f64> {
        if self.total > 0 {
            Some((self.completed as f64 / self.total as f64) * 100.0)
        } else {
            None
        }
    }
}

/// Model metadata
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub quantization: Option<String>,
    pub family: Option<String>,
    pub parameters: Option<String>,  // "7B", "70B"
    pub digest: Option<String>,
}

/// Currently loaded model
#[derive(Debug, Clone)]
pub struct RunningModel {
    pub name: String,
    pub vram_used: Option<u64>,
    pub gpu_ids: Vec<u32>,
}

/// GPU information
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub id: u32,
    pub name: String,
    pub memory_total: u64,
    pub memory_free: u64,
}

/// Backend errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    NotRunning,
    ModelNotFound(String),
    AlreadyLoaded(String),
    InsufficientMemory,
    NetworkError(String),
    ProcessError(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning => write!(f, "Backend not running"),
            Self::ModelNotFound(m) => write!(f, "Model not found: {}", m),
            Self::AlreadyLoaded(m) => write!(f, "Model already loaded: {}", m),
            Self::InsufficientMemory => write!(f, "Insufficient GPU memory"),
            Self::NetworkError(e) => write!(f, "Network error: {}", e),
            Self::ProcessError(e) => write!(f, "Process error: {}", e),
        }
    }
}

impl std::error::Error for BackendError {}
```

### 1.2 Async Trait (in spn-ollama, uses async fn in traits Rust 1.75+)

```rust
// crates/spn-ollama/src/backend.rs

use spn_core::{ModelInfo, RunningModel, GpuInfo, BackendError, PullProgress};

/// Abstract backend for local model inference
///
/// Implementors: OllamaBackend, LlamaCppBackend (future)
pub trait ModelBackend: Send + Sync {
    /// Backend identifier (e.g., "ollama", "llama-cpp")
    fn id(&self) -> &'static str;

    /// Check if backend server is running
    async fn is_running(&self) -> bool;

    /// Start the backend server
    async fn start(&self) -> Result<(), BackendError>;

    /// Stop the backend server
    async fn stop(&self) -> Result<(), BackendError>;

    /// List all available (downloaded) models
    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError>;

    /// List currently loaded/running models
    async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError>;

    /// List available GPUs
    async fn list_gpus(&self) -> Result<Vec<GpuInfo>, BackendError>;

    /// Pull (download) a model with progress callback
    async fn pull(
        &self,
        name: &str,
        progress: Option<Box<dyn Fn(PullProgress) + Send + Sync>>
    ) -> Result<(), BackendError>;

    /// Delete a model
    async fn delete(&self, name: &str) -> Result<(), BackendError>;

    /// Load a model into memory
    async fn load(&self, name: &str, config: &LoadConfig) -> Result<(), BackendError>;

    /// Unload a model from memory
    async fn unload(&self, name: &str) -> Result<(), BackendError>;

    /// Get the inference endpoint URL
    fn endpoint_url(&self) -> &str;
}

/// Configuration for loading a model
#[derive(Debug, Clone, Default)]
pub struct LoadConfig {
    /// Specific GPUs to use (empty = auto)
    pub gpu_ids: Vec<u32>,
    /// Number of layers to offload to GPU (-1 = all, 0 = none)
    pub gpu_layers: i32,
    /// Context size override
    pub context_size: Option<u32>,
}
```

### 1.3 Ollama Implementation

```rust
// crates/spn-ollama/src/ollama_backend.rs

pub struct OllamaBackend {
    client: OllamaClient,
    server: RwLock<Option<OllamaServer>>,
}

impl OllamaBackend {
    pub fn new() -> Self {
        Self {
            client: OllamaClient::new(),
            server: RwLock::new(None),
        }
    }
}

impl ModelBackend for OllamaBackend {
    fn id(&self) -> &'static str {
        "ollama"
    }

    async fn is_running(&self) -> bool {
        self.client.is_running().await
    }

    async fn start(&self) -> Result<(), BackendError> {
        if self.is_running().await {
            return Ok(());
        }

        let server = OllamaServer::start()
            .map_err(|e| BackendError::ProcessError(e.to_string()))?;

        *self.server.write().await = Some(server);
        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        self.client.list_models().await
            .map_err(|e| BackendError::NetworkError(e.to_string()))
    }

    async fn pull(
        &self,
        name: &str,
        progress: Option<Box<dyn Fn(PullProgress) + Send + Sync>>
    ) -> Result<(), BackendError> {
        self.client.pull_model(name, |p| {
            if let Some(ref cb) = progress {
                cb(p);
            }
        }).await.map_err(|e| BackendError::NetworkError(e.to_string()))
    }

    async fn load(&self, name: &str, config: &LoadConfig) -> Result<(), BackendError> {
        // Ollama auto-loads on first inference
        // For explicit load, we do a minimal generate

        // Set GPU config via environment
        if !config.gpu_ids.is_empty() {
            std::env::set_var("CUDA_VISIBLE_DEVICES",
                config.gpu_ids.iter().map(|g| g.to_string()).collect::<Vec<_>>().join(","));
        }

        if config.gpu_layers != 0 {
            std::env::set_var("OLLAMA_NUM_GPU", config.gpu_layers.to_string());
        }

        // Trigger model load
        self.client.generate(name, "").await
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        Ok(())
    }

    fn endpoint_url(&self) -> &str {
        "http://localhost:11434"
    }

    // ... other implementations
}
```

---

## 2. Nika Multi-Model Schema

### 2.1 YAML Schema

```yaml
# workflow.nika.yaml
name: multi-model-workflow
version: "1.0"

# Workflow-level default model (simple form)
model: anthropic/claude-sonnet-4

# Alternative: model config object (advanced)
# model:
#   default: anthropic/claude-sonnet-4
#   fallback: ollama/llama3.2

tasks:
  # Task 1: Uses workflow default (claude-sonnet-4)
  - id: analyze
    infer: |
      Analyze this code...

  # Task 2: Override with local model (simple form)
  - id: quick-check
    model: ollama/llama3.2
    infer: |
      Quick validation...

  # Task 3: Override with powerful model
  - id: deep-reasoning
    model: anthropic/claude-opus-4
    infer: |
      Complex reasoning...

  # Task 4: Advanced GPU config (object form)
  - id: heavy-inference
    model:
      name: ollama/llama3.2:70b
      gpu: [0, 1]        # Multi-GPU
      layers: 80         # GPU layers to offload
    infer: |
      Heavy computation...

  # Task 5: CPU-only inference
  - id: cpu-task
    model:
      name: ollama/llama3.2:7b
      gpu: []            # Empty = CPU only
    infer: |
      CPU inference...
```

### 2.2 Model String Format

```
provider/model-name[:tag]

Examples:
├── anthropic/claude-opus-4
├── anthropic/claude-sonnet-4
├── anthropic/claude-haiku
├── openai/gpt-4o
├── openai/gpt-4o-mini
├── ollama/llama3.2
├── ollama/llama3.2:70b
├── ollama/qwen2:7b-instruct
├── groq/llama-3.1-70b-versatile
├── mistral/mistral-large
└── deepseek/deepseek-coder
```

### 2.3 AST Types

```rust
// nika/tools/nika/src/ast/model.rs

use crate::ast::Span;

/// Model reference - parsed and validated
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    /// Provider identifier (e.g., "anthropic", "ollama")
    pub provider: String,
    /// Model name (e.g., "claude-opus-4", "llama3.2")
    pub model: String,
    /// Optional tag (e.g., "70b", "instruct")
    pub tag: Option<String>,
    /// Source span for diagnostics
    pub span: Span,
}

impl ModelRef {
    /// Parse "provider/model[:tag]" format
    pub fn parse(s: &str, span: Span) -> Result<Self, ModelParseError> {
        let s = s.trim();

        // Must contain /
        let (provider, rest) = s.split_once('/')
            .ok_or(ModelParseError::MissingSlash { span: span.clone() })?;

        // Validate provider
        if provider.is_empty() {
            return Err(ModelParseError::EmptyProvider { span: span.clone() });
        }

        // Check for tag
        let (model, tag) = if let Some((m, t)) = rest.split_once(':') {
            (m.to_string(), Some(t.to_string()))
        } else {
            (rest.to_string(), None)
        };

        if model.is_empty() {
            return Err(ModelParseError::EmptyModel { span });
        }

        Ok(Self {
            provider: provider.to_string(),
            model,
            tag,
            span,
        })
    }

    /// Check if this is a local model (Ollama, llama.cpp)
    pub fn is_local(&self) -> bool {
        matches!(self.provider.as_str(), "ollama" | "llama-cpp" | "local")
    }

    /// Check if this is a cloud provider
    pub fn is_cloud(&self) -> bool {
        !self.is_local()
    }

    /// Get full model string
    pub fn full_name(&self) -> String {
        match &self.tag {
            Some(tag) => format!("{}/{}:{}", self.provider, self.model, tag),
            None => format!("{}/{}", self.provider, self.model),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelParseError {
    MissingSlash { span: Span },
    EmptyProvider { span: Span },
    EmptyModel { span: Span },
}

/// Advanced model configuration (object form)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelConfig {
    /// Model reference
    pub model: ModelRef,
    /// GPU IDs to use (empty = auto or CPU-only)
    pub gpu: Vec<u32>,
    /// GPU layers to offload (-1 = all, 0 = none)
    pub layers: Option<i32>,
    /// Context size override
    pub context_size: Option<u32>,
}

/// Model specification (can be simple string or advanced config)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSpec {
    /// Simple form: "provider/model[:tag]"
    Simple(ModelRef),
    /// Advanced form with GPU config
    Advanced(ModelConfig),
}

impl ModelSpec {
    /// Get the underlying model reference
    pub fn model_ref(&self) -> &ModelRef {
        match self {
            Self::Simple(r) => r,
            Self::Advanced(c) => &c.model,
        }
    }
}
```

### 2.4 Task and Workflow Updates

```rust
// nika/tools/nika/src/ast/task.rs

pub struct Task {
    pub id: TaskId,
    pub model: Option<ModelSpec>,  // NEW: per-task model override
    pub verb: TaskVerb,
    pub depends_on: Vec<TaskRef>,
    pub condition: Option<Expr>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<Duration>,
    pub span: Span,
}

// nika/tools/nika/src/ast/workflow.rs

pub struct Workflow {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub model: Option<ModelSpec>,  // NEW: workflow-level default
    pub providers: Option<ProvidersConfig>,
    pub tasks: Vec<Task>,
    pub span: Span,
}

impl Workflow {
    /// Resolve effective model for a task
    pub fn effective_model(&self, task: &Task) -> Option<&ModelSpec> {
        task.model.as_ref()
            .or(self.model.as_ref())
    }

    /// Resolve effective model ref (unwrapped)
    pub fn effective_model_ref(&self, task: &Task) -> Option<&ModelRef> {
        self.effective_model(task).map(|s| s.model_ref())
    }
}
```

### 2.5 YAML Parsing

```rust
// nika/tools/nika/src/parser/model.rs

use serde::{Deserialize, Deserializer};

/// Deserialize model field (string or object)
pub fn deserialize_model_spec<'de, D>(deserializer: D) -> Result<Option<ModelSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ModelSpecRaw {
        Simple(String),
        Advanced {
            name: String,
            #[serde(default)]
            gpu: Vec<u32>,
            layers: Option<i32>,
            context_size: Option<u32>,
        },
    }

    let raw: Option<ModelSpecRaw> = Option::deserialize(deserializer)?;

    match raw {
        None => Ok(None),
        Some(ModelSpecRaw::Simple(s)) => {
            let model_ref = ModelRef::parse(&s, Span::default())
                .map_err(serde::de::Error::custom)?;
            Ok(Some(ModelSpec::Simple(model_ref)))
        }
        Some(ModelSpecRaw::Advanced { name, gpu, layers, context_size }) => {
            let model_ref = ModelRef::parse(&name, Span::default())
                .map_err(serde::de::Error::custom)?;
            Ok(Some(ModelSpec::Advanced(ModelConfig {
                model: model_ref,
                gpu,
                layers,
                context_size,
            })))
        }
    }
}
```

---

## 3. LSP Integration

### 3.1 Model Completions

```rust
// nika/tools/nika/src/lsp/completions/model.rs

use spn_core::{KNOWN_PROVIDERS, ProviderCategory};

pub fn complete_model(
    position: Position,
    document: &Document,
    ctx: &LspContext,
) -> Vec<CompletionItem> {
    let mut items = vec![];
    let text_before = document.text_before(position);

    // Check if we're after "model:" or "model: "
    if !is_model_context(&text_before) {
        return items;
    }

    // Extract what's already typed
    let typed = extract_model_prefix(&text_before);

    // If nothing typed, suggest providers
    if typed.is_empty() || !typed.contains('/') {
        for provider in KNOWN_PROVIDERS {
            if provider.category == ProviderCategory::Llm ||
               provider.category == ProviderCategory::Local {
                items.push(CompletionItem {
                    label: format!("{}/", provider.id),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some(provider.description.to_string()),
                    insert_text: Some(format!("{}/", provider.id)),
                    sort_text: Some(format!("0_{}", provider.id)),
                    ..Default::default()
                });
            }
        }
    } else {
        // Provider already typed, complete model names
        let provider_id = typed.split('/').next().unwrap_or("");
        items.extend(complete_models_for_provider(provider_id, ctx));
    }

    items
}

fn complete_models_for_provider(provider: &str, ctx: &LspContext) -> Vec<CompletionItem> {
    match provider {
        "anthropic" => vec![
            model_item("claude-opus-4", "Most capable Claude model"),
            model_item("claude-sonnet-4", "Balanced performance and cost"),
            model_item("claude-haiku", "Fast and efficient"),
        ],
        "openai" => vec![
            model_item("gpt-4o", "Most capable GPT-4"),
            model_item("gpt-4o-mini", "Fast and affordable"),
            model_item("o1-preview", "Reasoning model"),
        ],
        "ollama" => {
            // Query local Ollama for installed models
            if let Some(client) = &ctx.ollama_client {
                if let Ok(models) = client.list_models_sync() {
                    return models.iter().map(|m| {
                        model_item(&m.name, &format!("{:.1}GB", m.size as f64 / 1e9))
                    }).collect();
                }
            }
            // Fallback: popular models
            vec![
                model_item("llama3.2", "Llama 3.2 8B"),
                model_item("llama3.2:70b", "Llama 3.2 70B"),
                model_item("mistral", "Mistral 7B"),
                model_item("codellama", "Code Llama"),
            ]
        }
        "groq" => vec![
            model_item("llama-3.1-70b-versatile", "Fast Llama 70B"),
            model_item("llama-3.1-8b-instant", "Ultra-fast Llama 8B"),
            model_item("mixtral-8x7b-32768", "Mixtral MoE"),
        ],
        _ => vec![],
    }
}

fn model_item(name: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::VALUE),
        detail: Some(detail.to_string()),
        ..Default::default()
    }
}
```

### 3.2 Model Diagnostics

```rust
// nika/tools/nika/src/lsp/diagnostics/model.rs

pub fn validate_model_spec(
    model: &ModelSpec,
    ctx: &ValidationContext,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    let model_ref = model.model_ref();

    // 1. Check provider exists
    if spn_core::find_provider(&model_ref.provider).is_none() {
        diagnostics.push(Diagnostic {
            range: model_ref.span.clone().into(),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("nika-model-E001".into())),
            message: format!("Unknown provider: '{}'. Known providers: anthropic, openai, ollama, groq, mistral, deepseek", model_ref.provider),
            ..Default::default()
        });
        return diagnostics; // Stop here if provider invalid
    }

    // 2. For local providers, check if model is installed
    if model_ref.is_local() {
        if let Some(client) = &ctx.ollama_client {
            match client.has_model_sync(&model_ref.model) {
                Ok(false) => {
                    diagnostics.push(Diagnostic {
                        range: model_ref.span.clone().into(),
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: Some(NumberOrString::String("nika-model-W001".into())),
                        message: format!(
                            "Model '{}' not installed. Run: spn model add {}",
                            model_ref.model,
                            model_ref.full_name()
                        ),
                        ..Default::default()
                    });
                }
                Err(_) => {
                    diagnostics.push(Diagnostic {
                        range: model_ref.span.clone().into(),
                        severity: Some(DiagnosticSeverity::HINT),
                        code: Some(NumberOrString::String("nika-model-H001".into())),
                        message: "Cannot verify model installation (Ollama not running)".into(),
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }
    }

    // 3. Validate GPU config (if advanced form)
    if let ModelSpec::Advanced(config) = model {
        if !config.gpu.is_empty() {
            // Validate GPU IDs exist
            if let Some(gpus) = ctx.available_gpus() {
                let max_gpu = gpus.len() as u32;
                for &gpu_id in &config.gpu {
                    if gpu_id >= max_gpu {
                        diagnostics.push(Diagnostic {
                            range: model_ref.span.clone().into(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("nika-model-E002".into())),
                            message: format!(
                                "GPU {} not available. Available GPUs: 0-{}",
                                gpu_id,
                                max_gpu.saturating_sub(1)
                            ),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    diagnostics
}
```

### 3.3 Hover Information

```rust
// nika/tools/nika/src/lsp/hover/model.rs

pub fn hover_model(model_ref: &ModelRef, ctx: &HoverContext) -> Option<Hover> {
    let provider = spn_core::find_provider(&model_ref.provider)?;

    let mut content = format!("## {}\n\n", model_ref.full_name());
    content.push_str(&format!("**Provider:** {} ({})\n\n", provider.name, provider.id));

    // For local models, show more info
    if model_ref.is_local() {
        if let Some(client) = &ctx.ollama_client {
            if let Ok(Some(info)) = client.model_info_sync(&model_ref.model) {
                content.push_str(&format!("**Size:** {:.1} GB\n", info.size as f64 / 1e9));
                if let Some(q) = &info.quantization {
                    content.push_str(&format!("**Quantization:** {}\n", q));
                }
                if let Some(p) = &info.parameters {
                    content.push_str(&format!("**Parameters:** {}\n", p));
                }
            }
        }
    }

    content.push_str("\n---\n");
    content.push_str(&format!("*Env var: {}*", provider.env_var));

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: Some(model_ref.span.clone().into()),
    })
}
```

---

## 4. GPU Allocation Best Practices

### 4.1 Environment Variables (Ollama)

```bash
# Multi-model concurrent loading
OLLAMA_MAX_LOADED_MODELS=3    # Default: 1, max models in memory

# Parallel requests per model
OLLAMA_NUM_PARALLEL=4         # Default: 1

# GPU layer offloading
OLLAMA_NUM_GPU=99             # -1 = all layers, 0 = CPU only

# Specific GPUs
CUDA_VISIBLE_DEVICES=0,1      # Which GPUs to use
```

### 4.2 UX Commands

```bash
# Auto GPU allocation (default)
spn model start llama3.2

# Single GPU
spn model start llama3.2 --gpu 0

# Multi-GPU (for 70B+ models)
spn model start llama3.2:70b --gpus 0,1

# CPU only
spn model start llama3.2 --cpu

# Custom layer count
spn model start llama3.2 --layers 40
```

### 4.3 Daemon Integration

```rust
// crates/spn/src/daemon/model_manager.rs

pub struct ModelManager {
    backend: Box<dyn ModelBackend>,
    loaded_models: RwLock<HashMap<String, LoadedModelInfo>>,
    gpu_allocator: GpuAllocator,
}

impl ModelManager {
    /// Smart model loading with GPU allocation
    pub async fn load_model(
        &self,
        name: &str,
        config: Option<LoadConfig>
    ) -> Result<(), DaemonError> {
        let config = config.unwrap_or_else(|| self.auto_allocate(name));

        // Check if model needs to be pulled first
        if !self.backend.list_models().await?.iter().any(|m| m.name == name) {
            return Err(DaemonError::ModelNotInstalled(name.to_string()));
        }

        // Load model
        self.backend.load(name, &config).await?;

        // Track allocation
        self.loaded_models.write().await.insert(name.to_string(), LoadedModelInfo {
            name: name.to_string(),
            config,
            loaded_at: std::time::Instant::now(),
        });

        Ok(())
    }

    /// Auto-allocate GPU resources
    fn auto_allocate(&self, model_name: &str) -> LoadConfig {
        // Get model size estimate
        let size_gb = estimate_model_size(model_name);

        // Get available GPU memory
        let gpus = self.gpu_allocator.available_gpus();

        if gpus.is_empty() {
            // CPU-only fallback
            return LoadConfig {
                gpu_ids: vec![],
                gpu_layers: 0,
                context_size: None,
            };
        }

        // Simple heuristic: use first GPU with enough memory
        for gpu in &gpus {
            if gpu.memory_free > size_gb * 1_000_000_000 {
                return LoadConfig {
                    gpu_ids: vec![gpu.id],
                    gpu_layers: -1, // All layers
                    context_size: None,
                };
            }
        }

        // Multi-GPU for large models
        if size_gb > 20.0 && gpus.len() >= 2 {
            return LoadConfig {
                gpu_ids: gpus.iter().take(2).map(|g| g.id).collect(),
                gpu_layers: -1,
                context_size: None,
            };
        }

        // Partial offload as fallback
        LoadConfig {
            gpu_ids: vec![gpus[0].id],
            gpu_layers: 40, // Offload 40 layers, rest on CPU
            context_size: None,
        }
    }
}

fn estimate_model_size(name: &str) -> f64 {
    // Rough estimates for common models
    if name.contains("70b") { 40.0 }
    else if name.contains("13b") { 8.0 }
    else if name.contains("7b") { 4.5 }
    else { 5.0 } // Default estimate
}
```

---

## 5. Implementation Phases

### Phase 2B-1: spn-core Types (1 hour)
- [ ] Add `backend.rs` with types (PullProgress, ModelInfo, BackendError, etc.)
- [ ] Add to lib.rs exports
- [ ] Tests

### Phase 2B-2: ModelBackend Trait (2 hours)
- [ ] Create `crates/spn-ollama/src/backend.rs`
- [ ] Define `ModelBackend` trait
- [ ] Implement `OllamaBackend`
- [ ] Tests

### Phase 2B-3: Nika AST (2 hours)
- [ ] Add `ModelRef`, `ModelConfig`, `ModelSpec` types
- [ ] Update `Task` struct with `model` field
- [ ] Update `Workflow` struct with `model` field
- [ ] YAML parsing with serde

### Phase 2B-4: Nika LSP (3 hours)
- [ ] Model completions (providers + model names)
- [ ] Model diagnostics (validation)
- [ ] Hover info for models
- [ ] Quick fixes (pull missing model)

### Phase 2B-5: Daemon Integration (2 hours)
- [ ] `ModelManager` in daemon
- [ ] Auto GPU allocation
- [ ] IPC protocol for model commands
- [ ] Tests

---

## 6. Testing

### Unit Tests

```rust
#[test]
fn test_model_ref_parse() {
    let span = Span::default();

    // Simple form
    let m = ModelRef::parse("anthropic/claude-opus-4", span.clone()).unwrap();
    assert_eq!(m.provider, "anthropic");
    assert_eq!(m.model, "claude-opus-4");
    assert_eq!(m.tag, None);

    // With tag
    let m = ModelRef::parse("ollama/llama3.2:70b", span.clone()).unwrap();
    assert_eq!(m.provider, "ollama");
    assert_eq!(m.model, "llama3.2");
    assert_eq!(m.tag, Some("70b".to_string()));

    // Invalid
    assert!(ModelRef::parse("no-slash", span.clone()).is_err());
    assert!(ModelRef::parse("/no-provider", span.clone()).is_err());
    assert!(ModelRef::parse("provider/", span).is_err());
}

#[test]
fn test_model_ref_is_local() {
    let span = Span::default();

    let local = ModelRef::parse("ollama/llama3.2", span.clone()).unwrap();
    assert!(local.is_local());

    let cloud = ModelRef::parse("anthropic/claude-opus-4", span).unwrap();
    assert!(!cloud.is_local());
    assert!(cloud.is_cloud());
}
```

### E2E Tests

```yaml
# test-multi-model.nika.yaml
name: multi-model-test
model: ollama/llama3.2

tasks:
  - id: local-task
    infer: "Echo: hello"

  - id: override-task
    model: anthropic/claude-haiku
    infer: "Echo: world"
```

```bash
# Run test
cargo run -p nika -- run test-multi-model.nika.yaml --dry-run
# Expected: Shows model resolution for each task
```

---

## 7. Success Criteria

- [ ] `ModelBackend` trait implemented with Ollama backend
- [ ] Nika parses `model:` field at workflow and task level
- [ ] LSP provides model completions and validation
- [ ] GPU allocation auto-detects and allocates
- [ ] E2E test passes with multi-model workflow
- [ ] Zero clippy warnings
- [ ] All tests pass
