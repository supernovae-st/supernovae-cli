//! Help command implementation.
//!
//! Detailed help topics for SuperNovae CLI.

use crate::error::Result;
use crate::ux::design_system as ds;

/// Run help command with optional topic.
pub async fn run(topic: Option<&str>) -> Result<()> {
    match topic {
        Some("config") => show_config_help(),
        Some("scopes") => show_scopes_help(),
        Some("mcp") => show_mcp_help(),
        Some("sync") => show_sync_help(),
        Some("workflows") => show_workflows_help(),
        Some("registry") => show_registry_help(),
        Some("models") => show_models_help(),
        Some("providers") => show_providers_help(),
        Some("daemon") => show_daemon_help(),
        Some("architecture") | Some("arch") => show_architecture_help(),
        Some(t) => {
            eprintln!("{} Unknown topic: {}", ds::warning("!"), t);
            eprintln!();
            show_topics_list();
        }
        None => show_topics_list(),
    }
    Ok(())
}

fn show_topics_list() {
    println!("{}", ds::primary("SuperNovae Help"));
    println!("{}", ds::primary("==============="));
    println!();
    println!("{}", ds::highlight("Available topics:"));
    println!();
    println!("  {}", ds::muted("Getting Started"));
    println!(
        "    {}     Configuration system (spn.yaml, .spn/)",
        ds::primary("config")
    );
    println!(
        "    {}     Package scopes (@nika, @novanet, @community)",
        ds::primary("scopes")
    );
    println!("    {}   Package registry", ds::primary("registry"));
    println!();
    println!("  {}", ds::muted("Core Features"));
    println!("    {}        MCP server management", ds::primary("mcp"));
    println!(
        "    {}       IDE sync (Claude Code, Cursor, VS Code)",
        ds::primary("sync")
    );
    println!("    {}  Nika workflow packages", ds::primary("workflows"));
    println!();
    println!("  {}", ds::muted("Infrastructure"));
    println!(
        "    {}     Local LLM model management",
        ds::primary("models")
    );
    println!("    {}  API key management", ds::primary("providers"));
    println!("    {}     Background service", ds::primary("daemon"));
    println!("    {}       System architecture", ds::primary("arch"));
    println!();
    println!("Usage: {} <topic>", ds::command("spn topic"));
    println!();
    println!("{}", ds::highlight("Quick Reference:"));
    println!();
    println!(
        "  {}              Initialize project",
        ds::muted("spn init")
    );
    println!("  {}       Add package", ds::muted("spn add <pkg>"));
    println!(
        "  {}           Install dependencies",
        ds::muted("spn install")
    );
    println!("  {}              Sync to editors", ds::muted("spn sync"));
    println!(
        "  {}            System health check",
        ds::muted("spn doctor")
    );
}

fn show_config_help() {
    println!("{}", ds::primary("Configuration"));
    println!("{}", ds::primary("============="));
    println!();
    println!(
        "{}",
        ds::highlight("SuperNovae uses a layered configuration system:")
    );
    println!();
    println!("  {} Project-level manifest", ds::primary("1. spn.yaml"));
    println!("     Lists dependencies for a project");
    println!();
    println!(
        "  {} User-level settings",
        ds::primary("2. ~/.spn/config.json")
    );
    println!("     Registry credentials, sync preferences");
    println!();
    println!(
        "  {} Installed packages",
        ds::primary("3. ~/.spn/packages/")
    );
    println!("     Downloaded and extracted packages");
    println!();
    println!("{}", ds::highlight("spn.yaml Example:"));
    println!();
    println!("  {}", ds::muted("name: my-project"));
    println!("  {}", ds::muted("version: 1.0.0"));
    println!("  {}", ds::muted("dependencies:"));
    println!("  {}", ds::muted("  @nika/seo-audit: ^1.0.0"));
    println!("  {}", ds::muted("  @novanet/entity-explorer: ^0.5.0"));
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!("  {}         Create spn.yaml", ds::command("spn init"));
    println!("  {}  Show config", ds::command("spn config show"));
    println!(
        "  {}   Config file locations",
        ds::command("spn config where")
    );
}

fn show_scopes_help() {
    println!("{}", ds::primary("Package Scopes"));
    println!("{}", ds::primary("=============="));
    println!();
    println!("Packages are organized by scope (namespace):");
    println!();
    println!(
        "  {}   Official Nika workflow packages",
        ds::primary("@nika/*")
    );
    println!("              Examples: @nika/seo-audit, @nika/content-pipeline");
    println!();
    println!(
        "  {} Official NovaNet schema packages",
        ds::primary("@novanet/*")
    );
    println!("              Examples: @novanet/entity-explorer, @novanet/schema-tools");
    println!();
    println!(
        "  {}    Community-contributed packages",
        ds::primary("@community/*")
    );
    println!("              Examples: @community/my-workflow");
    println!();
    println!("{}", ds::highlight("Scope Features:"));
    println!();
    println!("  - Scopes are namespaces, not permissions");
    println!("  - Anyone can publish to @community/*");
    println!("  - @nika/* and @novanet/* require maintainer access");
    println!();
    println!("{}", ds::highlight("Index Structure:"));
    println!();
    println!("  {}", ds::muted("index/"));
    println!("  {}", ds::muted("├── @nika/"));
    println!("  {}", ds::muted("│   └── seo-audit"));
    println!("  {}", ds::muted("├── @novanet/"));
    println!("  {}", ds::muted("│   └── entity-explorer"));
    println!("  {}", ds::muted("└── @community/"));
    println!("  {}", ds::muted("    └── user-workflow"));
}

fn show_mcp_help() {
    println!("{}", ds::primary("MCP Servers"));
    println!("{}", ds::primary("==========="));
    println!();
    println!("MCP (Model Context Protocol) servers extend AI capabilities.");
    println!();
    println!("{}", ds::highlight("Built-in Aliases:"));
    println!();
    println!(
        "  {}          @neo4j/mcp-server-neo4j",
        ds::primary("neo4j")
    );
    println!(
        "  {}     @anthropic/mcp-server-filesystem",
        ds::primary("filesystem")
    );
    println!(
        "  {}         @anthropic/mcp-server-github",
        ds::primary("github")
    );
    println!(
        "  {}       @anthropic/mcp-server-postgres",
        ds::primary("postgres")
    );
    println!(
        "  {}         @anthropic/mcp-server-sqlite",
        ds::primary("sqlite")
    );
    println!(
        "  {}         @anthropic/mcp-server-memory",
        ds::primary("memory")
    );
    println!(
        "  {}      @anthropic/mcp-server-puppeteer",
        ds::primary("puppeteer")
    );
    println!(
        "  {}          @anthropic/mcp-server-fetch",
        ds::primary("fetch")
    );
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!(
        "  {}          Install neo4j MCP server",
        ds::command("spn mcp add neo4j")
    );
    println!(
        "  {}       Remove MCP server",
        ds::command("spn mcp remove neo4j")
    );
    println!(
        "  {}             List installed servers",
        ds::command("spn mcp list")
    );
    println!(
        "  {}         Test server connection",
        ds::command("spn mcp test neo4j")
    );
    println!();
    println!("{}", ds::highlight("Integration:"));
    println!();
    println!("  MCP servers are installed via npm globally.");
    println!(
        "  Use {} to sync MCP config to your editor.",
        ds::command("spn sync")
    );
}

fn show_sync_help() {
    println!("{}", ds::primary("IDE Sync"));
    println!("{}", ds::primary("========"));
    println!();
    println!("Sync packages and MCP servers to IDE configurations.");
    println!();
    println!("{}", ds::highlight("Supported IDEs:"));
    println!();
    println!("  {}     .claude/settings.json", ds::primary("claude-code"));
    println!("  {}         .cursor/settings.json", ds::primary("cursor"));
    println!("  {}         .vscode/settings.json", ds::primary("vscode"));
    println!(
        "  {}       .windsurf/settings.json",
        ds::primary("windsurf")
    );
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!(
        "  {}                    Sync all enabled editors",
        ds::command("spn sync")
    );
    println!(
        "  {}  Enable sync for editor",
        ds::command("spn sync --enable cursor")
    );
    println!(
        "  {}           Show sync status",
        ds::command("spn sync --status")
    );
    println!(
        "  {}          Preview changes",
        ds::command("spn sync --dry-run")
    );
    println!();
    println!("{}", ds::highlight("What Gets Synced:"));
    println!();
    println!("  - MCP server configurations (mcpServers section)");
    println!("  - Installed skills from skills.sh");
    println!("  - Package paths for Nika include: resolution");
    println!();
    println!("{}", ds::highlight("Sync Config:"));
    println!();
    println!("  Stored in ~/.spn/sync.json");
    println!("  Per-project overrides in .spn/sync.json");
}

fn show_workflows_help() {
    println!("{}", ds::primary("Nika Workflows"));
    println!("{}", ds::primary("=============="));
    println!();
    println!("Nika workflows are YAML files defining AI task pipelines.");
    println!();
    println!("{}", ds::highlight("Workflow Structure:"));
    println!();
    println!("  {}", ds::muted("name: my-workflow"));
    println!("  {}", ds::muted("description: Example workflow"));
    println!("  {}", ds::muted(""));
    println!("  {}", ds::muted("tasks:"));
    println!("  {}", ds::muted("  - id: fetch-data"));
    println!("  {}", ds::muted("    fetch: https://api.example.com/data"));
    println!("  {}", ds::muted("    use.ctx: api_data"));
    println!("  {}", ds::muted(""));
    println!("  {}", ds::muted("  - id: process"));
    println!("  {}", ds::muted("    infer: Process this data"));
    println!("  {}", ds::muted("    context: $api_data"));
    println!();
    println!("{}", ds::highlight("Verbs:"));
    println!();
    println!(
        "  {}   LLM inference (Claude, GPT, etc.)",
        ds::primary("infer:")
    );
    println!("  {}    Execute shell command", ds::primary("exec:"));
    println!("  {}   HTTP request", ds::primary("fetch:"));
    println!("  {}  MCP tool call", ds::primary("invoke:"));
    println!("  {}   Multi-turn agent loop", ds::primary("agent:"));
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!(
        "  {}     Run a workflow",
        ds::command("spn nk run file.yaml")
    );
    println!(
        "  {}   Validate syntax",
        ds::command("spn nk check file.yaml")
    );
    println!(
        "  {}          Open Nika Studio",
        ds::command("spn nk studio")
    );
}

fn show_registry_help() {
    println!("{}", ds::primary("Package Registry"));
    println!("{}", ds::primary("================"));
    println!();
    println!("SuperNovae uses a sparse index registry (like Cargo).");
    println!();
    println!("{}", ds::highlight("Registry Location:"));
    println!();
    println!(
        "  {}",
        ds::url("github.com/supernovae-st/supernovae-registry")
    );
    println!();
    println!("{}", ds::highlight("Registry Structure:"));
    println!();
    println!("  {}", ds::muted("supernovae-registry/"));
    println!(
        "  {}",
        ds::muted("├── config.json         # Registry metadata")
    );
    println!(
        "  {}",
        ds::muted("├── index/              # Sparse index (NDJSON)")
    );
    println!("  {}", ds::muted("│   ├── @nika/"));
    println!("  {}", ds::muted("│   │   └── seo-audit"));
    println!("  {}", ds::muted("│   └── @novanet/"));
    println!("  {}", ds::muted("│       └── schema-tools"));
    println!(
        "  {}",
        ds::muted("└── releases/           # Package tarballs")
    );
    println!("  {}", ds::muted("    └── @nika/"));
    println!("  {}", ds::muted("        └── seo-audit-1.0.0.tar.gz"));
    println!();
    println!("{}", ds::highlight("Index Format (NDJSON):"));
    println!();
    println!(
        "  {}",
        ds::muted(r#"{"name":"seo-audit","version":"1.0.0","checksum":"sha256:..."}"#)
    );
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!(
        "  {}      Search packages",
        ds::command("spn search <query>")
    );
    println!(
        "  {}          Package details",
        ds::command("spn info <pkg>")
    );
    println!(
        "  {}              Publish package",
        ds::command("spn publish")
    );
}

fn show_models_help() {
    println!("{}", ds::primary("Local LLM Models"));
    println!("{}", ds::primary("================"));
    println!();
    println!("Manage local LLM models via Ollama for offline inference.");
    println!();
    println!("{}", ds::highlight("Requirements:"));
    println!();
    println!(
        "  Ollama must be installed: {}",
        ds::primary("https://ollama.ai")
    );
    println!("  The spn daemon must be running for model operations.");
    println!();
    println!("{}", ds::highlight("Popular Models:"));
    println!();
    println!(
        "  {}    Fast, efficient (1B params)",
        ds::primary("llama3.2:1b")
    );
    println!("  {}    Balanced (7B params)", ds::primary("llama3.2:7b"));
    println!("  {}        Coding specialist", ds::primary("codellama"));
    println!("  {}     Lightweight (2.7B)", ds::primary("phi3:mini"));
    println!("  {}  Instruction-tuned", ds::primary("mistral:instruct"));
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!(
        "  {}             List installed models",
        ds::command("spn model list")
    );
    println!(
        "  {}  Download a model",
        ds::command("spn model pull <name>")
    );
    println!(
        "  {}  Load into memory",
        ds::command("spn model load <name>")
    );
    println!(
        "  {}    Free memory",
        ds::command("spn model unload <name>")
    );
    println!(
        "  {}    Remove model",
        ds::command("spn model remove <name>")
    );
    println!(
        "  {}  Search available",
        ds::command("spn model search <q>")
    );
    println!();
    println!("{}", ds::highlight("Storage:"));
    println!();
    println!("  Models are stored in ~/.ollama/models/");
    println!("  Typical sizes: 1GB - 40GB depending on model");
    println!();
    println!("{}", ds::highlight("Integration with Nika:"));
    println!();
    println!("  Nika workflows can use local models via the ollama provider:");
    println!("  {}", ds::muted("provider: ollama/llama3.2"));
}

fn show_providers_help() {
    println!("{}", ds::primary("API Key Management"));
    println!("{}", ds::primary("=================="));
    println!();
    println!("Securely manage API keys for LLM providers and MCP tools.");
    println!();
    println!("{}", ds::highlight("Security:"));
    println!();
    println!("  Keys are stored in your OS keychain (macOS/Windows/Linux).");
    println!("  Protected by your system login credentials.");
    println!("  Never stored in plain text or environment variables.");
    println!();
    println!("{}", ds::highlight("Supported Providers:"));
    println!();
    println!("  {}", ds::muted("LLM Providers"));
    println!("    {}    Claude API", ds::primary("anthropic"));
    println!("    {}       GPT-4, etc.", ds::primary("openai"));
    println!("    {}      Mistral AI", ds::primary("mistral"));
    println!("    {}         Groq (fast)", ds::primary("groq"));
    println!("    {}     DeepSeek", ds::primary("deepseek"));
    println!("    {}       Gemini", ds::primary("gemini"));
    println!();
    println!("  {}", ds::muted("MCP Tools"));
    println!("    {}        Graph database", ds::primary("neo4j"));
    println!("    {}       GitHub API", ds::primary("github"));
    println!("    {}   Perplexity search", ds::primary("perplexity"));
    println!("    {}    Firecrawl scraping", ds::primary("firecrawl"));
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!(
        "  {}         List all keys (masked)",
        ds::command("spn provider list")
    );
    println!("  {}  Set a key", ds::command("spn provider set <name>"));
    println!("  {}  Get a key", ds::command("spn provider get <name>"));
    println!(
        "  {}  Remove key",
        ds::command("spn provider delete <name>")
    );
    println!(
        "  {}  Validate format",
        ds::command("spn provider test <name>")
    );
    println!(
        "  {}       Move env vars to keychain",
        ds::command("spn provider migrate")
    );
    println!();
    println!("{}", ds::highlight("Key Resolution Priority:"));
    println!();
    println!("  1. OS Keychain (most secure)");
    println!("  2. Environment variable");
    println!("  3. .env file (least secure)");
}

fn show_daemon_help() {
    println!("{}", ds::primary("Background Daemon"));
    println!("{}", ds::primary("================="));
    println!();
    println!("The spn daemon is a background service that manages:");
    println!();
    println!("  - Secure credential caching (avoids keychain popups)");
    println!("  - Model lifecycle management");
    println!("  - IPC communication with Nika and MCP servers");
    println!();
    println!("{}", ds::highlight("Architecture:"));
    println!();
    println!("  ┌──────────────────────────────────────────┐");
    println!(
        "  │ {} (single keychain accessor)    │",
        ds::command("spn daemon")
    );
    println!("  └────────────────┬─────────────────────────┘");
    println!("                   │ Unix socket");
    println!("       ┌───────────┼───────────┐");
    println!("       ▼           ▼           ▼");
    println!("    ┌─────┐    ┌──────┐    ┌─────┐");
    println!("    │Nika │    │ MCP  │    │ spn │");
    println!("    └─────┘    └──────┘    └─────┘");
    println!();
    println!("{}", ds::highlight("Why a daemon?"));
    println!();
    println!("  Without daemon: Each process accessing keychain triggers");
    println!("  a system popup asking for permission. Annoying!");
    println!();
    println!("  With daemon: One auth prompt at daemon start, then silence.");
    println!("  All processes talk to daemon via Unix socket.");
    println!();
    println!("{}", ds::highlight("Commands:"));
    println!();
    println!(
        "  {}          Start daemon",
        ds::command("spn daemon start")
    );
    println!("  {}           Stop daemon", ds::command("spn daemon stop"));
    println!(
        "  {}         Check status",
        ds::command("spn daemon status")
    );
    println!("  {}        Restart", ds::command("spn daemon restart"));
    println!(
        "  {}        Install as system service",
        ds::command("spn daemon install")
    );
    println!(
        "  {}      Remove service",
        ds::command("spn daemon uninstall")
    );
    println!();
    println!("{}", ds::highlight("Files:"));
    println!();
    println!("  Socket: ~/.spn/daemon.sock");
    println!("  PID:    ~/.spn/daemon.pid");
    println!("  Logs:   Via system logging (journalctl/Console.app)");
}

fn show_architecture_help() {
    println!("{}", ds::primary("SuperNovae Architecture"));
    println!("{}", ds::primary("======================="));
    println!();
    println!("spn is the unified entry point for the SuperNovae AI toolkit.");
    println!();
    println!("{}", ds::highlight("System Overview:"));
    println!();
    println!("                    ┌─────────────────────────────────┐");
    println!(
        "                    │              {}                │",
        ds::primary("spn")
    );
    println!("                    │    (Package Manager + CLI)      │");
    println!("                    └────────────┬────────────────────┘");
    println!("                                 │");
    println!("          ┌──────────────────────┼──────────────────────┐");
    println!("          │                      │                      │");
    println!("          ▼                      ▼                      ▼");
    println!("    ┌──────────┐          ┌──────────┐          ┌──────────┐");
    println!(
        "    │   {}   │          │ {} │          │  {}  │",
        ds::primary("nika"),
        ds::primary("novanet"),
        ds::primary("ollama")
    );
    println!("    │ (Engine) │          │ (Brain)  │          │ (Models) │");
    println!("    └──────────┘          └──────────┘          └──────────┘");
    println!();
    println!("{}", ds::highlight("Components:"));
    println!();
    println!("  {} SuperNovae Package Manager", ds::primary("spn"));
    println!("      Packages, secrets, sync, daemon");
    println!();
    println!("  {} Workflow Engine", ds::primary("nika"));
    println!("      YAML workflows, 5 verbs, DAG execution");
    println!("      Access: spn nk <command>");
    println!();
    println!("  {} Knowledge Graph", ds::primary("novanet"));
    println!("      Neo4j-based, 61 node types, MCP server");
    println!("      Access: spn nv <command>");
    println!();
    println!("  {} Local Models", ds::primary("ollama"));
    println!("      LLM inference, model management");
    println!("      Access: spn model <command>");
    println!();
    println!("{}", ds::highlight("Communication:"));
    println!();
    println!("  spn → nika      Binary proxy (spn nk → nika)");
    println!("  spn → novanet   Binary proxy (spn nv → novanet)");
    println!("  spn → ollama    IPC via daemon");
    println!("  nika → novanet  MCP protocol (invoke: novanet_*)");
    println!();
    println!("{}", ds::highlight("Storage Locations:"));
    println!();
    println!("  ~/.spn/          spn config, packages, daemon");
    println!("  ~/.claude/       Claude Code skills, settings");
    println!("  ~/.ollama/       Local LLM models");
    println!("  ~/.nika/         Nika config, traces");
    println!();
    println!("{}", ds::highlight("Learn More:"));
    println!();
    println!(
        "  {}              System health check",
        ds::command("spn doctor")
    );
    println!(
        "  {}       Detailed status",
        ds::command("spn status --json")
    );
    println!("  {}    Interactive onboarding", ds::command("spn setup"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_help_no_topic() {
        let result = run(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_config() {
        let result = run(Some("config")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_models() {
        let result = run(Some("models")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_providers() {
        let result = run(Some("providers")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_daemon() {
        let result = run(Some("daemon")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_architecture() {
        let result = run(Some("architecture")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_arch_alias() {
        let result = run(Some("arch")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_help_unknown_topic() {
        let result = run(Some("unknown-topic")).await;
        assert!(result.is_ok());
    }
}
