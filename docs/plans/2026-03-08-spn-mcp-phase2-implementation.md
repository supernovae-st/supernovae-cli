# spn-mcp Phase 2: Complete Implementation Plan

**Date:** 2026-03-08
**Status:** Active
**Depends On:** Phase 1 (completed - security hardening)

---

## Executive Summary

This plan details implementation of 5 remaining improvements identified by sniper agents.
Designed for parallel execution with 6 independent agents.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  IMPLEMENTATION PHASES                                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Phase 1: Config Hardening         ████████░░░░░░░░░░░░  (BLOCKING)            │
│  Phase 2: Handler Refactoring      ░░░░░░░░░░░░░░░░░░░░  (ENABLES TESTING)     │
│  Phase 3: MCP Resources/Prompts    ░░░░░░░░░░░░░░░░░░░░  (PARALLEL)            │
│  Phase 4: Rate Limiting            ░░░░░░░░░░░░░░░░░░░░  (NEEDS CONFIG)        │
│  Phase 5: Test Coverage            ░░░░░░░░░░░░░░░░░░░░  (FINAL)               │
│                                                                                 │
│  Estimated: ~500 lines of code + ~300 lines of tests                           │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Config Hardening

**Agent:** `config-validator`
**Files:** `config/schema.rs`, `config/mod.rs`, `config/loader.rs`
**Priority:** BLOCKING (all other phases depend on valid configs)

### 1.1 Add Validation Method

```rust
// config/schema.rs

impl ApiConfig {
    /// Validate configuration after parsing.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate API name
        if self.name.is_empty() {
            errors.push("API name cannot be empty".into());
        }
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push(format!("API name '{}' contains invalid characters", self.name));
        }

        // Validate base_url
        if !self.base_url.starts_with("https://") && !self.base_url.starts_with("http://") {
            errors.push(format!("base_url '{}' must start with http:// or https://", self.base_url));
        }

        // Validate auth
        self.auth.validate(&mut errors);

        // Validate rate_limit bounds
        if let Some(ref rl) = self.rate_limit {
            if rl.requests_per_minute == 0 || rl.requests_per_minute > 10000 {
                errors.push(format!(
                    "requests_per_minute must be 1-10000, got {}",
                    rl.requests_per_minute
                ));
            }
            if rl.burst == 0 || rl.burst > 100 {
                errors.push(format!("burst must be 1-100, got {}", rl.burst));
            }
        }

        // Validate tools
        for tool in &self.tools {
            tool.validate(&mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl AuthConfig {
    fn validate(&self, errors: &mut Vec<String>) {
        // ApiKey requires key_name
        if self.auth_type == AuthType::ApiKey && self.key_name.is_none() {
            errors.push("ApiKey auth type requires 'key_name' field".into());
        }
    }
}

impl ToolDef {
    fn validate(&self, errors: &mut Vec<String>) {
        // Validate HTTP method
        const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
        if !VALID_METHODS.contains(&self.method.to_uppercase().as_str()) {
            errors.push(format!("Invalid HTTP method '{}' in tool '{}'", self.method, self.name));
        }

        // Validate array params have items
        for param in &self.params {
            if param.param_type == ParamType::Array && param.items.is_none() {
                errors.push(format!(
                    "Array parameter '{}' in tool '{}' requires 'items' field",
                    param.name, self.name
                ));
            }
        }

        // Validate path starts with /
        if !self.path.starts_with('/') {
            errors.push(format!(
                "Tool path '{}' in tool '{}' must start with '/'",
                self.path, self.name
            ));
        }
    }
}
```

### 1.2 Integrate Validation in Loader

```rust
// config/loader.rs

fn load_from_path(path: &PathBuf) -> Result<ApiConfig> {
    let content = fs::read_to_string(path)?;
    let config: ApiConfig = serde_yaml::from_str(&content)?;

    // Validate after parsing
    config.validate().map_err(|errors| {
        Error::ConfigValidation(format!(
            "Validation failed for {}:\n  - {}",
            path.display(),
            errors.join("\n  - ")
        ))
    })?;

    Ok(config)
}
```

### 1.3 Tests for Validation

```rust
#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validate_rejects_empty_name() {
        let config = ApiConfig {
            name: "".into(),
            // ...minimal valid fields
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_invalid_method() {
        let config = create_config_with_tool("INVALID", "/path");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("Invalid HTTP method"));
    }

    #[test]
    fn test_validate_rejects_array_without_items() {
        let config = create_config_with_array_param(None);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("requires 'items'"));
    }

    #[test]
    fn test_validate_rejects_apikey_without_key_name() {
        let config = create_config_with_auth(AuthType::ApiKey, None);
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_accepts_valid_config() {
        let config = create_valid_config();
        assert!(config.validate().is_ok());
    }
}
```

---

## Phase 2: Handler Refactoring

**Agent:** `handler-refactor`
**Files:** `server/handler.rs`
**Priority:** HIGH (enables testing)

### 2.1 Extract Request Building

```rust
impl DynamicHandler {
    /// Build HTTP request from tool definition and parameters.
    ///
    /// Validates path, constructs URL, applies auth and headers.
    fn build_request(
        &self,
        entry: &ToolEntry,
        params: &Value,
    ) -> Result<reqwest::RequestBuilder> {
        let config = &entry.api_config;
        let tool = &entry.tool_def;

        // Security: Validate tool path
        validate_tool_path(&tool.path)?;

        // Build URL
        let url = format!("{}{}", config.base_url, tool.path);

        // Get credential
        let credential = self
            .credentials
            .get(&config.name)
            .ok_or_else(|| Error::Credential(config.name.clone(), "not found".into()))?;

        // Create request with method
        let mut request = match tool.method.to_uppercase().as_str() {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "DELETE" => self.http_client.delete(&url),
            "PATCH" => self.http_client.patch(&url),
            "HEAD" => self.http_client.head(&url),
            "OPTIONS" => self.http_client.request(reqwest::Method::OPTIONS, &url),
            _ => return Err(Error::ConfigValidation(format!("Unknown method: {}", tool.method))),
        };

        // Apply authentication
        request = apply_auth(request, &config.auth, credential);

        // Add default headers
        if let Some(headers) = &config.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        // Add body if template exists
        if let Some(template) = &tool.body_template {
            let body = self.render_template(template, params)?;
            request = request.body(body).header("Content-Type", "application/json");
        }

        Ok(request)
    }

    /// Execute request and parse response.
    async fn execute_request(
        &self,
        request: reqwest::RequestBuilder,
        tool: &ToolDef,
    ) -> Result<Value> {
        tracing::debug!("Executing request");
        let response = request.send().await.map_err(Error::Http)?;

        // Check status
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| {
                tracing::warn!("Failed to read error response body: {}", e);
                String::new()
            });
            return Err(Error::Mcp(format!("API returned {}: {}", status, body)));
        }

        // Parse response
        let body: Value = response.json().await.map_err(Error::Http)?;

        // Apply response extraction
        let result = if let Some(ref response_config) = tool.response {
            if let Some(ref extract) = response_config.extract {
                extract_json_path(&body, extract)?
            } else {
                body
            }
        } else {
            body
        };

        Ok(result)
    }

    /// Execute a tool (orchestrator).
    async fn execute_tool(&self, entry: &ToolEntry, params: Value) -> Result<Value> {
        let request = self.build_request(entry, &params)?;
        self.execute_request(request, &entry.tool_def).await
    }
}
```

### 2.2 Tests for Extracted Functions

```rust
#[cfg(test)]
mod refactored_tests {
    use super::*;

    #[test]
    fn test_build_request_validates_path() {
        let handler = create_test_handler();
        let entry = create_entry_with_path("/../etc/passwd");

        let result = handler.build_request(&entry, &json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_build_request_applies_bearer_auth() {
        let handler = create_handler_with_credential("test", "secret123");
        let entry = create_bearer_entry("test");

        let request = handler.build_request(&entry, &json!({})).unwrap();
        // Verify auth header would be set (can't inspect directly)
    }

    #[tokio::test]
    async fn test_execute_request_handles_500() {
        let server = mockito::Server::new();
        let mock = server.mock("GET", "/test")
            .with_status(500)
            .with_body("Internal error")
            .create();

        let handler = create_handler_for_server(&server);
        let request = handler.http_client.get(server.url() + "/test");
        let tool = create_minimal_tool();

        let result = handler.execute_request(request, &tool).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("500"));
        mock.assert();
    }
}
```

---

## Phase 3: MCP Protocol Enhancement

**Agent A:** `mcp-resources`
**Agent B:** `mcp-prompts`
**Files:** `server/handler.rs`

### 3.1 Resources Implementation (Agent A)

```rust
impl ServerHandler for DynamicHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Dynamic REST-to-MCP wrapper. Tools are loaded from ~/.spn/apis/".into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()      // NEW
                .enable_prompts()        // NEW
                .build(),
            ..Default::default()
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move {
            let resources: Vec<Resource> = self.api_configs()
                .map(|config| {
                    Resource::new(
                        format!("api://{}", config.name),
                        config.description.clone().unwrap_or_else(|| {
                            format!("{} API configuration", config.name)
                        }),
                    )
                    .with_mime_type("application/json")
                })
                .collect();

            Ok(ListResourcesResult::with_all_items(resources))
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            let uri = request.uri.as_str();

            // Parse api://name format
            let name = uri.strip_prefix("api://")
                .ok_or_else(|| McpError::invalid_params("Invalid URI format", None))?;

            let config = self.get_api_config(name)
                .ok_or_else(|| McpError::resource_not_found(format!("API not found: {}", name), None))?;

            // Return sanitized config (no credentials)
            let sanitized = json!({
                "name": config.name,
                "version": config.version,
                "base_url": config.base_url,
                "description": config.description,
                "tools": config.tools.iter().map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "method": t.method,
                        "path": t.path,
                        "params": t.params,
                    })
                }).collect::<Vec<_>>(),
            });

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    uri,
                    serde_json::to_string_pretty(&sanitized).unwrap(),
                )],
            })
        }
    }
}
```

### 3.2 Prompts Implementation (Agent B)

```rust
impl ServerHandler for DynamicHandler {
    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        async move {
            let prompts: Vec<Prompt> = self.tools.values()
                .map(|entry| {
                    Prompt::new(
                        entry.full_name.clone(),
                        entry.tool_def.description.clone(),
                    )
                    .with_arguments(
                        entry.tool_def.params.iter().map(|p| {
                            PromptArgument::new(p.name.clone())
                                .with_description(p.description.clone().unwrap_or_default())
                                .required(p.required)
                        }).collect()
                    )
                })
                .collect();

            Ok(ListPromptsResult::with_all_items(prompts))
        }
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        async move {
            let name = request.name.as_ref();
            let entry = self.tools.get(name)
                .ok_or_else(|| McpError::invalid_params(format!("Prompt not found: {}", name), None))?;

            let tool = &entry.tool_def;
            let config = &entry.api_config;

            // Generate prompt template
            let prompt_text = format!(
                "Use the {} tool to call {}{}\n\n\
                 Description: {}\n\n\
                 Parameters:\n{}",
                entry.full_name,
                config.base_url,
                tool.path,
                tool.description.as_deref().unwrap_or("No description"),
                tool.params.iter()
                    .map(|p| format!(
                        "- {}: {} {}{}",
                        p.name,
                        format!("{:?}", p.param_type).to_lowercase(),
                        if p.required { "(required)" } else { "(optional)" },
                        p.description.as_ref().map(|d| format!(" - {}", d)).unwrap_or_default()
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            Ok(GetPromptResult {
                description: tool.description.clone(),
                messages: vec![PromptMessage::user(prompt_text)],
            })
        }
    }
}
```

---

## Phase 4: Rate Limiting

**Agent:** `rate-limiter`
**Files:** `server/handler.rs`, `server/rate_limit.rs`, `Cargo.toml`

### 4.1 Add Governor Dependency

```toml
# Cargo.toml
[dependencies]
governor = "0.6"
```

### 4.2 Rate Limiter Module

```rust
// server/rate_limit.rs

use governor::{Quota, RateLimiter as GovRateLimiter};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::config::RateLimitConfig;

pub type ApiRateLimiter = GovRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Create a rate limiter from config.
pub fn create_limiter(config: &RateLimitConfig) -> Arc<ApiRateLimiter> {
    let quota = Quota::per_minute(
        NonZeroU32::new(config.requests_per_minute).unwrap_or(NonZeroU32::MIN)
    ).allow_burst(
        NonZeroU32::new(config.burst).unwrap_or(NonZeroU32::MIN)
    );

    Arc::new(GovRateLimiter::direct(quota))
}

/// Check rate limit, return error if exceeded.
pub async fn check_limit(limiter: &ApiRateLimiter, api_name: &str) -> crate::error::Result<()> {
    limiter.check().map_err(|_| {
        crate::error::Error::Mcp(format!(
            "Rate limit exceeded for API '{}'. Please wait before retrying.",
            api_name
        ))
    })
}
```

### 4.3 Integrate in Handler

```rust
// server/handler.rs

use crate::server::rate_limit::{ApiRateLimiter, create_limiter, check_limit};

pub struct DynamicHandler {
    tools: HashMap<String, ToolEntry>,
    http_client: Client,
    tera: Tera,
    credentials: HashMap<String, String>,
    rate_limiters: HashMap<String, Arc<ApiRateLimiter>>,  // NEW
}

impl DynamicHandler {
    pub async fn new(configs: Vec<ApiConfig>) -> Result<Self> {
        // ... existing code ...

        let mut rate_limiters = HashMap::new();
        for config in &configs {
            if let Some(ref rl_config) = config.rate_limit {
                rate_limiters.insert(config.name.clone(), create_limiter(rl_config));
            }
        }

        Ok(Self {
            tools,
            http_client,
            tera,
            credentials,
            rate_limiters,
        })
    }

    async fn execute_tool(&self, entry: &ToolEntry, params: Value) -> Result<Value> {
        // Check rate limit FIRST
        if let Some(limiter) = self.rate_limiters.get(&entry.api_config.name) {
            check_limit(limiter, &entry.api_config.name).await?;
        }

        let request = self.build_request(entry, &params)?;
        self.execute_request(request, &entry.tool_def).await
    }
}
```

### 4.4 Tests

```rust
#[tokio::test]
async fn test_rate_limiter_blocks_after_limit() {
    let config = RateLimitConfig {
        requests_per_minute: 2,
        burst: 1,
    };
    let limiter = create_limiter(&config);

    // First request should succeed
    assert!(check_limit(&limiter, "test").await.is_ok());

    // Second request should succeed (within burst)
    assert!(check_limit(&limiter, "test").await.is_ok());

    // Third request should fail
    let result = check_limit(&limiter, "test").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Rate limit exceeded"));
}
```

---

## Phase 5: Test Coverage

**Agent:** `test-writer`
**Files:** `server/handler.rs`, `tests/integration.rs`, `Cargo.toml`

### 5.1 Add Test Dependencies

```toml
# Cargo.toml
[dev-dependencies]
mockito = "1.2"
tokio-test = "0.4"
```

### 5.2 Test Infrastructure

```rust
// server/handler.rs - test module additions

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Server, Mock};

    /// Create a test handler with mock credentials.
    fn create_test_handler() -> DynamicHandler {
        DynamicHandler {
            tools: HashMap::new(),
            http_client: Client::new(),
            tera: Tera::default(),
            credentials: HashMap::from([("test".into(), "secret".into())]),
            rate_limiters: HashMap::new(),
        }
    }

    /// Create a mock API config for testing.
    fn mock_api_config(name: &str, base_url: &str) -> ApiConfig {
        ApiConfig {
            name: name.into(),
            version: "1.0".into(),
            base_url: base_url.into(),
            description: Some("Test API".into()),
            auth: AuthConfig {
                auth_type: AuthType::Bearer,
                credential: name.into(),
                location: None,
                key_name: None,
            },
            rate_limit: None,
            headers: None,
            tools: vec![],
        }
    }

    /// Create a tool entry for testing.
    fn mock_tool_entry(api_name: &str, tool_name: &str, path: &str) -> ToolEntry {
        let config = Arc::new(mock_api_config(api_name, "https://api.test.com"));
        ToolEntry {
            full_name: format!("{}_{}", api_name, tool_name),
            api_config: config,
            tool_def: ToolDef {
                name: tool_name.into(),
                description: Some("Test tool".into()),
                method: "GET".into(),
                path: path.into(),
                body_template: None,
                params: vec![],
                response: None,
            },
        }
    }

    // ... existing tests ...

    #[tokio::test]
    async fn test_execute_tool_success() {
        let mut server = Server::new_async().await;
        let mock = server.mock("GET", "/test")
            .with_status(200)
            .with_body(r#"{"result": "success"}"#)
            .create_async()
            .await;

        let mut handler = create_test_handler();
        // Setup handler with server URL...

        // Execute and verify
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_execute_tool_handles_timeout() {
        // Test with very short timeout
    }

    #[tokio::test]
    async fn test_execute_tool_handles_invalid_json() {
        let mut server = Server::new_async().await;
        let mock = server.mock("GET", "/test")
            .with_status(200)
            .with_body("not json")
            .create_async()
            .await;

        // Should return error
    }
}
```

---

## Agent Assignment Summary

| Agent | Phase | Files | Scope |
|-------|-------|-------|-------|
| `config-validator` | 1 | schema.rs, mod.rs, loader.rs | Validation methods + tests |
| `handler-refactor` | 2 | handler.rs | Extract build_request, execute_request |
| `mcp-resources` | 3A | handler.rs | list_resources, read_resource |
| `mcp-prompts` | 3B | handler.rs | list_prompts, get_prompt |
| `rate-limiter` | 4 | rate_limit.rs, handler.rs, Cargo.toml | Governor integration |
| `test-writer` | 5 | handler.rs tests, integration.rs | mockito setup + critical tests |

---

## Dependency Graph

```
                    ┌─────────────────────┐
                    │  config-validator   │ ◄── BLOCKING
                    │     (Phase 1)       │
                    └─────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
    ┌─────────────────┐ ┌───────────┐ ┌─────────────────┐
    │ handler-refactor│ │rate-limiter│ │  mcp-resources  │
    │    (Phase 2)    │ │ (Phase 4) │ │    (Phase 3A)   │
    └─────────────────┘ └───────────┘ └─────────────────┘
              │               │               │
              │               │               ▼
              │               │       ┌─────────────────┐
              │               │       │  mcp-prompts    │
              │               │       │    (Phase 3B)   │
              │               │       └─────────────────┘
              │               │               │
              └───────────────┴───────────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │    test-writer      │
                    │     (Phase 5)       │
                    └─────────────────────┘
```

---

## Success Criteria

- [ ] All validation errors caught at config load time
- [ ] execute_tool() split into testable functions
- [ ] MCP Resources exposed for all APIs
- [ ] MCP Prompts generated for all tools
- [ ] Rate limiting enforced per-API
- [ ] Test coverage: handler.rs > 80%
- [ ] All existing tests pass
- [ ] Zero clippy warnings
