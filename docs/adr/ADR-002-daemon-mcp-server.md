# ADR-002: Daemon as MCP Server (Bridge Architecture)

**Status:** Accepted
**Date:** 2026-03-08
**Authors:** Thibaut, Claude
**Supersedes:** None
**Related:** ADR-001

---

## Context

The spn-daemon currently serves as a credential cache via Unix socket IPC:

```
Current:
Nika ──► spn-client (Rust crate) ──► Unix socket ──► spn-daemon ──► Keychain
```

This works but has limitations:
1. **Tight coupling** — Nika must link spn-client crate
2. **Non-standard** — Custom IPC protocol, not discoverable
3. **Limited scope** — Only secrets, not models or MCP health

We want Nika to interact with spn via standard MCP protocol, enabling:
- Loose coupling (no library linking)
- Discoverability (`nika mcp list` shows spn tools)
- Extensibility (easy to add new tools)

---

## Decision

We add an **MCP bridge** as a separate binary that connects to the existing daemon:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ARCHITECTURE: MCP Bridge (Option B)                                            │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn CLI ─────────► Unix Socket ─────────► spn-daemon (unchanged)              │
│                                                  ▲                              │
│                                                  │                              │
│  Nika ─────────► MCP (stdio) ─────────► spn-mcp-server ──────┘                 │
│                                          (new binary)                           │
│                                          (~300 lines)                           │
│                                                                                 │
│  Config in ~/.spn/mcp.yaml:                                                    │
│  servers:                                                                       │
│    spn:                                                                         │
│      command: spn-mcp-server                                                   │
│      transport: stdio                                                           │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Why Bridge (Option B) vs Embedded (Option A)?

| Aspect | Option A (Embedded) | Option B (Bridge) |
|--------|---------------------|-------------------|
| Daemon changes | Major | None |
| Protocol mixing | 2 protocols in 1 process | Clean separation |
| Complexity | Higher | Lower |
| Maintenance | Harder | Easier |
| Rollback | Hard | Easy (just remove binary) |

**We chose Option B** because it's additive and low-risk.

---

## MCP Tools Exposed

The spn-mcp-server exposes these tools:

### 1. `spn_get_secret`
```json
{
  "name": "spn_get_secret",
  "description": "Get API key for a provider from spn credential store",
  "inputSchema": {
    "type": "object",
    "properties": {
      "provider": {
        "type": "string",
        "description": "Provider name (e.g., anthropic, openai, neo4j)"
      }
    },
    "required": ["provider"]
  }
}
```

### 2. `spn_list_providers`
```json
{
  "name": "spn_list_providers",
  "description": "List all configured providers with status",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### 3. `spn_model_status`
```json
{
  "name": "spn_model_status",
  "description": "Get status of local models (loaded, VRAM usage)",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### 4. `spn_model_run`
```json
{
  "name": "spn_model_run",
  "description": "Run inference on a local model",
  "inputSchema": {
    "type": "object",
    "properties": {
      "model": { "type": "string" },
      "prompt": { "type": "string" },
      "temperature": { "type": "number", "default": 0.7 },
      "max_tokens": { "type": "integer", "default": 1024 }
    },
    "required": ["model", "prompt"]
  }
}
```

### 5. `spn_mcp_health`
```json
{
  "name": "spn_mcp_health",
  "description": "Check health of MCP servers",
  "inputSchema": {
    "type": "object",
    "properties": {
      "server": {
        "type": "string",
        "description": "Server name (optional, checks all if omitted)"
      }
    }
  }
}
```

### 6. `spn_schedule_list`
```json
{
  "name": "spn_schedule_list",
  "description": "List scheduled jobs",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

---

## Implementation

### New Crate: `spn-mcp-server`

```
crates/
├── spn-mcp-server/          # NEW
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Entry point, MCP server setup
│       ├── tools/           # Tool implementations
│       │   ├── mod.rs
│       │   ├── secrets.rs   # spn_get_secret, spn_list_providers
│       │   ├── models.rs    # spn_model_status, spn_model_run
│       │   └── mcp.rs       # spn_mcp_health
│       └── client.rs        # Daemon IPC client (reuse spn-client)
```

### Cargo.toml

```toml
[package]
name = "spn-mcp-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "spn-mcp-server"
path = "src/main.rs"

[dependencies]
spn-client = { path = "../spn-client" }
mcp-server = "0.1"  # MCP SDK
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
```

### main.rs Sketch

```rust
use mcp_server::{Server, Tool, ToolResult};
use spn_client::SpnClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = SpnClient::connect().await?;

    let server = Server::builder()
        .name("spn")
        .version(env!("CARGO_PKG_VERSION"))
        .tool(secrets::get_secret_tool())
        .tool(secrets::list_providers_tool())
        .tool(models::status_tool())
        .tool(models::run_tool())
        .tool(mcp::health_tool())
        .build();

    server.serve_stdio().await?;
    Ok(())
}
```

---

## Consequences

### Positive

1. **Loose coupling** — Nika doesn't need spn-client crate
2. **Discoverability** — `nika mcp list` shows spn tools
3. **Extensibility** — Easy to add new tools
4. **Standard protocol** — MCP is an open standard
5. **Low risk** — Additive change, easy rollback

### Negative

1. **Latency** — MCP overhead (~5-20ms per call)
2. **New binary** — One more thing to install
3. **Process spawn** — MCP servers typically spawned per session

### Mitigations

- **Latency**: Keep Unix socket for hot paths, MCP for tools
- **Binary**: Bundle in same release, single `cargo install`
- **Spawn**: spn-mcp-server connects to running daemon, fast startup

---

## Testing Strategy

### Unit Tests
- Tool input validation
- Response formatting
- Error handling

### Integration Tests
- spn-mcp-server ↔ spn-daemon communication
- MCP protocol compliance

### End-to-End Tests
- Nika workflow invoking spn_get_secret
- Error propagation

---

## Rollout Plan

### Phase 1: Implement (v0.16.0)
- Build spn-mcp-server binary
- Implement 6 tools
- Unit tests

### Phase 2: Integrate (v0.16.0)
- Add to mcp.yaml as optional server
- Document usage
- Integration tests

### Phase 3: Promote (v0.17.0)
- Enable by default in new installs
- Migration guide for existing users
- Deprecate direct spn-client usage in Nika

---

## Alternatives Considered

### Alternative A: Embed MCP in Daemon
Add MCP protocol handling directly to spn-daemon.

**Rejected because:**
- Mixes two protocols in one process
- Harder to test and maintain
- Larger daemon binary

### Alternative B: No MCP, Keep spn-client
Keep current architecture with library linking.

**Rejected because:**
- Tight coupling
- Not discoverable
- Non-standard

---

## References

- [MCP Specification](https://modelcontextprotocol.io/spec)
- [ADR-001: Ecosystem Role Distribution](./ADR-001-ecosystem-role-distribution.md)

---

## Changelog

| Date | Change |
|------|--------|
| 2026-03-08 | Initial version |
