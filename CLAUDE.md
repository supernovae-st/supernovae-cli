# supernovae-cli

**SuperNovae CLI (`spn`)** v0.10.0 — Unified package manager for the SuperNovae AI workflow ecosystem.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn — SuperNovae Package Manager v0.10.0                                       │
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
│  Integration:                                                                   │
│  ├── spn nk <args>              Proxy to nika CLI                               │
│  ├── spn nv <args>              Proxy to novanet CLI                            │
│  ├── spn sync                   Sync packages to editor configs                 │
│  └── spn doctor                 Verify installation health                      │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Tech Stack

- **Language:** Rust 2021 (MSRV 1.75)
- **CLI Framework:** clap v4
- **HTTP Client:** reqwest (rustls)
- **Async Runtime:** tokio
- **Serialization:** serde, serde_yaml, serde_json, toml
- **Security:** keyring (OS keychain), secrecy, zeroize, libc (mlock)
- **Performance:** rustc-hash (FxHashMap)

## Workspace Architecture (v0.10.0)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  CARGO WORKSPACE — 5 INDEPENDENT CRATES                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                        │
│  │  spn-core    │   │  spn-keyring │   │  spn-ollama  │                        │
│  │   v0.1.0     │   │    v0.1.0    │   │    v0.1.0    │                        │
│  ├──────────────┤   ├──────────────┤   ├──────────────┤                        │
│  │ • Provider   │   │ • OS keychain│   │ • Ollama API │                        │
│  │   definitions│   │   (macOS/Win │   │ • ModelBackend│                        │
│  │ • BackendErr │   │    /Linux)   │   │   trait      │                        │
│  │ • ModelInfo  │   │ • Secret mgmt│   │ • Pull/Load/ │                        │
│  │ • Validation │   │ • mlock()    │   │   Unload     │                        │
│  └──────────────┘   └──────────────┘   └──────────────┘                        │
│         │                  │                  │                                 │
│         └──────────────────┼──────────────────┘                                 │
│                            │                                                    │
│                            ▼                                                    │
│                   ┌──────────────┐                                              │
│                   │  spn-client  │  ← SDK for external tools                   │
│                   │    v0.2.1    │    (re-exports spn-core)                    │
│                   └──────────────┘                                              │
│                            │                                                    │
│                            ▼                                                    │
│                   ┌──────────────┐                                              │
│                   │   spn-cli    │  ← Main binary                              │
│                   │   v0.10.0    │    (all commands)                           │
│                   └──────────────┘                                              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
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

# Test (610 tests across workspace)
cargo test --workspace

# Lint (warnings = errors)
cargo clippy --workspace -- -D warnings

# Install locally
cargo install --path crates/spn
```

## Test Stats

- **610 tests passing** across workspace
- **Zero clippy errors** with `-D warnings`
- **MSRV:** Rust 1.85+

## Crate Versions

| Crate | Version | crates.io |
|-------|---------|-----------|
| spn-core | 0.1.0 | [Published](https://crates.io/crates/spn-core) |
| spn-keyring | 0.1.0 | [Published](https://crates.io/crates/spn-keyring) |
| spn-ollama | 0.1.0 | [Published](https://crates.io/crates/spn-ollama) |
| spn-client | 0.2.1 | [Published](https://crates.io/crates/spn-client) |
| spn-cli | 0.10.0 | [Published](https://crates.io/crates/spn-cli) |

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

**Distribution:** `brew install supernovae-st/tap/spn`
