# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.11.0] - 2026-03-05

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🦙 SPN v0.11.0 — MODEL CLI COMMANDS                                          ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🦙 Local LLM  │  6 Commands  │  Ollama Integration  │  VRAM Management       ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🦙 Model CLI Commands** | ✅ New | Full local LLM management via CLI |
| **📋 6 New Commands** | ✅ New | list, pull, load, unload, delete, status |
| **🔧 Daemon IPC** | ✅ Enhanced | Model operations via background daemon |
| **📊 VRAM Monitoring** | ✅ New | Track GPU memory usage per model |

### 🦙 Model Commands

```bash
# List installed models
spn model list [--json] [--running]

# Download a model from Ollama registry
spn model pull <name>           # e.g., llama3.2:1b, mistral:7b

# Load model into GPU/RAM
spn model load <name> [--keep-alive]

# Unload model from memory
spn model unload <name>

# Delete model from disk
spn model delete <name> [-y]

# Show running models and VRAM usage
spn model status [--json]
```

### 🏗️ Architecture

```
spn CLI ──► spn daemon (IPC) ──► spn-ollama ──► Ollama API (localhost:11434)
    │                                               │
    │                                               ▼
    │                                    ┌─────────────────────┐
    │                                    │  Downloaded Models  │
    │                                    │  • llama3.2:1b      │
    │                                    │  • mistral:7b       │
    │                                    │  • codellama:13b    │
    │                                    └─────────────────────┘
    │
    └──► Nika workflows can use: --provider ollama --model llama3.2:1b
```

### 🔧 Technical Details

- **409 LOC** new implementation in `commands/model.rs`
- **2 unit tests** for `format_size()` helper
- **IPC Protocol**: `ModelList`, `ModelPull`, `ModelLoad`, `ModelUnload`, `ModelDelete`, `ModelStatus`
- **spn-client**: `send_request()` now public for advanced usage

### 🐛 Bug Fixes

- **CI**: Fixed formatting issues in model.rs
- **Tests**: Fixed flaky `test_daemon_socket_exists` (no longer assumes daemon state)

## [0.10.0] - 2026-03-05

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🚀 SPN v0.10.0 — MODULAR WORKSPACE                                           ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  📦 5 Crates  │  🔐 Daemon Security  │  🦙 Model Manager  │  📋 crates.io     ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **📦 5 Independent Crates** | ✅ New | Modular architecture for crates.io |
| **🔐 Daemon Infrastructure** | ✅ New | Background credential caching |
| **🦙 Model Management** | ✅ New | Local model lifecycle (Ollama) |
| **📋 crates.io Publication** | ✅ Done | All core crates published |

### 🏗️ Workspace Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  CARGO WORKSPACE — 5 INDEPENDENT CRATES                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                        │
│  │  spn-core    │   │  spn-keyring │   │  spn-ollama  │                        │
│  │   v0.1.0     │   │    v0.1.0    │   │    v0.1.0    │                        │
│  └──────────────┘   └──────────────┘   └──────────────┘                        │
│         │                  │                  │                                 │
│         └──────────────────┼──────────────────┘                                 │
│                            ▼                                                    │
│                   ┌──────────────┐                                              │
│                   │  spn-client  │  ← SDK for external tools                   │
│                   │    v0.2.1    │                                              │
│                   └──────────────┘                                              │
│                            ▼                                                    │
│                   ┌──────────────┐                                              │
│                   │   spn-cli    │  ← Main binary (all commands)               │
│                   │   v0.10.0    │                                              │
│                   └──────────────┘                                              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Added

- **📦 Workspace restructuring**: Split into 5 independent crates for better modularity
  - `spn-core` (v0.1.0): Shared types, provider definitions, validation
  - `spn-keyring` (v0.1.0): OS keychain integration (macOS/Windows/Linux)
  - `spn-ollama` (v0.1.0): Ollama backend with `ModelBackend` trait
  - `spn-client` (v0.2.1): SDK for external tool integration
  - `spn-cli` (v0.10.0): Main CLI binary

- **🔐 Daemon infrastructure**: Background service for credential caching
  - Unix socket IPC with peer credential verification
  - PID file locking with `flock()` for single-instance guarantee
  - Graceful shutdown with `JoinSet` task tracking

- **🦙 Model management**: `ModelManager` for local model lifecycle
  - Pull, load, unload, delete operations
  - Running model status tracking
  - `DynModelBackend` trait for runtime polymorphism

- **📋 crates.io publication**: All core crates published and available

### Changed

- Renamed main crate from `spn` to `spn-cli` for crates.io compatibility
- `spn-client` now re-exports all `spn-core` types
- Internal path dependencies converted to version dependencies for publishing

### ⚠️ Security Fixes

| Issue | Severity | Fix |
|-------|----------|-----|
| Silent `mlock()` failures | 🟡 MEDIUM | Added `tracing::warn` logging |
| String zeroization gap | 🟡 MEDIUM | Fixed in `get_secret()` |
| IPC security model | 🟡 MEDIUM | Documented Response::Secret |

### 🔐 Security Hardening

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DAEMON SECURITY                                                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Socket permissions: 0600 (owner-only)                                          │
│  Peer verification:  SO_PEERCRED (Linux) / LOCAL_PEERCRED (macOS)              │
│  Memory protection:  mlock() + MADV_DONTDUMP                                   │
│  Auto-zeroization:   Zeroizing<T> wrapper                                       │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Fixed

- **🔐 Security**: 3 MEDIUM issues in secrets handling
- **🔧 Daemon**: 3 CRITICAL async/concurrency issues
  - PID file now holds `flock()` until shutdown
  - `JoinSet` tracks all connection tasks for graceful drain
  - Blocking keychain operations wrapped in `spawn_blocking`
- **🖥️ Compatibility**: Added Windows `cfg` gates for Unix-specific code

### 📊 Statistics

```
╭─────────────────────────────────────────────────────────────────────────────────╮
│  📊 v0.10.0 METRICS                                                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  📦 Crates:     5 (spn-core, spn-keyring, spn-ollama, spn-client, spn-cli)     │
│  🧪 Tests:      610 passing                                                     │
│  📏 Clippy:     Zero warnings                                                   │
│  🦀 MSRV:       Rust 1.85+                                                      │
│                                                                                 │
╰─────────────────────────────────────────────────────────────────────────────────╯
```

---

## [0.9.0] - 2026-03-04

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🚀 SPN v0.9.0 — DAEMON FOUNDATION                                            ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🔐 Secrets  │  🌐 13 Providers  │  📦 Workspace  │  🛠️ Error Handling        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### Added

- Initial daemon architecture design
- Secret management foundation (`spn provider` commands)
- Multi-provider support (13 LLM/MCP providers)

### Changed

- Migrated to workspace structure
- Improved error handling with `thiserror`

---

## [0.8.1] - 2026-03-03

### Fixed

- Zero clippy warnings
- README overhaul with accurate documentation

---

## [0.8.0] - 2026-03-02

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🚀 SPN v0.8.0 — IDE SYNC                                                     ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🔄 IDE Sync  │  📦 Registry  │  🔧 MCP Servers  │  📋 Installation           ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### Added

- IDE sync support (Claude Code, Cursor, VS Code)
- Package installation from registry
- MCP server management

---

## [0.7.0] - 2026-03-01

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🚀 SPN v0.7.0 — INITIAL RELEASE                                              ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🎉 First Release  │  📦 Package Manager  │  🔍 Registry  │  ⚡ Skills        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### Added

- Initial release
- Package manager foundation
- Registry client
- Skill management via `skills.sh`

---

[Unreleased]: https://github.com/supernovae-st/supernovae-cli/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/supernovae-st/supernovae-cli/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/supernovae-st/supernovae-cli/releases/tag/v0.7.0
