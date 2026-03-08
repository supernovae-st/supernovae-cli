# Plan: `spn mcp wrap` Interactive Wizard

**Created**: 2026-03-08
**Status**: Interactive wizard complete, OpenAPI parser pending
**Effort**: ~4 hours (~2h done)
**Target**: v0.16.0

---

## Problem

The `spn mcp wrap` command is designed but NOT implemented. Currently users must manually create YAML files in `~/.spn/apis/` to wrap REST APIs as MCP tools.

### Current State

```bash
spn mcp apis list      # ✅ Works - lists YAML configs
spn mcp apis validate  # ✅ Works - validates YAML
spn mcp apis info      # ✅ Works - shows API info

spn mcp wrap           # ✅ IMPLEMENTED (interactive wizard)
spn mcp wrap --from-openapi  # ⏳ PENDING (shows placeholder message)
```

### Gap Analysis Reference

From `docs/plans/2026-03-08-spn-mcp-gap-analysis.md`:
- Gap 1: Interactive wizard not implemented
- Gap 2: OpenAPI import not implemented

---

## Target UX

### Basic Interactive Flow

```bash
$ spn mcp wrap

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🛠️  MCP WRAPPER WIZARD                                                        ║
╚═══════════════════════════════════════════════════════════════════════════════╝

? API name: github-api
? Base URL: https://api.github.com
? Authentication:
  ❯ Bearer Token
    API Key (header)
    API Key (query)
    Basic Auth
    None

? Auth secret name [github]: github

╭─────────────────────────────────────────────────────────────────────────────────╮
│  Adding endpoints...                                                            │
╰─────────────────────────────────────────────────────────────────────────────────╯

? Endpoint 1 - Method: GET
? Endpoint 1 - Path: /repos/{owner}/{repo}
? Endpoint 1 - Tool name: github_get_repo
? Endpoint 1 - Description: Get repository details

? Add another endpoint? (Y/n): y

? Endpoint 2 - Method: POST
? Endpoint 2 - Path: /repos/{owner}/{repo}/issues
? Endpoint 2 - Tool name: github_create_issue
? Endpoint 2 - Description: Create a new issue

? Add another endpoint? (Y/n): n

✅ Created ~/.spn/apis/github-api.yaml (2 tools)
✅ Validated configuration

? Start server now? (Y/n): y
Starting github-api MCP server...
```

### OpenAPI Import Flow

```bash
$ spn mcp wrap --from-openapi ./openapi.yaml

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🛠️  MCP WRAPPER — OpenAPI Import                                              ║
╚═══════════════════════════════════════════════════════════════════════════════╝

Parsing OpenAPI spec...
Found: Acme API v2.1.0
Base URL: https://api.acme.com/v2

Discovered 47 endpoints:
  • GET /users → acme_list_users
  • POST /users → acme_create_user
  • GET /users/{id} → acme_get_user
  ... (44 more)

? Import all endpoints?
  ❯ All (47 endpoints)
    Select interactively
    Filter by tag

? Authentication method detected: OAuth2. Use it? (Y/n): y
? Auth secret name [acme]: acme-api

✅ Created ~/.spn/apis/acme-api.yaml (47 tools)
✅ Validated configuration
```

---

## Implementation

### File Structure

```
crates/spn/src/commands/
├── mcp.rs                   # Existing - add wrap subcommand
└── mcp/
    └── wrap.rs              # NEW - wizard implementation

crates/spn-mcp/src/
├── openapi/                 # NEW - OpenAPI parser
│   ├── mod.rs
│   └── parser.rs
└── wizard/                  # NEW - interactive wizard
    ├── mod.rs
    └── prompts.rs
```

### Step 1: Add CLI Subcommand

**File:** `crates/spn/src/commands/mcp.rs`

```rust
#[derive(Subcommand)]
pub enum McpCommand {
    // ... existing commands ...

    /// Wrap a REST API as MCP tools (interactive wizard)
    Wrap {
        /// Import from OpenAPI spec file
        #[arg(long)]
        from_openapi: Option<PathBuf>,

        /// API name (skip name prompt)
        #[arg(long)]
        name: Option<String>,

        /// Base URL (skip URL prompt)
        #[arg(long)]
        base_url: Option<String>,

        /// Non-interactive mode (requires --from-openapi)
        #[arg(long, short = 'y')]
        yes: bool,
    },
}
```

### Step 2: Interactive Wizard Module

**File:** `crates/spn/src/commands/mcp/wrap.rs`

```rust
use dialoguer::{Input, Select, Confirm, MultiSelect};
use spn_mcp::config::ApiConfig;

pub async fn run_wizard(
    from_openapi: Option<PathBuf>,
    name: Option<String>,
    base_url: Option<String>,
    yes: bool,
) -> Result<(), SpnError> {
    if let Some(spec_path) = from_openapi {
        run_openapi_import(spec_path, name, yes).await
    } else {
        run_interactive_wizard(name, base_url).await
    }
}

async fn run_interactive_wizard(
    name: Option<String>,
    base_url: Option<String>,
) -> Result<(), SpnError> {
    print_banner("MCP WRAPPER WIZARD");

    // Prompt for API name
    let name = name.unwrap_or_else(|| {
        Input::new()
            .with_prompt("API name")
            .interact_text()
            .unwrap()
    });

    // Prompt for base URL
    let base_url = base_url.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Base URL")
            .interact_text()
            .unwrap()
    });

    // Prompt for auth type
    let auth_types = vec![
        "Bearer Token",
        "API Key (header)",
        "API Key (query)",
        "Basic Auth",
        "None",
    ];
    let auth_selection = Select::new()
        .with_prompt("Authentication")
        .items(&auth_types)
        .default(0)
        .interact()
        .unwrap();

    // Build config
    let mut config = ApiConfig::new(&name, &base_url);
    config.auth = match auth_selection {
        0 => Some(AuthConfig::bearer("${spn:name}")),
        1 => Some(AuthConfig::api_key_header("${spn:name}")),
        2 => Some(AuthConfig::api_key_query("${spn:name}")),
        3 => Some(AuthConfig::basic("${spn:name}")),
        _ => None,
    };

    // Endpoint loop
    loop {
        let endpoint = prompt_endpoint()?;
        config.endpoints.push(endpoint);

        if !Confirm::new()
            .with_prompt("Add another endpoint?")
            .default(true)
            .interact()
            .unwrap()
        {
            break;
        }
    }

    // Save config
    let path = spn_paths::apis_dir().join(format!("{}.yaml", name));
    config.save(&path)?;

    println!("✅ Created {} ({} tools)", path.display(), config.endpoints.len());

    Ok(())
}
```

### Step 3: OpenAPI Parser

**File:** `crates/spn-mcp/src/openapi/parser.rs`

```rust
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub servers: Option<Vec<OpenApiServer>>,
    pub paths: std::collections::HashMap<String, PathItem>,
}

#[derive(Debug, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
}

pub fn parse_openapi(path: &Path) -> Result<OpenApiSpec, Error> {
    let content = std::fs::read_to_string(path)?;

    // Support both YAML and JSON
    if path.extension().map_or(false, |e| e == "json") {
        serde_json::from_str(&content).map_err(Into::into)
    } else {
        serde_yaml::from_str(&content).map_err(Into::into)
    }
}

pub fn convert_to_api_config(spec: OpenApiSpec) -> ApiConfig {
    let base_url = spec.servers
        .and_then(|s| s.first().map(|s| s.url.clone()))
        .unwrap_or_default();

    let mut config = ApiConfig::new(&spec.info.title, &base_url);

    for (path, item) in spec.paths {
        for (method, operation) in item.operations() {
            let tool_name = operation.operation_id
                .clone()
                .unwrap_or_else(|| generate_tool_name(&method, &path));

            config.endpoints.push(Endpoint {
                name: tool_name,
                method: method.to_uppercase(),
                path: path.clone(),
                description: operation.summary.clone(),
                parameters: convert_parameters(&operation.parameters),
            });
        }
    }

    config
}
```

### Step 4: Dependencies

**File:** `crates/spn/Cargo.toml`

```toml
[dependencies]
dialoguer = "0.11"  # Interactive prompts
```

**File:** `crates/spn-mcp/Cargo.toml`

```toml
[dependencies]
# Already has serde_yaml
```

### Step 5: Tests

**File:** `crates/spn-mcp/src/openapi/tests.rs`

```rust
#[test]
fn test_parse_openapi_yaml() {
    let spec = parse_openapi(Path::new("tests/fixtures/petstore.yaml")).unwrap();
    assert_eq!(spec.info.title, "Petstore API");
    assert!(!spec.paths.is_empty());
}

#[test]
fn test_convert_to_api_config() {
    let spec = parse_openapi(Path::new("tests/fixtures/petstore.yaml")).unwrap();
    let config = convert_to_api_config(spec);
    assert!(!config.endpoints.is_empty());
    assert!(config.endpoints.iter().any(|e| e.name.contains("pet")));
}
```

---

## Verification Checklist

- [x] `spn mcp wrap` launches interactive wizard
- [x] All prompts work (name, URL, auth, endpoints)
- [x] YAML file is created in correct location
- [x] YAML file passes validation
- [ ] `spn mcp wrap --from-openapi` parses OpenAPI 3.0+ (PENDING)
- [ ] Swagger 2.0 files are rejected with clear error (PENDING)
- [x] `--yes` flag skips confirmations
- [x] Tool names are sanitized (no special chars)

---

## Commit Strategy

```bash
# Commit 1: Add OpenAPI parser
git commit -m "feat(spn-mcp): add OpenAPI 3.0 parser

- Parse YAML and JSON OpenAPI specs
- Convert paths to Endpoint configs
- Extract auth schemes from security definitions

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

# Commit 2: Add interactive wizard
git commit -m "feat(cli): add spn mcp wrap interactive wizard

- Interactive prompts for API name, URL, auth
- Endpoint addition loop
- YAML config generation and validation

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"

# Commit 3: Add --from-openapi flag
git commit -m "feat(cli): add --from-openapi to spn mcp wrap

- Import endpoints from OpenAPI spec
- Interactive endpoint selection
- Tag-based filtering

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```
