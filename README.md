# supernovae-cli

> Package manager for AI workflows, schemas, skills, and MCP servers.

**Status:** Pre-release | **Version:** 0.5.0 | **License:** MIT

---

## Table of Contents

- [Overview](#overview)
- [The SuperNovae Ecosystem](#the-supernovae-ecosystem)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Commands Reference](#commands-reference)
- [Configuration](#configuration)
- [MCP Server Integration](#mcp-server-integration)
- [Related Projects](#related-projects)
- [Contributing](#contributing)
- [License](#license)

---

## Overview

`spn` is the unified CLI for the SuperNovae ecosystem. It manages AI packages with a unique ownership model that distinguishes between packages we own (workflows, schemas, jobs) and packages we proxy from existing ecosystems (skills, MCP servers).

```
+-----------------------------------------------------------------------------+
|  SUPERNOVAE OWNERSHIP MODEL                                                  |
+-----------------------------------------------------------------------------+
|                                                                              |
|  WE OWN (unique, no competitor)                                              |
|  +-- workflows/       -> Nika YAML DAG workflows (.nika.yaml)                |
|  +-- schemas/         -> NovaNet schemas (node-classes, arc-classes)         |
|  +-- jobs/            -> Nika jobs (cron, webhook, watch triggers)           |
|                                                                              |
|  INTEROP (proxy to existing ecosystems)                                      |
|  +-- skills           -> skills.sh (57K+ skills, 20+ AI assistants)          |
|  +-- mcp              -> npm (97M+ downloads/month, standard MCP)            |
|                                                                              |
+-----------------------------------------------------------------------------+
```

---

## The SuperNovae Ecosystem

### Mascots and Hierarchy

Understanding the relationship between components:

```
                            NIKA (Papillon)
                                 Runtime
                      Orchestrates the 5 semantic verbs
                                    |
        +---------------+-----------+-----------+---------------+
        |               |           |           |               |
        v               v           v           v               v
     infer:          exec:       fetch:     invoke:        agent:
      LLM           Shell        HTTP         MCP       (Space Chicken)
                                                              |
                                                        spawn_agent
                                                              |
                                                  +-----------+-----------+
                                                  v           v           v
                                            (Subagents - Poussins)
```

| Mascot | Role | What it does |
|--------|------|--------------|
| **Nika** | **Runtime** | Executes workflows, runs chat UI, launches agents |
| **Agent** | **One verb** | Multi-turn loop with MCP tools, spawns subagents |
| **Subagent** | **Spawned** | Subtask execution, depth-limited |

> **Important:** Nika is NOT an agent. Nika is the runtime that orchestrates agents.

### Package Types

The registry supports six package types, each with a specific scope prefix:

| Type | Scope | Description | Example |
|------|-------|-------------|---------|
| **workflow** | `@workflows/`, `@nika/` | YAML DAG definitions | `@nika/generate-page` |
| **agent** | `@agents/` | Agent configurations | `@agents/code-reviewer` |
| **skill** | `@skills/` | Reusable skill definitions | `@skills/brainstorming` |
| **prompt** | `@prompts/` | Prompt templates | `@prompts/seo-meta` |
| **job** | `@jobs/` | Scheduled/triggered jobs | `@jobs/daily-report` |
| **schema** | `@schemas/`, `@novanet/` | NovaNet graph schemas | `@novanet/core-schema` |

---

## Installation

### From Homebrew (Recommended)

```bash
# Add the SuperNovae tap
brew tap supernovae-st/tap

# Install spn (automatically installs nika as dependency)
brew install supernovae-st/tap/spn
```

### From Source

```bash
# Clone the repository
git clone https://github.com/supernovae-st/supernovae-cli
cd supernovae-cli

# Build and install
cargo install --path .
```

### From crates.io

```bash
cargo install spn
```

### Verify Installation

```bash
# Check spn version
spn --version

# Run diagnostic
spn doctor
```

---

## Quick Start

### 1. Initialize a Project

```bash
# Create a new project with spn.yaml
spn init

# Or create with specific options
spn init --name my-project --local
```

### 2. Add Packages

```bash
# Add a workflow package
spn add @nika/generate-page

# Add a schema package
spn add @novanet/core-schema

# Add a skill (proxied from skills.sh)
spn skill add brainstorming

# Add an MCP server (proxied from npm)
spn mcp add neo4j
```

### 3. Install Dependencies

```bash
# Install all packages from spn.yaml
spn install

# Install with exact versions from lockfile
spn install --frozen
```

### 4. Sync to Editors

```bash
# Sync configuration to all enabled editors
spn sync

# Sync to a specific editor
spn sync --target claude
```

---

## Architecture

### How spn Works

```
+-----------------------------------------------------------------------------+
|                           spn ARCHITECTURE                                   |
+-----------------------------------------------------------------------------+
|                                                                              |
|  User Commands                                                               |
|       |                                                                      |
|       v                                                                      |
|  +----------+     +-----------------+     +------------------+               |
|  |   CLI    |---->|  Package Mgmt   |---->|  Local Storage   |               |
|  | (clap)   |     | (add/install)   |     | (~/.spn/cache)   |               |
|  +----------+     +-----------------+     +------------------+               |
|       |                   |                       |                          |
|       |                   v                       v                          |
|       |           +---------------+       +---------------+                  |
|       |           | Index Client  |       |   Downloader  |                  |
|       |           | (sparse idx)  |       |  (tar.gz)     |                  |
|       |           +---------------+       +---------------+                  |
|       |                   |                       |                          |
|       v                   v                       v                          |
|  +----------+     +------------------+    +------------------+               |
|  |  Proxy   |     |    Registry      |    |   GitHub Rel.    |               |
|  | nk/nv    |     | (index.json)     |    |   (tarballs)     |               |
|  +----------+     +------------------+    +------------------+               |
|       |                                                                      |
|       v                                                                      |
|  +---------+  +-----------+                                                  |
|  |  Nika   |  |  NovaNet  |                                                  |
|  | (body)  |  |  (brain)  |                                                  |
|  +---------+  +-----------+                                                  |
|                                                                              |
+-----------------------------------------------------------------------------+
```

### Directory Structure

```
~/.spn/                          # Global spn directory
+-- cache/                       # Downloaded package cache
|   +-- workflows/               # Cached workflow packages
|   +-- schemas/                 # Cached schema packages
|   +-- skills/                  # Cached skill definitions
|   +-- prompts/                 # Cached prompt templates
|   +-- jobs/                    # Cached job definitions
|   +-- agents/                  # Cached agent configs
+-- config.yaml                  # User configuration

./                               # Project directory
+-- spn.yaml                     # Package manifest (committed)
+-- spn.lock                     # Resolved versions (committed)
+-- spn.local.yaml               # Local overrides (gitignored)
+-- .mcp.yaml                    # Team MCP servers (committed)
+-- .mcp.local.yaml              # Personal API keys (gitignored)
```

### The Registry

The SuperNovae registry uses a **sparse index** pattern inspired by Cargo:

1. **Index Repository**: Contains `index.json` files for each package
2. **Tarballs**: Stored as GitHub Releases on the registry repo
3. **Checksums**: SHA256 verification for all downloads

```
supernovae-index/
+-- @workflows/
|   +-- dev-productivity/
|       +-- code-review/
|           +-- index.json       # Version metadata
+-- @schemas/
|   +-- novanet/
|       +-- core-schema/
|           +-- index.json
```

---

## Commands Reference

### Package Management

| Command | Description |
|---------|-------------|
| `spn add <package>` | Add a package to manifest and install |
| `spn remove <package>` | Remove a package |
| `spn install` | Install all packages from manifest |
| `spn install --frozen` | Install exact versions from lockfile |
| `spn update [package]` | Update packages to latest compatible |
| `spn outdated` | List packages with available updates |
| `spn search <query>` | Search the registry |
| `spn info <package>` | Show package details |
| `spn list` | List installed packages |
| `spn publish` | Publish package to registry |
| `spn version <bump>` | Bump package version |

### Skills (via skills.sh)

| Command | Description |
|---------|-------------|
| `spn skill add <name>` | Add a skill from skills.sh |
| `spn skill remove <name>` | Remove a skill |
| `spn skill list` | List installed skills |
| `spn skill search <query>` | Search skills.sh |

### MCP Servers (via npm)

| Command | Description |
|---------|-------------|
| `spn mcp add <name>` | Add an MCP server |
| `spn mcp remove <name>` | Remove a server |
| `spn mcp list` | List installed servers |
| `spn mcp test <name>` | Test server connection |

### Nika Integration

| Command | Description |
|---------|-------------|
| `spn nk run <file>` | Run a Nika workflow |
| `spn nk check <file>` | Validate workflow syntax |
| `spn nk studio` | Open Nika Studio TUI |
| `spn nk jobs start` | Start the jobs daemon |
| `spn nk jobs status` | Check daemon status |
| `spn nk jobs stop` | Stop the daemon |

### NovaNet Integration

| Command | Description |
|---------|-------------|
| `spn nv tui` | Open NovaNet TUI |
| `spn nv query <query>` | Query the knowledge graph |
| `spn nv mcp start` | Start MCP server |
| `spn nv add-node <name>` | Add a node type |
| `spn nv add-arc <name>` | Add an arc type |
| `spn nv override <name>` | Override a node |
| `spn nv db start` | Start Neo4j |
| `spn nv db seed` | Seed database |
| `spn nv db reset` | Reset database |

### Editor Sync

| Command | Description |
|---------|-------------|
| `spn sync` | Sync to all enabled editors |
| `spn sync --target <editor>` | Sync to specific editor |
| `spn sync enable <editor>` | Enable editor sync |
| `spn sync disable <editor>` | Disable editor sync |
| `spn sync --status` | Show sync status |
| `spn sync --dry-run` | Preview changes |

### Configuration

| Command | Description |
|---------|-------------|
| `spn config show` | Show merged configuration |
| `spn config where` | Show config file locations |
| `spn config list --show-origin` | Show config with origins |
| `spn config edit` | Edit project config |
| `spn config edit --local` | Edit local overrides |
| `spn config edit --user` | Edit user config |

### Schema Management

| Command | Description |
|---------|-------------|
| `spn schema status` | Show schema state |
| `spn schema validate` | Validate coherence |
| `spn schema resolve` | Show merged schema |
| `spn schema diff` | Show changes |
| `spn schema exclude <node>` | Exclude a node |
| `spn schema include <node>` | Re-include a node |

### Utilities

| Command | Description |
|---------|-------------|
| `spn doctor` | Run system diagnostic |
| `spn init` | Initialize project |
| `spn help [topic]` | Show help |

---

## Configuration

### spn.yaml (Package Manifest)

```yaml
name: my-project
version: 0.1.0

# Owned packages
workflows:
  - "@nika/generate-page@^1.0.0"
  - "@nika/seo-audit@^2.0.0"

schemas:
  - "@novanet/core-schema@^0.14.0"

jobs:
  - "@jobs/daily-report@^1.0.0"

# Interop packages
skills:
  - "brainstorming"
  - "superpowers/tdd"

mcp:
  - "neo4j"
  - "perplexity"

# Editor sync configuration
sync:
  claude: true
  cursor: false
  nika: true
```

### spn.local.yaml (Local Overrides)

```yaml
# Override registry URL for development
registry:
  url: "http://localhost:8080"

# Local-only packages
workflows:
  - "@local/my-workflow@file:../my-workflow"
```

### .mcp.yaml (Team MCP Servers)

```yaml
servers:
  neo4j:
    package: "@neo4j/mcp-server-neo4j"
    env:
      NEO4J_URI: "bolt://localhost:7687"

  github:
    package: "@modelcontextprotocol/server-github"
```

### .mcp.local.yaml (Personal API Keys)

```yaml
# GITIGNORED - Never commit!
servers:
  neo4j:
    env:
      NEO4J_PASSWORD: "your-password"

  github:
    env:
      GITHUB_TOKEN: "ghp_xxxxx"
```

---

## MCP Server Integration

### Available Aliases

`spn` provides 48 short aliases for popular MCP servers:

| Alias | Full Package Name |
|-------|-------------------|
| `neo4j` | `@neo4j/mcp-server-neo4j` |
| `github` | `@modelcontextprotocol/server-github` |
| `filesystem` | `@modelcontextprotocol/server-filesystem` |
| `perplexity` | `perplexity-mcp` |
| `firecrawl` | `firecrawl-mcp` |
| `supabase` | `@supabase/mcp-server-supabase` |
| `postgres` | `@modelcontextprotocol/server-postgres` |
| `sqlite` | `@modelcontextprotocol/server-sqlite` |
| `slack` | `@modelcontextprotocol/server-slack` |
| `puppeteer` | `@anthropic/mcp-puppeteer` |

See `spn mcp list --all` for the complete list.

### Adding MCP Servers

```bash
# Using alias
spn mcp add neo4j

# Using full package name
spn mcp add @neo4j/mcp-server-neo4j

# With custom configuration
spn mcp add neo4j --env NEO4J_URI=bolt://localhost:7687
```

---

## Related Projects

| Repository | Description |
|------------|-------------|
| [nika](https://github.com/supernovae-st/nika) | Semantic YAML workflow engine |
| [novanet](https://github.com/supernovae-st/novanet) | Knowledge graph for localization |
| [supernovae-registry](https://github.com/supernovae-st/supernovae-registry) | Public package registry |
| [supernovae-index](https://github.com/supernovae-st/supernovae-index) | Sparse package index |
| [homebrew-tap](https://github.com/supernovae-st/homebrew-tap) | Homebrew formulas |

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing`)
3. Make changes with tests
4. Run checks (`cargo fmt && cargo clippy && cargo test`)
5. Commit (`git commit -m "feat(scope): description"`)
6. Push and create PR

### Development Setup

```bash
# Clone with DX setup
git clone https://github.com/supernovae-st/supernovae-cli
cd supernovae-cli

# Link DX configuration (if available)
ln -s ../supernovae-agi/dx/.claude .claude

# Build
cargo build

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- doctor
```

---

## License

MIT (c) SuperNovae Studio

---

**Links:** [Documentation](https://docs.supernovae.studio) | [Registry](https://registry.supernovae.studio) | [Discord](https://discord.gg/supernovae)
