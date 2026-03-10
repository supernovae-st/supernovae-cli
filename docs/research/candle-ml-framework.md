# Research Report: Hugging Face Candle ML Framework

## Summary

Candle is a minimalist ML framework for Rust developed by Hugging Face, focused on serverless inference and lightweight deployments. It provides a PyTorch-like API without Python dependencies, supports CUDA/Metal GPU acceleration, and includes implementations of major models (LLaMA, Whisper, Stable Diffusion). This report evaluates Candle for integration into `spn-providers` as a `CandleBackend`.

## Key Findings

### 1. Architecture and Computation Graph Model

Candle uses a **tensor-based architecture** similar to PyTorch with a `Device` abstraction for hardware portability.

```rust
use candle_core::{Device, Tensor, DType};

// Device abstraction - CPU, CUDA, or Metal
let device = Device::Cpu;
let device = Device::new_cuda(0)?;  // NVIDIA GPU
let device = Device::new_metal(0)?; // Apple Silicon

// Tensor operations mirror PyTorch
let a = Tensor::randn(0f32, 1., (2, 3), &device)?;
let b = Tensor::randn(0f32, 1., (3, 4), &device)?;
let c = a.matmul(&b)?;
```

**Key architectural points:**
- **Eager evaluation** (like PyTorch) - no static graph compilation
- **Device-agnostic tensors** - same code runs on CPU/GPU/Metal
- **No Python GIL** - true multicore parallelism
- **Memory-mapped safetensors** - efficient model loading via `VarBuilder`

**Crate structure:**
```
candle-core          Core tensor operations, Device, DType
candle-nn            Neural network layers (Linear, Conv, LayerNorm, etc.)
candle-transformers  High-level model implementations
candle-examples      Example applications
candle-kernels       CUDA custom kernels
candle-metal-kernels Metal custom kernels
candle-flash-attn    Flash attention v2 (Ampere/Ada/Hopper GPUs only)
candle-onnx          ONNX model evaluation
```

- Source: [Candle GitHub README](https://github.com/huggingface/candle)

### 2. GPU Support (CUDA and Metal)

**CUDA Support:**
```toml
# Cargo.toml
[dependencies]
candle-core = { version = "0.9", features = ["cuda"] }
# For cuDNN acceleration
candle-core = { version = "0.9", features = ["cuda", "cudnn"] }
# For multi-GPU via NCCL
candle-core = { version = "0.9", features = ["cuda", "nccl"] }
```

**Metal Support (Apple Silicon):**
```toml
[dependencies]
candle-core = { version = "0.9", features = ["metal"] }
```

**Device Detection Pattern:**
```rust
use candle_core::{Device, Result};

fn detect_device() -> Result<Device> {
    // Try CUDA first
    #[cfg(feature = "cuda")]
    {
        if let Ok(device) = Device::new_cuda(0) {
            return Ok(device);
        }
    }

    // Try Metal on macOS
    #[cfg(feature = "metal")]
    {
        if let Ok(device) = Device::new_metal(0) {
            return Ok(device);
        }
    }

    // Fallback to CPU (with optional MKL/Accelerate)
    Ok(Device::Cpu)
}
```

**Feature flags summary:**

| Feature | Backend | Requirements |
|---------|---------|--------------|
| `cuda` | NVIDIA CUDA | CUDA 11.8+ toolkit |
| `cudnn` | cuDNN acceleration | cuDNN library |
| `nccl` | Multi-GPU | NCCL library |
| `metal` | Apple Metal | macOS 12+, M1/M2/M3 |
| `mkl` | Intel MKL (CPU) | Intel MKL library |
| `accelerate` | Apple Accelerate (CPU) | macOS |

- Source: [candle-core Cargo.toml](https://github.com/huggingface/candle/blob/main/candle-core/Cargo.toml)

### 3. Models Available

Candle includes implementations of major model architectures:

**Language Models (LLMs):**
- LLaMA v1, v2, v3 (including SOLAR-10.7B variant)
- Mistral 7B, Mixtral 8x7B (MoE)
- Phi 1, 1.5, 2, 3
- Falcon
- Gemma v1, v2
- StarCoder, StarCoder2
- Qwen 1.5, Qwen3 MoE
- RWKV v5, v6
- Yi 6B, 34B
- Mamba (state space model)
- **Quantized versions** (GGML/GGUF support via llama.cpp types)

**Vision Models:**
- Stable Diffusion 1.5, 2.1, XL, Turbo
- YOLO v3, v8 (object detection, pose estimation)
- Segment Anything Model (SAM)
- SegFormer, DINOv2, VGG, ResNet, ViT, EfficientNet

**Audio Models:**
- Whisper (speech-to-text, multilingual)
- EnCodec (audio compression)
- MetaVoice-1B (text-to-speech)
- Parler-TTS (text-to-speech)

**Text/Embedding Models:**
- T5, FlanT5, MADLAD400
- BERT, JinaBERT
- CLIP, BLIP
- TrOCR (OCR)
- Marian-MT (translation)

**Model Formats Supported:**
| Format | Support | Notes |
|--------|---------|-------|
| safetensors | Full | Recommended, memory-mapped |
| GGML | Full | llama.cpp quantized types |
| GGUF | Full | Modern llama.cpp format |
| PyTorch .bin | Partial | Via conversion |
| npz | Full | NumPy format |

- Source: [Candle README Features](https://github.com/huggingface/candle#features)

### 4. Performance Benchmarks

**Direct benchmarks are limited** in public sources, but key performance characteristics:

**Design advantages:**
- **No Python overhead** - eliminates GIL bottleneck
- **Serverless-friendly** - small binary size (~10-50MB vs PyTorch ~2GB)
- **Fast cold starts** - 10x faster than Python (~200-500ms vs 3-5s)
- **Memory efficient** - 80-90% reduction vs Python equivalents

**Reported metrics (from ecosystem):**
- Rust ONNX Runtime: 3-5x faster CPU throughput vs Python
- Rust ONNX Runtime: up to 5x faster GPU throughput
- Flash-attention v2: significant speedup on Ampere+ GPUs

**Stable Diffusion requirements:**
- Requires GPU with >8GB VRAM (or CPU mode, much slower)
- Flash-attention support: Ampere, Ada, or Hopper GPUs only (RTX 3090/4090, A100/H100)

**No direct tok/s benchmarks vs llama.cpp or PyTorch were found in current sources.**

- Source: Perplexity search results on Rust ML performance

### 5. Memory Usage Patterns

**Key memory patterns:**
- **Memory-mapped loading** via `VarBuilder::from_mmaped_safetensors()`
- **Quantization support** - Q4_K_M, Q8_0, etc. via GGML/GGUF
- **No persistent computation graph** - eager execution

```rust
use candle_nn::VarBuilder;

// Memory-mapped loading (efficient for large models)
let vb = unsafe {
    VarBuilder::from_mmaped_safetensors(
        &["model.safetensors"],
        DType::F16,
        &device,
    )?
};
```

**Memory requirements by model:**

| Model | FP16 | Q4_K_M | Notes |
|-------|------|--------|-------|
| LLaMA 7B | ~14GB | ~4GB | |
| LLaMA 13B | ~26GB | ~7GB | |
| LLaMA 70B | ~140GB | ~40GB | Multi-GPU needed |
| Stable Diffusion XL | ~8GB | N/A | VRAM |
| Whisper Large | ~3GB | N/A | |

- Source: Candle examples documentation

### 6. Integration Patterns for CandleBackend

**Recommended architecture for `spn-providers`:**

```rust
use candle_core::{Device, DType, Tensor};
use candle_nn::VarBuilder;
use spn_core::{BackendError, ModelInfo, LoadConfig, GpuInfo};

/// Candle-based model backend for local inference.
pub struct CandleBackend {
    device: Device,
    models_dir: PathBuf,
    loaded_models: HashMap<String, LoadedModel>,
}

struct LoadedModel {
    name: String,
    dtype: DType,
    // Model-specific state (e.g., LlamaModel, WhisperModel)
}

impl CandleBackend {
    /// Create a new Candle backend with automatic device detection.
    pub fn new(models_dir: PathBuf) -> Result<Self, BackendError> {
        let device = Self::detect_best_device()?;
        Ok(Self {
            device,
            models_dir,
            loaded_models: HashMap::new(),
        })
    }

    fn detect_best_device() -> Result<Device, BackendError> {
        #[cfg(feature = "cuda")]
        if let Ok(device) = Device::new_cuda(0) {
            return Ok(device);
        }

        #[cfg(feature = "metal")]
        if let Ok(device) = Device::new_metal(0) {
            return Ok(device);
        }

        Ok(Device::Cpu)
    }

    /// Get GPU information.
    pub fn gpu_info(&self) -> Result<Vec<GpuInfo>, BackendError> {
        match &self.device {
            Device::Cuda(cuda_dev) => {
                // Query CUDA device properties
                todo!("Implement CUDA device query")
            }
            Device::Metal(_) => {
                // Query Metal device properties
                todo!("Implement Metal device query")
            }
            Device::Cpu => Ok(vec![]),
        }
    }
}
```

**Model Loading Pattern (LLaMA example):**

```rust
use candle_transformers::models::llama::{Llama, Config, Cache};
use tokenizers::Tokenizer;
use hf_hub::{api::sync::Api, Repo, RepoType};

impl CandleBackend {
    pub async fn load_llama(
        &mut self,
        model_id: &str,
        config: &LoadConfig,
    ) -> Result<(), BackendError> {
        let api = Api::new()
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;

        let repo = api.repo(Repo::with_revision(
            model_id,
            RepoType::Model,
            "main",
        ));

        // Load config
        let config_file = repo.get("config.json")
            .map_err(|e| BackendError::NetworkError(e.to_string()))?;
        let config: Config = serde_json::from_reader(
            std::fs::File::open(config_file)?
        )?;

        // Memory-mapped safetensors loading
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[repo.get("model.safetensors")?],
                DType::F16, // or DType::BF16 for Ampere+
                &self.device,
            )?
        };

        let model = Llama::load(vb, &config)
            .map_err(|e| BackendError::BackendSpecific(e.to_string()))?;

        // Store loaded model
        self.loaded_models.insert(model_id.to_string(), LoadedModel {
            name: model_id.to_string(),
            dtype: DType::F16,
            // ... model state
        });

        Ok(())
    }
}
```

**Inference Pattern (Whisper):**

```rust
use candle_transformers::models::whisper;

impl CandleBackend {
    pub async fn transcribe(
        &self,
        model_name: &str,
        audio_samples: &[f32],
    ) -> Result<String, BackendError> {
        // Load audio as tensor
        let audio = Tensor::from_vec(
            audio_samples.to_vec(),
            (1, audio_samples.len()),
            &self.device,
        )?;

        // Get loaded model
        let model = self.loaded_models.get(model_name)
            .ok_or(BackendError::ModelNotFound(model_name.to_string()))?;

        // Run inference
        // ... model-specific inference code

        Ok(transcription)
    }
}
```

- Source: Candle examples (quantized, whisper, stable-diffusion)

### 7. Limitations and Edge Cases

**Missing compared to PyTorch:**

| Feature | PyTorch | Candle | Notes |
|---------|---------|--------|-------|
| Autograd/Training | Full | Limited | Candle has basic backward pass, not production-ready |
| Dynamic shapes | Full | Limited | Static shapes preferred |
| Custom ops | Easy | Harder | Need to write Rust/CUDA kernels |
| Model zoo | Huge | Growing | ~50 models vs 100k+ |
| Debugging | Excellent | Basic | No equivalent to PyTorch profiler |
| ONNX export | Full | None | Load-only via candle-onnx |
| Distributed training | Full | NCCL only | Inference-focused |

**Known limitations:**

1. **Training is experimental** - Candle is primarily inference-focused. The `backward()` function exists but is not battle-tested for training.

2. **No dynamic shapes** - Tensor shapes must be known at operation time. Dynamic batching requires workarounds.

3. **Metal limitations** - Some operations may not have Metal kernels. Flash-attention is CUDA-only.

4. **Flash-attention v2** - Only works on Ampere/Ada/Hopper GPUs (compute capability 8.0+).

5. **Model compatibility** - Not all HuggingFace models have Candle implementations. Need to check `candle-transformers`.

6. **Quantization** - Supports GGML/GGUF formats but not all quantization schemes. No native QLoRA.

7. **No streaming API** - Need to implement token-by-token generation manually.

8. **Error handling** - Error types are model-specific, need unified error mapping.

**Edge cases to handle:**

```rust
// Handle device fallback
let device = Device::new_cuda(0).or_else(|_| {
    Device::new_metal(0).or_else(|_| Ok(Device::Cpu))
})?;

// Handle dtype compatibility
let dtype = if device.is_cuda() && supports_bf16(&device) {
    DType::BF16  // Better for Ampere+
} else {
    DType::F16
};

// Handle OOM gracefully
match model.forward(&input) {
    Err(candle_core::Error::Cuda(e)) if e.contains("out of memory") => {
        Err(BackendError::InsufficientMemory)
    }
    Err(e) => Err(BackendError::BackendSpecific(e.to_string())),
    Ok(output) => Ok(output),
}
```

- Source: Candle GitHub issues, error.rs

## Implementation Recommendations for spn-providers CandleBackend

### Recommended Cargo.toml

```toml
[package]
name = "spn-candle"
version = "0.1.0"
edition = "2021"

[dependencies]
spn-core = { version = "0.1", path = "../spn-core" }
candle-core = "0.9"
candle-nn = "0.9"
candle-transformers = "0.9"
tokenizers = "0.20"
hf-hub = "0.3"
safetensors = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
thiserror = "2"

[features]
default = ["cpu"]
cpu = []
cuda = ["candle-core/cuda", "candle-nn/cuda", "candle-transformers/cuda"]
cudnn = ["cuda", "candle-core/cudnn"]
metal = ["candle-core/metal", "candle-nn/metal", "candle-transformers/metal"]
mkl = ["candle-core/mkl"]
accelerate = ["candle-core/accelerate"]
flash-attn = ["candle-flash-attn"]

[target.'cfg(target_os = "macos")'.dependencies]
candle-core = { version = "0.9", features = ["accelerate"] }
```

### Trait Implementation Strategy

```rust
use spn_ollama::{ModelBackend, DynModelBackend, ProgressCallback};
use spn_core::{
    BackendError, ChatMessage, ChatOptions, ChatResponse,
    EmbeddingResponse, GpuInfo, LoadConfig, ModelInfo,
    PullProgress, RunningModel,
};

impl ModelBackend for CandleBackend {
    fn id(&self) -> &'static str { "candle" }
    fn name(&self) -> &'static str { "Candle" }

    async fn is_running(&self) -> bool {
        // Candle is library-based, always "running"
        true
    }

    async fn start(&self) -> Result<(), BackendError> {
        // No server to start
        Ok(())
    }

    async fn stop(&self) -> Result<(), BackendError> {
        // Unload all models
        self.loaded_models.clear();
        Ok(())
    }

    // ... implement other methods
}
```

### Model Support Priority

1. **Phase 1: LLMs** - Quantized LLaMA/Mistral (GGUF support)
2. **Phase 2: Embeddings** - BERT, sentence-transformers
3. **Phase 3: Whisper** - Speech-to-text
4. **Phase 4: Stable Diffusion** - Image generation (optional, high VRAM)

### Architecture Decision

**Recommendation: Separate `spn-candle` crate**

```
spn-core (types)
    |
    +-- spn-ollama (Ollama backend)
    |
    +-- spn-candle (Candle backend) [NEW]
    |
    +-- spn-client (daemon IPC)
```

This keeps Candle dependencies optional and allows users to choose their backend.

## Sources

1. [Candle GitHub Repository](https://github.com/huggingface/candle) - Official documentation and examples
2. [candle-core Cargo.toml](https://github.com/huggingface/candle/blob/main/candle-core/Cargo.toml) - Feature flags
3. [Candle Stable Diffusion Example](https://github.com/huggingface/candle/tree/main/candle-examples/examples/stable-diffusion)
4. [Candle Whisper Example](https://github.com/huggingface/candle/tree/main/candle-examples/examples/whisper)
5. [Candle Quantized LLaMA Example](https://github.com/huggingface/candle/tree/main/candle-examples/examples/quantized)
6. Perplexity AI search results (2025-03) - Performance comparisons, Metal support

## Methodology

- Tools used: Perplexity API (sonar model), GitHub raw content fetch, direct code analysis
- Pages analyzed: 12 (README, examples, source files)
- Time period covered: Candle v0.9.x (current stable, March 2025)

## Confidence Level

**Medium-High** - Architecture and feature documentation is well-covered. Performance benchmarks are limited in public sources. Training capabilities are documented as experimental by Candle maintainers.

## Further Research Suggestions

1. **Benchmark study**: Run tok/s comparisons between Candle, llama.cpp, and Ollama for quantized LLaMA models
2. **Metal performance**: Test Candle Metal backend on M1/M2/M3 for Whisper and LLaMA
3. **Memory profiling**: Measure actual VRAM usage for different model sizes and quantization levels
4. **Training viability**: Evaluate Candle's backward pass for fine-tuning use cases
5. **GGUF compatibility**: Test which GGUF quantization formats work with candle-transformers
