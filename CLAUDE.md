# supernovae-cli

**SuperNovae CLI (`spn`)** — Unified package manager for the SuperNovae AI workflow ecosystem.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn — SuperNovae Package Manager                                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Commands:                                                                      │
│  ├── spn pkg install <package>    Install from registry                         │
│  ├── spn nk <args>                Proxy to nika CLI                             │
│  ├── spn nv <args>                Proxy to novanet CLI                          │
│  ├── spn mcp <args>               Manage MCP servers                            │
│  ├── spn doctor                   Verify installation health                    │
│  └── spn help                     Show help                                     │
│                                                                                 │
│  Ecosystem:                                                                     │
│  ├── Nika       — YAML workflow engine (body)                                   │
│  ├── NovaNet    — Knowledge graph (brain)                                       │
│  └── Registry   — Package distribution                                          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Tech Stack

- **Language:** Rust
- **CLI Framework:** clap v4
- **HTTP Client:** reqwest
- **Serialization:** serde, serde_yaml, serde_json

## Project Structure

```
supernovae-cli/
├── src/
│   ├── main.rs           # Entry point
│   ├── commands/         # CLI subcommands
│   │   ├── pkg.rs        # Package management
│   │   ├── proxy.rs      # nk/nv proxy commands
│   │   ├── mcp.rs        # MCP server management
│   │   └── doctor.rs     # Health checks
│   └── lib.rs            # Library exports
├── .spn/                 # Local config (gitignored)
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

# Test
cargo test

# Install locally
cargo install --path .
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
| Commits | `type(scope): description` |
| Code Style | `cargo fmt` + `cargo clippy` |
| Testing | TDD preferred |

---

**Distribution:** `brew install supernovae-st/tap/spn`
