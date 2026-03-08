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
