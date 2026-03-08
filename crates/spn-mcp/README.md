# spn-mcp

Dynamic REST-to-MCP wrapper for the SuperNovae ecosystem.

## Overview

spn-mcp exposes REST APIs as MCP (Model Context Protocol) tools based on YAML configuration files. This allows Claude Code and Nika to interact with any REST API without writing custom MCP servers.

```
YAML Config -> MCP Tools

~/.spn/apis/dataforseo.yaml          MCP Tools:
├── name: dataforseo                 ├── dataforseo_keyword_ideas
├── tools:                           ├── dataforseo_domain_rank
│   ├── keyword_ideas         ->     ├── dataforseo_backlinks
│   ├── domain_rank                  └── dataforseo_referring_domains
│   └── backlinks
```

## Usage

### Via spn CLI (recommended)

```bash
# Start MCP server (loads all configs from ~/.spn/apis/)
spn mcp serve

# Start server for specific API only
spn mcp serve --api dataforseo

# List configured APIs
spn mcp apis list

# Validate configuration
spn mcp apis validate dataforseo

# Show API details
spn mcp apis info dataforseo
```

### Standalone Binary

```bash
# Start MCP server
spn-mcp serve

# List APIs
spn-mcp list

# Validate config
spn-mcp validate dataforseo
```

## Configuration

API configurations are YAML files in `~/.spn/apis/`:

```yaml
# ~/.spn/apis/dataforseo.yaml
name: dataforseo
version: "1.0"
base_url: https://api.dataforseo.com/v3
description: "DataForSEO API v3"

auth:
  type: basic
  credential: dataforseo  # resolved via spn daemon

rate_limit:
  requests_per_minute: 12
  burst: 2

headers:
  Content-Type: application/json

tools:
  - name: keyword_ideas
    description: "Get keyword ideas"
    method: POST
    path: /dataforseo_labs/google/keyword_ideas/live
    body_template: |
      [{"keywords": {{ keywords | json }}}]
    params:
      - name: keywords
        type: array
        items: string
        required: true
        description: "Seed keywords"
    response:
      extract: "tasks[0].result"
```

## Authentication Types

### Basic Auth
```yaml
auth:
  type: basic
  credential: myservice  # format: username:password
```

### Bearer Token
```yaml
auth:
  type: bearer
  credential: myservice  # The token value
```

### API Key
```yaml
auth:
  type: api_key
  credential: myservice
  location: header       # or: query
  key_name: X-API-Key
```

## Parameter Types

- `string` - Text values
- `integer` - Whole numbers
- `number` - Floating-point numbers
- `boolean` - true/false
- `array` - Lists (specify `items` type)
- `object` - Nested objects

## Credential Resolution

Credentials are resolved in this order:

1. **spn daemon** (recommended) - Stored in OS keychain
2. **Environment variable** - `{CREDENTIAL}_API_KEY`

```bash
# Store credential in keychain
spn provider set dataforseo

# Or use environment variable
export DATAFORSEO_API_KEY="username:password"
```

## Integration

### Claude Code

Add to your MCP settings (`~/.config/claude/settings.json`):

```json
{
  "mcpServers": {
    "spn-apis": {
      "command": "spn",
      "args": ["mcp", "serve"]
    }
  }
}
```

To load only specific APIs:

```json
{
  "mcpServers": {
    "stripe-api": {
      "command": "spn",
      "args": ["mcp", "serve", "--api", "stripe"]
    }
  }
}
```

For environment-based credentials (no spn daemon):

```json
{
  "mcpServers": {
    "spn-apis": {
      "command": "spn",
      "args": ["mcp", "serve"],
      "env": {
        "DATAFORSEO_API_KEY": "your-key-here",
        "STRIPE_API_KEY": "sk_live_xxx:"
      }
    }
  }
}
```

### Verifying Integration

After adding the MCP server to Claude Code, the tools will appear with names like:
- `dataforseo_keyword_ideas`
- `stripe_list_customers`
- `ahrefs_domain_rating`

### Nika Workflows

```yaml
steps:
  - invoke: dataforseo_keyword_ideas
    params:
      keywords: ["qr code", "barcode"]
    use.ctx: keyword_data
```

## Sample Configurations

Pre-configured APIs (after `spn setup`):

| API | Tools | Description |
|-----|-------|-------------|
| dataforseo | 7 | SEO data and keyword research |
| ahrefs | 5 | Backlinks and site analysis |
| semrush | 5 | SEO and competitive research |
| stripe | 9 | Payment processing |

## Troubleshooting

### "No secret found for provider"

Ensure the credential is stored:

```bash
# Check if credential exists
spn provider list

# Store credential if missing
spn provider set dataforseo
```

Or set environment variable: `export DATAFORSEO_API_KEY="user:pass"`

### "Could not connect to spn daemon"

The daemon auto-starts on credential access. If issues persist:

```bash
# Check daemon status
spn daemon status

# Manual start
spn daemon start
```

### Tools not appearing in Claude Code

1. Check config syntax: `spn mcp apis validate <name>`
2. Verify configs exist: `spn mcp apis list`
3. Check logs: `RUST_LOG=debug spn mcp serve 2>&1 | head -50`

## Development

```bash
# Build
cargo build -p spn-mcp

# Test
cargo test -p spn-mcp

# Run with tracing
RUST_LOG=debug spn mcp serve
```

## Security

spn-mcp includes multiple security hardening measures:

- **Path traversal protection**: API names are validated to prevent `../` attacks
- **URL injection prevention**: Tool paths are validated to prevent protocol-relative URLs
- **Credential isolation**: Secrets resolved via spn daemon (single keychain accessor)
- **Memory protection**: Credentials use `Zeroizing<T>` for auto-clear on drop

## License

AGPL-3.0-or-later
