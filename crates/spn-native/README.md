# spn-native

Native model inference and storage for the SuperNovae ecosystem.

## Overview

`spn-native` provides local GGUF model inference using [mistral.rs](https://github.com/EricLBuehler/mistral.rs) as the backend. This enables running large language models locally without requiring external services like Ollama.

## Features

- **Local GGUF Inference**: Run quantized models (Q4, Q5, Q8) directly on your hardware
- **Hugging Face Integration**: Download models from the Hugging Face Hub
- **Streaming Support**: Stream responses token-by-token
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **Feature-Gated**: Compile only what you need

## Installation

```toml
[dependencies]
spn-native = "0.1"

# With inference support (requires Rust 1.85+)
spn-native = { version = "0.1", features = ["inference"] }
```

## Usage

### Basic Inference

```rust,ignore
use spn_native::{NativeRuntime, LoadConfig, ChatOptions};

// Create runtime
let mut runtime = NativeRuntime::new();

// Load a GGUF model
let config = LoadConfig::default();
runtime.load("path/to/model.gguf", config).await?;

// Run inference
let response = runtime.infer("Hello, world!", ChatOptions::default()).await?;
println!("{}", response.message.content);
```

### Streaming

```rust,ignore
use spn_native::NativeRuntime;

let runtime = NativeRuntime::new();
runtime.load("model.gguf", Default::default()).await?;

let stream = runtime.infer_stream("Tell me a story", Default::default()).await?;
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.delta);
}
```

### Hugging Face Download

```rust,ignore
use spn_native::HuggingFaceStorage;

let storage = HuggingFaceStorage::new()?;
let path = storage.download("Qwen/Qwen3-8B-GGUF", "qwen3-8b-q4_k_m.gguf").await?;
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `inference` | Enable GGUF inference via mistral.rs | No |
| `huggingface` | Enable HF Hub downloads | No |

## Requirements

- **Rust 1.85+** (when `inference` feature enabled)
- **Metal** (macOS) or **CUDA** (Linux/Windows) for GPU acceleration

## Model Compatibility

Supports all GGUF models compatible with mistral.rs:
- Qwen 3
- Llama 3.x
- Mistral / Mixtral
- Phi-3
- And more...

## License

AGPL-3.0-or-later

## Related Crates

- [`spn-core`](https://crates.io/crates/spn-core) - Core types and validation
- [`spn-cli`](https://crates.io/crates/spn-cli) - SuperNovae CLI
