# spn-providers

LLM provider abstraction layer for the SuperNovae ecosystem.

## Overview

`spn-providers` provides unified traits and orchestration for interacting with multiple LLM backends, both cloud-based and local. It enables seamless switching between providers while maintaining a consistent API.

## Features

- **Unified Interface**: Single trait for all LLM providers
- **Model Orchestration**: Route requests to the appropriate backend
- **Backend Registry**: Dynamic backend registration and discovery
- **Feature-Gated**: Compile only the providers you need

## Installation

```toml
[dependencies]
spn-providers = "0.1"

# With specific provider support
spn-providers = { version = "0.1", features = ["anthropic", "openai"] }
```

## Usage

### Backend Registry

```rust,ignore
use spn_providers::{BackendRegistry, ModelBackend};

let mut registry = BackendRegistry::new();

// Register a backend
registry.register("ollama", Box::new(OllamaBackend::new()));

// Get a backend
let backend = registry.get("ollama")?;
```

### Model Orchestrator

```rust,ignore
use spn_providers::ModelOrchestrator;

let orchestrator = ModelOrchestrator::new(registry);

// Resolve a model alias
let backend = orchestrator.resolve("@models/llama3.2:1b")?;
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `anthropic` | Anthropic Claude API | No |
| `openai` | OpenAI API | No |
| `mistral` | Mistral AI API | No |
| `groq` | Groq API | No |
| `deepseek` | DeepSeek API | No |
| `gemini` | Google Gemini API | No |

## Traits

### ModelBackend

```rust,ignore
#[async_trait]
pub trait ModelBackend: Send + Sync {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendsError>;
    async fn model_info(&self, model: &str) -> Result<ModelInfo, BackendsError>;
    async fn pull(&self, model: &str, callback: Option<ProgressCallback>) -> Result<(), BackendsError>;
    async fn delete(&self, model: &str) -> Result<(), BackendsError>;
    async fn load(&self, model: &str) -> Result<(), BackendsError>;
    async fn unload(&self, model: &str) -> Result<(), BackendsError>;
    async fn running_models(&self) -> Result<Vec<String>, BackendsError>;
}
```

## Error Handling

All operations return `Result<T, BackendsError>`:

```rust,ignore
pub enum BackendsError {
    NotFound(String),
    ConnectionFailed(String),
    Timeout(String),
    InvalidResponse(String),
    // ...
}
```

## License

MIT OR Apache-2.0

## Related Crates

- [`spn-core`](https://crates.io/crates/spn-core) - Core types and validation
- [`spn-native`](https://crates.io/crates/spn-native) - Native inference backend
- [`spn-cli`](https://crates.io/crates/spn-cli) - SuperNovae CLI
