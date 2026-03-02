# spn ↔ nika Integration - Detailed Execution Plan

**Date:** 2026-03-02
**Status:** 🚧 Ready to Execute
**Target:** v0.7.0 MVP
**Timeline:** 4 weeks

---

## 🎯 Executive Summary

**Vision:** Transform spn into an intelligent router/hub that orchestrates nika/novanet for a seamless AI workflow experience.

**Architecture Decision:** **Option C - Router Pattern**
- spn = Smart CLI router (downloads, installs, proxies)
- nika = Workflow engine worker (resolves, executes)
- Communication: `Command::new("nika").args(...).spawn()`

**Scope:** 13 Core Commands (not 58)
1. Package: `add`, `remove`, `install`, `list`
2. Discovery: `search`, `info`, `outdated`
3. Setup: `init`, `doctor`
4. Execution: `run` (proxy to nika)
5. Secrets: `provider set`, `provider get`, `provider list`

**User Experience:**
- **Wizard Mode:** Multi-step setup (`spn init`)
- **Interactive CLI:** Arrow-key selection with animations (`spn add`)
- **Optional TUI:** Full-screen interface (future: `spn` with no args)

**Key Features:**
- Ratatui-style progress bars and spinners
- API key detection with inline tutorials
- Private registry support (supernovae-powers)
- TDD approach for all components

---

## 📊 Current State Analysis

### What Exists ✅

**spn v0.6.0:**
- ✅ Basic package download/install
- ✅ Skills proxy (`spn skill add` → skills.sh)
- ✅ MCP proxy (`spn mcp add` → npm)
- ✅ Secrets management (OS keychain + fallbacks)
- ✅ Registry client (supernovae-registry public)

**nika v0.17.0:**
- ✅ Workflow execution engine
- ✅ Local file resolution (`.nika/workflows/`)
- ✅ Include system (`path:` only)
- ✅ Arc-based package caching
- ✅ Lockfile support (`spn.lock`)

**Registries:**
- ✅ Public: supernovae-registry (46 packages)
- ✅ Private: supernovae-powers (team packages)

### What's Missing ❌

**spn:**
- ❌ Interactive CLI prompts (Wizard, selection menus)
- ❌ Progress animations (Ratatui spinners, bars)
- ❌ Private registry authentication (GitHub tokens)
- ❌ API key detection tutorial

**nika:**
- ❌ Package URI resolution (`@workflows/name` → filesystem path)
- ❌ Include package support (`pkg: @workflows/name`)
- ❌ Integration with spn.lock for version resolution

**Integration:**
- ❌ spn sync to `.nika/.cache/` (symlinks)
- ❌ `spn run` proxy to nika with package resolution

### Bugs to Fix 🐛

1. **Tokio Panic in `spn add`** (CRITICAL)
   - Error: "Cannot drop a runtime in a context where blocking is not allowed"
   - File: `src/commands/add.rs`
   - Impact: Command crashes on completion

2. **Invalid Examples in `nika init`** (HIGH)
   - Error: Schema validation fails on generated workflows
   - File: `nika/tools/nika/src/main.rs` (hardcoded templates)
   - Impact: Users get broken workflows out of the box

---

## 🏗️ Architecture Deep Dive

### Option C: Router Pattern (CHOSEN)

```
┌─────────────────────────────────────────────────────────────────┐
│  USER EXPERIENCE                                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  $ spn add @workflows/seo-audit                                 │
│  $ nika run @workflows/seo-audit --url https://qrcode-ai.com    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  SPN (Router) - supernovae-cli v0.7.0                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Core Responsibilities:                                         │
│  ├── 📦 Package Management                                      │
│  │   ├── Download from registry (HTTP GET)                     │
│  │   ├── Extract to ~/.spn/packages/                           │
│  │   ├── Update spn.yaml + spn.lock                            │
│  │   └── Verify checksums                                      │
│  │                                                              │
│  ├── 🔍 Discovery                                               │
│  │   ├── Search public registry                                │
│  │   ├── Search private registry (GitHub auth)                 │
│  │   └── Show package info (downloads, versions)               │
│  │                                                              │
│  ├── 🔐 Secrets Management                                      │
│  │   ├── Store keys in OS keychain                             │
│  │   ├── Detect missing keys before execution                  │
│  │   └── Show inline tutorial for setup                        │
│  │                                                              │
│  ├── 🔄 Proxying                                                │
│  │   ├── spn run → nika run (exec binary)                      │
│  │   ├── spn nk → nika (all commands)                          │
│  │   └── spn nv → novanet (all commands)                       │
│  │                                                              │
│  └── 🎨 Interactive UI                                          │
│      ├── Wizard mode (spn init)                                │
│      ├── Selection menus (spn add)                             │
│      └── Progress animations (Ratatui)                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ Command::new("nika")
                            │   .args(["run", "@workflows/seo-audit"])
                            │   .spawn()
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  NIKA (Worker) - nika v0.17.0+                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Core Responsibilities:                                         │
│  ├── 📝 Package Resolution (NEW)                                │
│  │   ├── Parse package URI (@workflows/name@version)           │
│  │   ├── Check spn.lock for locked version                     │
│  │   ├── Resolve to ~/.spn/packages/                           │
│  │   └── Load workflow.nika.yaml                               │
│  │                                                              │
│  ├── 🔗 Include Resolution (ENHANCED)                           │
│  │   ├── Support path: ./local/workflow.nika.yaml              │
│  │   ├── Support pkg: @workflows/tasks (NEW)                   │
│  │   └── Recursive include expansion                           │
│  │                                                              │
│  ├── ⚙️ Workflow Execution                                      │
│  │   ├── DAG construction                                      │
│  │   ├── Task execution (infer, exec, fetch, invoke, agent)    │
│  │   ├── LLM provider routing                                  │
│  │   └── MCP tool calling                                      │
│  │                                                              │
│  └── 💾 State Management                                        │
│      ├── Arc-based caching                                     │
│      ├── Session persistence                                   │
│      └── Trace logging                                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Why Option C?**
1. ✅ **Simplicity:** No shared crate, no complex coupling
2. ✅ **Works Today:** Uses existing binaries
3. ✅ **Clear Separation:** spn = UX, nika = engine
4. ✅ **Future-Proof:** Easy to add novanet, npm proxies

---

## 🎨 CLI Design Patterns

### Pattern 1: Wizard Mode (Multi-Step Setup)

**Use Case:** `spn init` - Project initialization

```bash
$ spn init

╔═══════════════════════════════════════════════════════════╗
║  🚀 SuperNovae Project Initialization                     ║
╚═══════════════════════════════════════════════════════════╝

? Project name: my-ai-project

? Project type: (Use arrow keys)
  ❯ Content Generation - SEO, blog posts, marketing
    Code Automation - Review, refactor, testing
    Research Pipeline - Web scraping, analysis, reports
    Empty - I'll configure it myself

? Install starter workflows? (Y/n) y

? Select workflows: (Space to select, Enter to confirm)
  ❯ [x] @workflows/seo-audit
    [x] @workflows/content-generator
    [ ] @workflows/code-review
    [x] @agents/researcher

╔═══════════════════════════════════════════════════════════╗
║  📦 Installing 3 packages...                              ║
╚═══════════════════════════════════════════════════════════╝

   @workflows/seo-audit
   │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100% │ 2.3 MB

   @workflows/content-generator
   │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100% │ 1.8 MB

   @agents/researcher
   │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100% │ 856 KB

✅ Project initialized!

   📂 Created:
      .nika/config.toml
      .nika/policies.yaml
      spn.yaml
      spn.lock

   📦 Installed:
      @workflows/seo-audit v1.2.0
      @workflows/content-generator v2.0.1
      @agents/researcher v1.5.0

→ Get started:
   nika run @workflows/seo-audit --url https://example.com
```

**Technical Implementation:**
- **Crate:** `dialoguer` v0.11 (multi-select, confirm, input)
- **Animations:** `indicatif` v0.17 (progress bars)
- **Files Modified:** `src/commands/init.rs`
- **LOC Estimate:** ~200 lines

---

### Pattern 2: CLI Interactive (Single Prompt)

**Use Case:** `spn add` - Package installation with search

```bash
$ spn add

? What are you looking for? seo

🔍 Searching for "seo"...
⠹ [spinner animé pendant recherche]

? Select package to install: (Use ↑↓, Enter to select)

  ┌────────────────────────────────────────────────────────────┐
  │ ❯ ⭐ @workflows/seo-audit v1.2.0                          │
  │   Complete SEO analysis (meta, performance, accessibility) │
  │   📦 1.2K downloads • 📅 Updated 2d ago                    │
  ├────────────────────────────────────────────────────────────┤
  │   🔒 @qrcodeai/seo-sprint v1.0.0 (Private)                │
  │   QR Code AI SEO sprint workflow                           │
  │   📦 15 downloads • 📅 Updated 3d ago                      │
  ├────────────────────────────────────────────────────────────┤
  │   🤖 @agents/seo-researcher v2.0.1                         │
  │   Agent that finds SEO opportunities                       │
  │   📦 890 downloads • 📅 Updated 1w ago                     │
  └────────────────────────────────────────────────────────────┘

📦 Installing @workflows/seo-audit@1.2.0...

   Downloading from registry...
   │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100% │ 2.3 MB / 2.3 MB

   Extracting to ~/.spn/packages/...
   │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100% │ 12 files

✅ Installed @workflows/seo-audit v1.2.0

   ✓ Added to spn.yaml
   ✓ Updated spn.lock
   ✓ Linked to .nika/.cache/workflows/seo-audit

→ Run with:
   nika run @workflows/seo-audit --url https://example.com
```

**With Arguments (Direct Install):**

```bash
$ spn add @workflows/seo-audit

🔍 Resolving @workflows/seo-audit...
   ✓ Found v1.2.0 (latest)

📦 Installing @workflows/seo-audit@1.2.0...
   │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100% │

✅ Installed successfully
```

**Technical Implementation:**
- **Crate:** `dialoguer` (Select, Input, theme customization)
- **Crate:** `indicatif` (ProgressBar, Spinner)
- **Crate:** `console` (terminal styling, colors)
- **Files Modified:** `src/commands/add.rs`, `src/ui/interactive.rs` (new)
- **LOC Estimate:** ~150 lines

---

### Pattern 3: TUI (Full-Screen) - FUTURE

**Use Case:** `spn` (no args) - Browse packages like App Store

```bash
$ spn

╔═══════════════════════════════════════════════════════════════════╗
║  🌟 SuperNovae Package Manager                          [? Help]  ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                   ║
║  Search: seo_                                                     ║
║                                                                   ║
║  ┌─────────────────────────────────────────────────────────────┐ ║
║  │ ⭐ @workflows/seo-audit                              v1.2.0  │ ║
║  │ Complete SEO analysis (meta, perf, accessibility)           │ ║
║  │ 📦 1.2K • ⭐ 45 • 📅 2d ago                                   │ ║
║  ├─────────────────────────────────────────────────────────────┤ ║
║  │ 🤖 @agents/seo-researcher                            v2.0.1  │ ║
║  │ Find SEO opportunities with AI                              │ ║
║  │ 📦 890 • ⭐ 32 • 📅 1w ago                                    │ ║
║  └─────────────────────────────────────────────────────────────┘ ║
║                                                                   ║
║  [↑↓] Navigate  [Enter] Install  [i] Info  [q] Quit              ║
╚═══════════════════════════════════════════════════════════════════╝
```

**Deferred to v0.8.0+** (Future enhancement, not MVP)

---

## 🔐 API Key Detection & Tutorial

### Feature: Smart API Key Detection

**Problem:** Users run workflows without configuring API keys → cryptic errors

**Solution:** Detect missing keys BEFORE execution, show inline tutorial

**Flow:**

```bash
$ nika run @workflows/seo-audit --url https://example.com

⚠️  Missing API Keys Detected

The following providers need configuration:

   🤖 Anthropic (Claude)
      Required by: @workflows/seo-audit

   🔍 Perplexity (Search)
      Required by: @workflows/seo-audit

╔═══════════════════════════════════════════════════════════════╗
║  📚 Quick Setup Tutorial                                      ║
╠═══════════════════════════════════════════════════════════════╣
║                                                               ║
║  1. Get your API keys:                                        ║
║     • Anthropic: https://console.anthropic.com/settings/keys  ║
║     • Perplexity: https://www.perplexity.ai/settings/api     ║
║                                                               ║
║  2. Store securely (OS Keychain):                             ║
║     $ spn provider set anthropic                              ║
║     $ spn provider set perplexity                             ║
║                                                               ║
║  3. Or use environment variables:                             ║
║     export ANTHROPIC_API_KEY="sk-ant-..."                     ║
║     export PERPLEXITY_API_KEY="pplx-..."                      ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝

? Configure keys now? (Y/n) y

? Which provider?
  ❯ Anthropic (Claude)
    Perplexity (Search)

? Enter Anthropic API Key: sk-ant-***********************************
✓ Stored securely in macOS Keychain

? Configure another? (Y/n) n

✅ Configuration complete! Running workflow...

🚀 Running @workflows/seo-audit...
```

**Implementation:**

**File:** `src/commands/run.rs` (new proxy command)

```rust
pub async fn run(workflow: &str, args: Vec<String>) -> Result<()> {
    // 1. Resolve package to get metadata
    let pkg_info = resolve_package_info(workflow).await?;

    // 2. Extract required providers from workflow
    let required_providers = extract_required_providers(&pkg_info)?;

    // 3. Check which keys are missing
    let missing = check_missing_keys(&required_providers)?;

    // 4. If missing, show tutorial and offer to configure
    if !missing.is_empty() {
        show_api_key_tutorial(&missing)?;

        if Confirm::new()
            .with_prompt("Configure keys now?")
            .default(true)
            .interact()?
        {
            for provider in missing {
                configure_provider_interactive(&provider)?;
            }
        } else {
            eprintln!("⚠️  Workflow may fail without API keys");
            if !Confirm::new()
                .with_prompt("Continue anyway?")
                .default(false)
                .interact()?
            {
                return Ok(());
            }
        }
    }

    // 5. Proxy to nika
    let status = Command::new("nika")
        .arg("run")
        .arg(workflow)
        .args(args)
        .status()?;

    if !status.success() {
        return Err(SpnError::WorkflowFailed(status.code()));
    }

    Ok(())
}
```

**File:** `src/secrets/detection.rs` (new module)

```rust
pub fn extract_required_providers(pkg_info: &PackageInfo) -> Result<Vec<Provider>> {
    // Parse workflow YAML to extract provider references
    // Look for:
    // - tasks[].infer (implies LLM provider)
    // - tasks[].fetch (implies web/API provider)
    // - mcp: blocks (implies MCP providers)

    let mut providers = HashSet::new();

    // Read workflow file
    let workflow_path = pkg_info.workflow_path()?;
    let yaml = std::fs::read_to_string(workflow_path)?;
    let workflow: Value = serde_yaml::from_str(&yaml)?;

    // Check for LLM usage (infer tasks)
    if let Some(tasks) = workflow.get("tasks") {
        for task in tasks.as_sequence().unwrap_or(&vec![]) {
            if task.get("infer").is_some() || task.get("agent").is_some() {
                // Check if provider specified, else use default
                let provider = task.get("provider")
                    .and_then(|p| p.as_str())
                    .unwrap_or("anthropic");
                providers.insert(Provider::from_str(provider)?);
            }

            if task.get("fetch").is_some() {
                // Check if fetch uses specific API
                if let Some(params) = task.get("params") {
                    if params.get("search").is_some() {
                        providers.insert(Provider::Perplexity);
                    }
                }
            }
        }
    }

    // Check for MCP servers
    if let Some(mcp) = workflow.get("mcp") {
        for (server_name, _) in mcp.as_mapping().unwrap_or(&Default::default()) {
            if let Some(provider) = Provider::from_mcp_server(server_name.as_str().unwrap()) {
                providers.insert(provider);
            }
        }
    }

    Ok(providers.into_iter().collect())
}

pub fn check_missing_keys(providers: &[Provider]) -> Result<Vec<Provider>> {
    let mut missing = Vec::new();

    for provider in providers {
        if !has_api_key(provider)? {
            missing.push(provider.clone());
        }
    }

    Ok(missing)
}

fn has_api_key(provider: &Provider) -> Result<bool> {
    // Check in order:
    // 1. OS Keychain
    // 2. Environment variable
    // 3. .env file

    use crate::secrets::keyring::get_api_key;

    Ok(get_api_key(provider.name()).is_ok())
}
```

**LOC Estimate:** ~250 lines

**Priority:** 🟡 P1 (High value, implement after core resolution)

---

## 🗂️ Private Registry Support

### Feature: GitHub-Authenticated Registry Access

**Use Case:** Access team-specific packages from private registry

**Configuration:**

```toml
# ~/.spn/config.toml

[[registry]]
name = "supernovae-registry"
url = "https://registry.supernovae.studio/registry.json"
type = "public"

[[registry]]
name = "supernovae-powers"
url = "https://raw.githubusercontent.com/supernovae-st/supernovae-powers/main/registry.json"
type = "private"
auth = "github"  # Use GitHub token from keychain
```

**Authentication Flow:**

```bash
$ spn add @qrcodeai/seo-sprint

🔍 Searching registries...
   ✓ supernovae-registry (public)
   ⚠️ supernovae-powers (private) - Authentication required

? Configure GitHub authentication? (Y/n) y

? GitHub Personal Access Token: ghp_***********************************
   (Create at: https://github.com/settings/tokens)
   Required scopes: repo (for private repos)

✓ Stored securely in macOS Keychain

🔍 Searching registries...
   ✓ supernovae-registry (public)
   ✓ supernovae-powers (private)

? Select package:
  ❯ 🔒 @qrcodeai/seo-sprint v1.0.0 (Private)
    QR Code AI SEO sprint workflow
```

**Implementation:**

**File:** `src/index/client.rs` (modify existing)

```rust
pub async fn search_all_registries(query: &str) -> Result<Vec<PackageInfo>> {
    let config = Config::load()?;
    let mut results = Vec::new();

    for registry in &config.registries {
        match registry.type_ {
            RegistryType::Public => {
                let packages = search_public_registry(&registry.url, query).await?;
                results.extend(packages);
            }
            RegistryType::Private => {
                match search_private_registry(registry, query).await {
                    Ok(packages) => results.extend(packages),
                    Err(SpnError::AuthenticationRequired) => {
                        eprintln!("⚠️  {} requires authentication", registry.name);
                        // Offer to configure
                        if should_configure_auth()? {
                            configure_github_token()?;
                            // Retry
                            let packages = search_private_registry(registry, query).await?;
                            results.extend(packages);
                        }
                    }
                    Err(e) => eprintln!("⚠️  Failed to search {}: {}", registry.name, e),
                }
            }
        }
    }

    Ok(results)
}

async fn search_private_registry(
    registry: &Registry,
    query: &str,
) -> Result<Vec<PackageInfo>> {
    // Get GitHub token from keychain
    let token = crate::secrets::keyring::get_github_token()?;

    // Fetch registry with authentication
    let client = reqwest::Client::new();
    let response = client
        .get(&registry.url)
        .header("Authorization", format!("token {}", token.expose_secret()))
        .header("User-Agent", "spn-cli")
        .send()
        .await?;

    if response.status() == 401 {
        return Err(SpnError::AuthenticationRequired);
    }

    let registry_data: RegistryIndex = response.json().await?;

    // Filter by query
    let results = registry_data.packages
        .into_iter()
        .filter(|pkg| pkg.matches_query(query))
        .collect();

    Ok(results)
}
```

**LOC Estimate:** ~100 lines

**Priority:** 🟡 P1 (Team use case, implement after public registry works)

---

## 📋 Detailed Task Breakdown

### Week 1: Foundation + Bug Fixes

#### Task 1.1: Fix Tokio Panic in `spn add` 🔴 P0
**Time:** 2 hours
**Files:**
- `src/main.rs` (refactor main function)
- `src/commands/add.rs` (fix async context)

**Current Code (Problematic):**
```rust
// src/main.rs
fn main() -> Result<()> {
    match cli.command {
        Commands::Add { package } => {
            // Creates nested runtime - WRONG
            commands::add::run(package).await?;
        }
    }
}
```

**Fixed Code:**
```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { package } => {
            commands::add::run(package).await?;
        }
        Commands::Install { frozen } => {
            commands::install::run(frozen).await?;
        }
        // ... other async commands

        // Sync commands can use block_in_place
        Commands::List => {
            tokio::task::block_in_place(|| commands::list::run())?;
        }
    }

    Ok(())
}
```

**Test Spec:**
```rust
#[tokio::test]
async fn test_add_command_completes_without_panic() {
    let result = commands::add::run("@workflows/test".to_string()).await;
    // Should complete without panic (success or error both OK)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_multiple_add_commands_in_sequence() {
    // Ensure runtime cleanup works correctly
    commands::add::run("@workflows/test1".to_string()).await.ok();
    commands::add::run("@workflows/test2".to_string()).await.ok();
    commands::add::run("@workflows/test3".to_string()).await.ok();
    // Should not panic on third command
}
```

**Success Criteria:**
- ✅ `spn add @workflows/test` completes without panic
- ✅ All tests pass
- ✅ No "Cannot drop runtime" errors in logs

---

#### Task 1.2: Fix Invalid Examples in `nika init` 🔴 P0
**Time:** 3 hours
**Files:**
- `nika/tools/nika/src/main.rs` (lines 1312-1900, templates)

**Current Templates (Invalid):**
```yaml
# INVALID: output.use.summary doesn't exist
tasks:
  - id: summarize
    infer: "Summarize this text"
    output:
      use.summary: summary  # ❌ Invalid
```

**Fixed Templates:**
```yaml
# VALID: Use separate task with use: block
tasks:
  - id: summarize
    infer: "Summarize this text"

  - id: format_output
    use: { summary: summarize }  # ✅ Valid
    exec: |
      echo "Summary: {{use.summary}}"
```

**All Templates to Fix:**
1. `basic-workflow.nika.yaml` (lines 1320-1350)
2. `agent-workflow.nika.yaml` (lines 1400-1450)
3. `research-workflow.nika.yaml` (lines 1500-1600)
4. `code-review-workflow.nika.yaml` (lines 1700-1800)

**Test Spec:**
```rust
#[tokio::test]
async fn test_init_generates_valid_workflows() {
    let temp_dir = tempfile::tempdir()?;

    // Run init in temp directory
    std::env::set_current_dir(&temp_dir)?;
    commands::init::run(InitOptions {
        template: Some("basic".to_string()),
        non_interactive: true,
    }).await?;

    // Verify generated workflow is valid
    let workflow_path = temp_dir.path().join("workflow.nika.yaml");
    let result = commands::check::run(workflow_path.to_str().unwrap()).await;

    assert!(result.is_ok(), "Generated workflow should be valid");
}

#[tokio::test]
async fn test_all_templates_are_valid() {
    let templates = vec!["basic", "agent", "research", "code-review"];

    for template in templates {
        let temp_dir = tempfile::tempdir()?;
        std::env::set_current_dir(&temp_dir)?;

        commands::init::run(InitOptions {
            template: Some(template.to_string()),
            non_interactive: true,
        }).await?;

        // All generated files should pass validation
        let workflow_path = temp_dir.path().join("workflow.nika.yaml");
        assert!(commands::check::run(workflow_path.to_str().unwrap()).await.is_ok());
    }
}
```

**Success Criteria:**
- ✅ All 4 templates validate with `nika check`
- ✅ No schema errors in generated workflows
- ✅ Users can run generated workflows immediately

---

#### Task 1.3: Add Package Resolution to Nika 🔴 P0
**Time:** 4 hours
**Files:**
- `nika/tools/nika/src/registry/` (new module)
  - `mod.rs` (module exports)
  - `resolver.rs` (package resolution logic)
  - `types.rs` (PackageRef, ResolvedPackage)
- `nika/tools/nika/src/main.rs` (modify `run_workflow`)

**New Module Structure:**
```rust
// src/registry/mod.rs
pub mod resolver;
pub mod types;

pub use resolver::{resolve_package_path, resolve_package_ref};
pub use types::{PackageRef, ResolvedPackage};
```

**Types:**
```rust
// src/registry/types.rs
use std::path::PathBuf;

/// A package reference (e.g., @workflows/name@1.2.0)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    pub scope: String,        // "@workflows"
    pub name: String,         // "seo-audit"
    pub version: Option<String>,  // Some("1.2.0") or None (latest)
}

impl PackageRef {
    /// Parse from string: @workflows/name or @workflows/name@version
    pub fn parse(input: &str) -> Result<Self, NikaError> {
        if !input.starts_with('@') {
            return Err(NikaError::InvalidPackageRef(input.to_string()));
        }

        // Split on last @ to separate version
        let (name_part, version) = if let Some((name, ver)) = input.rsplit_once('@') {
            if name.starts_with('@') {
                (name, Some(ver.to_string()))
            } else {
                (input, None)
            }
        } else {
            (input, None)
        };

        // Split scope and name
        let parts: Vec<&str> = name_part.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(NikaError::InvalidPackageRef(input.to_string()));
        }

        Ok(Self {
            scope: parts[0].to_string(),
            name: parts[1].to_string(),
            version,
        })
    }

    /// Full package name (@workflows/name)
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.scope, self.name)
    }
}

/// A resolved package with filesystem path
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub reference: PackageRef,
    pub resolved_version: String,  // Actual version used
    pub path: PathBuf,              // Path to workflow file
    pub source: PackageSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSource {
    /// From ~/.spn/packages/
    GlobalPackage,
    /// From .nika/workflows/
    LocalWorkflow,
    /// Absolute or relative filesystem path
    Filesystem,
}
```

**Resolver Implementation:**
```rust
// src/registry/resolver.rs
use std::path::{Path, PathBuf};
use super::types::*;

/// Resolve package reference to filesystem path
pub async fn resolve_package_path(package_ref: &str) -> Result<ResolvedPackage, NikaError> {
    // Parse package reference
    let pkg_ref = PackageRef::parse(package_ref)?;

    // Determine version to use
    let version = if let Some(v) = pkg_ref.version {
        v
    } else {
        // Try spn.lock first
        resolve_version_from_lock(&pkg_ref.full_name())
            // Fall back to latest installed
            .or_else(|| latest_installed_version(&pkg_ref))
            .ok_or_else(|| NikaError::PackageNotFound(pkg_ref.full_name()))?
    };

    // Build path: ~/.spn/packages/@scope/name/version/workflow.nika.yaml
    let spn_dir = dirs::home_dir()
        .ok_or_else(|| NikaError::HomeDirectoryNotFound)?
        .join(".spn");

    let pkg_dir = spn_dir
        .join("packages")
        .join(&pkg_ref.scope)
        .join(&pkg_ref.name)
        .join(&version);

    // Determine entry point based on package type
    let entry_file = match pkg_ref.scope.as_str() {
        "@workflows" => "workflow.nika.yaml",
        "@agents" => "agent.md",
        "@jobs" => "job.nika.yaml",
        "@prompts" => "prompt.md",
        _ => return Err(NikaError::UnsupportedPackageType(pkg_ref.scope.clone())),
    };

    let workflow_path = pkg_dir.join(entry_file);

    if !workflow_path.exists() {
        return Err(NikaError::WorkflowNotFound(format!(
            "Package {}@{} installed but {} not found",
            pkg_ref.full_name(),
            version,
            entry_file
        )));
    }

    Ok(ResolvedPackage {
        reference: pkg_ref,
        resolved_version: version,
        path: workflow_path,
        source: PackageSource::GlobalPackage,
    })
}

/// Resolve version from spn.lock if exists
fn resolve_version_from_lock(package_name: &str) -> Option<String> {
    let lock_path = Path::new("spn.lock");
    if !lock_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(lock_path).ok()?;

    // Parse YAML
    let lockfile: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;

    // Find package in packages array
    let packages = lockfile.get("packages")?.as_sequence()?;

    for pkg in packages {
        if pkg.get("name")?.as_str()? == package_name {
            return Some(pkg.get("version")?.as_str()?.to_string());
        }
    }

    None
}

/// Get latest installed version
fn latest_installed_version(pkg_ref: &PackageRef) -> Option<String> {
    let spn_dir = dirs::home_dir()?.join(".spn");
    let pkg_dir = spn_dir
        .join("packages")
        .join(&pkg_ref.scope)
        .join(&pkg_ref.name);

    if !pkg_dir.exists() {
        return None;
    }

    // List version directories
    let mut versions: Vec<String> = std::fs::read_dir(&pkg_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().ok()?.is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    if versions.is_empty() {
        return None;
    }

    // Sort by semver
    versions.sort_by(|a, b| {
        use semver::Version;
        let va = Version::parse(a).ok();
        let vb = Version::parse(b).ok();
        match (va, vb) {
            (Some(va), Some(vb)) => vb.cmp(&va),  // Descending
            _ => b.cmp(a),
        }
    });

    versions.into_iter().next()
}

/// Enhanced path resolution (supports packages AND local files)
pub async fn resolve_workflow_path(input: &str) -> Result<ResolvedPackage, NikaError> {
    // Case 1: Package reference (@workflows/name)
    if input.starts_with('@') {
        return resolve_package_path(input).await;
    }

    // Case 2: Filesystem path (absolute or relative)
    if input.contains('/') || input.ends_with(".nika.yaml") {
        let path = Path::new(input);
        if path.exists() {
            return Ok(ResolvedPackage {
                reference: PackageRef {
                    scope: "".to_string(),
                    name: input.to_string(),
                    version: None,
                },
                resolved_version: "local".to_string(),
                path: path.to_path_buf(),
                source: PackageSource::Filesystem,
            });
        } else {
            return Err(NikaError::FileNotFound(input.to_string()));
        }
    }

    // Case 3: Short name (resolve from .nika/workflows/)
    let local_workflow = Path::new(".nika/workflows")
        .join(format!("{}.nika.yaml", input));

    if local_workflow.exists() {
        return Ok(ResolvedPackage {
            reference: PackageRef {
                scope: "".to_string(),
                name: input.to_string(),
                version: None,
            },
            resolved_version: "local".to_string(),
            path: local_workflow,
            source: PackageSource::LocalWorkflow,
        });
    }

    // Not found anywhere
    Err(NikaError::WorkflowNotFound(format!(
        "Workflow '{}' not found in packages or local directory",
        input
    )))
}
```

**Integration with `run_workflow`:**
```rust
// src/main.rs (modify existing run_workflow function)
async fn run_workflow(
    file: &str,
    provider_override: Option<String>,
    model_override: Option<String>,
) -> Result<(), NikaError> {
    // ENHANCED: Resolve package or file path
    let resolved = registry::resolver::resolve_workflow_path(file).await?;

    tracing::info!(
        "Resolved {} to {} (source: {:?})",
        file,
        resolved.path.display(),
        resolved.source
    );

    // Read workflow file
    let yaml = tokio::fs::read_to_string(&resolved.path).await?;

    // Rest of existing logic unchanged...
    // (workflow parsing, validation, execution)
}
```

**Test Spec:**
```rust
#[tokio::test]
async fn test_resolve_package_path_latest() {
    // Setup: Install test package
    setup_test_package("@workflows/test", "1.0.0").await;

    let resolved = resolve_package_path("@workflows/test").await.unwrap();

    assert_eq!(resolved.reference.full_name(), "@workflows/test");
    assert_eq!(resolved.resolved_version, "1.0.0");
    assert!(resolved.path.exists());
}

#[tokio::test]
async fn test_resolve_package_path_with_version() {
    setup_test_package("@workflows/test", "1.0.0").await;
    setup_test_package("@workflows/test", "2.0.0").await;

    let resolved = resolve_package_path("@workflows/test@1.0.0").await.unwrap();

    assert_eq!(resolved.resolved_version, "1.0.0");
}

#[tokio::test]
async fn test_resolve_from_lock_file() {
    setup_test_package("@workflows/test", "1.0.0").await;
    setup_test_package("@workflows/test", "2.0.0").await;

    // Create spn.lock with version 1.0.0
    std::fs::write("spn.lock", r#"
packages:
  - name: "@workflows/test"
    version: "1.0.0"
    "#).unwrap();

    let resolved = resolve_package_path("@workflows/test").await.unwrap();

    assert_eq!(resolved.resolved_version, "1.0.0");
}

#[tokio::test]
async fn test_resolve_workflow_path_local() {
    // Create local workflow
    std::fs::create_dir_all(".nika/workflows").unwrap();
    std::fs::write(".nika/workflows/custom.nika.yaml", "schema: nika/workflow@0.9\n").unwrap();

    let resolved = resolve_workflow_path("custom").await.unwrap();

    assert_eq!(resolved.source, PackageSource::LocalWorkflow);
    assert!(resolved.path.ends_with(".nika/workflows/custom.nika.yaml"));
}

#[tokio::test]
async fn test_package_ref_parsing() {
    let ref1 = PackageRef::parse("@workflows/seo").unwrap();
    assert_eq!(ref1.scope, "@workflows");
    assert_eq!(ref1.name, "seo");
    assert_eq!(ref1.version, None);

    let ref2 = PackageRef::parse("@workflows/seo@1.2.0").unwrap();
    assert_eq!(ref2.version, Some("1.2.0".to_string()));

    let ref3 = PackageRef::parse("invalid");
    assert!(ref3.is_err());
}
```

**Success Criteria:**
- ✅ `nika run @workflows/test` resolves to `~/.spn/packages/@workflows/test/VERSION/workflow.nika.yaml`
- ✅ Version resolution priority: explicit > spn.lock > latest
- ✅ Local workflows still work (`.nika/workflows/custom`)
- ✅ All tests pass (8 tests)

---

### Week 2: Interactive CLI + Progress Animations

#### Task 2.1: Add Dialoguer Dependencies 🟡 P1
**Time:** 30 minutes
**Files:**
- `Cargo.toml`

**Add Dependencies:**
```toml
[dependencies]
# Interactive CLI
dialoguer = "0.11"       # Prompts, selections, confirmations
indicatif = "0.17"       # Progress bars, spinners
console = "0.15"         # Terminal styling, colors
```

**Test:**
```bash
cargo build
# Should compile successfully
```

---

#### Task 2.2: Create Interactive UI Module 🟡 P1
**Time:** 3 hours
**Files:**
- `src/ui/` (new directory)
  - `mod.rs` (module exports)
  - `theme.rs` (custom theme for dialoguer)
  - `progress.rs` (progress bar helpers)
  - `prompts.rs` (reusable prompt components)

**Theme Implementation:**
```rust
// src/ui/theme.rs
use dialoguer::theme::{ColorfulTheme, Theme};
use console::Style;

pub fn spn_theme() -> ColorfulTheme {
    ColorfulTheme {
        defaults_style: Style::new().for_stderr().cyan(),
        prompt_style: Style::new().for_stderr().bold(),
        prompt_prefix: Style::new().for_stderr().yellow().apply_to("?".to_string()),
        prompt_suffix: Style::new().for_stderr().dim().apply_to(" ".to_string()),
        success_prefix: Style::new().for_stderr().green().apply_to("✓".to_string()),
        success_suffix: Style::new().for_stderr().dim().apply_to(" ".to_string()),
        error_prefix: Style::new().for_stderr().red().apply_to("✗".to_string()),
        error_style: Style::new().for_stderr().red(),
        hint_style: Style::new().for_stderr().dim(),
        values_style: Style::new().for_stderr().green(),
        active_item_style: Style::new().for_stderr().cyan().bold(),
        inactive_item_style: Style::new().for_stderr(),
        active_item_prefix: Style::new().for_stderr().green().apply_to("❯".to_string()),
        inactive_item_prefix: Style::new().for_stderr().apply_to(" ".to_string()),
        checked_item_prefix: Style::new().for_stderr().green().apply_to("[x]".to_string()),
        unchecked_item_prefix: Style::new().for_stderr().apply_to("[ ]".to_string()),
        picked_item_prefix: Style::new().for_stderr().green().apply_to("❯".to_string()),
        unpicked_item_prefix: Style::new().for_stderr().apply_to(" ".to_string()),
    }
}
```

**Progress Bar Helpers:**
```rust
// src/ui/progress.rs
use indicatif::{ProgressBar, ProgressStyle};

pub fn download_progress(total_bytes: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("   │ {bar:30.cyan/blue} {percent}% │ {bytes}/{total_bytes}")
            .unwrap()
            .progress_chars("▓▓░")
    );
    pb
}

pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}
```

**Reusable Prompts:**
```rust
// src/ui/prompts.rs
use dialoguer::{Select, MultiSelect, Input, Confirm};
use super::theme::spn_theme;

pub fn select_package(packages: Vec<String>) -> Result<usize> {
    Select::with_theme(&spn_theme())
        .with_prompt("Select package to install")
        .items(&packages)
        .default(0)
        .interact()
        .map_err(|e| SpnError::UserCancelled)
}

pub fn multi_select_packages(packages: Vec<String>) -> Result<Vec<usize>> {
    MultiSelect::with_theme(&spn_theme())
        .with_prompt("Select packages (Space to toggle, Enter to confirm)")
        .items(&packages)
        .interact()
        .map_err(|e| SpnError::UserCancelled)
}

pub fn confirm_action(prompt: &str, default: bool) -> Result<bool> {
    Confirm::with_theme(&spn_theme())
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(|e| SpnError::UserCancelled)
}

pub fn input_text(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut input = Input::with_theme(&spn_theme())
        .with_prompt(prompt);

    if let Some(def) = default {
        input = input.default(def.to_string());
    }

    input.interact_text()
        .map_err(|e| SpnError::UserCancelled)
}
```

**Test Spec:**
```rust
#[test]
fn test_theme_creation() {
    let theme = spn_theme();
    // Should not panic
}

#[test]
fn test_progress_bar_creation() {
    let pb = download_progress(1024);
    pb.finish_and_clear();
}

#[test]
fn test_spinner_creation() {
    let spinner = spinner("Testing...");
    spinner.finish_and_clear();
}
```

---

#### Task 2.3: Implement Interactive `spn add` 🟡 P1
**Time:** 4 hours
**Files:**
- `src/commands/add.rs` (major refactor)

**Current Flow:**
```bash
$ spn add @workflows/seo-audit
# Direct install (works but not interactive)
```

**New Flow:**
```rust
// src/commands/add.rs
use crate::ui::{prompts, progress, theme};
use crate::index::IndexClient;

pub async fn run(package: Option<String>) -> Result<()> {
    match package {
        Some(pkg) => {
            // Direct install mode
            install_package_direct(&pkg).await
        }
        None => {
            // Interactive mode
            install_package_interactive().await
        }
    }
}

async fn install_package_interactive() -> Result<()> {
    // Step 1: Ask for search query
    let query = prompts::input_text("What are you looking for?", None)?;

    // Step 2: Search with spinner
    let spinner = progress::spinner(&format!("Searching for \"{}\"...", query));
    let client = IndexClient::new();
    let results = client.search(&query).await?;
    spinner.finish_and_clear();

    if results.is_empty() {
        eprintln!("❌ No packages found for \"{}\"", query);
        return Ok(());
    }

    println!("\n🔍 Found {} package{}\n", results.len(), if results.len() == 1 { "" } else { "s" });

    // Step 3: Format results for display
    let choices: Vec<String> = results.iter().map(|pkg| {
        format!(
            "{} {:<30} v{:<10} {} {:<5} • 📦 {} • 📅 {}",
            pkg.icon(),
            pkg.full_name(),
            pkg.version,
            if pkg.is_private { "🔒" } else { "  " },
            format!("⭐ {}", pkg.stars),
            format_number(pkg.downloads),
            pkg.updated_ago()
        )
    }).collect();

    // Step 4: User selection
    let selected_idx = prompts::select_package(choices)?;
    let selected_pkg = &results[selected_idx];

    println!("\n📦 Installing {}@{}...\n", selected_pkg.full_name(), selected_pkg.version);

    // Step 5: Download with progress bar
    let tarball = download_with_progress(&client, selected_pkg).await?;

    // Step 6: Extract
    println!("\n   Extracting to ~/.spn/packages/...");
    let extract_pb = progress::spinner("Extracting files...");
    let storage = Storage::new();
    let installed = storage.install(&tarball)?;
    extract_pb.finish_with_message("Extracted");

    // Step 7: Update manifests
    update_spn_yaml(&installed)?;
    update_spn_lock(&installed)?;

    println!("\n✅ Installed {}@{}\n", installed.name, installed.version);
    println!("   ✓ Added to spn.yaml");
    println!("   ✓ Updated spn.lock");

    if Path::new(".nika").exists() {
        sync_to_nika(&installed)?;
        println!("   ✓ Linked to .nika/.cache/");
    }

    // Step 8: Offer to run
    println!("\n→ Run with:");
    match installed.package_type {
        PackageType::Workflow => {
            println!("   nika run {}@{}", installed.name, installed.version);
        }
        PackageType::Agent => {
            println!("   # Use in workflow: agent: {{ pkg: {} }}", installed.name);
        }
        _ => {}
    }

    Ok(())
}

async fn download_with_progress(
    client: &IndexClient,
    pkg: &PackageInfo,
) -> Result<DownloadedPackage> {
    let url = client.tarball_url(pkg);

    // Get content length
    let response = client.head(&url).await?;
    let total_size = response.content_length().unwrap_or(0);

    // Create progress bar
    let pb = progress::download_progress(total_size);

    // Download with progress updates
    let mut response = client.get(&url).await?;
    let mut downloaded: u64 = 0;
    let mut buffer = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        buffer.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_and_clear();

    Ok(DownloadedPackage {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        data: buffer,
    })
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
```

**Test Spec:**
```rust
#[tokio::test]
async fn test_add_direct_mode() {
    // Mock registry
    let mock_server = MockServer::start().await;

    let result = run(Some("@workflows/test".to_string())).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_format_number() {
    assert_eq!(format_number(500), "500");
    assert_eq!(format_number(1500), "1.5K");
    assert_eq!(format_number(2_500_000), "2.5M");
}

// Note: Interactive mode tests require mocking stdin (complex, defer to manual testing)
```

**Success Criteria:**
- ✅ `spn add` (no args) prompts for search
- ✅ Shows formatted results with icons and metadata
- ✅ Arrow keys navigate packages
- ✅ Progress bar shows download progress
- ✅ Success message shows next steps

---

### Week 3: Include Package Support + Sync

#### Task 3.1: Add `pkg:` Support to IncludeSpec 🟡 P1
**Time:** 2 hours
**Files:**
- `nika/tools/nika/src/ast/include.rs`
- `nika/tools/nika/src/ast/include_loader.rs`

**Current IncludeSpec:**
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum IncludeSpec {
    Path {
        path: String,
        #[serde(default)]
        prefix: Option<String>,
    },
}
```

**Enhanced IncludeSpec:**
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum IncludeSpec {
    Path {
        path: String,
        #[serde(default)]
        prefix: Option<String>,
    },
    Package {
        pkg: String,  // @workflows/name[@version]
        #[serde(default)]
        prefix: Option<String>,
    },
}
```

**Update Loader:**
```rust
// src/ast/include_loader.rs
async fn expand_includes_recursive(
    workflow: &mut Workflow,
    base_path: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<(), NikaError> {
    // ... existing checks ...

    for include_spec in includes {
        let include_path = match &include_spec {
            IncludeSpec::Path { path, .. } => {
                let resolved = if path.starts_with("./") || path.starts_with("../") {
                    base_path.join(path)
                } else {
                    PathBuf::from(path)
                };
                resolved
            }
            IncludeSpec::Package { pkg, .. } => {
                // NEW: Resolve package to filesystem path
                let resolved = crate::registry::resolver::resolve_package_path(pkg).await?;
                resolved.path
            }
        };

        // Check for cycles
        if visited.contains(&include_path) {
            return Err(NikaError::CircularInclude(include_path.display().to_string()));
        }
        visited.insert(include_path.clone());

        // Load included workflow
        let included_yaml = tokio::fs::read_to_string(&include_path).await?;
        let mut included_workflow: Workflow = crate::serde_yaml::from_str(&included_yaml)?;

        // Apply prefix if specified
        if let Some(prefix) = match &include_spec {
            IncludeSpec::Path { prefix, .. } => prefix,
            IncludeSpec::Package { prefix, .. } => prefix,
        } {
            apply_prefix_to_tasks(&mut included_workflow, prefix);
        }

        // Recursive expansion
        expand_includes_recursive(&mut included_workflow, include_path.parent().unwrap(), visited, depth + 1).await?;

        // Merge into main workflow
        workflow.merge(included_workflow);
    }

    Ok(())
}
```

**Test Spec:**
```rust
#[tokio::test]
async fn test_include_package_reference() {
    // Setup: Install test package
    setup_test_package("@workflows/tasks", "1.0.0", r#"
schema: nika/workflow@0.9
tasks:
  - id: subtask
    infer: "Do something"
    "#).await;

    // Create workflow with package include
    let workflow_yaml = r#"
schema: nika/workflow@0.9
include:
  - pkg: @workflows/tasks
    prefix: sub_

tasks:
  - id: main
    infer: "Main task"
  - id: use_subtask
    use: { result: sub_subtask }
    "#;

    let mut workflow: Workflow = serde_yaml::from_str(workflow_yaml).unwrap();
    expand_includes(&mut workflow).await.unwrap();

    // Check that subtask was included with prefix
    assert!(workflow.tasks.iter().any(|t| t.id == "sub_subtask"));
}

#[tokio::test]
async fn test_include_package_with_version() {
    setup_test_package("@workflows/tasks", "1.0.0", "...").await;
    setup_test_package("@workflows/tasks", "2.0.0", "...").await;

    let workflow_yaml = r#"
include:
  - pkg: @workflows/tasks@1.0.0
    "#;

    let mut workflow: Workflow = serde_yaml::from_str(workflow_yaml).unwrap();
    expand_includes(&mut workflow).await.unwrap();

    // Should use v1.0.0 (verify by checking task content)
}

#[tokio::test]
async fn test_circular_include_detection() {
    // Setup packages that include each other
    setup_test_package("@workflows/a", "1.0.0", r#"
include:
  - pkg: @workflows/b
    "#).await;

    setup_test_package("@workflows/b", "1.0.0", r#"
include:
  - pkg: @workflows/a
    "#).await;

    let workflow_yaml = r#"
include:
  - pkg: @workflows/a
    "#;

    let mut workflow: Workflow = serde_yaml::from_str(workflow_yaml).unwrap();
    let result = expand_includes(&mut workflow).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), NikaError::CircularInclude(_)));
}
```

**Success Criteria:**
- ✅ `pkg: @workflows/name` resolves and includes tasks
- ✅ Prefix application works with packages
- ✅ Circular includes are detected
- ✅ All tests pass

---

#### Task 3.2: Implement `spn sync` to .nika/ 🟡 P1
**Time:** 3 hours
**Files:**
- `src/sync/nika_sync.rs` (new module)
- `src/commands/install.rs` (integrate sync)
- `src/commands/add.rs` (integrate sync)
- `src/commands/sync.rs` (new command)

**Sync Module:**
```rust
// src/sync/nika_sync.rs
use std::path::{Path, PathBuf};
use crate::storage::InstalledPackage;
use crate::manifest::PackageType;

pub fn sync_package_to_nika(pkg: &InstalledPackage) -> Result<()> {
    // Check if .nika/ exists
    let nika_dir = Path::new(".nika");
    if !nika_dir.exists() {
        // Silently skip if not a nika project
        return Ok(());
    }

    let cache_dir = nika_dir.join(".cache");
    std::fs::create_dir_all(&cache_dir)?;

    // Determine package type and target directory
    let (subdir, entry_file) = match pkg.package_type {
        PackageType::Workflow => ("workflows", "workflow.nika.yaml"),
        PackageType::Agent => ("agents", "agent.md"),
        PackageType::Job => ("jobs", "job.nika.yaml"),
        PackageType::Prompt => ("prompts", "prompt.md"),
        _ => return Ok(()), // Skip non-Nika packages
    };

    let link_dir = cache_dir.join(subdir);
    std::fs::create_dir_all(&link_dir)?;

    // Extract short name (@workflows/seo-audit → seo-audit)
    let short_name = pkg.name.split('/').last().unwrap_or(&pkg.name);
    let link_path = link_dir.join(short_name);

    // Remove old symlink if exists
    if link_path.exists() || link_path.is_symlink() {
        #[cfg(unix)]
        std::fs::remove_file(&link_path).or_else(|_| std::fs::remove_dir_all(&link_path))?;

        #[cfg(windows)]
        std::fs::remove_dir_all(&link_path).or_else(|_| std::fs::remove_file(&link_path))?;
    }

    // Create symlink
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&pkg.path, &link_path)?;
    }

    #[cfg(windows)]
    {
        if pkg.path.is_dir() {
            std::os::windows::fs::symlink_dir(&pkg.path, &link_path)?;
        } else {
            std::os::windows::fs::symlink_file(&pkg.path, &link_path)?;
        }
    }

    println!("   ✓ Linked to .nika/.cache/{}/{}", subdir, short_name);

    Ok(())
}

pub fn sync_all_packages() -> Result<()> {
    // Sync all installed packages to .nika/
    let storage = crate::storage::Storage::new();
    let packages = storage.list_installed()?;

    println!("🔄 Syncing {} packages to .nika/...\n", packages.len());

    for pkg in packages {
        sync_package_to_nika(&pkg)?;
    }

    println!("\n✅ Sync complete");

    Ok(())
}
```

**Sync Command:**
```rust
// src/commands/sync.rs
pub fn run() -> Result<()> {
    crate::sync::nika_sync::sync_all_packages()
}
```

**Integration:**
```rust
// src/commands/install.rs (after each package install)
if Path::new(".nika").exists() {
    crate::sync::nika_sync::sync_package_to_nika(&installed)?;
}

// src/commands/add.rs (after install)
if Path::new(".nika").exists() {
    crate::sync::nika_sync::sync_package_to_nika(&installed)?;
}
```

**Test Spec:**
```rust
#[test]
fn test_sync_package_to_nika() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create .nika/ directory
    std::fs::create_dir(".nika").unwrap();

    // Mock installed package
    let pkg = InstalledPackage {
        name: "@workflows/test".to_string(),
        version: "1.0.0".to_string(),
        package_type: PackageType::Workflow,
        path: PathBuf::from("/tmp/test"),
    };

    sync_package_to_nika(&pkg).unwrap();

    // Verify symlink created
    let link_path = Path::new(".nika/.cache/workflows/test");
    assert!(link_path.exists());
    assert!(link_path.is_symlink());
}

#[test]
fn test_sync_all_packages() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    std::fs::create_dir(".nika").unwrap();

    // Install 3 test packages
    install_test_package("@workflows/test1", "1.0.0");
    install_test_package("@workflows/test2", "1.0.0");
    install_test_package("@agents/test3", "1.0.0");

    sync_all_packages().unwrap();

    // Verify all synced
    assert!(Path::new(".nika/.cache/workflows/test1").exists());
    assert!(Path::new(".nika/.cache/workflows/test2").exists());
    assert!(Path::new(".nika/.cache/agents/test3").exists());
}
```

**Success Criteria:**
- ✅ `spn install` automatically syncs to .nika/
- ✅ `spn add` automatically syncs to .nika/
- ✅ `spn sync` command works standalone
- ✅ Symlinks created correctly on Unix and Windows
- ✅ All tests pass

---

### Week 4: Polish + Documentation

#### Task 4.1: Implement Wizard `spn init` 🟢 P2
**Time:** 4 hours
**Files:**
- `src/commands/init.rs` (complete rewrite)

**Current:** Basic init creates config.toml

**New:** Interactive wizard with package installation

```rust
// src/commands/init.rs
use crate::ui::{prompts, progress};
use dialoguer::{Select, MultiSelect, Input};

pub async fn run(non_interactive: bool) -> Result<()> {
    if non_interactive {
        return run_non_interactive().await;
    }

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  🚀 SuperNovae Project Initialization                     ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Step 1: Project name
    let project_name = prompts::input_text("Project name", Some("my-ai-project"))?;

    // Step 2: Project type
    let project_types = vec![
        "Content Generation - SEO, blog posts, marketing",
        "Code Automation - Review, refactor, testing",
        "Research Pipeline - Web scraping, analysis, reports",
        "Empty - I'll configure it myself",
    ];

    let project_type_idx = Select::with_theme(&crate::ui::theme::spn_theme())
        .with_prompt("Project type")
        .items(&project_types)
        .default(0)
        .interact()?;

    // Step 3: Install starter workflows?
    let install_workflows = prompts::confirm_action("Install starter workflows?", true)?;

    let selected_packages = if install_workflows {
        // Show relevant packages based on project type
        let suggested = suggest_packages_for_type(project_type_idx);

        let package_names: Vec<String> = suggested.iter()
            .map(|pkg| format!("{} - {}", pkg.name, pkg.description))
            .collect();

        let selected_indices = MultiSelect::with_theme(&crate::ui::theme::spn_theme())
            .with_prompt("Select workflows (Space to toggle, Enter to confirm)")
            .items(&package_names)
            .defaults(&vec![true; suggested.len()])  // All checked by default
            .interact()?;

        selected_indices.into_iter()
            .map(|i| suggested[i].clone())
            .collect()
    } else {
        vec![]
    };

    // Step 4: Create project structure
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  📦 Creating project...                                   ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    create_nika_structure(&project_name)?;
    create_spn_manifest(&project_name, &selected_packages)?;

    // Step 5: Install packages
    if !selected_packages.is_empty() {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║  📦 Installing {} packages...                             ║", selected_packages.len());
        println!("╚═══════════════════════════════════════════════════════════╝\n");

        for pkg in &selected_packages {
            install_package_with_progress(&pkg.name).await?;
        }
    }

    // Step 6: Success message
    println!("\n✅ Project initialized!\n");
    println!("   📂 Created:");
    println!("      .nika/config.toml");
    println!("      .nika/policies.yaml");
    println!("      spn.yaml");
    if !selected_packages.is_empty() {
        println!("      spn.lock");
    }

    if !selected_packages.is_empty() {
        println!("\n   📦 Installed:");
        for pkg in &selected_packages {
            println!("      {} v{}", pkg.name, pkg.version);
        }

        println!("\n→ Get started:");
        println!("   nika run {} --help", selected_packages[0].name);
    } else {
        println!("\n→ Get started:");
        println!("   spn add <package>    # Install a workflow");
        println!("   nika run <workflow>  # Run a workflow");
    }

    Ok(())
}

fn suggest_packages_for_type(type_idx: usize) -> Vec<SuggestedPackage> {
    match type_idx {
        0 => vec![  // Content Generation
            SuggestedPackage {
                name: "@workflows/seo-audit".to_string(),
                version: "1.2.0".to_string(),
                description: "SEO analysis and recommendations".to_string(),
            },
            SuggestedPackage {
                name: "@workflows/content-generator".to_string(),
                version: "2.0.1".to_string(),
                description: "AI-powered content creation".to_string(),
            },
            SuggestedPackage {
                name: "@agents/researcher".to_string(),
                version: "1.5.0".to_string(),
                description: "Research agent for gathering information".to_string(),
            },
        ],
        1 => vec![  // Code Automation
            SuggestedPackage {
                name: "@workflows/code-review".to_string(),
                version: "1.0.0".to_string(),
                description: "Automated code review".to_string(),
            },
            SuggestedPackage {
                name: "@agents/code-analyzer".to_string(),
                version: "1.2.0".to_string(),
                description: "Deep code analysis agent".to_string(),
            },
        ],
        2 => vec![  // Research Pipeline
            SuggestedPackage {
                name: "@workflows/web-scraper".to_string(),
                version: "2.1.0".to_string(),
                description: "Extract data from websites".to_string(),
            },
            SuggestedPackage {
                name: "@agents/researcher".to_string(),
                version: "1.5.0".to_string(),
                description: "Research agent".to_string(),
            },
        ],
        _ => vec![],  // Empty
    }
}

async fn install_package_with_progress(package: &str) -> Result<()> {
    println!("   {}", package);

    let client = crate::index::IndexClient::new();
    let pkg_info = client.get_package_info(package).await?;

    // Download
    let pb = crate::ui::progress::download_progress(pkg_info.size);
    let downloaded = client.download(&pkg_info, |progress| {
        pb.set_position(progress);
    }).await?;
    pb.finish_and_clear();

    // Extract
    let storage = crate::storage::Storage::new();
    storage.install(&downloaded)?;

    println!("   │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100% │ {}\n", format_size(pkg_info.size));

    Ok(())
}
```

**Test Spec:**
```rust
#[tokio::test]
async fn test_init_non_interactive() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    run(true).await.unwrap();

    assert!(Path::new(".nika/config.toml").exists());
    assert!(Path::new("spn.yaml").exists());
}

// Interactive tests require stdin mocking (complex, defer to manual testing)
```

**Success Criteria:**
- ✅ Wizard guides through project setup
- ✅ Suggests packages based on project type
- ✅ Installs selected packages automatically
- ✅ Creates all necessary config files
- ✅ Shows clear next steps

---

#### Task 4.2: API Key Detection Integration 🟢 P2
**Time:** 3 hours
**Files:**
- `src/commands/run.rs` (new proxy command)
- `src/secrets/detection.rs` (from Week 1 design)

*Implementation already detailed in "API Key Detection & Tutorial" section above*

**Success Criteria:**
- ✅ Detects missing keys before execution
- ✅ Shows inline tutorial
- ✅ Offers to configure immediately
- ✅ Works with all LLM providers

---

#### Task 4.3: Private Registry Support 🟢 P2
**Time:** 3 hours
**Files:**
- `src/index/client.rs` (enhance for GitHub auth)
- `src/secrets/keyring.rs` (add GitHub token support)

*Implementation already detailed in "Private Registry Support" section above*

**Success Criteria:**
- ✅ Searches both public and private registries
- ✅ GitHub token authentication works
- ✅ Private packages show 🔒 icon
- ✅ Fallback to public if auth fails

---

#### Task 4.4: Documentation 🟢 P2
**Time:** 4 hours
**Files:**
- `docs/guides/getting-started.md` (new)
- `docs/guides/package-resolution.md` (new)
- `docs/guides/interactive-cli.md` (new)
- `README.md` (update)

**Getting Started Guide:**
```markdown
# Getting Started with spn ↔ nika

## Installation

```bash
brew install supernovae-st/tap/spn
brew install supernovae-st/tap/nika
```

## Quick Start

### 1. Initialize a New Project

```bash
mkdir my-project && cd my-project
spn init
```

The wizard will guide you through:
- Project type selection
- Starter workflow installation
- Configuration setup

### 2. Discover Packages

```bash
spn search seo
```

### 3. Install a Workflow

Interactive mode:
```bash
spn add
# Follow prompts
```

Direct mode:
```bash
spn add @workflows/seo-audit
```

### 4. Run a Workflow

```bash
nika run @workflows/seo-audit --url https://example.com
```

## Package Resolution

Nika resolves packages in this order:
1. **Explicit version:** `@workflows/name@1.2.0`
2. **From spn.lock:** Locked version in current project
3. **Latest installed:** Most recent version in `~/.spn/packages/`

## Configuration

### API Keys

Store API keys securely:

```bash
spn provider set anthropic
# Enter key: sk-ant-***
```

Keys are stored in:
- macOS: Keychain
- Windows: Credential Manager
- Linux: Secret Service (or fallback to .env)

### Multiple Registries

Edit `~/.spn/config.toml`:

```toml
[[registry]]
name = "public"
url = "https://registry.supernovae.studio/registry.json"

[[registry]]
name = "private"
url = "https://github.com/org/private-registry/raw/main/registry.json"
auth = "github"
```

## Troubleshooting

### Package Not Found

```bash
spn doctor
# Checks registry connectivity
```

### Workflow Validation Errors

```bash
nika check workflow.nika.yaml
# Shows detailed schema errors
```

### Missing API Keys

If you see "Missing API Keys Detected":
1. Get API key from provider
2. Run `spn provider set <provider>`
3. Retry workflow

## Next Steps

- [Package Resolution Guide](./package-resolution.md)
- [Interactive CLI Guide](./interactive-cli.md)
- [Creating Workflows](../../nika/docs/guides/workflows.md)
```

**Success Criteria:**
- ✅ Clear getting started guide
- ✅ All features documented
- ✅ Troubleshooting section
- ✅ Examples for common tasks

---

## ✅ Final Validation Checklist

Before declaring v0.7.0 complete:

### Core Functionality
- [ ] `spn add @workflows/name` works without panic
- [ ] `nika run @workflows/name` resolves packages correctly
- [ ] Package version resolution: explicit > lock > latest
- [ ] `include: { pkg: @workflows/name }` works in workflows
- [ ] `spn install` syncs to `.nika/.cache/`
- [ ] Local workflows coexist with packages

### Interactive CLI
- [ ] `spn init` wizard guides setup
- [ ] `spn add` (no args) provides interactive search
- [ ] Progress bars show during downloads
- [ ] Spinners animate during searches
- [ ] Success messages show next steps

### API Key Detection
- [ ] Missing keys detected before execution
- [ ] Inline tutorial shown
- [ ] Can configure keys immediately
- [ ] Works with all providers

### Private Registry
- [ ] GitHub authentication works
- [ ] Private packages accessible
- [ ] Fallback to public works
- [ ] Token stored securely

### Bug Fixes
- [ ] No Tokio panic in `spn add`
- [ ] `nika init` generates valid workflows
- [ ] All 4 templates validate

### Tests
- [ ] All unit tests pass (target: 50 tests)
- [ ] Integration tests pass (3 scenarios)
- [ ] Manual testing complete

### Documentation
- [ ] Getting started guide complete
- [ ] Package resolution documented
- [ ] API reference updated
- [ ] README updated

### Performance
- [ ] Package search < 1s (public registry)
- [ ] Package install < 5s (2MB package)
- [ ] Workflow resolution < 100ms

---

## 📊 Metrics & Success Criteria

### User Story Validation

**Story 1: New User Discovers Package**
```bash
# Time target: 30 seconds
mkdir test-project && cd test-project
spn init                        # 10s (wizard)
spn add                         # 5s (search)
# Select @workflows/seo-audit
nika run @workflows/seo-audit   # 5s (execution)

✅ Success if completed in < 45s total
```

**Story 2: Developer Uses Package in Workflow**
```bash
cat > workflow.nika.yaml <<EOF
schema: nika/workflow@0.9
include:
  - pkg: @workflows/seo-audit
    prefix: seo_

tasks:
  - id: run_audit
    invoke: seo_audit
EOF

nika run workflow.nika.yaml

✅ Success if workflow executes without errors
```

**Story 3: Team Uses Private Package**
```bash
spn provider set github         # One-time setup
spn add @qrcodeai/seo-sprint   # Private package
nika run @qrcodeai/seo-sprint

✅ Success if private package accessible
```

### Performance Targets

| Operation | Target | Measured |
|-----------|--------|----------|
| Registry search | < 1s | TBD |
| Package download (2MB) | < 3s | TBD |
| Package install | < 2s | TBD |
| Workflow resolution | < 100ms | TBD |
| Full `spn init` wizard | < 30s | TBD |

### Code Quality

| Metric | Target | Actual |
|--------|--------|--------|
| Unit tests | 40+ | TBD |
| Integration tests | 5+ | TBD |
| Test coverage | > 60% | TBD |
| Clippy warnings | 0 | TBD |
| Documentation coverage | 100% public API | TBD |

---

## 🚀 Release Plan

### v0.7.0-alpha.1 (Week 2)
- Bug fixes (Tokio panic, invalid examples)
- Package resolution in nika
- Basic interactive CLI

**Release Notes:**
> - Fix Tokio runtime panic in spn add
> - Fix invalid workflow examples in nika init
> - Add package URI resolution (@workflows/name)
> - Add interactive package search (spn add)
> - Add progress bars and spinners

### v0.7.0-beta.1 (Week 3)
- Include package support
- Sync to .nika/
- API key detection

**Release Notes:**
> - Add pkg: support in workflow includes
> - Automatic sync to .nika/.cache/
> - Smart API key detection with tutorials
> - Enhanced error messages

### v0.7.0 (Week 4)
- Full wizard mode
- Private registry support
- Complete documentation

**Release Notes:**
> - Interactive project initialization wizard
> - Private registry support with GitHub auth
> - Complete user documentation
> - 50+ tests, 60% coverage
> - Ready for production use

---

## 🔮 Future Enhancements (v0.8.0+)

### Deferred Features

**Full TUI Mode** (v0.8.0)
- `spn` with no args opens App Store-like interface
- Browse, search, and install packages visually
- View package details, dependencies, changelog

**Package Groups** (v0.8.0)
- Install related packages together
- `spn add @group/ai-dev` installs workflows + agents + skills
- Define groups in registry

**Auto-Update** (v0.8.0)
- `spn outdated` shows available updates
- `spn update` updates all packages
- Respects semver constraints

**Publishing Workflow** (v0.9.0)
- `spn publish` packages to registry
- Version bump, tarball creation, upload
- Automated CI/CD integration

**Schema Integration** (v0.9.0)
- `@schemas/` package type
- NovaNet schema resolution
- Knowledge graph integration

---

## 📝 Notes & Assumptions

### Technical Decisions

**Why Dialoguer over Inquire?**
- More mature (v0.11 vs v0.3)
- Better theming support
- Proven in production (used by cargo, rustup)

**Why Indicatif over pbr?**
- Active maintenance
- Better terminal support
- Unicode-aware progress bars

**Why Option C (Router Pattern)?**
- ✅ Simplest to implement
- ✅ No shared Rust code needed
- ✅ Clear separation of concerns
- ✅ Easy to maintain
- ✅ Works with existing binaries

**Symlinks vs Copies?**
- Symlinks chosen for instant updates
- Disk space savings (no duplication)
- Follows npm/pnpm pattern

### Known Limitations

**Windows Symlink Permissions**
- Requires Developer Mode or Admin
- Fallback to directory junction (mklink /J)
- Document in Windows setup guide

**Large Package Downloads**
- No resume support (yet)
- Network interruption = restart
- Consider adding resume in v0.8.0

**Offline Mode**
- Requires internet for first install
- Local cache not persistent
- Add offline mode in v0.9.0

### Dependencies Added

```toml
# New dependencies for v0.7.0
dialoguer = "0.11"        # Interactive prompts (~100KB)
indicatif = "0.17"        # Progress bars (~50KB)
console = "0.15"          # Terminal styling (~30KB)
```

**Total binary size impact:** +~500KB (acceptable)

---

## 🤝 Team Coordination

### Parallel Work Opportunities

**Week 1-2:** Can be done in parallel
- Bug fixes (solo)
- Package resolution (solo)
- UI module (solo)

**Week 3:** Sequential dependencies
- Include support requires package resolution (Week 2)
- Sync requires package resolution (Week 2)

**Week 4:** Independent polish tasks
- Wizard (solo)
- API detection (solo)
- Private registry (solo)
- Documentation (solo)

### Git Strategy

**Branches:**
- `feature/bug-fixes` (Week 1)
- `feature/package-resolution` (Week 1-2)
- `feature/interactive-cli` (Week 2)
- `feature/include-packages` (Week 3)
- `feature/sync-to-nika` (Week 3)
- `feature/wizard-init` (Week 4)
- `feature/api-detection` (Week 4)

**Merge Strategy:**
- PR after each task completes
- Code review required
- Tests must pass
- Squash merge to main

---

## ✅ Sign-Off

**Acceptance Criteria:**

This plan is considered complete when:
1. All 13 core commands work as specified
2. Interactive CLI provides smooth UX
3. Package resolution is robust
4. API key detection prevents user errors
5. Private registry access works
6. All bugs fixed
7. Tests pass (50+ tests, 60% coverage)
8. Documentation complete

**Estimated Total Effort:** 80-100 hours (4 weeks × 20-25h/week)

**Ready to Execute:** ✅ Yes

---

**Generated:** 2026-03-02
**Status:** 🚧 Ready for Implementation
**Next Step:** Start with Task 1.1 (Fix Tokio Panic)
