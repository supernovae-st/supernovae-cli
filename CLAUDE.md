# supernovae-cli

**SuperNovae CLI (`spn`)** v0.2.1 — Unified package manager for the SuperNovae AI workflow ecosystem.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn — SuperNovae Package Manager v0.2.1                                        │
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
- **HTTP Client:** reqwest (blocking)
- **Serialization:** serde, serde_yaml, serde_json, toml
- **Checksum:** sha2

## Project Structure

```
supernovae-cli/
├── src/
│   ├── main.rs           # Entry point + CLI definition
│   ├── commands/         # CLI subcommands (add, install, sync, etc.)
│   ├── index/            # Registry client + downloader
│   ├── manifest/         # spn.yaml + spn.lock parsing
│   ├── storage/          # Local package storage (~/.spn/packages/)
│   ├── sync/             # IDE config sync (Claude Code, Cursor, VS Code)
│   ├── interop/          # Binary proxies (nika, novanet, npm)
│   └── error.rs          # Error types
├── Cargo.toml
└── CLAUDE.md
```

## Commands

```bash
# Build
cargo build --release

# Run
cargo run -- help
cargo run -- doctor
cargo run -- add @workflows/dev/code-review

# Test
cargo test                 # 87 tests

# Lint
cargo clippy

# Install locally
cargo install --path .
```

## Test Stats

- **87 tests passing**
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
