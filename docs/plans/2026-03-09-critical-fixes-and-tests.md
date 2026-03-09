# Critical Fixes and Test Coverage Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix critical error handling issues, add test coverage for untested MCP protocol files, and correct documentation mismatches.

**Architecture:** Three parallel workstreams - (1) error handling fixes in production code paths, (2) comprehensive test suites for MCP protocol types, (3) documentation accuracy cleanup.

**Tech Stack:** Rust 2021, serde_json, tokio, cargo test

---

## Task 1: Fix MCP stdout Error Handling

**Files:**
- Modify: `crates/spn/src/daemon/mcp/server.rs:65-66`

**Step 1: Read the current problematic code**

```rust
// Current code at line 65-66 (inside error handling block):
let _ = writeln!(stdout, "{}", serde_json::to_string(&response)?);
let _ = stdout.flush();
```

**Step 2: Fix the error handling**

Replace lines 65-66 with proper error logging:

```rust
if let Err(e) = writeln!(stdout, "{}", serde_json::to_string(&response)?) {
    error!("Failed to write MCP error response: {}", e);
}
if let Err(e) = stdout.flush() {
    error!("Failed to flush stdout: {}", e);
}
```

**Step 3: Run clippy to verify**

Run: `cargo clippy -p spn-cli -- -D warnings`
Expected: PASS with no warnings

**Step 4: Run tests**

Run: `cargo test -p spn-cli`
Expected: All tests pass

**Step 5: Commit**

```bash
git add crates/spn/src/daemon/mcp/server.rs
git commit -m "fix(mcp): log errors instead of silently ignoring stdout failures

MCP error responses were silently dropped when writeln/flush failed.
Now logs errors to help debug protocol issues.

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Task 2: Fix Hardcoded .expect() in Onboarding

**Files:**
- Modify: `crates/spn/src/ux/onboarding.rs:208`

**Step 1: Read the current problematic code**

```rust
// Current code at line 208:
let addr: SocketAddr = "127.0.0.1:11434".parse().expect("valid address");
```

**Step 2: Replace with direct construction (no parsing needed)**

Replace line 208 with:

```rust
let addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 11434));
```

**Step 3: Add the import if not present**

Ensure `std::net::SocketAddr` is imported (check top of file).

**Step 4: Run clippy to verify**

Run: `cargo clippy -p spn-cli -- -D warnings`
Expected: PASS with no warnings

**Step 5: Run tests**

Run: `cargo test -p spn-cli`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/spn/src/ux/onboarding.rs
git commit -m "fix(ux): remove .expect() by using direct SocketAddr construction

Replace string parsing with infallible SocketAddr::from() to
eliminate potential panic in production code path.

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Task 3: Add Tests for MCP Protocol Types

**Files:**
- Modify: `crates/spn/src/daemon/mcp/protocol.rs` (add test module)

**Step 3.1: Write test for McpRequest deserialization**

Add at the bottom of `protocol.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request_deserialize_with_params() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "test"}
        }"#;

        let request: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "tools/call");
        assert_eq!(request.id, Some(serde_json::json!(1)));
        assert_eq!(request.params["name"], "test");
    }

    #[test]
    fn test_mcp_request_deserialize_without_params() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "abc-123",
            "method": "initialize"
        }"#;

        let request: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "initialize");
        assert_eq!(request.id, Some(serde_json::json!("abc-123")));
        assert!(request.params.is_null());
    }

    #[test]
    fn test_mcp_request_deserialize_notification() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }"#;

        let request: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "notifications/initialized");
        assert!(request.id.is_none());
    }
}
```

**Step 3.2: Run test to verify it passes**

Run: `cargo test -p spn-cli mcp_request_deserialize`
Expected: 3 tests pass

**Step 3.3: Write test for McpResponse serialization**

Add to the test module:

```rust
    #[test]
    fn test_mcp_response_success_serialize() {
        let response = McpResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"status": "ok"})
        );

        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["status"], "ok");
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_mcp_response_error_serialize() {
        let response = McpResponse::error(
            Some(serde_json::json!(2)),
            PARSE_ERROR,
            "Invalid JSON"
        );

        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 2);
        assert_eq!(parsed["error"]["code"], PARSE_ERROR);
        assert_eq!(parsed["error"]["message"], "Invalid JSON");
        assert!(parsed.get("result").is_none());
    }

    #[test]
    fn test_mcp_response_notification_serialize() {
        let response = McpResponse::notification(serde_json::json!({"event": "ready"}));

        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert!(parsed.get("id").is_none());
        assert_eq!(parsed["result"]["event"], "ready");
    }
```

**Step 3.4: Run all protocol tests**

Run: `cargo test -p spn-cli protocol::tests`
Expected: 6 tests pass

**Step 3.5: Write test for error codes**

Add to the test module:

```rust
    #[test]
    fn test_error_codes_are_standard_jsonrpc() {
        // JSON-RPC 2.0 standard error codes
        assert_eq!(PARSE_ERROR, -32700);
        assert_eq!(INVALID_REQUEST, -32600);
        assert_eq!(METHOD_NOT_FOUND, -32601);
        assert_eq!(INVALID_PARAMS, -32602);
        assert_eq!(INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_initialize_result_serialize() {
        let result = InitializeResult {
            protocol_version: "2024-11-05".into(),
            capabilities: ServerCapabilities {
                tools: ToolCapabilities { list_changed: false },
            },
            server_info: ServerInfo {
                name: "spn".into(),
                version: "0.15.2".into(),
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["protocolVersion"], "2024-11-05");
        assert_eq!(parsed["serverInfo"]["name"], "spn");
        assert_eq!(parsed["capabilities"]["tools"]["listChanged"], false);
    }
```

**Step 3.6: Run all protocol tests**

Run: `cargo test -p spn-cli protocol::tests`
Expected: 8 tests pass

**Step 3.7: Commit**

```bash
git add crates/spn/src/daemon/mcp/protocol.rs
git commit -m "test(mcp): add comprehensive tests for protocol types

Add 8 tests covering:
- McpRequest deserialization (with/without params, notifications)
- McpResponse serialization (success, error, notification)
- Standard JSON-RPC error codes
- InitializeResult serialization

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Task 4: Add Tests for MCP Tool Definitions

**Files:**
- Modify: `crates/spn/src/daemon/mcp/tools.rs` (add test module)

**Step 4.1: Write test for list_tools completeness**

Add at the bottom of `tools.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tools_returns_all_tools() {
        let tools = list_tools();

        assert_eq!(tools.len(), 6, "Expected 6 tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"spn_secrets_get"));
        assert!(names.contains(&"spn_secrets_list"));
        assert!(names.contains(&"spn_secrets_check"));
        assert!(names.contains(&"spn_model_list"));
        assert!(names.contains(&"spn_model_run"));
        assert!(names.contains(&"spn_status"));
    }

    #[test]
    fn test_tool_names_constant_matches_list_tools() {
        let tools = list_tools();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        for name in TOOL_NAMES {
            assert!(
                tool_names.contains(name),
                "TOOL_NAMES contains '{}' but list_tools() doesn't return it",
                name
            );
        }

        for name in &tool_names {
            assert!(
                TOOL_NAMES.contains(name),
                "list_tools() returns '{}' but TOOL_NAMES doesn't contain it",
                name
            );
        }
    }
}
```

**Step 4.2: Run tests to verify they pass**

Run: `cargo test -p spn-cli tools::tests`
Expected: 2 tests pass

**Step 4.3: Write tests for tool schema validation**

Add to the test module:

```rust
    #[test]
    fn test_tool_schemas_are_valid_json_schema() {
        let tools = list_tools();

        for tool in &tools {
            // Every schema must be an object
            assert!(
                tool.input_schema.is_object(),
                "Tool '{}' schema is not an object",
                tool.name
            );

            // Every schema must have "type": "object"
            assert_eq!(
                tool.input_schema["type"], "object",
                "Tool '{}' schema type is not 'object'",
                tool.name
            );

            // Every schema must have "properties"
            assert!(
                tool.input_schema.get("properties").is_some(),
                "Tool '{}' schema has no 'properties' field",
                tool.name
            );
        }
    }

    #[test]
    fn test_secrets_get_requires_provider() {
        let tools = list_tools();
        let tool = tools.iter().find(|t| t.name == "spn_secrets_get").unwrap();

        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("provider")));
    }

    #[test]
    fn test_model_run_requires_model_and_prompt() {
        let tools = list_tools();
        let tool = tools.iter().find(|t| t.name == "spn_model_run").unwrap();

        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("model")));
        assert!(required.contains(&serde_json::json!("prompt")));
    }
```

**Step 4.4: Run all tool tests**

Run: `cargo test -p spn-cli tools::tests`
Expected: 5 tests pass

**Step 4.5: Write tests for ToolResult**

Add to the test module:

```rust
    #[test]
    fn test_tool_result_text_serialize() {
        let result = ToolResult::text("Hello, world!");
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Hello, world!");
        assert!(parsed.get("isError").is_none());
    }

    #[test]
    fn test_tool_result_error_serialize() {
        let result = ToolResult::error("Something went wrong");
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Something went wrong");
        assert_eq!(parsed["isError"], true);
    }

    #[test]
    fn test_secrets_get_params_deserialize() {
        let json = r#"{"provider": "anthropic"}"#;
        let params: SecretsGetParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.provider, "anthropic");
    }

    #[test]
    fn test_model_run_params_deserialize() {
        let json = r#"{
            "model": "llama3.2:3b",
            "prompt": "Hello",
            "system": "Be helpful",
            "temperature": 0.7
        }"#;
        let params: ModelRunParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.model, "llama3.2:3b");
        assert_eq!(params.prompt, "Hello");
        assert_eq!(params.system, Some("Be helpful".to_string()));
        assert_eq!(params.temperature, Some(0.7));
    }

    #[test]
    fn test_model_run_params_optional_fields() {
        let json = r#"{"model": "llama3.2:3b", "prompt": "Hello"}"#;
        let params: ModelRunParams = serde_json::from_str(json).unwrap();

        assert!(params.system.is_none());
        assert!(params.temperature.is_none());
    }
```

**Step 4.6: Run all tool tests**

Run: `cargo test -p spn-cli tools::tests`
Expected: 10 tests pass

**Step 4.7: Commit**

```bash
git add crates/spn/src/daemon/mcp/tools.rs
git commit -m "test(mcp): add comprehensive tests for tool definitions

Add 10 tests covering:
- list_tools() returns all 6 tools
- TOOL_NAMES constant matches list_tools()
- Tool schemas are valid JSON Schema objects
- Required parameters are enforced
- ToolResult serialization (text and error)
- Parameter deserialization for all tool types

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Task 5: Add Basic Tests for Jobs Command

**Files:**
- Modify: `crates/spn/src/commands/jobs.rs` (add test module)

**Step 5.1: Write tests for helper functions**

Find the helper functions at the bottom of `jobs.rs` and add tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_format_state_pending() {
        let state = format_state(JobState::Pending);
        assert!(state.contains("Pending") || state.contains("pending"));
    }

    #[test]
    fn test_format_state_running() {
        let state = format_state(JobState::Running);
        assert!(state.contains("Running") || state.contains("running"));
    }

    #[test]
    fn test_format_state_completed() {
        let state = format_state(JobState::Completed);
        assert!(state.contains("Completed") || state.contains("completed") || state.contains("Done"));
    }

    #[test]
    fn test_format_state_failed() {
        let state = format_state(JobState::Failed);
        assert!(state.contains("Failed") || state.contains("failed") || state.contains("Error"));
    }

    #[test]
    fn test_jobs_dir_returns_valid_path() {
        let path = jobs_dir().unwrap();
        assert!(path.to_string_lossy().contains(".spn/jobs"));
    }
}
```

**Step 5.2: Run tests to verify they pass**

Run: `cargo test -p spn-cli jobs::tests`
Expected: 8 tests pass (or adjust based on actual function signatures)

**Step 5.3: Commit**

```bash
git add crates/spn/src/commands/jobs.rs
git commit -m "test(jobs): add tests for helper functions

Add 8 tests covering:
- truncate() with short, long, exact length strings
- format_state() for all JobState variants
- jobs_dir() returns valid path

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Task 6: Fix Documentation - Remove spn daemon logs Reference

**Files:**
- Modify: `VERIFICATION.md:176`
- Modify: `docs/plans/2026-03-08-mcp-auto-sync-design.md:665`
- Modify: `docs/plans/daemon-registry-ux-roadmap.md:605`
- Modify: `docs/plans/2026-03-08-spn-v015-v018-roadmap.md:123`
- Modify: `docs/plans/2026-03-04-spn-daemon-architecture.md:730`

**Step 6.1: Fix VERIFICATION.md**

Find line 176 and update the table row:

```markdown
| `spn daemon logs` | ❌ | Not implemented - use `spn mcp logs` instead |
```

**Step 6.2: Fix planning docs**

For each planning doc, either:
- Remove the `spn daemon logs` reference, OR
- Mark it as "TODO" or "Not Yet Implemented"

Search for "daemon logs" in each file and update accordingly.

**Step 6.3: Commit**

```bash
git add VERIFICATION.md docs/plans/
git commit -m "docs: fix incorrect spn daemon logs references

spn daemon logs does not exist. Available daemon commands:
start, stop, status, restart, install, uninstall, mcp

Update VERIFICATION.md and planning docs to reflect reality.

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Task 7: Final Verification

**Step 7.1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass (1460+ tests)

**Step 7.2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings

**Step 7.3: Verify test count increased**

Run: `cargo test --workspace -- --list 2>&1 | grep -c "test$"`
Expected: Count should be ~1480+ (was 1460, added ~20 new tests)

**Step 7.4: Push all commits**

Run: `git push`

---

## Summary

| Task | Files Modified | Tests Added | Commits |
|------|----------------|-------------|---------|
| 1. MCP stdout fix | 1 | 0 | 1 |
| 2. .expect() fix | 1 | 0 | 1 |
| 3. Protocol tests | 1 | 8 | 1 |
| 4. Tools tests | 1 | 10 | 1 |
| 5. Jobs tests | 1 | 8 | 1 |
| 6. Doc fixes | 5 | 0 | 1 |
| 7. Verification | 0 | 0 | 0 |
| **Total** | **10** | **26** | **6** |

---

## Execution Choice

Plan complete and saved to `docs/plans/2026-03-09-critical-fixes-and-tests.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
