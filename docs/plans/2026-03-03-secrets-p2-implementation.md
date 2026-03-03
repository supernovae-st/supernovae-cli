# Secrets P2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the remaining P2 secret management features for spn CLI.

**Architecture:** Building on the existing secrets module with SpnKeyring, StorageBackend, and provider commands.

**Tech Stack:** Rust, keyring-rs, zeroize, sops (external), TOML

---

## Task 1: `spn secrets doctor` Command

**Files:**
- Create: `src/commands/secrets.rs`
- Modify: `src/main.rs` (add Secrets subcommand)
- Modify: `src/commands/mod.rs` (export secrets module)

**Purpose:** Health check for secrets configuration, similar to `git doctor` or `brew doctor`.

### Step 1: Add SecretsCommands enum to main.rs

```rust
#[derive(Subcommand)]
enum SecretsCommands {
    /// Run health checks on secrets configuration
    Doctor {
        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,
    },
}
```

Add to Commands enum:
```rust
/// Secrets management and diagnostics
Secrets {
    #[command(subcommand)]
    command: SecretsCommands,
},
```

### Step 2: Create secrets.rs command module

Implement `run_doctor(fix: bool)` that checks:
1. **Keychain Access** - Can we read/write to OS keychain?
2. **Permissions** - Is `~/.spn/secrets.env` properly protected (0600)?
3. **Gitignore** - Is `.env` in `.gitignore`?
4. **Orphaned Keys** - Keys in storage but not in known providers?
5. **Duplicate Keys** - Same key in multiple storage locations?
6. **Invalid Formats** - Keys that don't pass validation?
7. **Memory Protection** - Is mlock available?

Output format (ASCII box like status command):
```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  🩺 SECRETS DOCTOR                                                            ║
╚═══════════════════════════════════════════════════════════════════════════════╝

✓ Keychain access: OK
✓ Global secrets file permissions: 0600 (secure)
⚠ .env not in .gitignore - RISK: accidental commit
✓ Memory protection: mlock available
✗ Invalid key format: openai (missing sk- prefix)

Issues found: 2 (1 warning, 1 error)
Run `spn secrets doctor --fix` to auto-fix where possible.
```

---

## Task 2: Config Defaults `~/.spn/config.toml`

**Files:**
- Create: `src/config/defaults.rs`
- Modify: `src/config/mod.rs`
- Modify: `src/main.rs` (load defaults early)

**Purpose:** User preferences file for default provider, storage backend, etc.

### Config Schema

```toml
# ~/.spn/config.toml

[secrets]
default_storage = "keychain"  # keychain, env, global
auto_migrate = true           # Auto-migrate env vars to keychain

[provider]
default = "anthropic"         # Default LLM provider
model = "claude-sonnet-4-20250514"

[mcp]
auto_discover = true          # Auto-discover MCP servers
```

### Implementation

1. Create `SpnConfig` struct with serde
2. Load from `~/.spn/config.toml` at startup
3. Merge with environment variables (env takes precedence)
4. Use in provider commands when no explicit storage specified

---

## Task 3: SOPS Import/Export

**Files:**
- Modify: `src/commands/secrets.rs` (add import/export commands)

**Purpose:** Encrypted export/import of secrets for team sharing.

### Commands

```bash
spn secrets export --format sops > team-secrets.yaml
spn secrets import team-secrets.yaml
```

### Export Format (SOPS-compatible)

```yaml
# Encrypted with SOPS using age/gpg
sops:
    age:
        - recipient: age1...
          enc: |
            -----BEGIN AGE ENCRYPTED FILE-----
            ...
            -----END AGE ENCRYPTED FILE-----
providers:
    anthropic: ENC[AES256_GCM,data:...,iv:...,tag:...,type:str]
    openai: ENC[AES256_GCM,data:...,iv:...,tag:...,type:str]
```

### Implementation

1. **Export**: Read all keys, format as YAML, exec `sops encrypt`
2. **Import**: Exec `sops decrypt`, parse YAML, store each key
3. Support plaintext export with `--plaintext` flag (with warning)

---

## Task 4: Verification Tests

**Purpose:** Ensure all P1 features work correctly.

### Test Checklist

1. [ ] `spn provider list` shows all providers
2. [ ] `spn provider list --show-source` shows storage locations
3. [ ] `spn provider set anthropic --storage keychain` works
4. [ ] `spn provider set anthropic --storage env` works
5. [ ] `spn provider set anthropic --storage global` works
6. [ ] `spn provider get anthropic` shows masked key
7. [ ] `spn provider get anthropic --unmask` shows full key
8. [ ] `spn provider delete anthropic` removes key
9. [ ] `spn provider migrate` moves keys to keychain
10. [ ] `spn provider test anthropic` validates key format
11. [ ] `spn provider test all` tests all providers
12. [ ] `spn provider status` shows diagnostic report
13. [ ] `spn provider status --json` outputs JSON
14. [ ] Interactive wizard with `spn provider set anthropic` (no key)
15. [ ] MCP secrets work: neo4j, github, slack, perplexity, firecrawl, supadata

---

## Execution Order

1. **Task 1** - `spn secrets doctor` (new command)
2. **Task 2** - Config defaults (foundation for future features)
3. **Task 3** - SOPS integration (team workflow)
4. **Task 4** - Verification tests (ensure nothing broke)

---

## Notes

- All secrets use `Zeroizing<String>` for memory safety
- Validation happens before any storage operation
- JSON output available for all commands (scripting support)
- ASCII boxes for visual output (consistent with provider status)
