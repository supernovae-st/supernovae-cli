# Plan: Security Hardening & Rate Limiting

**Created**: 2026-03-08
**Status**: Ready for execution
**Effort**: ~3 hours
**Target**: v0.16.0
**Priority**: HIGH (security)

---

## Problem

From gap analysis (`docs/plans/2026-03-08-spn-mcp-gap-analysis.md`):

1. **Rate Limiting**: Config exists in YAML but NOT enforced at runtime
2. **Template Injection**: User input passed to Tera templates without sanitization
3. **Parameter Validation**: Type mismatches silently accepted
4. **Error Handling**: Silent swallowing in handler.rs:181

---

## Issues by Severity

### CRITICAL (Fix Immediately)

| Issue | File | Line | Risk |
|-------|------|------|------|
| Silent error swallowing | handler.rs | 181 | Requests fail silently |
| Template injection | handler.rs | ~150 | Code execution via `{{` |

### HIGH (Fix This Release)

| Issue | File | Risk |
|-------|------|------|
| Rate limiting not enforced | config.rs | API abuse possible |
| Parameter type bypass | handler.rs | Invalid data to APIs |

### MEDIUM (Fix Soon)

| Issue | File | Risk |
|-------|------|------|
| Missing input length limits | handler.rs | DoS via large payloads |
| No request timeout per-endpoint | client.rs | Hung requests |

---

## Implementation

### Step 1: Fix Silent Error Swallowing

**File:** `crates/spn-mcp/src/handler.rs` line ~181

```rust
// BEFORE (silent swallow)
match self.client.execute(request).await {
    Ok(response) => { /* ... */ }
    Err(_) => { /* silent */ }
}

// AFTER (proper error propagation)
match self.client.execute(request).await {
    Ok(response) => { /* ... */ }
    Err(e) => {
        tracing::error!(error = %e, "API request failed");
        return Err(McpError::ApiError {
            status: None,
            message: format!("Request failed: {}", e),
        });
    }
}
```

### Step 2: Template Injection Prevention

**File:** `crates/spn-mcp/src/handler.rs`

```rust
use tera::escape_html;

impl Handler {
    fn render_template(&self, template: &str, context: &Context) -> Result<String, Error> {
        // Sanitize all user-provided values before template rendering
        let mut safe_context = Context::new();

        for (key, value) in context.iter() {
            let safe_value = match value {
                Value::String(s) => {
                    // Escape template syntax AND HTML
                    let escaped = escape_html(s);
                    let escaped = escaped
                        .replace("{{", "&#123;&#123;")
                        .replace("}}", "&#125;&#125;")
                        .replace("{%", "&#123;%")
                        .replace("%}", "%&#125;");
                    Value::String(escaped)
                }
                other => other.clone(),
            };
            safe_context.insert(key, &safe_value);
        }

        self.tera.render(template, &safe_context)
    }
}
```

**Alternative: Disable Tera auto-escaping for controlled contexts**

```rust
// In config, mark which parameters are "trusted"
parameters:
  - name: owner
    type: string
    trusted: true  # From MCP, already validated
  - name: query
    type: string
    trusted: false  # User input, must escape
```

### Step 3: Enforce Rate Limiting

**File:** `crates/spn-mcp/src/rate_limiter.rs` (NEW)

```rust
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct ApiRateLimiter {
    limiter: Arc<RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>,
}

impl ApiRateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        Self {
            limiter: Arc::new(RateLimiter::keyed(quota)),
        }
    }

    pub async fn check(&self, api_name: &str) -> Result<(), RateLimitError> {
        self.limiter
            .check_key(&api_name.to_string())
            .map_err(|_| RateLimitError::TooManyRequests {
                api: api_name.to_string(),
                retry_after_secs: 60,
            })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for {api}. Retry after {retry_after_secs}s")]
    TooManyRequests {
        api: String,
        retry_after_secs: u64,
    },
}
```

**File:** `crates/spn-mcp/src/handler.rs`

```rust
impl Handler {
    pub async fn handle_tool_call(&self, name: &str, args: Value) -> Result<Value, McpError> {
        // ENFORCE rate limiting
        if let Some(rate_limit) = &self.config.rate_limit {
            self.rate_limiter
                .check(&self.config.name)
                .await
                .map_err(|e| McpError::RateLimited(e.to_string()))?;
        }

        // ... rest of handler
    }
}
```

### Step 4: Parameter Type Validation

**File:** `crates/spn-mcp/src/validation.rs` (NEW)

```rust
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum ParamType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
}

impl ParamType {
    pub fn validate(&self, value: &Value) -> Result<(), ValidationError> {
        match (self, value) {
            (ParamType::String, Value::String(_)) => Ok(()),
            (ParamType::Integer, Value::Number(n)) if n.is_i64() => Ok(()),
            (ParamType::Number, Value::Number(_)) => Ok(()),
            (ParamType::Boolean, Value::Bool(_)) => Ok(()),
            (ParamType::Array, Value::Array(_)) => Ok(()),
            (ParamType::Object, Value::Object(_)) => Ok(()),
            _ => Err(ValidationError::TypeMismatch {
                expected: format!("{:?}", self),
                got: value_type_name(value),
            }),
        }
    }
}

pub fn validate_parameters(
    params: &[ParamConfig],
    args: &serde_json::Map<String, Value>,
) -> Result<(), ValidationError> {
    for param in params {
        if let Some(value) = args.get(&param.name) {
            param.param_type.validate(value)?;

            // Length validation for strings
            if let Value::String(s) = value {
                if let Some(max_len) = param.max_length {
                    if s.len() > max_len {
                        return Err(ValidationError::TooLong {
                            param: param.name.clone(),
                            max: max_len,
                            got: s.len(),
                        });
                    }
                }
            }
        } else if param.required {
            return Err(ValidationError::MissingRequired(param.name.clone()));
        }
    }
    Ok(())
}
```

### Step 5: Input Length Limits

**File:** `crates/spn-mcp/src/config.rs`

```yaml
# Add to parameter schema
parameters:
  - name: query
    type: string
    required: true
    max_length: 10000  # NEW: Prevent DoS
  - name: body
    type: object
    max_size_bytes: 1048576  # NEW: 1MB limit
```

**File:** `crates/spn-mcp/src/handler.rs`

```rust
// Add size check before processing
fn check_payload_size(&self, args: &Value) -> Result<(), McpError> {
    let size = serde_json::to_string(args)
        .map(|s| s.len())
        .unwrap_or(0);

    const MAX_PAYLOAD: usize = 10 * 1024 * 1024; // 10MB

    if size > MAX_PAYLOAD {
        return Err(McpError::PayloadTooLarge {
            max: MAX_PAYLOAD,
            got: size,
        });
    }
    Ok(())
}
```

### Step 6: Request Timeout per Endpoint

**File:** `crates/spn-mcp/src/config.rs`

```yaml
endpoints:
  - name: slow_endpoint
    method: POST
    path: /process
    timeout_secs: 300  # NEW: 5 minute timeout
```

**File:** `crates/spn-mcp/src/client.rs`

```rust
impl ApiClient {
    pub async fn execute_with_timeout(
        &self,
        request: Request,
        timeout: Option<Duration>,
    ) -> Result<Response, Error> {
        let timeout = timeout.unwrap_or(Duration::from_secs(30));

        tokio::time::timeout(timeout, self.client.execute(request))
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(Error::Request)
    }
}
```

---

## Tests

### Security Tests

```rust
#[test]
fn test_template_injection_blocked() {
    let input = "{{ system('rm -rf /') }}";
    let result = render_template("Hello {{name}}", json!({"name": input}));
    assert!(!result.contains("system"));
    assert!(result.contains("&#123;"));
}

#[test]
fn test_rate_limiter_enforced() {
    let limiter = ApiRateLimiter::new(2); // 2 per minute

    assert!(limiter.check("test").await.is_ok());
    assert!(limiter.check("test").await.is_ok());
    assert!(limiter.check("test").await.is_err()); // Third should fail
}

#[test]
fn test_parameter_validation_strict() {
    let params = vec![ParamConfig {
        name: "count".into(),
        param_type: ParamType::Integer,
        required: true,
        ..Default::default()
    }];

    let args = json!({"count": "not-an-integer"});
    assert!(validate_parameters(&params, args.as_object().unwrap()).is_err());
}

#[test]
fn test_payload_size_limit() {
    let huge = "x".repeat(20 * 1024 * 1024); // 20MB
    let result = check_payload_size(&json!({"data": huge}));
    assert!(matches!(result, Err(McpError::PayloadTooLarge { .. })));
}
```

---

## Verification Checklist

- [ ] Silent errors now logged with `tracing::error`
- [ ] Template syntax in user input is escaped
- [ ] Rate limiting blocks requests over quota
- [ ] Type mismatches return validation errors
- [ ] Large payloads rejected before processing
- [ ] Endpoint timeouts respected
- [ ] All security tests pass

---

## Commit Strategy

```bash
# Commit 1: Fix error handling
git commit -m "fix(spn-mcp): propagate API errors instead of swallowing

SECURITY: Silent error swallowing could hide attack attempts

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

# Commit 2: Template injection prevention
git commit -m "security(spn-mcp): prevent template injection attacks

- Escape {{ and }} in user-provided values
- Escape Tera control flow syntax
- Add test coverage for injection attempts

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

# Commit 3: Rate limiting enforcement
git commit -m "feat(spn-mcp): enforce rate limiting at runtime

- Add ApiRateLimiter using governor crate
- Check rate limit before each API call
- Return 429-equivalent error when exceeded

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

# Commit 4: Parameter validation
git commit -m "feat(spn-mcp): add strict parameter type validation

- Validate types match schema before API call
- Enforce max_length on string params
- Enforce max_size_bytes on payloads

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Security Advisory

Consider publishing a security advisory after this fix:

```markdown
## Security Advisory: spn-mcp < 0.1.1

### Affected Versions
- spn-mcp 0.1.0

### Vulnerabilities Fixed

1. **Template Injection (MEDIUM)**: User input containing Tera template
   syntax was rendered, potentially allowing information disclosure.

2. **Rate Limiting Bypass (LOW)**: Rate limit configuration was not
   enforced, allowing unlimited API requests.

### Upgrade Path
Upgrade to spn-mcp >= 0.1.1 or spn-cli >= 0.16.0.
```
