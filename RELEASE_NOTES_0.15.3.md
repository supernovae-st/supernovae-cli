# spn v0.15.3 - Keyring UX Revolution

## TL;DR

**Zero-blocking daemon startup** - No more waiting for keychain popups before you can use `spn`.

```bash
# Before: daemon blocked until all keychain popups acknowledged (13+ popups!)
spn daemon start  # ... wait ... wait ... wait ...

# After: instant availability with lazy loading
spn daemon start --skip-preload --lazy  # Ready in <100ms
```

## What's New

### Daemon Startup Modes

| Flag | Behavior |
|------|----------|
| `--skip-preload` | Skip loading secrets at startup (socket available immediately) |
| `--lazy` | Load secrets on-demand (no keychain access until needed) |
| Both flags | Zero-blocking startup, secrets loaded on first request |

### Progress Output

Now you can see exactly what's happening during preload:

```
[1/13] Loaded: anthropic
[2/13] Loaded: openai
[3/13] Not found: mistral
...
Preload complete: 3 loaded, 10 not found, 0 errors
```

### Auto-Refresh After `spn provider set`

When you store a new API key, the daemon cache is automatically updated:

```bash
spn provider set anthropic  # Daemon cache refreshed automatically
# No need to restart daemon or manually invalidate cache
```

### New IPC: RefreshSecret

For tool integrations, the new `RefreshSecret` message allows external tools to trigger cache refresh:

```rust
let mut client = SpnClient::connect().await?;
let refreshed = client.refresh_secret("anthropic").await?;
```

## Breaking Changes

None - all changes are additive and backward compatible.

## Upgrade Path

```bash
# Option 1: Homebrew
brew upgrade spn

# Option 2: Cargo
cargo install spn-cli --force

# Option 3: Docker
docker pull ghcr.io/supernovae-st/spn:0.15.3
```

## Full Changelog

### Added
- `--skip-preload` flag for immediate daemon availability
- `--lazy` flag for on-demand secret loading
- Progress output during secret preload
- `RefreshSecret` IPC message for cache invalidation

### Changed
- GetSecret handler uses `get_or_load()` for lazy loading
- Auto-refresh daemon cache after `spn provider set`

### Fixed
- Use `&Path` instead of `&PathBuf` in `validate_workflow_path`

---

**Full Changelog**: https://github.com/supernovae-st/supernovae-cli/compare/0.15.2...0.15.3
