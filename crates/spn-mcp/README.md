# spn-mcp

Dynamic REST-to-MCP wrapper for the SuperNovae ecosystem.

## Overview

spn-mcp exposes REST APIs as MCP (Model Context Protocol) tools based on YAML configuration files. This allows Claude Code and Nika to interact with any REST API without writing custom MCP servers.

## Usage

```bash
# Start MCP server (loads all configs from ~/.spn/apis/)
spn-mcp serve

# Start server for specific API only
spn-mcp serve --api dataforseo

# List configured APIs
spn-mcp list

# Validate configuration
spn-mcp validate dataforseo
```

## Configuration

API configurations are YAML files in `~/.spn/apis/`:

```yaml
# ~/.spn/apis/dataforseo.yaml
name: dataforseo
base_url: https://api.dataforseo.com/v3
description: "DataForSEO API v3"

auth:
  type: basic
  credential: dataforseo  # resolved via spn daemon

rate_limit:
  requests_per_minute: 12

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
```

## Authentication

Credentials are resolved via spn daemon (OS keychain):

```bash
# Store credential
spn provider set dataforseo

# Credential format for Basic auth: "username:password"
```

## Integration

### Claude Code

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "spn-mcp": {
      "command": "spn-mcp",
      "args": ["serve"]
    }
  }
}
```

### Nika Workflows

```yaml
steps:
  - invoke: dataforseo_keyword_ideas
    params:
      keywords: ["qr code", "barcode"]
```

## License

AGPL-3.0-or-later
