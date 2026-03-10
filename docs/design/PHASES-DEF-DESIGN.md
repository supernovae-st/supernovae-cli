# spn-cli Evolution: Phases D, E, F Design Document

**Version:** 1.0.0
**Date:** 2026-03-10
**Author:** Architecture Team
**Status:** PROPOSAL

---

## Executive Summary

This document outlines the next evolution of spn-cli beyond the planned Phases A-C:

| Phase | Name | Objective | Timeline |
|-------|------|-----------|----------|
| **D** | Agent Swarms | Multi-agent orchestration with parallel workflows | Q3 2026 |
| **E** | Fine-tuning Studio | LoRA/QLoRA training pipelines | Q4 2026 |
| **F** | Deployment Engine | Model serving, edge deployment, containers | Q1 2027 |

**Foundation:** The existing codebase already provides substantial infrastructure:
- `daemon/agents/` - Agent types, roles, states, delegation
- `daemon/autonomy/` - Orchestration, policies, approval workflows
- `daemon/memory/` - Persistent context storage
- `daemon/traces/` - Reasoning trace capture
- `spn-providers` - 6 cloud LLM backends
- `spn-ollama` - Local model management

---

## Phase D: Agent Swarms

### Objective

Transform single-agent execution into coordinated multi-agent workflows. Enable developers to compose agent teams that parallelize work, communicate results, and solve complex tasks collaboratively.

**The "wow" moment:**
```bash
spn swarm run code-review src/ --agents 5
# Spawns 5 parallel agents, each reviews different files
# Results aggregated into unified report
```

### Key Features

1. **Declarative Swarm Composition**
   - YAML-based swarm definitions (aligned with Nika workflow syntax)
   - Role-based agent assignment
   - Dependency graphs for agent coordination

2. **Inter-Agent Communication**
   - Message bus for agent-to-agent communication
   - Shared blackboard for collaborative state
   - Event-driven coordination patterns

3. **Hierarchical Delegation**
   - Supervisor agents that decompose tasks
   - Worker agents that execute subtasks
   - Result aggregation and synthesis

4. **Work Distribution Strategies**
   - Round-robin for balanced load
   - Capability-based for specialized tasks
   - Priority queues for urgent work

5. **Real-time Observability**
   - TUI dashboard showing agent collaboration
   - Live message flow visualization
   - Progress tracking per agent

6. **Fault Tolerance**
   - Agent restart on failure
   - Task redistribution
   - Circuit breakers for cascading failures

7. **Cost Optimization**
   - Token budget management per swarm
   - Model tier selection (cheap for exploration, expensive for synthesis)
   - Automatic batching of similar requests

### Architecture

```
                           PHASE D: AGENT SWARMS
  ============================================================================

  ┌─────────────────────────────────────────────────────────────────────────┐
  │                         spn-swarm (new crate)                            │
  ├─────────────────────────────────────────────────────────────────────────┤
  │                                                                         │
  │  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐     │
  │  │  Swarm Parser   │    │  Swarm Planner  │    │ Swarm Executor  │     │
  │  │                 │    │                 │    │                 │     │
  │  │ • YAML loader   │───▶│ • DAG builder   │───▶│ • Agent pool    │     │
  │  │ • Schema valid  │    │ • Task decomp   │    │ • Work dispatch │     │
  │  │ • Type checking │    │ • Dependencies  │    │ • Result merge  │     │
  │  └─────────────────┘    └─────────────────┘    └─────────────────┘     │
  │                                                         │               │
  │                                                         ▼               │
  │  ┌──────────────────────────────────────────────────────────────────┐  │
  │  │                      Message Bus (tokio broadcast)                │  │
  │  │  ┌──────────────────────────────────────────────────────────┐    │  │
  │  │  │  Message Types:                                           │    │  │
  │  │  │  • TaskAssignment { agent_id, task, context }             │    │  │
  │  │  │  • TaskResult { agent_id, result, tokens_used }           │    │  │
  │  │  │  • AgentQuery { from, to, question }                      │    │  │
  │  │  │  • AgentResponse { from, to, answer }                     │    │  │
  │  │  │  • Broadcast { topic, payload }                           │    │  │
  │  │  └──────────────────────────────────────────────────────────┘    │  │
  │  └──────────────────────────────────────────────────────────────────┘  │
  │           │                     │                     │                 │
  │           ▼                     ▼                     ▼                 │
  │  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐       │
  │  │   Agent 1       │   │   Agent 2       │   │   Agent N       │       │
  │  │   (Explorer)    │   │   (Reviewer)    │   │   (Synth)       │       │
  │  │                 │   │                 │   │                 │       │
  │  │ ┌─────────────┐ │   │ ┌─────────────┐ │   │ ┌─────────────┐ │       │
  │  │ │ spn-providers│ │   │ │ spn-providers│ │   │ │ spn-providers│ │       │
  │  │ │   (Claude)   │ │   │ │   (GPT-4)   │ │   │ │   (Opus)    │ │       │
  │  │ └─────────────┘ │   │ └─────────────┘ │   │ └─────────────┘ │       │
  │  └─────────────────┘   └─────────────────┘   └─────────────────┘       │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘
                    │                                       │
                    ▼                                       ▼
         ┌─────────────────────┐              ┌─────────────────────┐
         │  daemon/traces/     │              │  daemon/memory/     │
         │  (existing)         │              │  (existing)         │
         │  • ReasoningTrace   │              │  • MemoryStore      │
         │  • TraceStep        │              │  • Namespaces       │
         └─────────────────────┘              └─────────────────────┘
```

### YAML Swarm Definition

```yaml
# swarms/code-review.swarm.yaml
swarm: code-review
version: 1
description: Parallel code review with synthesis

agents:
  - id: explorer
    role: explorer
    model: claude-3-haiku-20240307
    instances: 3  # 3 parallel explorers
    task: |
      Explore the codebase structure and identify files matching the pattern.
      Report file paths and their purposes.

  - id: reviewer
    role: reviewer
    model: claude-3-5-sonnet-20241022
    instances: auto  # Scale based on file count
    depends_on: [explorer]
    task: |
      Review each file for:
      - Code quality issues
      - Security vulnerabilities
      - Performance concerns

  - id: synthesizer
    role: general
    model: claude-3-opus
    instances: 1
    depends_on: [reviewer]
    aggregate: true  # Receives all reviewer results
    task: |
      Synthesize all reviews into a unified report.
      Prioritize issues by severity.

distribution:
  strategy: capability  # or: round-robin, priority, random

budget:
  max_tokens: 100000
  timeout: 5m

output:
  format: markdown
  file: review-report.md
```

### CLI Commands

```bash
# Run a predefined swarm
spn swarm run code-review src/

# Create a new swarm interactively
spn swarm create --interactive

# List available swarms
spn swarm list

# Monitor running swarm
spn swarm status <swarm-id>

# Cancel a swarm
spn swarm cancel <swarm-id>

# Swarm TUI dashboard
spn swarm watch <swarm-id>
```

### New Crate: spn-swarm

```toml
# crates/spn-swarm/Cargo.toml
[package]
name = "spn-swarm"
version = "0.1.0"
edition = "2021"
description = "Multi-agent swarm orchestration for SuperNovae"

[dependencies]
spn-core = { path = "../spn-core" }
spn-providers = { path = "../spn-providers", features = ["cloud"] }

# Async runtime
tokio = { version = "1.36", features = ["full"] }

# Message passing
async-broadcast = "0.7"  # Multi-producer multi-consumer

# DAG execution
petgraph = "0.6"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"

# Observability
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1.36", features = ["test-util"] }
```

### LOC Estimate

| Component | Lines |
|-----------|-------|
| Swarm parser (YAML) | ~500 |
| Message bus | ~300 |
| Agent pool manager | ~600 |
| Work distributor | ~400 |
| Result aggregator | ~300 |
| CLI integration | ~400 |
| TUI dashboard | ~500 |
| Tests | ~800 |
| **Total** | **~3,800** |

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Message ordering issues | Medium | High | Use sequence numbers, idempotent handlers |
| Token budget overruns | High | Medium | Hard limits with graceful degradation |
| Agent coordination deadlocks | Low | High | Timeout-based circuit breakers |
| LLM rate limits | Medium | Medium | Request queuing with backpressure |
| Complex debugging | High | Medium | Rich trace capture, replay capability |

---

## Phase E: Fine-tuning Studio

### Objective

Democratize model fine-tuning with a zero-config, TUI-driven experience. Enable developers to customize models for their specific domains without deep ML expertise.

**The "wow" moment:**
```bash
spn train create my-code-model --base llama3.2:8b
spn train add-data ./src/ --type code
spn train start --method qlora
# TUI shows live training progress, loss curves, checkpoints
```

### Key Features

1. **One-Command Training**
   - Automatic hyperparameter selection
   - Dataset preprocessing built-in
   - GPU/MPS detection and optimization

2. **Dataset Management**
   - Import from local files (code, docs, conversations)
   - Format auto-detection (JSONL, Parquet, text)
   - Data augmentation and cleaning

3. **Training Methods**
   - LoRA (Low-Rank Adaptation)
   - QLoRA (Quantized LoRA)
   - Full fine-tuning (for small models)
   - DPO (Direct Preference Optimization)

4. **Live Training Dashboard**
   - Loss curves in TUI
   - Checkpoint management
   - Early stopping controls
   - Resource utilization

5. **Model Merging**
   - Merge LoRA adapters into base
   - Multi-adapter stacking
   - Export to GGUF/safetensors

6. **Evaluation Suite**
   - Automated eval on holdout set
   - Custom eval prompts
   - Comparison with base model

7. **Cloud Training (Optional)**
   - Dispatch to cloud GPUs
   - Resume from checkpoints
   - Artifact management

### Architecture

```
                         PHASE E: FINE-TUNING STUDIO
  ============================================================================

  ┌─────────────────────────────────────────────────────────────────────────┐
  │                          spn-train (new crate)                           │
  ├─────────────────────────────────────────────────────────────────────────┤
  │                                                                         │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │                    Dataset Manager                               │   │
  │  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐ │   │
  │  │  │  Importers │  │ Processors │  │  Splitter  │  │  Exporter  │ │   │
  │  │  │            │  │            │  │            │  │            │ │   │
  │  │  │ • Code     │  │ • Tokenize │  │ • Train    │  │ • JSONL    │ │   │
  │  │  │ • JSONL    │  │ • Clean    │  │ • Valid    │  │ • Parquet  │ │   │
  │  │  │ • Parquet  │  │ • Augment  │  │ • Test     │  │ • HF Push  │ │   │
  │  │  │ • HF Hub   │  │ • Format   │  │            │  │            │ │   │
  │  │  └────────────┘  └────────────┘  └────────────┘  └────────────┘ │   │
  │  └─────────────────────────────────────────────────────────────────┘   │
  │                                   │                                     │
  │                                   ▼                                     │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │                    Training Pipeline                             │   │
  │  │                                                                  │   │
  │  │   ┌──────────────────────────────────────────────────────────┐  │   │
  │  │   │                 candle-transformers                       │  │   │
  │  │   │                                                           │  │   │
  │  │   │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐     │  │   │
  │  │   │  │  Base Model │   │ LoRA Layer  │   │   Trainer   │     │  │   │
  │  │   │  │  Loading    │──▶│  Injection  │──▶│   Loop      │     │  │   │
  │  │   │  │             │   │             │   │             │     │  │   │
  │  │   │  │ • GGUF      │   │ • Rank cfg  │   │ • Grad acc  │     │  │   │
  │  │   │  │ • SafeT     │   │ • Alpha     │   │ • Scheduler │     │  │   │
  │  │   │  │ • HF Hub    │   │ • Targets   │   │ • Checkpt   │     │  │   │
  │  │   │  └─────────────┘   └─────────────┘   └─────────────┘     │  │   │
  │  │   │                                              │            │  │   │
  │  │   └──────────────────────────────────────────────┼────────────┘  │   │
  │  │                                                  ▼               │   │
  │  │   ┌──────────────────────────────────────────────────────────┐  │   │
  │  │   │                Progress Channel (tokio mpsc)              │  │   │
  │  │   │  • TrainingStep { epoch, batch, loss, lr, tokens/s }      │  │   │
  │  │   │  • Checkpoint { path, metrics }                           │  │   │
  │  │   │  • EvalResult { perplexity, accuracy }                    │  │   │
  │  │   └──────────────────────────────────────────────────────────┘  │   │
  │  └─────────────────────────────────────────────────────────────────┘   │
  │                                   │                                     │
  │                                   ▼                                     │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │                    Training TUI                                  │   │
  │  │  ┌─────────────────────────────────────────────────────────────┐│   │
  │  │  │  Loss: 2.341 ▼  │ LR: 1e-4  │ Epoch: 2/5  │ ETA: 12m       ││   │
  │  │  ├─────────────────────────────────────────────────────────────┤│   │
  │  │  │                                                             ││   │
  │  │  │  Loss Curve                  │  GPU Utilization             ││   │
  │  │  │  ▁▂▃▄▅▅▄▃▃▂▂▂▂▁▁▁▁▁▁▁▁▁    │  ████████████░░░ 78%         ││   │
  │  │  │                              │  VRAM: 18.2/24 GB            ││   │
  │  │  │                              │                               ││   │
  │  │  ├─────────────────────────────────────────────────────────────┤│   │
  │  │  │  Checkpoints:                                               ││   │
  │  │  │  [x] epoch-1-loss-3.21  [x] epoch-2-loss-2.34  [ ] ...     ││   │
  │  │  ├─────────────────────────────────────────────────────────────┤│   │
  │  │  │  [P]ause  [S]top  [C]heckpoint  [E]val  [Q]uit              ││   │
  │  │  └─────────────────────────────────────────────────────────────┘│   │
  │  └─────────────────────────────────────────────────────────────────┘   │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
  ┌─────────────────────────────────────────────────────────────────────────┐
  │                          Artifact Storage                                │
  │                                                                         │
  │  ~/.spn/models/                                                         │
  │  └── my-code-model/                                                     │
  │      ├── config.yaml         # Training config                          │
  │      ├── dataset/            # Processed dataset                        │
  │      ├── checkpoints/        # Training checkpoints                     │
  │      │   ├── epoch-1/                                                   │
  │      │   └── epoch-2/                                                   │
  │      ├── adapter.safetensors # Final LoRA adapter                       │
  │      └── merged.gguf         # Merged model (optional)                  │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘
```

### CLI Commands

```bash
# Create training project
spn train create <name> --base <model>

# Add training data
spn train add-data <path> --type <code|chat|instruct>
spn train add-data https://huggingface.co/datasets/...

# Configure training
spn train config set epochs 3
spn train config set rank 64  # LoRA rank
spn train config set learning_rate 1e-4

# Start training
spn train start [--method lora|qlora|full]

# Monitor training (TUI)
spn train watch <name>

# Evaluate model
spn train eval <name> --prompts ./eval.jsonl

# Merge adapter into base
spn train merge <name> --output my-model.gguf

# Push to registry
spn train push <name> --tag v1.0
```

### Training Configuration

```yaml
# ~/.spn/models/my-code-model/config.yaml
name: my-code-model
base_model: llama3.2:8b
method: qlora

lora:
  rank: 64
  alpha: 128
  dropout: 0.05
  target_modules:
    - q_proj
    - v_proj
    - k_proj
    - o_proj

training:
  epochs: 3
  batch_size: 4
  gradient_accumulation_steps: 8
  learning_rate: 2e-4
  lr_scheduler: cosine
  warmup_ratio: 0.03
  weight_decay: 0.01
  max_grad_norm: 1.0

quantization:
  bits: 4
  double_quant: true
  quant_type: nf4

dataset:
  path: ./dataset
  train_split: 0.9
  format: instruction  # or: chat, completion
```

### New Crate: spn-train

```toml
# crates/spn-train/Cargo.toml
[package]
name = "spn-train"
version = "0.1.0"
edition = "2021"
description = "Model fine-tuning pipeline for SuperNovae"

[features]
default = ["cuda"]
cuda = ["candle-core/cuda"]
metal = ["candle-core/metal"]

[dependencies]
spn-core = { path = "../spn-core" }
spn-ollama = { path = "../spn-ollama" }

# ML framework
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
safetensors = "0.4"
tokenizers = "0.20"

# Data processing
arrow = "53"
parquet = { version = "53", features = ["async"] }

# Async runtime
tokio = { version = "1.36", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1.0"

# TUI
ratatui = "0.30"
crossterm = "0.28"

# Progress
indicatif = "0.18"

# Logging
tracing = "0.1"

[dev-dependencies]
tempfile = "3.10"
```

### LOC Estimate

| Component | Lines |
|-----------|-------|
| Dataset manager | ~1,200 |
| Training pipeline | ~2,000 |
| LoRA implementation | ~800 |
| Checkpoint management | ~500 |
| Evaluation suite | ~600 |
| Model merging | ~400 |
| CLI integration | ~500 |
| TUI dashboard | ~1,000 |
| Tests | ~1,500 |
| **Total** | **~8,500** |

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| GPU memory OOM | High | High | Gradient checkpointing, smaller batches |
| Training instability | Medium | High | Conservative defaults, monitoring |
| Candle compatibility | Medium | Medium | Pin versions, test on releases |
| Long training times | High | Low | Checkpointing, resume capability |
| Model quality | Medium | High | Automated eval, comparison metrics |
| Hardware fragmentation | High | Medium | Graceful CPU fallback, clear reqs |

---

## Phase F: Deployment Engine

### Objective

Enable one-command deployment of models to any target: local servers, edge devices, containers, or cloud platforms. Abstract away the complexity of model optimization, serving infrastructure, and scaling.

**The "wow" moment:**
```bash
spn deploy serve my-model --port 8080
# Instant OpenAI-compatible API server

spn deploy edge my-model --target wasm
# Compile to WebAssembly for browser/edge

spn deploy k8s my-model --replicas 3
# Generate Kubernetes manifests and deploy
```

### Key Features

1. **Local Model Server**
   - OpenAI-compatible REST API
   - Streaming responses
   - Concurrent request handling
   - Automatic batching

2. **Edge Deployment**
   - WebAssembly compilation
   - ONNX export
   - Mobile SDK generation (iOS/Android)
   - Raspberry Pi / embedded

3. **Container Deployment**
   - Auto-generated Dockerfile
   - Multi-stage builds
   - GPU passthrough
   - Health checks

4. **Kubernetes Integration**
   - Helm chart generation
   - HPA (auto-scaling)
   - Service mesh compatible
   - Observability (metrics, traces)

5. **Model Optimization**
   - Quantization (INT8, INT4)
   - Pruning
   - Knowledge distillation
   - GGUF conversion

6. **Monitoring Dashboard**
   - Request latency
   - Token throughput
   - GPU utilization
   - Cost estimation

7. **A/B Testing**
   - Traffic splitting
   - Gradual rollouts
   - Automatic rollback

### Architecture

```
                         PHASE F: DEPLOYMENT ENGINE
  ============================================================================

  ┌─────────────────────────────────────────────────────────────────────────┐
  │                         spn-deploy (new crate)                           │
  ├─────────────────────────────────────────────────────────────────────────┤
  │                                                                         │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │                    Model Optimizer                               │   │
  │  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐ │   │
  │  │  │ Quantizer  │  │  Pruner    │  │ Converter  │  │  Profiler  │ │   │
  │  │  │            │  │            │  │            │  │            │ │   │
  │  │  │ • INT8     │  │ • Magnitude│  │ • GGUF     │  │ • Latency  │ │   │
  │  │  │ • INT4     │  │ • Movement │  │ • ONNX     │  │ • Memory   │ │   │
  │  │  │ • FP16     │  │ • Struct   │  │ • CoreML   │  │ • Throughp │ │   │
  │  │  │ • AWQ      │  │            │  │ • TFLite   │  │            │ │   │
  │  │  └────────────┘  └────────────┘  └────────────┘  └────────────┘ │   │
  │  └─────────────────────────────────────────────────────────────────┘   │
  │                                   │                                     │
  │                                   ▼                                     │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │                    Deployment Targets                            │   │
  │  │                                                                  │   │
  │  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐ │   │
  │  │  │   Local    │  │    Edge    │  │  Container │  │    K8s     │ │   │
  │  │  │   Server   │  │            │  │            │  │            │ │   │
  │  │  │            │  │            │  │            │  │            │ │   │
  │  │  │ ┌────────┐ │  │ ┌────────┐ │  │ ┌────────┐ │  │ ┌────────┐ │ │   │
  │  │  │ │  Axum  │ │  │ │  WASM  │ │  │ │ Docker │ │  │ │  Helm  │ │ │   │
  │  │  │ │ Server │ │  │ │ Module │ │  │ │ Image  │ │  │ │ Chart  │ │ │   │
  │  │  │ └────────┘ │  │ └────────┘ │  │ └────────┘ │  │ └────────┘ │ │   │
  │  │  │            │  │            │  │            │  │            │ │   │
  │  │  │ • OpenAI   │  │ • Browser  │  │ • Multi-   │  │ • HPA      │ │   │
  │  │  │   compat   │  │ • Node.js  │  │   stage    │  │ • PDB      │ │   │
  │  │  │ • Stream   │  │ • Deno     │  │ • GPU      │  │ • Service  │ │   │
  │  │  │ • Batch    │  │ • Mobile   │  │ • Health   │  │ • Ingress  │ │   │
  │  │  └────────────┘  └────────────┘  └────────────┘  └────────────┘ │   │
  │  └─────────────────────────────────────────────────────────────────┘   │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘

                              LOCAL SERVER DETAIL
  ┌─────────────────────────────────────────────────────────────────────────┐
  │                                                                         │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │                     Axum HTTP Server                             │   │
  │  │                                                                  │   │
  │  │   POST /v1/chat/completions                                      │   │
  │  │   POST /v1/completions                                           │   │
  │  │   POST /v1/embeddings                                            │   │
  │  │   GET  /v1/models                                                │   │
  │  │   GET  /health                                                   │   │
  │  │   GET  /metrics (Prometheus)                                     │   │
  │  │                                                                  │   │
  │  └───────────────────────────┬─────────────────────────────────────┘   │
  │                              │                                          │
  │                              ▼                                          │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │                   Request Pipeline                               │   │
  │  │                                                                  │   │
  │  │   Request ──▶ Validate ──▶ Queue ──▶ Batch ──▶ Infer ──▶ Stream │   │
  │  │                              │                   │               │   │
  │  │                              ▼                   ▼               │   │
  │  │                        ┌─────────┐         ┌─────────┐           │   │
  │  │                        │ Backpres│         │ candle  │           │   │
  │  │                        │ sure    │         │ /llama  │           │   │
  │  │                        └─────────┘         │ .cpp    │           │   │
  │  │                                            └─────────┘           │   │
  │  └─────────────────────────────────────────────────────────────────┘   │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘
```

### CLI Commands

```bash
# === Local Server ===
spn deploy serve <model> [--port 8080] [--host 0.0.0.0]
spn deploy serve my-model --gpu 0 --max-batch 8

# === Edge Deployment ===
spn deploy edge <model> --target wasm --output ./dist/
spn deploy edge <model> --target onnx --optimize
spn deploy edge <model> --target coreml --ios

# === Container ===
spn deploy docker <model> --tag my-model:v1
spn deploy docker <model> --push ghcr.io/user/model

# === Kubernetes ===
spn deploy k8s <model> --replicas 3 --gpu nvidia
spn deploy k8s <model> --helm-chart ./chart/
spn deploy k8s <model> --apply  # Deploy to current context

# === Optimization ===
spn deploy optimize <model> --quantize int4
spn deploy optimize <model> --prune 0.3
spn deploy benchmark <model> --iterations 100

# === Monitoring ===
spn deploy status <deployment-id>
spn deploy logs <deployment-id>
spn deploy metrics <deployment-id>
```

### Deployment Configuration

```yaml
# deploy.yaml
name: my-model-api
model: my-model:latest

optimization:
  quantization: int4
  batch_optimization: true

server:
  port: 8080
  workers: 4
  max_concurrent: 100
  timeout_ms: 30000

  openai_compat: true
  endpoints:
    - /v1/chat/completions
    - /v1/embeddings

  cors:
    origins: ["*"]

  auth:
    type: api_key
    keys_from: env:API_KEYS

scaling:
  min_replicas: 1
  max_replicas: 10
  target_utilization: 0.7

monitoring:
  prometheus: true
  tracing: jaeger
  health_check:
    path: /health
    interval: 10s

resources:
  gpu: nvidia-a100
  memory: 24Gi
  cpu: 4
```

### Generated Dockerfile

```dockerfile
# Auto-generated by spn deploy docker
FROM nvidia/cuda:12.2-runtime-ubuntu22.04 AS runtime

# Install dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy model and server binary
COPY --from=builder /app/spn-serve /usr/local/bin/
COPY ./model /model

# Health check
HEALTHCHECK --interval=10s --timeout=3s \
    CMD curl -f http://localhost:8080/health || exit 1

# Run server
EXPOSE 8080
ENTRYPOINT ["spn-serve", "--model", "/model", "--port", "8080"]
```

### New Crate: spn-deploy

```toml
# crates/spn-deploy/Cargo.toml
[package]
name = "spn-deploy"
version = "0.1.0"
edition = "2021"
description = "Model deployment engine for SuperNovae"

[features]
default = ["server"]
server = ["axum", "tower", "tower-http"]
wasm = ["wasm-bindgen", "wasm-pack"]
docker = ["bollard"]
k8s = ["kube", "k8s-openapi"]

[dependencies]
spn-core = { path = "../spn-core" }
spn-ollama = { path = "../spn-ollama" }

# HTTP server
axum = { version = "0.7", features = ["ws"], optional = true }
tower = { version = "0.4", optional = true }
tower-http = { version = "0.5", features = ["cors", "trace"], optional = true }
hyper = { version = "1.0", features = ["full"] }

# ML inference
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"

# Async runtime
tokio = { version = "1.36", features = ["full"] }
futures = "0.3"

# Docker
bollard = { version = "0.16", optional = true }

# Kubernetes
kube = { version = "0.90", features = ["client", "runtime"], optional = true }
k8s-openapi = { version = "0.21", features = ["v1_29"], optional = true }

# WASM
wasm-bindgen = { version = "0.2", optional = true }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1.0"

# Metrics
prometheus = "0.13"

# Logging
tracing = "0.1"

[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
```

### LOC Estimate

| Component | Lines |
|-----------|-------|
| Model optimizer (quant/prune) | ~1,500 |
| HTTP server (Axum) | ~1,200 |
| Request pipeline | ~800 |
| Docker generation | ~600 |
| Kubernetes manifests | ~800 |
| WASM compilation | ~1,000 |
| Monitoring/metrics | ~500 |
| CLI integration | ~600 |
| Tests | ~1,500 |
| **Total** | **~8,500** |

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| WASM model size | High | Medium | Aggressive quantization, streaming |
| GPU compatibility | Medium | High | Clear requirements, fallback to CPU |
| K8s API changes | Low | Medium | Pin k8s-openapi version |
| Performance vs Ollama | Medium | Medium | Benchmark suite, optimization focus |
| Docker image size | High | Low | Multi-stage builds, distroless |
| Security (API keys) | Medium | High | Secret management, rate limiting |

---

## Cross-Phase Dependencies

```
                           DEPENDENCY GRAPH
  ============================================================================

                    spn-core (existing)
                           │
            ┌──────────────┼──────────────┐
            │              │              │
            ▼              ▼              ▼
     spn-providers    spn-ollama    spn-keyring
     (existing)       (existing)    (existing)
            │              │
            └──────┬───────┘
                   │
        ┌──────────┼──────────┐
        │          │          │
        ▼          ▼          ▼
    spn-swarm  spn-train  spn-deploy
    (Phase D)  (Phase E)   (Phase F)
        │          │          │
        └──────────┴──────────┘
                   │
                   ▼
            spn-cli (main)
```

### Shared Dependencies

| Dependency | Used By | Purpose |
|------------|---------|---------|
| `candle-*` | E, F | ML inference and training |
| `tokio` | D, E, F | Async runtime |
| `ratatui` | D, E, F | TUI components |
| `petgraph` | D | DAG execution |
| `axum` | F | HTTP server |

---

## Summary

### Total Effort Estimate

| Phase | New Crate | LOC | Duration | Team Size |
|-------|-----------|-----|----------|-----------|
| **D** | spn-swarm | ~3,800 | 6-8 weeks | 2 engineers |
| **E** | spn-train | ~8,500 | 10-12 weeks | 2-3 engineers |
| **F** | spn-deploy | ~8,500 | 10-12 weeks | 2-3 engineers |
| **Total** | 3 crates | ~20,800 | ~28-32 weeks | 2-3 engineers |

### Recommended Sequence

1. **Phase D (Agent Swarms)** - Builds on existing agent infrastructure
2. **Phase F (Deployment)** - High user demand, clear value
3. **Phase E (Fine-tuning)** - Most complex, benefits from F's infrastructure

### "Wow" Factor Summary

| Phase | The Moment |
|-------|------------|
| **D** | `spn swarm run code-review src/ --agents 5` - parallel AI team |
| **E** | `spn train start --method qlora` - one-command fine-tuning |
| **F** | `spn deploy edge my-model --target wasm` - browser-ready AI |

### Success Metrics

| Metric | Phase D | Phase E | Phase F |
|--------|---------|---------|---------|
| Setup time | < 1 min | < 5 min | < 2 min |
| Learning curve | 1 hour | 2 hours | 1 hour |
| Performance | 3x parallelism | Match HF | < 50ms latency |
| Documentation | 100% coverage | 100% coverage | 100% coverage |

---

## Appendix A: Alternative Approaches Considered

### Phase D Alternatives

1. **LangGraph integration** - Rejected: external dependency, Python
2. **CrewAI port** - Rejected: too opinionated, less flexible
3. **Actor model (actix)** - Considered: more complex than needed

### Phase E Alternatives

1. **PyTorch (via tch-rs)** - Rejected: large dependency, CUDA issues
2. **ONNX training** - Rejected: limited ecosystem
3. **Remote training only** - Rejected: user demand for local

### Phase F Alternatives

1. **Ollama-only serving** - Rejected: limited control
2. **vLLM integration** - Considered: could be future addition
3. **TensorRT-LLM** - Rejected: NVIDIA-only

---

## Appendix B: Competitive Analysis

| Feature | spn (proposed) | Ollama | vLLM | LangServe |
|---------|----------------|--------|------|-----------|
| Multi-agent | Yes (D) | No | No | Yes |
| Fine-tuning | Yes (E) | No | No | No |
| Edge deploy | Yes (F) | No | No | No |
| K8s native | Yes (F) | Manual | Yes | Yes |
| TUI | Yes | No | No | No |
| Rust native | Yes | Go | Python | Python |
| Local first | Yes | Yes | No | No |

---

*End of Design Document*
