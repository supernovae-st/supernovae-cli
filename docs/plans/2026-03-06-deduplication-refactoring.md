# Deduplication & Test Coverage Refactoring Plan

**Created:** 2026-03-06
**Completed:** 2026-03-06
**Status:** ✅ Complete
**Target:** spn-cli v0.13.0

## Execution Summary

| Phase | Status | Commits | Lines Changed |
|-------|--------|---------|---------------|
| Phase 1: SpnPaths | ✅ Complete | 6 | +419 (paths.rs) |
| Phase 2: IDE Adapters | ✅ Complete | 2 | +280/-69 (config_loader) |
| Phase 3: Test Coverage | ✅ Complete | 1 | +172 (10 tests now) |
| **Total** | **✅ Complete** | **9** | **+800 net** |

### Key Achievements

- **SpnPaths** now in spn-client (12 unit tests)
- **config_loader** extracted (10 unit tests)
- **downloader.rs** coverage: 4 → 10 tests
- Zero `dirs::home_dir().join(".spn")` scattered calls

---

## Executive Summary

Deep codebase analysis identified three major improvement areas:
1. **Path patterns** — 17 scattered path constructions need centralization ✅
2. **IDE adapters** — 683 lines of parallel code need merging ✅
3. **Test coverage** — Critical paths (downloader) need comprehensive tests ✅

---

## Phase 1: SpnPaths Abstraction

### Problem

Path construction is scattered across 9+ files with `dirs::home_dir().join(".spn")`:

| Location | Pattern | Purpose |
|----------|---------|---------|
| `spn-client/src/lib.rs:105` | `home.join(".spn").join("daemon.sock")` | Socket path |
| `daemon/mod.rs:73` | `home.join(".spn").join("daemon.pid")` | PID file |
| `daemon/mod.rs:86` | `home.join(".spn")` | Root dir |
| `config/global.rs:14` | `home.join(".spn").join("config.toml")` | Global config |
| `secrets/storage.rs:157` | `home.join(".spn").join("secrets.env")` | Secrets file |
| `storage/local.rs:110` | `home.join(".spn")` | Storage root |
| `interop/binary.rs:89` | `home.join(".spn").join("bin")` | Binary dir |

### Solution

Create `spn-core/src/paths.rs`:

```rust
/// Centralized path management for ~/.spn directory structure.
pub struct SpnPaths {
    root: PathBuf,
}

impl SpnPaths {
    /// Create paths rooted at ~/.spn
    pub fn new() -> Result<Self, PathError> {
        let home = dirs::home_dir()
            .ok_or(PathError::HomeNotFound)?;
        Ok(Self { root: home.join(".spn") })
    }

    /// Create with custom root (for testing)
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    // Directory paths
    pub fn root(&self) -> &Path { &self.root }
    pub fn bin_dir(&self) -> PathBuf { self.root.join("bin") }
    pub fn packages_dir(&self) -> PathBuf { self.root.join("packages") }
    pub fn cache_dir(&self) -> PathBuf { self.root.join("cache") }
    pub fn registry_dir(&self) -> PathBuf { self.root.join("registry") }

    // File paths
    pub fn config_file(&self) -> PathBuf { self.root.join("config.toml") }
    pub fn secrets_file(&self) -> PathBuf { self.root.join("secrets.env") }
    pub fn socket_file(&self) -> PathBuf { self.root.join("daemon.sock") }
    pub fn pid_file(&self) -> PathBuf { self.root.join("daemon.pid") }
    pub fn state_file(&self) -> PathBuf { self.root.join("state.json") }

    // Ensure directories exist
    pub fn ensure_dirs(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(self.bin_dir())?;
        std::fs::create_dir_all(self.packages_dir())?;
        std::fs::create_dir_all(self.cache_dir())?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("HOME directory not found. Set HOME environment variable.")]
    HomeNotFound,
}
```

### Migration Strategy

1. Add `SpnPaths` to `spn-core` (new file)
2. Re-export from `spn-client` for external consumers
3. Update each module one-by-one:
   - `daemon/mod.rs` — use `SpnPaths` for socket/pid
   - `config/global.rs` — use `SpnPaths::config_file()`
   - `secrets/storage.rs` — use `SpnPaths::secrets_file()`
   - `storage/local.rs` — use `SpnPaths` for root
   - `interop/binary.rs` — use `SpnPaths::bin_dir()`
4. Remove scattered `dirs::home_dir()` calls
5. Add tests for all path methods

### Commits

- [ ] `feat(core): add SpnPaths centralized path management`
- [ ] `refactor(daemon): use SpnPaths for socket and pid paths`
- [ ] `refactor(config): use SpnPaths for config file path`
- [ ] `refactor(secrets): use SpnPaths for secrets file path`
- [ ] `refactor(storage): use SpnPaths for storage root`
- [ ] `refactor(interop): use SpnPaths for binary directory`
- [ ] `test(core): add comprehensive SpnPaths tests`

---

## Phase 2: IDE Adapter Deduplication

### Problem

Two parallel implementations exist:
1. `sync/adapters.rs` (343 lines) — trait-based, used by `spn sync`
2. `sync/mcp_sync.rs` (683 lines) — function-based, TODO(v0.14)

Duplication within adapters.rs:
- ClaudeCode JSON loading: 17 lines
- Cursor JSON loading: 17 lines (99% identical)
- MCP insertion logic: 7 lines duplicated

### Solution

#### Step 1: Extract common JSON loading

```rust
// sync/config_loader.rs (new file)

/// Load JSON config with graceful fallback.
pub fn load_json_config(
    path: &Path,
    default_key: Option<&str>,
) -> Result<Value, ConfigError> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadFailed(path.to_path_buf(), e))?;
        Ok(serde_json::from_str(&content).unwrap_or_else(|_| {
            default_key.map_or(json!({}), |k| json!({ k: {} }))
        }))
    } else {
        Ok(default_key.map_or(json!({}), |k| json!({ k: {} })))
    }
}

/// Insert MCP servers into config.
pub fn insert_mcp_servers(
    config: &mut Value,
    servers: impl IntoIterator<Item = (String, Value)>,
) {
    let mcp_servers = config
        .as_object_mut()
        .and_then(|obj| obj.entry("mcpServers").or_insert(json!({})).as_object_mut());

    if let Some(servers_obj) = mcp_servers {
        for (name, value) in servers {
            servers_obj.insert(name, value);
        }
    }
}

/// Write JSON config with pretty formatting.
pub fn write_json_config(path: &Path, value: &Value) -> Result<(), ConfigError> {
    let content = serde_json::to_string_pretty(value)
        .map_err(ConfigError::SerializeFailed)?;
    std::fs::write(path, content)
        .map_err(|e| ConfigError::WriteFailed(path.to_path_buf(), e))
}
```

#### Step 2: Simplify adapters

```rust
// In ClaudeCodeAdapter::sync_package
fn sync_package(&self, ...) -> SyncResult {
    let config_path = self.config_path(project_root);

    let mut settings = load_json_config(&config_path, None)
        .map_err(|e| /* return SyncResult with error */)?;

    // Build MCP servers from manifest
    let servers = manifest.mcp.iter().map(|mcp| {
        (mcp_server_name(package_name), build_mcp_config(package_path, mcp))
    });

    insert_mcp_servers(&mut settings, servers);

    // Sync skills/hooks (Claude Code only)
    self.sync_skills(project_root, package_name, package_path, manifest)?;
    self.sync_hooks(project_root, package_name, package_path, manifest)?;

    write_json_config(&config_path, &settings)?;

    SyncResult { success: true, ... }
}
```

#### Step 3: Deprecate mcp_sync.rs

Mark `mcp_sync.rs` functions as deprecated, redirect to adapter trait:

```rust
#[deprecated(since = "0.13.0", note = "Use IdeAdapter trait instead")]
pub fn sync_to_claude_code(...) { ... }
```

### Commits

- [ ] `refactor(sync): extract config_loader module for JSON operations`
- [ ] `refactor(sync): simplify ClaudeCodeAdapter using config_loader`
- [ ] `refactor(sync): simplify CursorAdapter using config_loader`
- [ ] `refactor(sync): deprecate mcp_sync.rs parallel implementation`
- [ ] `test(sync): add config_loader unit tests`

---

## Phase 3: Test Coverage for Critical Paths

### Problem

`index/downloader.rs` (488 lines) has only 4 tests:
- `test_download_and_verify` — happy path
- `test_checksum_mismatch` — security check
- `test_extract_tarball` — extraction
- `test_cache_reuse` — caching

Missing critical tests:
- HTTP error handling (404, 500, timeout)
- Retry logic verification
- Corrupted tarball handling
- Progress callback behavior
- Concurrent downloads

### Solution

Add comprehensive tests in `index/downloader.rs`:

```rust
#[cfg(test)]
mod tests {
    // Existing tests...

    // === HTTP Error Tests ===

    #[tokio::test]
    async fn test_download_404_error() {
        let temp = TempDir::new().unwrap();
        let config = RegistryConfig::local(&temp.path().join("empty"), &temp.path());
        let downloader = PackageDownloader::new(config);

        let result = downloader.download("nonexistent", "1.0.0").await;

        assert!(matches!(result, Err(DownloadError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_download_timeout() {
        // Use non-routable IP (RFC 5737 TEST-NET-1)
        let config = RegistryConfig::new("http://192.0.2.1:12345")
            .with_timeout(Duration::from_millis(500));
        let downloader = PackageDownloader::new(config);

        let start = Instant::now();
        let result = downloader.download("test", "1.0.0").await;
        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(elapsed < Duration::from_secs(2));
    }

    // === Corruption Tests ===

    #[tokio::test]
    async fn test_corrupted_tarball() {
        let temp = TempDir::new().unwrap();
        let releases = temp.path().join("releases");
        std::fs::create_dir_all(&releases).unwrap();

        // Write invalid tar.gz
        std::fs::write(
            releases.join("test-1.0.0.tar.gz"),
            b"not a valid tarball"
        ).unwrap();

        let config = RegistryConfig::local(&temp.path(), &releases);
        let downloader = PackageDownloader::new(config);

        let result = downloader.download_and_extract("test", "1.0.0", &temp.path()).await;

        assert!(matches!(result, Err(DownloadError::ExtractionFailed { .. })));
    }

    // === Progress Callback Tests ===

    #[tokio::test]
    async fn test_progress_callback_invoked() {
        let temp = TempDir::new().unwrap();
        let (index, releases) = setup_test_registry(&temp);

        let config = RegistryConfig::local(&index, &releases);
        let downloader = PackageDownloader::new(config);

        let progress_count = Arc::new(AtomicUsize::new(0));
        let counter = progress_count.clone();

        let _result = downloader
            .download_with_progress("test", "1.0.0", move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        assert!(progress_count.load(Ordering::SeqCst) > 0);
    }

    // === Retry Logic Tests ===

    #[tokio::test]
    async fn test_retry_on_transient_failure() {
        // Test that retries happen on 503 Service Unavailable
        // Requires mock server or test double
    }
}
```

### Additional Test Files

Create `tests/install_workflow.rs` for E2E tests:

```rust
//! End-to-end installation workflow tests.
//!
//! These tests verify the complete flow: add → resolve → download → install → sync

#[tokio::test]
#[ignore = "requires network"]
async fn test_full_install_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("test-project");
    std::fs::create_dir_all(&project).unwrap();

    // Initialize project
    std::fs::write(
        project.join("spn.yaml"),
        "name: test-project\nversion: 0.1.0\n"
    ).unwrap();

    // Add package (simulated)
    // ... test add command

    // Install
    // ... test install command

    // Verify lockfile created
    assert!(project.join("spn.lock").exists());

    // Verify package installed
    // ... check ~/.spn/packages/
}
```

### Commits

- [ ] `test(downloader): add HTTP error handling tests`
- [ ] `test(downloader): add corrupted tarball handling tests`
- [ ] `test(downloader): add progress callback tests`
- [ ] `test(downloader): add retry logic tests`
- [ ] `test: add install workflow integration tests`

---

## Phase 4: Clean Up TODOs and Dead Code

### Problem

- 35 TODO comments targeting v0.14
- 47 `#[allow(dead_code)]` markers
- 3 blocking issues in config commands

### Solution

#### Step 1: Audit and categorize

| Category | Count | Action |
|----------|-------|--------|
| v0.14 scaffolding | 28 | Keep with clear doc |
| Obsolete stubs | 5 | Remove |
| Blocking issues | 3 | Fix or document |

#### Step 2: Fix blocking issues

1. **Config key path resolution** — Wire `SpnPaths` to config commands
2. **Config get/set** — Implement using new abstractions
3. **Sync command integration** — Wire adapter trait to CLI

#### Step 3: Document remaining TODOs

Add `docs/roadmap/v0.14-integration.md` documenting all deferred work.

### Commits

- [ ] `fix(config): wire SpnPaths to config commands`
- [ ] `refactor: remove obsolete dead code stubs`
- [ ] `docs: add v0.14 integration roadmap`

---

## Verification Checklist

### Phase 1 Complete When:
- [ ] `SpnPaths` struct in spn-core with full test coverage
- [ ] Zero direct `dirs::home_dir().join(".spn")` in codebase
- [ ] All path methods have documentation
- [ ] `cargo test` passes

### Phase 2 Complete When:
- [ ] `config_loader` module extracted and tested
- [ ] ClaudeCode/Cursor adapters use shared code
- [ ] `mcp_sync.rs` functions marked deprecated
- [ ] IDE sync still works (`spn sync` command)

### Phase 3 Complete When:
- [ ] `downloader.rs` has 10+ tests covering all error paths
- [ ] At least 1 E2E workflow test exists
- [ ] `cargo test` shows increased coverage

### Phase 4 Complete When:
- [ ] Config commands functional
- [ ] Obsolete dead code removed
- [ ] v0.14 roadmap documented

---

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| SpnPaths refactor | Medium | Incremental migration, tests first |
| Adapter merge | Low | Feature parity tests |
| New tests | Low | Only adds coverage |
| Dead code removal | Low | Careful audit |

---

## Timeline

All phases in single session with granular commits.

**Estimated Changes:**
- ~20 files modified
- ~400 lines added (mostly tests)
- ~200 lines removed (duplication)
- 15+ new tests
