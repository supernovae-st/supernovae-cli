# spn-ollama

Ollama backend for SuperNovae model management.

## Features

- **ModelBackend Trait**: Abstraction for local LLM backends
- **Ollama Implementation**: Full support for Ollama REST API
- **Streaming Progress**: Real-time download progress during model pulls
- **Process Management**: Start/stop Ollama server
- **GPU Support**: Model loading with GPU configuration

## Usage

```rust
use spn_ollama::{OllamaBackend, ModelBackend, LoadConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = OllamaBackend::new();

    // Check if Ollama is running
    if !backend.is_running().await {
        backend.start().await?;
    }

    // List installed models
    for model in backend.list_models().await? {
        println!("{} ({})", model.name, model.size_human());
    }

    // Pull a model with progress callback
    backend.pull("llama3.2:7b", Some(Box::new(|progress| {
        println!("{}: {:.1}%", progress.status, progress.percent());
    }))).await?;

    // Load with full GPU acceleration
    let config = LoadConfig::default()
        .with_gpu_layers(-1)  // All layers on GPU
        .with_context_size(4096);

    backend.load("llama3.2:7b", &config).await?;

    // Check running models
    for model in backend.running_models().await? {
        println!("Loaded: {}", model);
    }

    Ok(())
}
```

## ModelBackend Trait

The `ModelBackend` trait provides a unified interface for local LLM management:

```rust
pub trait ModelBackend: Send + Sync {
    fn id(&self) -> &'static str;
    async fn is_running(&self) -> bool;
    async fn start(&self) -> Result<(), BackendError>;
    async fn stop(&self) -> Result<(), BackendError>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError>;
    async fn pull(&self, name: &str, progress: Option<ProgressCallback>) -> Result<(), BackendError>;
    async fn load(&self, name: &str, config: &LoadConfig) -> Result<(), BackendError>;
    async fn unload(&self, name: &str) -> Result<(), BackendError>;
    async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError>;
    // ...
}
```

## Future Backends

The trait is designed to support additional backends:

- **llama.cpp**: Via HTTP server mode (OpenAI-compatible API)
- **vLLM**: High-performance inference server

## Dependencies

- `spn-core`: Core types (ModelInfo, LoadConfig, etc.)
- `reqwest`: HTTP client with streaming support
- `tokio`: Async runtime for process management
- `serde`: JSON serialization for Ollama API

## License

MIT OR Apache-2.0
