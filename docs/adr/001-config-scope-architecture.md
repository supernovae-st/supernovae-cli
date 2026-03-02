# ADR 001: Configuration Scope Architecture

**Status**: Accepted
**Date**: 2026-03-03
**Deciders**: Thibaut Melen, Claude, Nika

## Context

SuperNovae CLI (spn) currently conflates three distinct responsibilities:
1. **Package Management**: Installing @workflows/, @skills/, @agents/ from registry
2. **Configuration Management**: MCP servers, provider settings
3. **Editor Integration**: Syncing to .claude/, .cursor/ configs

This creates confusion around:
- Where configurations are stored (scope ambiguity)
- What needs to be synced to editors
- Whether sync should be bidirectional
- Precedence when conflicts occur

## Decision

We will **separate concerns** and implement a **clear 3-level scope hierarchy** following industry standards (npm, cargo, git).

### 1. Three Distinct Subsystems

#### Package Management (`spn add/install/remove`)
- Manages packages from registry: @workflows/, @agents/, @skills/, @prompts/, @jobs/, @schemas/
- Uses `spn.yaml` (team dependencies) + `spn.lock` (exact versions)
- Installation locations:
  - Global: `~/.spn/packages/` (with `-g` flag)
  - Local: `./spn_modules/` (project-specific)
- NO automatic sync to editors

#### Configuration Management (`spn config`)
- Manages three distinct config files:
  - `~/.spn/config.toml` (global user preferences)
  - `./mcp.yaml` (team MCP servers, committed to git)
  - `./.spn/local.yaml` (local overrides, gitignored)
- Handles MCP servers, provider settings, sync preferences
- Separate from package installation

#### Editor Integration (`spn sync`)
- **Selective sync** based on package type
- Only syncs packages requiring filesystem presence:
  - ✅ `@skills/` → `.claude/skills/` (process documentation)
  - ✅ MCP servers → `.claude/settings.json` (editor needs to start processes)
  - ❌ `@workflows/` (standalone execution via `nika run`)
  - ❌ `@agents/` (nika CLI subagents)
  - ❌ `@prompts/`, `@jobs/`, `@schemas/` (no editor integration needed)
- **Unidirectional flow**: Config files → `.claude/` (NOT bidirectional)
- Optional explicit import: `spn config import .claude/settings.json`

### 2. Scope Hierarchy (Resolution Order)

**Precedence**: `Local > Team > Global` (innermost wins)

```
~/.spn/
  ├── config.toml          # Global user preferences
  │                        # - Default providers, models
  │                        # - Personal MCP servers
  │                        # - Sync settings
  └── packages/            # Globally installed packages

./project/
  ├── spn.yaml            # Team dependencies (committed)
  │                        # - Project packages
  │                        # - Shared configuration
  ├── mcp.yaml            # Team MCP servers (committed)
  │                        # - Project infrastructure (neo4j, etc.)
  ├── spn.lock            # Lockfile (committed)
  ├── spn_modules/        # Local packages (gitignored)
  └── .spn/
      └── local.yaml      # Local overrides (gitignored)
                          # - Machine-specific settings
                          # - Development secrets
                          # - Password overrides
```

**Example Resolution**:
```yaml
# ~/.spn/config.toml (Global)
[providers.anthropic]
model = "claude-sonnet-4-5"

# ./mcp.yaml (Team)
servers:
  neo4j:
    command: npx
    args: ["-y", "@neo4j/mcp-server-neo4j"]
    env:
      NEO4J_URI: "bolt://localhost:7687"

# ./.spn/local.yaml (Local - WINS)
providers:
  anthropic:
    model: "claude-opus-4-5"  # Override for local dev

servers:
  neo4j:
    env:
      NEO4J_PASSWORD: "my-local-password"  # Machine secret
```

### 3. Selective Sync Criteria

Packages declare sync requirements in `manifest.yaml`:

```yaml
name: "@skills/brainstorming"
type: skill
version: "1.0.0"
integration:
  requires_sync: true
  editors: ["claude-code", "cursor"]
```

Sync rules by package type:
- `@skills/` → `requires_sync: true` (default)
- `@workflows/` → `requires_sync: false` (default)
- `@agents/` → `requires_sync: false`
- `@prompts/` → `requires_sync: false`
- `@jobs/` → `requires_sync: false`
- `@schemas/` → `requires_sync: false`

### 4. Unidirectional Sync with Explicit Import

**Flow**: `spn.yaml/mcp.yaml` → `spn install` → `~/.spn/packages/` → `spn sync` → `.claude/`

**NOT bidirectional** because:
- npm doesn't import from node_modules → package.json
- cargo doesn't read installed crates → Cargo.toml
- pip doesn't update requirements.txt from virtualenv

**Manual edits to .claude/**:
```bash
# Detect drift
spn sync --status
# ⚠️ .claude/settings.json has manual changes

# Import explicitly
spn config import .claude/settings.json --scope team

# Or force overwrite
spn sync --force
```

## Consequences

### Positive
- **Clear separation of concerns**: Package management ≠ Config management ≠ Editor sync
- **Industry-standard scopes**: Follows npm/cargo/git patterns (familiar to developers)
- **Selective sync**: Only sync what needs editor integration (~30% of packages)
- **Predictable behavior**: Unidirectional flow with explicit import option
- **Team collaboration**: Team configs (mcp.yaml) committed, local secrets gitignored
- **No magic**: Every action is explicit and understandable

### Negative
- **Breaking change**: Existing users need migration (pre-1.0, acceptable)
- **More config files**: 3 files vs 1 (but clearer purpose for each)
- **No auto-bidirectional**: Users must explicitly import manual .claude/ edits
- **Migration complexity**: Need to migrate existing ~/.spn/mcp.yaml to new structure

### Neutral
- **Increased complexity**: More files to manage, but clearer responsibilities
- **Learning curve**: Users need to understand scope hierarchy (but it's standard)

## Implementation Plan

### Phase 1: Config Separation (Commit 1)
- Create `config.toml`, `mcp.yaml`, `local.yaml` structure
- Migrate existing `~/.spn/mcp.yaml` → `~/.spn/config.toml`
- Add scope resolution logic (Local > Team > Global)
- Update `spn config` command to support `--scope` flag

### Phase 2: Selective Sync (Commit 2)
- Read `manifest.yaml` for `requires_sync` field
- Filter sync by package type (only @skills/ and MCP)
- Skip @workflows/, @agents/, etc. in `spn sync`
- Add warning when non-syncable packages detected

### Phase 3: Scope Commands (Commit 3)
- Implement `spn config set/get --scope=local|team|global`
- Show scope source in `spn config show <key>`
- Visualize merged config in `spn sync --status`
- Add scope indicators (🌍 Global, 👥 Team, 💻 Local)

### Phase 4: Import Command (Commit 4)
- Implement `spn config import .claude/settings.json`
- Drift detection in `spn sync --status`
- Conflict resolution prompts
- Force overwrite option (`spn sync --force`)

## Alternatives Considered

### Alternative 1: Keep Current Architecture
**Rejected**: Technical debt would compound, user confusion would increase

### Alternative 2: Bidirectional Auto-Sync
**Rejected**: Creates write conflicts, unpredictable behavior, not how package managers work

### Alternative 3: Single Config File with Sections
**Rejected**: Doesn't solve scope separation (team vs local), can't commit partial config

### Alternative 4: Sync Everything
**Rejected**: Unnecessary for 70% of packages, slows down operations, clutters .claude/

## References

- npm config precedence: https://docs.npmjs.com/cli/v11/configuring-npm/npmrc
- Cargo config hierarchy: https://doc.rust-lang.org/cargo/reference/config.html
- Git config scopes: https://git-scm.com/docs/git-config#_configuration_file
- Claude Code settings: .claude/settings.json documentation

## Notes

This ADR supersedes any previous assumptions about config management.

Migration guide for users will be provided in `docs/guides/migration-v0.7.md`.
