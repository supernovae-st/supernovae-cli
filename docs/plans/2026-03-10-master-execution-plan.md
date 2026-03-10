# Master Execution Plan v0.16.x
## Consolidated from 15-Agent Deep Analysis

**Generated:** 2026-03-10
**Status:** EXECUTING
**Target:** v0.15.4 → v0.16.0

---

## Executive Summary

15 specialized agents analyzed the entire codebase:

| Agent | Focus | Key Finding |
|-------|-------|-------------|
| rust-pro | Code quality | 6 critical patterns need fix |
| rust-async-expert | Async/Tokio | spawn_blocking needed for blocking I/O |
| rust-security | Security | 2 CVEs, 3 unsafe patterns |
| rust-perf | Performance | Binary 11MB, 30% reduction possible |
| rust-architect | Architecture | Score 6.5/10, trait consolidation needed |
| code-reviewer | Quality | Good patterns, minor improvements |
| code-explorer | Execution paths | Well-structured, clear flows |
| TODO/FIXME finder | Tech debt | 31 TODO(v0.16) need conversion |
| Test coverage | Coverage | 71% estimated, 1,550 passing |
| Plan reviewer | Portfolio | 60+ plans, good categorization |
| web-researcher | Best practices | Aligned with 2026 Rust patterns |
| cargo-audit | Dependencies | 2 vulnerabilities (HIGH, MEDIUM) |
| binary-size | Size | 11MB release, 30% reducible |
| test-timing | Performance | 1,550 tests in ~45s |
| CI/CD review | Pipeline | Score 8.2/10, production ready |

---

## Phase 0: IMMEDIATE (Today)

### 0.1 Security Vulnerabilities ⚠️ CRITICAL

```bash
# Fix quinn-proto (HIGH) and time (MEDIUM) CVEs
cargo update
cargo audit  # Verify clean
```

**Impact:** 2 vulnerabilities → 0

### 0.2 Already Completed P0 Fixes ✅

- [x] Added spn-providers to release-plz.toml
- [x] Replaced panic!() with unreachable!() in secrets.rs
- [x] Replaced .parse().unwrap() with .expect() in setup.rs
- [x] Removed stale dead_code comments from rate_limit.rs

---

## Phase 1: Sprint (This Week)

### 1.1 OLL-1: Add Retry Logic with Backoff

**File:** `crates/spn-ollama/src/client.rs`

```rust
use tokio::time::{sleep, Duration};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 100;

async fn with_retry<F, T, E>(operation: F) -> Result<T, E>
where
    F: Fn() -> impl Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut backoff = Duration::from_millis(INITIAL_BACKOFF_MS);

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < MAX_RETRIES => {
                tracing::warn!(
                    "Retry {}/{} after error: {}",
                    attempts + 1,
                    MAX_RETRIES,
                    e
                );
                sleep(backoff).await;
                backoff *= 2; // Exponential backoff
                attempts += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 1.2 MCP-2: Parameter Injection Safety

**File:** `crates/spn-mcp/src/server/handler.rs`

```rust
fn validate_parameter_name(name: &str) -> Result<(), Error> {
    // Allow only alphanumeric, underscore, hyphen
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(Error::Mcp(format!(
            "Invalid parameter name '{}': only alphanumeric, underscore, hyphen allowed",
            name
        )));
    }
    if name.len() > 64 {
        return Err(Error::Mcp("Parameter name exceeds 64 characters".into()));
    }
    Ok(())
}
```

### 1.3 KEY-2: .env File Support in spn-keyring

**File:** `crates/spn-keyring/src/lib.rs`

```rust
pub fn resolve_key(provider_id: &str) -> Option<Zeroizing<String>> {
    // 1. Try OS keychain first
    if let Ok(key) = SpnKeyring::get(provider_id) {
        return Some(key);
    }

    // 2. Try environment variable
    let env_var = format!("{}_API_KEY", provider_id.to_uppercase());
    if let Ok(value) = std::env::var(&env_var) {
        return Some(Zeroizing::new(value));
    }

    // 3. Try .env file via dotenvy
    dotenvy::dotenv().ok();
    if let Ok(value) = std::env::var(&env_var) {
        return Some(Zeroizing::new(value));
    }

    None
}
```

### 1.4 CMD-5 to CMD-8: Replace setup.rs unwraps

**Target unwraps at lines:** 282, 285, 288, 1000

Replace each `.unwrap()` with proper error handling using `?` operator or `.ok_or_else()`.

---

## Phase 2: Architecture Improvements (v0.16.0)

### 2.1 Trait Consolidation

Create unified error trait across crates:

```rust
// crates/spn-core/src/error.rs
pub trait SpnError: std::error::Error + Send + Sync + 'static {
    fn error_code(&self) -> &'static str;
    fn is_retryable(&self) -> bool;
    fn user_message(&self) -> String;
}
```

### 2.2 Binary Size Optimization

**File:** `Cargo.toml` (workspace root)

```toml
[profile.release]
lto = "thin"           # Link-Time Optimization
codegen-units = 1      # Single codegen unit
panic = "abort"        # No unwinding
strip = true           # Strip symbols

[profile.release-small]
inherits = "release"
lto = "fat"
opt-level = "z"        # Optimize for size
```

**Expected:** 11MB → ~7MB (30% reduction)

### 2.3 Test Coverage in CI

**File:** `.github/workflows/ci.yml`

```yaml
coverage:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: taiki-e/install-action@cargo-tarpaulin
    - run: cargo tarpaulin --workspace --out xml
    - uses: codecov/codecov-action@v4
      with:
        files: cobertura.xml
        fail_ci_if_error: true
```

---

## Phase 3: Tech Debt (v0.16.x)

### 3.1 Convert TODOs to GitHub Issues

31 TODO(v0.16) comments need conversion:

```bash
# Find all TODOs
grep -rn "TODO(v0.16)" crates/ --include="*.rs"

# Create issues via gh CLI
for todo in $(grep -rn "TODO(v0.16)" crates/ --include="*.rs"); do
    file=$(echo $todo | cut -d: -f1)
    line=$(echo $todo | cut -d: -f2)
    msg=$(echo $todo | cut -d: -f3-)
    gh issue create --title "$msg" --body "From $file:$line" --label "tech-debt"
done
```

### 3.2 Supply Chain Security

**File:** `deny.toml`

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-3-Clause"]

[bans]
multiple-versions = "warn"
```

---

## Execution Order

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  EXECUTION TIMELINE                                                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  TODAY (2026-03-10):                                                            │
│  ├── [x] P0 fixes (completed)                                                   │
│  ├── [ ] cargo update (security CVEs)                                           │
│  ├── [ ] Run full test suite                                                    │
│  └── [ ] Commit and push                                                        │
│                                                                                 │
│  THIS WEEK:                                                                     │
│  ├── [ ] OLL-1: Retry logic                                                     │
│  ├── [ ] MCP-2: Parameter validation                                            │
│  ├── [ ] KEY-2: .env support                                                    │
│  └── [ ] CMD-5-8: Setup unwrap fixes                                            │
│                                                                                 │
│  v0.16.0 RELEASE:                                                               │
│  ├── [ ] Trait consolidation                                                    │
│  ├── [ ] Binary size optimization                                               │
│  ├── [ ] CI coverage                                                            │
│  └── [ ] Supply chain security                                                  │
│                                                                                 │
│  ONGOING:                                                                       │
│  └── [ ] TODO → GitHub issues migration                                         │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Success Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Security vulnerabilities | 2 | 0 | 🔴 |
| Test coverage | 71% | 80% | 🟡 |
| Binary size | 11MB | 7MB | 🟡 |
| Architecture score | 6.5/10 | 8/10 | 🟡 |
| CI/CD score | 8.2/10 | 9/10 | 🟢 |
| Tests passing | 1,550 | 1,550+ | 🟢 |

---

## Verification Checklist

After each phase:

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo audit` shows no vulnerabilities
- [ ] `cargo fmt --check` passes
- [ ] CI pipeline green

---

## Related Plans

- [P0 Critical Fixes](2026-03-10-fix-plan-p0-critical.md) ✅ COMPLETED
- [P1 Sprint Fixes](2026-03-10-fix-plan-p1-sprint.md)
- [P2 Backlog](2026-03-10-fix-plan-p2-backlog.md)
- [Master Gap Analysis](2026-03-10-master-gap-analysis.md)
