# P1 Sprint Fix Plan
## Fix This Sprint
**Generated:** 2026-03-10
**Target:** v0.16.0

---

## Overview

8 high-priority issues to fix this sprint:

| ID | Issue | Crate | Time Est |
|----|-------|-------|----------|
| OLL-1 | Retry config unused | spn-ollama | 2h |
| MCP-2,3,4,5 | Parameter injection | spn-mcp | 4h |
| KEY-1,2 | .env support/docs | spn-keyring | 2h |
| CMD-5 | Setup unwraps | spn-cli | 1h |

**Total Estimated Time:** 9 hours

---

## Fix 1: Implement Retry Logic (OLL-1)

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn-ollama/src/client.rs`

### Problem
`ClientConfig` defines retry parameters (lines 28-32) but they're never used:
```rust
pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(500);
```

### Fix Approach

1. Create retry wrapper function:
```rust
async fn with_retry<T, F, Fut>(
    config: &ClientConfig,
    operation: F,
) -> Result<T, BackendError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, BackendError>>,
{
    let mut last_error = None;
    let mut delay = config.retry_delay;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() => {
                last_error = Some(e);
                if attempt < config.max_retries {
                    tracing::debug!(
                        "Retrying after error (attempt {}/{}): {:?}",
                        attempt + 1,
                        config.max_retries,
                        last_error
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap_or(BackendError::NetworkError(
        "Max retries exceeded".to_string(),
    )))
}
```

2. Add `is_retryable()` to BackendError:
```rust
impl BackendError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            BackendError::NetworkError(_) | BackendError::Timeout
        )
    }
}
```

3. Wrap API calls in `list_models()`, `model_info()`, etc.

### Test
```rust
#[tokio::test]
async fn test_retry_on_transient_failure() {
    // Mock server that fails first 2 requests, succeeds on 3rd
}
```

---

## Fix 2: Implement Parameter Injection (MCP-2,3,4,5)

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn-mcp/src/server/handler.rs`

### Problems

1. **MCP-2 (Path params)**: `{id}` not replaced in URL
2. **MCP-3 (URL injection)**: Only `base_url + path`, no param substitution
3. **MCP-4 (Query params)**: Not appended to URL
4. **MCP-5 (Header params)**: Not added to request

### Fix Approach

1. Update `handle_request()` around line 265:

```rust
// Build URL with path parameters
let mut url = format!("{}{}", config.base_url, tool.path);

// Inject path parameters
for param in &tool.params {
    if param.location == ParamLocation::Path {
        if let Some(value) = input.get(&param.name) {
            let placeholder = format!("{{{}}}", param.name);
            url = url.replace(&placeholder, &value.to_string().trim_matches('"'));
        }
    }
}

// Build query string
let mut query_params = Vec::new();
for param in &tool.params {
    if param.location == ParamLocation::Query {
        if let Some(value) = input.get(&param.name) {
            query_params.push(format!(
                "{}={}",
                urlencoding::encode(&param.name),
                urlencoding::encode(&value.to_string().trim_matches('"'))
            ));
        }
    }
}
if !query_params.is_empty() {
    url = format!("{}?{}", url, query_params.join("&"));
}

// Build request with headers
let mut request = self.client.request(method, &url);

for param in &tool.params {
    if param.location == ParamLocation::Header {
        if let Some(value) = input.get(&param.name) {
            request = request.header(
                &param.name,
                value.to_string().trim_matches('"'),
            );
        }
    }
}
```

2. Add `ParamLocation` enum if not exists:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ParamLocation {
    Path,
    Query,
    Header,
    Body,
}
```

3. Update `ParamDef` to include location:
```rust
pub struct ParamDef {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub location: ParamLocation, // NEW
    pub param_type: ParamType,
}
```

4. Update `parameter_to_param_def()` in parser.rs to set location.

### Test
```rust
#[tokio::test]
async fn test_path_parameter_injection() {
    let tool = ToolDef {
        name: "get_user".to_string(),
        path: "/users/{id}".to_string(),
        params: vec![ParamDef {
            name: "id".to_string(),
            location: ParamLocation::Path,
            ..Default::default()
        }],
        ..Default::default()
    };

    let input = json!({"id": "123"});
    // Verify URL becomes /users/123
}

#[tokio::test]
async fn test_query_parameter_injection() {
    // Similar test for query params
}
```

---

## Fix 3: Add .env Support to spn-keyring (KEY-1,2)

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn-keyring/src/lib.rs`

### Problem
`resolve_key()` claims to support .env but doesn't implement it.

### Fix Approach

1. Add `dotenvy` dependency to Cargo.toml:
```toml
[dependencies]
dotenvy = "0.15"
```

2. Update `resolve_key()` at line 460-476:
```rust
pub fn resolve_key(provider: &str) -> Option<(Zeroizing<String>, SecretSource)> {
    // 1. Try OS Keychain
    if let Ok(key) = SpnKeyring::get(provider) {
        return Some((key, SecretSource::Keychain));
    }

    // 2. Try environment variable
    if let Some(env_var) = provider_to_env_var(provider) {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Some((Zeroizing::new(key), SecretSource::Environment));
            }
        }

        // 3. Try .env file (NEW)
        if dotenvy::dotenv().is_ok() {
            if let Ok(key) = std::env::var(env_var) {
                if !key.is_empty() {
                    return Some((Zeroizing::new(key), SecretSource::DotEnv));
                }
            }
        }
    }

    None
}
```

3. Update documentation to match implementation.

### Test
```rust
#[test]
fn test_resolve_key_from_dotenv() {
    // Create temp .env file
    // Verify resolve_key returns DotEnv source
}
```

---

## Fix 4: Replace Setup Unwraps (CMD-5)

### Location
`/Users/thibaut/dev/supernovae/supernovae-cli/crates/spn/src/commands/setup.rs`

### Problem
Lines 2101-2109 have 3 unwraps in JSON schema injection:
```rust
let entry = schema["json.schemas"].as_array_mut().unwrap();
entry.push(...);
```

### Fix Approach

Replace with proper error handling:
```rust
let schemas = schema
    .get_mut("json.schemas")
    .and_then(|v| v.as_array_mut())
    .ok_or_else(|| anyhow::anyhow!("json.schemas not found or not an array"))?;

schemas.push(serde_json::json!({
    "fileMatch": ["*.nika.yaml", "*.nika.yml"],
    "url": "https://nika.sh/schema.json"
}));
```

### Test
```bash
cargo test -p spn-cli setup
cargo clippy -p spn-cli -- -D warnings
```

---

## Execution Order

```bash
# 1. Create branch
git checkout -b fix/p1-sprint-fixes

# 2. Fix in order (least risky first)
# 2a. CMD-5 (setup unwraps) - 1h
# 2b. KEY-1,2 (.env support) - 2h
# 2c. MCP-2,3,4,5 (params) - 4h
# 2d. OLL-1 (retry logic) - 2h

# 3. Test after each fix
cargo test --workspace

# 4. Commit atomically
git commit -m "fix(setup): replace unwraps with proper error handling"
git commit -m "feat(keyring): add .env file support to resolve_key"
git commit -m "feat(mcp): implement path/query/header parameter injection"
git commit -m "feat(ollama): implement retry logic with exponential backoff"

# 5. Push
git push -u origin fix/p1-sprint-fixes
```

---

## Verification Checklist

- [ ] All tests pass
- [ ] No new clippy warnings
- [ ] .env support works in spn-keyring
- [ ] Parameter injection works for path, query, header
- [ ] Retry logic handles transient failures
- [ ] Setup wizard handles missing JSON schemas gracefully
