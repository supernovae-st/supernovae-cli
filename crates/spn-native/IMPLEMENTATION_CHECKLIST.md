# spn-native Implementation Checklist

**Current Version:** v0.1.0 (Storage & Download)
**Next Version:** v0.2.0 (Inference with mistral.rs)
**Prepared by:** Rust Architect Review

---

## Immediate Actions (This Sprint)

### Code Quality & Documentation

- [ ] **Add comprehensive module docs**
  ```rust
  // src/inference/mod.rs
  //! Local inference backend using mistral.rs.
  //!
  //! # Features
  //! - Requires `inference` feature: `spn-native = { version = "0.2", features = ["inference"] }`
  //! - Supports all mistral.rs architectures (Llama, Qwen, Gemma, etc.)
  //! - GPU acceleration (Metal on macOS, CUDA on Linux)
  //! - Streaming inference for better UX
  ```

- [ ] **Review error handling gaps**
  - [ ] Add `NativeError::ModelNotLoaded`
  - [ ] Add `NativeError::InferenceFailed(String)`
  - [ ] Add `NativeError::UnsupportedArchitecture(String)`
  - [ ] Add proper error message for missing "inference" feature

- [ ] **Fix HTTP status code handling**
  ```rust
  // Current: treats 404 and 500 identically
  match response.status() {
      StatusCode::NOT_FOUND => {
          return Err(NativeError::ModelNotFound { repo, filename });
      }
      _ if !response.status().is_success() => {
          return Err(NativeError::Http(
              format!("HTTP {}: {}", response.status(), ...)
          ));
      }
      _ => {}
  }
  ```

- [ ] **Extract magic numbers to constants**
  ```rust
  const BYTES_PER_GB: u64 = 1_073_741_824;
  const KB_PER_GB: u64 = 1_048_576;
  const WINDOWS_RAM_DEFAULT: u32 = 16;
  const FALLBACK_RAM_DEFAULT: u32 = 8;
  ```

- [ ] **Add path traversal safety validation** (optional, low priority)
  ```rust
  fn validate_model_id(model_id: &str) -> Result<()> {
      if model_id.contains("..") {
          return Err(NativeError::InvalidConfig(".. not allowed in model ID".into()));
      }
      Ok(())
  }
  ```

### Testing

- [ ] **Add more comprehensive error tests**
  ```rust
  #[test]
  fn test_checksum_mismatch_cleanup() {
      // Verify corrupted file is deleted
  }

  #[test]
  fn test_interrupted_download_cleanup() {
      // Verify partial downloads don't corrupt cache
  }
  ```

- [ ] **Add property-based tests** (optional)
  ```rust
  use proptest::prelude::*;

  proptest! {
      #[test]
      fn prop_model_path_safe(repo in "[a-z0-9/-]{1,50}") {
          // Verify no path traversal
      }
  }
  ```

- [ ] **Mark integration tests with `#[ignore]`**
  ```rust
  #[tokio::test]
  #[ignore]  // Run with: cargo test -- --ignored --test-threads 1
  async fn test_real_huggingface_download() {
      // Requires network access
  }
  ```

---

## Phase 2 Preparation (Pre-Implementation)

### Architectural Decisions

- [ ] **Confirm mistral.rs version**
  - Target: v0.7.0 or later
  - Check: https://github.com/EricLBuehler/mistral.rs/releases
  - Action: Document minimum supported version

- [ ] **Decide on streaming architecture**
  - Option A: Return `Box<dyn Stream<Item = Token>>`
  - Option B: Use `futures::channel::mpsc` with spawned task
  - Option C: Callback-based (like current `download()`)
  - **Recommendation:** Option B (cleaner for async/await)

- [ ] **Plan GPU layer allocation strategy**
  - How to auto-detect available VRAM?
  - How to gracefully fall back to CPU?
  - Should we expose this in LoadConfig?

### Dependency Management

- [ ] **Create feature flag matrix**
  ```toml
  [features]
  default = []
  inference = ["dep:mistral-rs"]
  metal = ["candle-core/metal"]
  cuda = ["candle-core/cuda"]
  all = ["inference", "metal", "cuda"]
  ```

- [ ] **Pin mistral.rs version**
  - Add to Cargo.toml with exact version
  - Document breaking changes if any
  - Create MISTRAL_RS_CHANGELOG.md tracking compatibility

- [ ] **Test feature combinations**
  ```bash
  cargo test --no-default-features
  cargo test --features inference
  cargo test --all-features
  ```

### Documentation

- [ ] **Create MISTRAL_RS_INTEGRATION_PLAN.md** ✅ (Done)

- [ ] **Create GPU_SETUP.md**
  ```markdown
  # GPU Setup Guide

  ## macOS (Metal)
  - Automatic via candle-core/metal feature
  - Works on all Apple Silicon & Intel with Metal support

  ## Linux (CUDA)
  - Requires CUDA 12.0+
  - NVIDIA driver 525.0+
  - cargo build --features cuda

  ## Windows
  - TODO: Document
  ```

- [ ] **Create TROUBLESHOOTING.md**
  ```markdown
  # Troubleshooting

  ## "Model not found" errors
  - Check HuggingFace repo is public
  - Verify exact model name

  ## Out of memory
  - Try lower quantization (Q4 instead of F16)
  - Check system RAM with: spn-native detect-ram
  - Reduce context_size in LoadConfig

  ## GPU not detected
  - macOS: Check System Report > GPU
  - Linux: Run nvidia-smi
  ```

---

## Phase 2 Implementation (Detailed Task Breakdown)

### Module: inference/mod.rs

- [ ] **Define InferenceBackend trait**
  ```rust
  pub trait InferenceBackend: Send + Sync {
      async fn load(&mut self, model_path: PathBuf, config: LoadConfig) -> Result<()>;
      async fn unload(&mut self) -> Result<()>;
      fn is_loaded(&self) -> bool;
      fn model_info(&self) -> Option<ModelInfo>;
      async fn infer(&self, prompt: &str, opts: ChatOptions) -> Result<ChatResponse>;
  }
  ```

- [ ] **Re-export public types**
  ```rust
  pub use spn_core::{ChatOptions, ChatResponse, LoadConfig, ModelArchitecture};
  pub use crate::NativeError;

  #[cfg(feature = "inference")]
  pub use self::runtime::NativeRuntime;
  ```

- [ ] **Write module documentation with examples**

### Module: inference/runtime.rs

- [ ] **Create NativeRuntime struct**
  ```rust
  pub struct NativeRuntime {
      model_path: Option<PathBuf>,
      config: LoadConfig,

      #[cfg(feature = "inference")]
      model: Option<GgufModel>,

      #[cfg(feature = "inference")]
      tokenizer: Option<Tokenizer>,

      device: Option<Device>,
      metadata: Option<ModelInfo>,
  }
  ```

- [ ] **Implement load()**
  - [ ] Read GGUF file from disk
  - [ ] Parse GGUF header (detect architecture)
  - [ ] Select device (CPU/GPU)
  - [ ] Create appropriate builder based on architecture
  - [ ] Load tokenizer
  - [ ] Populate metadata
  - [ ] Handle errors gracefully

- [ ] **Implement infer()**
  - [ ] Validate model is loaded
  - [ ] Tokenize input
  - [ ] Run forward pass
  - [ ] Decode output
  - [ ] Return ChatResponse

- [ ] **Implement infer_stream()** (Phase 2b)
  - [ ] Return async stream of tokens
  - [ ] Handle interrupts gracefully

- [ ] **Implement unload()**
  - [ ] Clear model from memory
  - [ ] Release GPU resources

### Module: inference/builder.rs (Optional, Phase 2+)

- [ ] **Create builder pattern for NativeRuntime**
  ```rust
  pub struct NativeRuntimeBuilder {
      model_path: Option<PathBuf>,
      config: LoadConfig,
      device: Option<Device>,
  }

  impl NativeRuntimeBuilder {
      pub fn new() -> Self { ... }
      pub fn model_path(mut self, path: PathBuf) -> Self { ... }
      pub fn with_gpu_layers(mut self, layers: i32) -> Self { ... }
      pub fn force_cpu(mut self) -> Self { ... }
      pub fn build(self) -> Result<NativeRuntime> { ... }
  }
  ```

### Error Handling: error.rs

- [ ] **Add InferenceError variants**
  ```rust
  #[derive(Error, Debug)]
  pub enum NativeError {
      // ... existing variants ...

      #[error("Model not loaded")]
      #[cfg(feature = "inference")]
      ModelNotLoaded,

      #[error("Inference failed: {0}")]
      #[cfg(feature = "inference")]
      InferenceFailed(String),

      #[error("Unsupported architecture: {0}")]
      #[cfg(feature = "inference")]
      UnsupportedArchitecture(String),

      #[error("Device error: {0}")]
      #[cfg(feature = "inference")]
      DeviceError(String),

      #[error("Tokenizer error: {0}")]
      #[cfg(feature = "inference")]
      TokenizerError(String),
  }
  ```

- [ ] **Add feature-gated conversions**
  ```rust
  #[cfg(feature = "inference")]
  impl From<mistral_rs::MistralError> for NativeError {
      fn from(err: mistral_rs::MistralError) -> Self {
          NativeError::InferenceFailed(err.to_string())
      }
  }
  ```

### Tests: tests/inference.rs

- [ ] **Create test module with `#[ignore]` tests**
  ```rust
  #[cfg(feature = "inference")]
  mod inference_tests {
      #[tokio::test]
      #[ignore]
      async fn test_load_gguf() { ... }

      #[tokio::test]
      #[ignore]
      async fn test_infer_text() { ... }

      #[tokio::test]
      #[ignore]
      async fn test_unload() { ... }
  }
  ```

- [ ] **Document how to run ignored tests**
  ```bash
  # Run inference tests (requires model file)
  cargo test --test inference -- --ignored --test-threads 1
  ```

### Integration: lib.rs

- [ ] **Update public API**
  ```rust
  mod inference;

  #[cfg(feature = "inference")]
  pub use inference::NativeRuntime;

  #[cfg(feature = "inference")]
  pub use spn_core::{ChatOptions, ChatResponse};
  ```

- [ ] **Update module-level documentation**
  ```rust
  //! # Example: Download and Infer
  //!
  //! ```ignore
  //! #[cfg(feature = "inference")]
  //! #[tokio::main]
  //! async fn main() -> Result<(), Box<dyn std::error::Error>> {
  //!     // Download
  //!     let storage = HuggingFaceStorage::new(default_model_dir());
  //!     let model = find_model("qwen3:8b")?;
  //!     let request = DownloadRequest::curated(model);
  //!     let result = storage.download(&request, |_| {}).await?;
  //!
  //!     // Infer
  //!     let config = LoadConfig::default().with_gpu_layers(-1);
  //!     let mut runtime = NativeRuntime::new();
  //!     runtime.load(result.path, config).await?;
  //!     let response = runtime.infer("Hello", ChatOptions::default()).await?;
  //!     println!("{}", response.content);
  //!
  //!     Ok(())
  //! }
  //! ```
  ```

---

## Phase 2b: Nika Integration

### In nika/Cargo.toml

- [ ] **Add spn-native with inference feature**
  ```toml
  [dependencies]
  spn-native = { path = "../supernovae-cli/crates/spn-native", features = ["inference"] }
  ```

### In nika/src/

- [ ] **Create model_executor module**
  ```rust
  pub mod model_executor {
      use spn_native::NativeRuntime;
      use spn_core::LoadConfig;

      pub struct ModelExecutor {
          runtime: NativeRuntime,
          loaded_model: Option<String>,
      }

      impl ModelExecutor {
          pub async fn load_and_infer(&mut self, prompt: &str) -> Result<String> { ... }
      }
  }
  ```

- [ ] **Wire into Nika's execution engine**
  - [ ] Hook model loading commands
  - [ ] Hook infer commands
  - [ ] Handle errors from NativeRuntime

- [ ] **Update Nika tests to use local inference**
  - [ ] Remove Ollama dependency tests
  - [ ] Add feature guards: `#[cfg(feature = "native-inference")]`

### In nika/docs/

- [ ] **Create NATIVE_INFERENCE.md**
  - [ ] How to enable locally
  - [ ] GPU setup instructions
  - [ ] Troubleshooting guide
  - [ ] Performance expectations

---

## Phase 3: Streaming & Advanced

### Streaming Support

- [ ] **Implement infer_stream() properly**
  - [ ] Return async stream of tokens
  - [ ] Handle cancellation
  - [ ] Test with long outputs

- [ ] **Update Nika TUI to show streaming output**
  - [ ] Display tokens as they arrive
  - [ ] Show estimated tokens/sec
  - [ ] Allow user to stop generation

### Performance Optimization

- [ ] **Implement token caching** (KV cache management)
- [ ] **Add batch processing** for multiple requests
- [ ] **Profile GPU memory usage**
- [ ] **Optimize context window allocation**

---

## Quality Gates: Before Release

### Code Quality

- [ ] **Zero clippy warnings**
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

- [ ] **Format check**
  ```bash
  cargo fmt --check
  ```

- [ ] **Documentation tests pass**
  ```bash
  cargo test --doc
  ```

- [ ] **All unit tests pass**
  ```bash
  cargo test --lib
  ```

### Integration Testing

- [ ] **Manual test on macOS (Metal)**
  ```bash
  cargo run --example load-infer --features inference,metal
  ```

- [ ] **Manual test on Linux (CUDA, if available)**
  ```bash
  cargo run --example load-infer --all-features
  ```

- [ ] **Manual test on Windows (CPU fallback)**
  ```bash
  cargo run --example load-infer --features inference
  ```

### Documentation

- [ ] **All public items have doc comments**
  ```bash
  cargo doc --no-deps --open  # Check for red items
  ```

- [ ] **Examples compile and run**
  ```bash
  cargo test --example '*' --features inference
  ```

- [ ] **README.md is up-to-date**

### Performance Benchmarks

- [ ] **Model load time < 30 seconds** (for typical 8B model)
- [ ] **Inference latency < 2 seconds/token** (on 16GB RAM)
- [ ] **Memory footprint reasonable** (no memory leaks)

---

## Release Checklist

### Pre-Release (v0.2.0)

- [ ] **Bump version in Cargo.toml**
  ```toml
  [package]
  name = "spn-native"
  version = "0.2.0"
  ```

- [ ] **Update CHANGELOG.md**
  ```markdown
  ## [0.2.0] - 2024-MM-DD

  ### Added
  - Inference support via mistral.rs
  - NativeRuntime for local model execution
  - GPU acceleration (Metal, CUDA)
  - Streaming inference support

  ### Changed
  - Re-export ChatOptions, ChatResponse from spn-core

  ### Fixed
  - HTTP status code handling (distinguish 404 vs 500)
  ```

- [ ] **Tag release**
  ```bash
  git tag -a v0.2.0 -m "Add mistral.rs inference support"
  git push origin v0.2.0
  ```

- [ ] **Publish to crates.io**
  ```bash
  cargo publish
  ```

### Post-Release

- [ ] **Update downstream repos**
  - [ ] nika: bump spn-native dependency
  - [ ] docs: add inference examples

---

## Known Limitations & TODOs

### Current (v0.1.0)
- [ ] Windows RAM detection not implemented (uses 16GB default)
- [ ] No retry logic for network failures
- [ ] No streaming downloads

### Phase 2 (v0.2.0)
- [ ] VLMs not yet supported (text models only)
- [ ] No fine-tuning or LoRA support
- [ ] Single model loaded at a time

### Phase 3+
- [ ] Multi-model swapping (hot-swap)
- [ ] Distributed inference across devices
- [ ] Advanced quantization techniques

---

## Success Metrics

### Before Phase 2 Release
- [ ] 100% unit test pass rate
- [ ] Zero clippy warnings
- [ ] Documentation coverage > 95%
- [ ] Inference latency < 2 sec/token on reference hardware

### After Nika Integration
- [ ] Nika can run workflows offline
- [ ] Performance parity with Ollama ± 10%
- [ ] User feedback rating > 4.0/5.0

---

## Quick Reference: Command Summary

```bash
# Development
cargo test                           # Unit tests
cargo test -- --ignored             # Integration tests
cargo clippy -- -D warnings          # Lint
cargo fmt                            # Format
cargo doc --no-deps --open          # View docs

# Build
cargo build --release                # Native
cargo build --features inference     # With inference
cargo build --all-features          # All features

# Publish
cargo publish --dry-run              # Verify
cargo publish                        # Publish to crates.io

# Testing inference (Phase 2+)
cargo test --test inference -- --ignored
cargo run --example load-infer --features inference
```

---

**Last Updated:** 2026-03-10
**Status:** Ready for Phase 2 implementation
**Reviewed by:** Rust Architect (Claude)
