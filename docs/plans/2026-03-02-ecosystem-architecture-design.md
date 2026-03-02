# SuperNovae Ecosystem Architecture Design

**Date:** 2026-03-02
**Status:** Final Draft
**Authors:** Thibaut, Claude
**Version:** 2.0

---

## Overview

This document defines the unified architecture for the SuperNovae ecosystem CLI tools (`spn`, `nika`, `novanet`) with clear responsibilities, MCP management, editor sync strategy, and enhanced DX features.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  SUPERNOVAE ECOSYSTEM                                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn (Package Manager + Hub)                                                    │
│  ├── Unified `add` command with auto-detection                                  │
│  ├── MCP single source (~/.spn/mcp.yaml)                                        │
│  ├── Guided sync (first-time interactive, then auto)                            │
│  └── Editor adapters (Claude Code, Cursor, VS Code, Windsurf)                   │
│       │                                                                         │
│       ├────────────────────────────────────────────────────────────────────────►│
│       │  MCP, Skills, Agents                                                    │
│       │                                                                         │
│  nika (Workflow Engine - Body)           novanet (Knowledge Graph - Brain)      │
│  ├── YAML workflows                      ├── Neo4j + Rust TUI                   │
│  ├── 7 LLM providers                     ├── 61 NodeClasses                     │
│  ├── MCP client                          ├── MCP server (8 tools)               │
│  └── Reads ~/.spn/mcp.yaml               └── Reads ~/.spn/mcp.yaml              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Architecture Principles

### 1.1 Ownership Model

```
WE OWN (unique to SuperNovae)          INTEROP (proxy existing ecosystems)
├── @workflows/  → Nika DAG workflows   ├── skills → skills.sh (57K+ skills)
├── @schemas/    → NovaNet graphs       └── mcp    → npm registry
├── @jobs/       → Scheduled tasks
├── @agents/     → Multi-turn agents
└── @prompts/    → Prompt templates
```

### 1.2 Tool Responsibilities

| Tool | Role | Standalone | Via spn |
|------|------|------------|---------|
| **spn** | Package manager + Hub | Yes | - |
| **nika** | Workflow engine | Yes | `spn nk` |
| **novanet** | Knowledge graph | Yes | `spn nv` |

### 1.3 Data Flow

```
~/.spn/                          (SOURCE OF TRUTH)
├── packages/                    Installed packages
├── mcp.yaml                     MCP servers (shared)
├── config.yaml                  User preferences
└── state.json                   Installation state
        │
        ├──────────────► Nika (reads mcp.yaml + override in workflow)
        ├──────────────► NovaNet (reads mcp.yaml)
        └──────────────► Editors (via sync)
                         ├── Claude Code (.claude/)
                         ├── Cursor (.cursor/)
                         ├── VS Code (.vscode/)
                         └── Windsurf (.windsurf/)
```

---

## 2. MCP Management

### 2.1 Single Source of Truth

**Decision:** All MCP servers are managed in `~/.spn/mcp.yaml`

```yaml
# ~/.spn/mcp.yaml
version: 1
servers:
  neo4j:
    command: npx
    args: ["-y", "@neo4j/mcp-server"]
    env: {}

  novanet:
    command: novanet-mcp
    args: ["--stdio"]
    env:
      NEO4J_URI: bolt://localhost:7687

  linear:
    command: npx
    args: ["-y", "@linear/mcp-server"]
    env:
      LINEAR_API_KEY: ${LINEAR_API_KEY}
```

### 2.2 Inheritance Model

```
LEVEL 1: GLOBAL (~/.spn/mcp.yaml)
    │
    │ inherits
    ▼
LEVEL 2: PROJECT (./spn.yaml or ./.spn/mcp.yaml)
    │
    │ inherits
    ▼
LEVEL 3: WORKFLOW (workflow.nika.yaml)
```

**Project-level override:**
```yaml
# ./spn.yaml
mcp:
  use: [neo4j, novanet]      # Use these from global
  disable: [linear]           # Disable for this project
  servers:                    # Add project-specific
    custom-api:
      command: node
      args: ["./mcp-server.js"]
```

**Workflow-level override:**
```yaml
# workflow.nika.yaml
mcp:
  use: [novanet]              # Only use novanet for this workflow
  # OR inline definition:
  servers:
    temp-server:
      command: ...
```

### 2.3 Resolution Order

1. Workflow `mcp.servers` (highest priority)
2. Workflow `mcp.use` (references global/project)
3. Project `mcp.servers`
4. Project `mcp.use`
5. Global `~/.spn/mcp.yaml` (lowest priority)

---

## 3. Guided Sync Strategy

### 3.1 First-Time Setup (Interactive)

When no sync configuration exists, prompt the user:

```
$ spn add neo4j

✓ Detected: MCP server

Where do you want to install neo4j?

❯ Global (~/.spn/mcp.yaml)     Available in all projects
  Project (./spn.yaml)          Only this project

Which editors should have access?

[x] Claude Code    (detected)
[x] Cursor         (detected)
[ ] VS Code
[ ] Windsurf
[x] Nika workflows

┌─────────────────────────────────────────────────────┐
│ [x] Remember my choices (don't ask again)           │
└─────────────────────────────────────────────────────┘

✓ Added neo4j to global config
✓ Synced to: Claude Code, Cursor, Nika
```

### 3.2 Configuration Options

```yaml
# ~/.spn/config.yaml
sync:
  auto: true                    # Auto-sync after add/install
  prompt: false                 # Don't ask (use defaults)
  scope: global                 # Default scope: global | project
  editors:                      # Default editors to sync
    - claude-code
    - cursor
    - nika
```

### 3.3 Behavior Matrix

| sync.auto | sync.prompt | Behavior |
|-----------|-------------|----------|
| true | false | Auto-sync silently with feedback |
| true | true | Auto-sync with confirmation prompt |
| false | false | Manual (`spn sync` required) |
| false | true | Ask every time |
| (none) | (none) | Interactive guided setup |

### 3.4 CLI Flags (Always Override)

```bash
spn add X --no-sync          # Skip sync completely
spn add X --sync             # Force sync even if auto=false
spn add X --global           # Force global scope
spn add X --project          # Force project scope
spn add X --sync-to claude   # Sync only to specific editors
```

---

## 4. Unified Add Command

### 4.1 Auto-Detection by Scope

**Single command with intelligent routing:**

```bash
spn add <package>            # Auto-detect type from scope/manifest
```

**Detection rules:**

| Scope Prefix | Package Type | Handler |
|--------------|--------------|---------|
| `@workflows/` | Workflow | Own (Nika) |
| `@schemas/` | Schema | Own (NovaNet) |
| `@agents/` | Agent | Own + Sync |
| `@prompts/` | Prompt | Own + Sync |
| `@jobs/` | Job | Own (Nika) |
| `@mcp/` or npm pattern | MCP Server | Proxy (npm) |
| skills.sh pattern | Skill | Proxy (skills.sh) |

**Examples:**

```bash
# Workflows (owned)
spn add @workflows/code-review
spn add @workflows/seo-pipeline

# MCP servers (proxied to npm)
spn add @mcp/neo4j              # → npm @neo4j/mcp-server
spn add neo4j-mcp               # Auto-detect npm MCP

# Skills (proxied to skills.sh)
spn add brainstorming           # → skills.sh brainstorming
spn add @skills/tdd             # Explicit skill scope

# Agents (owned)
spn add @agents/code-reviewer

# Schemas (owned)
spn add @schemas/custom-entities
```

### 4.2 Explicit Type Override

```bash
spn add X --type mcp           # Force MCP type
spn add X --type skill         # Force skill type
spn add X --type workflow      # Force workflow type
```

### 4.3 Backwards Compatibility

Explicit subcommands remain available:

```bash
spn mcp add X                  # Explicit MCP
spn skill add X                # Explicit skill
spn workflow add X             # Explicit workflow
```

---

## 5. Status Overview Command

### 5.1 `spn status` Command

Single command to see everything at a glance:

```
$ spn status

╔═══════════════════════════════════════════════════════════════════════════════╗
║  SUPERNOVAE STATUS                                                            ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  📦 Packages (12 installed)                                                   ║
║  ├── @workflows/code-review      v1.2.0                                       ║
║  ├── @agents/code-reviewer       v0.5.0                                       ║
║  └── ... (10 more)                                                            ║
║                                                                               ║
║  🔌 MCP Servers (4 active)                                                    ║
║  ├── neo4j           ✅ Connected      ~/.spn/mcp.yaml                        ║
║  ├── novanet         ✅ Connected      ~/.spn/mcp.yaml                        ║
║  ├── linear          ⚠️  API key missing                                       ║
║  └── custom-api      ✅ Connected      ./spn.yaml                             ║
║                                                                               ║
║  🎯 Skills (8 enabled)                                                        ║
║  ├── brainstorming   Global                                                   ║
║  ├── tdd             Project                                                  ║
║  └── ... (6 more)                                                             ║
║                                                                               ║
║  🔄 Sync Status                                                               ║
║  ├── Claude Code     ✅ In sync                                               ║
║  ├── Cursor          ✅ In sync                                               ║
║  └── Nika            ⚠️  1 pending                                             ║
║                                                                               ║
║  💡 Quick Actions                                                             ║
║  spn sync            Sync pending changes                                     ║
║  spn mcp test        Test all MCP connections                                 ║
║  spn outdated        Check for updates                                        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### 5.2 Focused Views

```bash
spn status --packages          # Only packages
spn status --mcp               # Only MCP servers
spn status --skills            # Only skills
spn status --sync              # Only sync status
spn status --json              # JSON output for scripting
```

---

## 6. Interactive Init

### 6.1 `spn init` Wizard

Enhanced project initialization:

```
$ spn init

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🚀 SUPERNOVAE PROJECT SETUP                                                  ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  Step 1/4: Project Type                                                       ║
║                                                                               ║
║  ❯ AI Workflow Project      (Nika workflows + NovaNet knowledge)              ║
║    Web Application          (Skills + MCP for Claude Code)                    ║
║    CLI Tool                  (Minimal setup)                                  ║
║    Custom                    (Manual selection)                               ║
║                                                                               ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  Step 2/4: MCP Servers                                                        ║
║                                                                               ║
║  [x] novanet     Knowledge graph (recommended for AI workflows)               ║
║  [x] neo4j       Graph database                                               ║
║  [ ] linear      Project management                                           ║
║  [ ] github      Repository access                                            ║
║                                                                               ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  Step 3/4: Skills                                                             ║
║                                                                               ║
║  [x] brainstorming       Design refinement                                    ║
║  [x] tdd                 Test-driven development                              ║
║  [x] debugging           Systematic debugging                                 ║
║  [ ] code-review         Review code changes                                  ║
║                                                                               ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  Step 4/4: Editor Integration                                                 ║
║                                                                               ║
║  [x] Claude Code     (detected at ~/.claude/)                                 ║
║  [x] Cursor          (detected at ~/.cursor/)                                 ║
║  [ ] VS Code                                                                  ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

Creating project...
✓ Created spn.yaml
✓ Added 2 MCP servers
✓ Added 3 skills
✓ Synced to Claude Code, Cursor

Your project is ready! Next steps:
  spn status         View installation status
  nika studio        Open workflow editor
  novanet tui        Explore knowledge graph
```

### 6.2 Init Flags

```bash
spn init                       # Interactive wizard
spn init --preset ai-workflow  # Use preset (skip wizard)
spn init --mcp                 # Only setup MCP
spn init --yes                 # Accept all defaults
```

---

## 7. Presets and Templates

### 7.1 Built-in Presets

```yaml
# ~/.spn/presets/ai-workflow.yaml
name: AI Workflow Project
description: Full AI workflow setup with NovaNet + Nika
packages:
  mcp:
    - novanet
    - neo4j
  skills:
    - brainstorming
    - tdd
    - debugging
  workflows:
    - @workflows/dev/code-review
editors:
  - claude-code
  - cursor
```

### 7.2 Preset Commands

```bash
# List available presets
spn preset list

# Show preset details
spn preset show ai-workflow

# Create project from preset
spn init --preset ai-workflow

# Save current config as preset
spn preset save my-setup

# Share preset (creates shareable YAML)
spn preset export my-setup > my-setup.yaml
spn init --from ./my-setup.yaml
```

### 7.3 Community Presets

```bash
# Install from registry
spn preset add @presets/fullstack-ai

# Install from URL
spn preset add https://example.com/preset.yaml
```

---

## 8. Validation Command

### 8.1 `spn check` Command

Comprehensive validation of configuration:

```
$ spn check

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🔍 CONFIGURATION CHECK                                                       ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  ✅ PASSED                                                                    ║
║  ├── spn.yaml syntax valid                                                    ║
║  ├── All package versions resolved                                            ║
║  ├── MCP servers reachable (4/4)                                              ║
║  └── Editor configs in sync                                                   ║
║                                                                               ║
║  ⚠️  WARNINGS                                                                  ║
║  ├── linear: API key not set (LINEAR_API_KEY)                                 ║
║  └── @workflows/old-flow: deprecated, update to v2.0                          ║
║                                                                               ║
║  ❌ ERRORS                                                                    ║
║  └── (none)                                                                   ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

Run `spn check --fix` to auto-fix issues.
```

### 8.2 Check Modes

```bash
spn check                      # Full validation
spn check --mcp                # Only MCP servers
spn check --packages           # Only packages
spn check --sync               # Only sync status
spn check --fix                # Auto-fix what's possible
spn check --strict             # Treat warnings as errors
```

### 8.3 CI Integration

```yaml
# .github/workflows/ci.yml
- name: Validate spn config
  run: spn check --strict
```

---

## 9. Shortcuts and Fuzzy Search

### 9.1 Command Aliases

Built-in shortcuts for common operations:

```bash
# Short forms
spn a X              # → spn add X
spn r X              # → spn remove X
spn i                # → spn install
spn u X              # → spn update X
spn s                # → spn status
spn c                # → spn check

# Combined shortcuts
spn ai               # → spn add --install (add + install deps)
spn ag               # → spn add --global
spn ap               # → spn add --project
```

### 9.2 Fuzzy Search

Search across all package types:

```
$ spn search code

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🔎 SEARCH: "code"                                                            ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  WORKFLOWS (3)                                                                ║
║  ├── @workflows/code-review         Review code changes                       ║
║  ├── @workflows/code-gen            Generate code from spec                   ║
║  └── @workflows/code-explain        Explain code snippets                     ║
║                                                                               ║
║  SKILLS (2)                                                                   ║
║  ├── code-reviewer                  Review code in Claude                     ║
║  └── code-explorer                  Explore codebase                          ║
║                                                                               ║
║  AGENTS (1)                                                                   ║
║  └── @agents/code-reviewer          Autonomous code review agent              ║
║                                                                               ║
║  Use arrow keys to select, Enter to install                                   ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### 9.3 Interactive Mode

```bash
spn add                        # Opens fuzzy finder
spn search                     # Interactive search
spn search --type workflow     # Filter by type
```

---

## 10. Package Groups

### 10.1 Group Definition

Install multiple packages at once:

```yaml
# ~/.spn/groups.yaml
groups:
  ai-dev:
    description: AI development essentials
    packages:
      - @mcp/novanet
      - @mcp/neo4j
      - brainstorming
      - tdd
      - @workflows/code-review

  seo-pipeline:
    description: SEO content generation
    packages:
      - @workflows/seo-analysis
      - @workflows/content-gen
      - @mcp/novanet
```

### 10.2 Group Commands

```bash
# Install entire group
spn add @group/ai-dev

# List groups
spn group list

# Show group contents
spn group show ai-dev

# Create group from current project
spn group save my-project-deps

# Remove group
spn group remove ai-dev
```

### 10.3 Project Groups

Groups can be project-local:

```yaml
# ./spn.yaml
groups:
  dev-tools:
    packages:
      - tdd
      - debugging
      - code-reviewer
```

```bash
spn add @group/dev-tools       # Install project group
```

---

## 11. Package Types & Sync Behavior

### 11.1 Package Type Matrix

| Type | Scope Question | Editor Sync | Nika Access |
|------|----------------|-------------|-------------|
| **mcp** | Global vs Project | Yes (all editors) | Yes (mcp.yaml) |
| **skill** | Global vs Project | Yes (Claude, Cursor) | Yes (skills:) |
| **workflow** | Global vs Project | No (Nika only) | Yes (direct) |
| **agent** | Global vs Project | Yes (Claude, Cursor) | Yes (agent:) |
| **prompt** | Global vs Project | Yes (Claude, Cursor) | Yes (prompts/) |
| **schema** | Global vs Project | No (NovaNet only) | No |
| **job** | Project only | No | Yes (jobs/) |

### 11.2 Skills

**Same guided approach as MCP:**

```
$ spn add brainstorming

✓ Detected: Skill (via skills.sh)

Where do you want to install this skill?

❯ Global (~/.spn/skills/)       Available in all projects
  Project (./.claude/skills/)   Only this project

Which editors should have access?

[x] Claude Code    (detected)
[x] Cursor         (detected)
[x] Nika workflows (via skills:)

✓ Added brainstorming to global skills
✓ Synced to: Claude Code, Cursor
```

### 11.3 Agents

**Same guided approach:**

```
$ spn add @agents/code-reviewer

✓ Detected: Agent

Where do you want to install this agent?

❯ Global (~/.spn/agents/)       Available in all projects
  Project (./.claude/agents/)   Only this project

✓ Added code-reviewer to global agents
✓ Synced to: Claude Code, Cursor
```

### 11.4 Workflows

**Workflows are Nika-only, no editor sync needed:**

```
$ spn add @workflows/code-review

✓ Installed @workflows/code-review@1.0.0 to ~/.spn/packages/
✓ Available via: nika run @workflows/code-review

# No editor sync prompt (Nika only)
```

### 11.5 Schemas

**Schemas are NovaNet-only:**

```
$ spn add @schemas/custom-entities

✓ Installed @schemas/custom-entities@1.0.0 to ~/.spn/packages/
✓ Available via: novanet schema add --from ~/.spn/packages/...

# No editor sync, no Nika sync (NovaNet only)
```

---

## 12. CLI Command Reference

### 12.1 spn (Package Manager + Hub)

```bash
# Package Management (Unified)
spn add <pkg> [--global|--project] [--no-sync] [--type X]
spn remove <pkg>
spn install [--frozen]
spn update [pkg]
spn list [--type X]
spn search <query> [--type X]
spn info <pkg>
spn outdated

# Status & Validation
spn status [--packages|--mcp|--skills|--sync] [--json]
spn check [--mcp|--packages|--sync] [--fix] [--strict]
spn doctor

# MCP Servers (Explicit)
spn mcp add <server> [--global|--project] [--no-sync] [--sync-to X,Y]
spn mcp remove <server>
spn mcp list
spn mcp test [server]

# Skills (Proxy to skills.sh)
spn skill add <name> [--global|--project]
spn skill remove <name>
spn skill list
spn skill search <query>

# Sync
spn sync [--target <editor>] [--status]
spn sync --enable <editor>
spn sync --disable <editor>

# Presets & Groups
spn preset list|show|save|export
spn group list|show|save|remove
spn add @group/<name>

# Init & Config
spn init [--preset X] [--mcp] [--yes]
spn config show|list|get|set|edit

# Proxies
spn nk <args>                    # → nika <args>
spn nv <args>                    # → novanet <args>

# Shortcuts
spn a X   # add
spn r X   # remove
spn i     # install
spn u X   # update
spn s     # status
spn c     # check
```

### 12.2 nika (Workflow Engine)

```bash
# Execution
nika <file.nika.yaml>            # Direct run
nika run <file> [--provider X]
nika check <file> [--strict]

# TUI (6 views)
nika                             # Home
nika chat                        # Chat
nika studio [file]               # Editor
nika ui [--view X]               # Specific view

# Observability
nika trace list|show|export|clean

# Providers
nika provider list|set|get|delete|test|migrate

# MCP (workflow-level)
nika mcp list|test|tools

# Jobs
nika jobs start|stop|status|list|trigger|pause|resume|history

# Config
nika config list|get|set|edit|path|reset
nika init
nika doctor
```

### 12.3 novanet (Knowledge Graph)

```bash
# TUI
novanet [tui]

# Read
novanet blueprint [--view X]
novanet data|overlay|query|search

# CRUD
novanet node create|edit|delete
novanet arc create|delete

# Schema
novanet schema generate|validate|stats

# Database
novanet db seed|migrate|reset|verify

# Domain
novanet locale list|import|generate
novanet knowledge generate|list
novanet entity seed|list|validate

# Utils
novanet doctor
novanet completions
```

---

## 13. File Locations

### 13.1 Global (User-level)

```
~/.spn/
├── config.yaml              # User preferences (sync, defaults)
├── mcp.yaml                 # MCP servers (source of truth)
├── state.json               # Installed packages state
├── groups.yaml              # User-defined groups
├── presets/                 # Custom presets
│   └── *.yaml
├── packages/                # Installed packages
│   ├── @workflows/
│   ├── @schemas/
│   ├── @skills/
│   ├── @agents/
│   ├── @prompts/
│   └── @jobs/
├── skills/                  # Global skills (symlinks)
├── agents/                  # Global agents (symlinks)
└── cache/
    └── registry/            # Sparse index cache
```

### 13.2 Project-level

```
./
├── spn.yaml                 # Project manifest
├── spn.lock                 # Locked versions
├── .spn/
│   └── mcp.yaml             # Project MCP overrides (optional)
├── .claude/
│   ├── settings.json        # Claude Code config (generated)
│   ├── skills/              # Project skills (symlinks)
│   └── agents/              # Project agents (symlinks)
├── .cursor/
│   └── mcp.json             # Cursor config (generated)
├── .nika/
│   ├── config.toml          # Nika project config
│   ├── traces/              # Execution traces
│   └── sessions/            # Editor sessions
└── workflows/
    └── *.nika.yaml          # Project workflows
```

---

## 14. Implementation Phases

### Phase 1: Foundation (Week 1-2)

**Priority: Critical - Base infrastructure**

| Task | Description | Effort |
|------|-------------|--------|
| 1.1 | Create `~/.spn/mcp.yaml` format | 2h |
| 1.2 | Create `~/.spn/config.yaml` format | 2h |
| 1.3 | Update `spn mcp add` to write to mcp.yaml directly | 4h |
| 1.4 | Add `--global` and `--project` flags to all commands | 4h |
| 1.5 | Update Nika to read from `~/.spn/mcp.yaml` | 4h |
| 1.6 | Implement MCP inheritance (global → project → workflow) | 8h |

**Deliverables:**
- [ ] `spn mcp add X` writes to `~/.spn/mcp.yaml`
- [ ] `nika mcp list` reads from `~/.spn/mcp.yaml`
- [ ] Inheritance working with override

### Phase 2: Guided Sync (Week 2-3)

**Priority: High - Core DX improvement**

| Task | Description | Effort |
|------|-------------|--------|
| 2.1 | Implement interactive prompts for first-time setup | 6h |
| 2.2 | Add `sync.auto`, `sync.prompt`, `sync.scope` config options | 4h |
| 2.3 | Add `--no-sync`, `--sync`, `--sync-to` flags | 4h |
| 2.4 | Store user preferences in `~/.spn/config.yaml` | 2h |
| 2.5 | Implement "Remember my choices" persistence | 2h |

**Deliverables:**
- [ ] First-time interactive wizard works
- [ ] Preferences persist across sessions
- [ ] CLI flags override config

### Phase 3: Editor Adapters (Week 3-4)

**Priority: High - Editor integration**

| Task | Description | Effort |
|------|-------------|--------|
| 3.1 | Claude Code adapter (full: MCP, skills, agents, hooks) | 8h |
| 3.2 | Cursor adapter (MCP + skills) | 4h |
| 3.3 | VS Code adapter (stub for future) | 2h |
| 3.4 | Windsurf adapter (stub for future) | 2h |
| 3.5 | Auto-sync after `spn add` (if configured) | 4h |
| 3.6 | Sync status feedback messages | 2h |

**Deliverables:**
- [ ] `spn add X` syncs to detected editors
- [ ] `spn sync --status` shows sync state
- [ ] Claude Code + Cursor fully supported

### Phase 4: Unified Add Command (Week 4-5)

**Priority: Medium - DX enhancement**

| Task | Description | Effort |
|------|-------------|--------|
| 4.1 | Implement auto-detection by scope prefix | 4h |
| 4.2 | Add `--type` flag for explicit override | 2h |
| 4.3 | Route to appropriate handler (npm, skills.sh, own) | 4h |
| 4.4 | Update help text and documentation | 2h |
| 4.5 | Add fuzzy search integration | 6h |

**Deliverables:**
- [ ] `spn add neo4j` auto-detects MCP
- [ ] `spn add brainstorming` auto-detects skill
- [ ] Fuzzy search works in interactive mode

### Phase 5: Status & Check Commands (Week 5-6)

**Priority: Medium - Visibility**

| Task | Description | Effort |
|------|-------------|--------|
| 5.1 | Implement `spn status` with ASCII UI | 6h |
| 5.2 | Add focused views (--packages, --mcp, etc.) | 4h |
| 5.3 | Implement `spn check` validation | 6h |
| 5.4 | Add `--fix` auto-fix capability | 4h |
| 5.5 | Add JSON output for scripting | 2h |

**Deliverables:**
- [ ] `spn status` shows complete overview
- [ ] `spn check` validates configuration
- [ ] `spn check --fix` auto-repairs issues

### Phase 6: Init Wizard & Presets (Week 6-7)

**Priority: Medium - Onboarding**

| Task | Description | Effort |
|------|-------------|--------|
| 6.1 | Implement interactive `spn init` wizard | 8h |
| 6.2 | Create built-in presets (ai-workflow, web-app, cli-tool) | 4h |
| 6.3 | Implement `spn preset` commands | 4h |
| 6.4 | Add preset save/export functionality | 4h |

**Deliverables:**
- [ ] `spn init` interactive wizard works
- [ ] `spn init --preset ai-workflow` works
- [ ] Custom presets can be saved

### Phase 7: Groups & Shortcuts (Week 7-8)

**Priority: Low - Power user features**

| Task | Description | Effort |
|------|-------------|--------|
| 7.1 | Implement package groups | 6h |
| 7.2 | Add `spn group` commands | 4h |
| 7.3 | Implement command shortcuts (a, r, i, u, s, c) | 2h |
| 7.4 | Add shell completion scripts | 4h |

**Deliverables:**
- [ ] `spn add @group/ai-dev` works
- [ ] Shortcuts work (spn a, spn s, etc.)
- [ ] Shell completions available

### Phase 8: Polish & Documentation (Week 8)

**Priority: Low - Final touches**

| Task | Description | Effort |
|------|-------------|--------|
| 8.1 | Update README with new commands | 4h |
| 8.2 | Add migration guide from v0.2 | 4h |
| 8.3 | Create video walkthrough | 4h |
| 8.4 | Add `spn doctor` comprehensive checks | 4h |

**Deliverables:**
- [ ] Documentation complete
- [ ] Migration guide available
- [ ] v0.3.0 ready for release

---

## 15. Open Questions (Resolved)

| Question | Decision | Rationale |
|----------|----------|-----------|
| Schema sync to NovaNet | No auto-run | User should explicitly run `novanet schema add` |
| Job activation | No auto-register | User should explicitly run `nika jobs register` |
| Workflow registration | Yes, indexed | `nika run @name` syntax enabled by default |

---

## 16. Decision Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| MCP source | `~/.spn/mcp.yaml` | Single source, shared by all tools |
| Auto-sync | Configurable (guided first) | Best DX without surprises |
| Inheritance | global → project → workflow | Flexibility with sane defaults |
| spn nk/nv | Keep as proxies | Shows unified ecosystem |
| Unified add | Auto-detect by scope | Simpler DX, one command |
| Status command | ASCII UI with focused views | Quick overview without leaving terminal |
| Presets | YAML-based with registry support | Shareable, versionable |
| Groups | Local + shared groups | Batch install common setups |
| Shortcuts | Single-letter aliases | Power user efficiency |

---

## 17. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to first workflow | < 2 min | `spn init --preset ai-workflow` |
| Commands to add MCP | 1 | `spn add neo4j` |
| Sync correctness | 100% | All editors in sync after add |
| Check coverage | > 90% | All config issues detected |
| User satisfaction | > 4.5/5 | Survey after v0.3.0 |

---

## Appendix A: Migration from v0.2

```bash
# 1. Backup existing config
cp ~/.spn/config.yaml ~/.spn/config.yaml.bak

# 2. Update spn
brew upgrade spn

# 3. Run migration
spn migrate

# 4. Verify
spn check
spn status
```

**Migration handles:**
- Moving MCP from editor configs to `~/.spn/mcp.yaml`
- Converting old skill paths to new structure
- Updating project `spn.yaml` format

---

## Appendix B: Quick Reference Card

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  spn QUICK REFERENCE                                                          ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  ADD PACKAGES                          STATUS & VALIDATION                    ║
║  spn add <pkg>      Auto-detect        spn status        Overview             ║
║  spn add <pkg> -g   Global             spn check         Validate             ║
║  spn add <pkg> -p   Project            spn check --fix   Auto-fix             ║
║                                                                               ║
║  SHORTCUTS                             SYNC                                   ║
║  spn a X            add                spn sync          Sync all             ║
║  spn r X            remove             spn sync -t X     Sync to X            ║
║  spn i              install                                                   ║
║  spn u X            update             INIT                                   ║
║  spn s              status             spn init          Wizard               ║
║  spn c              check              spn init --preset Preset               ║
║                                                                               ║
║  GROUPS                                PROXIES                                ║
║  spn add @group/X   Install group      spn nk <args>     → nika               ║
║  spn group list     List groups        spn nv <args>     → novanet            ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```
