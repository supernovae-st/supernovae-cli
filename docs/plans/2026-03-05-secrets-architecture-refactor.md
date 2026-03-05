# Secrets Architecture Refactor Plan

**Date:** 2026-03-05
**Status:** BRAINSTORM
**Authors:** Thibaut + Claude

---

## Executive Summary

L'analyse de 5 agents a révélé **534 LOC dupliquées** entre `nika` et `spn-cli` pour la gestion des secrets. Ce plan propose une architecture clean avec 2 nouveaux crates et une migration en 3 phases.

---

## 1. Problèmes Identifiés

### 1.1 Duplication de Code

| Composant | nika | spn-cli | Copies |
|-----------|------|---------|--------|
| Provider list | 7 providers | 13 providers | 4 |
| Env var mapping | `provider_env_var()` | `provider_to_env_var()` | 3 |
| Keyring access | `SpnKeyring` (281 LOC) | `SpnKeyring` (731 LOC) | 2 |
| Key validation | `validate_key_format()` | `validate()` | 2 |
| Key masking | `mask_api_key()` | `mask()` | 2 |

### 1.2 Security Gap

```
nika providers:     [anthropic, openai, mistral, groq, deepseek, gemini, ollama]
spn-cli providers:  [anthropic, openai, mistral, groq, deepseek, gemini, ollama,
                     neo4j, github, slack, perplexity, firecrawl, supadata]

MISSING in nika:    neo4j, github, slack, perplexity, firecrawl, supadata (6 MCP!)
```

### 1.3 Anti-Patterns (Rust Pro Analysis)

1. **Unnecessary clone**: `SecretString::from(value.clone())` au lieu de `SecretString::from(value)`
2. **Sync lock across await**: `parking_lot::Mutex` held across `.await` points
3. **Plain String in IPC**: Protocol utilise `String` au lieu de `SecretString`

### 1.4 Usages à Migrer (Explorer Analysis)

```
nika/tools/nika/src/tui/widgets/provider_modal/
├── keyring.rs:47    SpnKeyring::new()
├── keyring.rs:52    SpnKeyring::get()
├── keyring.rs:61    SpnKeyring::set()
├── keyring.rs:70    SpnKeyring::delete()
├── keyring.rs:79    SpnKeyring::exists()
├── mod.rs:15        use keyring::SpnKeyring
├── mod.rs:89        SpnKeyring::get_secret()
├── state.rs:34      SpnKeyring::exists()
└── state.rs:156     SpnKeyring::set()
```

---

## 2. Architecture Proposée

### 2.1 Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PROPOSED ARCHITECTURE                                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐                                                            │
│  │  spn-secrets    │  ← NEW: Provider definitions, validation, env mapping     │
│  │  (no deps)      │     Zero external dependencies, pure Rust                  │
│  └────────┬────────┘                                                            │
│           │                                                                     │
│  ┌────────▼────────┐                                                            │
│  │  spn-keyring    │  ← NEW: Direct OS keychain access                          │
│  │  deps: keyring  │     Wraps keyring crate, provides SpnKeyring              │
│  │        secrecy  │                                                            │
│  └────────┬────────┘                                                            │
│           │                                                                     │
│  ┌────────▼────────┐                                                            │
│  │  spn-client     │  ← UPDATED: Daemon IPC + fallback to spn-keyring          │
│  │  deps: tokio    │     Unified API for all secret access                      │
│  │        secrecy  │                                                            │
│  │     spn-secrets │                                                            │
│  │     spn-keyring │                                                            │
│  └────────┬────────┘                                                            │
│           │                                                                     │
│  ┌────────▼────────┐      ┌─────────────┐                                       │
│  │      nika       │      │   spn-cli   │                                       │
│  │  deps:          │      │  deps:      │                                       │
│  │    spn-client   │      │  spn-client │                                       │
│  └──────────────────┘      └─────────────┘                                       │
│                                                                                 │
│  REMOVED FROM NIKA:                                                             │
│  ├── tui/widgets/provider_modal/keyring.rs  (281 LOC)                           │
│  └── Direct keyring dependency                                                  │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Crate Specifications

#### `spn-secrets` (NEW)

```toml
[package]
name = "spn-secrets"
version = "0.1.0"
description = "Provider definitions and validation for SuperNovae secrets"

[dependencies]
# NONE - pure Rust, no external deps
```

```rust
// src/lib.rs
pub mod providers;
pub mod validation;
pub mod env;

pub use providers::{Provider, ProviderCategory, KNOWN_PROVIDERS};
pub use validation::{validate_key_format, ValidationResult};
pub use env::provider_to_env_var;
```

**Exports:**
- `Provider` struct with name, env_var, category, prefix, min_length
- `ProviderCategory::Llm | Mcp | Custom`
- `KNOWN_PROVIDERS: &[Provider]` (all 13+)
- `validate_key_format(provider, key) -> ValidationResult`
- `provider_to_env_var(provider) -> &str`
- `mask_key(key) -> String`

#### `spn-keyring` (NEW)

```toml
[package]
name = "spn-keyring"
version = "0.1.0"
description = "OS keychain integration for SuperNovae secrets"

[dependencies]
spn-secrets = { version = "0.1", path = "../spn-secrets" }
keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
secrecy = "0.10"
thiserror = "2"
```

```rust
// src/lib.rs
pub struct SpnKeyring { /* ... */ }

impl SpnKeyring {
    pub fn new(provider: &str) -> Result<Self, KeyringError>;
    pub fn get(&self) -> Result<SecretString, KeyringError>;
    pub fn set(&self, secret: &str) -> Result<(), KeyringError>;
    pub fn delete(&self) -> Result<(), KeyringError>;
    pub fn exists(&self) -> bool;

    // Convenience
    pub fn get_secret(provider: &str) -> Result<SecretString, KeyringError>;
}
```

#### `spn-client` (UPDATED)

```toml
[dependencies]
spn-secrets = { version = "0.1", path = "../spn-secrets" }
spn-keyring = { version = "0.1", path = "../spn-keyring" }
# ... existing deps
```

```rust
// Add re-exports
pub use spn_secrets::{Provider, KNOWN_PROVIDERS, validate_key_format, mask_key};
pub use spn_keyring::SpnKeyring;
```

---

## 3. Migration Plan

### Phase 1: Create Foundation Crates (Day 1)

**Objectif:** Créer `spn-secrets` et `spn-keyring` sans casser l'existant.

```
Tasks:
├── [1.1] Create crates/spn-secrets/
│   ├── Cargo.toml
│   ├── src/lib.rs
│   ├── src/providers.rs      ← Consolidate all 13+ providers
│   ├── src/validation.rs     ← Key format validation
│   └── src/env.rs            ← Env var mapping
│
├── [1.2] Create crates/spn-keyring/
│   ├── Cargo.toml
│   ├── src/lib.rs
│   ├── src/keyring.rs        ← SpnKeyring impl
│   └── src/error.rs          ← KeyringError
│
├── [1.3] Add to workspace Cargo.toml
│
├── [1.4] Write tests
│   ├── spn-secrets: Unit tests (no I/O)
│   └── spn-keyring: Integration tests with mock keyring
│
└── [1.5] Publish to crates.io (internal first)
```

**Deliverables:**
- `spn-secrets` v0.1.0 (0 deps, ~200 LOC)
- `spn-keyring` v0.1.0 (~150 LOC)
- 100% test coverage for validation logic

### Phase 2: Update spn-client (Day 2)

**Objectif:** Refactor `spn-client` pour utiliser les nouveaux crates.

```
Tasks:
├── [2.1] Update spn-client/Cargo.toml
│   ├── Add dep: spn-secrets
│   └── Add dep: spn-keyring
│
├── [2.2] Remove duplicated code
│   ├── Delete: KNOWN_PROVIDERS in lib.rs
│   ├── Delete: provider_to_env_var() in lib.rs
│   └── Delete: validate() function
│
├── [2.3] Update FallbackClient
│   └── Use spn_keyring::SpnKeyring instead of inline impl
│
├── [2.4] Add re-exports
│   └── pub use spn_secrets::*; pub use spn_keyring::SpnKeyring;
│
├── [2.5] Fix anti-patterns
│   ├── Remove unnecessary .clone() on SecretString
│   ├── Use tokio::sync::Mutex instead of parking_lot
│   └── Consider SecretString in IPC protocol
│
└── [2.6] Bump version to 0.2.0
```

**Deliverables:**
- `spn-client` v0.2.0 (breaking change: re-exports)
- Reduced LOC by ~300
- All tests pass

### Phase 3: Migrate nika (Day 3)

**Objectif:** Supprimer la duplication dans nika, utiliser `spn-client` uniquement.

```
Tasks:
├── [3.1] Update nika/Cargo.toml
│   ├── Update: spn-client = "0.2"
│   └── Remove: keyring dependency (feature-gated fallback)
│
├── [3.2] Delete duplicated files
│   └── DELETE: tui/widgets/provider_modal/keyring.rs (281 LOC)
│
├── [3.3] Update imports in provider_modal/
│   ├── mod.rs: use spn_client::{SpnKeyring, validate_key_format, mask_key}
│   └── state.rs: use spn_client::SpnKeyring
│
├── [3.4] Update secrets.rs
│   └── Simplify: use spn_client for everything
│
├── [3.5] Update PROVIDERS list
│   └── Use spn_client::KNOWN_PROVIDERS (now 13+ instead of 7)
│
├── [3.6] Test all TUI flows
│   ├── Provider modal: add/edit/delete keys
│   ├── Key validation errors
│   └── Daemon/fallback modes
│
└── [3.7] Bump version to 0.21.0
```

**Deliverables:**
- `nika` v0.21.0
- **-281 LOC** (keyring.rs deleted)
- **+6 providers** (MCP secrets now available)
- Zero duplication

---

## 4. File Changes Summary

### Files to CREATE

| Path | LOC | Purpose |
|------|-----|---------|
| `crates/spn-secrets/Cargo.toml` | 15 | Package metadata |
| `crates/spn-secrets/src/lib.rs` | 20 | Module exports |
| `crates/spn-secrets/src/providers.rs` | 100 | Provider definitions |
| `crates/spn-secrets/src/validation.rs` | 80 | Key validation |
| `crates/spn-secrets/src/env.rs` | 30 | Env var mapping |
| `crates/spn-keyring/Cargo.toml` | 20 | Package metadata |
| `crates/spn-keyring/src/lib.rs` | 150 | SpnKeyring impl |

**Total NEW:** ~415 LOC

### Files to DELETE

| Path | LOC | Reason |
|------|-----|--------|
| `nika/.../provider_modal/keyring.rs` | 281 | Replaced by spn-keyring |

**Total DELETED:** 281 LOC

### Files to MODIFY

| Path | Changes |
|------|---------|
| `supernovae-cli/Cargo.toml` | Add workspace members |
| `crates/spn-client/Cargo.toml` | Add deps, bump to 0.2.0 |
| `crates/spn-client/src/lib.rs` | Remove duplication, add re-exports |
| `crates/spn/Cargo.toml` | Update spn-client dep |
| `nika/tools/nika/Cargo.toml` | Update spn-client, remove keyring |
| `nika/.../provider_modal/mod.rs` | Update imports |
| `nika/.../provider_modal/state.rs` | Update imports |
| `nika/.../secrets.rs` | Simplify, use KNOWN_PROVIDERS |

---

## 5. Testing Strategy

### Unit Tests (spn-secrets)

```rust
#[test]
fn test_all_providers_have_env_var() {
    for provider in KNOWN_PROVIDERS {
        assert!(!provider.env_var.is_empty());
    }
}

#[test]
fn test_validate_anthropic_key() {
    assert!(validate_key_format("anthropic", "sk-ant-api03-xxxxx").is_valid());
    assert!(!validate_key_format("anthropic", "invalid").is_valid());
}
```

### Integration Tests (spn-keyring)

```rust
#[test]
#[ignore] // Requires keychain access
fn test_keyring_roundtrip() {
    let kr = SpnKeyring::new("test-provider").unwrap();
    kr.set("test-secret").unwrap();
    assert_eq!(kr.get().unwrap().expose_secret(), "test-secret");
    kr.delete().unwrap();
    assert!(!kr.exists());
}
```

### E2E Tests (nika)

```bash
# Test daemon mode
spn daemon start
nika provider list  # Should show 13+ providers

# Test fallback mode
spn daemon stop
nika provider list  # Should still work via keyring
```

---

## 6. Rollback Plan

Si problème après Phase 3:

1. **Revert nika** to v0.20.1 (pre-migration)
2. **Keep** spn-secrets/spn-keyring (no breaking changes)
3. **Downgrade** spn-client to 0.1.x

Les nouveaux crates sont additifs, donc rollback = revert nika seulement.

---

## 7. Success Metrics

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Duplicated LOC | 534 | 0 | -534 |
| Provider copies | 4 | 1 | -3 |
| Providers in nika | 7 | 13+ | +6 |
| Keyring deps | 2 (nika + spn) | 1 (spn-keyring) | -1 |
| Test coverage | ~70% | 90%+ | +20% |

---

## 8. Open Questions

1. **Should `spn-secrets` be published to crates.io?**
   - Pro: Reusable by other projects
   - Con: Maintenance burden
   - **Decision:** Keep internal for now, publish later if demand

2. **SecretString in IPC protocol?**
   - Current: Plain String in JSON
   - Ideal: Encrypted or at least SecretString
   - **Decision:** Phase 2 - out of scope for this refactor

3. **Async keyring access?**
   - Current: Sync keyring crate
   - Tokio spawn_blocking is fine for occasional access
   - **Decision:** Keep sync, wrap in spawn_blocking

---

## 9. Timeline

```
Day 1: Phase 1 (Foundation)
├── Morning: Create spn-secrets
├── Afternoon: Create spn-keyring
└── Evening: Tests + Review

Day 2: Phase 2 (spn-client)
├── Morning: Refactor spn-client
├── Afternoon: Fix anti-patterns
└── Evening: Tests + Publish 0.2.0

Day 3: Phase 3 (nika)
├── Morning: Delete keyring.rs, update imports
├── Afternoon: Test all TUI flows
└── Evening: Publish 0.21.0

Day 4: Documentation
├── Update CLAUDE.md files
├── Update README.md
└── Create ADR for this decision
```

---

## 10. Decision

**APPROVED ARCHITECTURE:**

```
spn-secrets (pure Rust, 0 deps)
    ↓
spn-keyring (keyring crate wrapper)
    ↓
spn-client (daemon IPC + fallback)
    ↓
nika + spn-cli (consumers)
```

**Next Step:** Implement Phase 1 - Create `spn-secrets` crate.
