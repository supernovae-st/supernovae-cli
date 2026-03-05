# spn-keyring

OS keychain wrapper for the SuperNovae CLI ecosystem.

## Features

- **Cross-platform**: macOS Keychain, Windows Credential Manager, Linux Secret Service
- **Secure by default**: Auto-zeroizing strings, SecretString wrapping
- **Validation**: Uses `spn-core` provider definitions for key format validation
- **Migration**: Helpers to migrate from env vars to keychain

## Usage

```rust
use spn_keyring::{SpnKeyring, KeyringError};
use zeroize::Zeroizing;

// Store a key
SpnKeyring::set("anthropic", "sk-ant-...")?;

// Retrieve with auto-zeroize
let key: Zeroizing<String> = SpnKeyring::get("anthropic")?;

// Check if key exists
if SpnKeyring::exists("openai") {
    println!("OpenAI key is configured");
}

// Delete a key
SpnKeyring::delete("anthropic")?;

// List all stored providers
let providers = SpnKeyring::list();
```

## Security

All retrieved keys are wrapped in `Zeroizing<String>` which automatically
clears memory when dropped. For maximum safety when passing to external APIs,
use `SpnKeyring::get_secret()` which returns a `SecretString`.

## Integration with spn-core

This crate uses `spn_core::KNOWN_PROVIDERS` as the single source of truth
for provider definitions, including:
- Provider IDs and names
- Environment variable mappings
- Key prefix validation
- Category (LLM, MCP, Local)

## License

MIT OR Apache-2.0
