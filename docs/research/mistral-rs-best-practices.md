# Research Report: mistral.rs Best Practices for Optimization

## Summary

This report documents best practices for using mistral.rs (v0.7+) for local LLM inference, focusing on PagedAttention, prefix caching, GPU layer configuration, tokenizer setup, and streaming implementation. The research is based on the official EricLBuehler/mistral.rs repository and docs.rs documentation.

## Key Findings

### 1. PagedAttention Configuration

PagedAttention is a memory-efficient attention mechanism that reduces memory fragmentation during inference using paged key-value caches.

**Benefits:**
- Up to 50% lower VRAM usage for long contexts
- Better handling of batch sizes > 1
- Supports concurrent request processing
- Block-level prefix caching integration

**Requirements:**
- CUDA (Unix-like platforms) or Metal (macOS)
- Block sizes: 8, 16, or 32 (default: 32)
- NOT supported on Windows or CPU-only builds

**Enabling PagedAttention with GgufModelBuilder:**

```rust
use mistralrs::{
    GgufModelBuilder, PagedAttentionMetaBuilder, MemoryGpuConfig,
    PagedCacheType, TextMessages, TextMessageRole,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = GgufModelBuilder::new(
        "path/to/model/directory",
        vec!["model.Q4_K_M.gguf"],
    )
    .with_logging()
    .with_paged_attn(
        PagedAttentionMetaBuilder::default()
            .with_block_size(32)  // Options: 8, 16, 32
            .with_gpu_memory(MemoryGpuConfig::ContextSize(4096))
            .with_paged_cache_type(PagedCacheType::Auto)  // Or F8E4M3 for FP8
            .build()?,
    )
    .build()
    .await?;

    Ok(())
}
```

**MemoryGpuConfig Options:**

| Config | Description | Use Case |
|--------|-------------|----------|
| `MbAmount(usize)` | Fixed MB allocation for KV cache | Known VRAM budget |
| `Utilization(f32)` | Percentage of available memory (0.0-1.0) | Dynamic allocation |
| `ContextSize(usize)` | Allocate for N tokens | Known max context length |

**Caveats:**
- Metal: KV cache is automatically capped to `max_seq_len * max_batch_size` to avoid memory pressure (unified memory)
- CUDA: Uses full available VRAM for maximum concurrency (vLLM approach)
- Override with `--pa-memory-mb` CLI flag if needed

**Source:** [docs/PAGED_ATTENTION.md](https://github.com/EricLBuehler/mistral.rs/blob/master/docs/PAGED_ATTENTION.md)

---

### 2. Prefix Caching (prefix_cache_n)

Prefix caching reuses computed KV cache blocks across requests that share common prefixes (like system prompts).

**How It Works:**
1. **Block Hashing:** Each block gets a unique hash based on content and parent hash
2. **Cache Lookup:** Scheduler checks for matching blocks by hash
3. **Block Reuse:** Matched blocks skip recomputation
4. **LRU Eviction:** Least recently used blocks are evicted first

**Configuration:**

```rust
let model = GgufModelBuilder::new("model_dir", vec!["model.gguf"])
    .with_prefix_cache_n(Some(16))  // Cache 16 prefix sequences (default)
    // .with_prefix_cache_n(None)   // Disable prefix caching
    .build()
    .await?;
```

**Optimal Settings:**

| Scenario | Setting | Rationale |
|----------|---------|-----------|
| Single-user chat | `Some(4-8)` | Few active conversations |
| Multi-user server | `Some(16-32)` | More concurrent sessions |
| Batch processing (same prompt) | `Some(1-4)` | Shared prefix across batch |
| Memory-constrained | `None` or `Some(4)` | Reduce cache overhead |
| High-throughput API | `Some(32+)` | Maximize cache hits |

**Interaction with PagedAttention:**
- When PagedAttention is enabled: Uses **block-level** prefix caching
- Without PagedAttention: Uses **sequence-level** prefix caching
- The `prefix_cache_n` setting controls both systems (mutually exclusive)

**Log Messages:**
- `Prefix caching enabled (block-level, PagedAttention).`
- `Prefix caching enabled (sequence-level, non-paged attention).`

**Source:** [docs/PAGED_ATTENTION.md](https://github.com/EricLBuehler/mistral.rs/blob/master/docs/PAGED_ATTENTION.md)

---

### 3. GPU Layer Configuration (Metal/CUDA)

**Feature Flags:**

| Flag | Platform | Notes |
|------|----------|-------|
| `metal` | macOS | Apple Metal GPU support |
| `accelerate` | macOS | Apple Accelerate framework |
| `cuda` | Linux/WSL | NVIDIA CUDA support |
| `flash-attn` | Linux/WSL | Flash Attention 2/3 (requires cuda) |
| `cudnn` | Linux/WSL | cuDNN acceleration |

**Cargo.toml Configuration:**

```toml
[dependencies.mistralrs]
version = "0.7"
default-features = false
features = []  # CPU-only

# For Metal (macOS):
# features = ["metal"]

# For CUDA (Linux):
# features = ["cuda", "flash-attn"]
```

**Device Mapping:**

```rust
use mistralrs::{GgufModelBuilder, DeviceMapSetting, AutoDeviceMapParams};
use candle_core::Device;

// Automatic device mapping (default)
let model = GgufModelBuilder::new("dir", vec!["model.gguf"])
    .with_device_mapping(DeviceMapSetting::Auto(AutoDeviceMapParams::default()))
    .build()
    .await?;

// Force CPU
let model = GgufModelBuilder::new("dir", vec!["model.gguf"])
    .with_force_cpu()
    .build()
    .await?;

// Explicit device
let model = GgufModelBuilder::new("dir", vec!["model.gguf"])
    .with_device(Device::new_metal(0)?)  // Or Device::new_cuda(0)?
    .build()
    .await?;
```

**Model Size vs VRAM Guidelines:**

| Model Size | Q4_K_M Size | Recommended VRAM | Notes |
|------------|-------------|------------------|-------|
| 7B | ~4GB | 8GB+ | Fits on most GPUs |
| 13B | ~7GB | 16GB+ | M1 Pro/Max, RTX 3080+ |
| 30B | ~17GB | 32GB+ | M2 Ultra, A100 |
| 70B | ~40GB | 48GB+ | Requires multi-GPU or offloading |

**Metal-Specific Notes:**
- Unified memory: GPU and CPU share RAM
- PagedAttention auto-caps KV cache to prevent system pressure
- Use `MISTRALRS_METAL_PRECOMPILE=0` in CI to skip shader compilation

---

### 4. Tokenizer Configuration

**When to Use `with_tok_model_id()`:**

| Scenario | Use `with_tok_model_id()` | Example |
|----------|---------------------------|---------|
| GGUF with embedded tokenizer | NO | Most GGUF files |
| GGUF missing tokenizer | YES | Older/custom GGUFs |
| HuggingFace Safetensors | YES | Non-GGUF models |
| Custom chat template | Optional | Override default |
| Specific tokenizer version | YES | Compatibility fix |

**Code Examples:**

```rust
// GGUF with embedded tokenizer (default - no extra config)
let model = GgufModelBuilder::new("dir", vec!["model.gguf"])
    .build()
    .await?;

// GGUF needing external tokenizer (e.g., Llama 3.1 GGUFs from bartowski)
let model = GgufModelBuilder::new(
    "bartowski/Meta-Llama-3.1-8B-Instruct-GGUF",
    vec!["Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf"],
)
.with_tok_model_id("meta-llama/Meta-Llama-3.1-8B-Instruct")  // Source tokenizer
.build()
.await?;

// Local GGUF with custom chat template
let model = GgufModelBuilder::new("dir", vec!["model.gguf"])
    .with_chat_template("chat_templates/mistral.json")
    .build()
    .await?;

// Explicit tokenizer.json path
let model = GgufModelBuilder::new("dir", vec!["model.gguf"])
    .with_tokenizer_json("/path/to/tokenizer.json")
    .build()
    .await?;
```

**GGUF Embedded Tokenizer:**
- Most modern GGUFs (from TheBloke, bartowski, etc.) include the tokenizer
- mistral.rs auto-detects and uses the embedded tokenizer
- Override only when experiencing tokenization issues

---

### 5. Streaming Implementation

**API Overview:**

| Method | Return Type | Use Case |
|--------|-------------|----------|
| `send_chat_request()` | `ChatCompletionResponse` | Full response (blocking) |
| `stream_chat_request()` | `impl Stream<Item=Response>` | Token-by-token streaming |
| `chat()` | `String` | Simplified one-liner |

**Streaming Implementation:**

```rust
use futures::StreamExt;
use mistralrs::{
    Model, GgufModelBuilder, RequestBuilder, Response,
    ChatCompletionChunkResponse, ChunkChoice, Delta,
    TextMessageRole, TextMessages,
};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = GgufModelBuilder::new("dir", vec!["model.gguf"])
        .with_logging()
        .build()
        .await?;

    // Build request with sampling parameters
    let request = RequestBuilder::new()
        .add_message(TextMessageRole::System, "You are a helpful assistant.")
        .add_message(TextMessageRole::User, "Write a haiku about Rust.")
        .set_sampler_temperature(0.7)
        .set_sampler_max_len(512);

    // Stream the response
    let mut stream = model.stream_chat_request(request).await?;

    let stdout = std::io::stdout();
    let lock = stdout.lock();
    let mut buf = std::io::BufWriter::new(lock);

    while let Some(chunk) = stream.next().await {
        match chunk {
            Response::Chunk(ChatCompletionChunkResponse { choices, .. }) => {
                if let Some(ChunkChoice {
                    delta: Delta { content: Some(content), .. },
                    ..
                }) = choices.first()
                {
                    buf.write_all(content.as_bytes())?;
                    buf.flush()?;  // Flush for real-time output
                }
            }
            Response::Done(_) => {
                println!();  // Final newline
                break;
            }
            Response::Error(e) => {
                eprintln!("\nError: {e}");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
```

**Lifetime Management:**
- The stream holds a reference to the model
- Model must outlive the stream
- Use `Arc<Model>` if sharing across tasks

**Error Handling Pattern:**

```rust
while let Some(chunk) = stream.next().await {
    match chunk {
        Response::Chunk(c) => { /* handle content */ }
        Response::Done(final_response) => {
            // Access final usage stats
            println!("Tokens: {}", final_response.usage.total_tokens);
            break;
        }
        Response::Error(e) => {
            // Handle inference errors
            return Err(e.into());
        }
        Response::ModelError(msg, _) => {
            // Handle model-specific errors
            eprintln!("Model error: {msg}");
            break;
        }
        _ => {}  // Handle other response types
    }
}
```

---

## Recommendations for spn-native Implementation

### Current State Analysis

Your current implementation in `/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn-native/src/inference/runtime.rs`:

```rust
// Current (simplified, missing optimizations)
let model = GgufModelBuilder::new(parent, vec![filename])
    .with_logging()
    .build()
    .await?;
```

### Recommended Improvements

1. **Enable PagedAttention (if Metal feature is enabled):**

```rust
#[cfg(feature = "inference")]
async fn load(&mut self, model_path: PathBuf, config: LoadConfig) -> Result<(), NativeError> {
    use mistralrs::{
        GgufModelBuilder, PagedAttentionMetaBuilder, MemoryGpuConfig,
        paged_attn_supported,
    };

    let parent = model_path.parent().map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let filename = model_path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .ok_or_else(|| NativeError::InvalidConfig("Invalid model path".to_string()))?;

    let mut builder = GgufModelBuilder::new(&parent, vec![&filename])
        .with_logging()
        .with_prefix_cache_n(Some(config.prefix_cache_n.unwrap_or(16)));

    // Enable PagedAttention if supported (Metal or CUDA)
    if paged_attn_supported() {
        let context_size = config.context_length.unwrap_or(4096);
        builder = builder.with_paged_attn(
            PagedAttentionMetaBuilder::default()
                .with_block_size(32)
                .with_gpu_memory(MemoryGpuConfig::ContextSize(context_size))
                .build()
                .map_err(|e| NativeError::InvalidConfig(e.to_string()))?,
        );
    }

    let model = builder.build().await
        .map_err(|e| NativeError::InvalidConfig(format!("Failed to build model: {e}")))?;

    // ... rest of implementation
}
```

2. **Add LoadConfig fields:**

```rust
// In spn-core/src/lib.rs
pub struct LoadConfig {
    pub gpu_layers: Option<i32>,
    pub context_length: Option<usize>,    // NEW: For PagedAttention
    pub prefix_cache_n: Option<usize>,    // NEW: Prefix cache sequences
    pub force_cpu: bool,                  // NEW: Force CPU mode
}
```

3. **Implement Streaming:**

```rust
use futures_util::Stream;
use std::pin::Pin;

pub async fn infer_stream(
    &self,
    prompt: &str,
    options: ChatOptions,
) -> Result<Pin<Box<dyn Stream<Item = Result<String, NativeError>> + Send>>, NativeError> {
    let model = self.model.as_ref()
        .ok_or_else(|| NativeError::InvalidConfig("No model loaded".to_string()))?;

    let model = model.read().await;

    let messages = TextMessages::new()
        .add_message(TextMessageRole::User, prompt);

    let mut request = RequestBuilder::from(messages);
    if let Some(temp) = options.temperature {
        request = request.set_sampler_temperature(f64::from(temp));
    }
    if let Some(max) = options.max_tokens {
        request = request.set_sampler_max_len(max as usize);
    }

    let stream = model.stream_chat_request(request).await
        .map_err(|e| NativeError::InvalidConfig(e.to_string()))?;

    Ok(Box::pin(stream.filter_map(|chunk| async move {
        match chunk {
            Response::Chunk(c) => {
                c.choices.first()
                    .and_then(|ch| ch.delta.content.clone())
                    .map(Ok)
            }
            Response::Error(e) => Some(Err(NativeError::InvalidConfig(e.to_string()))),
            _ => None,
        }
    })))
}
```

4. **Async Model Loading (Wrap in spawn_blocking):**

```rust
pub async fn load(&mut self, model_path: PathBuf, config: LoadConfig) -> Result<(), NativeError> {
    // Build configuration on main thread
    let builder_config = BuilderConfig {
        parent: model_path.parent()...,
        filename: model_path.file_name()...,
        config: config.clone(),
    };

    // Spawn blocking task for heavy I/O
    let model = tokio::task::spawn_blocking(move || {
        // Create runtime for async build inside blocking context
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let builder = GgufModelBuilder::new(...)
                // ... configure builder
                .build()
                .await
        })
    })
    .await
    .map_err(|e| NativeError::InvalidConfig(e.to_string()))?
    .map_err(|e| NativeError::InvalidConfig(e.to_string()))?;

    self.model = Some(Arc::new(RwLock::new(model)));
    Ok(())
}
```

---

## Breaking Changes to Watch (v0.7+)

| Change | Impact | Migration |
|--------|--------|-----------|
| MSRV raised to 1.83.0 | Requires newer Rust | Update toolchain |
| Device mapping API | `DeviceMapper` usage changed | Use `DeviceMapSetting` |
| HF cache path | Now explicit setting | Use `with_hf_cache_path()` if needed |
| Prefix caching fixes | May alter long-context outputs | Test thoroughly |
| Paged attention empty cache | Memory patterns changed | Monitor VRAM usage |

---

## Sources

1. [EricLBuehler/mistral.rs GitHub](https://github.com/EricLBuehler/mistral.rs) - Official repository
2. [docs/PAGED_ATTENTION.md](https://github.com/EricLBuehler/mistral.rs/blob/master/docs/PAGED_ATTENTION.md) - PagedAttention documentation
3. [mistralrs/src/gguf.rs](https://github.com/EricLBuehler/mistral.rs/blob/master/mistralrs/src/gguf.rs) - GgufModelBuilder source
4. [mistralrs/examples/getting_started/streaming/main.rs](https://github.com/EricLBuehler/mistral.rs/blob/master/mistralrs/examples/getting_started/streaming/main.rs) - Streaming example
5. [mistralrs/examples/getting_started/gguf_locally/main.rs](https://github.com/EricLBuehler/mistral.rs/blob/master/mistralrs/examples/getting_started/gguf_locally/main.rs) - Local GGUF example
6. [docs.rs/mistralrs](https://docs.rs/mistralrs) - API documentation

---

## Methodology

- **Tools used:** Perplexity API, GitHub API, direct file fetching
- **Sources analyzed:** 15+ source files from mistral.rs repository
- **Time period covered:** mistral.rs v0.7.x (current as of March 2026)

## Confidence Level

**High** - Based on official repository source code and documentation. API details verified against actual source files.

## Further Research Suggestions

1. **FlashAttention Integration:** Research how to enable FlashAttention v2/v3 with PagedAttention for maximum performance
2. **Multi-GPU Support:** Investigate NCCL-based tensor parallelism for large models
3. **KV Cache Quantization:** Benchmark FP8 (F8E4M3) vs Auto cache types for quality/speed tradeoffs
4. **ISQ (In-Situ Quantization):** Research runtime quantization options for non-GGUF models
