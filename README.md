# supernovae-cli

> Package manager for AI workflows, schemas, skills, and MCP servers.

**Status:** Pre-release | **Version:** 0.5.0

---

## 🦋 🐔 🐤 Mascots & Hierarchy

Understanding the SuperNovae ecosystem:

```
                            🦋 NIKA (Papillon)
                                 Runtime
                      Orchestrates the 5 semantic verbs
                                    │
        ┌───────────────┬───────────┼───────────┬───────────────┐
        │               │           │           │               │
        ▼               ▼           ▼           ▼               ▼
     infer:          exec:       fetch:     invoke:        agent: 🐔
      LLM           Shell        HTTP         MCP       (Space Chicken)
                                                              │
                                                        spawn_agent
                                                              │
                                                  ┌───────────┼───────────┐
                                                  ▼           ▼           ▼
                                                 🐤          🐤          🐤
                                            (Subagents - Poussins)
```

| Mascot | Role | What it does |
|--------|------|--------------|
| 🦋 **Nika** | **Runtime** | Executes workflows, runs chat UI, launches agents |
| 🐔 **Agent** | **One verb** | Multi-turn loop with MCP tools, spawns subagents |
| 🐤 **Subagent** | **Spawned** | Subtask execution, depth-limited |

> **Important:** Nika is NOT an agent. Nika is the runtime that orchestrates agents.

---

## Overview

`spn` is the CLI for the SuperNovae ecosystem. It manages AI packages with a unique ownership model:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  SUPERNOVAE OWNERSHIP MODEL                                                      │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ✅ WE OWN (unique, no competitor)                                              │
│  ├── workflows/       → Nika YAML DAG workflows (.nika.yaml)                    │
│  ├── schemas/         → NovaNet schemas (node-classes, arc-classes)             │
│  └── jobs/            → Nika jobs (cron, webhook, watch triggers)               │
│                                                                                  │
│  🔗 INTEROP (proxy to existing ecosystems)                                      │
│  ├── skills           → skills.sh (57K+ skills, 20+ AI assistants)              │
│  └── mcp              → npm (97M+ downloads/month, standard MCP)                │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Installation

```bash
# From source
cargo install --path .

# Or via cargo
cargo install spn
```

---

## Quick Start

```bash
# Initialize project
spn init

# Add packages (owned)
spn add @nika/generate-page        # Workflow
spn add @novanet/core-schema       # Schema

# Add packages (interop)
spn skill add brainstorming        # → skills.sh
spn mcp add neo4j                  # → npm

# Install from manifest
spn install

# Sync to editors
spn sync
```

---

## Commands

### Package Management (Owned)

```bash
spn add <package>           # Add package
spn remove <package>        # Remove package
spn install                 # Install from spn.yaml
spn install --frozen        # Exact versions from lock
spn update [package]        # Update packages
spn outdated                # List outdated
spn search <query>          # Search registry
spn info <package>          # Package details
spn list                    # List installed
spn publish                 # Publish package
spn version <bump>          # Bump version
```

### Interop (Skills & MCP)

```bash
# Skills (via skills.sh)
spn skill add <name>        # Add skill
spn skill remove <name>     # Remove skill
spn skill list              # List installed
spn skill search <query>    # Search skills.sh

# MCP Servers (via npm)
spn mcp add <name>          # Add MCP server
spn mcp remove <name>       # Remove server
spn mcp list                # List installed
spn mcp test <name>         # Test connection
```

### Nika Integration

```bash
spn nk run <file>           # Run workflow
spn nk check <file>         # Validate workflow
spn nk studio               # Open TUI
spn nk jobs start           # Start jobs daemon
spn nk jobs status          # Check jobs
spn nk jobs stop            # Stop daemon
```

### NovaNet Integration

```bash
spn nv tui                  # Open TUI
spn nv query <query>        # Query knowledge graph
spn nv mcp start            # Start MCP server
spn nv add-node <name>      # Add node type
spn nv add-arc <name>       # Add arc type
spn nv override <name>      # Override node
spn nv db start             # Start Neo4j
spn nv db seed              # Seed database
spn nv db reset             # Reset database
```

### Cross-Editor Sync

```bash
spn sync                    # Sync all enabled editors
spn sync --target claude    # Sync specific editor
spn sync enable cursor      # Enable editor
spn sync disable vscode     # Disable editor
spn sync --status           # Show sync status
spn sync --dry-run          # Preview changes
```

### Configuration

```bash
spn config show             # Show merged config
spn config where            # Show config locations
spn config list --show-origin # Config with origins
spn config edit             # Edit project config
spn config edit --local     # Edit local overrides
spn config edit --user      # Edit user config
```

### Schema Management

```bash
spn schema status           # Show schema state
spn schema validate         # Validate coherence
spn schema resolve          # Show merged schema
spn schema diff             # Show changes
spn schema exclude <node>   # Exclude node
spn schema include <node>   # Re-include node
```

### Utilities

```bash
spn doctor                  # System diagnostic
spn init                    # Initialize project
spn init --local            # Create local config
spn init --mcp              # Create MCP config
spn help [topic]            # Show help
```

---

## Configuration Files

| File | Committed | Purpose |
|------|-----------|---------|
| `spn.yaml` | ✅ | Package manifest |
| `spn.lock` | ✅ | Resolved versions |
| `spn.local.yaml` | ❌ | Local overrides |
| `.mcp.yaml` | ✅ | Team MCP servers |
| `.mcp.local.yaml` | ❌ | Personal API keys |

### spn.yaml Example

```yaml
name: my-project
version: 0.1.0

workflows:
  - "@nika/generate-page@^1.0.0"
  - "@nika/seo-audit@^2.0.0"

schemas:
  - "@novanet/core-schema@^0.14.0"

skills:
  - "brainstorming"
  - "superpowers/tdd"

mcp:
  - "neo4j"
  - "perplexity"

sync:
  claude: true
  cursor: false
  nika: true
```

---

## MCP Aliases

48 MCP servers are available via short aliases:

```bash
spn mcp add neo4j       # → @neo4j/mcp-server-neo4j
spn mcp add github      # → @modelcontextprotocol/server-github
spn mcp add filesystem  # → @modelcontextprotocol/server-filesystem
spn mcp add perplexity  # → perplexity-mcp
spn mcp add firecrawl   # → firecrawl-mcp
spn mcp add supabase    # → @supabase/mcp-server-supabase
```

See `spn mcp list` for all available aliases.

---

## Related Repositories

| Repo | Description |
|------|-------------|
| [supernovae-registry](https://github.com/SuperNovae-studio/supernovae-registry) | Public package registry |
| [supernovae-powers](https://github.com/SuperNovae-studio/supernovae-powers) | Private package registry |
| [supernovae-index](https://github.com/SuperNovae-studio/supernovae-index) | Sparse package index |
| [nika](https://github.com/SuperNovae-studio/nika) | Workflow engine |
| [novanet](https://github.com/SuperNovae-studio/novanet) | Knowledge graph |

---

## License

MIT © SuperNovae Studio
