# supernovae-cli

**SuperNovae CLI (`spn`)** v0.15.0 — The Agentic AI Toolkit for the SuperNovae ecosystem.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn — The Agentic AI Toolkit v0.15.0                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Package Commands:                                                              │
│  ├── spn add <package>          Add package to project                          │
│  ├── spn remove <package>       Remove package                                  │
│  ├── spn install [--frozen]     Install from spn.yaml                           │
│  ├── spn update [package]       Update packages                                 │
│  ├── spn list                   List installed packages                         │
│  ├── spn search <query>         Search registry                                 │
│  ├── spn info <package>         Show package info                               │
│  └── spn outdated               List outdated packages                          │
│                                                                                 │
│  Security Commands:                                                             │
│  ├── spn provider list          List API keys and sources                       │
│  ├── spn provider set <name>    Store key in OS Keychain                        │
│  ├── spn provider get <name>    Get key (masked by default)                     │
│  ├── spn provider delete <name> Remove key from keychain                        │
│  ├── spn provider migrate       Move env vars to keychain                       │
│  └── spn provider test <name>   Validate key format                             │
│                                                                                 │
│  Model Commands (v0.10.0):                                                      │
│  ├── spn model list             List local models (via Ollama)                  │
│  ├── spn model pull <name>      Download model                                  │
│  ├── spn model load <name>      Load model into memory                          │
│  ├── spn model unload <name>    Unload model from memory                        │
│  └── spn model delete <name>    Delete local model                              │
│                                                                                 │
│  Skill/MCP Commands:                                                            │
│  ├── spn skill add/remove/list  Manage skills (via skills.sh)                   │
│  └── spn mcp add/remove/list    Manage MCP servers (via npm)                    │
│                                                                                 │
│  Setup Commands (v0.15.0):                                                      │
│  ├── spn setup                  Interactive onboarding wizard                   │
│  ├── spn setup nika             Install and configure Nika workflow engine      │
│  └── spn setup novanet          Install and configure NovaNet knowledge graph   │
│                                                                                 │
│  Status & Integration:                                                          │
│  ├── spn status [--json]        System status dashboard                         │
│  ├── spn nk <args>              Proxy to nika CLI                               │
│  ├── spn nv <args>              Proxy to novanet CLI                            │
│  ├── spn sync                   Sync packages to editor configs                 │
│  └── spn doctor                 Verify installation health                      │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Tech Stack

- **Language:** Rust 2021 (MSRV 1.85)
- **CLI Framework:** clap v4
- **HTTP Client:** reqwest (rustls)
- **Async Runtime:** tokio
- **Serialization:** serde, serde_yaml, serde_json, toml
- **Security:** keyring (OS keychain), secrecy, zeroize, libc (mlock)
- **Performance:** rustc-hash (FxHashMap)

## Workspace Architecture (v0.15.0)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  UNIFIED 6-CRATE ARCHITECTURE                                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │  LAYER 1: spn-core (Zero Dependencies, WASM-Compatible)                  │  │
│  │  ├── KNOWN_PROVIDERS (13 providers: 7 LLM + 6 MCP)                       │  │
│  │  ├── Provider { id, name, env_var, category, key_prefix }                │  │
│  │  ├── validate_key_format(), mask_key(), provider_to_env_var()            │  │
│  │  └── McpServer, McpConfig (generic transport types)                      │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│                                      ▼                                          │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │  LAYER 2: spn-keyring (OS Keychain Wrapper)                              │  │
│  │  ├── SpnKeyring::get/set/delete/exists (macOS/Windows/Linux)             │  │
│  │  ├── Zeroizing<String>, SecretString (memory protection)                 │  │
│  │  └── resolve_key() with priority: keychain > env > .env                  │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│                                      ▼                                          │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │  LAYER 3: spn-client (Unix Socket IPC + Re-exports)                      │  │
│  │  ├── SpnClient::connect() → daemon at ~/.spn/daemon.sock                 │  │
│  │  ├── SpnClient::connect_with_fallback() → env var fallback               │  │
│  │  ├── Re-exports: KNOWN_PROVIDERS, Provider, validate_key_format()        │  │
│  │  └── Protocol: Ping/GetSecret/HasSecret/ListProviders                    │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│              ┌───────────────────────┴───────────────────────┐                  │
│              │                                               │                  │
│              ▼                                               ▼                  │
│  ┌────────────────────────┐                   ┌────────────────────────┐        │
│  │  spn-cli (v0.15.0)     │                   │  Nika (v0.21.1)        │        │
│  │  • provider set/get    │                   │  • spn-daemon feature  │        │
│  │  • model pull/load     │                   │  • KNOWN_PROVIDERS     │        │
│  │  • setup wizard        │                   │  • Unified secrets     │        │
│  └────────────────────────┘                   └────────────────────────┘        │
│                                                                                 │
│  Also in workspace:                                                             │
│  • spn-ollama (ModelBackend trait, Ollama API client)                           │
│  • spn-mcp (Dynamic REST-to-MCP wrapper, MCP server binary)                     │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Why This Architecture?

**Problem Solved:** macOS Keychain popup fatigue. Each process accessing Keychain
triggers "allow access?" dialogs. With Nika spawning multiple MCP servers, this
was unbearable.

**Solution:** The `spn daemon` is the SOLE keychain accessor. Consumers (Nika, MCP
servers) connect via Unix socket IPC. One auth prompt at daemon start, then silence.

```
Without daemon:           With daemon:
Nika → Keychain (popup)   Nika → spn-client → daemon.sock → Keychain
MCP1 → Keychain (popup)                        (one accessor, no popups)
MCP2 → Keychain (popup)
```

## Project Structure

```
supernovae-cli/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── spn-core/           # Shared types, provider definitions
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs # LLM/MCP provider registry
│   │       ├── backend.rs  # BackendError, ModelInfo, LoadConfig
│   │       └── validate.rs # Key format validation
│   │
│   ├── spn-keyring/        # OS keychain integration
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── keyring.rs  # Platform-specific keychain
│   │       └── memory.rs   # mlock/LockedBuffer/Zeroizing
│   │
│   ├── spn-ollama/         # Ollama backend
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs   # HTTP client for Ollama API
│   │       ├── backend.rs  # ModelBackend trait
│   │       └── ollama.rs   # OllamaBackend implementation
│   │
│   ├── spn-client/         # SDK for external tools
│   │   └── src/lib.rs      # Re-exports spn-core types
│   │
│   ├── spn-mcp/            # Dynamic REST-to-MCP wrapper
│   │   └── src/
│   │       ├── main.rs     # MCP server binary
│   │       ├── config.rs   # YAML config parser
│   │       ├── endpoints.rs # REST endpoint mapping
│   │       └── tools.rs    # MCP tool generation
│   │
│   └── spn/                # Main CLI (spn-cli)
│       └── src/
│           ├── main.rs     # Entry point + CLI definition
│           ├── commands/   # CLI subcommands
│           ├── index/      # Registry client + downloader
│           ├── manifest/   # spn.yaml + spn.lock parsing
│           ├── storage/    # Local package storage
│           ├── sync/       # IDE config sync
│           ├── interop/    # Binary proxies
│           ├── secrets/    # Credential management
│           └── error.rs    # Error types
│
├── CHANGELOG.md
├── CLAUDE.md
└── README.md
```

## Daemon Architecture (v0.10.0)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DAEMON (spn-daemon) — Background Credential Cache                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  IPC Layer:                                                                     │
│  ├── Unix socket (~/.spn/daemon.sock)                                           │
│  ├── Socket permissions: 0600 (owner-only)                                      │
│  ├── Peer credential verification (SO_PEERCRED / LOCAL_PEERCRED)               │
│  └── PID file with flock() for single-instance guarantee                       │
│                                                                                 │
│  Lifecycle:                                                                     │
│  ├── Auto-start on first `spn provider get`                                    │
│  ├── Graceful shutdown with JoinSet task tracking                              │
│  ├── SIGTERM/SIGINT handling                                                   │
│  └── Connection drain on shutdown                                              │
│                                                                                 │
│  Security:                                                                      │
│  ├── mlock() on secret memory (prevents swap)                                  │
│  ├── MADV_DONTDUMP (excludes from core dumps)                                  │
│  ├── Zeroizing<T> wrapper (auto-clear on drop)                                 │
│  └── SecretString (prevents Debug/Display exposure)                            │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Model Management (v0.10.0)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  MODEL MANAGER — Local Model Lifecycle                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ModelBackend Trait:                                                            │
│  ├── is_running()      Check if backend is available                           │
│  ├── start() / stop()  Control backend process                                 │
│  ├── list_models()     List installed models                                   │
│  ├── model_info()      Get model details (size, quant, params)                │
│  ├── pull()            Download model with progress callback                   │
│  ├── delete()          Remove local model                                      │
│  ├── load() / unload() Control model memory residence                         │
│  └── running_models()  List currently loaded models                           │
│                                                                                 │
│  DynModelBackend:                                                               │
│  └── Object-safe version for runtime polymorphism (Box<dyn DynModelBackend>)   │
│                                                                                 │
│  Backends:                                                                      │
│  └── Ollama (implemented) — more backends planned                              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Security Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  SECRETS MANAGEMENT                                                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Storage Layer:                                                                 │
│  ├── OS Keychain (macOS/Windows/Linux)     Encrypted, protected by login       │
│  ├── Environment variables                  Less secure, but convenient         │
│  └── .env files                             Least secure, dev convenience       │
│                                                                                 │
│  Memory Protection:                                                             │
│  ├── Zeroizing<T>      Auto-clear on drop (zeroize crate)                       │
│  ├── SecretString      Prevents Debug/Display exposure (secrecy crate)         │
│  ├── mlock()           Prevents swap to disk (Unix via libc)                   │
│  └── MADV_DONTDUMP     Excludes from core dumps (Linux)                        │
│                                                                                 │
│  Key Resolution Priority:                                                       │
│  1. OS Keychain (most secure)                                                   │
│  2. Environment variable                                                        │
│  3. .env file (via dotenvy)                                                     │
│                                                                                 │
│  Supported Providers:                                                           │
│  ├── LLM: anthropic, openai, mistral, groq, deepseek, gemini, ollama           │
│  └── MCP: neo4j, github, slack, perplexity, firecrawl, supadata                │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Release Automation (v0.15.0)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  FULLY AUTOMATED RELEASE PIPELINE                                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Tools:                                                                         │
│  ├── release-plz              Automated release PRs and crates.io publishing   │
│  ├── git-cliff                CHANGELOG generation from conventional commits   │
│  └── cargo-semver-checks      SemVer compatibility validation                  │
│                                                                                 │
│  Workflow (Zero Manual Steps):                                                  │
│  ├── 1. Push to main          Triggers release-plz.yml                         │
│  ├── 2. Validation            fmt, clippy, tests, semver-checks                │
│  ├── 3. Release PR created    Version bumps + CHANGELOG updates                │
│  ├── 4. Merge PR              Triggers release.yml                             │
│  ├── 5. Git tag created       v0.X.Y format                                    │
│  ├── 6. Binaries built        macOS, Linux (native + musl), Windows            │
│  ├── 7. Docker published      ghcr.io/supernovae-st/spn (~5MB scratch image)   │
│  ├── 8. crates.io published   All 6 crates in dependency order                 │
│  └── 9. GitHub Release        With binaries, SLSA provenance, SBOM             │
│                                                                                 │
│  Configuration Files:                                                           │
│  ├── cliff.toml               git-cliff configuration                          │
│  ├── release-plz.toml         release-plz workspace/package config             │
│  └── .github/workflows/       release-plz.yml + release.yml                    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Feature Flags (v0.15.0)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  CONDITIONAL COMPILATION FOR DIFFERENT BUILD TARGETS                            │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn-keyring crate:                                                             │
│  ├── default = ["os-keychain"]                                                  │
│  └── os-keychain              Enable OS keychain integration (keyring crate)   │
│                                                                                 │
│  spn-cli crate:                                                                 │
│  ├── default = ["native"]                                                       │
│  ├── native                   Full features including OS keychain              │
│  ├── os-keychain              Optional keychain support                        │
│  └── docker                   Minimal build for containers (no keychain)       │
│                                                                                 │
│  Build Targets:                                                                 │
│  ├── Native (macOS/Linux/Windows)   Full features, dynamic linking             │
│  └── Docker (musl)                  Static binary, no keychain, scratch image  │
│                                                                                 │
│  Fallback Behavior:                                                             │
│  └── Without keychain, resolve_api_key() falls back to env vars                │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Commands

```bash
# Build
cargo build --release

# Run
cargo run -p spn-cli -- help
cargo run -p spn-cli -- doctor
cargo run -p spn-cli -- add @workflows/dev/code-review

# Security
cargo run -p spn-cli -- provider list
cargo run -p spn-cli -- provider set anthropic
cargo run -p spn-cli -- provider migrate
cargo run -p spn-cli -- provider test all

# Models (v0.10.0)
cargo run -p spn-cli -- model list
cargo run -p spn-cli -- model pull llama3.2:7b

# Setup (v0.15.0)
cargo run -p spn-cli -- setup              # Interactive wizard
cargo run -p spn-cli -- setup nika         # Install Nika
cargo run -p spn-cli -- setup novanet      # Install NovaNet

# Test (920+ tests across workspace)
cargo test --workspace

# Lint (warnings = errors)
cargo clippy --workspace -- -D warnings

# Install locally
cargo install --path crates/spn
```

## Test Stats

- **1288 tests passing** across workspace
- **Zero clippy errors** with `-D warnings`
- **MSRV:** Rust 1.85+

## Crate Versions

| Crate | Version | crates.io |
|-------|---------|-----------|
| spn-core | 0.1.1 | [Published](https://crates.io/crates/spn-core) |
| spn-keyring | 0.1.3 | [Published](https://crates.io/crates/spn-keyring) |
| spn-ollama | 0.1.4 | [Published](https://crates.io/crates/spn-ollama) |
| spn-client | 0.3.1 | [Published](https://crates.io/crates/spn-client) |
| spn-mcp | 0.1.2 | [Published](https://crates.io/crates/spn-mcp) |
| spn-cli | 0.15.0 | [Published](https://crates.io/crates/spn-cli) |

## Storage Layout

```
~/.spn/
├── config.toml           # User config
├── daemon.sock           # Unix socket (v0.10.0)
├── daemon.pid            # PID file with flock
├── registry/             # Index cache
├── packages/             # Installed packages
│   └── @scope/name/version/
│       ├── manifest.yaml
│       └── skills/
└── bin/                  # Binary stubs (nika, novanet)
```

## DX Setup

For full Claude Code DX (skills, hooks, agents), create a symlink:

```bash
ln -s ../supernovae-agi/dx/.claude .claude
```

## Related Repos

| Repo | Description |
|------|-------------|
| [supernovae-agi](https://github.com/supernovae-st/supernovae-agi) | Monorepo (NovaNet + Nika) |
| [homebrew-tap](https://github.com/supernovae-st/homebrew-tap) | Homebrew formulas |
| [supernovae-registry](https://github.com/supernovae-st/supernovae-registry) | Public package registry |

## Conventions

| Aspect | Convention |
|--------|------------|
| Commits | `type(scope): description` with co-authors |
| Code Style | `cargo fmt` + `cargo clippy -- -D warnings` |
| Testing | TDD preferred, 80% coverage target |
| MSRV | Rust 1.85+ |

---

**Distribution:**
- Homebrew: `brew install supernovae-st/tap/spn`
- Docker: `docker pull ghcr.io/supernovae-st/spn:latest`
- Cargo: `cargo install spn-cli`
