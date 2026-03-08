# Plan: Fix NovaNet MCP Authentication

**Date:** 2026-03-08
**Status:** In Progress
**Priority:** Critical

## Problem Summary

The NovaNet MCP server fails to authenticate with Neo4j, causing all specialized tools (`novanet_describe`, `novanet_search`, `novanet_traverse`, etc.) to return "unauthorized due to authentication failure".

Meanwhile, the generic Neo4j MCP (`mcp__neo4j__read_neo4j_cypher`) works fine with the same database.

## Root Cause Analysis

### Investigation Results

1. **Claude MCP list shows:**
   ```
   plugin:supernovae:novanet: novanet-mcp  - ✗ Failed to connect
   neo4j: uvx mcp-neo4j-cypher - ✓ Connected
   ```

2. **Plugin config** (`~/.claude/plugins/marketplaces/claude-code-supernovae/.mcp.json`):
   ```json
   {
     "mcpServers": {
       "novanet": {
         "command": "novanet-mcp",
         "env": {
           "NOVANET_MCP_NEO4J_PASSWORD": "${NOVANET_PASSWORD}"
         }
       }
     }
   }
   ```

3. **Environment variable check:**
   ```bash
   $ echo $NOVANET_PASSWORD
   # Empty - NOT SET!
   ```

### Root Cause

The Claude Code plugin references `${NOVANET_PASSWORD}` but this environment variable is not defined anywhere. The novanet-mcp server starts with an empty password and Neo4j rejects the authentication.

## Fix Options

### Option A: Set Environment Variable (Recommended)

Add `NOVANET_PASSWORD` to shell config for persistence.

**Pros:** Simple, secure (not in git), works globally
**Cons:** User must restart shell/Claude Code

### Option B: Use spn-keyring Integration

Store password in OS keychain via `spn provider set novanet`.

**Pros:** Most secure, follows our existing pattern
**Cons:** Requires novanet-mcp to support spn-keyring (code change)

### Option C: Hardcode Password in Plugin

Change plugin config to use literal password.

**Pros:** Works immediately
**Cons:** Less secure, password in git history

## Implementation Plan

### Phase 1: Immediate Fix (Option A)

1. [ ] Add `NOVANET_PASSWORD` to `~/.env` or `~/.zshrc`
2. [ ] Restart Claude Code
3. [ ] Verify with `claude mcp list`
4. [ ] Test all novanet_* tools

### Phase 2: Proper Integration (Option B)

1. [ ] Add `novanet` to KNOWN_PROVIDERS in spn-core
2. [ ] Update novanet-mcp to use spn-client for secrets
3. [ ] Update plugin .mcp.json to not require env var
4. [ ] Document the setup process

## Verification Steps

After fix, all these should work:

```
novanet_describe(describe="schema")
novanet_search(query="qr-code", kinds=["Entity"])
novanet_traverse(start_key="qr-code", direction="both")
novanet_introspect(target="classes")
```

## Files to Modify

| File | Change |
|------|--------|
| `~/.zshrc` or `~/.env` | Add NOVANET_PASSWORD |
| (Optional) `spn-core/src/providers.rs` | Add novanet provider |
| (Optional) `novanet-mcp/src/server/config.rs` | Add spn-client fallback |

## Timeline

- **Phase 1:** Immediate (5 minutes)
- **Phase 2:** Next sprint (if needed)

---

**Author:** Claude + Thibaut
**Last Updated:** 2026-03-08
