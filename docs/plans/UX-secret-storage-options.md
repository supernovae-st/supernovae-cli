# UX: Secret Storage Options

**Status:** Proposed
**Priority:** High (core UX improvement)
**Related:** TODO-developer-id-signing.md

## Current State

```
┌─────────────────────────────────────────────────────────────────┐
│  CURRENT: spn provider set anthropic                            │
│                                                                  │
│  → ALWAYS stores in OS Keychain                                  │
│  → No choice for user                                            │
│  → macOS shows popup without Developer ID                        │
└─────────────────────────────────────────────────────────────────┘
```

**Resolution priority (read-only):**
1. OS Keychain
2. Environment variable
3. .env file

## Proposed: User-Selectable Storage

### Option 1: Flag-Based Selection

```bash
# Store in keychain (current default)
spn provider set anthropic

# Store in keychain explicitly
spn provider set anthropic --storage keychain

# Add to .env file
spn provider set anthropic --storage env

# Add to ~/.spn/secrets.env (global)
spn provider set anthropic --storage global-env

# Just validate and print export command
spn provider set anthropic --storage shell
# Output: export ANTHROPIC_API_KEY='sk-ant-...'
```

### Option 2: Interactive Wizard

```bash
$ spn provider set anthropic

Setting API key for anthropic (ANTHROPIC_API_KEY)

Where should this key be stored?

  ❯ 🔐 OS Keychain (recommended)
      Most secure. Protected by your login password.
      Note: May show popup on macOS until we have Developer ID signing.

    📁 Project .env file
      Stored in ./.env (gitignored). Good for project-specific keys.
      ⚠️ Make sure .env is in .gitignore!

    🌍 Global secrets file
      Stored in ~/.spn/secrets.env. Shared across all projects.

    📋 Copy to clipboard
      Key is validated and copied. You handle storage.
```

### Option 3: Configuration Default

```toml
# ~/.spn/config.toml

[secrets]
default_storage = "keychain"  # keychain | env | global-env | prompt

# Per-provider override
[secrets.providers.anthropic]
storage = "env"  # This provider uses .env
```

## Storage Comparison Table

| Storage | Security | Convenience | macOS Popup | CI/CD | Multi-Project |
|---------|----------|-------------|-------------|-------|---------------|
| 🔐 Keychain | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | Yes* | ❌ | ✅ |
| 📁 .env | ⭐⭐⭐ | ⭐⭐⭐⭐ | No | ⚠️ | ❌ |
| 🌍 ~/.spn/secrets.env | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | No | ❌ | ✅ |
| 📦 Shell env | ⭐⭐ | ⭐⭐ | No | ✅ | ✅ |

*Until we have Developer ID signing ($99/year)

## Recommendation

**Phase 1 (Now):** Add `--storage` flag
- Minimal change
- Backward compatible
- Users can choose

**Phase 2 (Later):** Interactive wizard
- Better onboarding
- Explains security tradeoffs
- Config file for defaults

## Implementation Notes

### .env Storage

```rust
// In keyring.rs or new env_storage.rs
fn store_in_dotenv(provider: &str, key: &str, global: bool) -> Result<()> {
    let env_var = provider_env_var(provider);
    let path = if global {
        dirs::home_dir().unwrap().join(".spn/secrets.env")
    } else {
        PathBuf::from(".env")
    };

    // Append to .env file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    writeln!(file, "{}={}", env_var, key)?;

    // Warn if not gitignored
    if !global && !is_gitignored(".env") {
        eprintln!("⚠️ Warning: .env is not in .gitignore!");
    }

    Ok(())
}
```

### Security Considerations

1. **Keychain** - Best security, managed by OS
2. **.env files** - Should be gitignored, readable by processes
3. **Shell env** - Visible in `ps aux`, least secure
4. **~/.spn/secrets.env** - Good middle ground, single location

### Messages to User

```
When user chooses .env:
"⚠️ Make sure .env is in your .gitignore to avoid committing secrets!"

When user chooses shell:
"⚠️ Environment variables may be visible to other processes.
   For production, consider using a secrets manager."

When keychain shows popup:
"💡 Tip: The macOS popup will persist until we have Developer ID signing.
   Use --storage env to avoid the popup, or click 'Allow' each time."
```

## Commands Summary (After Implementation)

```bash
# Set key (interactive choice)
spn provider set <name>

# Set with explicit storage
spn provider set <name> --storage keychain
spn provider set <name> --storage env
spn provider set <name> --storage global-env
spn provider set <name> --storage shell

# List with source info
spn provider list --show-source

# Migrate between storage types
spn provider migrate --from env --to keychain
spn provider migrate --from keychain --to global-env

# Configure default
spn config set secrets.default_storage env
```

## Open Questions

1. Should `--storage shell` actually store anything or just print?
2. Should we support encrypted .env files?
3. Should we integrate with external secrets managers (1Password, Vault)?
