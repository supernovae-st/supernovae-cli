# spn-core

Core types and validation for the SuperNovae ecosystem.

## Features

- **Zero dependencies** - Pure Rust, fast compilation, WASM-compatible
- **Provider definitions** - 13+ LLM and MCP service providers
- **Validation** - Key format validation with detailed error messages
- **MCP types** - Configuration types for MCP server management

## Usage

```rust
use spn_core::{
    // Provider definitions
    Provider, ProviderCategory, KNOWN_PROVIDERS,
    find_provider, provider_to_env_var,

    // Validation
    validate_key_format, mask_key, ValidationResult,

    // MCP types
    McpServer, McpConfig, McpSource,
};

// Validate an API key
match validate_key_format("anthropic", "sk-ant-...") {
    ValidationResult::Valid => println!("Key is valid!"),
    ValidationResult::InvalidPrefix { expected, .. } => {
        println!("Key should start with: {}", expected);
    }
    _ => {}
}

// Mask a key for display
let masked = mask_key("sk-ant-secret-key-12345");
assert_eq!(masked, "sk-ant-••••••••");

// Get environment variable name
let env_var = provider_to_env_var("anthropic");
assert_eq!(env_var, Some("ANTHROPIC_API_KEY"));
```

## Supported Providers

### LLM Providers (7)
- `anthropic` - Anthropic Claude
- `openai` - OpenAI GPT
- `mistral` - Mistral AI
- `groq` - Groq
- `deepseek` - DeepSeek
- `gemini` - Google Gemini
- `ollama` - Ollama (local)

### MCP Service Providers (6)
- `neo4j` - Neo4j Graph Database
- `github` - GitHub API
- `slack` - Slack API
- `perplexity` - Perplexity AI
- `firecrawl` - Firecrawl Web Scraping
- `supadata` - Supadata API

## License

AGPL-3.0-or-later
