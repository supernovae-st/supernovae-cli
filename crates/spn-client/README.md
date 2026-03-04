# spn-client

Client library for communicating with the spn daemon.

## Overview

`spn-client` provides a simple, secure interface for Rust applications to retrieve API keys and secrets from the spn daemon. This eliminates the need for each application to directly access the OS keychain, solving the macOS Keychain popup problem where multiple binaries cannot share "Always Allow" permissions.

## Features

- **Unix Socket IPC**: Secure communication with the spn daemon
- **Fallback Mode**: Gracefully falls back to environment variables if daemon is unavailable
- **Zero Dependencies on Keychain**: No direct keyring/keychain access required
- **Async-First**: Built on Tokio for modern async Rust applications

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
spn-client = "0.1"
```

## Usage

### Basic Usage

```rust
use spn_client::SpnClient;

#[tokio::main]
async fn main() -> Result<(), spn_client::Error> {
    // Connect to the daemon
    let mut client = SpnClient::connect().await?;

    // Get an API key
    let api_key = client.get_secret("anthropic").await?;

    // Use with your LLM client
    let llm_client = anthropic::Client::with_api_key(api_key.expose_secret());

    Ok(())
}
```

### Fallback Mode

For applications that should work even without the daemon:

```rust
use spn_client::SpnClient;

#[tokio::main]
async fn main() -> Result<(), spn_client::Error> {
    // Falls back to env vars if daemon unavailable
    let mut client = SpnClient::connect_with_fallback().await?;

    if client.is_fallback_mode() {
        println!("Warning: Running without daemon, using env vars");
    }

    let api_key = client.get_secret("anthropic").await?;
    Ok(())
}
```

### Check Secret Availability

```rust
use spn_client::SpnClient;

async fn check_providers(client: &mut SpnClient) -> Result<(), spn_client::Error> {
    // Check if a specific provider is configured
    if client.has_secret("openai").await? {
        println!("OpenAI available");
    }

    // List all available providers
    let providers = client.list_providers().await?;
    println!("Available: {:?}", providers);

    Ok(())
}
```

## Supported Providers

| Provider | Environment Variable Fallback |
|----------|-------------------------------|
| anthropic | ANTHROPIC_API_KEY |
| openai | OPENAI_API_KEY |
| mistral | MISTRAL_API_KEY |
| groq | GROQ_API_KEY |
| deepseek | DEEPSEEK_API_KEY |
| gemini | GEMINI_API_KEY |
| ollama | OLLAMA_HOST |
| neo4j | NEO4J_PASSWORD |
| github | GITHUB_TOKEN |
| perplexity | PERPLEXITY_API_KEY |
| firecrawl | FIRECRAWL_API_KEY |

## Protocol

Communication uses length-prefixed JSON over Unix sockets:

```
Socket: ~/.spn/daemon.sock
Format: [4-byte big-endian length][JSON payload]
```

See the [protocol documentation](../spn/src/daemon/README.md) for details.

## Security

- Socket permissions: `0600` (owner only)
- Peer credential verification via `SO_PEERCRED`
- Secrets never written to disk
- Memory zeroed on drop (via `secrecy` crate)

## License

MIT
