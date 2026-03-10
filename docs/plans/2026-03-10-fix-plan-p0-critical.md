# P0 Critical Fix Plan
## Must Fix Before Next Release
**Generated:** 2026-03-10
**Target:** v0.15.5

---

## Overview

4 critical issues that must be fixed immediately:

| ID | Issue | Severity | File | Time Est |
|----|-------|----------|------|----------|
| CI-1 | spn-providers missing from release-plz | CRITICAL | release-plz.toml | 5 min |
| CMD-1 | panic!() in test code | CRITICAL | secrets.rs:790 | 5 min |
| MCP-1 | Rate limiting dead code | CRITICAL | handler.rs:259 | 30 min |
| CMD-3 | Unconditional .parse().unwrap() | HIGH | setup.rs:1476 | 5 min |

**Total Estimated Time:** 45 minutes

---

## Fix 1: Add spn-providers to release-plz.toml

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/release-plz.toml`

### Problem
The 7th crate `spn-providers` is not listed in release-plz.toml, causing:
- No automatic versioning
- No CHANGELOG generation
- Manual release required

### Fix
Add after line 89:

```toml
[[package]]
name = "spn-providers"
git_release_enable = false
git_tag_enable = false
changelog_path = "crates/spn-providers/CHANGELOG.md"
changelog_update = true
```

### Test
```bash
release-plz update --dry-run
```

---

## Fix 2: Replace panic!() with proper assertion

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn/src/commands/secrets.rs:790`

### Problem
```rust
panic!("Unexpected result type")
```

Test uses `panic!()` instead of assertion macros, causing unclear CI failures.

### Fix
Replace with:
```rust
unreachable!("Unexpected result type - expected MemoryProtection variant")
```

Or better:
```rust
#[allow(clippy::match_wildcard_for_single_variants)]
_ => unreachable!("Test only expects MemoryProtection result variant"),
```

### Test
```bash
cargo test -p spn-cli test_check_memory_protection
```

---

## Fix 3: Wire rate limiting to handler

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn-mcp/src/server/handler.rs`

### Problem
Rate limiter is created (line 71-79) and stored in `DynamicHandler.rate_limiters`, but:
- Only logs warning at line 259-261
- Never actually blocks requests

### Current Code (line 259-261)
```rust
if let Some(limiter) = self.rate_limiters.get(&api_name) {
    if rate_limit::check_limit(limiter, &api_name).is_err() {
        tracing::warn!("Rate limit exceeded for API: {}", api_name);
        // WARNING ONLY - doesn't block!
    }
}
```

### Fix
Replace with:
```rust
if let Some(limiter) = self.rate_limiters.get(&api_name) {
    rate_limit::check_limit(limiter, &api_name).map_err(|e| {
        tracing::warn!("Rate limit exceeded for API: {}", api_name);
        Error::Mcp(format!("Rate limit exceeded: {}", e))
    })?;
}
```

### Test
```bash
# Add test in handler_tests.rs
cargo test -p spn-mcp test_rate_limit_blocks_requests
```

### Test Code to Add
```rust
#[tokio::test]
async fn test_rate_limit_blocks_requests() {
    // Create handler with strict rate limit (1 request/minute)
    let config = RateLimitConfig {
        requests_per_second: 1,
        burst_size: 1,
    };
    // ... test that second request is blocked
}
```

---

## Fix 4: Replace unconditional unwrap

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn/src/commands/setup.rs:1476`

### Problem
```rust
&"127.0.0.1:7687".parse().unwrap()
```

Hardcoded string parsing that should never fail, but unwrap() is still bad practice.

### Fix
Replace with:
```rust
&"127.0.0.1:7687".parse().expect("static address should always parse")
```

Or use const:
```rust
use std::net::SocketAddr;

const NEO4J_DEFAULT_ADDR: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    7687,
);

// Then use:
&NEO4J_DEFAULT_ADDR
```

### Test
```bash
cargo test -p spn-cli check_neo4j_connection
cargo clippy -p spn-cli -- -D warnings
```

---

## Execution Order

```bash
# 1. Create branch
git checkout -b fix/p0-critical-issues

# 2. Fix CI-1 (release-plz.toml)
# Edit release-plz.toml

# 3. Fix CMD-1 (panic in test)
# Edit crates/spn/src/commands/secrets.rs:790

# 4. Fix CMD-3 (unwrap)
# Edit crates/spn/src/commands/setup.rs:1476

# 5. Fix MCP-1 (rate limiting)
# Edit crates/spn-mcp/src/server/handler.rs:259-261

# 6. Run all tests
cargo test --workspace

# 7. Run clippy
cargo clippy --workspace -- -D warnings

# 8. Commit each fix separately
git add release-plz.toml && git commit -m "fix(release): add spn-providers to release-plz.toml"
git add crates/spn/src/commands/secrets.rs && git commit -m "fix(test): replace panic with unreachable in secrets test"
git add crates/spn/src/commands/setup.rs && git commit -m "fix(setup): use expect instead of unwrap for static address"
git add crates/spn-mcp/src/server/handler.rs && git commit -m "fix(mcp): enforce rate limiting instead of warning only"

# 9. Push
git push -u origin fix/p0-critical-issues
```

---

## Verification Checklist

- [ ] `cargo test --workspace` passes (1,288+ tests)
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `release-plz update --dry-run` includes spn-providers
- [ ] Rate limiting actually blocks requests in test
- [ ] No panic!() in non-test code

---

## Success Criteria

1. All 4 fixes committed separately (atomic commits)
2. CI passes on all platforms
3. No regression in existing functionality
4. Rate limiting is now enforced, not just warned
