# supernovae-cli

**SuperNovae CLI (`spn`)** v0.6.0 — Unified package manager for the SuperNovae AI workflow ecosystem.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn — SuperNovae Package Manager v0.6.0                                        │
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
│  Security Commands (v0.6.0):                                                    │
│  ├── spn provider list          List API keys and sources                       │
│  ├── spn provider set <name>    Store key in OS Keychain                        │
│  ├── spn provider get <name>    Get key (masked by default)                     │
│  ├── spn provider delete <name> Remove key from keychain                        │
│  ├── spn provider migrate       Move env vars to keychain                       │
│  └── spn provider test <name>   Validate key format                             │
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

- **Language:** Rust 2021
- **CLI Framework:** clap v4
- **HTTP Client:** reqwest (rustls)
- **Serialization:** serde, serde_yaml, serde_json, toml
- **Security:** keyring (OS keychain), secrecy, zeroize, libc (mlock)
- **Performance:** rustc-hash (FxHashMap)

## Project Structure

```
supernovae-cli/
├── src/
│   ├── main.rs           # Entry point + CLI definition
│   ├── commands/         # CLI subcommands (add, install, sync, provider, etc.)
│   ├── index/            # Registry client + downloader
│   ├── manifest/         # spn.yaml + spn.lock parsing
│   ├── storage/          # Local package storage (~/.spn/packages/)
│   ├── sync/             # IDE config sync (Claude Code, Cursor, VS Code)
│   ├── interop/          # Binary proxies (nika, novanet, npm)
│   ├── secrets/          # Secure credential management (v0.6.0)
│   │   ├── mod.rs        # Module exports
│   │   ├── keyring.rs    # OS keychain integration
│   │   ├── types.rs      # Provider definitions, SecureString
│   │   └── memory.rs     # mlock/LockedBuffer/LockedString
│   └── error.rs          # Error types
├── Cargo.toml
└── CLAUDE.md
```

## Security Architecture (v0.6.0)

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
cargo run -- help
cargo run -- doctor
cargo run -- add @workflows/dev/code-review

# Security (v0.6.0)
cargo run -- provider list              # Show all keys
cargo run -- provider set anthropic     # Store key securely
cargo run -- provider migrate           # Move env vars to keychain
cargo run -- provider test all          # Validate all keys

# Test
cargo test                              # 158 tests

# Lint
cargo clippy

# Install locally
cargo install --path .
```

## Test Stats

- **158 tests passing**
- **Zero clippy errors** (minor style warnings allowed)

## Storage Layout

```
~/.spn/
├── config.toml           # User config
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
| Code Style | `cargo fmt` + `cargo clippy` |
| Testing | TDD preferred |

---

**Distribution:** `brew install supernovae-st/tap/spn`
