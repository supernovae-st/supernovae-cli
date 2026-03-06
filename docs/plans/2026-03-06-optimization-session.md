# Optimization Session Plan

**Created:** 2026-03-06
**Status:** 🔄 In Progress
**Target:** spn-cli v0.13.0

## Executive Summary

Deep codebase analysis identified 4 major improvement areas across 6 dimensions.
This plan executes all actionable items in priority order.

---

## Phase 1: Quick Wins (Dead Code + Clippy)

**Estimated:** 30 minutes | **Commits:** 5-8

### 1.1 Remove Deprecated Functions

| File | Line | Function | Action |
|------|------|----------|--------|
| `spn-client/src/lib.rs` | 122-133 | `default_socket_path()` | DELETE |
| `spn/src/storage/local.rs` | 241 | `package_path()` | DELETE |

### 1.2 Remove Unused Exports

| File | Line | Export | Action |
|------|------|--------|--------|
| `spn/src/interop/mod.rs` | 19-20 | `ModelRegistry`, `ModelPackage` | REMOVE allow + exports |

### 1.3 Fix Critical Clippy Issues

| Lint | File | Line | Fix |
|------|------|------|-----|
| `redundant_clone` | `sync/config.rs` | 109 | Remove .clone() |
| `needless_collect` | `commands/provider.rs` | 168 | Use .any() |
| `unused_async` | 51 functions | various | Remove async keyword |

### 1.4 Add Workspace Clippy Config

Add to `Cargo.toml`:
```toml
[workspace.lints.clippy]
uninlined_format_args = "allow"
doc_markdown = "allow"
use_self = "allow"
missing_const_for_fn = "allow"
```

---

## Phase 2: Performance Optimizations

**Estimated:** 2 hours | **Commits:** 3-5

### 2.1 Arc Cache in Index Client (HIGH IMPACT)

**Problem:** Every cache hit clones entire `Vec<IndexEntry>`
**Solution:** Use `Arc<Vec<IndexEntry>>` for zero-copy cache hits

```rust
// Before
cache: DashMap<String, Vec<IndexEntry>>

// After
cache: DashMap<String, Arc<Vec<IndexEntry>>>
```

**Files:** `crates/spn/src/index/client.rs`

### 2.2 State File Caching (HIGH IMPACT)

**Problem:** Every install/uninstall reads+writes state.json
**Solution:** Cache state in memory, flush on demand

**Files:** `crates/spn/src/storage/local.rs`

### 2.3 Compact Serialization

**Problem:** `to_string_pretty()` for internal files
**Solution:** Use compact JSON for state.json

---

## Phase 3: Test Coverage (Critical Paths)

**Estimated:** 4-6 hours | **Commits:** 6-10

### 3.1 Daemon Secrets Tests (HIGH PRIORITY)

**File:** `crates/spn/src/daemon/secrets.rs` (273 LoC, 0 tests)

| Test | Description |
|------|-------------|
| `test_preload_all_success` | Load multiple providers from keychain |
| `test_preload_keyring_error` | Handle keychain unavailable |
| `test_get_cached_hit` | Cache hit returns SecretString |
| `test_get_cached_miss` | Cache miss returns None |
| `test_has_cached` | Boolean cache lookup |

### 3.2 Daemon Server Tests (CRITICAL)

**File:** `crates/spn/src/daemon/server.rs` (449 LoC, 2 tests)

| Test | Description |
|------|-------------|
| `test_pid_file_locking` | Concurrent daemon start prevention |
| `test_socket_permissions` | Verify 0600 permissions |
| `test_stale_pid_cleanup` | Detect and remove stale PID |

### 3.3 Index Downloader Tests

**File:** `crates/spn/src/index/downloader.rs`

| Test | Description |
|------|-------------|
| `test_checksum_mismatch_rejected` | Security: wrong hash fails |
| `test_checksum_invalid_format` | Malformed hash handled |
| `test_retry_on_network_error` | Retry middleware works |

### 3.4 Index Client Tests

**File:** `crates/spn/src/index/client.rs`

| Test | Description |
|------|-------------|
| `test_ndjson_parsing` | Parse multi-line NDJSON |
| `test_cache_hit_performance` | Verify no clone on hit |
| `test_fetch_latest_semver` | Correct version selection |

---

## Phase 4: Dependency Cleanup

**Estimated:** 30 minutes | **Commits:** 2-3

### 4.1 Remove Unused Dependencies

| Crate | Package | Action |
|-------|---------|--------|
| `clap_complete` | spn-cli | Remove if shell completions unused |

### 4.2 Fix Path Dependencies

Ensure all internal crates use workspace path dependencies consistently.

### 4.3 Document Deprecations

Add note about serde_yaml → serde_yml migration for v0.14.

---

## Execution Checklist

### Phase 1: Quick Wins
- [ ] Remove `default_socket_path()` from spn-client
- [ ] Remove `package_path()` from storage/local.rs
- [ ] Remove unused model_registry exports
- [ ] Fix `redundant_clone` in sync/config.rs
- [ ] Fix `needless_collect` in commands/provider.rs
- [ ] Remove `unused_async` from 51 functions
- [ ] Add workspace clippy config
- [ ] Commit: "refactor: remove deprecated and dead code"
- [ ] Commit: "fix: resolve clippy warnings"

### Phase 2: Performance
- [ ] Implement Arc cache in index/client.rs
- [ ] Add state caching to LocalStorage
- [ ] Switch to compact serialization
- [ ] Commit: "perf: use Arc for zero-copy cache hits"
- [ ] Commit: "perf: cache state file in memory"

### Phase 3: Tests
- [ ] Add daemon/secrets.rs tests
- [ ] Add daemon/server.rs tests
- [ ] Add index/downloader.rs security tests
- [ ] Add index/client.rs parsing tests
- [ ] Commit: "test(daemon): add secrets manager tests"
- [ ] Commit: "test(daemon): add server lifecycle tests"
- [ ] Commit: "test(index): add security and parsing tests"

### Phase 4: Dependencies
- [ ] Audit clap_complete usage
- [ ] Fix internal path dependencies
- [ ] Commit: "chore(deps): cleanup unused dependencies"

---

## Success Criteria

| Metric | Before | Target |
|--------|--------|--------|
| Clippy warnings | 770 | <50 |
| Dead code markers | 47 | <40 |
| Test count | 457 | 500+ |
| Cache clone overhead | 5-15ms | 0ms |

---

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| Remove deprecated API | LOW | Unused, no external consumers |
| Arc cache refactor | LOW | Internal change, same API |
| Clippy fixes | LOW | Mechanical changes |
| New tests | NONE | Only adds coverage |

