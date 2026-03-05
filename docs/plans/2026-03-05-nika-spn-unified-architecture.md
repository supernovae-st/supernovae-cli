# Nika-SPN Unified Architecture Plan

**Date:** 2026-03-05
**Version:** v1.0
**Status:** APPROVED
**Authors:** Thibaut + Claude (5-agent analysis + sequential thinking)

---

## Executive Summary

L'analyse de 5 agents spécialisés a révélé que l'architecture nika↔spn est **conceptuellement solide** mais souffre de **duplication massive** (~1000 LOC). Ce plan propose une architecture à **4 crates** qui élimine la duplication tout en gardant une API simple.

---

## 1. Current State Analysis

### 1.1 What nika CONSUMES from spn

| Category | Integration | Source | Status |
|----------|-------------|--------|--------|
| **Secrets** | spn-client IPC | `~/.spn/daemon.sock` | ✅ Implemented (v0.20.1) |
| **MCP Config** | Direct read | `~/.spn/mcp.yaml` | ✅ Works but duplicated |
| **Packages** | Registry lookup | `~/.spn/packages/` | ✅ Works but duplicated |
| **Providers** | Hardcoded list | `secrets.rs` | ⚠️ 7 vs 13 (missing 6 MCP) |
| **TUI** | File tree + keyring | `provider_modal/` | ⚠️ 281 LOC duplicated |

### 1.2 Duplication Inventory

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DUPLICATION ANALYSIS                                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  PROVIDERS (4 copies)                                                           │
│  ├── nika/secrets.rs:39-47          → 7 providers (ANTHROPIC..OLLAMA)          │
│  ├── nika/provider_modal/mod.rs     → 7 providers (hardcoded)                  │
│  ├── spn-client/lib.rs:274-288      → 13 providers (ANTHROPIC..SUPADATA)       │
│  └── spn/daemon/secrets.rs:18-32    → 13 providers (with env vars)             │
│                                                                                 │
│  MCP TYPES (2 copies)                                                           │
│  ├── nika/mcp/spn_config.rs         → SpnMcpServer, SpnMcpConfig (400 LOC)     │
│  └── spn/mcp/types.rs               → McpServer, McpConfig (100% identical)    │
│                                                                                 │
│  KEYRING WRAPPER (2 copies)                                                     │
│  ├── nika/provider_modal/keyring.rs → SpnKeyring (281 LOC)                     │
│  └── spn/secrets/keyring.rs         → SpnKeyring (731 LOC, superset)           │
│                                                                                 │
│  VALIDATION (2 copies)                                                          │
│  ├── nika/provider_modal/keyring.rs → validate_key_format()                    │
│  └── spn/secrets/types.rs           → validate() with more rules               │
│                                                                                 │
│  REGISTRY TYPES (2 copies)                                                      │
│  ├── nika/registry/types.rs         → PackageRef, Manifest                     │
│  └── spn/manifest/*.rs              → Similar types                            │
│                                                                                 │
│  TOTAL DUPLICATED: ~1000 LOC                                                    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 1.3 Security Gap

```
nika providers:  anthropic, openai, mistral, groq, deepseek, gemini, ollama
                 (7 LLM providers only)

spn providers:   anthropic, openai, mistral, groq, deepseek, gemini, ollama,
                 neo4j, github, slack, perplexity, firecrawl, supadata
                 (7 LLM + 6 MCP = 13 total)

MISSING IN NIKA: neo4j, github, slack, perplexity, firecrawl, supadata
                 → MCP servers can't use daemon for secrets!
```

---

## 2. Proposed Architecture

### 2.1 4-Crate Design

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  UNIFIED ARCHITECTURE (4 Crates)                                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  LAYER 1: spn-core (Pure Data, 0 Dependencies)                                  │
│  ┌───────────────────────────────────────────────────────────────────────────┐ │
│  │  • Provider definitions (13+ with env vars, prefixes, validation rules)  │ │
│  │  • MCP types (McpServer, McpConfig, McpSource)                            │ │
│  │  • Registry types (PackageRef, Manifest)                                  │ │
│  │  • Validation functions (validate_key_format, mask_key)                   │ │
│  │  • NO I/O, NO async, NO external deps = WASM-compatible                   │ │
│  └───────────────────────────────────────────────────────────────────────────┘ │
│                                     ▲                                           │
│                                     │                                           │
│  LAYER 2a: spn-keyring         LAYER 2b: (config merged into spn-client)       │
│  ┌─────────────────────────┐                                                    │
│  │  • OS keychain wrapper  │                                                    │
│  │  • Sync API only        │                                                    │
│  │  • Used by daemon ONLY  │                                                    │
│  │  • deps: keyring, secrecy│                                                   │
│  └─────────────────────────┘                                                    │
│            ▲                                                                    │
│            │                                                                    │
│  LAYER 3: spn-client (IPC + Config + Re-exports)                                │
│  ┌───────────────────────────────────────────────────────────────────────────┐ │
│  │  • Unix socket IPC to daemon (async)                                      │ │
│  │  • Env var fallback mode                                                  │ │
│  │  • Config loading (MCP, registry) - merged from proposed spn-config      │ │
│  │  • Re-exports EVERYTHING from spn-core                                    │ │
│  │  • deps: tokio, serde_yaml, dirs, spn-core                               │ │
│  └───────────────────────────────────────────────────────────────────────────┘ │
│                                     ▲                                           │
│                                     │                                           │
│  CONSUMERS                                                                      │
│  ┌─────────────────────────────┐    ┌─────────────────────────────────────┐   │
│  │         NIKA                │    │          SPN-CLI                    │   │
│  │  deps: spn-client           │    │  deps: spn-client, spn-keyring      │   │
│  │  (one import gets all)      │    │  (keyring for daemon writes)        │   │
│  └─────────────────────────────┘    └─────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Dependency Graph

```
                    spn-core
                   (0 deps)
                   /       \
                  /         \
        spn-keyring      spn-client
        (keyring,        (tokio, serde_yaml,
         secrecy)         dirs, spn-core)
              \              /
               \            /
                spn (CLI)
                (spn-client + spn-keyring)

                     |
                     v
                   nika
               (spn-client only)
```

### 2.3 Why 4 Crates (Not 3 or 5)?

| Option | Pros | Cons |
|--------|------|------|
| **3 crates** (merge core+config) | Simpler | Core would have I/O deps |
| **4 crates** (chosen) | Clean separation, 0-dep core | One more crate |
| **5 crates** (separate config) | Maximum separation | Over-engineered, config only used by client |

**Decision**: 4 crates balances simplicity with clean architecture.

---

## 3. Crate Specifications

### 3.1 spn-core

```toml
[package]
name = "spn-core"
version = "0.1.0"
description = "Core types and validation for SuperNovae ecosystem"
license = "AGPL-3.0-or-later"

[dependencies]
# NONE - pure Rust
```

**Contents:**

```rust
// src/lib.rs
pub mod providers;
pub mod mcp;
pub mod registry;
pub mod validation;

// Re-exports
pub use providers::{Provider, ProviderCategory, KNOWN_PROVIDERS};
pub use mcp::{McpServer, McpConfig, McpSource};
pub use registry::{PackageRef, Manifest, SkillEntry};
pub use validation::{validate_key_format, mask_key, ValidationResult};
```

```rust
// src/providers.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderCategory {
    Llm,
    Mcp,
    Custom,
}

#[derive(Debug, Clone)]
pub struct Provider {
    pub name: &'static str,
    pub env_var: &'static str,
    pub category: ProviderCategory,
    pub key_prefix: Option<&'static str>,
    pub min_length: usize,
}

pub const KNOWN_PROVIDERS: &[Provider] = &[
    // LLM (7)
    Provider { name: "anthropic", env_var: "ANTHROPIC_API_KEY", category: ProviderCategory::Llm, key_prefix: Some("sk-ant-"), min_length: 20 },
    Provider { name: "openai", env_var: "OPENAI_API_KEY", category: ProviderCategory::Llm, key_prefix: Some("sk-"), min_length: 20 },
    Provider { name: "mistral", env_var: "MISTRAL_API_KEY", category: ProviderCategory::Llm, key_prefix: None, min_length: 32 },
    Provider { name: "groq", env_var: "GROQ_API_KEY", category: ProviderCategory::Llm, key_prefix: Some("gsk_"), min_length: 20 },
    Provider { name: "deepseek", env_var: "DEEPSEEK_API_KEY", category: ProviderCategory::Llm, key_prefix: Some("sk-"), min_length: 20 },
    Provider { name: "gemini", env_var: "GEMINI_API_KEY", category: ProviderCategory::Llm, key_prefix: None, min_length: 20 },
    Provider { name: "ollama", env_var: "OLLAMA_API_BASE_URL", category: ProviderCategory::Llm, key_prefix: Some("http"), min_length: 10 },
    // MCP (6)
    Provider { name: "neo4j", env_var: "NEO4J_PASSWORD", category: ProviderCategory::Mcp, key_prefix: None, min_length: 8 },
    Provider { name: "github", env_var: "GITHUB_TOKEN", category: ProviderCategory::Mcp, key_prefix: Some("ghp_"), min_length: 20 },
    Provider { name: "slack", env_var: "SLACK_TOKEN", category: ProviderCategory::Mcp, key_prefix: Some("xoxb-"), min_length: 20 },
    Provider { name: "perplexity", env_var: "PERPLEXITY_API_KEY", category: ProviderCategory::Mcp, key_prefix: Some("pplx-"), min_length: 20 },
    Provider { name: "firecrawl", env_var: "FIRECRAWL_API_KEY", category: ProviderCategory::Mcp, key_prefix: Some("fc-"), min_length: 20 },
    Provider { name: "supadata", env_var: "SUPADATA_API_KEY", category: ProviderCategory::Mcp, key_prefix: None, min_length: 20 },
];

pub fn provider_to_env_var(name: &str) -> Option<&'static str> {
    KNOWN_PROVIDERS.iter()
        .find(|p| p.name == name)
        .map(|p| p.env_var)
}

pub fn find_provider(name: &str) -> Option<&'static Provider> {
    KNOWN_PROVIDERS.iter().find(|p| p.name == name)
}
```

```rust
// src/mcp.rs
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSource {
    Global,
    Project,
    Workflow,
}

#[derive(Debug, Clone)]
pub struct McpServer {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub source: Option<McpSource>,
}

#[derive(Debug, Clone)]
pub struct McpConfig {
    pub version: u32,
    pub servers: HashMap<String, McpServer>,
}
```

```rust
// src/validation.rs
use crate::providers::{find_provider, ProviderCategory};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    InvalidPrefix { expected: &'static str, got: String },
    TooShort { min: usize, got: usize },
    UnknownProvider,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }
}

pub fn validate_key_format(provider: &str, key: &str) -> ValidationResult {
    let Some(p) = find_provider(provider) else {
        return ValidationResult::UnknownProvider;
    };

    if key.len() < p.min_length {
        return ValidationResult::TooShort { min: p.min_length, got: key.len() };
    }

    if let Some(prefix) = p.key_prefix {
        if !key.starts_with(prefix) {
            return ValidationResult::InvalidPrefix {
                expected: prefix,
                got: key.chars().take(prefix.len()).collect()
            };
        }
    }

    ValidationResult::Valid
}

pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix_len = key.find('-').map(|i| i + 1).unwrap_or(4).min(8);
    format!("{}••••••••", &key[..prefix_len])
}
```

### 3.2 spn-keyring

```toml
[package]
name = "spn-keyring"
version = "0.1.0"
description = "OS keychain integration for SuperNovae secrets"
license = "AGPL-3.0-or-later"

[dependencies]
spn-core = { version = "0.1", path = "../spn-core" }
keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
secrecy = "0.10"
zeroize = { version = "1", features = ["derive"] }
thiserror = "2"
```

```rust
// src/lib.rs
use secrecy::zeroize::Zeroizing;
use thiserror::Error;

const SERVICE: &str = "spn";

#[derive(Error, Debug)]
pub enum KeyringError {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("provider not found: {0}")]
    NotFound(String),
}

pub struct SpnKeyring;

impl SpnKeyring {
    pub fn get(provider: &str) -> Result<Zeroizing<String>, KeyringError> {
        let entry = keyring::Entry::new(SERVICE, provider)?;
        let password = entry.get_password()?;
        Ok(Zeroizing::new(password))
    }

    pub fn set(provider: &str, secret: &str) -> Result<(), KeyringError> {
        let entry = keyring::Entry::new(SERVICE, provider)?;
        entry.set_password(secret)?;
        Ok(())
    }

    pub fn delete(provider: &str) -> Result<(), KeyringError> {
        let entry = keyring::Entry::new(SERVICE, provider)?;
        entry.delete_credential()?;
        Ok(())
    }

    pub fn exists(provider: &str) -> bool {
        Self::get(provider).is_ok()
    }
}
```

### 3.3 spn-client (Updated)

```toml
[package]
name = "spn-client"
version = "0.2.0"  # Breaking change
description = "Client library for SuperNovae daemon with config loading"
license = "AGPL-3.0-or-later"

[dependencies]
spn-core = { version = "0.1", path = "../spn-core" }
tokio = { version = "1.36", features = ["net", "io-util", "sync"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
secrecy = { version = "0.10", features = ["serde"] }
thiserror = "2"
dirs = "5.0"
tracing = "0.1"
rustc-hash = "2.1"
```

```rust
// src/lib.rs

// ══════════════════════════════════════════════════════════════════════════════
// RE-EXPORTS FROM spn-core (consumers get everything via spn-client)
// ══════════════════════════════════════════════════════════════════════════════

pub use spn_core::{
    // Providers
    Provider, ProviderCategory, KNOWN_PROVIDERS,
    provider_to_env_var, find_provider,
    // MCP
    McpServer, McpConfig, McpSource,
    // Registry
    PackageRef, Manifest, SkillEntry,
    // Validation
    validate_key_format, mask_key, ValidationResult,
};

// ══════════════════════════════════════════════════════════════════════════════
// CONFIG LOADING (merged from spn-config proposal)
// ══════════════════════════════════════════════════════════════════════════════

mod config;
pub use config::{
    // MCP config loading
    McpConfigLoader, load_mcp_servers, load_mcp_servers_by_name,
    // Registry/package loading
    spn_home, packages_dir, resolve_package_path, load_registry,
};

// ══════════════════════════════════════════════════════════════════════════════
// IPC CLIENT (existing functionality)
// ══════════════════════════════════════════════════════════════════════════════

mod daemon;
mod fallback;
mod protocol;
mod error;

pub use daemon::SpnClient;
pub use error::Error;
pub use secrecy::{ExposeSecret, SecretString};

// Helper: check if daemon is running
pub fn daemon_socket_exists() -> bool {
    daemon::socket_path().exists()
}
```

```rust
// src/config/mcp.rs
use crate::{McpConfig, McpServer, McpSource};
use std::path::PathBuf;
use std::collections::HashMap;

pub struct McpConfigLoader {
    global_path: PathBuf,
    project_root: Option<PathBuf>,
}

impl McpConfigLoader {
    pub fn new() -> Self {
        Self {
            global_path: dirs::home_dir()
                .unwrap_or_default()
                .join(".spn")
                .join("mcp.yaml"),
            project_root: None,
        }
    }

    pub fn with_project(mut self, root: PathBuf) -> Self {
        self.project_root = Some(root);
        self
    }

    pub fn load_global(&self) -> Result<McpConfig, crate::Error> {
        // ... load from ~/.spn/mcp.yaml
    }

    pub fn load_project(&self) -> Result<Option<McpConfig>, crate::Error> {
        // ... load from .spn/mcp.yaml
    }

    pub fn load_merged(&self) -> Result<McpConfig, crate::Error> {
        let mut config = self.load_global()?;
        if let Some(project_config) = self.load_project()? {
            for (name, server) in project_config.servers {
                config.servers.insert(name, server);
            }
        }
        Ok(config)
    }
}

/// Convenience function (matches nika's current API)
pub fn load_mcp_servers() -> Result<HashMap<String, McpServer>, crate::Error> {
    let config = McpConfigLoader::new().load_merged()?;
    Ok(config.servers.into_iter()
        .filter(|(_, s)| s.enabled)
        .collect())
}

pub fn load_mcp_servers_by_name(names: &[&str]) -> Result<HashMap<String, McpServer>, crate::Error> {
    let all = load_mcp_servers()?;
    Ok(all.into_iter()
        .filter(|(name, _)| names.contains(&name.as_str()))
        .collect())
}
```

---

## 4. Migration Plan

### Phase 1: Create Foundation (Day 1)

```
Tasks:
├── [1.1] Create crates/spn-core/
│   ├── Cargo.toml (0 deps)
│   ├── src/lib.rs
│   ├── src/providers.rs    ← Consolidate 4 copies
│   ├── src/mcp.rs          ← Consolidate 2 copies
│   ├── src/registry.rs
│   └── src/validation.rs   ← Consolidate 2 copies
│
├── [1.2] Create crates/spn-keyring/
│   ├── Cargo.toml
│   └── src/lib.rs          ← Extract from spn/secrets/keyring.rs
│
├── [1.3] Add to workspace Cargo.toml
│
└── [1.4] Write unit tests
    ├── spn-core: validation, provider lookup
    └── spn-keyring: mock tests
```

### Phase 2: Update spn-client (Day 2)

```
Tasks:
├── [2.1] Add spn-core dependency
│
├── [2.2] Add config loading module
│   ├── src/config/mod.rs
│   ├── src/config/mcp.rs      ← Extract from spn/mcp/config.rs
│   └── src/config/registry.rs ← Extract from spn/storage/
│
├── [2.3] Add re-exports in lib.rs
│
├── [2.4] Remove duplicated code
│   ├── DELETE: KNOWN_PROVIDERS (line 274-288)
│   └── DELETE: provider_to_env_var (line 291-308)
│
├── [2.5] Bump version to 0.2.0
│
└── [2.6] Update tests
```

### Phase 3: Update spn daemon (Day 2)

```
Tasks:
├── [3.1] Add spn-core, spn-keyring dependencies
│
├── [3.2] Update daemon/secrets.rs
│   ├── Use KNOWN_PROVIDERS from spn-core
│   └── Use SpnKeyring from spn-keyring
│
├── [3.3] Remove duplicated PROVIDERS constant
│
└── [3.4] Verify daemon still works
```

### Phase 4: Migrate nika (Day 3)

```
Tasks:
├── [4.1] Update Cargo.toml
│   ├── Update: spn-client = "0.2"
│   └── Remove: keyring (no longer needed)
│
├── [4.2] Delete duplicated files
│   └── DELETE: tui/widgets/provider_modal/keyring.rs (281 LOC)
│
├── [4.3] Update provider_modal/mod.rs
│   ├── Remove: mod keyring;
│   └── Add: use spn_client::{SpnKeyring, validate_key_format, mask_key, KNOWN_PROVIDERS};
│
├── [4.4] Update provider_modal/state.rs
│   └── Use spn_client::SpnKeyring
│
├── [4.5] Simplify mcp/spn_config.rs
│   └── Replace types with: use spn_client::{McpServer, McpConfig, load_mcp_servers};
│
├── [4.6] Update secrets.rs
│   └── Use KNOWN_PROVIDERS (now has 13+ instead of 7)
│
├── [4.7] Test all TUI flows
│
└── [4.8] Bump version to 0.21.0
```

### Phase 5: Documentation & ADR (Day 4)

```
Tasks:
├── [5.1] Update supernovae-cli/CLAUDE.md
├── [5.2] Update nika CLAUDE.md
├── [5.3] Create ADR-002-spn-crate-architecture.md
├── [5.4] Update README.md
└── [5.5] Create migration guide for external users
```

---

## 5. Quantified Impact

### 5.1 Lines of Code

| Action | LOC |
|--------|-----|
| **New**: spn-core | +350 |
| **New**: spn-keyring | +100 |
| **New**: spn-client config module | +300 |
| **Delete**: nika keyring.rs | -281 |
| **Delete**: nika spn_config.rs (partial) | -350 |
| **Delete**: spn-client duplicates | -150 |
| **Delete**: spn daemon duplicates | -50 |
| **Net Change** | **-81 LOC** |

### 5.2 Duplication

| Metric | Before | After |
|--------|--------|-------|
| Provider list copies | 4 | 1 |
| MCP type copies | 2 | 1 |
| Keyring wrapper copies | 2 | 1 |
| Validation copies | 2 | 1 |

### 5.3 Features

| Metric | Before | After |
|--------|--------|-------|
| Providers in nika | 7 | 13+ |
| MCP secrets in nika | ❌ | ✅ |
| Keychain popups | Multiple | Zero |

---

## 6. Consumer API (Post-Migration)

### 6.1 Nika Usage

```rust
// Single import gets EVERYTHING
use spn_client::{
    // Providers & validation
    Provider, KNOWN_PROVIDERS, validate_key_format, mask_key,
    ProviderCategory, find_provider,

    // MCP config
    McpServer, McpConfig, McpSource, load_mcp_servers,

    // Registry
    PackageRef, resolve_package_path, spn_home,

    // IPC client
    SpnClient, ExposeSecret, SecretString,

    // Helpers
    daemon_socket_exists,
};

// Load secrets
let mut client = SpnClient::connect_with_fallback().await?;
let key = client.get_secret("anthropic").await?;

// Load MCP servers
let servers = load_mcp_servers()?;

// Validate user input
match validate_key_format("anthropic", &user_input) {
    ValidationResult::Valid => { /* save */ },
    ValidationResult::InvalidPrefix { expected, .. } => {
        eprintln!("Key should start with {}", expected);
    },
    _ => { /* handle */ }
}
```

### 6.2 Cargo.toml

```toml
[dependencies]
spn-client = "0.2"  # Gets everything via re-exports
```

---

## 7. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking change in nika | Version bump 0.20 → 0.21, changelog |
| spn-client version skew | Config version field for compatibility |
| More crates = more maintenance | Workspace deps, automated CI |
| Circular dependency | spn nk is process proxy, not code dep |

---

## 8. Success Criteria

- [ ] Zero duplicated provider lists
- [ ] Zero duplicated MCP types
- [ ] Zero duplicated keyring wrappers
- [ ] nika has 13+ providers (was 7)
- [ ] MCP secrets work in nika workflows
- [ ] Zero keychain popups in production
- [ ] All existing tests pass
- [ ] `cargo clippy` clean

---

## 9. Open Questions

1. **Should spn-core be published to crates.io?**
   - Pro: Other projects could use provider definitions
   - Con: Maintenance burden
   - **Decision**: Internal for now, publish if demand

2. **Version synchronization strategy?**
   - Option A: Lockstep versions (all crates same version)
   - Option B: Independent semver (more flexible)
   - **Decision**: Independent semver with workspace deps

---

## 10. Timeline

| Day | Phase | Deliverable |
|-----|-------|-------------|
| 1 | Foundation | spn-core + spn-keyring created |
| 2 | spn-client | v0.2.0 with config loading + re-exports |
| 2 | spn daemon | Uses new crates |
| 3 | nika | v0.21.0 with zero duplication |
| 4 | Documentation | ADR + updated docs |

**Total: 4 days**

---

## Appendix A: File Reference

### Files to CREATE

| Path | Purpose |
|------|---------|
| `crates/spn-core/Cargo.toml` | Package metadata (0 deps) |
| `crates/spn-core/src/lib.rs` | Module exports |
| `crates/spn-core/src/providers.rs` | Provider definitions |
| `crates/spn-core/src/mcp.rs` | MCP types |
| `crates/spn-core/src/registry.rs` | Registry types |
| `crates/spn-core/src/validation.rs` | Validation functions |
| `crates/spn-keyring/Cargo.toml` | Package metadata |
| `crates/spn-keyring/src/lib.rs` | SpnKeyring |
| `crates/spn-client/src/config/mod.rs` | Config module |
| `crates/spn-client/src/config/mcp.rs` | MCP loader |
| `crates/spn-client/src/config/registry.rs` | Registry loader |

### Files to DELETE

| Path | Reason |
|------|--------|
| `nika/.../provider_modal/keyring.rs` | Use spn-keyring via spn-client |

### Files to MODIFY

| Path | Changes |
|------|---------|
| `supernovae-cli/Cargo.toml` | Add workspace members |
| `crates/spn-client/Cargo.toml` | Add spn-core dep, bump to 0.2.0 |
| `crates/spn-client/src/lib.rs` | Add re-exports, config module |
| `crates/spn/Cargo.toml` | Add spn-core, spn-keyring deps |
| `crates/spn/src/daemon/secrets.rs` | Use new crates |
| `nika/tools/nika/Cargo.toml` | Update spn-client, remove keyring |
| `nika/.../provider_modal/mod.rs` | Update imports |
| `nika/.../provider_modal/state.rs` | Update imports |
| `nika/.../mcp/spn_config.rs` | Simplify with spn-client |
| `nika/.../secrets.rs` | Use KNOWN_PROVIDERS |

---

**APPROVED FOR IMPLEMENTATION**
