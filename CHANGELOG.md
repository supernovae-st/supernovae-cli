# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **daemon**: MCP auto-sync Phases 1-5
  - `RecentProjects` tracker for project-level file watching
  - `ForeignTracker` for detecting MCPs added directly to editors
  - `McpDiff` for comparing spn and client MCP configs
  - `WatcherService` for file system monitoring with debounce
  - `NotificationService` for native desktop notifications
  - Watcher integrated into daemon event loop (Phase 5)
- **status**: Client sync status tracking for MCP servers
  - New `ClientSyncStatus` type with per-client sync state
  - Visual sync indicators (● synced, ○ pending, ⊘ disabled)
  - Server emojis for visual identification
- **mcp**: New `spn mcp adopt` command to adopt foreign MCPs
- **mcp**: New `spn mcp status` command for detailed MCP status

### Fixed

- **mcp**: Preserve env vars when adopting foreign MCPs
- **daemon**: Fix TOCTOU race condition in `mark_our_write`

## [0.15.2](https://github.com/supernovae-st/supernovae-cli/releases/tag/0.15.2) - 2026-03-08

### Fixed

- **docker**: Bundle CA certificates with webpki-roots


## [0.15.1](https://github.com/supernovae-st/supernovae-cli/releases/tag/0.15.1) - 2026-03-08

### Added

- **cli**: Add lazy install prompt to spn nv command
- **cli**: Add lazy install prompt to spn nk command
- **cli**: Add ecosystem status to setup wizard
- **cli**: Add ecosystem tool detection module
- **cli**: Add --from-openapi flag to mcp wrap command
- **mcp**: Add wrap wizard for REST-to-MCP tool generation
- **cli**: Add unified backup system
- **spn-mcp**: Add OpenAPI 3.0 parser module

### Changed

- Apply cargo fmt

### Documentation

- Fix env vars, versions, and add Code of Conduct
- Complete v0.15.0 release preparation

### Fixed

- **deps**: Replace atty with std::io::IsTerminal
- **security**: Harden backup system against multiple vulnerabilities
- **test**: Avoid clippy approx_constant warning

### Security

- **mcp**: Remove unnecessary borrow in hint_line call

### Security

- **mcp**: Add payload size and parameter type validation


## [0.15.0](https://github.com/supernovae-st/supernovae-cli/releases/tag/v0.15.0) - 2026-03-08

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🚀 SPN v0.15.0 — THE AGENTIC AI TOOLKIT                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🤖 Agents  │  📋 Jobs  │  🧠 Memory  │  🔮 Autonomy  │  🛠️ spn-mcp          ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🤖 Agent Delegation** | ✅ New | Multi-agent task delegation system |
| **📋 Job Scheduler** | ✅ New | Background workflow execution |
| **🧠 Cross-Session Memory** | ✅ New | Persistent context across sessions |
| **🔮 Autonomy Orchestration** | ✅ New | Self-directed task execution |
| **🛠️ spn-mcp Crate** | ✅ New | Dynamic REST-to-MCP wrapper |
| **🖥️ Windows CI** | ✅ New | Full Windows support in CI |

### Added

- **daemon**: Wire JobScheduler to IPC handler and enhance Nika setup
- **ux**: Add TransformingSpinner to model commands
- **model**: Add pull_with_progress method to ModelManager
- **protocol**: Add ModelProgress type for streaming updates
- **spn-mcp**: Add rate limiting and MCP Resources support
- **spn-mcp**: Add test infrastructure and security hardening
- **daemon**: Add autonomy orchestration system (Phase 13)
- **daemon**: Add proactive suggestion system (Phase 12)
- **daemon**: Add agent delegation system (Phase 11)
- **daemon**: Add reasoning trace capture system (Phase 10)
- **daemon**: Add cross-session memory system (Phase 9)
- **suggest**: Enhance smart wizard with interactive mode (Phase 8)
- **jobs**: Add jobs CLI commands (Phase 7)
- **daemon**: Add job scheduler for background workflows
- **daemon**: Add MCP server over stdio
- **suggest**: Add context-aware suggestion wizard
- **tui**: Add spn explore interactive TUI
- **spn**: Integrate spn-mcp into main CLI
- **spn-mcp**: Add dynamic REST-to-MCP wrapper crate
- **mcp**: Add `spn mcp logs` command for viewing server logs
- **completion**: Add install/uninstall/status subcommands
- **model**: Add `spn model run` command for LLM inference
- **status**: Add unified status dashboard with ASCII rendering

### Changed

- **ci**: Add Windows to test matrix
- **release**: Add spn-mcp to release-plz.toml
- **docs**: Update all version references to v0.15.0

### Documentation

- **plan**: Complete master plan v0.15-v0.18
- **adr**: Add ecosystem architecture ADRs (001-003)
- **plan**: Add spn v0.15-v0.18 evolution roadmap

### Fixed

- **setup**: Correct Windsurf settings.json path detection
- **daemon**: Add missing autonomy type exports
- **spn-mcp**: Critical error handling and security hardening
- **spn**: Remove phantom explore/tui module references

### 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  6-CRATE WORKSPACE (v0.15.0)                                                    │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn-core (0.1.1)     Core types, providers, validation                        │
│  spn-keyring (0.1.3)  OS keychain integration                                  │
│  spn-ollama (0.1.4)   Ollama model backend                                     │
│  spn-client (0.3.0)   Daemon SDK for external tools                            │
│  spn-mcp (0.1.0)      Dynamic REST-to-MCP wrapper ← NEW                        │
│  spn-cli (0.15.0)     Main CLI binary                                          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## [0.14.3](https://github.com/supernovae-st/supernovae-cli/releases/tag/0.14.3) - 2026-03-07

╔═══════════════════════════════════════════════════════════════════════════════╗
║  ⚡ SPN v0.14.3 — POLISH & PERFORMANCE                                        ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🐚 Completions  │  📊 Protocol  │  ⚡ FxHashMap  │  📦 Dependencies           ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🐚 Shell Completions** | ✅ New | `install`, `uninstall`, `status` subcommands |
| **📊 IPC Protocol** | ✅ Enhanced | Version negotiation for daemon communication |
| **⚡ FxHashMap** | ✅ Perf | Faster hashing (~20% in hot paths) |
| **📦 Dependencies** | ✅ Updated | reqwest 0.13, indicatif 0.18 |

### Added

- **cli**: Add shell completion subcommands (`install`, `uninstall`, `status`)
- **cli**: Add verbose logging support
- **ipc**: Add protocol versioning for daemon communication

### Changed

- Format FxHashMap collection patterns
- Simplify conditional patterns

### Fixed

- **daemon**: Improve async correctness

### Performance

- Use FxHashMap for faster hashing
- **tokio**: Use minimal features for smaller binary

### Dependencies

- Update reqwest 0.12→0.13, indicatif 0.17→0.18, reqwest-retry 0.6→0.9


## [0.14.2](https://github.com/supernovae-st/supernovae-cli/releases/tag/0.14.2) - 2026-03-07

### 🎨 UX Enhancements

| Feature | Description |
|---------|-------------|
| **Human-Readable Formatters** | File sizes, durations, counts displayed nicely |
| **Enhanced Help** | Improved `--help` output with examples |

### Added

- **ux**: Add human-readable formatters (file sizes, durations, counts)
- **ux**: Enhanced `--help` output with usage examples


## [0.14.1](https://github.com/supernovae-st/supernovae-cli/releases/tag/0.14.1) - 2026-03-07

### 🔧 Housekeeping Release

Minor release focused on ecosystem alignment and publishing automation.

### Changed

- **release**: Enable automated crates.io publishing
- **docs**: Align version references across all crates

### Fixed

- **cli**: Align `spn nk config` with nika CLI subcommands


## [0.14.0](https://github.com/supernovae-st/supernovae-cli/releases/tag/0.14.0) - 2026-03-07

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🎨 SPN v0.14.0 — THE DELIGHT RELEASE                                         ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🎨 Design System  │  🦙 Streaming  │  🔧 Error Handling  │  ⚡ Performance   ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🎨 Semantic Design System** | ✅ New | Unified UX with consistent theming |
| **🦙 Streaming Chat** | ✅ New | `chat_stream()` in spn-ollama |
| **🔧 Error Handling** | ✅ Fixed | All CLI commands use `Result` types |
| **⏱️ Configurable Timeouts** | ✅ New | `ClientConfig` with retry logic |

### Added

- **ux**: Add semantic design system with unified theming
- **ux**: Add comprehensive UX module for consistent CLI experience
- **cli**: Complete Phase 2-5 UX improvements
- **🦙 spn-ollama**: Add `chat_stream()` method for streaming chat completions
- **🦙 spn-ollama**: Add `BoxedTokenCallback` type for trait object compatibility
- **🦙 spn-ollama**: Add `DynModelBackend::chat_stream()` for runtime polymorphism
- **⏱️ spn-ollama**: Add `ClientConfig` with configurable timeouts (connect, request, model)
- **🔄 spn-ollama**: Add retry logic infrastructure (`with_retry()`, `is_retryable()`)

### Fixed

- **cli**: Replace all `exit(1)` calls with proper `SpnError::CommandFailed` (18 occurrences)
  - provider.rs (11 calls)
  - mcp.rs (4 calls)
  - skill.rs (3 calls)
- **lint**: Resolve all clippy warnings (`io_other_error`, `uninlined_format_args`)
- **cli**: Resolve 3 bugs found in e2e testing

### Technical

- Error handling now returns `Result<(), SpnError>` consistently across all CLI commands
- Improved error messages with proper context and suggestions

## [0.12.5] - 2026-03-06

### Changed

- **📦 Dependencies**: Update all dependencies to latest versions

## [0.12.4] - 2026-03-06

### Fixed

- **🐳 Docker**: Disable spn-keyring default features for musl builds

## [0.12.3] - 2026-03-06

### Changed

- **📦 Release**: Bump spn-keyring to v0.1.2, spn-core to v0.1.1
- **📦 Release**: Bump spn-client to v0.2.3 (add SpnPaths export)

## [0.12.2] - 2026-03-05

### Changed

- **🐳 Docker**: Static musl builds for minimal `scratch` image (~5MB)
- **🔧 Feature flags**: `os-keychain` feature for conditional keychain support
- **📦 Build matrix**: Separate musl targets for Docker, gnu for native releases

### Technical

- Added `--no-default-features --features docker` for container builds
- `spn-keyring` now has `os-keychain` feature (default enabled)
- Keyring operations gracefully return `Locked` when feature disabled
- Automatic fallback to environment variables in Docker

## [0.12.1] - 2026-03-05

### Fixed

- **🐳 Docker**: Fixed missing `libdbus` in container (switched from distroless to debian-slim)
- Binary now runs correctly in Docker environment

## [0.12.0] - 2026-03-05

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🐳 SPN v0.12.0 — DOCKER DISTRIBUTION                                         ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🐳 Docker  │  ghcr.io  │  Multi-arch  │  SLSA Provenance                     ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🐳 Docker Images** | ✅ New | `ghcr.io/supernovae-st/spn` |
| **🏗️ Multi-arch** | ✅ New | linux/amd64 + linux/arm64 |
| **🔐 SLSA Provenance** | ✅ New | Supply chain security |
| **📦 SBOM** | ✅ New | Software Bill of Materials |

### 🐳 Docker Usage

```bash
# Run directly
docker run --rm ghcr.io/supernovae-st/spn:latest --version

# With project mount
docker run --rm -v $(pwd):/workspace ghcr.io/supernovae-st/spn:latest list

# With API keys
docker run --rm \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  ghcr.io/supernovae-st/spn:latest provider test anthropic
```

### 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DOCKER DISTRIBUTION PIPELINE                                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  build job (existing)                                                           │
│  ├── aarch64-unknown-linux-gnu ──┐                                             │
│  └── x86_64-unknown-linux-gnu  ──┼── docker-publish job (new)                  │
│                                  │   ├── Extract binaries                      │
│                                  │   ├── Build multi-arch image                │
│                                  │   ├── Push to ghcr.io                       │
│                                  │   └── Generate attestations                 │
│                                  │                                              │
│  Tags: :latest, :0.12.0, :0.12, :0, :sha-XXXXXX                                │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 🔧 Technical Details

- **Base Image**: `gcr.io/distroless/cc-debian12:nonroot` (~18MB total)
- **Platforms**: `linux/amd64`, `linux/arm64`
- **Security**: Non-root user, SLSA provenance, SBOM
- **Registry**: `ghcr.io/supernovae-st/spn`

### ⚠️ Limitations

| Feature | Docker | Native |
|---------|--------|--------|
| OS Keychain | ❌ Use env vars | ✅ Full support |
| Daemon socket | ⚠️ Volume mount | ✅ Direct |
| Ollama | ⚠️ Network/sidecar | ✅ Direct |

### 📦 Distribution Channels

| Channel | Command |
|---------|---------|
| **Homebrew** | `brew install supernovae-st/tap/spn` |
| **Cargo** | `cargo install spn-cli` |
| **Docker** | `docker pull ghcr.io/supernovae-st/spn:latest` |
| **Binaries** | GitHub Releases |

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
  - `spn-keyring` (v0.1.1): OS keychain integration (macOS/Windows/Linux)
  - `spn-ollama` (v0.1.0): Ollama backend with `ModelBackend` trait
  - `spn-client` (v0.2.2): SDK for external tool integration
  - `spn-cli` (v0.12.2): Main CLI binary

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

[Unreleased]: https://github.com/supernovae-st/supernovae-cli/compare/v0.15.0...HEAD
[0.15.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.14.3...v0.15.0
[0.14.3]: https://github.com/supernovae-st/supernovae-cli/compare/v0.14.2...v0.14.3
[0.14.2]: https://github.com/supernovae-st/supernovae-cli/compare/v0.14.1...v0.14.2
[0.14.1]: https://github.com/supernovae-st/supernovae-cli/compare/v0.14.0...v0.14.1
[0.14.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.12.5...v0.14.0
[0.12.5]: https://github.com/supernovae-st/supernovae-cli/compare/v0.12.4...v0.12.5
[0.12.4]: https://github.com/supernovae-st/supernovae-cli/compare/v0.12.3...v0.12.4
[0.12.3]: https://github.com/supernovae-st/supernovae-cli/compare/v0.12.2...v0.12.3
[0.12.2]: https://github.com/supernovae-st/supernovae-cli/compare/v0.12.1...v0.12.2
[0.12.1]: https://github.com/supernovae-st/supernovae-cli/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/supernovae-st/supernovae-cli/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/supernovae-st/supernovae-cli/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/supernovae-st/supernovae-cli/releases/tag/v0.7.0
