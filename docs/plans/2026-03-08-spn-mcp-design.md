# spn-mcp: Dynamic REST-to-MCP Wrapper

**Date:** 2026-03-08
**Status:** Approved
**Author:** Thibaut + Claude

## Overview

spn-mcp is a Rust MCP server that dynamically wraps REST APIs into MCP tools at runtime. It reads YAML configuration files that describe API endpoints and exposes them as MCP tools to Claude Code and Nika.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  spn-mcp — Dynamic REST-to-MCP Wrapper                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌──────────────────────┐     ┌──────────────────────┐                         │
│  │  ~/.spn/apis/        │     │  spn-mcp binary      │                         │
│  │  ├── dataforseo.yaml │────▶│  ├── Config loader   │                         │
│  │  ├── ahrefs.yaml     │     │  ├── Tool registry   │                         │
│  │  └── semrush.yaml    │     │  ├── HTTP executor   │──────▶ REST APIs        │
│  └──────────────────────┘     │  └── MCP server      │                         │
│                               └──────────────────────┘                         │
│                                        │                                        │
│                                        │ stdio (JSON-RPC)                       │
│                                        ▼                                        │
│                          ┌──────────────────────────┐                          │
│                          │  MCP Clients             │                          │
│                          │  ├── Claude Code         │                          │
│                          │  └── Nika workflows      │                          │
│                          └──────────────────────────┘                          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Motivation

Many APIs don't have official MCP servers. Creating one-off servers for each API is tedious. spn-mcp solves this by:

1. **YAML-driven**: Define API → MCP mapping in config files
2. **Dynamic registration**: No recompilation needed to add new APIs
3. **Unified auth**: Credentials managed via spn daemon (keychain)
4. **Rate limiting**: Built-in per-API rate limiting
5. **Ecosystem integration**: Works with Nika, Claude Code, and spn

## Architecture Decision

### B+C Hybrid Approach

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  WHY B+C HYBRID?                                                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Option A: Daemon extension (pure C)                                            │
│  ├── Problem: MCP servers are CHILD processes of Claude/Nika                   │
│  ├── Problem: Daemon is already-running BACKGROUND process                     │
│  └── ❌ Architecture mismatch                                                   │
│                                                                                 │
│  Option B: Standalone binary (pure B)                                           │
│  ├── Works: spn-mcp is spawned by Claude/Nika as child process                 │
│  ├── Problem: How to access keychain credentials?                              │
│  └── ⚠️  Needs credential solution                                              │
│                                                                                 │
│  Option B+C: Standalone binary + daemon for secrets                             │
│  ├── spn-mcp binary: spawned by Claude/Nika                                    │
│  ├── spn-client: IPC to daemon for credentials                                 │
│  ├── spn daemon: keychain access (no popup fatigue)                            │
│  └── ✅ Clean separation of concerns                                            │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Ecosystem Integration

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  SUPERNOVAE ECOSYSTEM                                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌────────────────────────────────────────────────────────────────────────────┐│
│  │  MCP CONSUMERS (Clients)                                                   ││
│  │  ┌─────────────────┐    ┌─────────────────┐                               ││
│  │  │  Claude Code    │    │  Nika Workflows │                               ││
│  │  │  (direct chat)  │    │  (invoke: verb) │                               ││
│  │  └────────┬────────┘    └────────┬────────┘                               ││
│  │           │                      │                                         ││
│  └───────────┼──────────────────────┼─────────────────────────────────────────┘│
│              │                      │                                          │
│              │  stdio (JSON-RPC)    │  stdio (JSON-RPC)                        │
│              │                      │                                          │
│  ┌───────────┼──────────────────────┼─────────────────────────────────────────┐│
│  │  MCP PROVIDERS (Servers)         │                                         ││
│  │           │                      │                                         ││
│  │           ▼                      ▼                                         ││
│  │  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐        ││
│  │  │  novanet-mcp    │    │  spn-mcp        │    │  Other MCPs     │        ││
│  │  │  (knowledge)    │    │  (REST wrapper) │    │  (plugins)      │        ││
│  │  │                 │    │                 │    │                 │        ││
│  │  │  novanet_query  │    │  dataforseo_*   │    │  firecrawl_*    │        ││
│  │  │  novanet_search │    │  ahrefs_*       │    │  perplexity_*   │        ││
│  │  │  novanet_generate    │  semrush_*      │    │  github_*       │        ││
│  │  └────────┬────────┘    └────────┬────────┘    └─────────────────┘        ││
│  │           │                      │                                         ││
│  └───────────┼──────────────────────┼─────────────────────────────────────────┘│
│              │                      │                                          │
│              ▼                      ▼                                          │
│  ┌─────────────────┐       ┌─────────────────┐                                 │
│  │  Neo4j          │       │  REST APIs      │                                 │
│  │  (NovaNet KG)   │       │  (DataForSEO,   │                                 │
│  │                 │       │   Ahrefs, etc)  │                                 │
│  └─────────────────┘       └─────────────────┘                                 │
│                                                                                 │
│  ┌────────────────────────────────────────────────────────────────────────────┐│
│  │  SHARED INFRASTRUCTURE                                                     ││
│  │                                                                            ││
│  │  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐        ││
│  │  │  spn daemon     │◄───│  spn-client     │◄───│  spn-mcp        │        ││
│  │  │  (keychain)     │    │  (IPC library)  │    │  (uses client)  │        ││
│  │  └─────────────────┘    └─────────────────┘    └─────────────────┘        ││
│  │                                                                            ││
│  └────────────────────────────────────────────────────────────────────────────┘│
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
crates/spn-mcp/
├── Cargo.toml
└── src/
    ├── main.rs           # Entry point, CLI parsing
    ├── lib.rs            # Library exports
    │
    ├── config/           # YAML configuration
    │   ├── mod.rs
    │   ├── schema.rs     # ApiConfig, ToolDef, AuthConfig structs
    │   ├── loader.rs     # Load configs from ~/.spn/apis/
    │   └── validate.rs   # Config validation
    │
    ├── auth/             # Authentication handlers
    │   ├── mod.rs
    │   ├── basic.rs      # HTTP Basic Auth
    │   ├── bearer.rs     # Bearer token
    │   └── api_key.rs    # API key (header/query)
    │
    ├── runtime/          # Tool execution
    │   ├── mod.rs
    │   ├── registry.rs   # Dynamic tool registration
    │   ├── executor.rs   # HTTP request execution
    │   ├── rate_limit.rs # Per-API rate limiting
    │   └── template.rs   # Body/URL template rendering
    │
    └── server/           # MCP server
        ├── mod.rs
        ├── handler.rs    # ServerHandler implementation
        └── transport.rs  # stdio transport setup
```

## YAML Configuration Schema

```yaml
# ~/.spn/apis/dataforseo.yaml
name: dataforseo
version: "1.0"
base_url: https://api.dataforseo.com/v3
description: "DataForSEO API v3 - SEO data and keyword research"

# Authentication
auth:
  type: basic                    # basic | bearer | api_key
  credential: dataforseo         # → spn provider get dataforseo
  # For api_key type:
  # location: header | query
  # key_name: X-API-Key

# Rate limiting
rate_limit:
  requests_per_minute: 12        # DataForSEO limit: 12/min
  burst: 3                       # Allow small bursts

# Default headers
headers:
  Content-Type: application/json
  Accept: application/json

# Tools definition
tools:
  - name: keyword_ideas
    description: "Get keyword ideas based on seed keywords"
    method: POST
    path: /dataforseo_labs/google/keyword_ideas/live

    # Request body template (Tera syntax)
    body_template: |
      [{
        "keywords": {{ keywords | json }},
        "location_code": {{ location_code | default(value=2840) }},
        "language_code": "{{ language_code | default(value='en') }}",
        "limit": {{ limit | default(value=100) }}
      }]

    # Parameters exposed as MCP tool inputs
    params:
      - name: keywords
        type: array
        items: string
        required: true
        description: "Seed keywords to generate ideas from"

      - name: location_code
        type: integer
        required: false
        default: 2840
        description: "Google Ads location code (2840 = USA)"

      - name: language_code
        type: string
        required: false
        default: "en"
        description: "Language code (en, fr, de, etc.)"

      - name: limit
        type: integer
        required: false
        default: 100
        description: "Maximum results to return"

    # Response handling
    response:
      # Extract specific field from response
      extract: "tasks[0].result"
      # Or transform with template
      # transform: |
      #   {% for item in result %}...{% endfor %}

  - name: search_volume
    description: "Get search volume for keywords"
    method: POST
    path: /keywords_data/google_ads/search_volume/live
    body_template: |
      [{
        "keywords": {{ keywords | json }},
        "location_code": {{ location_code | default(value=2840) }}
      }]
    params:
      - name: keywords
        type: array
        items: string
        required: true
      - name: location_code
        type: integer
        required: false
        default: 2840
```

## CLI Commands

```bash
# Create new API wrapper (interactive wizard)
spn mcp wrap dataforseo
#  → Prompts for: base URL, auth type, credential name
#  → Creates: ~/.spn/apis/dataforseo.yaml
#  → Option to add tools interactively

# Add tool to existing wrapper
spn mcp wrap dataforseo --add-tool
#  → Prompts for: name, method, path, params
#  → Appends to: ~/.spn/apis/dataforseo.yaml

# Import from OpenAPI spec
spn mcp wrap dataforseo --from-openapi ./openapi.json
#  → Parses OpenAPI spec
#  → Generates YAML config with all endpoints
#  → Uses rmcp-openapi patterns

# List configured APIs
spn mcp list
#  dataforseo    5 tools    ~/.spn/apis/dataforseo.yaml
#  ahrefs        3 tools    ~/.spn/apis/ahrefs.yaml

# Start MCP server (for manual testing)
spn mcp serve
#  → Loads all configs from ~/.spn/apis/
#  → Starts stdio MCP server
#  → Registers all tools dynamically

# Start server for specific API only
spn mcp serve dataforseo
#  → Only loads dataforseo.yaml
#  → Tools: dataforseo_keyword_ideas, dataforseo_search_volume

# Test a tool
spn mcp test dataforseo keyword_ideas --keywords '["qr code"]'
#  → Executes tool and shows response
#  → Useful for debugging

# Validate config
spn mcp validate dataforseo
#  → Checks YAML syntax
#  → Validates auth credentials exist
#  → Tests connection to API
```

## Dynamic Tool Registration

Based on rmcp research, we use dynamic registration without compile-time macros:

```rust
use rmcp::{ServerHandler, ToolRoute, RpcParams, CallToolResult};
use std::collections::HashMap;

pub struct DynamicHandler {
    tools: HashMap<String, ToolDef>,
    executor: HttpExecutor,
}

#[async_trait]
impl ServerHandler for DynamicHandler {
    // Dynamic tool listing
    async fn list_tools(&self) -> Vec<Tool> {
        self.tools.values()
            .map(|def| Tool {
                name: format!("{}_{}", self.api_name, def.name),
                description: def.description.clone(),
                input_schema: def.to_json_schema(),
            })
            .collect()
    }

    // Dynamic tool invocation
    async fn call_tool(
        &self,
        name: &str,
        params: RpcParams
    ) -> CallToolResult {
        let tool_def = self.tools.get(name)
            .ok_or_else(|| format!("Unknown tool: {}", name))?;

        // Render body template with params
        let body = self.render_template(&tool_def.body_template, &params)?;

        // Execute HTTP request
        let response = self.executor.execute(
            &tool_def.method,
            &tool_def.path,
            body,
        ).await?;

        // Extract/transform response
        let result = self.process_response(response, &tool_def.response)?;

        Ok(CallToolResult::success(result))
    }
}
```

## Implementation Phases

### Phase 1: MVP (~8h)

Core functionality to get DataForSEO working:

1. **Crate scaffold**
   - `crates/spn-mcp/` with Cargo.toml
   - Dependencies: rmcp 0.16, tokio, reqwest, serde_yaml, tera

2. **Config parsing**
   - `config/schema.rs`: ApiConfig, ToolDef, AuthConfig
   - `config/loader.rs`: Load from `~/.spn/apis/`

3. **Auth via spn-client**
   - `auth/basic.rs`: HTTP Basic from credential
   - `auth/bearer.rs`: Bearer token
   - Integration with `spn-client::resolve_api_key()`

4. **HTTP executor**
   - `runtime/executor.rs`: reqwest client
   - `runtime/template.rs`: Tera template rendering

5. **MCP server**
   - `server/handler.rs`: DynamicHandler impl
   - Dynamic tool registration

6. **CLI command**
   - `spn mcp serve` command

### Phase 2: UX (~4h)

1. **Interactive wizard**
   - `spn mcp wrap <name>` with dialoguer prompts
   - Tool definition wizard

2. **Testing**
   - `spn mcp test <api> <tool>`
   - `spn mcp validate <api>`

3. **Listing**
   - `spn mcp list`

### Phase 3: Advanced (~4h)

1. **OpenAPI import**
   - `--from-openapi` flag
   - Parse OpenAPI spec
   - Generate YAML config

2. **Rate limiting**
   - `runtime/rate_limit.rs`: Token bucket
   - Per-API limits from config

3. **Response transforms**
   - Tera templates for response processing
   - JSON path extraction

### Phase 4: Integration (~2h)

1. **Claude Code config**
   - Auto-register in `claude_desktop_config.json`

2. **Nika integration**
   - Test with `invoke: dataforseo_keyword_ideas`
   - Document workflow examples

## Research Findings

### rmcp-openapi

Discovered existing crate that converts OpenAPI specs to MCP tools:

```rust
// From rmcp-openapi - dynamic tool building
let spec: OpenApiSpec = serde_json::from_str(&openapi_json)?;
let tools = spec.paths.iter()
    .map(|(path, item)| {
        ToolRoute::new_dyn(
            &item.operation_id,
            &item.description,
            item.to_json_schema(),
            move |params| async move {
                // Execute HTTP request
            }
        )
    })
    .collect();
```

This pattern informs our approach but we use YAML configs instead of requiring OpenAPI specs.

### Key rmcp Patterns

1. **ToolRoute::new_dyn()** - Dynamic tool without macros
2. **Manual list_tools()** - Return tools from HashMap
3. **schemars** - Generate JSON Schema for params
4. **async_trait** - Async handler methods

## Configuration Location

```
~/.spn/
├── config.toml           # spn config
├── daemon.sock           # IPC socket
├── apis/                 # NEW: API wrapper configs
│   ├── dataforseo.yaml
│   ├── ahrefs.yaml
│   └── semrush.yaml
└── packages/             # Installed packages
```

## Tool Naming Convention

Tools are named `{api_name}_{tool_name}`:

- `dataforseo_keyword_ideas`
- `dataforseo_search_volume`
- `ahrefs_backlinks`
- `semrush_domain_overview`

This prevents collisions and makes tool origin clear.

## Security Considerations

1. **Credentials never in YAML** - Only credential names, resolved via spn-client
2. **Keychain storage** - Actual secrets in OS keychain via daemon
3. **Socket permissions** - daemon.sock is 0600
4. **Rate limiting** - Prevents accidental API abuse

## Success Criteria

- [ ] `spn mcp serve` starts and responds to MCP protocol
- [ ] DataForSEO keyword_ideas tool works from Claude Code
- [ ] Credentials resolved via spn daemon
- [ ] Nika workflow can `invoke: dataforseo_keyword_ideas`
- [ ] `spn mcp wrap` creates valid config interactively

## Related Work

- **novanet-mcp**: Static MCP server with compile-time tools (reference implementation)
- **rmcp-openapi**: OpenAPI → MCP conversion (pattern reference)
- **spn-client**: IPC library for daemon communication

## References

- [rmcp crate docs](https://docs.rs/rmcp/0.16)
- [DataForSEO API v3 research](../../../novanet/docs/research/dataforseo-api-v3-research.md)
- [MCP specification](https://modelcontextprotocol.io/docs)
