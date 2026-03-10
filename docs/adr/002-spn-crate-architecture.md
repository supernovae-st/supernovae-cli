# ADR 002: Unified 4-Crate Architecture

**Status**: Accepted
**Date**: 2026-03-05
**Deciders**: Thibaut Melen, Claude, Nika

## Context

The SuperNovae ecosystem has multiple Rust binaries that need secure credential management:
- **spn-cli**: Package manager and setup wizard
- **Nika**: Workflow engine with MCP server spawning
- **Future tools**: NovaNet CLI, etc.

Problems with previous architecture:
1. **macOS Keychain popup fatigue**: Each process accessing Keychain triggers "allow access?" dialogs
2. **Code duplication**: Provider definitions duplicated across repos
3. **Inconsistent validation**: Different repos had different key format validation
4. **MCP server credentials**: Each MCP server process triggered separate Keychain prompts

## Decision

We will implement a **layered 4-crate architecture** where:
1. **spn-core** defines types (zero dependencies, WASM-compatible)
2. **spn-keyring** wraps OS Keychain
3. **spn-client** provides IPC to the daemon
4. **Consumers** (spn-cli, Nika) depend only on spn-client

### Crate Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  LAYER 1: spn-core (Zero Dependencies)                                          │
│  ├── KNOWN_PROVIDERS: 13 providers (7 LLM + 6 MCP)                              │
│  ├── Provider { id, name, env_var, category, key_prefix }                       │
│  ├── validate_key_format(), mask_key(), provider_to_env_var()                   │
│  └── McpServer, McpConfig (generic transport types)                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│  LAYER 2: spn-keyring (depends on spn-core)                                     │
│  ├── SpnKeyring::get/set/delete/exists                                          │
│  ├── OS integration: macOS Keychain, Windows Credential Manager, Linux Secret   │
│  └── Memory protection: Zeroizing<T>, SecretString, mlock()                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│  LAYER 3: spn-client (depends on spn-core, re-exports types)                    │
│  ├── SpnClient::connect() → Unix socket IPC to daemon                           │
│  ├── SpnClient::connect_with_fallback() → env var fallback                      │
│  ├── Re-exports: KNOWN_PROVIDERS, Provider, validate_key_format()               │
│  └── Protocol: Ping/GetSecret/HasSecret/ListProviders                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│  CONSUMERS (depend on spn-client only)                                          │
│  ├── spn-cli: provider set/get commands, setup wizard                           │
│  └── Nika: workflow engine, MCP server spawning                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Daemon Architecture

The `spn daemon` is the SOLE Keychain accessor:

```
Without daemon:           With daemon:
Nika → Keychain (popup)   Nika → spn-client → daemon.sock → Keychain
MCP1 → Keychain (popup)                        (one accessor, no popups)
MCP2 → Keychain (popup)
```

**Daemon lifecycle:**
- Auto-starts on first `spn provider get` (lazy initialization)
- Listens on `~/.spn/daemon.sock` (Unix socket)
- Single-instance via PID file with flock()
- Graceful shutdown on SIGTERM

### KNOWN_PROVIDERS (Single Source of Truth)

All 13 providers defined in spn-core:

```rust
pub static KNOWN_PROVIDERS: &[Provider] = &[
    // LLM (7)
    Provider { id: "anthropic", env_var: "ANTHROPIC_API_KEY", ... },
    Provider { id: "openai", env_var: "OPENAI_API_KEY", ... },
    Provider { id: "mistral", env_var: "MISTRAL_API_KEY", ... },
    Provider { id: "groq", env_var: "GROQ_API_KEY", ... },
    Provider { id: "deepseek", env_var: "DEEPSEEK_API_KEY", ... },
    Provider { id: "gemini", env_var: "GEMINI_API_KEY", ... },
    Provider { id: "ollama", env_var: "OLLAMA_API_BASE_URL", ... },
    // MCP (6)
    Provider { id: "neo4j", env_var: "NEO4J_PASSWORD", ... },
    Provider { id: "github", env_var: "GITHUB_TOKEN", ... },
    Provider { id: "slack", env_var: "SLACK_BOT_TOKEN", ... },
    Provider { id: "perplexity", env_var: "PERPLEXITY_API_KEY", ... },
    Provider { id: "firecrawl", env_var: "FIRECRAWL_API_KEY", ... },
    Provider { id: "supadata", env_var: "SUPADATA_API_KEY", ... },
];
```

### Consumer Integration Pattern

Consumers use spn-client via feature flag:

```toml
# Nika Cargo.toml
[dependencies]
spn-client = { version = "0.2", optional = true }

[features]
default = ["spn-daemon"]
spn-daemon = ["dep:spn-client"]
```

```rust
// Nika secrets.rs
#[cfg(feature = "spn-daemon")]
use spn_client::KNOWN_PROVIDERS;

pub async fn load_secrets() -> SecretsLoadResult {
    for p in KNOWN_PROVIDERS {
        // Use p.env_var, p.name, p.category
    }
}
```

### Fallback Mode

When daemon is unavailable (Docker, no socket):
1. `SpnClient::connect_with_fallback()` returns fallback client
2. Fallback reads from environment variables only
3. No Keychain access, no popups
4. Graceful degradation for CI/CD environments

## Consequences

### Positive

- **Zero popups**: Single Keychain accessor eliminates prompt fatigue
- **Single source of truth**: KNOWN_PROVIDERS in spn-core, no duplication
- **WASM-compatible core**: spn-core has zero dependencies
- **Graceful fallback**: Works without daemon (env vars only)
- **Clear dependency flow**: Consumers → spn-client → spn-core

### Negative

- **Daemon process**: Additional background process to manage
- **IPC overhead**: Unix socket round-trip vs direct Keychain access
- **Feature flag complexity**: Consumers need `spn-daemon` feature flag

### Neutral

- **crates.io publishing**: All 4 crates published independently
- **Version alignment**: Crates can version independently

## Implementation Details

### spn-core (v0.1.1)

```
crates/spn-core/
├── src/
│   ├── lib.rs          # Re-exports
│   ├── provider.rs     # Provider, KNOWN_PROVIDERS, ProviderCategory
│   ├── validate.rs     # validate_key_format(), mask_key()
│   ├── backend.rs      # BackendError, ModelInfo (for spn-providers, spn-native)
│   └── mcp.rs          # McpServer, McpConfig (generic types)
└── Cargo.toml          # [dependencies] (none)
```

### spn-keyring (v0.1.1)

```
crates/spn-keyring/
├── src/
│   └── lib.rs          # SpnKeyring, resolve_key(), SecretSource
└── Cargo.toml          # [dependencies] keyring, secrecy, zeroize, spn-core
```

### spn-client (v0.2.2)

```
crates/spn-client/
├── src/
│   ├── lib.rs          # SpnClient, re-exports from spn-core
│   ├── protocol.rs     # Request/Response enums
│   └── error.rs        # Error types
└── Cargo.toml          # [dependencies] tokio, serde, spn-core
```

## Migration Notes

### For Nika

```diff
# Cargo.toml
- keyring = "3"
- secrecy = "0.10"
+ spn-client = { version = "0.2", optional = true }

+ [features]
+ default = ["spn-daemon"]
+ spn-daemon = ["dep:spn-client"]

# secrets.rs
- const PROVIDERS: &[(&str, &str)] = &[
-     ("anthropic", "ANTHROPIC_API_KEY"),
-     ...
- ];
+ #[cfg(feature = "spn-daemon")]
+ use spn_client::KNOWN_PROVIDERS;
```

### For spn-cli

No changes needed - spn-cli already uses spn-keyring and spn-client.

## Alternatives Considered

### Alternative 1: Direct Keychain Access Everywhere

**Rejected**: Causes macOS popup fatigue, unacceptable UX

### Alternative 2: Environment Variables Only

**Rejected**: Less secure, doesn't leverage OS security

### Alternative 3: Shared Library (dylib)

**Rejected**: Complicates deployment, version conflicts

### Alternative 4: HTTP API Instead of Unix Socket

**Rejected**: Slower, more complex, security concerns

## References

- spn-client crate: https://crates.io/crates/spn-client
- spn-core crate: https://crates.io/crates/spn-core
- macOS Keychain documentation: https://developer.apple.com/documentation/security/keychain_services
- Unix domain sockets: https://man7.org/linux/man-pages/man7/unix.7.html

## Notes

This ADR documents the architecture as of spn-client v0.2.2 and Nika v0.21.0.

The daemon is automatically started by spn-cli and persists across sessions.
Users can manually stop it with `spn daemon stop` if needed.
