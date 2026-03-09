# MCP Auto-Sync Environment Manager

**Date:** 2026-03-08
**Status:** Design Complete - Ready for Implementation
**Author:** Claude + Thibaut
**Version:** v0.16.0 target

---

## Final Decisions Summary

| Decision | Choice |
|----------|--------|
| **Watch Scope** | Option A + Lite C (globals + 5 recent projects) |
| **Recent Projects Trigger** | Any `spn` command in a directory |
| **Recent Projects Max** | 5 (configurable) |
| **Foreign MCP Handling** | A+B (log + status + native notification) |
| **Daemon Integration** | Single daemon (WatcherService added) |
| **Notifications** | N3 (macOS native + log) |

---

## Overview

Transform `spn` into THE single source of truth for all MCP configurations across
all AI clients (Claude Code, Cursor, Windsurf, VSCode, Nika). Implement automatic
synchronization and foreign MCP adoption.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ARCHITECTURE: spn as MCP Environment Manager                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Source of Truth                                                                │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │  ~/.spn/mcp.yaml                                                         │  │
│  │  ├── servers:                                                            │  │
│  │  │   ├── neo4j: { command, args, env, enabled }                          │  │
│  │  │   ├── firecrawl: { ... }                                              │  │
│  │  │   └── dataforseo: { ... }                                             │  │
│  │  └── metadata:                                                           │  │
│  │      └── last_sync: "2026-03-08T10:30:00Z"                               │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                              │                                                  │
│                              ▼ (auto-sync daemon)                               │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │  Clients (bidirectional sync)                                            │  │
│  │                                                                          │  │
│  │  Claude Code          Cursor             Nika              Windsurf      │  │
│  │  .claude/             .cursor/           ~/.config/        .windsurf/    │  │
│  │    mcp.json             mcp.json         nika/mcp.yaml       mcp.json    │  │
│  │                                                                          │  │
│  │  ● = synced from spn                                                     │  │
│  │  ○ = foreign (adopted or pending)                                        │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Terminology

| Term | Definition |
|------|------------|
| **Client** | An AI tool that consumes MCPs (Claude Code, Cursor, Nika, Windsurf, VSCode) |
| **MCP** | Model Context Protocol server |
| **Foreign MCP** | An MCP added directly to a client without going through spn |
| **Adoption** | Process of importing a foreign MCP into spn's source of truth |
| **Sync** | Propagating spn's MCPs to client config files |

---

---

## MCP Config Paths (Research Findings)

**Critical discovery**: Current spn implementation has incorrect paths for some clients.

### Authoritative Paths (March 2026)

| Client | Scope | Path | File | Status |
|--------|-------|------|------|--------|
| **Claude Code** | Global | `~/.claude.json` | JSON with `mcpServers` | Needs update |
| **Claude Code** | Project | `.mcp.json` | Dedicated MCP file | **NEW** |
| **Claude Code** | Project (legacy) | `.claude/settings.json` | Settings file | Current impl |
| **Cursor** | Global | `~/.cursor/mcp.json` | Dedicated MCP | ✅ Correct |
| **Cursor** | Project | `.cursor/mcp.json` | Dedicated MCP | ✅ Correct |
| **Windsurf** | Global ONLY | `~/.codeium/windsurf/mcp_config.json` | Dedicated MCP | ❌ WRONG |
| **Nika** | Global | `~/.config/nika/mcp.yaml` | YAML | ✅ Correct |

### Key Corrections Needed

#### 1. Windsurf (Critical)

**Current (WRONG):**
```rust
// mcp_sync.rs line 124
root.join(".windsurf").join("mcp.json")
// or
home.join(".windsurf").join("mcp.json")
```

**Correct:**
```rust
// Windsurf ONLY supports global config
dirs::home_dir()
    .join(".codeium")
    .join("windsurf")
    .join("mcp_config.json")
```

**Note:** Windsurf does NOT support project-level MCP configs (as of March 2026).

#### 2. Claude Code (Update recommended)

**Current:** `.claude/settings.json` (works, but legacy)

**Modern approach:**
- Project MCP: `.mcp.json` (dedicated, clean separation)
- Global MCP: `~/.claude.json`

**Recommendation:** Support both for backwards compatibility:
1. Read from `.mcp.json` first (new)
2. Fall back to `.claude/settings.json` (legacy)
3. Write to `.mcp.json` by default

### Watch Scope for File Watcher

Based on research, the file watcher should monitor:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  WATCH PATHS                                                                    │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  SOURCE OF TRUTH (always watch):                                                │
│  └── ~/.spn/mcp.yaml                                                            │
│                                                                                 │
│  GLOBAL CONFIGS (user home):                                                    │
│  ├── ~/.claude.json                   (Claude Code global)                      │
│  ├── ~/.cursor/mcp.json               (Cursor global)                           │
│  ├── ~/.codeium/windsurf/mcp_config.json  (Windsurf - ONLY location)            │
│  └── ~/.config/nika/mcp.yaml          (Nika global)                             │
│                                                                                 │
│  PROJECT CONFIGS (per working directory):                                       │
│  ├── ./.mcp.json                      (Claude Code project - preferred)         │
│  ├── ./.claude/settings.json          (Claude Code project - legacy)            │
│  └── ./.cursor/mcp.json               (Cursor project)                          │
│                                                                                 │
│  NOT WATCHED (no project support):                                              │
│  └── Windsurf (global only)                                                     │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Precedence Rules

| Client | Precedence (highest → lowest) |
|--------|------------------------------|
| Claude Code | Managed → `.mcp.json` → `.claude/settings.json` → `~/.claude.json` |
| Cursor | `.cursor/mcp.json` (project) → `~/.cursor/mcp.json` (global) |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` (only option) |
| Nika | spn-client IPC → `~/.config/nika/mcp.yaml` |

### Sources

- Claude Code docs: `settings.json` documentation (https://claude.ai/code/docs)
- Cursor: rulesync project PR #1273, official docs
- Windsurf: Official docs (https://docs.windsurf.com/windsurf/cascade/mcp)
- Nika: spn-client integration

---

## Auto-Sync Architecture

### File Watcher Daemon

Runs as part of `spn daemon` (extends existing daemon):

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  FILE WATCHER COMPONENT                                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Watches:                                                                       │
│  ├── ~/.spn/mcp.yaml                    (source of truth)                       │
│  ├── .claude/mcp.json                   (project + global)                      │
│  ├── .cursor/mcp.json                   (project + global)                      │
│  ├── .windsurf/mcp.json                 (project + global)                      │
│  └── ~/.config/nika/mcp.yaml            (Nika config)                           │
│                                                                                 │
│  Behavior:                                                                      │
│  ├── Debounce: 500ms (coalesce rapid changes)                                   │
│  ├── Origin tracking: skip self-triggered changes                               │
│  ├── Checksum: skip write if content unchanged                                  │
│  └── Conflict resolution: spn wins (with foreign adoption prompt)               │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Implementation: `notify` crate

```rust
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

// Debounced watcher with 500ms delay
let (tx, rx) = channel();
let mut watcher = watcher(tx, Duration::from_millis(500))?;

// Watch source of truth
watcher.watch("~/.spn/mcp.yaml", RecursiveMode::NonRecursive)?;

// Watch client configs
for client in enabled_clients() {
    watcher.watch(client.config_path(), RecursiveMode::NonRecursive)?;
}
```

---

## Foreign MCP Adoption

### Detection Flow

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  FOREIGN MCP DETECTION                                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  1. File watcher detects change in client config                                │
│  2. Parse client config, extract MCP list                                       │
│  3. Compare with spn's source of truth                                          │
│  4. Identify "foreign" MCPs (in client but not in spn)                          │
│  5. For each foreign MCP:                                                       │
│     a. Check if first-time seeing this MCP                                      │
│     b. If first-time: prompt user (interactive) or mark pending (daemon)        │
│     c. If already marked "ignore": skip                                         │
│     d. If adopted: merge into spn + sync to all clients                         │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Adoption Prompt

When running interactively (`spn status` or `spn sync`):

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  🆕 FOREIGN MCP DETECTED                                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  Found in: Cursor (.cursor/mcp.json)                                          ║
║  Server:   my-custom-mcp                                                      ║
║  Command:  npx @custom/mcp-server                                             ║
║                                                                               ║
║  Options:                                                                     ║
║  [A] Adopt → Import to spn and sync to all clients                            ║
║  [I] Ignore → Don't ask again for this MCP                                    ║
║  [S] Skip → Decide later                                                      ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Storage of Adoption Decisions

```yaml
# ~/.spn/mcp.yaml
servers:
  neo4j: { ... }
  firecrawl: { ... }

foreign:
  ignored:
    - "my-custom-mcp"           # User chose to ignore
  pending:
    - name: "another-mcp"       # Detected but not yet decided
      source: "cursor"
      detected: "2026-03-08T10:30:00Z"
```

---

## MCP Emojis

Each MCP type gets a distinctive emoji for quick visual identification:

| MCP | Emoji | Rationale |
|-----|-------|-----------|
| neo4j | 🔷 | Graph/diamond shape |
| github | 🐙 | GitHub octocat |
| slack | 💬 | Chat bubbles |
| perplexity | 🔮 | AI/search oracle |
| firecrawl | 🔥 | Fire for crawling |
| supadata | 📺 | Video/media |
| dataforseo | 📊 | SEO analytics |
| ahrefs | 🔗 | Backlinks |
| context7 | 📚 | Documentation |
| novanet | 🌐 | Knowledge graph |
| sequential-thinking | 🧠 | Thinking/reasoning |
| 21st | 🎨 | Design/UI |
| spn-mcp | ⚡ | Dynamic wrapper |
| (unknown) | 🔌 | Generic plugin |

### Implementation

```rust
// crates/spn/src/status/mcp.rs
pub fn mcp_emoji(name: &str) -> &'static str {
    match name {
        "neo4j" | "@neo4j/mcp-neo4j" => "🔷",
        "github" | "github-mcp" => "🐙",
        "slack" | "slack-mcp" => "💬",
        "perplexity" | "perplexity-mcp" => "🔮",
        "firecrawl" | "firecrawl-mcp" => "🔥",
        "supadata" | "supadata-mcp" => "📺",
        "dataforseo" => "📊",
        "ahrefs" => "🔗",
        "context7" => "📚",
        "novanet" => "🌐",
        "sequential-thinking" => "🧠",
        "21st" | "magic" => "🎨",
        "spn-mcp" => "⚡",
        _ => "🔌",
    }
}
```

---

## Extended Status Dashboard

### Current `spn status` (preserved)

The existing dashboard remains the default view:

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  ✦ spn status                                    The Agentic AI Toolkit  ✦  ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

┌─ 🦙 LOCAL MODELS ────────────────────────────────────────────────────────────┐
│  ...existing content...                                                      │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🔑 CREDENTIALS ─────────────────────────────────────────────────────────────┐
│  ...existing content...                                                      │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🔌 MCP SERVERS ─────────────────────────────────────────────────────────────┐
│                                                                              │
│  Server         Status      Transport   Credential    Clients               │
│  ──────────────────────────────────────────────────────────────────────────  │
│  🔷 neo4j       ○ ready     stdio       → neo4j       ● ● ● ○               │
│  🔥 firecrawl   ○ ready     stdio       → firecrawl   ● ● ○ ○               │
│  🔮 perplexity  ○ ready     stdio       → perplexity  ● ● ● ●               │
│  📊 dataforseo  ○ ready     stdio       → dataforseo  ● ○ ○ ○               │
│  🌐 novanet     ○ ready     stdio       (no key)      ● ● ● ○               │
│  🧠 sequential  ○ ready     stdio       (no key)      ● ○ ○ ○               │
│                                                                              │
│  8/8 active   Legend: ● synced  ○ not synced  [C]laude [Cu]rsor [N]ika [W]  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 📡 DAEMON ──────────────────────────────────────────────────────────────────┐
│  ...existing content...                                                      │
└──────────────────────────────────────────────────────────────────────────────┘

  🔑 8/15 Keys    🔌 8/8 MCPs    🦙 3 Models    📡 Daemon OK
```

### New: `spn status mcp` (detailed MCP view)

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  ✦ spn status mcp                                        MCP Detail View  ✦  ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

┌─ 🔷 NEO4J ───────────────────────────────────────────────────────────────────┐
│                                                                              │
│  Command:     npx -y @neo4j/mcp-neo4j                                        │
│  Transport:   stdio                                                          │
│  Credential:  neo4j → 🔐 keychain ✅                                         │
│                                                                              │
│  Sync Status:                                                                │
│  ├── Claude Code    ● synced     .claude/mcp.json                            │
│  ├── Cursor         ● synced     .cursor/mcp.json                            │
│  ├── Nika           ● synced     ~/.config/nika/mcp.yaml                     │
│  └── Windsurf       ○ disabled   (run: spn sync --enable windsurf)           │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 📊 DATAFORSEO ──────────────────────────────────────────────────────────────┐
│                                                                              │
│  Command:     spn-mcp --config ~/.spn/apis/dataforseo.yaml                   │
│  Transport:   stdio                                                          │
│  Credential:  dataforseo → 🔐 keychain ✅                                    │
│  Tools:       4 (keyword_ideas, serp_google, domain_metrics, backlinks)      │
│                                                                              │
│  Sync Status:                                                                │
│  ├── Claude Code    ● synced     .claude/mcp.json                            │
│  ├── Cursor         ○ pending    (run: spn sync)                             │
│  ├── Nika           ○ pending                                                │
│  └── Windsurf       ○ disabled                                               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🆕 FOREIGN MCPs ────────────────────────────────────────────────────────────┐
│                                                                              │
│  Found 1 MCP not managed by spn:                                             │
│                                                                              │
│  🔌 my-custom-mcp                                                            │
│     Source: Cursor (.cursor/mcp.json)                                        │
│     Command: npx @custom/mcp-server                                          │
│                                                                              │
│     [A]dopt  [I]gnore  [S]kip                                                │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

  8 MCPs managed    1 foreign    4 clients enabled
```

### New: `spn status clients` (detailed clients view)

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  ✦ spn status clients                                  Client Detail View  ✦  ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

┌─ 🤖 CLAUDE CODE ─────────────────────────────────────────────────────────────┐
│                                                                              │
│  Status:      ✅ enabled                                                     │
│  Config:      /Users/thibaut/dev/supernovae/.claude/mcp.json                 │
│  Last Sync:   2026-03-08 10:30:00 (2 minutes ago)                            │
│                                                                              │
│  MCPs (8/8 synced):                                                          │
│  ├── 🔷 neo4j           ● synced                                             │
│  ├── 🔥 firecrawl       ● synced                                             │
│  ├── 🔮 perplexity      ● synced                                             │
│  ├── 📊 dataforseo      ● synced                                             │
│  ├── 🌐 novanet         ● synced                                             │
│  ├── 🧠 sequential      ● synced                                             │
│  ├── 📚 context7        ● synced                                             │
│  └── 🎨 21st            ● synced                                             │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 📝 CURSOR ──────────────────────────────────────────────────────────────────┐
│                                                                              │
│  Status:      ✅ enabled                                                     │
│  Config:      /Users/thibaut/.cursor/mcp.json                                │
│  Last Sync:   2026-03-08 09:15:00 (1 hour ago)                               │
│                                                                              │
│  MCPs (6/8 synced, 1 foreign):                                               │
│  ├── 🔷 neo4j           ● synced                                             │
│  ├── 🔥 firecrawl       ● synced                                             │
│  ├── 🔮 perplexity      ● synced                                             │
│  ├── 📊 dataforseo      ○ pending     (newer in spn)                         │
│  ├── 🌐 novanet         ● synced                                             │
│  ├── 📚 context7        ● synced                                             │
│  └── 🔌 my-custom-mcp   🆕 foreign    (not in spn)                           │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🦋 NIKA ────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  Status:      ✅ enabled                                                     │
│  Config:      ~/.config/nika/mcp.yaml                                        │
│  Last Sync:   2026-03-08 10:30:00 (2 minutes ago)                            │
│                                                                              │
│  MCPs (5/8 synced):                                                          │
│  ├── 🔷 neo4j           ● synced                                             │
│  ├── 🔮 perplexity      ● synced                                             │
│  ├── 🌐 novanet         ● synced                                             │
│  ├── 📚 context7        ● synced                                             │
│  └── 🧠 sequential      ● synced                                             │
│                                                                              │
│  Note: 3 MCPs not synced to Nika (not in allowlist)                          │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🌊 WINDSURF ────────────────────────────────────────────────────────────────┐
│                                                                              │
│  Status:      ❌ disabled                                                    │
│  Config:      ~/.windsurf/mcp.json (not found)                               │
│                                                                              │
│  Run: spn sync --enable windsurf                                             │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

  4 clients    3 enabled    1 disabled    8 MCPs    1 foreign
```

---

## Command Structure

```
spn status              # Unified dashboard (existing, extended)
spn status mcp          # Detailed MCP view (new)
spn status clients      # Detailed clients view (new)
spn status --json       # JSON output (existing)
```

### CLI Definition

```rust
#[derive(Parser)]
pub struct StatusCommand {
    /// Output format
    #[arg(long)]
    json: bool,

    /// Subcommand for detailed views
    #[command(subcommand)]
    command: Option<StatusSubcommand>,
}

#[derive(Subcommand)]
pub enum StatusSubcommand {
    /// Detailed MCP servers view
    Mcp,
    /// Detailed clients view
    Clients,
}
```

---

---

## Technical Specification

### Watch Scope (A + Lite C)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DAEMON FILE WATCHER                                                            │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ALWAYS WATCH (Global):                                                         │
│  ├── ~/.spn/mcp.yaml                    Source of truth                         │
│  ├── ~/.cursor/mcp.json                 Cursor global                           │
│  ├── ~/.claude.json                     Claude Code global                      │
│  └── ~/.codeium/windsurf/mcp_config.json  Windsurf (global only)                │
│                                                                                 │
│  RECENT PROJECTS (Lite C - max 5):                                              │
│  └── For each recent project:                                                   │
│      ├── /project/.cursor/mcp.json                                              │
│      ├── /project/.mcp.json             Claude Code project (new)               │
│      └── /project/.claude/settings.json Claude Code project (legacy)            │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Recent Projects Tracking

**File:** `~/.spn/recent.yaml`

```yaml
max_projects: 5
projects:
  - path: /Users/thibaut/dev/supernovae/supernovae-cli
    last_used: 2026-03-08T10:30:00Z
  - path: /Users/thibaut/dev/supernovae/nika
    last_used: 2026-03-08T09:15:00Z
  - path: /Users/thibaut/dev/supernovae/novanet
    last_used: 2026-03-07T16:00:00Z
```

**Trigger:** Any `spn` command executed in a directory adds it to recent.

**Behavior:**
- New project → added to top of list
- Existing project → moved to top, timestamp updated
- List exceeds max → oldest removed
- Daemon notified → starts/stops watching paths

### Foreign MCP Detection

**File:** `~/.spn/foreign.yaml`

```yaml
ignored:              # User chose "Ignore"
  - my-test-mcp
  - dev-only-server

pending:              # Detected, awaiting decision
  - name: new-mcp
    source: cursor    # Which client added it
    scope: global     # global or project
    path: ~/.cursor/mcp.json
    detected: 2026-03-08T10:30:00Z
    config:
      command: npx
      args: ["-y", "@new/mcp-server"]
      env:
        API_KEY: "${API_KEY}"
```

**Detection Flow:**

```
File change detected
       │
       ▼
Parse client config (JSON)
       │
       ▼
Compare with ~/.spn/mcp.yaml
       │
       ▼
New MCP found? ──NO──► Done
       │
      YES
       │
       ▼
In ignored list? ──YES──► Done
       │
      NO
       │
       ▼
Add to pending list
       │
       ├──► Log: "Foreign MCP detected: {name}"
       │
       └──► Native notification: "spn: New MCP '{name}' detected"
```

### Daemon Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn daemon (single process)                                                    │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐                  │
│  │ KeychainService │  │   IpcService    │  │ WatcherService  │  ← NEW           │
│  │   (existing)    │  │   (existing)    │  │                 │                  │
│  ├─────────────────┤  ├─────────────────┤  ├─────────────────┤                  │
│  │ • get_secret    │  │ • Unix socket   │  │ • notify crate  │                  │
│  │ • set_secret    │  │ • IPC protocol  │  │ • debounce 500ms│                  │
│  │ • OS keychain   │  │ • peer verify   │  │ • origin track  │                  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘                  │
│           │                    │                    │                           │
│           └────────────────────┴────────────────────┘                           │
│                                │                                                │
│                         JoinSet (tokio)                                         │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Notification System

**Native macOS notification:**
- Crate: `notify-rust`
- Title: "spn"
- Body: "Foreign MCP detected: {name}"
- Action: Click opens terminal with `spn status`

**Log output:**
- Daemon stderr: `[WATCH] Foreign MCP detected: {name} (source: cursor)`
- Visible via `spn mcp logs` or daemon stderr

### New Files

| File | Purpose |
|------|---------|
| `~/.spn/recent.yaml` | Recent projects list |
| `~/.spn/foreign.yaml` | Foreign MCP tracking (ignored + pending) |
| `crates/spn/src/daemon/watcher.rs` | WatcherService implementation |
| `crates/spn/src/daemon/recent.rs` | Recent projects manager |
| `crates/spn/src/daemon/foreign.rs` | Foreign MCP detector |

### Dependencies

```toml
# Cargo.toml additions
notify = "6"           # File watching
notify-rust = "4"      # macOS notifications
```

---

## Implementation Phases

### Phase 1: MCP Emojis + Client Sync Column (Quick Win)

1. Add `mcp_emoji()` function to `status/mcp.rs`
2. Add "Clients" column to MCP table with sync status dots
3. Update footer to show client count

**Files:**
- `crates/spn/src/status/mcp.rs`
- `crates/spn/src/status/render.rs`

### Phase 2: Detailed Subcommands

1. Add `StatusSubcommand` enum
2. Implement `spn status mcp` renderer
3. Implement `spn status clients` renderer

**Files:**
- `crates/spn/src/commands/status.rs`
- `crates/spn/src/status/render.rs` (add `render_mcp_detail`, `render_clients_detail`)

### Phase 3: Foreign MCP Detection

1. Add `foreign` section to `~/.spn/mcp.yaml` schema
2. Implement diff logic: `spn MCPs` vs `client MCPs`
3. Store ignored/pending MCPs

**Files:**
- `crates/spn/src/mcp/config.rs`
- `crates/spn/src/status/mcp.rs`

### Phase 4: File Watcher Daemon

1. Add `notify` dependency
2. Implement `WatcherService` in daemon
3. Add origin tracking to prevent loops
4. Implement debounce + checksum

**Files:**
- `Cargo.toml` (add `notify = "6"`)
- `crates/spn/src/daemon/watcher.rs` (new)
- `crates/spn/src/daemon/mod.rs`

### Phase 5: Interactive Adoption

1. Add `dialoguer` prompts for foreign MCP adoption
2. Implement adopt/ignore/skip actions
3. Trigger immediate sync on adopt

**Files:**
- `crates/spn/src/commands/status.rs`
- `crates/spn/src/mcp/adopt.rs` (new)

---

## Data Structures

### Extended McpServerStatus

```rust
#[derive(Debug, Clone, Serialize)]
pub struct McpServerStatus {
    pub name: String,
    pub emoji: &'static str,
    pub status: ServerStatus,
    pub transport: Transport,
    pub credential: Option<String>,
    pub command: String,
    pub client_sync: ClientSyncStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientSyncStatus {
    pub claude_code: SyncState,
    pub cursor: SyncState,
    pub nika: SyncState,
    pub windsurf: SyncState,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum SyncState {
    Synced,
    Pending,
    Disabled,
    NotApplicable,
}
```

### ForeignMcp

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignMcp {
    pub name: String,
    pub source_client: String,
    pub command: String,
    pub args: Vec<String>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignMcpConfig {
    pub ignored: Vec<String>,
    pub pending: Vec<ForeignMcp>,
}
```

---

## Testing Strategy

1. **Unit tests**: Emoji mapping, sync state detection, diff logic
2. **Integration tests**: File watcher with temp directories
3. **Manual testing**: Real editor configs

---

## Open Questions

1. **Nika MCP allowlist**: Should Nika have a separate allowlist, or sync all MCPs?
2. **Conflict resolution**: When client has newer config, should we prompt?
3. **Auto-start watcher**: Should file watcher start with daemon, or separate?

---

## Summary

This design extends `spn status` to become a comprehensive MCP environment manager:

- **Preserved**: Existing dashboard design
- **Added**: MCP emojis, client sync status column
- **New**: `spn status mcp` and `spn status clients` subcommands
- **New**: Foreign MCP detection and adoption workflow
- **Future**: Auto-sync file watcher daemon

The implementation is phased to deliver quick wins (emojis, columns) before
tackling the more complex auto-sync daemon.
