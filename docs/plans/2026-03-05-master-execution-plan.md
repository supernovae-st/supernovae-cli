# Master Execution Plan: SPN Crate Architecture Refactor

**Date:** 2026-03-05
**Version:** v2.0
**Status:** EXECUTING
**Coordinated Plans:**
- `2026-03-05-secrets-architecture-refactor.md` (Plan 1)
- `2026-03-05-nika-spn-unified-architecture.md` (Plan 1B)
- `2026-03-05-plan2-model-management.md` (Plan 2)
- `2026-03-05-plan2b-model-backend-trait.md` (Plan 2B) ← NEW

---

## Executive Summary

Ce plan maître coordonne l'architecture complète spn↔nika en **9 phases**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASES OVERVIEW                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  FOUNDATION (Crates)                                                        │
│  ├── Phase 1A: spn-core (types, validation)        ✅ DONE                  │
│  ├── Phase 1B: spn-keyring (OS keychain)           → NEXT                   │
│  ├── Phase 1C: spn-core backend types              → Add PullProgress, etc  │
│  └── Phase 1D: spn-ollama (ModelBackend impl)      → NEW                    │
│                                                                             │
│  CLIENT + DAEMON                                                            │
│  ├── Phase 2: spn-client v0.2.0 (re-exports)                                │
│  └── Phase 3: spn daemon (model manager)                                    │
│                                                                             │
│  NIKA INTEGRATION                                                           │
│  ├── Phase 4A: Nika secrets migration                                       │
│  ├── Phase 4B: Nika AST + model: field             ← CRITICAL               │
│  └── Phase 4C: Nika LSP (completions, diagnostics)                          │
│                                                                             │
│  FINALIZATION                                                               │
│  ├── Phase 5: Publish crates.io                                             │
│  └── Phase 6: E2E tests (multi-model workflow)                              │
│                                                                             │
│  FUTURE (designed now)                                                      │
│  └── Phase 7: LlamaCppBackend (via llama-cpp-rs)                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Dependency Graph

```
                    spn-core (0 deps)
                         │
          ┌──────────────┼──────────────┐
          │              │              │
          ▼              ▼              ▼
     spn-keyring    spn-ollama    (future: spn-llama-cpp)
          │              │
          └──────┬───────┘
                 │
                 ▼
            spn-client (v0.2.0)
                 │
          ┌──────┴──────┐
          │             │
          ▼             ▼
        spn-cli       nika
```

---

## Pre-Flight Checklist

- [x] All plans reviewed and approved
- [x] Dependencies identified
- [x] Crate names available on crates.io
- [x] Phase 1A completed (spn-core 39 tests)
- [ ] All tests pass before continuing
- [ ] Clean git state

---

## Phase 1A: spn-core ✅ COMPLETE

**Status:** DONE (2026-03-05)
**Tests:** 39 passing (30 unit + 9 doctests)
**Clippy:** Zero warnings

**Created:**
```
crates/spn-core/
├── Cargo.toml (0 deps)
├── README.md
└── src/
    ├── lib.rs
    ├── providers.rs    # 13 providers, Copy + Eq + Hash
    ├── validation.rs   # validate_key_format + Display
    ├── mcp.rs          # McpServer, McpConfig
    └── registry.rs     # PackageRef, PackageManifest
```

**Code Review Applied:**
- `Provider` is now `Copy + Eq + Hash`
- `find_provider()` uses `eq_ignore_ascii_case` (zero allocation)
- `providers_by_category()` returns iterator
- `ValidationResult` implements `Display`
- `#[must_use]` on all public functions

---

## Phase 1B: spn-keyring → NEXT

**Goal:** OS keychain wrapper

**Files:**
```
crates/spn-keyring/
├── Cargo.toml
└── src/lib.rs
```

**Dependencies:**
```toml
spn-core = { path = "../spn-core" }
keyring = "3"
secrecy = "0.10"
zeroize = "1"
thiserror = "2"
```

**API:**
```rust
pub struct SpnKeyring;

impl SpnKeyring {
    pub fn get(provider: &str) -> Result<Zeroizing<String>, KeyringError>;
    pub fn set(provider: &str, key: &str) -> Result<(), KeyringError>;
    pub fn delete(provider: &str) -> Result<(), KeyringError>;
    pub fn list() -> Vec<String>;
}
```

---

## Phase 1C: spn-core Backend Types

**Goal:** Add types for ModelBackend trait (zero-dep, used by spn-ollama)

**Add to spn-core:**
```rust
// src/backend.rs

#[derive(Debug, Clone)]
pub struct PullProgress {
    pub status: String,
    pub completed: u64,
    pub total: u64,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub quantization: Option<String>,
    pub parameters: Option<String>,
    pub digest: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RunningModel {
    pub name: String,
    pub vram_used: Option<u64>,
    pub gpu_ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub id: u32,
    pub name: String,
    pub memory_total: u64,
    pub memory_free: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    NotRunning,
    ModelNotFound(String),
    AlreadyLoaded(String),
    InsufficientMemory,
    NetworkError(String),
    ProcessError(String),
}

#[derive(Debug, Clone, Default)]
pub struct LoadConfig {
    pub gpu_ids: Vec<u32>,
    pub gpu_layers: i32,  // -1 = all, 0 = none
    pub context_size: Option<u32>,
}
```

---

## Phase 1D: spn-ollama (ModelBackend Implementation)

**Goal:** Create spn-ollama crate with ModelBackend trait + Ollama impl

**Files:**
```
crates/spn-ollama/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── backend.rs      # ModelBackend trait
    ├── ollama.rs       # OllamaBackend implementation
    ├── client.rs       # HTTP client for Ollama API
    └── server.rs       # Process management
```

**Dependencies:**
```toml
spn-core = { path = "../spn-core" }
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1", features = ["process", "io-util"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
futures-util = "0.3"
thiserror = "2"
tracing = "0.1"
```

**Trait (Rust 1.75+ async fn in traits):**
```rust
pub trait ModelBackend: Send + Sync {
    fn id(&self) -> &'static str;
    async fn is_running(&self) -> bool;
    async fn start(&self) -> Result<(), BackendError>;
    async fn stop(&self) -> Result<(), BackendError>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError>;
    async fn running_models(&self) -> Result<Vec<RunningModel>, BackendError>;
    async fn pull(&self, name: &str, progress: Option<ProgressCb>) -> Result<(), BackendError>;
    async fn delete(&self, name: &str) -> Result<(), BackendError>;
    async fn load(&self, name: &str, config: &LoadConfig) -> Result<(), BackendError>;
    async fn unload(&self, name: &str) -> Result<(), BackendError>;
    fn endpoint_url(&self) -> &str;
}
```

---

## Phase 2: spn-client v0.2.0

**Changes:**
1. Add spn-core dependency
2. Re-export all spn-core types
3. Add config module (MCP loader)
4. Delete duplicated code

**Cargo.toml:**
```toml
[package]
name = "spn-client"
version = "0.2.0"

[dependencies]
spn-core = { path = "../spn-core" }
# ... existing deps
```

**lib.rs:**
```rust
// Re-export everything from spn-core
pub use spn_core::*;

// Client-specific modules
pub mod config;
pub mod ipc;
```

---

## Phase 3: spn Daemon with ModelManager

**Goal:** Add ModelManager to daemon

**New daemon modules:**
```
crates/spn/src/daemon/
├── mod.rs
├── server.rs           # Unix socket server
├── secrets.rs          # Use spn_core::KNOWN_PROVIDERS
├── model_manager.rs    # NEW: ModelManager
└── mcp_manager.rs      # Future: MCP process management
```

**ModelManager:**
```rust
pub struct ModelManager {
    backend: Box<dyn ModelBackend>,
    loaded_models: RwLock<HashMap<String, LoadedModelInfo>>,
}

impl ModelManager {
    pub async fn pull(&self, name: &str) -> Result<(), DaemonError>;
    pub async fn start(&self, name: &str, config: LoadConfig) -> Result<(), DaemonError>;
    pub async fn stop(&self, name: &str) -> Result<(), DaemonError>;
    pub async fn list(&self) -> Vec<ModelStatus>;
}
```

**IPC Protocol Extension:**
```rust
pub enum Request {
    // Existing
    GetSecret { provider: String },
    SetSecret { provider: String, key: String },

    // Model commands (NEW)
    ModelList,
    ModelPull { name: String },
    ModelStart { name: String, config: Option<LoadConfig> },
    ModelStop { name: String },
    ModelStatus { name: String },
}
```

---

## Phase 4A: Nika Secrets Migration

**Goal:** Use spn-client v0.2.0 for secrets

**DELETE:**
```
nika/tools/nika/src/tui/widgets/provider_modal/keyring.rs  # -281 LOC
```

**MODIFY:**
```
nika/tools/nika/
├── Cargo.toml              # spn-client = "0.2"
├── src/
│   ├── secrets.rs          # Use spn_core::KNOWN_PROVIDERS
│   └── mcp/spn_config.rs   # Use spn_client::McpConfig
```

---

## Phase 4B: Nika AST + model: Field ← CRITICAL

**Goal:** Add `model:` field to workflow and task

**Schema Example:**
```yaml
name: multi-model-workflow
model: anthropic/claude-sonnet-4  # Workflow default

tasks:
  - id: quick
    model: ollama/llama3.2        # Override: local
    infer: "Quick check..."

  - id: deep
    model: anthropic/claude-opus-4 # Override: powerful
    infer: "Deep reasoning..."

  - id: heavy
    model:                         # Advanced GPU config
      name: ollama/llama3.2:70b
      gpu: [0, 1]
    infer: "Heavy compute..."
```

**AST Types (new file: ast/model.rs):**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
    pub tag: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelConfig {
    pub model: ModelRef,
    pub gpu: Vec<u32>,
    pub layers: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSpec {
    Simple(ModelRef),
    Advanced(ModelConfig),
}
```

**Task Update:**
```rust
pub struct Task {
    pub id: TaskId,
    pub model: Option<ModelSpec>,  // NEW
    pub verb: TaskVerb,
    // ...
}
```

**Workflow Update:**
```rust
pub struct Workflow {
    pub name: String,
    pub model: Option<ModelSpec>,  // NEW
    pub tasks: Vec<Task>,
    // ...
}
```

---

## Phase 4C: Nika LSP Integration

**Goal:** Completions + diagnostics for model: field

**Completions:**
- Provider suggestions: `anthropic/`, `ollama/`, `openai/`
- Model suggestions based on provider
- For ollama: query local installed models

**Diagnostics:**
- Error: Unknown provider
- Warning: Model not installed (ollama)
- Error: Invalid GPU ID

**Hover:**
- Provider info (env var, description)
- Model info (size, parameters for local)

---

## Phase 5: Publish to crates.io

**Order:**
1. `spn-core` v0.1.0
2. `spn-keyring` v0.1.0
3. `spn-ollama` v0.1.0
4. `spn-client` v0.2.0

---

## Phase 6: E2E Tests

### Test 1: Multi-Model Workflow

```yaml
# test-multi-model.nika.yaml
name: multi-model-test
model: ollama/llama3.2

tasks:
  - id: local
    infer: "Hello from local"

  - id: cloud
    model: anthropic/claude-haiku
    infer: "Hello from cloud"
```

```bash
spn daemon start
cargo run -p nika -- run test-multi-model.nika.yaml --dry-run
spn daemon stop
```

### Test 2: GPU Allocation

```bash
spn model start llama3.2:70b --gpus 0,1
spn model status
# Expected: Shows GPU allocation
```

---

## Phase 7: LlamaCppBackend (Future, designed NOW)

**Goal:** Add llama.cpp support via llama-cpp-rs

**HTTP Server Option (OpenAI-compatible):**
```bash
./llama-server -m model.gguf --port 8080
# Same API as Ollama → same ModelBackend interface
```

**Native FFI Option (llama-cpp-rs):**
```rust
pub struct LlamaCppBackend {
    model_path: PathBuf,
    // Uses llama_cpp_2 crate directly
}

impl ModelBackend for LlamaCppBackend {
    // More control, but more complexity
}
```

**Recommendation:** Start with HTTP server mode (same interface), add native later.

---

## Rollback Plan

| Phase | Rollback |
|-------|----------|
| 1A | ✅ Complete, no rollback needed |
| 1B-1D | Delete new crate directories |
| 2-3 | `git checkout crates/` |
| 4A-4C | `git checkout` nika changes |
| 5 | `cargo yank` published crates |
| 6 | Tests non-destructive |

---

## Success Criteria

- [x] spn-core: 51 tests (41 unit + 10 doctests), 0 clippy warnings
- [x] spn-keyring: 6 tests, 0 clippy warnings
- [x] spn-ollama: 6 tests, ModelBackend trait + OllamaBackend + DynModelBackend
- [x] spn-client v0.2.0 with re-exports (7 tests, 0 clippy)
- [x] spn daemon with ModelManager (model_manager.rs, handler.rs updated)
- [ ] Nika: `model:` field in AST
- [ ] Nika: LSP completions/diagnostics
- [ ] Multi-model workflow E2E test passes
- [ ] All crates published to crates.io

---

## Execution Log

```
[x] Phase 1A started: 2026-03-05
[x] Phase 1A completed: 2026-03-05 (51 tests, 0 clippy)
[x] Phase 1B started: 2026-03-05
[x] Phase 1B completed: 2026-03-05 (6 tests, 0 clippy)
[x] Phase 1C started: 2026-03-05
[x] Phase 1C completed: 2026-03-05 (added backend.rs, 12 new tests)
[x] Phase 1D started: 2026-03-05
[x] Phase 1D completed: 2026-03-05 (6 tests, ModelBackend trait + OllamaBackend)
[x] Phase 2 started: 2026-03-05
[x] Phase 2 completed: 2026-03-05 (spn-client v0.2.0, re-exports spn-core, 7 tests)
[x] Phase 3 started: 2026-03-05
[x] Phase 3 completed: 2026-03-05 (ModelManager, IPC protocol extended, serde feature)
[ ] Phase 4A started:
[ ] Phase 4A completed:
[ ] Phase 4B started:
[ ] Phase 4B completed:
[ ] Phase 4C started:
[ ] Phase 4C completed:
[ ] Phase 5 started:
[ ] Phase 5 completed:
[ ] Phase 6 started:
[ ] Phase 6 completed:
```

---

## Next Actions

1. **Phase 4:** Migrate nika (secrets, AST model field, LSP)

Daemon complete with ModelManager. Ready for nika integration.
