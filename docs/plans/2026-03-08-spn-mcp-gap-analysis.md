# spn-mcp Gap Analysis Plan

**Date:** 2026-03-08
**Status:** Draft
**Analyzed by:** 10 parallel sniper agents

---

## Executive Summary

Analysis of spn-mcp crate identified **100+ issues** across 6 categories. This plan prioritizes fixes by severity and provides implementation guidance.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ISSUE BREAKDOWN                                                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  🔴 CRITICAL (1)     Production error handling - silent failure                 │
│  🟠 HIGH (8)         Security + MCP protocol gaps                               │
│  🟡 MEDIUM (15)      Config validation + refactoring                            │
│  🟢 LOW (10+)        Dead code + test coverage                                  │
│                                                                                 │
│  Total: ~100+ issues across error handling, security, protocol, config,         │
│         dead code, and test coverage                                            │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Critical Production Fix (Immediate)

### 1.1 Silent Error Swallowing in handler.rs:181

**File:** `crates/spn-mcp/src/server/handler.rs`
**Line:** 181
**Severity:** 🔴 CRITICAL

```rust
// CURRENT (BAD)
let body = response.text().await.unwrap_or_default();

// FIX
let body = response.text().await.unwrap_or_else(|e| {
    tracing::warn!("Failed to read error response body: {}", e);
    String::new()
});
```

**Impact:** Production errors are silently discarded, making debugging impossible.

---

## Phase 2: Security Hardening (HIGH)

### 2.1 Template Injection Prevention

**File:** `crates/spn-mcp/src/server/handler.rs`
**Function:** `render_template()`

**Issue:** User-provided parameters are passed directly to Tera templates without sanitization.

**Fix:**
```rust
fn render_template(&self, template: &str, params: &Value) -> Result<String> {
    // Validate template doesn't contain dangerous directives
    if template.contains("{% include") || template.contains("{% import") {
        return Err(Error::ConfigValidation(
            "Template contains forbidden directives".into()
        ));
    }

    // Existing render logic...
}
```

### 2.2 Parameter Validation Bypass

**Issue:** Tool parameters aren't validated against schema before execution.

**Fix:** Add pre-execution validation in `execute_tool()`:
```rust
// Before building request
self.validate_params(&entry.tool_def, &params)?;
```

### 2.3 Rate Limiting Not Enforced

**Issue:** `rate_limit` config exists but is never used.

**Fix:** Either implement or remove (recommend: defer to Phase 3 as MEDIUM).

---

## Phase 3: MCP Protocol Completion (HIGH/MEDIUM)

### 3.1 Missing ServerHandler Methods

| Feature | Priority | Implementation |
|---------|----------|----------------|
| Resources | HIGH | Expose API configs as resources |
| Prompts | HIGH | Template prompt support |
| Sampling | MEDIUM | LLM integration (defer) |
| Pagination | MEDIUM | Already stubbed, needs real impl |
| Progress | LOW | Streaming progress for long requests |
| Cancellation | LOW | Request cancellation support |

### 3.2 Recommended Immediate Implementation

```rust
// Add to DynamicHandler impl
fn list_resources(&self, ...) -> ... {
    // Expose each API config as a resource
    let resources: Vec<Resource> = self.api_configs
        .iter()
        .map(|c| Resource::new(
            format!("api://{}", c.name),
            c.description.clone().unwrap_or_default()
        ))
        .collect();
    Ok(ListResourcesResult::with_all_items(resources))
}
```

---

## Phase 4: Config Schema Validation (MEDIUM)

### 4.1 Validation Gaps to Fix

| Gap | File | Fix |
|-----|------|-----|
| ApiKey auth requires key_name | schema.rs | Add validation in AuthConfig::validate() |
| rate_limit bounds | schema.rs | requests_per_minute: 1-1000, burst: 1-100 |
| array items required | schema.rs | Validate items field when type=array |
| HTTP method whitelist | schema.rs | Only allow GET/POST/PUT/DELETE/PATCH |
| body_template syntax | loader.rs | Parse template during load, not at runtime |

### 4.2 Add JsonSchema Derivation

```rust
use schemars::JsonSchema;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ApiConfig {
    // ...
}
```

This enables `spn mcp apis schema <name>` command for config authoring.

---

## Phase 5: Dead Code Removal (LOW)

### 5.1 Unused Error Variants

**File:** `crates/spn-mcp/src/error.rs`

```rust
// REMOVE these unused variants:
ToolNotFound(String),      // Never constructed
MissingParam(String),      // Never constructed
RateLimited { ... },       // Never constructed (rate limit not implemented)
```

### 5.2 Unused ResponseConfig.transform Field

**File:** `crates/spn-mcp/src/config/schema.rs`

```rust
pub struct ResponseConfig {
    pub extract: Option<String>,
    pub transform: Option<String>,  // REMOVE - never used
}
```

### 5.3 CLI Dead MCP Code

Check `crates/spn/src/commands/mcp/` for any stubs from Phase 1 that are now redundant.

---

## Phase 6: Test Coverage (LOW)

### 6.1 Missing Handler Tests

| Function | Priority | Test Type |
|----------|----------|-----------|
| DynamicHandler::new() | HIGH | Unit (mock configs) |
| execute_tool() | HIGH | Integration (mock HTTP) |
| resolve_credential() | HIGH | Unit (mock daemon/env) |
| render_template() | MEDIUM | Unit (edge cases) |
| extract_json_path() | LOW | Already has tests |

### 6.2 Missing Config Tests

- Invalid YAML parsing errors
- Missing required fields
- Type mismatches in params
- Circular dependencies (if any)

### 6.3 Test Infrastructure Needed

```rust
// test_utils.rs
pub fn mock_api_config(name: &str) -> ApiConfig { ... }
pub fn mock_http_server() -> mockito::Server { ... }
```

---

## Phase 7: Refactoring (LOW)

### 7.1 Long Function Breakdown

**execute_tool() - 72 lines** → Split into:
- `build_request()` - URL + method
- `apply_auth_and_headers()` - Auth + headers
- `execute_and_parse()` - Send + parse response

### 7.2 Code Duplication

Extract common patterns:
- Error response formatting
- JSON path extraction logic (could be a separate module)

---

## Implementation Order

```
Week 1: Phase 1 (Critical) + Phase 2.1-2.2 (Security)
Week 2: Phase 4 (Config validation)
Week 3: Phase 3.1-3.2 (MCP Resources/Prompts)
Week 4: Phase 5 (Dead code) + Phase 6 (Tests)
Backlog: Phase 7 (Refactoring), Phase 3 remainder
```

---

## Verification Checklist

- [ ] Line 181 error handling fixed
- [ ] Template injection prevented
- [ ] Parameter validation added
- [ ] Config validation complete
- [ ] Dead code removed
- [ ] 80%+ test coverage
- [ ] All clippy warnings resolved
- [ ] `cargo test --workspace` passes

---

## Related Files

| File | Changes |
|------|---------|
| `crates/spn-mcp/src/server/handler.rs` | Phase 1, 2, 3, 7 |
| `crates/spn-mcp/src/config/schema.rs` | Phase 4, 5 |
| `crates/spn-mcp/src/config/loader.rs` | Phase 4 |
| `crates/spn-mcp/src/error.rs` | Phase 5 |
| `crates/spn-mcp/src/server/mod.rs` | Phase 6 (tests) |
