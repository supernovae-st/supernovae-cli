# Master Execution Plan: spn v0.9.0 → v0.12.0

**Date:** 2026-03-04
**Status:** Ready for Execution
**Authors:** Thibaut, Claude, Nika

---

## Executive Summary

This plan consolidates all previous designs into an actionable execution roadmap for evolving `spn` from a package manager into a **unified AI infrastructure orchestrator**.

### Vision Recap

```
spn = npm pour l'IA (package manager + infrastructure)
nika = Node.js pour l'IA (runtime)
novanet = Knowledge graph (brain)
```

### Key Deliverables

| Version | Milestone | Core Feature |
|---------|-----------|--------------|
| v0.9.0 | Daemon Foundation | spn-client crate + Unix socket IPC |
| v0.10.0 | Secret Unification | Nika uses spn daemon for secrets |
| v0.11.0 | Process Manager | MCP server + Ollama management |
| v0.12.0 | MCP Gateway | Single endpoint for all MCP tools |

---

## Current State Analysis

### What Exists (v0.8.1)

```
spn v0.8.1
├── Package management (add, install, search, publish)
├── Secret management (keyring, env, .env)
├── MCP config (~/.spn/mcp.yaml)
├── Interop (nika proxy, novanet proxy, npm proxy)
└── 158 tests passing
```

### What's Missing

| Gap | Impact | Solution |
|-----|--------|----------|
| Keychain popup spam | UX nightmare on macOS | spn daemon (sole accessor) |
| Secret duplication | Nika has own config.toml | Nika uses spn-client |
| No process management | Manual MCP server start | spn service commands |
| Fragmented MCP | Multiple connections needed | MCP gateway aggregation |

---

## Phase 1: Workspace + spn-client (v0.9.0)

**Duration:** 3-5 days
**Goal:** Create foundation for daemon architecture

### 1.1 Workspace Restructure

Convert single crate to workspace:

```
supernovae-cli/
├── Cargo.toml              # [workspace]
├── crates/
│   ├── spn/                # CLI binary (current src/)
│   │   ├── Cargo.toml
│   │   └── src/
│   └── spn-client/         # Library crate
│       ├── Cargo.toml
│       └── src/
├── docs/
└── README.md
```

**Tasks:**

- [ ] Create workspace Cargo.toml
- [ ] Move src/ to crates/spn/src/
- [ ] Create crates/spn/Cargo.toml (copy dependencies)
- [ ] Create crates/spn-client/ structure
- [ ] Verify `cargo build` works
- [ ] Verify `cargo test` passes
- [ ] Update CI/CD for workspace

### 1.2 spn-client Crate

Minimal library for daemon communication:

```rust
// spn-client/src/lib.rs
pub struct SpnClient { ... }

impl SpnClient {
    pub async fn connect() -> Result<Self>;
    pub async fn get_secret(&self, provider: &str) -> Result<SecretString>;
    pub async fn has_secret(&self, provider: &str) -> Result<bool>;
    pub async fn list_providers(&self) -> Result<Vec<String>>;
}
```

**Dependencies (minimal):**

```toml
[dependencies]
tokio = { version = "1", features = ["net", "io-util", "sync"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
secrecy = "0.10"
thiserror = "2"
dirs = "5"
tracing = "0.1"
```

**Tasks:**

- [ ] Create Cargo.toml with minimal deps
- [ ] Implement protocol types (Request, Response)
- [ ] Implement SpnClient struct
- [ ] Implement connect() with fallback logic
- [ ] Implement get_secret(), has_secret(), list_providers()
- [ ] Add error types
- [ ] Write unit tests
- [ ] Write integration tests (with mock server)

### 1.3 Protocol Specification

IPC protocol via Unix socket:

```
Socket: ~/.spn/daemon.sock
Permissions: 0600
Format: length-prefixed JSON

Request:
{ "cmd": "GET_SECRET", "provider": "anthropic" }

Response:
{ "ok": true, "secret": "sk-ant-..." }
{ "ok": false, "error": "NotFound" }
```

**Commands:**

| Command | Request | Response |
|---------|---------|----------|
| PING | `{}` | `{"ok": true, "version": "0.9.0"}` |
| GET_SECRET | `{"provider": "..."}` | `{"ok": true, "secret": "..."}` |
| HAS_SECRET | `{"provider": "..."}` | `{"ok": true, "exists": true}` |
| LIST_PROVIDERS | `{}` | `{"ok": true, "providers": [...]}` |

---

## Phase 2: Daemon Core (v0.9.0 continued)

**Duration:** 5-7 days
**Goal:** Implement daemon server in spn

### 2.1 Daemon Module Structure

```
crates/spn/src/
├── daemon/
│   ├── mod.rs          # Public API
│   ├── server.rs       # Unix socket listener
│   ├── handler.rs      # Request handlers
│   ├── secrets.rs      # Secret Manager (cached keyring)
│   ├── socket.rs       # Socket utils, SO_PEERCRED
│   └── error.rs        # DaemonError
└── commands/
    └── daemon.rs       # CLI: spn daemon start/stop/status
```

### 2.2 Secret Manager

```rust
pub struct SecretManager {
    cache: Arc<RwLock<FxHashMap<String, LockedString>>>,
    keyring: SpnKeyring,
}

impl SecretManager {
    pub async fn preload_all(&self) -> Result<()>;
    pub fn get_cached(&self, provider: &str) -> Option<SecretString>;
    pub fn has_cached(&self, provider: &str) -> bool;
    pub fn list_cached(&self) -> Vec<String>;
}
```

**Tasks:**

- [ ] Create daemon module structure
- [ ] Implement SecretManager with mlock cache
- [ ] Implement Unix socket server with SO_PEERCRED
- [ ] Implement request handler (dispatch by cmd)
- [ ] Implement PID file with flock
- [ ] Add signal handling (SIGTERM, SIGINT)
- [ ] Add daemon CLI commands
- [ ] Write integration tests

### 2.3 Security Hardening (P0)

Before release:

- [ ] SO_PEERCRED validation on every connection
- [ ] Socket path ownership verification
- [ ] PID file atomic creation with flock
- [ ] Stale socket cleanup on start

---

## Phase 3: Nika Migration (v0.10.0)

**Duration:** 1 week
**Goal:** Remove secret duplication from Nika

### 3.1 Files to Modify in Nika

| File | Change | LOC |
|------|--------|-----|
| `config.rs` | Remove ApiKeys struct | -100 |
| `provider/rig.rs` | from_env → with_api_key | ~50 |
| `tui/state.rs` | Remove key fields | -80 |
| `tui/widgets/provider_modal/` | Use spn-client | ~100 |
| Tests | Update/remove api_key tests | -60 |

### 3.2 Integration Pattern

```rust
// In Nika
use spn_client::SpnClient;

async fn get_provider_key(provider: &str) -> Result<SecretString> {
    let client = SpnClient::connect().await?;
    client.get_secret(provider).await
}

// Then use with rig
let key = get_provider_key("anthropic").await?;
let client = anthropic::Client::with_api_key(key.expose_secret());
```

### 3.3 Fallback Behavior

If daemon not running:

```rust
impl SpnClient {
    pub async fn connect_with_fallback() -> Result<Self> {
        match Self::connect().await {
            Ok(client) => Ok(client),
            Err(_) => {
                tracing::warn!("spn daemon not running, using env vars");
                Ok(Self::env_fallback())
            }
        }
    }
}
```

**Tasks:**

- [ ] Add spn-client dependency to Nika
- [ ] Modify config.rs (remove ApiKeys)
- [ ] Modify provider/rig.rs (use spn-client)
- [ ] Modify TUI state
- [ ] Modify provider_modal widgets
- [ ] Update error messages ("Run: spn daemon start")
- [ ] Update tests
- [ ] Write migration guide

---

## Phase 4: Process Manager (v0.11.0)

**Duration:** 1-2 weeks
**Goal:** Manage MCP servers and Ollama

### 4.1 New Components

```rust
pub struct ProcessManager {
    processes: Arc<RwLock<FxHashMap<String, ManagedProcess>>>,
    secrets: Arc<SecretManager>,
}

pub struct ManagedProcess {
    id: String,
    kind: ProcessKind,
    child: Child,
    started_at: Instant,
    config: ProcessConfig,
}

pub enum ProcessKind {
    Ollama,
    McpServer { name: String },
}
```

### 4.2 CLI Commands

```bash
spn service start ollama
spn service start neo4j-mcp
spn service stop <id>
spn service list
spn service logs <id>
```

### 4.3 Secure Spawning

```rust
async fn spawn_service(&self, config: ProcessConfig) -> Result<String> {
    let mut cmd = Command::new(&config.command);

    // Security: clear inherited env
    cmd.env_clear();

    // Inject only required secrets
    let env = self.secrets.build_env_for_process(&config.required_secrets);
    cmd.envs(env);

    // Non-secret env vars
    cmd.envs(&config.env);

    // Spawn and register
    let child = cmd.spawn()?;
    // ...
}
```

**Tasks:**

- [ ] Create ProcessManager
- [ ] Implement spawn with env_clear + injection
- [ ] Implement health supervision loop
- [ ] Create ServiceRegistry
- [ ] Add service CLI commands
- [ ] Implement Ollama integration
- [ ] Implement generic MCP server spawn
- [ ] Write tests

---

## Phase 5: MCP Gateway (v0.12.0)

**Duration:** 2-3 weeks
**Goal:** Single MCP endpoint aggregating all servers

### 5.1 Architecture

```
Claude Code / Nika
       │
       ▼
  spn MCP Gateway (:9999)
       │
   ┌───┴───┬───────┐
   ▼       ▼       ▼
neo4j   firecrawl  novanet
```

### 5.2 Tool Routing

```rust
pub struct McpGateway {
    routes: Arc<RwLock<FxHashMap<String, String>>>,
    servers: Arc<RwLock<FxHashMap<String, McpConnection>>>,
}

impl McpGateway {
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        let endpoint = self.routes.read().await.get(name)?;
        let conn = self.servers.read().await.get(&endpoint)?;
        conn.call_tool(name, args).await
    }
}
```

**Tasks:**

- [ ] Implement MCP protocol (JSON-RPC 2.0)
- [ ] Create McpGateway router
- [ ] Implement tool aggregation
- [ ] Add HTTP/SSE transport
- [ ] Modify Nika to use gateway
- [ ] Write tests

---

## Testing Strategy

### Unit Tests

Each component has isolated tests:

```rust
#[tokio::test]
async fn test_secret_manager_cache() {
    let manager = SecretManager::new_test();
    manager.set_cached("anthropic", "sk-test").await;
    assert!(manager.has_cached("anthropic"));
}
```

### Integration Tests

Full daemon workflow:

```rust
#[tokio::test]
async fn test_daemon_secret_flow() {
    let daemon = TestDaemon::start().await;
    let client = SpnClient::connect_to(&daemon.socket).await?;

    client.set_secret("test", "value").await?;
    let secret = client.get_secret("test").await?;
    assert_eq!(secret.expose_secret(), "value");

    daemon.shutdown().await;
}
```

### Security Tests

```rust
#[test]
fn test_secret_not_in_debug() {
    let secret = LockedString::new("sensitive".to_string())?;
    assert!(!format!("{:?}", secret).contains("sensitive"));
}

#[test]
fn test_socket_permissions() {
    create_socket("/tmp/test.sock")?;
    let mode = fs::metadata("/tmp/test.sock")?.permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}
```

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Workspace migration breaks CI | High | Test locally first, incremental commits |
| macOS Keychain behavior changes | Medium | Document minimum OS version |
| Nika migration breaks users | High | Graceful fallback to env vars |
| Process manager complexity | Medium | Start with basic spawn, add features incrementally |

---

## Success Criteria

### v0.9.0 Complete When:

- [ ] `cargo build` works in workspace
- [ ] `spn daemon start` launches and accepts connections
- [ ] `spn-client` can GET_SECRET from running daemon
- [ ] macOS Keychain popup appears only once per session
- [ ] All existing tests still pass

### v0.10.0 Complete When:

- [ ] Nika uses spn-client for secrets
- [ ] No ApiKeys in Nika config.toml
- [ ] Fallback to env vars works when daemon down

### v0.11.0 Complete When:

- [ ] `spn service start ollama` works
- [ ] `spn service start neo4j-mcp` spawns with injected secrets
- [ ] Health supervision restarts crashed services

### v0.12.0 Complete When:

- [ ] Single MCP endpoint aggregates all servers
- [ ] Nika connects to gateway only
- [ ] Tool discovery returns combined list

---

## Commit Strategy

Each logical change gets its own commit:

```
docs(plans): add master execution plan v0.9.0-v0.12.0
refactor(workspace): convert to Cargo workspace
feat(spn-client): add protocol types and error handling
feat(spn-client): implement SpnClient with Unix socket
feat(daemon): add SecretManager with mlock cache
feat(daemon): implement Unix socket server
feat(daemon): add CLI commands (start/stop/status)
test(daemon): add integration tests
```

---

## Next Steps

1. **Immediate:** Execute Phase 1.1 (Workspace Restructure)
2. **Today:** Complete spn-client basic implementation
3. **This week:** Finish daemon core and test
4. **Next week:** Start Nika migration

---

## Related Documents

- [spn Daemon Architecture](./2026-03-04-spn-daemon-architecture.md) - Full technical design
- [Secret Management Design](./2026-03-03-secret-management-design.md) - UX research
- [Vision Summary](./2026-03-02-VISION-SUMMARY.md) - High-level vision
