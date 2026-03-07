# Phase 3-5: Complete spn-cli Improvements

**Date:** 2026-03-07
**Status:** Complete
**Target:** spn-cli v0.13.0

---

## Overview

Complete all remaining improvements from the master plan:

| Phase | Description | Tasks | Effort |
|-------|-------------|-------|--------|
| 1 | Bug Fixes | 2 | 1h |
| 2 | Proxy Completeness | 11 commands | 2-3h |
| 3 | UX Improvements | ✅ Done | - |
| 4 | Documentation | 3 | 1-2h |
| 5 | Unified Model RFC | 1 (design only) | 1h |

---

## Phase 1: Bug Fixes

### 1A: Fix `config show` Output

**Issue:** `spn config show` may have truncated/empty output.

**File:** `crates/spn/src/commands/config.rs`

**Action:** Debug and fix output formatting.

### 1B: Verify `nv db` Subcommands

**Issue:** `spn nv db` subcommands may not match novanet.

**File:** `crates/spn/src/commands/nv.rs`

**Action:** Check `novanet db --help` and align.

---

## Phase 2: Proxy Completeness

### 2A: Nika Proxy Commands (3)

| Command | Proxies To | Description |
|---------|------------|-------------|
| `spn nk trace` | `nika trace` | Manage execution traces |
| `spn nk new` | `nika new` | Create workflow from template |
| `spn nk config` | `nika config` | Manage Nika configuration |

### 2B: NovaNet Proxy Commands (8)

| Command | Proxies To | Description |
|---------|------------|-------------|
| `spn nv search` | `novanet search` | Search nodes |
| `spn nv entity` | `novanet entity` | Entity operations |
| `spn nv export` | `novanet export` | Export subgraph |
| `spn nv locale` | `novanet locale` | Locale operations |
| `spn nv knowledge` | `novanet knowledge` | Knowledge generation |
| `spn nv stats` | `novanet stats` | Graph statistics |
| `spn nv diff` | `novanet diff` | Schema vs DB drift |
| `spn nv doc` | `novanet doc` | Documentation generation |

---

## Phase 3: UX Improvements

✅ **COMPLETED** in previous session:
- "Did you mean?" suggestions
- 4 new help topics (models, providers, daemon, architecture)
- Improved error messages

---

## Phase 4: Documentation

### 4A: README Architecture Diagram

Add visual architecture section to README.md.

### 4B: FAQ Section

Common questions and answers.

### 4C: Troubleshooting Guide

Common issues and solutions.

---

## Phase 5: Unified Model RFC (Design Only)

### Goal

Design the unified component model for v0.14:
- `@mcp/neo4j` instead of `spn mcp add neo4j`
- `@skills/brainstorming` instead of `spn skill add brainstorming`
- `@models/llama3.2` instead of `spn model pull llama3.2`

### Deliverable

RFC document in `docs/rfcs/0001-unified-component-model.md`

---

## Execution Order

```
1. [1A] Fix config show
2. [1B] Verify nv db
3. [2A] Add nk trace, new, config
4. [2B] Add nv search, entity, export, locale, knowledge, stats, diff, doc
5. [4A] README architecture
6. [4B] FAQ section
7. [4C] Troubleshooting guide
8. [5]  RFC document
9. [E]  Final verification and commit
```

---

## Success Criteria

- [x] `spn config show` displays full config (with helpful message when empty)
- [x] `spn nv db` subcommands match novanet (removed `start`, added `verify`)
- [x] `spn nk trace` works (already implemented)
- [x] `spn nk new` works (already implemented)
- [x] `spn nk config` works (already implemented)
- [x] `spn nv search` works (already implemented)
- [x] `spn nv entity` works (already implemented)
- [x] `spn nv export` works (already implemented)
- [x] `spn nv locale` works (already implemented)
- [x] `spn nv knowledge` works (already implemented)
- [x] `spn nv stats` works (already implemented)
- [x] `spn nv diff` works (already implemented)
- [x] `spn nv doc` works (already implemented)
- [x] README has architecture diagram (already had Mermaid diagrams)
- [x] FAQ section exists (added)
- [x] Troubleshooting guide exists (added)
- [x] RFC document created (`docs/rfcs/0001-unified-component-model.md`)
- [x] All tests pass (830 tests)
