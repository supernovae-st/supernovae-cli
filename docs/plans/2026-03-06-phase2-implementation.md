# Phase 2: UX Completion Plan

**Date:** 2026-03-06
**Status:** Complete
**Target:** spn-cli v0.13.0

---

## Overview

Complete remaining UX improvements from the original plan:
- A: "Did you mean" suggestions
- B: New help topics
- C: Schema bug fix + setup novanet
- D: Documentation updates
- E: Final verification

---

## A: "Did You Mean" Suggestions

### Goal
When user types a typo like `spn modle`, suggest the correct command.

### Implementation

**File:** `crates/spn/src/suggest.rs` (new)

```rust
// Levenshtein distance for fuzzy matching
// Find closest command when user makes typo
pub fn suggest_command(input: &str, commands: &[&str]) -> Option<String>
```

**Integration:** `main.rs` - catch unrecognized subcommand error

### Commands to Match
```
add, remove, install, update, outdated, search, info, list, publish,
version, init, mcp, skill, model, provider, nk, nv, schema, config,
sync, secrets, daemon, doctor, status, topic, setup
```

---

## B: New Help Topics

### Goal
Add missing topics to `spn topic <name>`:
- `models` - Local LLM management with Ollama
- `providers` - API key management
- `daemon` - Background service architecture
- `architecture` - How spn/nika/novanet work together

### Implementation

**File:** `crates/spn/src/commands/topic.rs`

Add new topic entries with clear, helpful content.

---

## C: Bug Fixes

### C1: Schema Status Bug

**Issue:** `spn schema status` may not match novanet's actual command.

**Action:** Verify novanet schema subcommands and align.

### C2: Setup NovaNet

**Issue:** `spn setup novanet` shows placeholder message.

**File:** `crates/spn/src/commands/setup.rs`

**Implementation:**
1. Check if novanet binary exists
2. If not, offer to install via Homebrew
3. Check Neo4j connection
4. Initialize novanet config
5. Optionally seed database

---

## D: Documentation Updates

### D1: Update Improvement Plan Status

Mark completed items in `docs/plans/2026-03-06-spn-cli-improvements.md`

### D2: Update CLAUDE.md

Add new commands and features.

---

## E: Verification

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. Manual test of each new feature
4. Commit and push

---

## Execution Order

```
1. [A] suggest.rs         → Fuzzy command matching
2. [B] topic.rs           → New topics
3. [C1] schema.rs         → Verify/fix schema status
4. [C2] setup.rs          → Implement setup novanet
5. [D] Documentation      → Update plans + CLAUDE.md
6. [E] Verification       → Test + commit
```

---

## Success Criteria

- [x] `spn modle` suggests `spn model`
- [x] `spn topic models` shows LLM help
- [x] `spn topic providers` shows API key help
- [x] `spn topic daemon` shows daemon help
- [x] `spn topic architecture` shows ecosystem diagram
- [x] `spn schema stats` works correctly (with `status` alias)
- [x] `spn setup novanet` runs wizard
- [x] All tests pass (740+ tests)
- [x] Documentation updated
