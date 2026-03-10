# Master Gap Analysis Report
## spn v0.15.4 — Comprehensive Audit
**Generated:** 2026-03-10
**Agents Used:** 10 parallel exploration agents

---

## Executive Summary

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| CI/CD Pipeline | 1 | 2 | 3 | 2 | 8 |
| spn-core | 0 | 2 | 1 | 1 | 4 |
| spn-keyring | 0 | 4 | 4 | 2 | 10 |
| spn-client | 2 | 4 | 3 | 2 | 11 |
| spn-ollama | 1 | 6 | 4 | 2 | 13 |
| spn-mcp | 6 | 3 | 3 | 7 | 19 |
| spn-cli Commands | 3 | 8 | 12 | 8 | 31 |
| Roadmap/Docs | 0 | 3 | 3 | 2 | 8 |
| **TOTAL** | **13** | **32** | **33** | **26** | **104** |

**Test Status:** 1,288 tests passing | Zero clippy warnings

---

## 1. CI/CD Pipeline Gaps

### Critical

| ID | Issue | File | Impact |
|----|-------|------|--------|
| CI-1 | `spn-providers` missing from release-plz.toml | release-plz.toml | Version drift, manual releases needed |

### High

| ID | Issue | File | Fix |
|----|-------|------|-----|
| CI-2 | MSRV inconsistency (1.88 vs 1.75) | Cargo.toml | Align to 1.88 |
| CI-3 | No code coverage reporting | test.yml | Add cargo-tarpaulin |

### Medium

| ID | Issue | File | Fix |
|----|-------|------|-----|
| CI-4 | Windows not tested | test.yml | Document limitation |
| CI-5 | Dual binary targets warning | spn/Cargo.toml | Resolve or document |
| CI-6 | Publish timeout tight (10m) | release-plz.toml | Increase to 15m |

---

## 2. spn-core Gaps (v0.1.2)

### High

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| CORE-1 | Unimplemented scope extraction in `as_ref()` | registry.rs:235 | Implement TODO |
| CORE-2 | No validation of scoped name format | registry.rs:120 | Add validation |

### Medium

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| CORE-3 | No provider ID uniqueness test | providers.rs | Add test |

### Low

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| CORE-4 | Missing serde feature CI test | All types | Add CI job |

---

## 3. spn-keyring Gaps (v0.1.4)

### High

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| KEY-1 | Missing .env in resolve_key() | lib.rs:460-476 | Add dotenvy support |
| KEY-2 | Docs promise .env not in code | lib.rs:454-455, README | Update docs or implement |
| KEY-3 | DotEnv test but no code | lib.rs:511 | Remove or implement |
| KEY-4 | Incomplete platform documentation | lib.rs:93-97 | Update docs |

### Medium

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| KEY-5 | Inconsistent env_var handling | lib.rs:467 vs keyring.rs:50 | Unify |
| KEY-6 | Async/Sync mismatch undocumented | secrets.rs:107 | Document |
| KEY-7 | No feature flag tests | Cargo.toml:13-17 | Add CI test |
| KEY-8 | HOME env var injection risk | lib.rs:256 | Validate or use dirs crate |

---

## 4. spn-client Gaps (v0.3.3)

### Critical

| ID | Issue | File:Line | Impact |
|----|-------|-----------|--------|
| CLI-1 | Only 6/18 protocol commands implemented | lib.rs | 67% API missing |
| CLI-2 | No streaming response support | lib.rs:421-475 | Model operations blocked |

### High

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| CLI-3 | MODEL_* commands missing (6) | lib.rs | Implement |
| CLI-4 | JOB_* commands missing (5) | lib.rs | Implement |
| CLI-5 | No progress callback mechanism | lib.rs | Add |
| CLI-6 | Error variants incomplete | error.rs | Add model/job errors |

### Medium

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| CLI-7 | Fallback mode inconsistencies | lib.rs | Standardize |
| CLI-8 | Protocol version not enforced | lib.rs:251-258 | Use ProtocolMismatch |
| CLI-9 | Fixed 30s timeout for all ops | lib.rs:108 | Make configurable |

---

## 5. spn-ollama Gaps (v0.1.6)

### Critical

| ID | Issue | File:Line | Impact |
|----|-------|-----------|--------|
| OLL-1 | Retry config unused | client.rs:28-32 | No transient failure recovery |

### High

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| OLL-2 | `.unwrap_or_default()` on error text | client.rs:423,472,542,621,676 | Proper error handling |
| OLL-3 | Unsafe embedding extraction | client.rs:633 | Add validation |
| OLL-4 | No stream chunk timeout | client.rs:549-574 | Add timeout |
| OLL-5 | Only 404 mapped to ModelNotFound | client.rs:221+ | Map more status codes |
| OLL-6 | Child process not tracked | ollama.rs:116-134 | Store PID |
| OLL-7 | Silent callback discarding | ollama.rs:186 | Log at INFO level |

### Medium

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| OLL-8 | gpu_ids always empty | ollama.rs:233 | Document limitation |
| OLL-9 | GPU info stub | ollama.rs:239-242 | Document |
| OLL-10 | No timeout validation | client.rs:34-46 | Add bounds check |
| OLL-11 | Happy path only tests | client.rs | Add error tests |

---

## 6. spn-mcp Gaps (v0.1.4)

### Critical

| ID | Issue | File:Line | Impact |
|----|-------|-----------|--------|
| MCP-1 | Rate limiting dead code | handler.rs:259 | DoS vulnerability |
| MCP-2 | Path params not injected | parser.rs:356 | Broken tools |
| MCP-3 | URL param injection missing | handler.rs:267 | Failed requests |
| MCP-4 | Query params not applied | parser.rs:430 | Ignored params |
| MCP-5 | Header params not supported | parser.rs:121 | Lost data |
| MCP-6 | Request body from OpenAPI lost | handler.rs:300 | Incomplete |

### High

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| MCP-7 | JSON path extraction fragile | handler.rs:667-698 | Improve parser |
| MCP-8 | Tool name collisions | parser.rs:458-482 | Add deduplication |
| MCP-9 | No error classification | handler.rs:314-320 | Add retry logic |

### Medium

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| MCP-10 | Template injection blacklist | handler.rs:346-365 | Use whitelist |
| MCP-11 | Credential fallback incomplete | handler.rs:558-593 | Complete fallback |
| MCP-12 | OpenAPI version too permissive | parser.rs:204-207 | Strict check |

---

## 7. spn-cli Command Gaps

### Critical

| ID | Issue | File:Line | Impact |
|----|-------|-----------|--------|
| CMD-1 | panic!() in test | secrets.rs:790 | CI failure |
| CMD-2 | unreachable!() for Team scope | config.rs:441 | Dead code |
| CMD-3 | Unconditional unwrap | setup.rs:1476 | Potential crash |

### High (Unsafe Patterns)

| ID | Issue | File:Line | Count |
|----|-------|-----------|-------|
| CMD-4 | .unwrap() in setup paths | setup.rs:2027,2042,2056 | 3 |
| CMD-5 | .unwrap() in JSON schema | setup.rs:2101-2109 | 3 |
| CMD-6 | .unwrap() in MCP parsing | mcp.rs:230,1314,1359 | 3 |
| CMD-7 | Silent return Ok(()) | setup.rs:158,743 | 2 |
| CMD-8 | Silent return Ok(()) | mcp.rs:91,173 | 2 |

### Medium (Missing Validation)

| ID | Issue | File:Line | Fix |
|----|-------|-----------|-----|
| CMD-9 | No daemon readiness verification | setup.rs:830+ | Add check |
| CMD-10 | No Neo4j readiness check | setup.rs | Add TCP + query check |
| CMD-11 | filter().unwrap() antipattern | jobs.rs:471 | Use nth() or ok_or() |

---

## 8. Roadmap & Documentation Gaps

### High

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| DOC-1 | CHANGELOG [Unreleased] empty | CHANGELOG.md | Populate with v0.16 |
| DOC-2 | 5 crates not created for roadmap | Workspace | Create scaffolds |
| DOC-3 | 26 v0.16 TODOs in code | Various | Create issues |

### Medium

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| DOC-4 | spn-providers not in ROADMAP | ROADMAP.md | Add or clarify |
| DOC-5 | No performance baselines | - | Establish benchmarks |
| DOC-6 | Tight timeline (8 months to v1.0) | ROADMAP.md | Review scope |

---

## Priority Fix Order

### P0 — Must Fix Before Next Release

1. **CI-1**: Add spn-providers to release-plz.toml
2. **CMD-1**: Replace panic!() with proper assertion
3. **MCP-1**: Wire rate limiting to handler
4. **CMD-3**: Replace .parse().unwrap() with .expect() or ?

### P1 — Fix This Sprint

5. **OLL-1**: Implement retry logic
6. **MCP-2,3,4,5**: Implement parameter injection
7. **KEY-1,2**: Add .env support or update docs
8. **CMD-5**: Replace setup unwraps with ?

### P2 — Fix Next Sprint

9. **CLI-1,2**: Implement missing protocol commands
10. **OLL-2,3**: Improve error handling
11. **MCP-6,7,8**: Complete OpenAPI support
12. **CORE-1,2**: Complete registry validation

### P3 — Backlog

13. All LOW severity items
14. Documentation updates
15. Additional test coverage

---

## Test Verification Commands

```bash
# Current state
cargo test --workspace
cargo clippy --workspace -- -D warnings

# After fixes
cargo test --workspace --all-features
cargo test -p spn-core --features serde
cargo build --no-default-features
cargo build --no-default-features --features docker
```

---

## Next Steps

1. Create fix plans for each category (P0/P1)
2. Run cargo test to establish baseline
3. Execute fixes with TDD (test first, then fix)
4. Push and verify CI passes
5. Update CHANGELOG [Unreleased] section
