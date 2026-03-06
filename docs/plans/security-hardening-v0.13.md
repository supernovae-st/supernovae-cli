# Security Hardening Plan v0.13

**Created:** 2026-03-06
**Completed:** 2026-03-06
**Status:** ✅ Complete
**Target:** spn-cli v0.13.0

## Executive Summary

Deep code review identified 10 issues across 3 priority levels. This plan addresses security vulnerabilities, robustness gaps, and code quality improvements.

---

## Phase 1: Security (CRITICAL)

### 1.1 Path Traversal Fix

**File:** `crates/spn/src/storage/local.rs:191`

**Problem:** Direct path joining without validation allows malicious packages to write outside `~/.spn/packages/`.

```rust
// VULNERABLE
let full_path = self.packages_dir.join(&path).join(version);
```

**Attack Vector:**
- Malicious package with path `../../.ssh/authorized_keys`
- Installation writes outside sandbox
- Arbitrary file write vulnerability

**Fix:**
```rust
fn validate_package_path(path: &str) -> Result<(), StorageError> {
    // Reject absolute paths
    if Path::new(path).is_absolute() {
        return Err(StorageError::InvalidPath(format!(
            "Absolute paths not allowed: {}", path
        )));
    }
    // Reject path traversal
    if path.contains("..") {
        return Err(StorageError::InvalidPath(format!(
            "Path traversal not allowed: {}", path
        )));
    }
    // Reject hidden files at root
    if path.starts_with('.') {
        return Err(StorageError::InvalidPath(format!(
            "Hidden paths not allowed: {}", path
        )));
    }
    Ok(())
}
```

**Test Cases:**
- `test_rejects_absolute_path`
- `test_rejects_path_traversal`
- `test_rejects_hidden_root`
- `test_accepts_valid_path`

---

### 1.2 Remove Dead Code Allowance

**File:** `crates/spn/src/main.rs:14`

**Problem:** Global `#![allow(dead_code)]` hides unimplemented features and unused code.

**Steps:**
1. Remove `#![allow(dead_code)]`
2. Run `cargo build 2>&1 | grep "warning: .* is never used"`
3. For each warning:
   - If stub/placeholder → Remove or add `#[allow(dead_code)]` with TODO comment
   - If actually used but compiler can't see → Add `#[allow(dead_code)]` with explanation
   - If genuinely dead → Remove

**Expected Findings:**
- Unused command stubs
- Unused helper functions
- Unused struct fields

---

### 1.3 Fix /tmp Fallback Security

**Files:**
- `crates/spn-client/src/lib.rs:100`
- `crates/spn/src/sync/mcp_sync.rs:299`
- `crates/spn/src/commands/sync.rs:109`
- `crates/spn/src/commands/status.rs:31`

**Problem:** Fallback to `/tmp` or `.` when HOME is missing creates security risks.

```rust
// VULNERABLE - world-writable, predictable path
.unwrap_or_else(|| PathBuf::from("/tmp/spn-daemon.sock"))
```

**Fix:** Return proper errors instead of silent fallbacks.

```rust
fn get_socket_path() -> Result<PathBuf, Error> {
    dirs::home_dir()
        .map(|h| h.join(".spn").join("daemon.sock"))
        .ok_or_else(|| Error::ConfigError(
            "HOME directory not found. Set HOME environment variable.".into()
        ))
}
```

---

## Phase 2: Robustness (HIGH)

### 2.1 Replace expect() with Proper Errors

**Files (7 locations):**
- `crates/spn/src/daemon/service.rs:167,181,258,272,397`
- `crates/spn/src/storage/local.rs:406`
- `crates/spn/src/secrets/storage.rs:302`

**Problem:** `expect()` panics instead of returning errors.

```rust
// PANIC-PRONE
let home = dirs::home_dir().expect("HOME not set");
```

**Fix:**
```rust
// ROBUST
let home = dirs::home_dir()
    .ok_or_else(|| ServiceError::Configuration(
        "HOME directory not found".into()
    ))?;
```

---

### 2.2 Add IPC Timeout

**File:** `crates/spn-client/src/lib.rs`

**Problem:** No timeout on socket operations can cause indefinite hangs.

**Fix:**
```rust
const IPC_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn send_request(&mut self, request: Request) -> Result<Response, Error> {
    tokio::time::timeout(IPC_TIMEOUT, self.send_request_inner(request))
        .await
        .map_err(|_| Error::Timeout("IPC request timed out after 5s".into()))?
}
```

**New Error Variant:**
```rust
#[error("Operation timed out: {0}")]
Timeout(String),
```

---

### 2.3 Feature Flag CI Matrix

**File:** `.github/workflows/test.yml`

**Problem:** Only default features tested. `docker` and `no-default-features` never verified.

**Fix:** Add matrix to CI:
```yaml
strategy:
  matrix:
    features:
      - ""  # default
      - "--no-default-features"
      - "--features docker"
      - "--all-features"
```

---

## Phase 3: Clean-up (MEDIUM)

### 3.1 Deduplicate PROVIDERS

**Files:**
- `crates/spn-core/src/providers.rs` - KNOWN_PROVIDERS (source of truth)
- `crates/spn/src/daemon/secrets.rs:18-32` - PROVIDERS (duplicate)

**Problem:** Provider list maintained in two places.

**Fix:** Remove local `PROVIDERS` constant, use `spn_core::KNOWN_PROVIDERS` directly.

---

### 3.2 Optimize NDJSON Buffer

**File:** `crates/spn-ollama/src/client.rs:306`

**Problem:** String reallocation on every line.

```rust
// INEFFICIENT - allocates new String
buffer = buffer[newline_pos + 1..].to_string();
```

**Fix:**
```rust
// EFFICIENT - reuses allocation
buffer.drain(..=newline_pos);
```

---

### 3.3 Add Real Integration Tests

**New Files:**
- `crates/spn/tests/integration_network.rs`
- `crates/spn-ollama/tests/timeout.rs`

**Tests to Add:**
```rust
#[tokio::test]
#[ignore = "requires network"]
async fn test_github_registry_fetch() {
    let client = IndexClient::new();
    let result = client.fetch_latest("@workflows/data/json-transformer").await;
    assert!(result.is_ok() || matches!(result, Err(IndexError::PackageNotFound(_))));
}

#[tokio::test]
async fn test_ollama_connection_timeout() {
    let config = ClientConfig::new()
        .with_connect_timeout(Duration::from_millis(100));
    let client = OllamaClient::with_config("http://192.0.2.1:11434", config);
    assert!(!client.is_running().await);
}
```

---

## Verification Checklist

### Phase 1 Complete When:
- [x] `cargo build` shows no path traversal warnings
- [x] No `#![allow(dead_code)]` in main.rs
- [x] No `/tmp` or `.` fallbacks in codebase
- [x] All tests pass

### Phase 2 Complete When:
- [x] Zero `expect()` on `dirs::home_dir()`
- [x] IPC operations have 30s configurable timeout
- [x] CI tests all feature combinations
- [x] All tests pass

### Phase 3 Complete When:
- [x] Single PROVIDERS definition (uses spn_client re-exports)
- [x] NDJSON uses `drain()` not `to_string()`
- [x] Network integration tests exist (ignored by default)
- [x] All tests pass

---

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| Path validation | Low | Only rejects malicious paths |
| Remove dead code | Medium | May reveal missing features |
| Error instead of fallback | Medium | Better UX than silent wrong behavior |
| IPC timeout | Low | 5s is generous |
| CI matrix | Low | More testing is good |

---

## Timeline

- **Phase 1:** Immediate (security critical)
- **Phase 2:** Same session
- **Phase 3:** Same session if time permits

**Estimated Changes:**
- ~15 files modified
- ~200 lines added
- ~50 lines removed
- 10+ new tests
