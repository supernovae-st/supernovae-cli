<div align="center">

<!-- SuperNovae ASCII Logo with stars -->
```
        ✦                                              ✦
     ✧  ███████╗██████╗ ███╗   ██╗    ██████╗██╗     ██╗  ✧
     ·  ██╔════╝██╔══██╗████╗  ██║   ██╔════╝██║     ██║  ·
        ███████╗██████╔╝██╔██╗ ██║   ██║     ██║     ██║
     ·  ╚════██║██╔═══╝ ██║╚██╗██║   ██║     ██║     ██║  ·
     ✧  ███████║██║     ██║ ╚████║   ╚██████╗███████╗██║  ✧
        ╚══════╝╚═╝     ╚═╝  ╚═══╝    ╚═════╝╚══════╝╚═╝
        ✦                                              ✦
```

# 🌟 SuperNovae CLI

### The Agentic AI Toolkit

<sup>✨ Local models • MCP servers • LLM providers • Secrets • Status Dashboard ✨</sup>

<!-- Primary Badges (Dynamic) -->
[![Crates.io](https://img.shields.io/crates/v/spn-cli?style=for-the-badge&logo=rust&logoColor=white&labelColor=6366f1&color=000000)](https://crates.io/crates/spn-cli)
[![Docker](https://img.shields.io/docker/v/supernovae-st/spn?style=for-the-badge&logo=docker&logoColor=white&labelColor=2496ED&color=000000&label=docker)](https://ghcr.io/supernovae-st/spn)
[![Rust](https://img.shields.io/badge/rust_1.85+-f97316?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/crates/l/spn-cli?style=for-the-badge&logo=opensourceinitiative&logoColor=white&color=10b981)](LICENSE)

<!-- GitHub Badges (Dynamic) -->
[![CI](https://img.shields.io/github/actions/workflow/status/supernovae-st/supernovae-cli/test.yml?style=flat-square&logo=github&label=CI)](https://github.com/supernovae-st/supernovae-cli/actions)
[![Release](https://img.shields.io/github/v/release/supernovae-st/supernovae-cli?style=flat-square&logo=github&label=release)](https://github.com/supernovae-st/supernovae-cli/releases/latest)
[![Downloads](https://img.shields.io/crates/d/spn-cli?style=flat-square&logo=rust&label=downloads)](https://crates.io/crates/spn-cli)
[![Stars](https://img.shields.io/github/stars/supernovae-st/supernovae-cli?style=flat-square&logo=github&label=stars)](https://github.com/supernovae-st/supernovae-cli/stargazers)

<!-- Feature Badges -->
[![MCP](https://img.shields.io/badge/🔌_MCP_servers-48_aliases-ec4899?style=flat-square)](#-mcp-servers)
[![Skills](https://img.shields.io/badge/🎯_skills-57K+_from_skills.sh-f59e0b?style=flat-square)](#-skills)
[![Security](https://img.shields.io/badge/🔐_OS_keychain-native-10b981?style=flat-square)](#-security)
[![Config](https://img.shields.io/badge/⚙️_config_scopes-3_levels-6366f1?style=flat-square)](#three-level-config-scope)

<!-- Navigation -->
<p>
<a href="#-installation">🚀 Install</a> •
<a href="#-quick-start">⚡ Quick Start</a> •
<a href="#-architecture">🏗️ Architecture</a> •
<a href="#-commands-reference">📖 Commands</a> •
<a href="#-configuration-files">📄 Config</a> •
<a href="#-contributing">🤝 Contribute</a>
</p>

---

**spn** — **The Agentic AI Toolkit**<br>
Local models, MCP servers, LLM providers, and secrets. Works with **Ollama**, **Claude**, **OpenAI**, and **any AI editor** (Claude Code, Cursor, Windsurf, Continue.dev).

<br>

```
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   🌟  "The Agentic AI Toolkit"                                        ║
    ║                                                                       ║
    ║       • 7 LLM providers           • Any AI editor                     ║
    ║       • 48 MCP server aliases     • OS Keychain secrets               ║
    ║       • Ollama model manager      • Status dashboard                  ║
    ║       • Nika direct integration   • Open source first                 ║
    ║                                                                       ║
    ╚═══════════════════════════════════════════════════════════════════════╝
```

</div>

<br>

---

## ✨ Highlights

- 🦙 **Open Source First** — Works with [Ollama](https://ollama.ai), [Continue.dev](https://continue.dev), and local models out of the box
- 🤖 **7 LLM Providers** — Claude, OpenAI, Mistral, Groq, DeepSeek, Gemini, Ollama
- 🦋 **Nika Direct Integration** — MCP configs read directly from `~/.spn/mcp.yaml` (no sync needed)
- 📦 **Rich Ecosystem** — 48 MCP server aliases, 57K+ skills from [skills.sh](https://skills.sh)
- 🔐 **Secure by Design** — OS Keychain integration with memory protection
- ⚡ **Cargo-style Index** — Sparse registry for lightning-fast package resolution
- 🎯 **Three-Level Config** — Global/Team/Local scope hierarchy (like git)
- 🔄 **Universal Sync** — Claude Code, Cursor, Windsurf, VS Code — any AI editor

<br>

---

## 📑 Table of Contents

- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Architecture](#-architecture)
  - [Three-Level Config Scope](#three-level-config-scope)
  - [Selective Package Sync](#selective-package-sync)
  - [Security Architecture](#security-architecture)
- [Commands Reference](#-commands-reference)
  - [Status Dashboard](#-status-dashboard)
  - [Package Management](#-package-management)
  - [Configuration Management](#-configuration-management)
  - [Onboarding](#-onboarding)
  - [Secrets Management](#-secrets-management)
  - [Security](#-security)
  - [Skills](#-skills)
  - [MCP Servers](#-mcp-servers)
  - [Nika Integration](#-nika-integration)
  - [NovaNet Integration](#-novanet-integration)
  - [Sync & Editor Integration](#-sync--editor-integration)
- [Configuration Files](#-configuration-files)
- [The SuperNovae Ecosystem](#-the-supernovae-ecosystem)
- [FAQ](#-faq)
- [Troubleshooting](#-troubleshooting)
- [Directory Structure](#-directory-structure)
- [Contributing](#-contributing)

<br>

---

## 🚀 Installation

### Homebrew (Recommended)

```bash
brew install supernovae-st/tap/spn
```

### Cargo

```bash
cargo install spn
```

### From Source

```bash
git clone https://github.com/supernovae-st/supernovae-cli
cd supernovae-cli
cargo install --path .
```

### Docker

```bash
# Run directly
docker run --rm ghcr.io/supernovae-st/spn:latest --version

# With project mount
docker run --rm -v $(pwd):/workspace ghcr.io/supernovae-st/spn:latest list

# With API keys (env vars)
docker run --rm \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  ghcr.io/supernovae-st/spn:latest provider test anthropic
```

> **Note:** Docker cannot access OS Keychain. Use environment variables for secrets in containers.

> **Platform Support:** macOS and Linux are fully supported. Windows is not yet supported (the daemon uses Unix sockets).

### Verify Installation

```bash
spn --version  # spn-cli 0.15.0
spn doctor     # System diagnostic

# Enable shell completions
spn completion bash >> ~/.bashrc
spn completion zsh >> ~/.zshrc
spn completion fish > ~/.config/fish/completions/spn.fish
```

<br>

---

## ⚡ Quick Start

```bash
# 1. Initialize a project
spn init

# 2. Add packages
spn add @nika/generate-page      # Workflow
spn skill add brainstorming      # Skill (via skills.sh)
spn mcp add neo4j                # MCP server (via npm)

# 3. Configure providers
spn provider set anthropic       # Store API key in OS Keychain

# 4. Sync to your editor
spn sync --interactive           # Preview changes before syncing
```

<br>

---

## 🏗️ Architecture

### Three-Level Config Scope

`spn` uses a three-level configuration hierarchy following industry standards (npm, cargo, git):

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': { 'primaryColor': '#6366f1', 'primaryTextColor': '#fff', 'primaryBorderColor': '#818cf8', 'lineColor': '#a5b4fc', 'secondaryColor': '#1e1b4b', 'tertiaryColor': '#312e81'}}}%%
flowchart TB
    subgraph GLOBAL["🌍 Global Scope (~/.spn/config.toml)"]
        G1["Provider settings"]
        G2["Default MCP servers"]
        G3["User preferences"]
    end

    subgraph TEAM["👥 Team Scope (./mcp.yaml)"]
        T1["Project MCP servers"]
        T2["Shared workflows"]
        T3["Team schemas"]
    end

    subgraph LOCAL["💻 Local Scope (./.spn/local.yaml)"]
        L1["Developer overrides"]
        L2["Local API keys"]
        L3["Debug settings"]
    end

    GLOBAL -->|"Merged"| RESOLVER["⚙️ Config Resolver"]
    TEAM -->|"Overrides"| RESOLVER
    LOCAL -->|"Final Override"| RESOLVER
    RESOLVER --> APP["🚀 Application"]

    style GLOBAL fill:#1e293b
    style TEAM fill:#334155
    style LOCAL fill:#475569
    style RESOLVER fill:#6366f1
```

**Precedence:** Local > Team > Global (innermost wins)

#### Global Scope (`~/.spn/config.toml`)

User-level settings that apply to all projects:

```toml
[providers.anthropic]
model = "claude-opus-4"
endpoint = "https://api.anthropic.com"

[providers.openai]
model = "gpt-4"

[sync]
enabled_editors = ["claude-code"]
auto_sync = true

[servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
```

#### Team Scope (`./mcp.yaml`)

Project-level MCP servers shared with the team:

```yaml
servers:
  neo4j:
    command: "npx"
    args: ["-y", "@neo4j/mcp-server-neo4j"]
    env:
      NEO4J_URI: "bolt://localhost:7687"

  perplexity:
    command: "npx"
    args: ["-y", "perplexity-mcp"]
```

**Committed to git** — Shared with team members.

#### Local Scope (`./.spn/local.yaml`)

Developer-specific overrides:

```yaml
servers:
  neo4j:
    env:
      NEO4J_PASSWORD: "dev-password"
      NEO4J_DATABASE: "test"

providers:
  anthropic:
    model: "claude-haiku-4"  # Override for local testing
```

**Gitignored** — Never committed.

<br>

### Config Resolution Flow

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': { 'primaryColor': '#10b981', 'primaryTextColor': '#fff', 'primaryBorderColor': '#34d399', 'lineColor': '#6ee7b7', 'secondaryColor': '#064e3b', 'tertiaryColor': '#065f46'}}}%%
sequenceDiagram
    participant User
    participant CLI as spn CLI
    participant Global as 🌍 Global
    participant Team as 👥 Team
    participant Local as 💻 Local
    participant Resolver
    participant App

    User->>CLI: spn config show
    CLI->>Global: Load ~/.spn/config.toml
    Global-->>CLI: Base config
    CLI->>Team: Load ./mcp.yaml
    Team-->>CLI: Project config
    CLI->>Local: Load ./.spn/local.yaml
    Local-->>CLI: Overrides
    CLI->>Resolver: Merge all configs
    Note over Resolver: Local > Team > Global
    Resolver-->>CLI: Resolved config
    CLI->>App: Apply configuration
    App-->>User: Display merged result
```

<br>

### Selective Package Sync

Not all package types need editor integration. `spn` intelligently determines what to sync based on package type:

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': { 'primaryColor': '#f59e0b', 'primaryTextColor': '#fff', 'primaryBorderColor': '#fbbf24', 'lineColor': '#fcd34d', 'secondaryColor': '#78350f', 'tertiaryColor': '#92400e'}}}%%
flowchart LR
    subgraph PACKAGES["📦 Installed Packages"]
        SKILLS["@skills/<br/>brainstorming"]
        WORKFLOWS["@workflows/<br/>generate-page"]
        AGENTS["@agents/<br/>code-reviewer"]
        PROMPTS["@prompts/<br/>seo-meta"]
    end

    subgraph DECISION["🎯 Sync Decision"]
        SKILLS --> YES1["✅ Requires Sync"]
        WORKFLOWS --> NO1["❌ No Sync"]
        AGENTS --> NO2["❌ No Sync"]
        PROMPTS --> NO3["❌ No Sync"]
    end

    subgraph REASON["📝 Reason"]
        YES1 --> R1["Skills need .claude/skills/<br/>for editor integration"]
        NO1 --> R2["Standalone execution<br/>via nika CLI"]
        NO2 --> R3["CLI subagents<br/>no editor presence needed"]
        NO3 --> R4["Template files<br/>accessed programmatically"]
    end

    subgraph EDITORS["📂 Editor Configs"]
        R1 --> ED[".claude/skills/<br/>.cursor/skills/<br/>.windsurf/skills/"]
    end

    style YES1 fill:#10b981
    style NO1 fill:#ef4444
    style NO2 fill:#ef4444
    style NO3 fill:#ef4444
```

#### Sync Behavior Table

| Package Type | Default Sync | Reason | Override |
|:-------------|:-------------|:-------|:---------|
| **@skills/** | ✅ YES | Requires `.claude/skills/` for editor integration | N/A |
| **@workflows/** | ❌ NO | Standalone execution via `nika` CLI | `integration.requires_sync: true` in manifest |
| **@agents/** | ❌ NO | CLI subagents, no editor presence needed | `integration.requires_sync: true` in manifest |
| **@prompts/** | ❌ NO | Template files accessed programmatically | `integration.requires_sync: true` in manifest |
| **@jobs/** | ❌ NO | Cron/webhook triggers, no editor integration | `integration.requires_sync: true` in manifest |
| **@schemas/** | ❌ NO | NovaNet database schemas, no editor integration | `integration.requires_sync: true` in manifest |

<br>

#### Sync Flow

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': { 'primaryColor': '#8b5cf6', 'primaryTextColor': '#fff', 'primaryBorderColor': '#a78bfa', 'lineColor': '#c4b5fd', 'secondaryColor': '#4c1d95', 'tertiaryColor': '#5b21b6'}}}%%
flowchart TB
    CMD["spn sync --interactive"]

    subgraph PHASE1["Phase 1: MCP Servers"]
        MCP_LOAD["Load MCP config<br/>from all scopes"]
        MCP_MERGE["Merge configs<br/>(Local > Team > Global)"]
        MCP_SYNC["Sync to editors:<br/>.claude/settings.json<br/>.cursor/mcp.json<br/>.windsurf/mcp.json"]
    end

    subgraph PHASE2["Phase 2: Package Sync"]
        PKG_LIST["List installed packages"]
        PKG_FILTER["Filter by requires_sync()"]
        PKG_LINK["Create symlinks:<br/>.claude/skills/<br/>.claude/hooks/"]
    end

    subgraph CONFIRM["Interactive Confirmation"]
        DIFF["Show colored diffs"]
        PROMPT["Ask for confirmation"]
        APPLY["Apply changes"]
    end

    CMD --> PHASE1
    MCP_LOAD --> MCP_MERGE
    MCP_MERGE --> MCP_SYNC
    PHASE1 --> PHASE2
    PKG_LIST --> PKG_FILTER
    PKG_FILTER --> PKG_LINK
    PHASE2 --> CONFIRM
    DIFF --> PROMPT
    PROMPT -->|Yes| APPLY
    PROMPT -->|No| CANCEL["❌ Cancel"]

    style CMD fill:#6366f1
    style APPLY fill:#10b981
    style CANCEL fill:#ef4444
```

<br>

### Security Architecture

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': { 'primaryColor': '#10b981', 'primaryTextColor': '#fff', 'primaryBorderColor': '#34d399', 'lineColor': '#6ee7b7', 'secondaryColor': '#064e3b', 'tertiaryColor': '#065f46'}}}%%
flowchart TB
    subgraph STORAGE["🔐 Layer 1: Storage"]
        KC["OS Keychain<br/>(macOS/Windows/Linux)"]
        ENV["Environment Variables"]
        DOT[".env Files"]
    end

    subgraph MEMORY["🧠 Layer 2: Memory Protection"]
        ZERO["Zeroizing&lt;T&gt;<br/>(Auto-clear on drop)"]
        SECRET["SecretString<br/>(No Debug/Display)"]
        MLOCK["mlock()<br/>(Prevent swap to disk)"]
        DUMP["MADV_DONTDUMP<br/>(Exclude from core dumps)"]
    end

    subgraph VALIDATION["✅ Layer 3: Validation"]
        FORMAT["Format Check<br/>(Provider-specific patterns)"]
        MASK["Masked Display<br/>(sk-ant...X format)"]
        TEST["Connection Test<br/>(Optional)"]
    end

    KC -->|Priority 1| ZERO
    ENV -->|Priority 2| ZERO
    DOT -->|Priority 3| ZERO
    ZERO --> SECRET
    SECRET --> MLOCK
    MLOCK --> DUMP
    DUMP --> FORMAT
    FORMAT --> MASK
    MASK --> TEST
    TEST --> APP["🚀 Application"]

    style KC fill:#10b981
    style ENV fill:#f59e0b
    style DOT fill:#ef4444
    style APP fill:#6366f1
```

**Defense-in-depth protection:**

| Layer | Protection | Technology | Platform |
|:------|:-----------|:-----------|:---------|
| Storage | Encrypted at rest | macOS Keychain | macOS |
| Storage | Encrypted at rest | Windows Credential Manager | Windows |
| Storage | Encrypted at rest | Linux Secret Service | Linux |
| Memory | Auto-clear on drop | `zeroize` crate | All |
| Memory | Prevent debug exposure | `secrecy` crate | All |
| Memory | Prevent swap to disk | `mlock()` via `libc` | Unix |
| Memory | Exclude from core dumps | `MADV_DONTDUMP` | Linux |
| Validation | Format verification | Provider-specific regex | All |
| Display | Masked output | `sk-ant...X` format | All |

<br>

---

## 📖 Commands Reference

### 📊 Status Dashboard

View the complete state of your SuperNovae ecosystem at a glance.

#### `spn status [--json]`

Display the unified status dashboard showing local models, credentials, MCP servers, and daemon status.

```bash
# ASCII dashboard (default)
spn status

# Machine-readable JSON output
spn status --json
```

**Output:**
```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  ✦ spn status                                    The Agentic AI Toolkit  ✦  ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

┌─ 🦙 LOCAL MODELS ────────────────────────────────────────────────────────────┐
│  Ollama → http://localhost:11434                            ✅ running        │
│  Memory  2.1 / 16.0 GB                    ████░░░░░░░░░░░░  13%              │
│  Models                                                                      │
│  ├── ● llama3.2:1b          1.2 GB  ← active                                 │
│  └── ○ mistral:7b           4.1 GB                                           │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🔑 CREDENTIALS ─────────────────────────────────────────────────────────────┐
│  Name          Type   Status      Source      Endpoint                       │
│  anthropic     LLM    ✅ ready     🔐 keychain api.anthropic.com             │
│  openai        LLM    ✅ ready     📦 env      api.openai.com                 │
│  ollama        LLM    ✅ local     🦙 local    localhost:11434                │
│  neo4j         MCP    ✅ ready     🔐 keychain bolt://localhost:7687          │
│  7/13 configured   │   🔐 2 keychain   📦 4 env   🦙 1 local                  │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🔌 MCP SERVERS ───────────────────────────────────────────────────────────────┐
│  Server        Status      Transport   Command             Credential         │
│  neo4j         ○ ready     stdio       uvx                 → neo4j            │
│  perplexity    ○ ready     stdio       npx                 → perplexity       │
│  7/8 active                                                                   │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 📡 DAEMON ──────────────────────────────────────────────────────────────────┐
│  spn daemon ✅ running   PID 12345   ~/.spn/daemon.sock   Uptime 2h 15m      │
└──────────────────────────────────────────────────────────────────────────────┘

  🔑 7/13 Keys    🔌 7/8 MCPs    🦙 2 Models    📡 Daemon OK
```

**Dashboard sections:**

| Section | Shows |
|:--------|:------|
| 🦙 LOCAL MODELS | Ollama status, memory usage, installed/loaded models |
| 🔑 CREDENTIALS | All LLM + MCP credentials, their sources (keychain/env/.env/local) |
| 🔌 MCP SERVERS | Configured servers, their status, transport, and linked credentials |
| 📡 DAEMON | spn daemon status, PID, socket path, uptime |

**Use cases:**
- Quick health check before starting work
- Verify API keys are configured correctly
- Debug "why isn't X working" issues
- Confirm local models are loaded

<br>

---

### 📦 Package Management

Commands for installing and managing packages from the SuperNovae registry.

#### `spn add <package>`

Add a package to the manifest and install it.

```bash
# Add a workflow
spn add @nika/generate-page

# Add a schema
spn add @novanet/core-schema

# Add a job
spn add @jobs/daily-report

# Add with specific version
spn add @nika/generate-page@1.2.0

# Add with version constraint
spn add @nika/generate-page@^1.0.0
```

**What happens:**
1. Downloads package from registry
2. Adds to `spn.yaml` manifest
3. Updates `spn.lock` with resolved version
4. Installs to `~/.spn/packages/`
5. Syncs to editors (if applicable)

<br>

#### `spn remove <package>`

Remove a package from the project.

```bash
spn remove @nika/generate-page
```

**What happens:**
1. Removes from `spn.yaml` manifest
2. Updates `spn.lock`
3. Removes from `~/.spn/packages/`
4. Removes editor symlinks

<br>

#### `spn install`

Install all packages from `spn.yaml`.

```bash
# Install all packages
spn install

# Install exact versions from lockfile (CI mode)
spn install --frozen
```

**Use cases:**
- After cloning a project
- After pulling changes to `spn.yaml`
- In CI/CD pipelines (use `--frozen`)

<br>

#### `spn update [package]`

Update packages to latest compatible versions.

```bash
# Update all packages
spn update

# Update specific package
spn update @nika/generate-page
```

**What happens:**
1. Checks registry for latest compatible versions
2. Updates `spn.lock`
3. Downloads new versions
4. Re-syncs to editors

<br>

#### `spn search <query>`

Search the registry for packages.

```bash
spn search workflow
spn search seo
spn search @nika
```

**Output:**
```
📦 @nika/generate-page v1.2.0
   Generate landing pages from semantic templates

📦 @nika/seo-audit v2.0.1
   Comprehensive SEO analysis workflow

📦 @workflows/content-generator v0.5.0
   AI-powered content generation
```

<br>

#### `spn info <package>`

Show detailed information about a package.

```bash
spn info @nika/generate-page
```

**Output:**
```
📦 @nika/generate-page

Version: 1.2.0
Type: workflow
Author: SuperNovae Studio
License: MIT
Repository: https://github.com/supernovae-st/nika-workflows

Description:
Generate landing pages from semantic templates with NovaNet context.

Dependencies:
  - @novanet/core-schema ^0.14.0

Files:
  - generate-page.yaml (workflow)
  - templates/ (assets)

Installation:
  spn add @nika/generate-page
```

<br>

#### `spn list`

List all installed packages.

```bash
spn list
```

**Output:**
```
Installed Packages:

Workflows (2):
  @nika/generate-page v1.2.0
  @nika/seo-audit v2.0.1

Schemas (1):
  @novanet/core-schema v0.14.0

Skills (3):
  brainstorming (via skills.sh)
  superpowers/tdd (via skills.sh)
  coding-standards (via skills.sh)

MCP Servers (2):
  neo4j (@neo4j/mcp-server-neo4j v0.1.0)
  github (@modelcontextprotocol/server-github v0.2.0)
```

<br>

#### `spn outdated`

Show packages with available updates.

```bash
spn outdated
```

**Output:**
```
Outdated Packages:

Package                    Current    Latest    Type
@nika/generate-page        1.2.0      1.3.0     minor
@novanet/core-schema       0.14.0     0.15.0    minor
@nika/seo-audit            2.0.1      3.0.0     major ⚠️

Run 'spn update' to update all packages.
Run 'spn update <package>' to update a specific package.
```

<br>

#### `spn publish`

Publish the current package to the registry.

```bash
# Dry run (preview)
spn publish --dry-run

# Actual publish
spn publish
```

**Requirements:**
- `spn.json` manifest in current directory
- Authenticated with registry
- Unique version (not already published)

<br>

---

### ⚙️ Configuration Management

Commands for managing configuration across three scopes.

#### `spn config show [section]`

Show the resolved configuration (merged from all scopes).

```bash
# Show all configuration
spn config show

# Show specific section
spn config show providers
spn config show servers
spn config show sync
```

**Output:**
```
⚙️  Resolved Configuration

Providers:
  anthropic model = claude-opus-4
  openai model = gpt-4

Sync:
  enabled_editors = ["claude-code"]
  auto_sync = true

MCP Servers:
  neo4j npx -y @neo4j/mcp-server-neo4j
  github npx -y @modelcontextprotocol/server-github
```

<br>

#### `spn config where`

Show the locations of all config files.

```bash
spn config where
```

**Output:**
```
📁 Config File Locations

   Precedence: Local > Team > Global

   ✓ 🌍 Global   ~/.spn/config.toml
   ✓ 👥 Team     ./mcp.yaml
   ○ 💻 Local    ./.spn/local.yaml

   ✓ = exists, ○ = not found
```

<br>

#### `spn config list [--show-origin]`

List all configuration values.

```bash
# List all values
spn config list

# Show which scope defined each value
spn config list --show-origin
```

**Output with `--show-origin`:**
```
📋 Configuration Values

  providers.anthropic.model = claude-opus-4 (🌍 global)
  providers.openai.model = gpt-4 (🌍 global)
  servers.neo4j = <configured> (👥 team)
  servers.github = <configured> (🌍 global)
  sync.enabled_editors = ["claude-code"] (💻 local)

   Use 'spn config get <key> --show-origin' for detailed origin info
```

<br>

#### `spn config get <key> [--show-origin]`

Get a specific configuration value.

```bash
# Get value
spn config get providers.anthropic.model

# Show which scope defined it
spn config get providers.anthropic.model --show-origin
```

**Output:**
```
🔍 Getting value for key: providers.anthropic.model

Value: claude-opus-4
Origin: 🌍 Global (~/.spn/config.toml)
```

<br>

#### `spn config set <key> <value> [--scope=<scope>]`

Set a configuration value in a specific scope.

```bash
# Set in global scope (default)
spn config set providers.anthropic.model claude-opus-4

# Set in team scope
spn config set servers.neo4j.command npx --scope=team

# Set in local scope
spn config set providers.anthropic.model claude-haiku-4 --scope=local
```

**Scopes:**
- `global` (default) — User-level (~/.spn/config.toml)
- `team` — Project-level (./mcp.yaml)
- `local` — Developer overrides (./.spn/local.yaml)

<br>

#### `spn config edit [--local|--user|--mcp]`

Open a configuration file in your editor.

```bash
# Edit team config (default)
spn config edit

# Edit local config
spn config edit --local

# Edit global config
spn config edit --user

# Edit MCP config
spn config edit --mcp
```

**Environment variables used:**
1. `$EDITOR` (preferred)
2. `$VISUAL` (fallback)
3. `vi` (final fallback)

<br>

#### `spn config import <file> [--scope=<scope>] [--yes]`

Import MCP servers from an editor config file.

```bash
# Import from Claude Code settings
spn config import .claude/settings.json

# Import to specific scope
spn config import .claude/settings.json --scope=global

# Skip confirmation prompt
spn config import .claude/settings.json --yes
```

**What happens:**
1. Parses `.claude/settings.json` or `.cursor/mcp.json`
2. Extracts `mcpServers` section
3. Shows preview of what will be imported
4. Asks for confirmation (unless `--yes`)
5. Imports to specified scope
6. Creates target config file if it doesn't exist

**Example:**

```bash
$ spn config import .claude/settings.json --scope=team

📥 Importing configuration from .claude/settings.json
   Target scope: 👥 Team

MCP Servers to import:
  • neo4j npx -y @neo4j/mcp-server-neo4j
    env: 2 variables
  • github npx -y @modelcontextprotocol/server-github
    env: 1 variable

Import 2 servers into team scope? [Y/n] y

✅ Imported to ./mcp.yaml
```

**Supported formats:**
- `.claude/settings.json` (Claude Code)
- `.cursor/mcp.json` (Cursor)
- `.windsurf/mcp.json` (Windsurf)

<br>

---

### 🚀 Onboarding

Commands for first-time setup and configuration.

#### `spn setup`

Interactive onboarding wizard for first-time users.

```bash
# Full interactive wizard
spn setup

# Quick setup: auto-detect and migrate keys
spn setup --quick

# Verbose output
spn setup --verbose
```

**What happens:**
1. Detects existing API keys in environment
2. Shows provider signup URLs with descriptions
3. Prompts to migrate keys to OS Keychain
4. Configures default providers
5. Sets up MCP server aliases

**Provider Information (Open Source First):**

| Provider | Signup URL | Description | Cost |
|:---------|:-----------|:------------|:-----|
| 🦙 **Ollama** | [ollama.ai](https://ollama.ai) | Local inference, full privacy, no API key | **Free** |
| Anthropic | [console.anthropic.com](https://console.anthropic.com/settings/keys) | Best for complex reasoning, extended thinking | Paid |
| OpenAI | [platform.openai.com](https://platform.openai.com/api-keys) | Versatile, great for chat and embeddings | Paid |
| Mistral | [console.mistral.ai](https://console.mistral.ai/api-keys) | European, strong code generation | Paid |
| Groq | [console.groq.com](https://console.groq.com/keys) | Fastest inference, great for real-time | Free tier |
| DeepSeek | [platform.deepseek.com](https://platform.deepseek.com/api_keys) | Cost-effective, strong reasoning | Paid |
| Gemini | [aistudio.google.com](https://aistudio.google.com/app/apikey) | Google's model, multimodal capabilities | Free tier |

> 💡 **Tip:** Start with Ollama for local development — no API keys, no costs, full privacy.

#### `spn setup nika`

Install and configure Nika workflow engine with 5-step wizard.

```bash
spn setup nika
```

**What happens:**

| Step | Action | Description |
|:----:|:-------|:------------|
| 1 | Install CLI | Downloads `nika` binary via cargo or homebrew |
| 2 | Install LSP | Sets up `nika-lsp` for IDE support |
| 3 | Start Daemon | Auto-starts `spn daemon` for unified secret management |
| 4 | Check Providers | Verifies LLM API keys are configured |
| 5 | Configure Editors | Auto-detects and configures IDE integrations |

**Editor Integration:**
- **Claude Code** — Syncs MCP config via `spn sync`
- **VS Code** — Adds YAML schema for `.nika.yaml` validation
- **Cursor** — Adds YAML schema for `.nika.yaml` validation
- **Windsurf** — Adds YAML schema for `.nika.yaml` validation

```
🦋 Nika Setup Complete!

WHAT'S NEXT?
  nika              Launch TUI (Home View)
  nika chat         Start chat session
  nika studio       Open workflow studio
  nika --help       Show all commands
```

#### `spn setup novanet`

Install and configure NovaNet knowledge graph.

```bash
spn setup novanet
```

**What happens:**
1. Installs `novanet` CLI
2. Configures Neo4j connection
3. Sets up MCP server integration

<br>

---

### 🔑 Secrets Management

Commands for managing and auditing secrets configuration.

#### `spn secrets doctor`

Run health checks on secrets configuration.

```bash
spn secrets doctor
```

**Output:**
```
🏥 Secrets Health Check

Storage Status:
  ✅ OS Keychain accessible
  ✅ Environment variables loaded
  ⚠️  .env file found (consider migration)

Key Analysis:
  🔐 0 keys in OS Keychain
  📦 6 keys in environment variables
  ⚠️  2 keys in .env files (insecure)

Recommendations:
  1. Migrate environment keys to OS Keychain
     Run: spn provider migrate
  2. Remove .env files from version control
     Add to .gitignore: .env

Memory Protection:
  ✅ mlock available (limit: unlimited)
  ✅ MADV_DONTDUMP available
```

<br>

#### `spn secrets export <file> [--format=<format>]`

Export secrets to encrypted file.

```bash
# Export to SOPS-encrypted file
spn secrets export secrets.enc.yaml

# Export as JSON
spn secrets export secrets.enc.json --format=json

# Export masked (for sharing config structure)
spn secrets export secrets.masked.yaml --masked
```

**Formats:**
- `yaml` (default) — SOPS-encrypted YAML
- `json` — SOPS-encrypted JSON
- `env` — Encrypted .env format

**Security:**
- Uses SOPS (Secrets OPerationS) for encryption
- Supports age, PGP, AWS KMS, GCP KMS, Azure Key Vault
- Never exports unencrypted secrets

<br>

#### `spn secrets import <file>`

Import secrets from encrypted file.

```bash
# Import from SOPS-encrypted file
spn secrets import secrets.enc.yaml

# Import with verbose output
spn secrets import secrets.enc.yaml --verbose
```

**What happens:**
1. Decrypts file using SOPS
2. Validates key formats
3. Shows preview of keys to import
4. Asks for confirmation
5. Stores in OS Keychain

<br>

---

### 🔐 Security

Commands for managing API keys and credentials securely.

#### `spn provider list [--show-source]`

List all stored API keys (masked for security).

```bash
# List all keys
spn provider list

# Show where each key is stored
spn provider list --show-source
```

**Output:**
```
🔐 Stored API Keys

  anthropic: sk-ant-***************X (OS Keychain) ✓
  openai:    sk-***************X (Environment)
  neo4j:     bolt://***:***@localhost:7687 (.env file) ⚠️

Legend:
  ✓ = Secure (OS Keychain)
  ⚠️ = Less secure (env var or .env file)

Migrate to keychain: spn provider migrate
```

<br>

#### `spn provider set <name> [--key=<key>]`

Store an API key in the OS Keychain.

```bash
# Interactive (prompts for key)
spn provider set anthropic

# Non-interactive (for scripts)
spn provider set anthropic --key=sk-ant-api03-...
```

**Prompts for key (interactive):**
```
🔐 Setting API key for: anthropic

Enter API key (input hidden):
Confirm API key:

✅ Key stored in OS Keychain
   Security: Encrypted at rest
   Location: macOS Keychain

Test connection: spn provider test anthropic
```

**Security features:**
- Input hidden during typing
- Key confirmed before storing
- Stored encrypted in OS-native keychain
- Memory protected with `mlock()`
- Auto-cleared on drop with `zeroize`

<br>

#### `spn provider get <name> [--unmask]`

Get a stored API key (masked by default).

```bash
# Get masked key
spn provider get anthropic

# Get full key (for scripts)
spn provider get anthropic --unmask
```

**Output (masked):**
```
🔍 API Key for: anthropic

Key: sk-ant-***************X
Source: OS Keychain
```

**Output (unmasked):**
```
🔍 API Key for: anthropic

⚠️  WARNING: Full key displayed below. Keep this secure!

Key: sk-ant-api03-xxxxxxxxxxxxxxxxxxxx
Source: OS Keychain

Use in scripts:
  export ANTHROPIC_API_KEY=$(spn provider get anthropic --unmask)
```

<br>

#### `spn provider delete <name>`

Remove an API key from the OS Keychain.

```bash
spn provider delete anthropic
```

**Confirmation prompt:**
```
⚠️  Delete API key for: anthropic

This will remove the key from OS Keychain.
You will need to set it again to use this provider.

Delete? [y/N] y

✅ Key deleted from OS Keychain
```

<br>

#### `spn provider migrate [--yes]`

Migrate API keys from environment variables to OS Keychain.

```bash
# Interactive (asks for confirmation)
spn provider migrate

# Non-interactive (for scripts)
spn provider migrate --yes
```

**What happens:**
1. Scans environment variables for known patterns
2. Scans `.env` files in current directory
3. Shows what will be migrated
4. Asks for confirmation (unless `--yes`)
5. Stores each key in OS Keychain
6. Shows reminder to remove from `.env` files

**Output:**
```
🔄 Migrating API keys to OS Keychain

Found keys in environment:
  • ANTHROPIC_API_KEY (from .env)
  • OPENAI_API_KEY (from environment)
  • NEO4J_PASSWORD (from .env)

Migrate 3 keys to OS Keychain? [Y/n] y

✅ anthropic migrated
✅ openai migrated
✅ neo4j migrated

⚠️  Remember to:
   1. Remove keys from .env files
   2. Add .env to .gitignore
   3. Update team documentation

Test: spn provider test all
```

<br>

#### `spn provider test <name|all>`

Test provider connection and key validity.

```bash
# Test specific provider
spn provider test anthropic

# Test all providers
spn provider test all
```

**Output:**
```
🧪 Testing: anthropic

  Format: ✅ Valid (sk-ant-api03-...)
  Length: ✅ Correct (64 characters)
  Prefix: ✅ Valid (sk-ant-api03-)

Connection test: Not implemented yet
(Use provider's CLI tools to verify)

Key is valid and ready to use!
```

**Supported Providers (7 LLM + 6 MCP):**

| Type | Provider | Environment Variable | Key Format | Notes |
|:-----|:---------|:---------------------|:-----------|:------|
| 🦙 LLM | **ollama** | `OLLAMA_API_BASE_URL` | `http://...` | Local, free, private |
| LLM | anthropic | `ANTHROPIC_API_KEY` | `sk-ant-api03-...` | Claude models |
| LLM | openai | `OPENAI_API_KEY` | `sk-...` | GPT models |
| LLM | mistral | `MISTRAL_API_KEY` | `...` | EU provider |
| LLM | groq | `GROQ_API_KEY` | `gsk_...` | Fast inference |
| LLM | deepseek | `DEEPSEEK_API_KEY` | `...` | Cost-effective |
| LLM | gemini | `GEMINI_API_KEY` | `AI...` | Multimodal |
| MCP | neo4j | `NEO4J_PASSWORD` | `...` | Graph database |
| MCP | github | `GITHUB_TOKEN` | `ghp_...` | Code integration |
| MCP | slack | `SLACK_BOT_TOKEN` | `xoxb-...` | Team messaging |
| MCP | perplexity | `PERPLEXITY_API_KEY` | `...` | AI search |
| MCP | firecrawl | `FIRECRAWL_API_KEY` | `fc-...` | Web scraping |
| MCP | supadata | `SUPADATA_API_KEY` | `...` | Data API |

<br>

---

### 🦙 Model Management

Commands for managing local LLM models via Ollama. Requires daemon running (`spn daemon start`).

#### `spn model list [--json] [--running]`

List installed models.

```bash
# List all installed models
spn model list

# Output as JSON
spn model list --json

# Only show currently loaded models
spn model list --running
```

**Output:**
```
Installed Models

  NAME                                 SIZE      QUANT
  ----------------------------------------------------
  llama3.2:1b                        1.2 GB       Q8_0
  mistral:7b                         4.1 GB       Q4_K_M

  2 model(s) installed
```

<br>

#### `spn model pull <name>`

Download a model from the Ollama registry.

```bash
# Pull latest version
spn model pull llama3.2

# Pull specific variant
spn model pull llama3.2:1b
spn model pull mistral:7b
spn model pull codellama:13b
```

**Output:**
```
-> Pulling model: llama3.2:1b
   This may take a while...
* Model 'llama3.2:1b' pulled successfully
```

<br>

#### `spn model load <name> [--keep-alive]`

Load a model into GPU/RAM memory.

```bash
# Load model (auto-unloads after inactivity)
spn model load llama3.2:1b

# Keep loaded until manually unloaded
spn model load llama3.2:1b --keep-alive
```

**Output:**
```
-> Loading model: llama3.2:1b
* Model 'llama3.2:1b' loaded
   Model will stay loaded until manually unloaded
```

<br>

#### `spn model unload <name>`

Unload a model from memory to free GPU/RAM.

```bash
spn model unload llama3.2:1b
```

**Output:**
```
-> Unloading model: llama3.2:1b
* Model 'llama3.2:1b' unloaded
```

<br>

#### `spn model delete <name> [-y]`

Delete a model from disk.

```bash
# Interactive (asks for confirmation)
spn model delete llama3.2:1b

# Skip confirmation
spn model delete llama3.2:1b -y
```

<br>

#### `spn model status [--json]`

Show running models and VRAM usage.

```bash
spn model status
spn model status --json
```

**Output:**
```
Model Status

  MODEL                                  VRAM
  --------------------------------------------
  * llama3.2:1b                        1.6 GB
  * mistral:7b                         5.2 GB
  --------------------------------------------
  Total VRAM                           6.8 GB
```

**Use with Nika:**
```bash
# Use local model in Nika workflow
nika run workflow.yaml --provider ollama --model llama3.2:1b
```

<br>

---

### 🎯 Skills

Commands for managing skills from [skills.sh](https://skills.sh) (57K+ skills).

#### `spn skill add <name>`

Add a skill to the project.

```bash
# Add a skill
spn skill add brainstorming

# Add from a specific publisher
spn skill add superpowers/tdd

# Add with version
spn skill add brainstorming@1.0.0
```

**What happens:**
1. Downloads skill from skills.sh
2. Adds to `spn.yaml` under `skills:`
3. Installs to `~/.spn/packages/@skills/<name>/`
4. Syncs to `.claude/skills/`, `.cursor/skills/`, etc.

<br>

#### `spn skill remove <name>`

Remove a skill from the project.

```bash
spn skill remove brainstorming
```

**What happens:**
1. Removes from `spn.yaml`
2. Removes from `~/.spn/packages/`
3. Removes symlinks from editor configs

<br>

#### `spn skill list`

List all installed skills.

```bash
spn skill list
```

**Output:**
```
🎯 Installed Skills

  brainstorming v1.2.0
  superpowers/tdd v2.0.1
  coding-standards v0.5.0

Total: 3 skills

Usage in Claude Code:
  /brainstorming
  /spn-powers:tdd
  /coding-standards
```

<br>

#### `spn skill search <query>`

Search skills on skills.sh.

```bash
spn skill search workflow
spn skill search tdd
spn skill search @superpowers
```

**Output:**
```
🔍 Skills matching "workflow":

  brainstorming v1.2.0
  Interactive design refinement using Socratic method

  superpowers/workflow-design v2.0.0
  Design workflow architecture with validation

  agile/sprint-planning v1.5.0
  Sprint planning with story estimation

Search performed via skills.sh
Add with: spn skill add <name>
```

<br>

---

### 🔌 MCP Servers

Commands for managing MCP servers from npm (97M+ downloads).

#### `spn mcp add <name> [options]`

Add an MCP server to the project.

```bash
# Add to team scope (default)
spn mcp add neo4j

# Add to global scope
spn mcp add github --global

# Add to project scope
spn mcp add perplexity --project

# Skip automatic sync
spn mcp add neo4j --no-sync

# Sync only to specific editors
spn mcp add neo4j --sync-to=claude,cursor
```

**What happens:**
1. Resolves alias to npm package (e.g., `neo4j` → `@neo4j/mcp-server-neo4j`)
2. Installs npm package globally
3. Adds to specified config scope
4. Syncs to enabled editors (unless `--no-sync`)

**48 Built-in Aliases:**

```bash
# Database
spn mcp add neo4j
spn mcp add postgres
spn mcp add sqlite
spn mcp add supabase

# Development
spn mcp add github
spn mcp add gitlab
spn mcp add filesystem

# Search & AI
spn mcp add perplexity
spn mcp add brave-search
spn mcp add tavily

# Web Scraping
spn mcp add firecrawl
spn mcp add puppeteer
spn mcp add playwright

# Communication
spn mcp add slack
spn mcp add discord

# And 33 more...
```

Run `spn mcp list --all` to see complete list.

<br>

#### `spn mcp remove <name> [--global|--project]`

Remove an MCP server.

```bash
# Remove from team scope (default)
spn mcp remove neo4j

# Remove from global scope
spn mcp remove github --global

# Remove from project scope
spn mcp remove perplexity --project
```

**What happens:**
1. Removes from specified config scope
2. Removes from editor configs
3. Keeps npm package installed (manual cleanup: `npm uninstall -g <package>`)

<br>

#### `spn mcp list [--global|--project|--json]`

List installed MCP servers.

```bash
# List all servers
spn mcp list

# Show only global servers
spn mcp list --global

# Show only project servers
spn mcp list --project

# Output as JSON
spn mcp list --json
```

**Output:**
```
🔌 Installed MCP Servers

Global Servers (from ~/.spn/config.toml):
  github    @modelcontextprotocol/server-github v0.2.0

Team Servers (from ./mcp.yaml):
  neo4j     @neo4j/mcp-server-neo4j v0.1.0
  perplexity perplexity-mcp v1.0.0

Local Overrides (from ./.spn/local.yaml):
  neo4j     (env overrides: NEO4J_PASSWORD)

Total: 3 servers

Test: spn mcp test <name>
```

<br>

#### `spn mcp test <name>`

Test MCP server connection.

```bash
# Test specific server
spn mcp test neo4j

# Test all servers
spn mcp test all
```

**Output:**
```
🧪 Testing: neo4j

  Package: @neo4j/mcp-server-neo4j v0.1.0
  Command: npx -y @neo4j/mcp-server-neo4j

  Environment:
    NEO4J_URI:      bolt://localhost:7687 ✓
    NEO4J_PASSWORD: ********** ✓

  Connection: ✅ Server responds
  Tools: 8 available
    - neo4j_query
    - neo4j_execute
    - neo4j_schema
    - ...

Server is healthy and ready to use!
```

<br>

---

### 🦋 Nika Integration

> 🎯 **Direct Integration:** Nika reads MCP configs **directly** from `~/.spn/mcp.yaml` — no sync required!
>
> This means your MCP servers are instantly available to Nika workflows without running `spn sync`.

Commands for interacting with the Nika workflow runtime.

#### `spn nk run <file>`

Run a Nika workflow.

```bash
# Run a workflow
spn nk run generate-page.yaml

# Run with variables
spn nk run generate-page.yaml --var entity=qr-code --var locale=fr-FR
```

**Proxy to:** `nika run <file>`

<br>

#### `spn nk check <file>`

Validate workflow syntax.

```bash
spn nk check generate-page.yaml
```

**Output:**
```
✅ Workflow is valid

Steps: 5
Verbs used: infer (2), invoke (2), fetch (1)
Dependencies resolved: ✓
```

<br>

#### `spn nk studio`

Open Nika Studio TUI (interactive workflow editor).

```bash
spn nk studio
```

**Features:**
- Visual workflow editor
- Real-time syntax validation
- Step-by-step execution
- Variable inspector
- Debug mode

<br>

#### `spn nk jobs start|status|stop`

Manage the Nika jobs daemon.

```bash
# Start daemon
spn nk jobs start

# Check status
spn nk jobs status

# Stop daemon
spn nk jobs stop
```

**Jobs daemon:**
- Runs workflows on schedule (cron)
- Handles webhook triggers
- Processes background tasks

<br>

---

### 🧠 NovaNet Integration

Commands for interacting with the NovaNet knowledge graph.

#### `spn nv tui`

Open NovaNet TUI (interactive graph explorer).

```bash
spn nv tui
```

**Features:**
- Browse node classes and arc classes
- Query the graph with Cypher
- Visualize relationships
- Generate native content

<br>

#### `spn nv query <query>`

Query the knowledge graph with Cypher.

```bash
spn nv query "MATCH (n:Entity) RETURN n LIMIT 10"
```

<br>

#### `spn nv mcp start|stop`

Start or stop the NovaNet MCP server.

```bash
# Start MCP server
spn nv mcp start

# Stop MCP server
spn nv mcp stop
```

**MCP Tools provided:**
- `novanet_generate` — Generate native content
- `novanet_describe` — Describe entities
- `novanet_traverse` — Navigate relationships
- `novanet_introspect` — Query schema

<br>

#### `spn nv add-node|add-arc`

Add node or arc types to the schema.

```bash
# Add a node type
spn nv add-node Product --realm=shared --layer=core

# Add an arc type
spn nv add-arc ProductCategory --from=Product --to=Category
```

<br>

#### `spn nv db start|seed|reset`

Manage the Neo4j database.

```bash
# Start Neo4j
spn nv db start

# Seed with initial data
spn nv db seed

# Reset and reseed
spn nv db reset
```

<br>

---

### 🔄 Sync & Editor Integration

> 🦋 **Nika Exception:** Nika reads `~/.spn/mcp.yaml` **directly** — no sync required for Nika workflows.
>
> The sync command is only needed for **IDE integration** (Claude Code, Cursor, Windsurf).

Commands for syncing packages to editor configurations.

#### `spn sync [options]`

Sync packages and MCP servers to editor configs.

```bash
# Sync to all enabled editors
spn sync

# Sync to specific editor
spn sync --target claude-code

# Preview changes without applying
spn sync --dry-run

# Interactive mode with diff preview
spn sync --interactive

# Combine options
spn sync --target cursor --interactive
```

**What gets synced:**

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': { 'primaryColor': '#ec4899', 'primaryTextColor': '#fff', 'primaryBorderColor': '#f472b6', 'lineColor': '#f9a8d4', 'secondaryColor': '#831843', 'tertiaryColor': '#9f1239'}}}%%
flowchart LR
    subgraph SOURCES["📦 Sources"]
        MCP_CONFIG["MCP Config<br/>(3 scopes merged)"]
        SKILLS["@skills/<br/>packages"]
        HOOKS["Hook configs"]
    end

    subgraph EDITORS["📂 Editors"]
        CLAUDE[".claude/<br/>settings.json<br/>skills/<br/>hooks/"]
        CURSOR[".cursor/<br/>mcp.json<br/>skills/<br/>hooks/"]
        WINDSURF[".windsurf/<br/>mcp.json<br/>skills/<br/>hooks/"]
    end

    MCP_CONFIG --> CLAUDE
    MCP_CONFIG --> CURSOR
    MCP_CONFIG --> WINDSURF
    SKILLS --> CLAUDE
    SKILLS --> CURSOR
    SKILLS --> WINDSURF
    HOOKS --> CLAUDE

    style MCP_CONFIG fill:#6366f1
    style SKILLS fill:#10b981
    style HOOKS fill:#f59e0b
```

**Supported Editors:**

| Editor | Config Directory | MCP File | Skills Directory |
|:-------|:----------------|:---------|:-----------------|
| Claude Code | `.claude/` | `settings.json` | `.claude/skills/` |
| Cursor | `.cursor/` | `mcp.json` | `.cursor/skills/` |
| Windsurf | `.windsurf/` | `mcp.json` | `.windsurf/skills/` |
| VS Code | `.vscode/` | N/A | N/A (not supported) |

<br>

#### `spn sync --enable <editor>`

Enable automatic sync for an editor.

```bash
spn sync --enable claude-code
spn sync --enable cursor
spn sync --enable windsurf
```

**What happens:**
1. Adds editor to `~/.spn/config.toml` under `sync.enabled_editors`
2. Editor will be synced on every `spn add`, `spn install`, `spn mcp add`

<br>

#### `spn sync --disable <editor>`

Disable automatic sync for an editor.

```bash
spn sync --disable cursor
```

**What happens:**
1. Removes editor from `sync.enabled_editors`
2. Editor will no longer be synced automatically

<br>

#### `spn sync --status`

Show sync status and configuration.

```bash
spn sync --status
```

**Output:**
```
📊 Sync Status

Enabled targets:
  ✅ Claude Code
  ✅ Cursor

Detected IDEs in current directory:
  ✓ Claude Code (.claude/settings.json)
  ✓ Cursor (.cursor/mcp.json)
  ○ Windsurf (no config found)

Last sync: 2024-03-15 14:32:05 UTC

Auto-sync: Enabled
Sync on: add, install, mcp add

Configure: spn sync --enable <editor>
```

<br>

---

### 🏥 System Diagnostic

#### `spn doctor`

Run comprehensive system diagnostic.

```bash
spn doctor
```

**Checks:**
- ✅ spn installation
- ✅ Nika binary available
- ✅ NovaNet binary available
- ✅ Node.js and npm installed
- ✅ Git configuration
- ✅ Neo4j connection
- ✅ API keys configured
- ✅ Editor configs valid
- ✅ Package manifest syntax
- ✅ Lockfile consistency

**Output:**
```
🏥 SuperNovae System Diagnostic

Installation:
  ✅ spn v0.15.0 installed
  ✅ nika v0.21.1 available
  ✅ novanet v0.14.0 available

Dependencies:
  ✅ Node.js v20.11.0
  ✅ npm v10.2.4
  ✅ Git v2.43.0

Configuration:
  ✅ ~/.spn/config.toml exists
  ✅ ./mcp.yaml valid
  ○  ./.spn/local.yaml not found (optional)

API Keys:
  ✅ anthropic (OS Keychain)
  ✅ openai (Environment)
  ⚠️  neo4j (not configured)

Database:
  ✅ Neo4j running at bolt://localhost:7687
  ✅ 1,234 nodes, 5,678 relationships

Editors:
  ✅ Claude Code (.claude/settings.json)
  ✅ Cursor (.cursor/mcp.json)

Packages:
  ✅ 5 packages installed
  ✅ spn.lock matches spn.yaml

Overall: ✅ System is healthy

Issues:
  ⚠️  Neo4j credentials not configured
     Fix: spn provider set neo4j

Run 'spn doctor --verbose' for detailed diagnostics.
```

<br>

---

### 🚀 Project Initialization

#### `spn init [options]`

Initialize a new SuperNovae project.

```bash
# Interactive wizard
spn init

# Create local config
spn init --local

# Create MCP config
spn init --mcp

# Initialize from template
spn init --template nika
spn init --template novanet
```

**Interactive Wizard:**

```
🌟 SuperNovae Project Setup

What would you like to create?
  > Nika workflow project
    NovaNet schema project
    Full-stack (Nika + NovaNet)
    Empty project

Project name: my-project
Description: My awesome AI project

Initialize git repository? Yes

Which editors do you use?
  [x] Claude Code
  [ ] Cursor
  [ ] Windsurf

Add example workflows? Yes

✅ Created my-project/
   ├── spn.yaml
   ├── mcp.yaml
   ├── .gitignore
   ├── README.md
   └── examples/
       └── hello-workflow.yaml

Next steps:
  cd my-project
  spn add @nika/generate-page
  spn sync
```

<br>

---

## 📄 Configuration Files

### Project Manifest (`spn.yaml`)

Main project configuration file (committed to git).

```yaml
name: my-project
version: 0.1.0
description: My awesome AI project

# Package dependencies
workflows:
  - "@nika/generate-page@^1.0.0"
  - "@nika/seo-audit@^2.0.0"

schemas:
  - "@novanet/core-schema@^0.14.0"

jobs:
  - "@jobs/daily-report@^1.0.0"

# Interop packages (via skills.sh)
skills:
  - "brainstorming"
  - "superpowers/tdd"

# Interop packages (via npm)
mcp:
  - "neo4j"
  - "perplexity"

# Editor sync preferences
sync:
  claude: true
  cursor: true
  nika: true
  auto_sync: true
```

<br>

### Team MCP Config (`./mcp.yaml`)

Team-level MCP server configuration (committed to git).

```yaml
servers:
  neo4j:
    command: "npx"
    args: ["-y", "@neo4j/mcp-server-neo4j"]
    env:
      NEO4J_URI: "bolt://localhost:7687"
      NEO4J_DATABASE: "neo4j"

  perplexity:
    command: "npx"
    args: ["-y", "perplexity-mcp"]

  github:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-github"]
    env:
      GITHUB_REPO: "supernovae-st/my-project"
```

<br>

### Local Overrides (`./.spn/local.yaml`)

Developer-specific overrides (gitignored).

```yaml
# Override MCP server settings
servers:
  neo4j:
    env:
      NEO4J_PASSWORD: "dev-password"
      NEO4J_DATABASE: "test"

# Override provider settings
providers:
  anthropic:
    model: "claude-haiku-4"  # Use cheaper model for local testing
    endpoint: "http://localhost:8080"  # Local proxy

# Override sync settings
sync:
  auto_sync: false  # Disable auto-sync during development
```

**Add to `.gitignore`:**
```gitignore
.spn/local.yaml
```

<br>

### Global User Config (`~/.spn/config.toml`)

User-level configuration (never committed).

```toml
# Provider defaults
[providers.anthropic]
model = "claude-opus-4"
endpoint = "https://api.anthropic.com"

[providers.openai]
model = "gpt-4"

# Sync preferences
[sync]
enabled_editors = ["claude-code", "cursor"]
auto_sync = true

# Global MCP servers (available in all projects)
[servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem"]
```

<br>

### Package Lockfile (`spn.lock`)

Resolved package versions (committed to git).

```json
{
  "version": 1,
  "packages": {
    "@nika/generate-page": {
      "version": "1.2.0",
      "resolved": "https://registry.supernovae.studio/@nika/generate-page/1.2.0.tar.gz",
      "integrity": "sha256-abc123...",
      "dependencies": {
        "@novanet/core-schema": "^0.14.0"
      }
    },
    "@novanet/core-schema": {
      "version": "0.14.0",
      "resolved": "https://registry.supernovae.studio/@novanet/core-schema/0.14.0.tar.gz",
      "integrity": "sha256-def456..."
    }
  }
}
```

**Important:** Always commit `spn.lock` to ensure reproducible builds.

<br>

---

## 🌌 The SuperNovae Ecosystem

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': { 'primaryColor': '#6366f1', 'primaryTextColor': '#fff', 'primaryBorderColor': '#818cf8', 'lineColor': '#a5b4fc', 'secondaryColor': '#1e1b4b', 'tertiaryColor': '#312e81'}}}%%
flowchart TB
    subgraph SPN["🌟 spn — The Agentic AI Toolkit v0.15.0"]
        direction LR
        subgraph OWNED["We Own"]
            W["📋 workflows/<br/>YAML DAGs"]
            S["🧬 schemas/<br/>Graph models"]
            J["⏰ jobs/<br/>Cron triggers"]
        end
        subgraph INTEROP["Interop"]
            SK["🎯 skills/<br/>57K+ from skills.sh"]
            MCP["🔌 mcp/<br/>97M+ from npm"]
        end
    end

    subgraph CONFIG["⚙️ Configuration System"]
        GLOBAL["🌍 Global<br/>~/.spn/config.toml"]
        TEAM["👥 Team<br/>./mcp.yaml"]
        LOCAL["💻 Local<br/>./.spn/local.yaml"]
        RESOLVER["Config Resolver<br/>(Local > Team > Global)"]

        GLOBAL --> RESOLVER
        TEAM --> RESOLVER
        LOCAL --> RESOLVER
    end

    subgraph RUNTIME["🚀 Runtime Engines"]
        NIKA["🦋 Nika v0.21.1<br/>Workflow Runtime<br/><i>5 Semantic Verbs</i>"]
        NOVANET["🧠 NovaNet v0.17.2<br/>Knowledge Graph<br/><i>Neo4j + MCP Server</i>"]
    end

    subgraph EDITORS["📂 Editor Integration"]
        CLAUDE[".claude/<br/>settings.json<br/>skills/<br/>hooks/"]
        CURSOR[".cursor/<br/>mcp.json<br/>skills/"]
        WINDSURF[".windsurf/<br/>mcp.json<br/>skills/"]
    end

    SPN --> RESOLVER
    RESOLVER --> NIKA
    RESOLVER --> NOVANET
    NIKA <-->|MCP Protocol| NOVANET

    RESOLVER -.->|Sync| CLAUDE
    RESOLVER -.->|Sync| CURSOR
    RESOLVER -.->|Sync| WINDSURF

    SK -.-> |"skills.sh"| SKILLS[(skills.sh<br/>57K+ skills)]
    MCP -.-> |"npm registry"| NPM[(npm<br/>97M+ downloads)]

    style SPN fill:#1e293b
    style CONFIG fill:#334155
    style RUNTIME fill:#475569
    style EDITORS fill:#64748b
    style RESOLVER fill:#6366f1
```

### Mascots & Roles

| Mascot | Role | Description |
|--------|------|-------------|
| **spn** 🌟 | CLI | The Agentic AI Toolkit — models, MCP servers, providers, secrets |
| **Nika** 🦋 | Runtime | Orchestrates workflows via 5 semantic verbs: `infer`, `exec`, `fetch`, `invoke`, `agent` |
| **NovaNet** 🧠 | Brain | Knowledge graph for localization, entities, and semantic relationships |

> **Note:** Nika reads MCP configs directly from `~/.spn/mcp.yaml` — no sync needed between spn and Nika.

<br>

### Package Types

| Type | Scope | Sync? | Description | Example |
|:-----|:------|:------|:------------|:--------|
| **workflow** | `@nika/`, `@workflows/` | ❌ NO | YAML DAG definitions | `@nika/generate-page` |
| **schema** | `@novanet/`, `@schemas/` | ❌ NO | Graph node/arc classes | `@novanet/core-schema` |
| **job** | `@jobs/` | ❌ NO | Cron/webhook triggers | `@jobs/daily-report` |
| **skill** | `@skills/` | ✅ YES | Reusable prompts | `brainstorming` |
| **agent** | `@agents/` | ❌ NO | Agent configurations | `@agents/code-reviewer` |
| **prompt** | `@prompts/` | ❌ NO | Prompt templates | `@prompts/seo-meta` |

<br>

---

## ❓ FAQ

### General

**Q: What's the difference between spn and Nika?**

spn is the package manager (install, configure, sync). Nika is the runtime engine (execute workflows). They work together: spn manages what's installed, Nika runs it.

**Q: Do I need all three tools (spn, Nika, NovaNet)?**

No. Start with spn alone for MCP servers and skills. Add Nika when you need AI workflows. Add NovaNet when you need a knowledge graph for localization.

**Q: Is this only for Claude Code?**

No. spn supports Claude Code, Cursor, Windsurf, and any editor with MCP support. Skills sync to all enabled editors.

### Configuration

**Q: Where are my API keys stored?**

In your OS keychain (macOS Keychain, Windows Credential Manager, or Linux Secret Service). Run `spn provider list --show-source` to see each key's location.

**Q: What's the difference between global, team, and local config?**

- **Global** (`~/.spn/config.toml`): User settings, applies everywhere
- **Team** (`./mcp.yaml`): Project settings, committed to git
- **Local** (`./.spn/local.yaml`): Your overrides, gitignored

**Q: How do I share MCP servers with my team?**

Add them to `./mcp.yaml` and commit to git. Team members run `spn sync` after pulling.

### Troubleshooting

**Q: `spn mcp add` fails with npm error**

Ensure Node.js 18+ is installed: `node --version`. If using nvm, run `nvm use 18` or higher.

**Q: API key not found after setting it**

Check the key exists: `spn provider list`. If using Docker, environment variables are required (no keychain access).

**Q: Sync not updating editor config**

1. Check enabled editors: `spn sync --status`
2. Enable missing editor: `spn sync --enable claude-code`
3. Force sync: `spn sync`

---

## 🔧 Troubleshooting

### Common Issues

#### "command not found: spn"

**Cause:** spn binary not in PATH.

```bash
# Check installation
which spn || echo "Not installed"

# Reinstall via Homebrew
brew reinstall supernovae-st/tap/spn

# Or reinstall via Cargo
cargo install --force spn-cli
```

#### "Failed to access keychain"

**Cause:** OS keychain permissions.

```bash
# macOS: Reset keychain permissions
security unlock-keychain ~/Library/Keychains/login.keychain-db

# Linux: Ensure secret service is running
systemctl --user status gnome-keyring-daemon
```

#### "MCP server failed to start"

**Cause:** Missing npm package or environment variables.

```bash
# Check server config
spn mcp list

# Test specific server
spn mcp test neo4j

# Check npm package
npm list -g @neo4j/mcp-server-neo4j
```

#### "Sync has no effect"

**Cause:** Editor not enabled or config already up-to-date.

```bash
# Check sync status
spn sync --status

# Enable editor
spn sync --enable claude-code

# Force sync with preview
spn sync --interactive
```

#### "Provider test fails"

**Cause:** Invalid key format or network issue.

```bash
# Test key format only
spn provider test anthropic

# Re-set key
spn provider set anthropic

# Check key source
spn provider list --show-source
```

### Debug Mode

For detailed diagnostics:

```bash
# Run any command with verbose output
spn -v doctor          # info level
spn -vv doctor         # debug level
spn -vvv doctor        # trace level

# Check all systems
spn doctor --verbose

# Export diagnostics
spn doctor --json > diagnostics.json
```

### Getting Help

- **GitHub Issues:** [supernovae-st/supernovae-cli/issues](https://github.com/supernovae-st/supernovae-cli/issues)
- **Discord:** [discord.gg/supernovae](https://discord.gg/supernovae)
- **Documentation:** [supernovae.studio/docs](https://supernovae.studio/docs)

---

## 🗂️ Directory Structure

```
~/.spn/                          # Global spn directory
├── config.toml                  # Global user config (v0.7.0)
├── packages/                    # Downloaded packages
│   ├── @nika/
│   ├── @novanet/
│   ├── @skills/
│   └── @jobs/
├── registry/                    # Cached registry index
└── bin/                         # Binary stubs (nika, novanet)

./                               # Project directory
├── spn.yaml                     # Package manifest (committed)
├── spn.lock                     # Resolved versions (committed)
├── mcp.yaml                     # Team MCP servers (committed) (v0.7.0)
├── .spn/
│   └── local.yaml               # Local overrides (gitignored) (v0.7.0)
├── .claude/
│   ├── settings.json            # Claude Code config (generated)
│   ├── skills/                  # Skill symlinks (generated)
│   └── hooks/                   # Hook configs (generated)
├── .cursor/
│   ├── mcp.json                 # Cursor MCP config (generated)
│   └── skills/                  # Skill symlinks (generated)
└── .windsurf/
    ├── mcp.json                 # Windsurf MCP config (generated)
    └── skills/                  # Skill symlinks (generated)
```

**What to commit:**
- ✅ `spn.yaml` — Package manifest
- ✅ `spn.lock` — Resolved versions
- ✅ `mcp.yaml` — Team MCP servers
- ❌ `.spn/local.yaml` — Local overrides
- ❌ `.claude/`, `.cursor/`, `.windsurf/` — Generated configs

<br>

---

## 🔗 Related Projects

| Repository | Description | Version |
|:-----------|:------------|:--------|
| [nika](https://github.com/supernovae-st/nika) | 🦋 Semantic YAML workflow engine | v0.21.1 |
| [novanet](https://github.com/supernovae-st/novanet) | 🧠 Knowledge graph for localization | v0.17.2 |
| [supernovae-registry](https://github.com/supernovae-st/supernovae-registry) | 📦 Public package registry | - |
| [supernovae-index](https://github.com/supernovae-st/supernovae-index) | 📇 Sparse package index | - |
| [homebrew-tap](https://github.com/supernovae-st/homebrew-tap) | 🍺 Homebrew formulas | - |

<br>

---

## 🤝 Contributing

We welcome contributions! Here's how to get started:

### Development Setup

```bash
# Clone the repository
git clone https://github.com/supernovae-st/supernovae-cli
cd supernovae-cli

# Build the project
cargo build

# Run tests (920+ tests)
cargo test

# Run linter
cargo clippy

# Format code
cargo fmt

# Install locally for testing
cargo install --path .
```

<br>

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_config_resolution

# Run tests with coverage
cargo tarpaulin --out Html
```

<br>

### Conventions

- **Commits:** `type(scope): description`
  - Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
  - Examples: `feat(config): add import command`, `fix(sync): resolve path issue`
- **Code style:** Run `cargo fmt` before committing
- **Linting:** Run `cargo clippy` and fix all warnings
- **Testing:** TDD preferred, aim for 80%+ coverage

<br>

### Project Structure

```rust
src/
├── main.rs              // CLI entry point
├── commands/            // Command implementations
│   ├── add.rs
│   ├── config.rs
│   ├── provider.rs
│   ├── sync.rs
│   └── ...
├── config/              // Configuration system (v0.7.0)
│   ├── mod.rs           // Module exports
│   ├── types.rs         // Config data types
│   ├── scope.rs         // Scope definitions
│   ├── resolver.rs      // Config merging
│   ├── global.rs        // Global config I/O
│   ├── team.rs          // Team config I/O
│   └── local.rs         // Local config I/O
├── index/               // Registry client
├── manifest/            // spn.yaml parsing
├── storage/             // Package storage
├── sync/                // Editor sync (v0.7.0)
│   ├── types.rs         // Sync types
│   ├── adapters.rs      // Editor adapters
│   └── mcp_sync.rs      // MCP sync logic
├── secrets/             // Credential management (v0.6.0)
│   ├── keyring.rs       // OS keychain
│   ├── types.rs         // Provider types
│   ├── memory.rs        // Memory protection
│   ├── storage.rs       // Storage abstraction (v0.8.0)
│   ├── env_storage.rs   // Environment storage (v0.8.0)
│   └── wizard.rs        // Setup wizard (v0.8.0)
└── error.rs             // Error types
```

<br>

---

## 📄 License

**MIT** © [SuperNovae Studio](https://supernovae.studio)

<br>

---

<div align="center">

## 🌟 Part of the SuperNovae Ecosystem

<table>
<tr>
<td align="center">
<a href="https://github.com/supernovae-st/novanet">
<img src="https://img.shields.io/badge/🧠_NovaNet-Knowledge_Graph-0ea5e9?style=for-the-badge" alt="NovaNet">
</a>
<br><sub>Brain: Knowledge Graph + MCP Server</sub>
</td>
<td align="center">
<a href="https://github.com/supernovae-st/nika">
<img src="https://img.shields.io/badge/🦋_Nika-Workflow_Engine-7c3aed?style=for-the-badge" alt="Nika">
</a>
<br><sub>Body: DAG Workflows + MCP Client</sub>
</td>
<td align="center">
<a href="https://github.com/supernovae-st/supernovae-cli">
<img src="https://img.shields.io/badge/🌟_spn-Agentic_Toolkit-6366f1?style=for-the-badge" alt="spn">
</a>
<br><sub>Manager: The Agentic AI Toolkit</sub>
</td>
</tr>
</table>

<br>

<!-- SuperNovae Studio -->
<a href="https://supernovae.studio">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://avatars.githubusercontent.com/u/186506682?s=200&v=4">
  <source media="(prefers-color-scheme: light)" srcset="https://avatars.githubusercontent.com/u/186506682?s=200&v=4">
  <img src="https://avatars.githubusercontent.com/u/186506682?s=200&v=4" width="100" height="100" alt="SuperNovae Studio">
</picture>
</a>

### **[SuperNovae Studio](https://supernovae.studio)**

*Building the future of AI workflows* 🚀

<br>

<!-- Team -->
<table>
<tr>
<td align="center">
<a href="https://github.com/ThibautMelen">
<img src="https://github.com/ThibautMelen.png" width="80" height="80" alt="Thibaut Melen" style="border-radius: 50%;"><br>
<sub><b>Thibaut Melen</b></sub>
</a>
<br><sub>🎨 Founder & Architect</sub>
</td>
<td align="center">
<a href="https://github.com/NicolasCELLA">
<img src="https://github.com/NicolasCELLA.png" width="80" height="80" alt="Nicolas Cella" style="border-radius: 50%;"><br>
<sub><b>Nicolas Cella</b></sub>
</a>
<br><sub>🚀 Co-Founder & Engineer</sub>
</td>
</tr>
</table>

<br>

<!-- Links -->
[![Website](https://img.shields.io/badge/🌟_supernovae.studio-6366f1?style=for-the-badge)](https://supernovae.studio)
[![GitHub](https://img.shields.io/badge/🐙_supernovae--st-181717?style=for-the-badge&logo=github&logoColor=white)](https://github.com/supernovae-st)
[![Twitter](https://img.shields.io/badge/🐦_@SuperNovaeAI-1DA1F2?style=for-the-badge&logo=twitter&logoColor=white)](https://twitter.com/SuperNovaeAI)
[![Discord](https://img.shields.io/badge/💬_Discord-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/supernovae)

<br>

<!-- Social Stats -->
[![Stars](https://img.shields.io/github/stars/supernovae-st/supernovae-cli?style=social&label=Star)](https://github.com/supernovae-st/supernovae-cli)
&nbsp;&nbsp;
[![Forks](https://img.shields.io/github/forks/supernovae-st/supernovae-cli?style=social&label=Fork)](https://github.com/supernovae-st/supernovae-cli/fork)
&nbsp;&nbsp;
[![Watchers](https://img.shields.io/github/watchers/supernovae-st/supernovae-cli?style=social&label=Watch)](https://github.com/supernovae-st/supernovae-cli)

<br>

---

<sub>Made with 💜 and 🦀 by the SuperNovae team</sub>

<br>

**⭐ Star us on GitHub — it helps others discover SuperNovae!**

<br>

<sup>Zero Clippy Warnings • Open Source First • Nika Direct Integration • 7 LLM Providers • Automated Releases</sup>

</div>
