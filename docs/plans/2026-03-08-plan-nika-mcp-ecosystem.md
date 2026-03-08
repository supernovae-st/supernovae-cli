# Plan: Unified MCP Ecosystem & Auto-Install

**Created**: 2026-03-08
**Status**: Ready for execution
**Effort**: ~8 hours total (can be split into phases)
**Target**: v0.16.0 (Phase 1), v0.17.0 (Phase 2)

---

## Architecture Decision

After brainstorming analysis, **Option 3 (MCP) is cleaner than subprocess**:

| Criterion | Subprocess | MCP Server |
|-----------|------------|------------|
| Ecosystem coherence | ⚠️ Only subprocess | ✅ Everything is MCP |
| Startup speed | ❌ Fork each time | ✅ Daemon in memory |
| Communication | ⚠️ stdout parsing | ✅ JSON-RPC typed |
| Error handling | ❌ Exit codes | ✅ Error objects |
| State | ❌ Stateless | ✅ Sessions/cache |
| Monitoring | ❌ None | ✅ Centralized logs |

**Decision**: Create `nika-mcp` server so spn communicates with Nika via MCP protocol.

---

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  UNIFIED MCP ECOSYSTEM (v0.17.0 target)                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│                    ┌──────────────────────────────────────┐                     │
│                    │           spn daemon                 │                     │
│                    │  (orchestrator, job scheduler)       │                     │
│                    └──────────────┬───────────────────────┘                     │
│                                   │ MCP Protocol                                │
│              ┌────────────────────┼────────────────────┬───────────────────┐    │
│              │                    │                    │                   │    │
│              ▼                    ▼                    ▼                   ▼    │
│   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ ┌───────────┐  │
│   │  nika-mcp       │  │  novanet-mcp    │  │  spn-mcp        │ │ external  │  │
│   │  (NEW)          │  │  (existing)     │  │  (existing)     │ │ MCPs      │  │
│   ├─────────────────┤  ├─────────────────┤  ├─────────────────┤ └───────────┘  │
│   │ • nika_infer    │  │ • novanet_query │  │ • Dynamic REST  │                │
│   │ • nika_workflow │  │ • novanet_*     │  │   wrappers      │                │
│   │ • nika_chat     │  │   (14 tools)    │  │                 │                │
│   │ • nika_status   │  │                 │  │                 │                │
│   └─────────────────┘  └─────────────────┘  └─────────────────┘                │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Auto-Install (v0.16.0)

### Goal

When user runs `spn setup` or uses Nika/NovaNet features, auto-install missing tools.

### Step 1.1: Detection Logic

**File:** `crates/spn/src/interop/detect.rs` (NEW)

```rust
use std::process::Command;
use which::which;

#[derive(Debug, Clone, PartialEq)]
pub enum InstallStatus {
    Installed { version: String, path: PathBuf },
    NotInstalled,
    Outdated { current: String, latest: String },
}

pub struct EcosystemTools {
    pub nika: InstallStatus,
    pub novanet: InstallStatus,
}

impl EcosystemTools {
    pub fn detect() -> Self {
        Self {
            nika: detect_nika(),
            novanet: detect_novanet(),
        }
    }
}

fn detect_nika() -> InstallStatus {
    match which("nika") {
        Ok(path) => {
            let version = Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            InstallStatus::Installed { version, path }
        }
        Err(_) => InstallStatus::NotInstalled,
    }
}

fn detect_novanet() -> InstallStatus {
    match which("novanet") {
        Ok(path) => {
            let version = Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            InstallStatus::Installed { version, path }
        }
        Err(_) => InstallStatus::NotInstalled,
    }
}
```

### Step 1.2: On-Demand Install Prompts

**File:** `crates/spn/src/commands/setup.rs`

```rust
pub async fn run_setup() -> Result<(), SpnError> {
    let tools = EcosystemTools::detect();

    // Show detection results
    println!("\n{}", "Ecosystem Status:".bold());

    match &tools.nika {
        InstallStatus::Installed { version, .. } => {
            println!("  {} Nika {} installed", "✅".green(), version);
        }
        InstallStatus::NotInstalled => {
            println!("  {} Nika not installed", "⚠️".yellow());
        }
        InstallStatus::Outdated { current, latest } => {
            println!("  {} Nika {} (update available: {})", "🔄".blue(), current, latest);
        }
    }

    // Similar for novanet...

    // Prompt for missing tools
    if tools.nika == InstallStatus::NotInstalled {
        if Confirm::new()
            .with_prompt("Install Nika workflow engine?")
            .default(true)
            .interact()? {
            install_nika().await?;
        }
    }

    Ok(())
}

async fn install_nika() -> Result<(), SpnError> {
    println!("\n{}", "Installing Nika...".bold());

    // Prefer cargo install if rustup available
    if which("cargo").is_ok() {
        let status = Command::new("cargo")
            .args(["install", "nika", "--locked"])
            .status()?;

        if !status.success() {
            return Err(SpnError::InstallFailed("nika".into()));
        }
    } else {
        // Fallback to binary download
        download_binary("nika").await?;
    }

    println!("  {} Nika installed successfully", "✅".green());
    Ok(())
}
```

### Step 1.3: Lazy Install on First Use

**File:** `crates/spn/src/commands/nk.rs`

```rust
pub async fn run_nk(args: &[String]) -> Result<(), SpnError> {
    let tools = EcosystemTools::detect();

    if tools.nika == InstallStatus::NotInstalled {
        eprintln!("{}", "Nika is not installed.".yellow());

        if atty::is(atty::Stream::Stdin) {
            // Interactive terminal - prompt
            if Confirm::new()
                .with_prompt("Install Nika now?")
                .default(true)
                .interact()? {
                install_nika().await?;
            } else {
                return Err(SpnError::MissingTool("nika".into()));
            }
        } else {
            // Non-interactive - error
            return Err(SpnError::MissingTool("nika".into()));
        }
    }

    // Now run nika
    proxy_to_nika(args)
}
```

---

## Phase 2: nika-mcp Server (v0.17.0)

### Goal

Nika becomes both MCP client (consumes NovaNet) AND MCP server (exposes workflows).

### Confirmed MVP Tools (3 tools)

| Tool | Purpose | Complexity |
|------|---------|------------|
| `nika_infer` | Single LLM call | Simple |
| `nika_run` | Execute workflow (file or inline) | Medium |
| `nika_status` | Health check and provider status | Simple |

**Deferred to v0.2.0:**
- `nika_chat` — Requires session management (not stable yet)
- `nika_agent` — Agentic loop complexity
- Individual verbs (`nika_exec`, `nika_fetch`, `nika_invoke`) — Use `nika_run --inline` instead

### Step 2.1: nika-mcp Crate

**File:** `nika/crates/nika-mcp/Cargo.toml` (NEW)

```toml
[package]
name = "nika-mcp"
version = "0.1.0"
edition = "2021"
description = "MCP server exposing Nika workflow engine capabilities"
license = "AGPL-3.0-or-later"
repository = "https://github.com/supernovae-st/nika"

[[bin]]
name = "nika-mcp"
path = "src/main.rs"

[dependencies]
# Nika core
nika-core = { path = "../nika-core" }

# MCP Protocol (same version as novanet-mcp for compatibility)
rmcp = { version = "0.16", features = ["server", "transport-io"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"  # JSON Schema generation (like novanet-mcp)

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
thiserror = "2"
anyhow = "1"
```

### Step 2.2: MCP Tools Definition (Confirmed Specs)

**File:** `nika/crates/nika-mcp/src/tools/mod.rs`

```rust
//! MCP Tools module for nika-mcp
//!
//! MVP v0.1.0: nika_infer, nika_run, nika_status
//! v0.2.0: nika_chat, nika_agent

pub mod infer;
pub mod run;
pub mod status;

pub use infer::{InferParams, InferResult};
pub use run::{RunParams, RunResult, StepResult};
pub use status::{StatusParams, StatusResult};
```

**File:** `nika/crates/nika-mcp/src/tools/infer.rs`

```rust
//! nika_infer tool - Single LLM inference call

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for nika_infer tool
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InferParams {
    /// The prompt to send to the LLM
    pub prompt: String,

    /// LLM provider (default: anthropic)
    #[serde(default)]
    pub provider: Option<String>,

    /// Model name (provider-specific, uses default if omitted)
    #[serde(default)]
    pub model: Option<String>,

    /// System prompt (optional)
    #[serde(default)]
    pub system: Option<String>,

    /// Sampling temperature 0.0-2.0 (default: 0.7)
    #[serde(default)]
    pub temperature: Option<f32>,
}

/// Result from nika_infer tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InferResult {
    /// LLM response text
    pub response: String,

    /// Model used for inference
    pub model: String,

    /// Provider used
    pub provider: String,

    /// Input tokens consumed
    pub tokens_input: usize,

    /// Output tokens generated
    pub tokens_output: usize,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}
```

**File:** `nika/crates/nika-mcp/src/tools/run.rs`

```rust
//! nika_run tool - Execute workflow file or inline YAML

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for nika_run tool
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RunParams {
    /// Path to .nika.yaml workflow file (mutually exclusive with inline)
    #[serde(default)]
    pub path: Option<String>,

    /// Inline YAML workflow (mutually exclusive with path)
    #[serde(default)]
    pub inline: Option<String>,

    /// Variables passed to workflow ($var syntax)
    #[serde(default)]
    pub variables: Option<serde_json::Map<String, serde_json::Value>>,

    /// Workflow timeout in seconds (default: 300)
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Result from nika_run tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RunResult {
    /// Final workflow output
    pub output: String,

    /// Number of steps executed
    pub steps_executed: usize,

    /// Number of steps skipped (conditional)
    pub steps_skipped: usize,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Detailed results per step (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_results: Option<Vec<StepResult>>,
}

/// Result for a single workflow step
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct StepResult {
    /// Step name/identifier
    pub step: String,

    /// Execution status
    pub status: String,

    /// Tokens consumed (if LLM step)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<usize>,

    /// Step execution time in milliseconds
    pub execution_time_ms: u64,
}
```

**File:** `nika/crates/nika-mcp/src/tools/status.rs`

```rust
//! nika_status tool - Runtime status and health check

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameters for nika_status tool
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct StatusParams {
    /// Include detailed provider/MCP info (default: false)
    #[serde(default)]
    pub verbose: Option<bool>,
}

/// Result from nika_status tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct StatusResult {
    /// Nika version
    pub version: String,

    /// Overall status
    pub status: String,

    /// Provider configuration status
    pub providers: HashMap<String, String>,

    /// Connected MCP servers
    pub mcp_servers: Vec<String>,

    /// Number of active workflows
    pub active_workflows: usize,

    /// Uptime in seconds (if daemon mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<u64>,
}
```

### Step 2.3: MCP Server Handler (following novanet-mcp pattern)

**File:** `nika/crates/nika-mcp/src/server/handler.rs`

```rust
//! MCP Server Handler for nika-mcp
//!
//! Implements rmcp::ServerHandler using macro-based routing (like novanet-mcp).

use crate::error::Error;
use crate::server::State;
use crate::tools::{InferParams, RunParams, StatusParams};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use std::sync::Arc;
use tracing::instrument;

/// Nika MCP Handler with tool routing
#[derive(Clone)]
pub struct NikaHandler {
    state: Arc<State>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NikaHandler {
    /// Create a new handler with the given state
    pub fn new(state: Arc<State>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    /// Single LLM inference call
    #[tool(
        name = "nika_infer",
        description = "🚀 QUICK INFERENCE - Single LLM call with prompt. Returns model response. Supports all providers (anthropic, openai, ollama, mistral, groq, deepseek, gemini). Use for simple prompts. For multi-step workflows, use nika_run instead."
    )]
    #[instrument(name = "nika_infer", skip(self), fields(provider, model))]
    async fn nika_infer(
        &self,
        params: Parameters<InferParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = crate::tools::infer::execute(&self.state, params.0)
            .await
            .map_err(McpError::from)?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Execute workflow file or inline YAML
    #[tool(
        name = "nika_run",
        description = "🔄 EXECUTE WORKFLOW - Run a Nika workflow file or inline YAML. Supports all 5 verbs (infer, exec, fetch, invoke, agent) with DAG execution. For single LLM calls, use nika_infer instead. Pass variables to parameterize workflows."
    )]
    #[instrument(name = "nika_run", skip(self), fields(path, inline))]
    async fn nika_run(
        &self,
        params: Parameters<RunParams>,
    ) -> Result<CallToolResult, McpError> {
        // Validate: path XOR inline required
        let p = &params.0;
        if p.path.is_none() && p.inline.is_none() {
            return Err(McpError::invalid_params(
                "Either 'path' or 'inline' is required",
                None,
            ));
        }
        if p.path.is_some() && p.inline.is_some() {
            return Err(McpError::invalid_params(
                "Cannot specify both 'path' and 'inline'",
                None,
            ));
        }

        let result = crate::tools::run::execute(&self.state, params.0)
            .await
            .map_err(McpError::from)?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Runtime status and health check
    #[tool(
        name = "nika_status",
        description = "📊 STATUS CHECK - Get Nika runtime status. Shows configured providers, active workflows, MCP connections, and system health. Use to verify Nika is properly configured before running workflows."
    )]
    #[instrument(name = "nika_status", skip(self))]
    async fn nika_status(
        &self,
        params: Parameters<StatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = crate::tools::status::execute(&self.state, params.0)
            .await
            .map_err(McpError::from)?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

impl ServerHandler for NikaHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "nika-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn get_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: Some(rmcp::model::ToolsCapability {
                list_changed: Some(false),
            }),
            ..Default::default()
        }
    }
}
```

**File:** `nika/crates/nika-mcp/src/main.rs`

```rust
//! nika-mcp - MCP server exposing Nika workflow engine
//!
//! Usage:
//!   nika-mcp              # Start server over stdio
//!   nika-mcp --version    # Show version

use clap::Parser;
use nika_mcp::server::{NikaHandler, State};
use rmcp::ServiceExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(name = "nika-mcp", version, about = "MCP server for Nika workflow engine")]
struct Args {
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    fmt().with_env_filter(filter).with_writer(std::io::stderr).init();

    tracing::info!("Starting nika-mcp v{}", env!("CARGO_PKG_VERSION"));

    // Initialize state
    let state = State::new().await?;
    let handler = NikaHandler::new(state);

    // Serve over stdio
    let service = handler.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}
```

### Step 2.4: spn Integration

**File:** `crates/spn/src/commands/infer.rs` (NEW)

```rust
//! spn infer - Routes to nika_infer MCP tool

use crate::error::SpnError;
use serde_json::json;

/// Simple inference command that routes to nika-mcp
pub async fn run_infer(
    prompt: &str,
    provider: Option<&str>,
    model: Option<&str>,
    system: Option<&str>,
) -> Result<(), SpnError> {
    // Ensure nika-mcp is running (auto-start if needed)
    ensure_nika_mcp_running().await?;

    // Connect to nika-mcp via MCP
    let client = McpClient::connect("nika-mcp").await?;

    let mut params = json!({ "prompt": prompt });
    if let Some(p) = provider {
        params["provider"] = json!(p);
    }
    if let Some(m) = model {
        params["model"] = json!(m);
    }
    if let Some(s) = system {
        params["system"] = json!(s);
    }

    let result = client.call_tool("nika_infer", params).await?;

    // Print result (extract response from InferResult)
    if let Some(content) = result.get("content") {
        for item in content.as_array().unwrap_or(&vec![]) {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                // Parse InferResult JSON and print just the response
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    if let Some(response) = parsed.get("response").and_then(|r| r.as_str()) {
                        println!("{}", response);
                        return Ok(());
                    }
                }
                // Fallback: print raw
                println!("{}", text);
            }
        }
    }

    Ok(())
}

async fn ensure_nika_mcp_running() -> Result<(), SpnError> {
    // Check if nika-mcp process is running
    // If not, start it in background
    // This is similar to how spn daemon auto-starts
    todo!("Implement nika-mcp auto-start")
}
```

**File:** `crates/spn/src/main.rs` (additions)

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...

    /// Run LLM inference (routes to Nika via MCP)
    Infer {
        /// Prompt to send to the LLM
        prompt: String,

        /// LLM provider (anthropic, openai, ollama, etc.)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model name (provider-specific)
        #[arg(short, long)]
        model: Option<String>,

        /// System prompt
        #[arg(short, long)]
        system: Option<String>,
    },

    /// Run a Nika workflow (routes to Nika via MCP)
    Run {
        /// Path to .nika.yaml workflow file
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Inline YAML workflow
        #[arg(short, long)]
        inline: Option<String>,

        /// Variables as JSON (e.g., '{"locale": "fr-FR"}')
        #[arg(short, long)]
        vars: Option<String>,
    },
}
```

---

## Phase 3: MCP Config in spn (v0.17.0)

### Auto-register nika-mcp

**File:** `~/.spn/mcp.yaml`

```yaml
servers:
  # Auto-added when nika is installed
  nika:
    command: nika-mcp
    args: []
    env: {}

  # Auto-added when novanet is installed
  novanet:
    command: novanet-mcp
    args: []
    env:
      NEO4J_URI: bolt://localhost:7687
```

**File:** `crates/spn/src/interop/mcp_registry.rs`

```rust
pub fn register_ecosystem_mcps(tools: &EcosystemTools) -> Result<(), Error> {
    let mut config = McpConfig::load()?;

    // Register nika-mcp if nika installed
    if matches!(tools.nika, InstallStatus::Installed { .. }) {
        if !config.servers.contains_key("nika") {
            config.servers.insert("nika".into(), McpServer {
                command: "nika-mcp".into(),
                args: vec![],
                env: HashMap::new(),
            });
        }
    }

    // Register novanet-mcp if novanet installed
    if matches!(tools.novanet, InstallStatus::Installed { .. }) {
        if !config.servers.contains_key("novanet") {
            config.servers.insert("novanet".into(), McpServer {
                command: "novanet-mcp".into(),
                args: vec![],
                env: HashMap::new(),
            });
        }
    }

    config.save()?;
    Ok(())
}
```

---

## Verification Checklist

### Phase 1 (Auto-Install)
- [ ] `spn setup` detects missing Nika/NovaNet
- [ ] Interactive prompt offers installation
- [ ] `cargo install` works when cargo available
- [ ] Binary download fallback works
- [ ] `spn nk` prompts for install if missing
- [ ] Non-interactive mode errors cleanly

### Phase 2 (nika-mcp)
- [ ] `nika-mcp` binary builds and runs
- [ ] MCP initialize handshake works
- [ ] `nika_infer` tool works (single prompt → response)
- [ ] `nika_run` tool works with path
- [ ] `nika_run` tool works with inline YAML
- [ ] `nika_run` validates path XOR inline
- [ ] `nika_status` tool returns provider info
- [ ] Error handling follows MCP spec
- [ ] Tracing instrumentation works

### Phase 3 (Integration)
- [ ] `spn infer "prompt"` routes to nika-mcp
- [ ] `spn run -p workflow.nika.yaml` routes to nika-mcp
- [ ] nika-mcp auto-registered in mcp.yaml
- [ ] spn daemon discovers and uses nika-mcp
- [ ] Auto-start nika-mcp if not running

---

## Commit Strategy

```bash
# Phase 1 (spn repo)
git commit -m "feat(interop): add ecosystem tool detection

- Detect nika and novanet installations
- Version checking for updates
- Path resolution via which crate

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

git commit -m "feat(setup): add auto-install for Nika and NovaNet

- Interactive prompts during setup wizard
- cargo install preferred, binary fallback
- On-demand install in spn nk/nv commands

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

# Phase 2 (nika repo)
git commit -m "feat(nika-mcp): create MCP server crate

- 3 MVP tools: nika_infer, nika_run, nika_status
- JSON-RPC over stdio transport (rmcp 0.16)
- JsonSchema via schemars (like novanet-mcp)
- Tracing instrumentation

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

git commit -m "feat(nika-mcp): implement tool handlers

- nika_infer: single LLM inference with provider/model options
- nika_run: workflow execution (path or inline YAML)
- nika_status: runtime health check and provider status

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

# Phase 3 (spn repo)
git commit -m "feat(cli): add spn infer command

- Route to nika_infer via MCP protocol
- Auto-start nika-mcp if not running
- Support --provider, --model, --system flags

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

git commit -m "feat(cli): add spn run command

- Route to nika_run via MCP protocol
- Support --path and --inline options
- Variable passing via --vars JSON

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Benefits

| Before | After |
|--------|-------|
| `spn nk run --inline "..."` subprocess | `spn infer "..."` via MCP |
| Each call forks nika binary | nika-mcp daemon stays in memory |
| stdout/stderr parsing | Typed JSON-RPC with schemas |
| No state between calls | Persistent connections |
| Manual ecosystem install | Auto-detected and installed |
| 3 separate tools (spn, nika, novanet) | Unified MCP ecosystem |
| Different protocols | Everything speaks MCP |
