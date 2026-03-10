# mistral.rs Integration Plan for spn-native

**Target:** Integrate mistral.rs v0.7.0+ for local LLM inference
**Phase:** 2 (after v0.1.0 release)
**Timeline:** 2-3 months
**Status:** Planning & Architecture

---

## 1. Current State vs. mistral.rs Goals

### What spn-native Does Today
- ✅ Download GGUF models from HuggingFace
- ✅ Verify checksums
- ✅ Detect system RAM (for auto-quantization)
- ✅ Provide ModelStorage trait implementation

### What mistral.rs Adds (Phase 2)
- ⏳ Load GGUF models into memory
- ⏳ Execute inference (forward pass)
- ⏳ Support multiple architectures (Llama, Qwen, Phi, etc.)
- ⏳ GPU acceleration (via candle framework)

### Architecture: Storage Layer → Inference Layer

```
Phase 1 (Current)                Phase 2 (Future)
┌─────────────────────┐          ┌──────────────────────┐
│  HuggingFaceStorage │          │  NativeRuntime       │
│  ├── download()     │          │  ├── load()          │
│  ├── list_models()  │          │  ├── infer()         │
│  └── delete()       │          │  ├── unload()        │
└─────────────────────┘          │  └── stream_infer()  │
         ↓                        └──────────────────────┘
   ~/.spn/models/                      ↓
   (GGUF files on disk)           (mistral.rs backend)
```

---

## 2. Integration Strategy: Layered API Design

### Proposed Module Structure

```
spn-native/src/
├── lib.rs                 # Public API (unchanged)
├── storage.rs             # HuggingFaceStorage (unchanged)
├── platform.rs            # RAM detection (unchanged)
├── error.rs               # Error types (add InferenceError)
│
└── inference/             # NEW: Phase 2
    ├── mod.rs             # Public trait + exports
    ├── runtime.rs         # NativeRuntime struct
    ├── builder.rs         # NativeRuntimeBuilder (pattern)
    ├── context.rs         # ExecutionContext, ChatOptions
    ├── response.rs        # InferenceResponse, streaming
    └── mistral_rs.rs      # mistral.rs specific impl
```

### Clean Dependency Boundary

**Storage (Phase 1):**
```rust
// storage.rs depends on:
use reqwest::Client;
use tokio::fs;
use sha2::Sha256;

// Does NOT depend on:
// - mistral.rs
// - candle
// - llm-chain or similar
```

**Inference (Phase 2):**
```rust
// inference/runtime.rs depends on:
use mistral_rs::prelude::*;
use candle::Tensor;

// Does NOT depend on:
// - reqwest
// - HuggingFaceStorage directly
```

**Top-level consumer (Nika):**
```rust
// nika/src/model_executor.rs
use spn_native::{
    HuggingFaceStorage,       // Phase 1
    NativeRuntime,            // Phase 2
    default_model_dir,
    detect_available_ram_gb,
};
```

---

## 3. mistral.rs Dependency Plan

### Why mistral.rs?

1. **Supports all target models** (Llama, Qwen, Phi, Gemma)
2. **GPU-first architecture** (via candle)
3. **Minimal external dependencies** (pure Rust inference)
4. **Active development** (maintained by Eric Buehler)
5. **Proven in production** (Discord, Anthropic tests)

### Cargo.toml Changes

**Current (Phase 1):**
```toml
[dependencies]
spn-core = "0.2.0"
tokio = { version = "1.48", features = ["full"] }
reqwest = { version = "0.12", features = ["rustls-tls", "stream"] }
# ... other Phase 1 deps
```

**Phase 2 Addition:**
```toml
[dependencies]
# ... existing Phase 1 deps ...

# NEW: mistral.rs integration
[features]
default = []
inference = [
    "dep:mistral-rs",
    "dep:candle-core",
    "dep:candle-nn",
    "dep:tokenizers",
]
native = ["inference"]  # Alias for convenience

[dependencies]
# Optional: mistral.rs (enabled by "inference" feature)
mistral-rs = { version = "0.7", optional = true, features = ["metal"] }
candle-core = { version = "0.4", optional = true }
candle-nn = { version = "0.4", optional = true }
tokenizers = { version = "0.15", optional = true }
```

**Why optional?**
- Headless environments (build spn without GPU support)
- Reduced compile time for CLI tools
- Nika enables `inference` feature when compiling locally
- Cloud runners can skip inference entirely

### Feature Flag Strategy

```toml
# spn-cli (always download-only)
[dependencies]
spn-native = { version = "0.1", default-features = false }

# nika (includes inference)
[dependencies]
spn-native = { version = "0.1", features = ["inference"] }

# Users can also do:
# cargo install spn-cli --features native-inference
```

---

## 4. API Design: InferenceBackend Trait

### Unified Trait (Extensible)

```rust
// inference/mod.rs

use spn_core::{LoadConfig, ChatOptions, ModelInfo, ChatResponse};
use std::path::PathBuf;

/// Trait for any inference backend (mistral.rs, Ollama, llama.cpp, etc.)
pub trait InferenceBackend: Send + Sync {
    /// Load a model from disk.
    async fn load(&mut self, model_path: PathBuf, config: LoadConfig) -> Result<()>;

    /// Unload the model from memory.
    async fn unload(&mut self) -> Result<()>;

    /// Check if a model is currently loaded.
    fn is_loaded(&self) -> bool;

    /// Get metadata about loaded model.
    fn model_info(&self) -> Option<ModelInfo>;

    /// Generate a response (non-streaming).
    async fn infer(&self, prompt: &str, opts: ChatOptions) -> Result<ChatResponse>;

    /// Generate a response (streaming).
    async fn infer_stream(
        &self,
        prompt: &str,
        opts: ChatOptions,
    ) -> Result<impl futures::Stream<Item = Result<String>>>;
}
```

### NativeRuntime Implementation

```rust
// inference/runtime.rs

#[cfg(feature = "inference")]
pub struct NativeRuntime {
    // Configuration
    model_path: PathBuf,
    config: LoadConfig,

    // mistral.rs backend (variant per architecture)
    model: Option<GgufModel>,  // Pseudocode
    tokenizer: Option<Tokenizer>,

    // Execution state
    device: candle::Device,
    metadata: Option<ModelInfo>,
}

#[cfg(feature = "inference")]
impl NativeRuntime {
    pub fn new() -> Self {
        Self {
            model_path: Default::default(),
            config: LoadConfig::default(),
            model: None,
            tokenizer: None,
            device: candle::Device::Cpu,  // or GPU
            metadata: None,
        }
    }

    pub async fn load(&mut self, model_path: PathBuf, config: LoadConfig) -> Result<()> {
        // 1. Detect device (CPU or GPU)
        let device = self.select_device(&config)?;

        // 2. Load GGUF file
        let model_data = tokio::fs::read(&model_path).await?;
        let gguf = GgufFile::from_bytes(&model_data)?;

        // 3. Detect architecture and create appropriate builder
        let arch = detect_architecture(&gguf)?;
        let model = match arch {
            ModelArchitecture::Llama => {
                mistral_rs::TextModelBuilder::new(&model_path)
                    .with_device(&device)
                    .with_gpu_layers(config.gpu_layers)
                    .build()?
            }
            ModelArchitecture::Qwen3 => {
                // Similar for Qwen
            }
            // ... other architectures
        };

        self.model = Some(model);
        self.device = device;
        self.model_path = model_path;
        self.config = config;
        Ok(())
    }

    pub async fn infer(&self, prompt: &str, opts: ChatOptions) -> Result<ChatResponse> {
        let model = self.model.as_ref().ok_or(NativeError::ModelNotLoaded)?;

        // 1. Tokenize input
        let tokens = self.tokenizer.encode(prompt)?;

        // 2. Run inference
        let output_tokens = model.forward(&tokens, &self.device)?;

        // 3. Decode output
        let response = self.tokenizer.decode(&output_tokens)?;

        Ok(ChatResponse {
            content: response,
            tokens_generated: output_tokens.len() as u32,
            // ... other fields
        })
    }
}
```

### Error Extension

```rust
// error.rs: Add inference errors

#[derive(Error, Debug)]
pub enum NativeError {
    // ... existing errors ...

    #[cfg(feature = "inference")]
    #[error("Model not loaded")]
    ModelNotLoaded,

    #[cfg(feature = "inference")]
    #[error("Inference error: {0}")]
    InferenceFailed(String),

    #[cfg(feature = "inference")]
    #[error("Unsupported architecture: {0}")]
    UnsupportedArchitecture(String),

    #[cfg(feature = "inference")]
    #[error("Device error: {0}")]
    DeviceError(String),
}
```

---

## 5. Integration with spn-core Types

### Mapping Table

| spn-core Type | Used By mistral.rs | Notes |
|---------------|-------------------|-------|
| **ModelArchitecture** | Runtime selection | Determines builder (TextModelBuilder vs VisionModelBuilder) |
| **Quantization** | GGUF loading hints | Filename contains quant level; hints GPU layer allocation |
| **LoadConfig** | Forward pass setup | Maps to mistral.rs `GgufModelBuilder.with_gpu_layers()` |
| **ChatOptions** | Inference parameters | Maps to generation kwargs (temperature, top_p, max_tokens) |
| **ModelInfo** | Post-load metadata | Return after successful load |
| **PullProgress** | Not used (inference) | Phase 1 only |

### Example: Auto-quantization Flow

```rust
// nika/src/main.rs
use spn_core::{find_model, auto_select_quantization, LoadConfig};
use spn_native::{HuggingFaceStorage, NativeRuntime, default_model_dir, detect_available_ram_gb};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Detect hardware
    let ram_gb = detect_available_ram_gb();
    println!("Detected: {} GB RAM", ram_gb);

    // 2. Find model & select quantization
    let model = find_model("qwen3:8b")?;
    let quant = auto_select_quantization(model, ram_gb);
    println!("Selected quantization: {}", quant.name());

    // 3. Download (Phase 1)
    let storage = HuggingFaceStorage::new(default_model_dir());
    let request = DownloadRequest::curated(model).with_quantization(quant);
    let download = storage.download(&request, |progress| {
        eprintln!("{}", progress);
    }).await?;
    println!("Downloaded to: {:?}", download.path);

    // 4. Load for inference (Phase 2) - requires "inference" feature
    #[cfg(feature = "inference")]
    {
        let config = LoadConfig::default()
            .with_gpu_layers(-1)        // Use all available GPU layers
            .with_context_size(4096);

        let mut runtime = NativeRuntime::new();
        runtime.load(download.path, config).await?;
        println!("Model loaded!");

        // 5. Run inference
        let response = runtime.infer(
            "What is 2+2?",
            ChatOptions::default()
                .with_temperature(0.7)
                .with_max_tokens(100)
        ).await?;
        println!("Response: {}", response.content);
    }

    Ok(())
}
```

---

## 6. Architecture Diagram: Full Flow

```
User (spn-cli or Nika)
    │
    ├─ Phase 1: Download ─────────────────────┐
    │                                         │
    v                                         v
spn-native::HuggingFaceStorage      ~/.spn/models/model.gguf
    │ download()                       (GGUF file on disk)
    │ ├─ HTTP stream from HF            │
    │ ├─ SHA256 verification            │
    │ └─ Save to disk                   │
    │                                   │
    └─────────────────────────────────────┘
                      │
                      v
    ┌─────────────────────────────────────────────┐
    │ Phase 2: Load & Infer (requires "inference" feature)
    ├─────────────────────────────────────────────┤
    │
    ├─ spn-native::NativeRuntime
    │   ├─ Load GGUF from disk
    │   ├─ Detect architecture
    │   ├─ Create mistral.rs builder
    │   │   ├─ GgufModelBuilder (text models)
    │   │   ├─ VisionModelBuilder (VLMs)
    │   │   └─ EmbeddingModelBuilder (embeddings)
    │   │
    │   ├─ Allocate GPU layers (if GPU available)
    │   ├─ Load tokenizer
    │   └─ Ready for inference
    │
    ├─ infer(prompt, options)
    │   ├─ Tokenize input
    │   ├─ Forward pass via mistral.rs
    │   ├─ Decode output tokens
    │   └─ Return ChatResponse
    │
    └─────────────────────────────────────────────┘
```

---

## 7. GPU Support Strategy

### GPU Layers Configuration

**Current LoadConfig (Phase 1):**
```rust
pub struct LoadConfig {
    pub context_size: u32,
    pub batch_size: u32,
    pub gpu_layers: i32,  // ← Will feed into mistral.rs
}
```

**GPU Layers Semantics:**
```
gpu_layers = -1  → Load ALL layers on GPU (maximum speed)
gpu_layers = 0   → CPU only
gpu_layers = 20  → Load top 20 layers on GPU (mixed mode)
gpu_layers = 100 → Load as many layers as fit on GPU
```

### Device Selection Algorithm

```rust
// inference/runtime.rs

fn select_device(&self, config: &LoadConfig) -> Result<Device> {
    #[cfg(target_os = "macos")]
    {
        // Prefer Metal on macOS if available
        if candle::Device::Metal.is_available() {
            return Ok(candle::Device::Metal);
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Prefer CUDA on Linux (if NVIDIA GPU available)
        if candle::Device::Cuda.is_available() {
            return Ok(candle::Device::Cuda);
        }
    }

    // Fallback to CPU
    Ok(candle::Device::Cpu)
}
```

### Feature Flags for GPU Backends

```toml
[features]
inference = ["dep:mistral-rs"]
metal = ["candle-core/metal"]         # macOS GPU
cuda = ["candle-core/cuda"]           # NVIDIA GPU
all = ["inference", "metal", "cuda"]
```

---

## 8. Testing Strategy

### Unit Tests (Phase 2)

```rust
// tests/inference_tests.rs

#[cfg(feature = "inference")]
mod tests {
    use super::*;
    use spn_native::NativeRuntime;
    use spn_core::LoadConfig;
    use tempfile::tempdir;

    #[tokio::test]
    #[ignore]  // Requires model file
    async fn test_load_gguf() {
        let model_path = PathBuf::from("~/.spn/models/test/model.gguf");
        let config = LoadConfig::default();

        let mut runtime = NativeRuntime::new();
        let result = runtime.load(model_path, config).await;

        assert!(result.is_ok());
        assert!(runtime.is_loaded());
    }

    #[tokio::test]
    #[ignore]
    async fn test_infer_text() {
        // Setup: load a small model
        let mut runtime = setup_runtime().await;

        // Execute
        let response = runtime
            .infer("Hello", ChatOptions::default())
            .await;

        // Assert
        assert!(response.is_ok());
        assert!(!response.unwrap().content.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_streaming_infer() {
        let runtime = setup_runtime().await;
        let mut stream = runtime
            .infer_stream("Hello", ChatOptions::default())
            .await
            .unwrap();

        let mut tokens = Vec::new();
        while let Some(token) = stream.next().await {
            tokens.push(token.unwrap());
        }

        assert!(!tokens.is_empty());
    }
}
```

### Benchmark Tests

```rust
// benches/inference_bench.rs

#[bench]
fn bench_load_gguf(b: &mut Bencher) {
    // Measure time to load model into memory
}

#[bench]
fn bench_infer_simple_prompt(b: &mut Bencher) {
    // Measure time for single inference
}

#[bench]
fn bench_tokenization(b: &mut Bencher) {
    // Measure tokenizer performance
}
```

---

## 9. Release Strategy

### Version Numbering

```
v0.1.0 ─ Storage only (Phase 1) - RELEASED
v0.2.0 ─ Inference (Phase 2)
v0.3.0 ─ Streaming + Advanced (Phase 3)
v0.4.0 ─ Multiple backends (Phase 4)
v1.0.0 ─ (Will this ever happen? Probably not)
```

### Feature Rollout

```
v0.1.x: HuggingFaceStorage, platform detection
v0.2.0: NativeRuntime with inference support
v0.2.1+: GPU optimizations, architecture-specific builders
v0.3.0: Streaming inference, batch processing
v0.4.0: Multiple backends (Ollama, llama.cpp)
```

---

## 10. Migration Path for Ollama → mistral.rs

### Current State (if using Ollama via spn-ollama)
```rust
use spn_ollama::OllamaBackend;
use spn_core::ModelBackend;

let backend = OllamaBackend::connect("http://localhost:11434")?;
let response = backend.infer("prompt", options).await?;
```

### After Phase 2 (mistral.rs available)
```rust
use spn_native::NativeRuntime;
use spn_core::InferenceBackend;

let runtime = NativeRuntime::new();
runtime.load(path, config).await?;
let response = runtime.infer("prompt", options).await?;
```

### Unified Enum (Phase 3+)
```rust
pub enum LocalBackend {
    #[cfg(feature = "inference")]
    Native(NativeRuntime),
    #[cfg(feature = "ollama")]
    Ollama(OllamaBackend),
}

impl LocalBackend {
    pub async fn infer(&self, prompt: &str, opts: ChatOptions) -> Result<String> {
        match self {
            #[cfg(feature = "inference")]
            Self::Native(rt) => rt.infer(prompt, opts).await,
            #[cfg(feature = "ollama")]
            Self::Ollama(ob) => ob.infer(prompt, opts).await,
        }
    }
}
```

---

## 11. Parallel Development

### Phase 1 (Current - spn v0.1.0)
**Owner:** @thibaut
- [ ] Finalize HuggingFaceStorage
- [ ] Release spn v0.1.0
- [ ] Review and approve architecture

### Phase 2 (spn-native v0.2.0)
**Owner:** @thibaut + team
- [ ] Review mistral.rs v0.7.0 API
- [ ] Create `inference/` module structure
- [ ] Implement NativeRuntime
- [ ] Add feature flags
- [ ] Write tests (with `#[ignore]`)
- [ ] Release spn-native v0.2.0

### Phase 2b (Nika Integration)
**Owner:** Nika team
- [ ] Update Nika to depend on `spn-native` (with "inference" feature)
- [ ] Remove Ollama integration (optional; can co-exist)
- [ ] Add NativeRuntime to Nika's execution engine
- [ ] Test end-to-end workflow

### Phase 3 (Streaming & Advanced)
- [ ] Implement streaming inference
- [ ] Add batch processing
- [ ] Optimize GPU memory management

---

## 12. Known Unknowns & Decisions Needed

### Decision 1: Ollama vs. mistral.rs Roadmap
**Question:** Should spn-native completely replace Ollama, or co-exist?

**Options:**
- A) Phase mistral.rs in, deprecate Ollama (cleaner, break Nika users)
- B) Support both simultaneously (more maintenance, flexible)
- C) Keep Ollama for server scenarios, mistral.rs for local

**Recommendation:** Option A (replace, not co-exist). Simplifies testing and maintenance.

### Decision 2: Streaming Inference Priority
**Question:** Is streaming output important for Nika workflows?

**Options:**
- A) Focus on simple request/response first (v0.2)
- B) Build streaming from the start (more complex, better UX)

**Recommendation:** Option A. Implement streaming in v0.3 after stabilizing v0.2.

### Decision 3: Vision Model Support
**Question:** Should Phase 2 include VLMs (Llava, Idefics, etc.)?

**Options:**
- A) Text models only in v0.2; add VLMs in v0.3
- B) Build unified TextModelBuilder + VisionModelBuilder from start

**Recommendation:** Option A. Reduces Phase 2 scope; VLMs add complexity.

---

## 13. Success Criteria

### Phase 2 Success
- [ ] NativeRuntime loads and infers on CPU (all target archs)
- [ ] GPU inference works on macOS (Metal) and Linux (CUDA, if available)
- [ ] Error messages are clear and actionable
- [ ] <2s infer time for 8B models on 16GB RAM systems
- [ ] 100% of tests pass (including integration tests)
- [ ] Documentation is complete

### Nika Integration Success
- [ ] Nika can run full workflows offline (no Ollama required)
- [ ] Users report >90% parity with Ollama-based workflows
- [ ] Inference latency is acceptable (<2s per token on typical hardware)

---

## Summary Table

| Phase | Timeline | Feature | Owner | Status |
|-------|----------|---------|-------|--------|
| 1 | Done | HuggingFaceStorage | @thibaut | ✅ Released v0.1.0 |
| 2 | 2-3 mo | NativeRuntime (text) | @thibaut | ⏳ Planning |
| 2b | +1 mo | Nika integration | Nika team | ⏳ Planning |
| 3 | +2 mo | Streaming, advanced | @thibaut | ⏳ Future |
| 4 | +3 mo | Multiple backends | @thibaut | ⏳ Future |

---

**Next Steps:**
1. Review this plan with team
2. Create GitHub issues for Phase 2 work
3. Set up feature branch for inference/ module
4. Begin mistral.rs v0.7.0 API review
