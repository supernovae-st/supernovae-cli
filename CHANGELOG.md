# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.0] - 2026-03-05

### Added

- **Workspace restructuring**: Split into 5 independent crates for better modularity
  - `spn-core` (v0.1.0): Shared types, provider definitions, validation
  - `spn-keyring` (v0.1.0): OS keychain integration (macOS/Windows/Linux)
  - `spn-ollama` (v0.1.0): Ollama backend with `ModelBackend` trait
  - `spn-client` (v0.2.1): SDK for external tool integration
  - `spn-cli` (v0.10.0): Main CLI binary
- **Daemon infrastructure**: Background service for credential caching
  - Unix socket IPC with peer credential verification
  - PID file locking with `flock()` for single-instance guarantee
  - Graceful shutdown with `JoinSet` task tracking
- **Model management**: `ModelManager` for local model lifecycle
  - Pull, load, unload, delete operations
  - Running model status tracking
  - `DynModelBackend` trait for runtime polymorphism
- **crates.io publication**: All core crates published and available

### Changed

- Renamed main crate from `spn` to `spn-cli` for crates.io compatibility
- `spn-client` now re-exports all `spn-core` types
- Internal path dependencies converted to version dependencies for publishing

### Fixed

- **Security**: 3 MEDIUM issues in secrets handling
  - Added `tracing::warn` when `mlock()` fails (previously silent)
  - Fixed intermediate String zeroization in `get_secret()`
  - Documented IPC security model for Response::Secret
- **Daemon**: 3 CRITICAL async/concurrency issues
  - PID file now holds `flock()` until shutdown
  - `JoinSet` tracks all connection tasks for graceful drain
  - Blocking keychain operations wrapped in `spawn_blocking`
- **Compatibility**: Added Windows `cfg` gates for Unix-specific code

### Security

- Socket permissions set to `0600` (owner-only)
- Peer credential verification via `SO_PEERCRED` (Linux) / `LOCAL_PEERCRED` (macOS)
- Memory protection with `mlock()` and `MADV_DONTDUMP`
- Automatic secret zeroization with `Zeroizing<T>` wrapper

## [0.9.0] - 2026-03-04

### Added

- Initial daemon architecture design
- Secret management foundation (`spn provider` commands)
- Multi-provider support (13 LLM/MCP providers)

### Changed

- Migrated to workspace structure
- Improved error handling with `thiserror`

## [0.8.1] - 2026-03-03

### Fixed

- Zero clippy warnings
- README overhaul with accurate documentation

## [0.8.0] - 2026-03-02

### Added

- IDE sync support (Claude Code, Cursor, VS Code)
- Package installation from registry
- MCP server management

## [0.7.0] - 2026-03-01

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
