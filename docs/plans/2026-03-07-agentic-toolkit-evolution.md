# spn Evolution: The Agentic AI Toolkit

**Date**: 2026-03-07
**Status**: Approved
**Version**: 0.15.0

---

## Summary

Transform `spn` from "package manager" positioning to "The Agentic AI Toolkit" with:
1. New tagline and branding
2. `spn status` - unified dashboard showing everything
3. `spn explore` - TUI browser for discovery
4. `spn suggest` - wizard for guided recommendations

---

## Phase 1: `spn status` Dashboard

**Priority**: HIGH (solves immediate pain point)

### Design

```
$ spn status

┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  ✦ spn status                                    The Agentic AI Toolkit  ✦   ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

┌─ 🦙 LOCAL MODELS ────────────────────────────────────────────────────────────┐
│                                                                              │
│  Ollama → http://localhost:11434                            ✅ running       │
│  Memory  4.1 / 16.0 GB                    ████████░░░░░░░░  25%              │
│                                                                              │
│  Models                                                                      │
│  ├── ● llama3.2:7b      loaded   4.1 GB                                      │
│  ├── ○ mistral:7b       ready    4.0 GB                                      │
│  └── ○ codellama:13b    ready    7.3 GB                                      │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🔑 CREDENTIALS ─────────────────────────────────────────────────────────────┐
│                                                                              │
│  Name          Type   Status      Source       Endpoint                      │
│  ──────────────────────────────────────────────────────────────────────────  │
│  anthropic     LLM    ✅ ready    🔐 keychain   api.anthropic.com             │
│  openai        LLM    ✅ ready    📦 env        api.openai.com                │
│  ollama        LLM    ✅ local    🦙 local      localhost:11434               │
│  mistral       LLM    ✅ ready    🔐 keychain   api.mistral.ai                │
│  groq          LLM    ❌ ──       ──            ──                            │
│  deepseek      LLM    ❌ ──       ──            ──                            │
│  gemini        LLM    ❌ ──       ──            ──                            │
│  ──────────────────────────────────────────────────────────────────────────  │
│  neo4j         MCP    ✅ ready    🔐 keychain   bolt://localhost:7687         │
│  github        MCP    ✅ ready    🔐 keychain   api.github.com                │
│  firecrawl     MCP    ✅ ready    📦 env        api.firecrawl.dev             │
│  perplexity    MCP    ✅ ready    📄 .env       api.perplexity.ai             │
│  slack         MCP    ❌ ──       ──            ──                            │
│  supadata      MCP    ❌ ──       ──            ──                            │
│                                                                              │
│  8/13 configured   │   🔐 4 keychain   📦 2 env   📄 1 .env   🦙 1 local      │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🔌 MCP SERVERS ─────────────────────────────────────────────────────────────┐
│                                                                              │
│  Server        Status       Transport   Uptime       Tools    Credential     │
│  ──────────────────────────────────────────────────────────────────────────  │
│  neo4j         ✅ connected  stdio       2h 34m       3        → neo4j       │
│  github        ✅ connected  stdio       2h 34m       12       → github      │
│  firecrawl     ✅ connected  stdio       1h 12m       8        → firecrawl   │
│  perplexity    ⏳ starting   stdio       ──           ──       → perplexity  │
│  context7      ✅ connected  stdio       2h 34m       2        (no key)      │
│  novanet       ⏸️  disabled   ──          ──           ──       ──            │
│                                                                              │
│  4/6 active                                                                  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 📡 DAEMON ──────────────────────────────────────────────────────────────────┐
│                                                                              │
│  spn daemon ✅ running   PID 12345   ~/.spn/daemon.sock   Uptime 2h 34m      │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

  🔑 8/13 Keys    🔌 4/6 MCPs    🦙 3 Models    📡 Daemon OK
```

### Implementation Tasks

#### 1.1 Create status command structure
- File: `crates/spn/src/commands/status.rs`
- Add `StatusCommands` enum to CLI
- Wire up in `main.rs`

#### 1.2 Ollama status collector
- File: `crates/spn/src/status/ollama.rs`
- Query `http://localhost:11434/api/tags` for models
- Query `http://localhost:11434/api/ps` for loaded models
- Get memory usage from loaded model info

#### 1.3 Credentials collector
- File: `crates/spn/src/status/credentials.rs`
- Use existing `KNOWN_PROVIDERS` from spn-core
- Check each provider: keychain → env → .env
- Return source and endpoint for each

#### 1.4 MCP servers collector
- File: `crates/spn/src/status/mcp.rs`
- Read from `~/.spn/mcp.yaml` and project `mcp.yaml`
- Check process status (running/disabled)
- Map credentials to servers

#### 1.5 Daemon status collector
- File: `crates/spn/src/status/daemon.rs`
- Check socket exists and is responsive
- Get PID from pidfile
- Calculate uptime

#### 1.6 ASCII renderer
- File: `crates/spn/src/status/render.rs`
- Box drawing characters
- Progress bars for memory
- Color coding (green/red/yellow)

#### 1.7 JSON output
- Add `--json` flag for machine-readable output
- Structured output for scripting

---

## Phase 2: Branding Update

**Priority**: MEDIUM

### Tasks

#### 2.1 Update tagline
- README.md: "One config. Every AI tool." → "The Agentic AI Toolkit"
- CLAUDE.md: Update description
- Cargo.toml: Update package description

#### 2.2 Update CLI help
- `spn --help` header
- `spn --version` output

#### 2.3 Update highlights
- Reorder to emphasize:
  1. Local models (Ollama)
  2. Unified credentials
  3. MCP ecosystem
  4. Editor sync

---

## Phase 3: `spn explore` TUI

**Priority**: LOW (after status works)

### Design
- Interactive TUI browser
- Categories: Skills, Workflows, MCPs, Models
- Search/filter in real-time
- Preview panel
- Install with Enter

### Tasks
- New TUI module using ratatui
- Registry browser
- Local package browser
- Model browser (Ollama library)

---

## Phase 4: `spn suggest` Wizard

**Priority**: LOW (after explore works)

### Design
- Step-by-step questions
- "What do you want to build?"
- Suggests packages/skills based on answers

### Tasks
- Wizard flow engine
- Question bank
- Recommendation engine

---

## File Changes Summary

```
crates/spn/src/
├── commands/
│   ├── mod.rs              # Add status module
│   └── status.rs           # NEW: Status command
├── status/                  # NEW: Status collectors
│   ├── mod.rs
│   ├── ollama.rs           # Ollama status
│   ├── credentials.rs      # Unified credentials
│   ├── mcp.rs              # MCP servers
│   ├── daemon.rs           # Daemon status
│   └── render.rs           # ASCII rendering
└── main.rs                 # Wire up status command

README.md                   # Branding update
CLAUDE.md                   # Branding update
crates/spn/Cargo.toml       # Description update
```

---

## Testing

- [ ] `spn status` shows all sections
- [ ] `spn status --json` outputs valid JSON
- [ ] Works when Ollama not running (graceful)
- [ ] Works when daemon not running (graceful)
- [ ] Works with no credentials (shows empty)
- [ ] Colors render correctly
- [ ] Box drawing works in all terminals

---

## Success Criteria

1. User can see at a glance:
   - Which models are available/loaded
   - Which credentials are configured and where
   - Which MCP servers are running
   - Daemon status

2. No more "where is this running?" confusion

3. Clear visual hierarchy with ASCII art

---

## v0.16.0: Daemon as MCP Server + Job Scheduler

**Priority**: HIGH
**ADR**: ADR-002 (Daemon MCP), ADR-003 (Job Scheduler Migration)

### Overview

Transform the spn daemon into an MCP server exposing 6 tools, and add a job scheduler
that was previously planned for Nika. This makes spn the central orchestrator.

### Daemon MCP Tools

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DAEMON MCP TOOLS (6 total)                                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn_get_secret       Get API key for a provider                                │
│  spn_list_providers   List all providers with status                            │
│  spn_model_status     Get Ollama models status                                  │
│  spn_model_run        Run inference on local model                              │
│  spn_mcp_health       Check MCP server health                                   │
│  spn_jobs_list        List scheduled jobs                                       │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Job Scheduler Design

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  `spn jobs` — Intelligent Job Scheduler                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  COMMANDS:                                                                      │
│  ├── spn jobs                    List all jobs with status                      │
│  ├── spn jobs add <spec>         Add job (wizard if no flags)                   │
│  ├── spn jobs rm <name>          Remove job                                     │
│  ├── spn jobs run <name>         Run job now (bypass schedule)                  │
│  ├── spn jobs pause <name>       Pause scheduled job                            │
│  ├── spn jobs resume <name>      Resume paused job                              │
│  ├── spn jobs logs [name]        View job execution logs                        │
│  ├── spn jobs edit <name>        Edit job in $EDITOR                            │
│  └── spn jobs templates          List available workflow templates              │
│                                                                                 │
│  SCHEDULE SHORTCUTS:                                                            │
│  ├── --every 2h                  Every 2 hours                                  │
│  ├── --daily 3am                 Daily at 3am                                   │
│  ├── --hourly                    Every hour                                     │
│  ├── --weekly sun@2am            Weekly on Sunday at 2am                        │
│  └── --cron "0 * * * *"          Raw cron expression                            │
│                                                                                 │
│  JOB TYPES:                                                                     │
│  ├── spn jobs add -- "curl ..."           Shell command                         │
│  ├── spn jobs add --nika workflow.yaml    Nika workflow                         │
│  ├── spn jobs add --mcp neo4j.backup      MCP tool invocation                   │
│  └── spn jobs add --run "spn model pull"  spn command                           │
│                                                                                 │
│  STORAGE: ~/.spn/jobs/                                                          │
│  ├── job-name.yaml               Job definition                                 │
│  └── logs/                       Execution logs                                 │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Smart Wizard (DEFAULT when Ollama running)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  INTELLIGENT WORKFLOW CREATION                                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  $ spn jobs add                                                                 │
│  🧙 Ollama detected (llama3.2 running)                                          │
│                                                                                 │
│  What job would you like to create?                                             │
│  > every morning, check my repos and summarize PRs                              │
│                                                                                 │
│  ✨ Understanding intent...                                                     │
│  ├── Schedule: daily at 8:00 AM                                                 │
│  ├── Action: GitHub API → PR list → LLM summary                                 │
│  └── Type: Nika workflow (multi-step)                                           │
│                                                                                 │
│  📝 Generated workflow:                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  workflow: check-prs                                                    │    │
│  │  steps:                                                                 │    │
│  │    - fetch: https://api.github.com/user/repos                           │    │
│  │      headers: { Authorization: "Bearer ${spn:github}" }                 │    │
│  │      use.ctx: repos                                                     │    │
│  │    - infer: "Summarize open PRs from these repos: $repos"               │    │
│  │      provider: anthropic                                                │    │
│  │      use.ctx: summary                                                   │    │
│  │    - exec: 'osascript -e "display notification \"$summary\""'           │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                 │
│  Save as ~/.spn/jobs/check-prs.nika.yaml? [Y/n]                                 │
│                                                                                 │
│  ARCHITECTURE (behind the scenes):                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  1. spn → Ollama (llama3.2): Parse natural language intent              │    │
│  │  2. spn → Nika MCP: nika_schema_introspect (available verbs/tools)      │    │
│  │  3. spn → Nika MCP: nika_workflow_scaffold (generate YAML)              │    │
│  │  4. spn → Nika MCP: nika_workflow_validate (check before save)          │    │
│  │  5. spn → Save to ~/.spn/jobs/                                          │    │
│  │  6. spn daemon → Schedule via internal cron                             │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Built-in Workflows (Hybrid Bundling)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  WORKFLOW TEMPLATES                                                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  EMBEDDED (3-5 essentials, ~50KB in binary):                                    │
│  ├── github-pr-check       Check PRs and summarize                              │
│  ├── backup-notify         Backup reminder with status                          │
│  └── health-check          System health dashboard                              │
│                                                                                 │
│  REGISTRY (on-demand, cached at ~/.spn/workflows/registry/):                    │
│  ├── slack-daily-digest    Daily Slack summary                                  │
│  ├── neo4j-backup          Neo4j backup workflow                                │
│  ├── model-cleanup         Clean unused Ollama models                           │
│  └── ... (more from registry)                                                   │
│                                                                                 │
│  USER (custom at ~/.spn/workflows/custom/):                                     │
│  └── my-workflow.yaml      User-created workflows                               │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Nika Integration (Subprocess)

Jobs of type "nika" are executed via subprocess:

```rust
// spn-daemon job executor
async fn execute_nika_job(workflow: &Path) -> Result<JobResult> {
    let output = Command::new("nika")
        .args(["run", workflow.to_str().unwrap()])
        .env("SPN_DAEMON_SOCK", socket_path())
        .output()
        .await?;

    JobResult {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}
```

### Implementation Tasks

#### 5.1 Daemon MCP Protocol
- File: `crates/spn/src/daemon/mcp.rs`
- Implement MCP server in daemon
- 6 tools as described above

#### 5.2 Job Scheduler Core
- File: `crates/spn/src/daemon/scheduler.rs`
- Cron-like scheduler using tokio
- Job queue with priorities
- Execution tracking

#### 5.3 Jobs CLI
- File: `crates/spn/src/commands/jobs.rs`
- All subcommands: add, rm, run, pause, resume, logs, edit, templates
- Schedule shortcuts parsing

#### 5.4 Smart Wizard
- File: `crates/spn/src/commands/jobs/wizard.rs`
- Ollama detection and prompting
- Nika MCP integration for workflow generation
- Template matching fallback

#### 5.5 Built-in Workflows
- File: `crates/spn/src/workflows/`
- Embedded templates with include_str!
- Registry client for extended library

---

## v0.17.0: Agentic Foundations

**Priority**: MEDIUM
**Dependencies**: v0.16.0

### Overview

Lay groundwork for autonomous AI operations with structured memory,
reasoning traces, and agent delegation.

### Features

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  AGENTIC FOUNDATIONS                                                            │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  1. STRUCTURED MEMORY                                                           │
│  ├── ~/.spn/memory/                  Persistent agent memory                    │
│  ├── spn remember "key" "value"      Store information                          │
│  ├── spn recall "query"              Retrieve with semantic search              │
│  └── Integration with Nika contexts                                             │
│                                                                                 │
│  2. REASONING TRACES                                                            │
│  ├── ~/.spn/traces/                  Execution traces                           │
│  ├── spn trace <job-id>              View reasoning steps                       │
│  ├── JSON-LD format for graph import                                            │
│  └── NovaNet integration (novanet_trace_import)                                 │
│                                                                                 │
│  3. AGENT DELEGATION                                                            │
│  ├── spn delegate "task"             Spawn autonomous agent                     │
│  ├── Uses Nika agent: verb                                                      │
│  ├── Checkpoint/resume support                                                  │
│  └── Human-in-the-loop approvals                                                │
│                                                                                 │
│  4. WORKFLOW CHAINING                                                           │
│  ├── Job dependencies (after: [job1, job2])                                     │
│  ├── Conditional execution (if: $prev.success)                                  │
│  └── Fan-out/fan-in patterns                                                    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Implementation Tasks

#### 6.1 Memory System
- Embedding storage (local or via Ollama)
- SQLite for structured data
- Vector search for recall

#### 6.2 Trace Capture
- Hook into Nika execution events
- JSON-LD serialization
- NovaNet MCP tool for import

#### 6.3 Agent Delegation
- Task decomposition
- Checkpoint serialization
- Approval workflow

---

## v0.18.0: Full Autonomy

**Priority**: LOW
**Dependencies**: v0.17.0

### Overview

Complete the autonomous AI toolkit with self-improvement,
proactive suggestions, and continuous learning.

### Features

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  FULL AUTONOMY                                                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  1. PROACTIVE SUGGESTIONS                                                       │
│  ├── spn watch                       Monitor and suggest optimizations          │
│  ├── "You ran this 5 times, create a job?"                                      │
│  └── Pattern detection from traces                                              │
│                                                                                 │
│  2. SELF-IMPROVEMENT                                                            │
│  ├── Job auto-optimization           Refine workflows based on results          │
│  ├── A/B testing for prompts         Compare provider responses                 │
│  └── Cost optimization               Route to cheapest effective provider       │
│                                                                                 │
│  3. CONTINUOUS LEARNING                                                         │
│  ├── Feedback loops                  Learn from corrections                     │
│  ├── NovaNet knowledge updates       Write back to graph                        │
│  └── Workflow versioning             Track improvements                         │
│                                                                                 │
│  4. COLLABORATIVE AGENTS                                                        │
│  ├── Multi-agent orchestration       Parallel task execution                    │
│  ├── Inter-agent communication       Via MCP tools                              │
│  └── Conflict resolution             Consensus mechanisms                       │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Timeline

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ROADMAP                                                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  v0.15.0 (Current)     Status Dashboard + Branding                              │
│  ├── Phase 1: spn status                                         ██████████    │
│  ├── Phase 2: Branding update                                    ██████████    │
│  ├── Phase 3: spn explore TUI                                    ░░░░░░░░░░    │
│  └── Phase 4: spn suggest                                        ░░░░░░░░░░    │
│                                                                                 │
│  v0.16.0               Daemon MCP + Job Scheduler                               │
│  ├── Daemon MCP server (6 tools)                                 ░░░░░░░░░░    │
│  ├── spn jobs commands                                           ░░░░░░░░░░    │
│  ├── Smart wizard (Ollama + Nika)                                ░░░░░░░░░░    │
│  └── Built-in workflows                                          ░░░░░░░░░░    │
│                                                                                 │
│  v0.17.0               Agentic Foundations                                      │
│  ├── Structured memory                                           ░░░░░░░░░░    │
│  ├── Reasoning traces                                            ░░░░░░░░░░    │
│  ├── Agent delegation                                            ░░░░░░░░░░    │
│  └── Workflow chaining                                           ░░░░░░░░░░    │
│                                                                                 │
│  v0.18.0               Full Autonomy                                            │
│  ├── Proactive suggestions                                       ░░░░░░░░░░    │
│  ├── Self-improvement                                            ░░░░░░░░░░    │
│  ├── Continuous learning                                         ░░░░░░░░░░    │
│  └── Collaborative agents                                        ░░░░░░░░░░    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## ADRs

| ADR | Title | Status |
|-----|-------|--------|
| ADR-001 | Agentic AI Toolkit Evolution | Approved |
| ADR-002 | Daemon as MCP Server | Approved |
| ADR-003 | Job Scheduler Migration (Nika → spn) | Approved |

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Nika dependency for smart wizard | Fallback to templates if Nika not installed |
| Ollama not running | Graceful degradation to template selection |
| Complex scheduler bugs | Start simple (subset of cron), expand gradually |
| Memory system overhead | Optional feature, lazy loading |
