# P2 Backlog Fix Plan
## Fix Next Sprint
**Generated:** 2026-03-10
**Target:** v0.16.x - v0.17.0

---

## Overview

12 medium-priority issues for next sprint:

| Category | Issues | Time Est |
|----------|--------|----------|
| spn-client protocol | CLI-1,2,3,4,5,6 | 8h |
| spn-ollama error handling | OLL-2,3 | 2h |
| spn-mcp OpenAPI support | MCP-6,7,8 | 4h |
| spn-core validation | CORE-1,2 | 2h |

**Total Estimated Time:** 16 hours

---

## 1. Implement Missing Protocol Commands (CLI-1 to CLI-6)

### Current State
Only 6/18 protocol commands implemented:
- ✅ Ping, GetSecret, HasSecret, ListProviders, RefreshSecret, WatcherStatus
- ❌ ModelList, ModelPull, ModelLoad, ModelUnload, ModelStatus, ModelDelete
- ❌ ModelRun, JobSubmit, JobStatus, JobList, JobCancel, JobStats

### Implementation Plan

#### Phase A: Model Commands (4h)

```rust
// crates/spn-client/src/lib.rs

impl SpnClient {
    /// List available models from daemon
    pub async fn model_list(&self) -> Result<Vec<ModelInfo>, ClientError> {
        let response = self.send_request(Request::ModelList).await?;
        match response {
            Response::ModelListResult { models } => Ok(models),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Pull a model with progress callback
    pub async fn model_pull<F>(&self, name: &str, on_progress: F) -> Result<(), ClientError>
    where
        F: Fn(PullProgress) + Send + 'static,
    {
        // Send initial request
        let response = self.send_request(Request::ModelPull {
            name: name.to_string()
        }).await?;

        // Handle streaming progress
        loop {
            match self.read_response().await? {
                Response::Progress(progress) => on_progress(progress),
                Response::StreamEnd { error: None, .. } => return Ok(()),
                Response::StreamEnd { error: Some(e), .. } => {
                    return Err(ClientError::DaemonError(e))
                }
                _ => continue,
            }
        }
    }

    /// Load model into memory
    pub async fn model_load(&self, name: &str, config: LoadConfig) -> Result<(), ClientError> {
        let response = self.send_request(Request::ModelLoad {
            name: name.to_string(),
            config,
        }).await?;
        match response {
            Response::Success => Ok(()),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Unload model from memory
    pub async fn model_unload(&self, name: &str) -> Result<(), ClientError> {
        let response = self.send_request(Request::ModelUnload {
            name: name.to_string()
        }).await?;
        match response {
            Response::Success => Ok(()),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Get model status
    pub async fn model_status(&self) -> Result<Vec<RunningModel>, ClientError> {
        let response = self.send_request(Request::ModelStatus).await?;
        match response {
            Response::ModelStatusResult { models } => Ok(models),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Delete a model
    pub async fn model_delete(&self, name: &str) -> Result<(), ClientError> {
        let response = self.send_request(Request::ModelDelete {
            name: name.to_string()
        }).await?;
        match response {
            Response::Success => Ok(()),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
```

#### Phase B: Job Commands (4h)

```rust
impl SpnClient {
    /// Submit a new job
    pub async fn job_submit(&self, workflow: &str, args: Value) -> Result<JobId, ClientError> {
        let response = self.send_request(Request::JobSubmit {
            workflow: workflow.to_string(),
            args,
            priority: 0,
        }).await?;
        match response {
            Response::JobSubmitted { job_id } => Ok(job_id),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Get job status
    pub async fn job_status(&self, job_id: &str) -> Result<Option<JobStatus>, ClientError> {
        let response = self.send_request(Request::JobStatus {
            job_id: job_id.to_string(),
        }).await?;
        match response {
            Response::JobStatusResult { job } => Ok(job),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// List jobs by state
    pub async fn job_list(&self, state: Option<JobState>) -> Result<Vec<JobSummary>, ClientError> {
        let response = self.send_request(Request::JobList { state }).await?;
        match response {
            Response::JobListResult { jobs } => Ok(jobs),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Cancel a job
    pub async fn job_cancel(&self, job_id: &str) -> Result<bool, ClientError> {
        let response = self.send_request(Request::JobCancel {
            job_id: job_id.to_string(),
        }).await?;
        match response {
            Response::JobCancelled { cancelled } => Ok(cancelled),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Get job statistics
    pub async fn job_stats(&self) -> Result<JobStats, ClientError> {
        let response = self.send_request(Request::JobStats).await?;
        match response {
            Response::JobStatsResult(stats) => Ok(stats),
            Response::Error(e) => Err(ClientError::DaemonError(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
```

### Tests Required
- Unit tests for each method
- Integration tests with mock daemon
- Streaming progress callback tests

---

## 2. Improve Ollama Error Handling (OLL-2, OLL-3)

### Fix OLL-2: Replace unwrap_or_default on error text

**Files:** `crates/spn-ollama/src/client.rs` lines 423, 472, 542, 621, 676

```rust
// Before:
let text = response.text().await.unwrap_or_default();

// After:
let text = match response.text().await {
    Ok(t) => t,
    Err(e) => {
        tracing::warn!("Failed to read error response: {}", e);
        format!("HTTP {}: (unable to read response body)", status)
    }
};
```

### Fix OLL-3: Validate embedding extraction

**File:** `crates/spn-ollama/src/client.rs` line 633

```rust
// Before:
let embedding = body.embeddings.into_iter().next().unwrap_or_default();

// After:
let embedding = body.embeddings
    .into_iter()
    .next()
    .ok_or_else(|| BackendError::ApiError(
        "Ollama returned empty embeddings array".to_string()
    ))?;

// Also validate dimension
if embedding.is_empty() {
    return Err(BackendError::ApiError(
        "Ollama returned zero-dimension embedding".to_string()
    ));
}
```

---

## 3. Complete OpenAPI Support (MCP-6, MCP-7, MCP-8)

### Fix MCP-6: Request body from OpenAPI

**File:** `crates/spn-mcp/src/openapi/parser.rs`

```rust
fn operation_to_tool(
    path: &str,
    method: HttpMethod,
    op: &Operation,
    path_params: &[&Parameter],
) -> Option<ToolDef> {
    // ... existing code ...

    // NEW: Handle requestBody
    let body_schema = op.request_body.as_ref().and_then(|rb| {
        rb.content
            .get("application/json")
            .and_then(|media| media.schema.as_ref())
    });

    if let Some(schema) = body_schema {
        // Convert schema to body_template
        tool.body_template = Some(schema_to_template(schema));
    }

    Some(tool)
}

fn schema_to_template(schema: &Schema) -> String {
    // Generate Tera template from JSON Schema
    // Example: {"name": "{{ name }}", "email": "{{ email }}"}
}
```

### Fix MCP-7: Improve JSON path extraction

**File:** `crates/spn-mcp/src/server/handler.rs` lines 667-698

```rust
fn extract_json_path(value: &Value, path: &str) -> Option<Value> {
    let mut current = value;

    for segment in parse_path_segments(path) {
        current = match segment {
            PathSegment::Key(key) => current.get(key)?,
            PathSegment::Index(idx) => current.get(idx)?,
            PathSegment::Wildcard => {
                // Return array of matched values
                return Some(extract_wildcard(current));
            }
        };
    }

    Some(current.clone())
}

enum PathSegment {
    Key(String),
    Index(usize),
    Wildcard,
}

fn parse_path_segments(path: &str) -> Vec<PathSegment> {
    // Parse "data[0].items[*].name" into segments
}
```

### Fix MCP-8: Tool name collision detection

**File:** `crates/spn-mcp/src/openapi/parser.rs` lines 458-482

```rust
pub fn parse_openapi(spec: &OpenApiSpec) -> Result<Vec<ToolDef>, OpenApiError> {
    let mut tools = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for (path, path_item) in &spec.paths {
        for (method, op) in path_item.operations() {
            if let Some(mut tool) = operation_to_tool(path, method, op, &[]) {
                // Check for collision
                if seen_names.contains(&tool.name) {
                    // Make unique by appending method
                    tool.name = format!("{}_{}", tool.name, method.as_str().to_lowercase());
                }

                if seen_names.contains(&tool.name) {
                    // Still collision - add counter
                    let mut counter = 2;
                    while seen_names.contains(&format!("{}_{}", tool.name, counter)) {
                        counter += 1;
                    }
                    tool.name = format!("{}_{}", tool.name, counter);
                }

                seen_names.insert(tool.name.clone());
                tools.push(tool);
            }
        }
    }

    Ok(tools)
}
```

---

## 4. Complete Registry Validation (CORE-1, CORE-2)

### Fix CORE-1: Implement scope extraction

**File:** `crates/spn-core/src/registry.rs` line 235

```rust
pub fn as_ref(&self) -> PackageRef {
    // Extract scope from name if present
    let (scope, name) = if self.name.starts_with('@') {
        if let Some(slash_pos) = self.name.find('/') {
            (
                Some(self.name[1..slash_pos].to_string()),
                self.name[slash_pos + 1..].to_string(),
            )
        } else {
            (None, self.name.clone())
        }
    } else {
        (None, self.name.clone())
    };

    PackageRef {
        scope,
        name,
        version: Some(self.version.clone()),
    }
}
```

### Fix CORE-2: Validate scoped name format

**File:** `crates/spn-core/src/registry.rs` line 120-160

```rust
pub fn parse(input: &str) -> Option<Self> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Validate scoped package format
    if input.starts_with('@') {
        // Must have / separator
        let slash_pos = input.find('/')?;

        // Scope cannot be empty
        let scope = &input[1..slash_pos];
        if scope.is_empty() {
            return None;
        }

        // Name cannot be empty
        let rest = &input[slash_pos + 1..];
        if rest.is_empty() || rest.starts_with('@') {
            return None;
        }

        // Validate characters (alphanumeric, -, _)
        if !is_valid_package_name(scope) || !is_valid_package_component(rest) {
            return None;
        }
    }

    // ... rest of parsing
}

fn is_valid_package_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_valid_package_component(s: &str) -> bool {
    // Parse name@version format
    let (name, _version) = s.split_once('@').unwrap_or((s, ""));
    is_valid_package_name(name)
}
```

### Tests to Add

```rust
#[test]
fn test_parse_invalid_scoped_names() {
    assert!(PackageRef::parse("@/name").is_none(), "empty scope");
    assert!(PackageRef::parse("@scope/").is_none(), "empty name");
    assert!(PackageRef::parse("@scope/@invalid").is_none(), "double @");
    assert!(PackageRef::parse("@scope//name").is_none(), "double /");
    assert!(PackageRef::parse("@scope/name@1.0.0@2.0.0").is_some()); // First version used
}

#[test]
fn test_provider_ids_unique() {
    let ids: Vec<_> = KNOWN_PROVIDERS.iter().map(|p| p.id.to_lowercase()).collect();
    let unique: std::collections::HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(ids.len(), unique.len(), "Provider IDs should be unique (case-insensitive)");
}
```

---

## Execution Order

1. **Week 1:** spn-core validation (CORE-1, CORE-2) - low risk
2. **Week 1:** spn-ollama error handling (OLL-2, OLL-3) - isolated changes
3. **Week 2:** spn-mcp OpenAPI (MCP-6, MCP-7, MCP-8) - moderate complexity
4. **Week 2-3:** spn-client protocol (CLI-1 to CLI-6) - most complex

---

## Verification Checklist

- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] No new clippy warnings
- [ ] API documentation updated
- [ ] CHANGELOG.md updated
