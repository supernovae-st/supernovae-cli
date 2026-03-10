# spn Roadmap to v1.0.0

> **The ULTIMATE AI Toolkit** - From package manager to full-stack AI development platform

**Current Version:** v0.16.0
**Target:** v1.0.0
**Timeline:** Q2 2026 - Q4 2026
**Philosophy:** 0.x.x forever for rapid iteration, 1.0.0 = production-ready full-stack AI platform

---

## Vision

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn v1.0.0 — The Complete AI Development Platform                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │   MANAGE    │  │    RUN      │  │    TRAIN    │  │   DEPLOY    │            │
│  │   v0.15.x   │  │   v0.16-17  │  │   v0.18-19  │  │   v1.0.0    │            │
│  ├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤            │
│  │ • Packages  │  │ • Local LLM │  │ • Swarms    │  │ • Serve     │            │
│  │ • Secrets   │  │ • Hardware  │  │ • Fine-tune │  │ • Edge      │            │
│  │ • MCP/Sync  │  │ • Recommend │  │ • LoRA/QLoRA│  │ • Kubernetes│            │
│  │ • Daemon    │  │ • Inference │  │ • Dashboard │  │ • WASM      │            │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘            │
│        ✅              Phase B+C        Phase D+E        Phase F               │
│     COMPLETE           IN PROGRESS       PLANNED         PLANNED               │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Current State (v0.15.4)

### What We Have

| Feature | Status | Details |
|---------|--------|---------|
| **Package Management** | :white_check_mark: Complete | add, remove, install, search, update, outdated |
| **Provider Management** | :white_check_mark: Complete | 7 LLM + 8 MCP providers, OS keychain, migration |
| **Cloud Backends** | :white_check_mark: Complete | spn-providers with Anthropic, OpenAI, Mistral, Groq, DeepSeek, Gemini |
| **MCP Server Management** | :white_check_mark: Complete | 48 aliases, auto-sync, foreign adoption |
| **Editor Sync** | :white_check_mark: Complete | Claude Code, Cursor, VS Code, Windsurf |
| **Daemon Architecture** | :white_check_mark: Complete | Unix IPC, peer verification, job scheduler |
| **Model Management** | :white_check_mark: Complete | Ollama integration, pull/load/unload |
| **Agent Delegation** | :white_check_mark: Complete | Job submission, cross-session memory |
| **Setup Wizards** | :white_check_mark: Complete | Interactive setup for Nika, NovaNet |

### Crate Architecture

```
spn-core (0.2.0)      → Zero-dep types, 15 providers
spn-keyring (0.1.5)   → OS keychain, mlock, Zeroizing<T>
spn-client (0.3.4)    → Unix socket IPC, daemon protocol
spn-mcp (0.1.5)       → REST-to-MCP wrapper
spn-cli (0.16.0)      → Main CLI binary
spn-providers (0.1.0) → 6 cloud backends (Anthropic, OpenAI, Mistral, Groq, DeepSeek, Gemini)
spn-native (0.1.0)    → HuggingFace downloads + native inference (mistral.rs)
────────────────────────────────────────────────────
Total: 7 crates, ~18,000 LOC, 1,500+ tests
```

---

## Version Roadmap

### v0.16.0 - Phase B: Local Inference Engine

**Target:** April 2026
**Theme:** Pure-Rust local LLM inference without Ollama dependency

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  NEW CRATE: spn-inference (~4,000 LOC)                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌──────────────────┐         ┌──────────────────┐                              │
│  │      Candle      │ ◄────── │    mistral.rs    │                              │
│  │  (ML Framework)  │         │ (LLM Inference)  │                              │
│  ├──────────────────┤         ├──────────────────┤                              │
│  │ • Tensor ops     │         │ • Fast inference │                              │
│  │ • GPU/CPU        │         │ • Vision models  │                              │
│  │ • Transformers   │         │ • Quantization   │                              │
│  └──────────────────┘         └──────────────────┘                              │
│           │                            │                                        │
│           └──────────┬─────────────────┘                                        │
│                      ▼                                                          │
│           ┌──────────────────┐                                                  │
│           │  InferenceBackend│                                                  │
│           │      Trait       │                                                  │
│           ├──────────────────┤                                                  │
│           │ • OllamaBackend  │ (existing)                                       │
│           │ • CandleBackend  │ (new)                                            │
│           │ • MistralBackend │ (new)                                            │
│           └──────────────────┘                                                  │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

| Feature | Description | Priority |
|---------|-------------|----------|
| **Candle Integration** | HuggingFace ML framework for Rust | P0 |
| **mistral.rs Backend** | Fast local inference with vision support | P0 |
| **Model Format Support** | GGUF, safetensors, GGML | P0 |
| **GPU Acceleration** | CUDA, Metal, CPU fallback | P1 |
| **Progress Streaming** | indicatif-based download/inference progress | P1 |
| **Model Caching** | Smart cache with LRU eviction | P2 |

#### New Dependencies

```toml
[dependencies]
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
mistralrs = "0.4"
mistralrs-core = "0.4"
half = "2.4"              # f16 support
safetensors = "0.4"       # Model format
indicatif = "0.17"        # Progress bars
```

#### CLI Commands

```bash
spn inference run <model> --prompt "..."    # Local inference
spn inference load <model> --backend candle # Load with specific backend
spn inference benchmark <model>             # Performance benchmark
spn inference convert <model> --format gguf # Model conversion
```

#### Success Metrics

- [ ] Local inference without Ollama dependency
- [ ] < 100ms time-to-first-token for 7B models
- [ ] GPU utilization > 80% on supported hardware
- [ ] Support for top 10 HuggingFace models

#### Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| GPU driver compatibility | High | CPU fallback, clear hardware requirements |
| Model format fragmentation | Medium | Focus on GGUF/safetensors first |
| Memory pressure on large models | Medium | Streaming, quantization options |

---

### v0.17.0 - Phase C: Hardware Intelligence

**Target:** May 2026
**Theme:** Smart hardware detection and model recommendations

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  NEW CRATE: spn-hw (~2,500 LOC)                                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Hardware Detection Layer:                                                      │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐                │
│  │    GPU     │  │    CPU     │  │    RAM     │  │   Disk     │                │
│  ├────────────┤  ├────────────┤  ├────────────┤  ├────────────┤                │
│  │ CUDA cores │  │ Cores/SMT  │  │ Total/Free │  │ SSD speed  │                │
│  │ VRAM size  │  │ AVX/AVX2   │  │ Bandwidth  │  │ Available  │                │
│  │ Compute    │  │ ARM NEON   │  │ Channels   │  │ Cache dir  │                │
│  └────────────┘  └────────────┘  └────────────┘  └────────────┘                │
│         │              │              │              │                          │
│         └──────────────┴──────────────┴──────────────┘                          │
│                                 │                                               │
│                                 ▼                                               │
│                    ┌────────────────────────┐                                   │
│                    │   Recommendation       │                                   │
│                    │       Engine           │                                   │
│                    ├────────────────────────┤                                   │
│                    │ • Best model for HW    │                                   │
│                    │ • Quantization level   │                                   │
│                    │ • Context window size  │                                   │
│                    │ • Batch size tuning    │                                   │
│                    └────────────────────────┘                                   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

| Feature | Description | Priority |
|---------|-------------|----------|
| **GPU Detection** | CUDA, Metal, ROCm capability detection | P0 |
| **CPU Profiling** | AVX/AVX2/AVX-512, ARM NEON detection | P0 |
| **Memory Analysis** | RAM/VRAM available, bandwidth estimation | P0 |
| **Model Recommendations** | Suggest optimal models for hardware | P0 |
| **Quantization Advisor** | Recommend Q4/Q5/Q8/F16 based on VRAM | P1 |
| **Benchmark Suite** | Built-in performance testing | P1 |

#### New Dependencies

```toml
[dependencies]
sysinfo = "0.32"           # System information
nvml-wrapper = "0.10"      # NVIDIA GPU info
metal = "0.29"             # Apple GPU (optional)
llmfit-core = "0.1"        # Hardware recommendations (our crate)
```

#### CLI Commands

```bash
spn hw detect                    # Show hardware capabilities
spn hw recommend                 # Recommend models for this machine
spn hw benchmark [model]         # Run inference benchmark
spn hw profile --duration 60s    # Profile resource usage
```

#### Success Metrics

- [ ] Accurate GPU/CPU/RAM detection on macOS, Linux, Windows
- [ ] Model recommendations within 10% of optimal performance
- [ ] Hardware report generation in < 5 seconds
- [ ] Support for NVIDIA, AMD, Apple Silicon, Intel GPUs

---

### v0.18.0 - Phase D: Agent Swarms

**Target:** July 2026
**Theme:** Multi-agent orchestration and collaboration

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  NEW CRATE: spn-swarm (~3,800 LOC)                                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                         Message Bus (tokio broadcast)                    │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │   │
│  │  │ Agent A │──│ Agent B │──│ Agent C │──│ Agent D │──│ Agent E │       │   │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘       │   │
│  │       │            │            │            │            │             │   │
│  │       └────────────┴────────────┴────────────┴────────────┘             │   │
│  │                              │                                          │   │
│  └──────────────────────────────┼──────────────────────────────────────────┘   │
│                                 │                                               │
│                                 ▼                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                     Orchestration Layer                                  │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                   │   │
│  │  │  DAG Engine  │  │   Consensus  │  │   Artifact   │                   │   │
│  │  │  (petgraph)  │  │   Protocol   │  │    Store     │                   │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘                   │   │
│  │        │                  │                  │                          │   │
│  │        └──────────────────┴──────────────────┘                          │   │
│  │                          │                                              │   │
│  └──────────────────────────┼──────────────────────────────────────────────┘   │
│                             ▼                                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                      Swarm TUI (ratatui)                                 │   │
│  │  ┌────────────────────────────────────────────────────────────────┐     │   │
│  │  │  Agent Status    │  Message Flow    │  DAG Progress            │     │   │
│  │  │  ├── A: thinking │  A→B: "analyze"  │  [████████░░] 80%        │     │   │
│  │  │  ├── B: waiting  │  B→C: "result"   │  Step 4/5: synthesis     │     │   │
│  │  │  └── C: working  │  C→A: "feedback" │  ETA: 2m 34s             │     │   │
│  │  └────────────────────────────────────────────────────────────────┘     │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

| Feature | Description | Priority |
|---------|-------------|----------|
| **Agent Registry** | Define agent capabilities and roles | P0 |
| **Message Bus** | Async inter-agent communication | P0 |
| **DAG Execution** | Dependency-aware task scheduling | P0 |
| **Consensus Protocol** | Multi-agent decision making | P1 |
| **Artifact Store** | Shared state and intermediate results | P1 |
| **Swarm TUI** | Real-time visualization of swarm activity | P1 |
| **Swarm Templates** | Pre-built patterns (review, research, code) | P2 |

#### New Dependencies

```toml
[dependencies]
petgraph = "0.7"           # DAG execution
tokio = { features = ["sync", "broadcast"] }
dashmap = "6.0"            # Concurrent artifact store
uuid = "1.8"               # Agent/message IDs
ratatui = "0.29"           # TUI framework
```

#### CLI Commands

```bash
spn swarm create <name>          # Create new swarm
spn swarm run <swarm.yaml>       # Execute swarm workflow
spn swarm status                 # Show running swarms
spn swarm tui                    # Launch swarm monitor
spn swarm template list          # List available templates
spn swarm template use research  # Use research swarm template
```

#### Swarm YAML Format

```yaml
swarm: code-review
agents:
  - name: security-reviewer
    role: "Security analysis expert"
    capabilities: ["security", "vulnerability-detection"]
  - name: performance-reviewer
    role: "Performance optimization expert"
    capabilities: ["performance", "complexity-analysis"]
  - name: synthesizer
    role: "Consolidates feedback"
    capabilities: ["synthesis", "prioritization"]

dag:
  - step: parallel-review
    agents: [security-reviewer, performance-reviewer]
    input: $code
  - step: synthesis
    agent: synthesizer
    depends: [parallel-review]
    output: final-review

consensus:
  strategy: weighted-vote
  threshold: 0.7
```

#### Success Metrics

- [ ] Support for 10+ concurrent agents
- [ ] Message latency < 10ms within swarm
- [ ] DAG execution with automatic retry on failure
- [ ] TUI updates at 60fps with 100+ messages/second

---

### v0.19.0 - Phase E: Fine-Tuning Studio

**Target:** September 2026
**Theme:** Local model customization with LoRA/QLoRA

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  NEW CRATE: spn-train (~8,500 LOC)                                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Training Pipeline:                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
│  │   Dataset   │───▶│  Tokenizer  │───▶│   Trainer   │───▶│   Export    │      │
│  │   Loader    │    │   Manager   │    │   (Candle)  │    │   (GGUF)    │      │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘      │
│        │                  │                  │                  │               │
│        └──────────────────┴──────────────────┴──────────────────┘               │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                    Training TUI Dashboard                                │   │
│  │  ┌────────────────────────────────────────────────────────────────┐     │   │
│  │  │  Training Progress                                             │     │   │
│  │  │  ──────────────────────────────────────────────────────────────│     │   │
│  │  │  Epoch: 3/10    Loss: 0.342    LR: 1e-4    GPU: 78%           │     │   │
│  │  │  [████████████████████░░░░░░░░░░░░░░░░░░░░░] 52%              │     │   │
│  │  │                                                                │     │   │
│  │  │  Loss Curve:                  GPU Memory:                      │     │   │
│  │  │  ▲                            ┌────────────────┐               │     │   │
│  │  │  │  ╲                         │████████████░░░ │ 12.4/16 GB    │     │   │
│  │  │  │   ╲_____                   └────────────────┘               │     │   │
│  │  │  └────────────▶                                                │     │   │
│  │  │     Epoch                     ETA: 2h 34m                      │     │   │
│  │  └────────────────────────────────────────────────────────────────┘     │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  Techniques Supported:                                                          │
│  ├── LoRA (Low-Rank Adaptation)                                                │
│  ├── QLoRA (Quantized LoRA)                                                    │
│  ├── Full fine-tuning (small models)                                           │
│  └── Adapter merging                                                           │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

| Feature | Description | Priority |
|---------|-------------|----------|
| **Dataset Management** | Load, validate, tokenize datasets | P0 |
| **LoRA Training** | Low-rank adaptation for efficient fine-tuning | P0 |
| **QLoRA Support** | 4-bit quantized training for consumer GPUs | P0 |
| **Training TUI** | Real-time loss curves, GPU stats, ETA | P0 |
| **Checkpoint Management** | Save/resume, best checkpoint selection | P1 |
| **GGUF Export** | Convert adapters to deployable format | P1 |
| **Hyperparameter Search** | Basic grid search for learning rate, etc. | P2 |
| **Evaluation Suite** | Perplexity, benchmark evaluation | P2 |

#### New Dependencies

```toml
[dependencies]
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
tokenizers = "0.21"        # HuggingFace tokenizers
hf-hub = "0.3"             # HuggingFace model hub
parquet = "55"             # Dataset format
arrow = "55"               # Data processing
ratatui = "0.29"           # TUI framework
```

#### CLI Commands

```bash
spn train init <project>         # Initialize training project
spn train dataset add <path>     # Add dataset to project
spn train config                 # Configure hyperparameters
spn train start                  # Start training (TUI mode)
spn train start --headless       # Background training
spn train status                 # Show training progress
spn train export --format gguf   # Export trained adapter
spn train eval <model>           # Evaluate model performance
```

#### Training Config Format

```yaml
training:
  base_model: "meta-llama/Llama-3.2-7B"
  technique: lora

  lora:
    rank: 16
    alpha: 32
    dropout: 0.05
    target_modules: ["q_proj", "v_proj"]

  hyperparameters:
    learning_rate: 2e-4
    batch_size: 4
    gradient_accumulation: 8
    epochs: 3
    warmup_ratio: 0.03
    weight_decay: 0.01

  dataset:
    path: "./data/training.parquet"
    text_column: "text"
    max_length: 2048

  checkpointing:
    save_steps: 500
    save_total_limit: 3

  output:
    dir: "./output"
    format: gguf
```

#### Success Metrics

- [ ] Train 7B model LoRA on 16GB VRAM consumer GPU
- [ ] QLoRA training on 8GB VRAM
- [ ] Training throughput > 1000 tokens/second on RTX 4090
- [ ] Export to GGUF compatible with Ollama/llama.cpp

---

### v1.0.0 - Phase F: Deployment Engine

**Target:** November 2026
**Theme:** Production-ready model serving, edge deployment, Kubernetes

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  NEW CRATE: spn-deploy (~8,500 LOC)                                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Deployment Targets:                                                            │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐                    │
│  │   Local Serve  │  │      Edge      │  │   Kubernetes   │                    │
│  │    (Axum)      │  │     (WASM)     │  │    (k8s API)   │                    │
│  ├────────────────┤  ├────────────────┤  ├────────────────┤                    │
│  │ • REST API     │  │ • Browser      │  │ • Helm charts  │                    │
│  │ • Streaming    │  │ • Cloudflare   │  │ • Autoscaling  │                    │
│  │ • OpenAI compat│  │ • Deno Deploy  │  │ • GPU nodes    │                    │
│  │ • Rate limiting│  │ • Edge runtime │  │ • Monitoring   │                    │
│  └────────────────┘  └────────────────┘  └────────────────┘                    │
│         │                   │                   │                               │
│         └───────────────────┴───────────────────┘                               │
│                             │                                                   │
│                             ▼                                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                    Unified Deployment Pipeline                           │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                   │   │
│  │  │   Package    │  │   Deploy     │  │   Monitor    │                   │   │
│  │  │   Model      │  │   Target     │  │   Health     │                   │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘                   │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  Container Distribution:                                                        │
│  ├── ghcr.io/supernovae-st/spn-serve:latest                                    │
│  ├── Dockerfile.serve (optimized for inference)                                │
│  └── Helm chart: supernovae-st/spn-serve                                       │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Key Features

| Feature | Description | Priority |
|---------|-------------|----------|
| **Local Serve** | REST API with OpenAI-compatible endpoints | P0 |
| **Streaming Response** | SSE for real-time token streaming | P0 |
| **WASM Export** | Edge-ready model packaging | P0 |
| **Kubernetes Deployment** | Helm charts, autoscaling | P0 |
| **Health Monitoring** | Prometheus metrics, health checks | P1 |
| **Rate Limiting** | Token bucket, concurrent request limits | P1 |
| **Model Versioning** | Blue/green deployment, rollback | P2 |
| **A/B Testing** | Traffic splitting between model versions | P2 |

#### New Dependencies

```toml
[dependencies]
axum = "0.8"               # Web framework
tower = "0.5"              # Middleware
tower-http = "0.6"         # HTTP middleware
wasm-bindgen = "0.2"       # WASM bindings
kube = "0.98"              # Kubernetes API
k8s-openapi = "0.24"       # K8s types
prometheus = "0.13"        # Metrics
tracing = "0.1"            # Structured logging
tracing-subscriber = "0.3" # Log formatting
```

#### CLI Commands

```bash
# Local serving
spn serve <model>                     # Start local server
spn serve <model> --port 8080         # Custom port
spn serve <model> --openai-compat     # OpenAI API format

# Edge deployment
spn deploy edge build <model>         # Build WASM package
spn deploy edge publish --cloudflare  # Deploy to Cloudflare Workers
spn deploy edge publish --deno        # Deploy to Deno Deploy

# Kubernetes
spn deploy k8s init                   # Generate Helm chart
spn deploy k8s apply                  # Apply to cluster
spn deploy k8s status                 # Show deployment status
spn deploy k8s scale --replicas 3     # Scale deployment
spn deploy k8s rollback               # Rollback to previous version
```

#### Success Metrics

- [ ] < 50ms latency for local serve (p99)
- [ ] OpenAI API compatibility for drop-in replacement
- [ ] WASM package < 50MB for edge deployment
- [ ] Kubernetes deployment with 0-downtime rolling updates
- [ ] Autoscaling from 1 to 10 replicas within 60 seconds

---

## Summary Timeline

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  2026 ROADMAP                                                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Q1 2026 (Jan-Mar)                                                              │
│  └── v0.15.x ✅ COMPLETE                                                        │
│      • Agent delegation, job scheduler, MCP auto-sync                          │
│      • Cross-session memory, autonomy orchestration                            │
│                                                                                 │
│  Q2 2026 (Apr-Jun)                                                              │
│  ├── v0.16.0 (Apr) - Phase B: Local Inference                                  │
│  │   • spn-inference crate, Candle + mistral.rs                                │
│  │   • GPU acceleration, model caching                                         │
│  │                                                                              │
│  └── v0.17.0 (May) - Phase C: Hardware Intelligence                            │
│      • spn-hw crate, hardware detection                                        │
│      • Model recommendations, benchmark suite                                  │
│                                                                                 │
│  Q3 2026 (Jul-Sep)                                                              │
│  ├── v0.18.0 (Jul) - Phase D: Agent Swarms                                     │
│  │   • spn-swarm crate, message bus, DAG execution                             │
│  │   • Swarm TUI, consensus protocols                                          │
│  │                                                                              │
│  └── v0.19.0 (Sep) - Phase E: Fine-Tuning Studio                               │
│      • spn-train crate, LoRA/QLoRA training                                    │
│      • Training TUI, GGUF export                                               │
│                                                                                 │
│  Q4 2026 (Oct-Nov)                                                              │
│  └── v1.0.0 (Nov) - Phase F: Deployment Engine                                 │
│      • spn-deploy crate, Axum serve, WASM edge                                 │
│      • Kubernetes deployment, production-ready                                 │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| GPU compatibility issues | Medium | High | Extensive testing, CPU fallback |
| Candle/mistral.rs API instability | Medium | High | Pin versions, abstraction layer |
| WASM bundle size too large | Medium | Medium | Model quantization, lazy loading |
| K8s complexity | Low | Medium | Helm abstractions, sensible defaults |

### Resource Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Scope creep in Phase E | High | High | Strict feature prioritization |
| Insufficient GPU testing hardware | Medium | High | Cloud GPU credits, CI integration |
| Community adoption | Medium | Medium | Clear documentation, examples |

### Dependencies

| External Dependency | Risk Level | Alternative |
|---------------------|------------|-------------|
| Candle (HuggingFace) | Low | Well-maintained, active community |
| mistral.rs | Medium | Fork if abandoned, contribute upstream |
| Ollama | Low | Mature, optional dependency |
| Kubernetes APIs | Low | Stable, versioned APIs |

---

## Crate Architecture at v1.0.0

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  FINAL ARCHITECTURE: 10 CRATES                                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  LAYER 0: Core Types (Zero Dependencies)                                │   │
│  │  ├── spn-core (0.2.0)        Providers, validation, types              │   │
│  │  └── spn-hw (0.1.0)          Hardware detection, recommendations       │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  LAYER 1: Security & IPC                                                │   │
│  │  └── spn-keyring (0.2.0)     OS keychain, memory protection            │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  LAYER 2: Inference Backends                                            │   │
│  │  ├── spn-providers (0.1.0)   Cloud backends (Anthropic, OpenAI, etc.)  │   │
│  │  └── spn-native (0.1.0)      HuggingFace + mistral.rs inference        │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  LAYER 3: Training & Orchestration                                      │   │
│  │  ├── spn-train (0.1.0)       LoRA/QLoRA fine-tuning                    │   │
│  │  └── spn-swarm (0.1.0)       Multi-agent orchestration                 │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  LAYER 4: Deployment & Integration                                      │   │
│  │  ├── spn-deploy (0.1.0)      Serve, edge, Kubernetes                   │   │
│  │  ├── spn-mcp (0.2.0)         MCP server wrapper                        │   │
│  │  └── spn-client (0.4.0)      SDK for external tools                    │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  LAYER 5: CLI                                                           │   │
│  │  └── spn-cli (1.0.0)         Main CLI binary                           │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  Total: 10 crates, ~40,000 LOC, ~2,500 tests                                   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Success Metrics Summary

### v1.0.0 Definition of Done

| Category | Metric | Target |
|----------|--------|--------|
| **Inference** | Time-to-first-token (7B model) | < 100ms |
| **Inference** | Throughput (tokens/second) | > 50 tok/s on RTX 4090 |
| **Training** | LoRA on 16GB VRAM | 7B model |
| **Training** | QLoRA on 8GB VRAM | 7B model |
| **Deployment** | Local serve latency (p99) | < 50ms |
| **Deployment** | K8s scale-out time | < 60s |
| **Swarms** | Concurrent agents | 10+ |
| **Swarms** | Message latency | < 10ms |
| **Tests** | Coverage | > 80% |
| **Tests** | Total tests | > 2,500 |
| **Docs** | API coverage | 100% |

### KPIs

| Metric | v0.15.x | v1.0.0 Target |
|--------|---------|---------------|
| Total crates | 6 | 10 |
| Lines of code | ~15,000 | ~40,000 |
| Test count | 1,288 | 2,500+ |
| Supported models | Ollama only | Ollama + GGUF + safetensors |
| Deployment targets | Local | Local + Edge + K8s |

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development guidelines.

### Phase Ownership

| Phase | Lead | Status |
|-------|------|--------|
| Phase B (Inference) | TBD | Planning |
| Phase C (Hardware) | TBD | Planning |
| Phase D (Swarms) | TBD | Planning |
| Phase E (Training) | TBD | Planning |
| Phase F (Deploy) | TBD | Planning |

---

## Changelog

- **2026-03-10**: Initial roadmap creation
- See [CHANGELOG.md](./CHANGELOG.md) for detailed version history

---

*This roadmap is ambitious but achievable. We're building the ULTIMATE AI toolkit.*

**spn: Manage. Run. Train. Deploy.**
