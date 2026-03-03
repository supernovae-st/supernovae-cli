# Secret Management UX Design

**Status:** Approved
**Date:** 2026-03-03
**Authors:** Thibaut + Claude

## Executive Summary

Complete redesign of secret management in spn CLI to provide:
- User-selectable storage options (keychain, .env, global)
- Interactive wizard for onboarding
- Key rotation reminders with metadata tracking
- Import/export with SOPS-compatible encryption
- 1Password/Vault hybrid integration

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                     SECRET MANAGEMENT v0.8.0                                    │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                        STORAGE BACKENDS                                  │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐  │   │
│  │  │  🔐 Keychain │  │  📁 .env     │  │  🌍 Global   │  │  🔒 1Pass   │  │   │
│  │  │  (OS native) │  │  (project)   │  │  (~/.spn/)   │  │  (hybrid)   │  │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                        ENCRYPTION LAYER                                  │   │
│  │  ┌──────────────────────────┐  ┌──────────────────────────┐             │   │
│  │  │  age crate (v0.11)       │  │  rops crate (v0.1.7)     │             │   │
│  │  │  • Simple encryption     │  │  • SOPS-compatible       │             │   │
│  │  │  • .env.age files        │  │  • .env.sops.yaml        │             │   │
│  │  │  • Passphrase/recipient  │  │  • AWS KMS / Age backend │             │   │
│  │  └──────────────────────────┘  └──────────────────────────┘             │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    │                                            │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                        METADATA & ROTATION                               │   │
│  │  ~/.spn/keys/metadata.toml                                               │   │
│  │  ┌────────────────────────────────────────────────────────────────────┐ │   │
│  │  │ [providers.anthropic]                                              │ │   │
│  │  │ key_id = "sk-ant-...abc"                                           │ │   │
│  │  │ storage = "keychain"                                               │ │   │
│  │  │ created_at = "2026-03-03T10:00:00Z"                                │ │   │
│  │  │ rotation_days = 90                                                 │ │   │
│  │  │ rotation_due = "2026-06-01T10:00:00Z"                              │ │   │
│  │  └────────────────────────────────────────────────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Features by Priority

### P0: --storage Flag (Critical Path)

```bash
# Current behavior (unchanged default)
spn provider set anthropic
# → Prompts for key, stores in keychain

# New: Explicit storage selection
spn provider set anthropic --storage keychain   # OS keychain (default)
spn provider set anthropic --storage env        # .env file (project)
spn provider set anthropic --storage global     # ~/.spn/secrets.env
spn provider set anthropic --storage shell      # Print export command only
```

**Implementation:**
- Add `--storage` flag to `ProviderSetArgs`
- Create `StorageBackend` enum: `Keychain | Env | Global | Shell`
- Route to appropriate storage function

### P1: Interactive Wizard

```
$ spn provider set anthropic

Setting API key for anthropic (ANTHROPIC_API_KEY)

? Where should this key be stored?

  ❯ 🔐 OS Keychain (recommended)
      Most secure. Protected by your login password.
      Note: May show popup on macOS until Developer ID signing.

    📁 Project .env file
      Stored in ./.env (gitignored). Good for project-specific keys.
      ⚠️ Make sure .env is in .gitignore!

    🌍 Global secrets file
      Stored in ~/.spn/secrets.env. Shared across all projects.

    📋 Copy to clipboard
      Key is validated and copied. You handle storage.

? Enter your Anthropic API key: sk-ant-api03-...

✅ API key stored in OS Keychain
   Masked: sk-ant-...xyz
   Rotation reminder set for 90 days (2026-06-01)
```

**Implementation:**
- Use `dialoguer::Select` for storage choice
- Use `dialoguer::Password` for key input
- Auto-detect if running in TTY for wizard vs direct mode

### P1: Provider Status Command

```bash
$ spn provider status

┌─────────────────────────────────────────────────────────────────────────┐
│  🔑 API KEY STATUS                                                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Provider       Storage      Status       Rotation                      │
│  ─────────────────────────────────────────────────────────────────────  │
│  anthropic      🔐 keychain  ✅ valid     ⚠️ 5 days (rotate soon!)      │
│  openai         📁 .env      ✅ valid     ✅ 67 days                    │
│  mistral        🌍 global    ✅ valid     ✅ 82 days                    │
│  groq           ❌ missing   -            -                             │
│                                                                         │
│  💡 Tip: Run 'spn provider set groq' to add missing key                 │
│  ⚠️ Warning: anthropic key rotation due in 5 days                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

**Implementation:**
- New subcommand `spn provider status`
- Read metadata from `~/.spn/keys/metadata.toml`
- Color-coded status indicators
- Rotation warnings

### P2: Config Defaults

```toml
# ~/.spn/config.toml

[secrets]
default_storage = "keychain"  # keychain | env | global
rotation_days = 90            # Default rotation reminder
warn_days = 14                # Days before rotation to start warning

[secrets.providers.anthropic]
storage = "keychain"          # Override per-provider
rotation_days = 60            # Anthropic keys rotate faster

[secrets.providers.openai]
storage = "env"               # OpenAI uses .env for this project
```

### P2: Import/Export with SOPS

```bash
# Export all keys to encrypted file
spn secrets export secrets.sops.yaml --format sops
spn secrets export secrets.age --format age

# Import from encrypted file
spn secrets import secrets.sops.yaml
spn secrets import secrets.age

# Encrypt existing .env file
spn secrets encrypt .env                    # → .env.sops.yaml
spn secrets encrypt .env --format age       # → .env.age

# Decrypt to .env
spn secrets decrypt secrets.sops.yaml       # → .env
```

### P3: Key Rotation Reminders

**Notification Strategy:**
1. **Auto-warning** on `spn provider list` / `spn provider status`
2. **Dedicated check** via `spn provider status --check-rotation`
3. **Shell hook** (optional) in `~/.zshrc`:
   ```bash
   spn provider status --quiet 2>/dev/null || true
   ```

### P3: 1Password/Vault Hybrid CLI

```bash
# Try 1Password CLI, fall back to manual
$ spn provider set anthropic --from 1password

Checking for 1Password CLI...
✅ Found: op version 2.24.0

? Select 1Password item:
  ❯ Anthropic API Key (Personal)
    Anthropic Production (Work)

✅ Imported from 1Password
   Stored in: OS Keychain
```

**Fallback if `op` not found:**
```
1Password CLI not found.

To install: brew install 1password-cli
Or enter key manually: [hidden input]
```

### P4: Encrypted .env Files

**Supported formats:**
- `.env.age` - age encryption (simple, personal)
- `.env.sops.yaml` - SOPS format (GitOps, teams)

**Auto-detection by extension:**
```rust
fn detect_format(path: &Path) -> EncryptedFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("age") => EncryptedFormat::Age,
        Some("sops") | Some("yaml") if path.to_string_lossy().contains(".sops") => {
            EncryptedFormat::Sops
        }
        _ => EncryptedFormat::Unknown,
    }
}
```

## File Structure

```
~/.spn/
├── config.toml              # User preferences
├── secrets.env              # Global secrets (unencrypted)
├── secrets.env.age          # Global secrets (age encrypted)
├── secrets.sops.yaml        # Global secrets (SOPS encrypted)
└── keys/
    └── metadata.toml        # Rotation metadata (non-sensitive)
```

## Security Considerations

1. **Memory Protection** - Continue using `secrecy` + `zeroize` for all secrets
2. **File Permissions** - 0o600 for secret files, 0o700 for directories
3. **Atomic Writes** - temp + fsync + rename pattern
4. **No Plaintext Logging** - Use `mask_api_key()` everywhere
5. **Validation** - Validate key format before storage

## Dependencies to Add

```toml
# Cargo.toml additions
age = "0.11"                  # Simple file encryption
rops = "0.1.7"                # SOPS-compatible encryption
```

## Testing Strategy

1. **Unit Tests** - Each storage backend, encryption format
2. **Integration Tests** - Full flow with mock keychain
3. **Security Tests** - Memory zeroing, file permissions, no leaks
4. **UX Tests** - Interactive flows with mock TTY

## Migration Path

- v0.7.0 → v0.8.0: Fully backward compatible
- Existing keychain keys continue to work
- New features are opt-in via flags/config
