# SuperNovae CLI Specification

**Project:** supernovae-cli (spn)
**Version:** v0.15.5
**License:** AGPL-3.0-or-later
**Language:** Rust (MSRV 1.85+)
**Status:** Production-ready with 1563+ tests

---

## 1. Project Overview

**spn** is the unified package manager and credential vault for the SuperNovae AI ecosystem. It solves the chaos of modern AI development:

```
Problem:                          Solution:
├── API keys in .env files   →   OS Keychain (unified)
├── MCP per-editor config    →   One ~/.spn/mcp.yaml
├── Local models scattered   →   spn model pull/load
├── Workflows lost in git    →   spn add @workflows/...
└── No orchestration         →   spn daemon (background IPC)
```

**Core Functions:**
1. **Package Management** — Install workflows, schemas, skills, MCP servers
2. **Credential Management** — Secure API keys in OS keychain
3. **Model Management** — Local LLM lifecycle (pull, load, unload, delete)
4. **MCP Server Management** — 48 built-in aliases, custom configs
5. **Editor Sync** — Push configs to Claude Code, Cursor, Windsurf
6. **Background Daemon** — Single keychain accessor, IPC server

---

## 2. Architecture

### 2.1 Workspace Structure

```
supernovae-cli/ (7 crates, strictly layered)
├── spn-core/        ← Layer 0: Definitions (zero deps)
├── spn-keyring/     ← Layer 1: OS keychain wrapper
├── spn-ollama/      ← Layer 1: Model backend (Ollama client)
├── spn-providers/   ← Layer 1: Backend abstraction (cloud + local)
├── spn-client/      ← Layer 2: Daemon SDK (IPC types)
├── spn-mcp/         ← Layer 2: REST-to-MCP wrapper
└── spn/             ← Layer 3: Main CLI + daemon
```

### 2.2 Crate Responsibilities

#### **spn-core (0.1.2)** — Zero-dependency type definitions

Provides the single source of truth for provider registry and validation.

**Exports:**
- `KNOWN_PROVIDERS: &[Provider]` — 13 providers (7 LLM + 6 MCP)
- `Provider` struct — id, name, category, env_var, key_prefix, endpoint
- `validate_key_format(provider, key)` → `ValidationResult` (Valid, InvalidPrefix, TooShort, etc.)
- `mask_key(secret)` → Visible prefix + masked suffix (e.g., `sk-ant-••••••••`)
- `provider_to_env_var(id)` → Environment variable name
- `McpConfig`, `McpServer`, `PackageManifest` — Configuration types

**Design:**
- No external dependencies (fast compilation, WASM-compatible)
- All provider definitions in one place for consistency
- Validation rules embedded in code, not external config

**Supported Providers:**
```
LLM Providers (7):
  - Anthropic (Claude)
  - OpenAI (GPT)
  - Mistral
  - Groq
  - DeepSeek
  - Google Gemini
  - Ollama (local)

MCP Providers (6):
  - Neo4j (knowledge graph)
  - GitHub (code integration)
  - Slack (messaging)
  - Perplexity (AI search)
  - Firecrawl (web scraping)
  - Supadata (data extraction)
```

#### **spn-keyring (0.1.4)** — OS keychain integration

Wraps platform-specific keychain APIs (macOS Keychain, Windows Credential Manager, Linux Secret Service).

**Exports:**
- `SpnKeyring::get/set/delete/exists(provider_id)` → `Zeroizing<String>`
- `resolve_key(provider_id)` → Priority: Keychain > Env > .env

**Security Properties:**
- Keys stored in OS-encrypted keychain (never in plain text)
- Retrieved into `Zeroizing<String>` (auto-clears on drop)
- `SecretString` wrapper (prevents Debug/Display leaks)
- `mlock()` on retrieval (prevents swap to disk on Unix)
- `MADV_DONTDUMP` (excludes from core dumps on Linux)

**Feature Flags:**
- `os-keychain` (default) — Full keyring integration
- Fallback mode (without feature) — Reads from env/env files only

#### **spn-client (0.3.3)** — Daemon SDK for external tools

Enables safe, non-blocking access to secrets from Nika and other tools.

**Exports:**
- `SpnClient::connect()` → Connects to `~/.spn/daemon.sock`
- `SpnClient::connect_with_fallback()` → Falls back to env if no daemon
- `async get_secret(provider_id) → SecretString`
- `async has_secret(provider_id) → bool`
- `async list_providers() → Vec<String>`
- IPC Protocol (see section 4.1)

**Use Case:**
```rust
// Nika workflow engine
let mut client = SpnClient::connect_with_fallback().await?;
let neo4j_key = client.get_secret("neo4j").await?;
// Zero keychain popups on macOS (daemon handles it once)
```

**Re-exports:**
- All `spn_core` types for convenience (Provider, validate_key_format, etc.)
- Secrecy types (SecretString, ExposeSecret)

#### **spn-ollama (0.1.6)** — Local model backend

Manages Ollama API for model download, loading, and inference.

**Exports:**
- `ModelBackend` trait — Pluggable backend interface
- `OllamaBackend` struct — Ollama implementation
- `pull()`, `delete()`, `load()`, `unload()`, `list_models()`, `running_models()`
- Retry logic with exponential backoff (network resilience)

**Capabilities:**
- Download models from Ollama registry (100+ available)
- Query VRAM usage and model info
- Infer hardware recommendations (future)

#### **spn-providers (0.1.0)** — Backend abstraction layer

Provides unified traits and orchestration for cloud and local LLM backends.

**Exports:**
- `ModelBackend` trait — Interface for local backends (Ollama, llama.cpp)
- `CloudBackend` trait — Interface for cloud providers (Anthropic, OpenAI, etc.)
- `BackendRegistry` — Manages registered backends
- `ModelOrchestrator` — Routes `@models/` aliases to correct backends
- `ModelRef`, `ModelAlias` — Model resolution types

**Features:**
- Conditional compilation via feature flags (anthropic, openai, mistral, etc.)
- Unified error handling via `BackendsError`
- Async trait support via `async-trait`

#### **spn-mcp (0.1.4)** — REST-to-MCP wrapper

Converts REST APIs into MCP servers (future extensibility).

**Planned Features:**
- OpenAPI 3.0 parsing
- Automatic MCP tool generation
- Rate limiting, auth handling

#### **spn-cli (0.15.5)** — Main CLI binary and daemon

The user-facing CLI and the background daemon process.

**Modules:**
- `commands/` — Subcommand handlers (add, remove, install, etc.)
- `daemon/` — Background server, IPC protocol, credential cache
- `config/` — Three-level config resolution (global/team/local)
- `secrets/` — Key management, migration, SOPS export
- `mcp/` — MCP server configuration, testing, file watching
- `status/` — System health dashboard
- `sync/` — Editor config writers (Claude Code, Cursor, Windsurf)
- `storage/` — Package installation and versioning
- `manifest/` — spn.yaml/spn.lock parsing
- `tui/` — Interactive prompts and progress bars

---

## 3. Core Concepts

### 3.1 Daemon Architecture (v0.10.0+)

**Problem:** Keychain access on macOS triggers repeated "allow access?" popups when multiple processes (Nika, MCP servers, IDE plugins) need credentials.

**Solution:** Single daemon process is the sole keychain accessor. Clients connect via Unix socket IPC.

```
BEFORE:                          AFTER:
Nika → Keychain (popup)          Nika → spn-client → daemon.sock
MCP1 → Keychain (popup)                              ↓
MCP2 → Keychain (popup)                           Keychain
MCP3 → Keychain (popup)                        (one accessor)
```

**Lifecycle:**
1. Auto-starts on first `spn provider get`
2. Socket at `~/.spn/daemon.sock` (permissions: 0600)
3. PID file with flock() for single-instance guarantee
4. Graceful shutdown with connection drain on SIGTERM/SIGINT

**IPC Protocol:** See section 4.1

### 3.2 Three-Level Configuration

Configuration scope: Local wins over Team wins over Global.

```
~/.spn/config.toml               ← Global defaults
   ↑
./mcp.yaml                       ← Team-shared (committed to git)
   ↑
./.spn/local.yaml                ← Local overrides (gitignored)
   ↓
Resolved configuration
```

**Usage:**
```bash
spn config show              # View resolved config
spn config get providers.anthropic.model
spn config set providers.anthropic.model claude-opus-4
spn config where             # Show all file locations
```

### 3.3 Provider Registry

13 providers defined in spn-core, accessible via KNOWN_PROVIDERS.

**Provider Structure:**
```rust
pub struct Provider {
    pub id: &'static str,                    // "anthropic"
    pub name: &'static str,                  // "Anthropic (Claude)"
    pub category: ProviderCategory,          // LLM or MCP
    pub env_var: &'static str,               // "ANTHROPIC_API_KEY"
    pub key_prefix: &'static str,            // "sk-ant-"
    pub endpoint: &'static str,              // "api.anthropic.com"
}
```

**Validation:**
```rust
pub enum ValidationResult {
    Valid,
    InvalidPrefix { expected, got },
    TooShort { min_length },
    InvalidFormat { expected_pattern },
    // ...
}
```

### 3.4 Key Resolution Priority

When spn needs an API key, it searches in this order:

1. **OS Keychain** (most secure) — `spn provider set` stores here
2. **Environment variable** — e.g., `$ANTHROPIC_API_KEY`
3. **.env file** — Via dotenvy crate

```bash
# Store in keychain (recommended)
spn provider set anthropic

# Or via environment
export ANTHROPIC_API_KEY=sk-ant-...

# Or in .env (least secure, for local dev only)
echo "ANTHROPIC_API_KEY=sk-ant-..." >> .env
```

### 3.5 MCP Server Management

**48 Built-in Aliases** (via npm packages):
- Database: neo4j, postgres, sqlite, supabase
- Dev Tools: github, gitlab, filesystem
- Search/AI: perplexity, brave-search, tavily, jina, exa
- Web: firecrawl, puppeteer, playwright
- Communication: slack, discord
- And 28+ more...

**Custom Configuration:**
```yaml
# ~/.spn/mcp.yaml (managed by spn)
servers:
  neo4j:
    command: npx
    args: ["-y", "@neo4j/mcp-neo4j"]
    env:
      NEO4J_URI: bolt://localhost:7687
      NEO4J_PASSWORD: ${spn:neo4j}  # Resolved by daemon
```

**Features:**
- `spn mcp add <name>` — Add from aliases
- `spn mcp test <name>` — Verify connection
- `spn mcp logs --follow` — Real-time log streaming
- `spn mcp adopt` — Adopt foreign MCPs (added directly to editors)

### 3.6 Model Management (v0.10.0+)

**Ollama Integration:**
```bash
spn model pull llama3.2:1b       # Download 1.2 GB
spn model load llama3.2:1b       # Load into VRAM
spn model status                 # Show running models + VRAM
spn model unload llama3.2:1b     # Free VRAM
spn model list                   # Show installed models
```

**Backend Trait:**
```rust
pub trait ModelBackend: Send + Sync {
    async fn is_running(&self) -> bool;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn pull(&self, name: &str, progress: impl Fn(PullProgress)) -> Result<()>;
    async fn load(&self, name: &str, config: LoadConfig) -> Result<RunningModel>;
    async fn unload(&self, name: &str) -> Result<()>;
    async fn running_models(&self) -> Result<Vec<RunningModel>>;
    // ...
}
```

---

## 4. API/Interface Specifications

### 4.1 Daemon IPC Protocol

**Transport:** Unix domain sockets at `~/.spn/daemon.sock`

**Security:**
- Socket permissions: 0600 (owner-only)
- Peer credential verification: `SO_PEERCRED` (Unix) / `LOCAL_PEERCRED` (macOS)

**Protocol Version:** 1 (defined in spn-client)

**Request/Response Types:**

```rust
pub enum Request {
    Ping,
    GetSecret { provider_id: String },
    HasSecret { provider_id: String },
    ListProviders,
    RefreshSecret { provider_id: String },
    // ... more types
}

pub enum Response {
    Pong,
    SecretValue(SecretString),
    Boolean(bool),
    ProviderList(Vec<String>),
    RefreshStatus { was_cached: bool },
    Error { message: String },
}
```

**Example Flow:**
```
Client:     Ping
Daemon:     Pong

Client:     GetSecret { provider_id: "anthropic" }
Daemon:     SecretValue(sk-ant-••••••••) [masked in logs]

Client:     HasSecret { provider_id: "openai" }
Daemon:     Boolean(false)
```

### 4.2 Command Interface

**Package Management:**
```bash
spn add <package>                # Add to project (spn.yaml)
spn remove <package>             # Remove
spn install [--frozen]           # Install from spn.lock
spn update [package]             # Update to latest
spn list                         # List installed
spn search <query>               # Search registry
spn info <package>               # Show package metadata
spn outdated                     # List outdated
```

**Credential Management:**
```bash
spn provider list                # Show all keys + status
spn provider set <name>          # Store in keychain (interactive)
spn provider get <name>          # Retrieve (masked)
spn provider delete <name>       # Remove from keychain
spn provider migrate             # Move env vars → keychain
spn provider test [name]         # Validate key format
```

**Model Management:**
```bash
spn model list                   # List installed
spn model pull <name>            # Download
spn model load <name>            # Load into VRAM
spn model unload <name>          # Free VRAM
spn model status                 # Show running + memory
spn model delete <name>          # Remove local
```

**MCP Server Management:**
```bash
spn mcp list                     # List configured
spn mcp add <name>               # Add from aliases or file
spn mcp remove <name>            # Remove
spn mcp test <name>              # Verify connection
spn mcp logs [--follow]          # Show/stream MCP logs
spn mcp status                   # Detailed MCP info
spn mcp adopt                    # Adopt foreign MCPs
```

**Configuration:**
```bash
spn config show                  # View resolved config
spn config get <path>            # Get single value (dot notation)
spn config set <path> <value>    # Set value
spn config where                 # Show file locations
```

**Sync:**
```bash
spn sync                         # Sync to all enabled editors
spn sync --target <editor>       # Sync to one editor
spn sync --enable <editor>       # Enable auto-sync
spn sync --interactive           # Preview changes before writing
```

**System:**
```bash
spn setup                        # Interactive onboarding
spn setup nika                   # Install Nika (5-step wizard)
spn setup novanet                # Install NovaNet + Neo4j
spn status [--json]              # System dashboard
spn doctor                       # Health check
spn nk <args>                    # Proxy to nika CLI
spn nv <args>                    # Proxy to novanet CLI
```

### 4.3 Configuration Format

**spn.yaml — Project Manifest**
```yaml
version: "1"

packages:
  - "@nika/generate-page"
  - "@novanet/localization"
  - "@workflows/code-review"

settings:
  model_backend: ollama
  default_provider: anthropic
  sync:
    auto_sync: true
    targets: [claude-code, cursor]
```

**spn.lock — Dependency Lock**
```yaml
version: "1"
packages:
  "@nika/generate-page":
    version: "1.2.3"
    hash: "sha256:..."
    timestamp: "2026-03-10T00:00:00Z"
```

**~/.spn/config.toml — Global Config**
```toml
[providers.anthropic]
model = "claude-opus-4"

[providers.openai]
model = "gpt-4"

[sync]
auto_sync = false

[daemon]
preload = true
lazy_loading = false
```

**./.spn/local.yaml — Local Overrides**
```yaml
providers:
  anthropic:
    model: claude-3-sonnet

sync:
  auto_sync: true
```

**mcp.yaml — MCP Server Config (Team-shared)**
```yaml
servers:
  neo4j:
    command: npx
    args: ["-y", "@neo4j/mcp-neo4j"]
    env:
      NEO4J_URI: bolt://localhost:7687
      NEO4J_USERNAME: neo4j
      NEO4J_PASSWORD: ${spn:neo4j}

  github:
    command: npx
    args: ["-y", "@github/mcp-github"]
    env:
      GITHUB_TOKEN: ${spn:github}
```

---

## 5. Security Model

### 5.1 Secret Storage

**Priority Order (High to Low Security):**
1. **OS Keychain** (recommended)
   - macOS: Keychain Access (encrypted with login password)
   - Windows: Credential Manager
   - Linux: Secret Service (GNOME Keyring, KWallet)

2. **Environment Variables**
   - Read-only access, no shell history
   - Suitable for CI/CD but not local

3. **.env Files**
   - Plain text, only for development
   - Must be gitignored

### 5.2 Memory Protection

All keys in memory are protected:

| Mechanism | Purpose |
|-----------|---------|
| `Zeroizing<T>` | Auto-clear on drop (zeroize crate) |
| `SecretString` | Prevents Debug/Display exposure (secrecy crate) |
| `mlock()` | Prevents swap to disk (Unix only, via libc) |
| `MADV_DONTDUMP` | Excludes from core dumps (Linux) |

```rust
// Key is auto-zeroed when dropped
let key: Zeroizing<String> = SpnKeyring::get("anthropic")?;
let exposed = key.expose_secret(); // Must explicitly expose
// key is zeroed here ↓
```

### 5.3 Daemon Security

**Socket-level:**
- Permissions: 0600 (owner-only)
- Peer credential verification (prevents privilege escalation)

**Process-level:**
- Single-instance guarantee via PID file with flock()
- SIGTERM graceful shutdown (connection drain)

**IPC-level:**
- No credential forwarding in request/response
- SecretString types prevent leaks in logs

### 5.4 Deployment Security

**Feature Flags:**
- `os-keychain` (default) — Full keyring for native builds
- Without feature — Falls back to env vars (suitable for Docker)

**Docker Build:**
```dockerfile
FROM rust:latest AS builder
RUN cargo build --release --no-default-features

# scratch image (5 MB, no CA certs needed)
FROM scratch
COPY --from=builder /app/spn /spn
```

---

## 6. Configuration System

### 6.1 Resolution Algorithm

```
1. Load global config from ~/.spn/config.toml
2. Load team config from ./mcp.yaml (if exists)
3. Load local config from ./.spn/local.yaml (if exists)
4. Merge: local > team > global (local wins)
5. Apply environment variable overrides
```

### 6.2 Supported Keys

**Top-level sections:**
- `providers.*` — Provider defaults (model, endpoint, etc.)
- `sync.*` — Sync settings (auto_sync, targets)
- `daemon.*` — Daemon settings (preload, lazy_loading)
- `storage.*` — Package storage location
- `mcp.*` — MCP server defaults

**Example dot notation:**
```bash
spn config get providers.anthropic.model
# → "claude-opus-4"

spn config set providers.anthropic.model claude-3-sonnet
# Updates ~/.spn/config.toml (global)

spn config set --local providers.anthropic.model claude-3-opus
# Updates ./.spn/local.yaml (local)
```

### 6.3 Environment Variable Fallbacks

Each setting can be overridden via environment variables:

```bash
export SPN_PROVIDERS_ANTHROPIC_MODEL=claude-3-opus
export SPN_SYNC_AUTO_SYNC=true
export SPN_DAEMON_LAZY_LOADING=false
```

---

## 7. Integration Points

### 7.1 Nika Integration

**Nika reads MCP configs directly from spn:**

```rust
// nika/src/mcp/client.rs
let mcp_config = SpnConfig::load_mcp_config()?;
// Reads ~/.spn/mcp.yaml

for server in mcp_config.servers {
    // Spawn MCP server with spn-resolved credentials
    let env = resolve_env(&server.env)?;
    // Calls spn daemon: GetSecret for ${spn:neo4j}
}
```

**Benefits:**
- No config duplication (single source of truth)
- Credentials resolved by daemon (no keychain popups in Nika)
- Automatic sync (Nika sees new MCPs without restart)

### 7.2 NovaNet Integration

**NovaNet uses spn-client for credential access:**

```rust
// novanet/src/neo4j/connection.rs
let client = SpnClient::connect_with_fallback().await?;
let neo4j_password = client.get_secret("neo4j").await?;
```

### 7.3 Editor Sync

**spn syncs to three editors:**
- Claude Code: `.claude/settings.json` (MCP servers array)
- Cursor: `.cursor/mcp.json`
- Windsurf: `.windsurf/mcp.json`

**Workflow:**
```bash
spn mcp add neo4j                # Updates ~/.spn/mcp.yaml
spn sync                         # Writes to all enabled editors
# Claude Code automatically detects changes via file watcher
```

---

## 8. Feature Flags and Build Variants

### 8.1 spn-keyring Feature Flags

| Flag | Default | Effect |
|------|---------|--------|
| `os-keychain` | Yes | Enable OS keychain integration |

### 8.2 spn-cli Feature Flags

| Flag | Default | Effect |
|------|---------|--------|
| `native` | Yes | Full features (keychain + daemon) |
| `docker` | No | Minimal (no keychain, pure env fallback) |

### 8.3 Build Targets

**Native (Full Features):**
```bash
cargo build --release
# Includes: keychain, daemon, all MCP aliases
```

**Docker (Minimal):**
```bash
cargo build --release --no-default-features --features docker
# Excludes: keychain (env fallback only)
# Result: 5 MB scratch image
```

---

## 9. Testing Strategy

**Test Coverage:** 1563+ tests passing

| Category | Count | Examples |
|----------|-------|----------|
| Unit Tests | 600+ | Provider validation, key masking, config resolution |
| Integration Tests | 400+ | Daemon IPC, credential retrieval, editor sync |
| CLI Tests | 288+ | Command parsing, subcommand execution |

**CI/CD:**
- GitHub Actions: test.yml (all platforms)
- Linting: `cargo clippy -- -D warnings` (zero warnings)
- Formatting: `cargo fmt --check` (all formatted)
- MSRV: Rust 1.85+ (enforced in CI)

---

## 10. Release Process

**Fully Automated (Zero Manual Steps):**

1. **Prepare** — `git push` to main
2. **Validation** — GitHub Actions: test, clippy, semver-checks
3. **Release PR** — release-plz creates PR with version bumps
4. **Merge** — `git merge` release PR
5. **Publish** — GitHub Actions:
   - Create git tag (`v0.15.5`)
   - Build binaries (macOS, Linux native + musl)
   - Build Docker image (ghcr.io/supernovae-st/spn)
   - Publish to crates.io (all 6 crates in dependency order)
   - Create GitHub Release with binaries + SBOM + SLSA provenance

**Tools:**
- release-plz — Version management and crates.io publishing
- git-cliff — CHANGELOG generation from conventional commits
- cargo-semver-checks — SemVer compliance validation

---

## 11. Dependencies

### Core Dependencies (spn-core)

**Zero external dependencies** — Pure Rust, WASM-compatible.

### Workspace Dependencies

| Crate | Key Dependencies |
|-------|------------------|
| spn-core | None |
| spn-keyring | secrecy, zeroize, keyring, libc |
| spn-client | secrecy, tokio, serde |
| spn-ollama | reqwest, serde, tokio |
| spn-cli | clap, reqwest, tokio, serde_yaml, toml, notify |

**Version Policy:**
- Prefer stable, widely-used crates
- No vendored dependencies (use crates.io)
- Minimal transitive dependency tree

---

## 12. Error Handling

**Error Types:**

| Error | Source | Handling |
|-------|--------|----------|
| `ValidationError` | spn-core | Format check fails (caught at input) |
| `KeyringError` | spn-keyring | OS keychain operation fails (retry/fallback) |
| `IpcError` | spn-client | Daemon socket unavailable (fallback to env) |
| `BackendError` | spn-ollama | Ollama API fails (retry with exponential backoff) |
| `CliError` | spn-cli | User input, file I/O (graceful messages) |

**Example:**
```bash
$ spn provider set anthropic
Error: Invalid key format
Expected prefix: sk-ant-
Got: sk-openai-...

Hint: Did you mean to use 'spn provider set openai'?
```

---

## 13. Performance Characteristics

### Startup Time
- First invocation: ~50ms (Rust startup)
- Daemon initialization: ~100ms (preload) or instant (lazy)

### Memory Usage
- spn CLI process: ~10 MB resident
- spn daemon process: ~5 MB + secret cache

### IPC Latency
- Local socket communication: <1ms per request
- Keychain access (daemon): ~10-50ms (varies by OS)

---

## 14. Future Roadmap

**Phase A (v0.16.0) — Unified Backend Registry** ✅ In Progress
- `@models/` aliases in spn.yaml
- Cloud providers as backends
- Intent-based model selection

**Phase B (v0.17.0) — Multimodal Backends**
- Candle (HuggingFace models)
- Vision models, image generation/analysis
- Speech-to-text, text-to-speech

**Phase C (v0.17.5) — Hardware-Aware Recommendations**
- llmfit-core integration
- System resource detection
- Model scoring based on hardware

**Phase D (v0.18.0) — Reasoning Models**
- OpenAI o1/o3 support
- DeepSeek-R1 support
- Reasoning trace capture

**Phase E (v0.18.5) — Agentic Capabilities**
- Nested agent spawning
- Schema introspection
- Dynamic task decomposition

**Phase F (v0.19.0) — MCP Auto-Sync**
- File system monitoring
- Foreign MCP detection
- Desktop notifications

---

## 15. See Also

| Document | Purpose |
|----------|---------|
| **README.md** | User guide, quick start, features |
| **CLAUDE.md** | Project context for Claude Code |
| **CHANGELOG.md** | Version history and release notes |
| **crates/*/src/lib.rs** | API documentation (rustdoc) |

---

**Last Updated:** 2026-03-10
**Maintainer:** SuperNovae Studio (@ThibautMelen, @NicolasCELLA)
**License:** AGPL-3.0-or-later
