# RFC 0001: Unified Component Model

**Status:** Draft
**Target:** spn-cli v0.14.0
**Author:** Thibaut Melen
**Date:** 2026-03-07

---

## Summary

Introduce a unified `@` prefix syntax for all installable components, replacing multiple command families with a single consistent interface.

## Motivation

Currently, spn has separate command families for each component type:

```bash
spn mcp add neo4j           # MCP servers
spn skill add brainstorming # Skills
spn model pull llama3.2     # Models
spn add @nika/workflow      # Packages
```

This creates cognitive overhead:
- Users must remember which command family to use
- Tab completion is fragmented
- Documentation is scattered
- The mental model is inconsistent

## Proposal

### Unified @ Syntax

```bash
# NEW: Unified syntax
spn add @mcp/neo4j
spn add @skills/brainstorming
spn add @models/llama3.2
spn add @nika/generate-page

# Also works without 'add'
spn @mcp/neo4j
spn @skills/brainstorming
```

### Component Namespaces

| Namespace | Source | Current Command | New Command |
|-----------|--------|-----------------|-------------|
| `@mcp/` | npm registry | `spn mcp add X` | `spn add @mcp/X` |
| `@skills/` | skills.sh | `spn skill add X` | `spn add @skills/X` |
| `@models/` | Ollama | `spn model pull X` | `spn add @models/X` |
| `@nika/` | SuperNovae registry | `spn add @nika/X` | (unchanged) |
| `@novanet/` | SuperNovae registry | `spn add @novanet/X` | (unchanged) |
| `@workflows/` | SuperNovae registry | `spn add @workflows/X` | (unchanged) |
| `@jobs/` | SuperNovae registry | `spn add @jobs/X` | (unchanged) |

### Benefits

1. **Single Mental Model**: All components use the same `spn add @namespace/name` pattern
2. **Better Tab Completion**: `spn add @<tab>` shows all namespaces
3. **Unified Listing**: `spn list` shows everything in one view
4. **Simplified Documentation**: One pattern to learn, one section to document
5. **Extensible**: New component types just add a namespace

### Command Mapping

| Operation | Current | New |
|-----------|---------|-----|
| Add | `spn mcp add neo4j` | `spn add @mcp/neo4j` |
| Remove | `spn mcp remove neo4j` | `spn remove @mcp/neo4j` |
| List | `spn mcp list` | `spn list --namespace=mcp` or `spn list @mcp/*` |
| Search | `spn mcp search X` | `spn search @mcp/X` |
| Info | (none) | `spn info @mcp/neo4j` |

### Shorthand Syntax

For convenience, the `add` command can be omitted:

```bash
# Equivalent commands
spn add @mcp/neo4j
spn @mcp/neo4j

# Like git shortcuts
git commit -m "X"   # Full
git c -m "X"        # Alias
```

### Backwards Compatibility

The old commands remain as aliases for two major versions:

```bash
# v0.14: Both work
spn mcp add neo4j          # Deprecated, shows warning
spn add @mcp/neo4j         # Preferred

# v0.16: Old commands removed
spn mcp add neo4j          # Error: use 'spn add @mcp/neo4j'
```

## Implementation

### Phase 1: Parser

Add namespace detection to the CLI parser:

```rust
enum ComponentRef {
    Mcp(String),       // @mcp/name
    Skill(String),     // @skills/name
    Model(String),     // @models/name
    Package(String),   // @nika/name, @novanet/name, etc.
}

impl ComponentRef {
    fn parse(input: &str) -> Option<Self> {
        if let Some(name) = input.strip_prefix("@mcp/") {
            Some(ComponentRef::Mcp(name.to_string()))
        } else if let Some(name) = input.strip_prefix("@skills/") {
            Some(ComponentRef::Skill(name.to_string()))
        } else if let Some(name) = input.strip_prefix("@models/") {
            Some(ComponentRef::Model(name.to_string()))
        } else if input.starts_with('@') {
            Some(ComponentRef::Package(input.to_string()))
        } else {
            None
        }
    }
}
```

### Phase 2: Routing

Route to appropriate backend based on namespace:

```rust
async fn add_component(component: &str) -> Result<()> {
    match ComponentRef::parse(component) {
        Some(ComponentRef::Mcp(name)) => mcp::add(&name).await,
        Some(ComponentRef::Skill(name)) => skill::add(&name).await,
        Some(ComponentRef::Model(name)) => model::pull(&name).await,
        Some(ComponentRef::Package(name)) => package::add(&name).await,
        None => Err(SpnError::InvalidComponent(component.to_string())),
    }
}
```

### Phase 3: Unified Listing

Single `spn list` command with filters:

```bash
$ spn list
Installed Components

@mcp/
  neo4j           @neo4j/mcp-server-neo4j v0.1.0
  github          @modelcontextprotocol/server-github v0.2.0

@skills/
  brainstorming   v1.2.0
  tdd             v2.0.1

@models/
  llama3.2:1b     1.2 GB

@nika/
  generate-page   v1.2.0

Total: 6 components
```

### Phase 4: Deprecation Warnings

Add deprecation notices to old commands:

```rust
fn mcp_add(name: &str) {
    eprintln!(
        "{} 'spn mcp add' is deprecated. Use: spn add @mcp/{}",
        "⚠️".yellow(),
        name
    );
    // Continue with operation
}
```

## Alternatives Considered

### 1. Keep Separate Commands

**Rejected:** Increases cognitive load, fragments UX, harder to document.

### 2. Use Subcommands Instead of Prefixes

```bash
spn mcp:add neo4j    # Colon syntax
spn mcp.add neo4j    # Dot syntax
```

**Rejected:** Less intuitive than @ prefix, doesn't align with npm/cargo patterns.

### 3. Single Namespace for Everything

```bash
spn add neo4j        # Auto-detect type
```

**Rejected:** Ambiguous, requires guessing, potential conflicts.

## Open Questions

1. **Version constraints for models**: Should `@models/llama3.2:1b` include variant in name or use separate syntax?

2. **Scope specification**: How to specify global vs local in new syntax?
   - Option A: `spn add @mcp/neo4j --global`
   - Option B: `spn add @global/@mcp/neo4j`

3. **Alias expansion**: Should tab completion show full names?
   - `spn add @mcp/ne<tab>` → `@mcp/neo4j (@neo4j/mcp-server-neo4j)`

## Timeline

| Phase | Description | Version |
|-------|-------------|---------|
| 1 | RFC review and approval | - |
| 2 | Implement parser and routing | v0.14.0 |
| 3 | Add deprecation warnings | v0.14.0 |
| 4 | Update documentation | v0.14.0 |
| 5 | Remove deprecated commands | v0.16.0 |

## References

- [npm scoped packages](https://docs.npmjs.com/cli/v10/using-npm/scope)
- [Cargo package names](https://doc.rust-lang.org/cargo/reference/manifest.html#the-name-field)
- [Go module paths](https://go.dev/ref/mod#module-path)
