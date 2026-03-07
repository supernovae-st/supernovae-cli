//! Help command implementation.
//!
//! Detailed help topics for SuperNovae CLI.

use crate::error::Result;
use colored::Colorize;

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
            eprintln!("{} Unknown topic: {}", "!".yellow(), t);
            eprintln!();
            show_topics_list();
        }
        None => show_topics_list(),
    }
    Ok(())
}

fn show_topics_list() {
    println!("{}", "SuperNovae Help".cyan().bold());
    println!("{}", "===============".cyan());
    println!();
    println!("{}", "Available topics:".bold());
    println!();
    println!("  {}", "Getting Started".dimmed());
    println!(
        "    {}     Configuration system (spn.yaml, .spn/)",
        "config".cyan()
    );
    println!(
        "    {}     Package scopes (@nika, @novanet, @community)",
        "scopes".cyan()
    );
    println!("    {}   Package registry", "registry".cyan());
    println!();
    println!("  {}", "Core Features".dimmed());
    println!("    {}        MCP server management", "mcp".cyan());
    println!(
        "    {}       IDE sync (Claude Code, Cursor, VS Code)",
        "sync".cyan()
    );
    println!("    {}  Nika workflow packages", "workflows".cyan());
    println!();
    println!("  {}", "Infrastructure".dimmed());
    println!("    {}     Local LLM model management", "models".cyan());
    println!("    {}  API key management", "providers".cyan());
    println!("    {}     Background service", "daemon".cyan());
    println!("    {}       System architecture", "arch".cyan());
    println!();
    println!("Usage: {} <topic>", "spn topic".green());
    println!();
    println!("{}", "Quick Reference:".bold());
    println!();
    println!("  {}              Initialize project", "spn init".dimmed());
    println!("  {}       Add package", "spn add <pkg>".dimmed());
    println!(
        "  {}           Install dependencies",
        "spn install".dimmed()
    );
    println!("  {}              Sync to editors", "spn sync".dimmed());
    println!("  {}            System health check", "spn doctor".dimmed());
}

fn show_config_help() {
    println!("{}", "Configuration".cyan().bold());
    println!("{}", "=============".cyan());
    println!();
    println!(
        "{}",
        "SuperNovae uses a layered configuration system:".bold()
    );
    println!();
    println!("  {} Project-level manifest", "1. spn.yaml".cyan());
    println!("     Lists dependencies for a project");
    println!();
    println!("  {} User-level settings", "2. ~/.spn/config.json".cyan());
    println!("     Registry credentials, sync preferences");
    println!();
    println!("  {} Installed packages", "3. ~/.spn/packages/".cyan());
    println!("     Downloaded and extracted packages");
    println!();
    println!("{}", "spn.yaml Example:".bold());
    println!();
    println!("  {}", "name: my-project".dimmed());
    println!("  {}", "version: 1.0.0".dimmed());
    println!("  {}", "dependencies:".dimmed());
    println!("  {}", "  @nika/seo-audit: ^1.0.0".dimmed());
    println!("  {}", "  @novanet/entity-explorer: ^0.5.0".dimmed());
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!("  {}         Create spn.yaml", "spn init".green());
    println!("  {}  Show config", "spn config show".green());
    println!("  {}   Config file locations", "spn config where".green());
}

fn show_scopes_help() {
    println!("{}", "Package Scopes".cyan().bold());
    println!("{}", "==============".cyan());
    println!();
    println!("Packages are organized by scope (namespace):");
    println!();
    println!("  {}   Official Nika workflow packages", "@nika/*".cyan());
    println!("              Examples: @nika/seo-audit, @nika/content-pipeline");
    println!();
    println!("  {} Official NovaNet schema packages", "@novanet/*".cyan());
    println!("              Examples: @novanet/entity-explorer, @novanet/schema-tools");
    println!();
    println!(
        "  {}    Community-contributed packages",
        "@community/*".cyan()
    );
    println!("              Examples: @community/my-workflow");
    println!();
    println!("{}", "Scope Features:".bold());
    println!();
    println!("  - Scopes are namespaces, not permissions");
    println!("  - Anyone can publish to @community/*");
    println!("  - @nika/* and @novanet/* require maintainer access");
    println!();
    println!("{}", "Index Structure:".bold());
    println!();
    println!("  {}", "index/".dimmed());
    println!("  {}", "├── @nika/".dimmed());
    println!("  {}", "│   └── seo-audit".dimmed());
    println!("  {}", "├── @novanet/".dimmed());
    println!("  {}", "│   └── entity-explorer".dimmed());
    println!("  {}", "└── @community/".dimmed());
    println!("  {}", "    └── user-workflow".dimmed());
}

fn show_mcp_help() {
    println!("{}", "MCP Servers".cyan().bold());
    println!("{}", "===========".cyan());
    println!();
    println!("MCP (Model Context Protocol) servers extend AI capabilities.");
    println!();
    println!("{}", "Built-in Aliases:".bold());
    println!();
    println!("  {}          @neo4j/mcp-server-neo4j", "neo4j".cyan());
    println!(
        "  {}     @anthropic/mcp-server-filesystem",
        "filesystem".cyan()
    );
    println!("  {}         @anthropic/mcp-server-github", "github".cyan());
    println!(
        "  {}       @anthropic/mcp-server-postgres",
        "postgres".cyan()
    );
    println!("  {}         @anthropic/mcp-server-sqlite", "sqlite".cyan());
    println!("  {}         @anthropic/mcp-server-memory", "memory".cyan());
    println!(
        "  {}      @anthropic/mcp-server-puppeteer",
        "puppeteer".cyan()
    );
    println!("  {}          @anthropic/mcp-server-fetch", "fetch".cyan());
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!(
        "  {}          Install neo4j MCP server",
        "spn mcp add neo4j".green()
    );
    println!(
        "  {}       Remove MCP server",
        "spn mcp remove neo4j".green()
    );
    println!(
        "  {}             List installed servers",
        "spn mcp list".green()
    );
    println!(
        "  {}         Test server connection",
        "spn mcp test neo4j".green()
    );
    println!();
    println!("{}", "Integration:".bold());
    println!();
    println!("  MCP servers are installed via npm globally.");
    println!(
        "  Use {} to sync MCP config to your editor.",
        "spn sync".cyan()
    );
}

fn show_sync_help() {
    println!("{}", "IDE Sync".cyan().bold());
    println!("{}", "========".cyan());
    println!();
    println!("Sync packages and MCP servers to IDE configurations.");
    println!();
    println!("{}", "Supported IDEs:".bold());
    println!();
    println!("  {}     .claude/settings.json", "claude-code".cyan());
    println!("  {}         .cursor/settings.json", "cursor".cyan());
    println!("  {}         .vscode/settings.json", "vscode".cyan());
    println!("  {}       .windsurf/settings.json", "windsurf".cyan());
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!(
        "  {}                    Sync all enabled editors",
        "spn sync".green()
    );
    println!(
        "  {}  Enable sync for editor",
        "spn sync --enable cursor".green()
    );
    println!(
        "  {}           Show sync status",
        "spn sync --status".green()
    );
    println!(
        "  {}          Preview changes",
        "spn sync --dry-run".green()
    );
    println!();
    println!("{}", "What Gets Synced:".bold());
    println!();
    println!("  - MCP server configurations (mcpServers section)");
    println!("  - Installed skills from skills.sh");
    println!("  - Package paths for Nika include: resolution");
    println!();
    println!("{}", "Sync Config:".bold());
    println!();
    println!("  Stored in ~/.spn/sync.json");
    println!("  Per-project overrides in .spn/sync.json");
}

fn show_workflows_help() {
    println!("{}", "Nika Workflows".cyan().bold());
    println!("{}", "==============".cyan());
    println!();
    println!("Nika workflows are YAML files defining AI task pipelines.");
    println!();
    println!("{}", "Workflow Structure:".bold());
    println!();
    println!("  {}", "name: my-workflow".dimmed());
    println!("  {}", "description: Example workflow".dimmed());
    println!("  {}", "".dimmed());
    println!("  {}", "tasks:".dimmed());
    println!("  {}", "  - id: fetch-data".dimmed());
    println!("  {}", "    fetch: https://api.example.com/data".dimmed());
    println!("  {}", "    use.ctx: api_data".dimmed());
    println!("  {}", "".dimmed());
    println!("  {}", "  - id: process".dimmed());
    println!("  {}", "    infer: Process this data".dimmed());
    println!("  {}", "    context: $api_data".dimmed());
    println!();
    println!("{}", "Verbs:".bold());
    println!();
    println!("  {}   LLM inference (Claude, GPT, etc.)", "infer:".cyan());
    println!("  {}    Execute shell command", "exec:".cyan());
    println!("  {}   HTTP request", "fetch:".cyan());
    println!("  {}  MCP tool call", "invoke:".cyan());
    println!("  {}   Multi-turn agent loop", "agent:".cyan());
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!("  {}     Run a workflow", "spn nk run file.yaml".green());
    println!("  {}   Validate syntax", "spn nk check file.yaml".green());
    println!("  {}          Open Nika Studio", "spn nk studio".green());
}

fn show_registry_help() {
    println!("{}", "Package Registry".cyan().bold());
    println!("{}", "================".cyan());
    println!();
    println!("SuperNovae uses a sparse index registry (like Cargo).");
    println!();
    println!("{}", "Registry Location:".bold());
    println!();
    println!(
        "  {}",
        "github.com/supernovae-st/supernovae-registry".cyan()
    );
    println!();
    println!("{}", "Registry Structure:".bold());
    println!();
    println!("  {}", "supernovae-registry/".dimmed());
    println!(
        "  {}",
        "├── config.json         # Registry metadata".dimmed()
    );
    println!(
        "  {}",
        "├── index/              # Sparse index (NDJSON)".dimmed()
    );
    println!("  {}", "│   ├── @nika/".dimmed());
    println!("  {}", "│   │   └── seo-audit".dimmed());
    println!("  {}", "│   └── @novanet/".dimmed());
    println!("  {}", "│       └── schema-tools".dimmed());
    println!(
        "  {}",
        "└── releases/           # Package tarballs".dimmed()
    );
    println!("  {}", "    └── @nika/".dimmed());
    println!("  {}", "        └── seo-audit-1.0.0.tar.gz".dimmed());
    println!();
    println!("{}", "Index Format (NDJSON):".bold());
    println!();
    println!(
        "  {}",
        r#"{"name":"seo-audit","version":"1.0.0","checksum":"sha256:..."}"#.dimmed()
    );
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!("  {}      Search packages", "spn search <query>".green());
    println!("  {}          Package details", "spn info <pkg>".green());
    println!("  {}              Publish package", "spn publish".green());
}

fn show_models_help() {
    println!("{}", "Local LLM Models".cyan().bold());
    println!("{}", "================".cyan());
    println!();
    println!("Manage local LLM models via Ollama for offline inference.");
    println!();
    println!("{}", "Requirements:".bold());
    println!();
    println!("  Ollama must be installed: {}", "https://ollama.ai".cyan());
    println!("  The spn daemon must be running for model operations.");
    println!();
    println!("{}", "Popular Models:".bold());
    println!();
    println!("  {}    Fast, efficient (1B params)", "llama3.2:1b".cyan());
    println!("  {}    Balanced (7B params)", "llama3.2:7b".cyan());
    println!("  {}        Coding specialist", "codellama".cyan());
    println!("  {}     Lightweight (2.7B)", "phi3:mini".cyan());
    println!("  {}  Instruction-tuned", "mistral:instruct".cyan());
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!(
        "  {}             List installed models",
        "spn model list".green()
    );
    println!("  {}  Download a model", "spn model pull <name>".green());
    println!("  {}  Load into memory", "spn model load <name>".green());
    println!("  {}    Free memory", "spn model unload <name>".green());
    println!("  {}    Remove model", "spn model delete <name>".green());
    println!("  {}  Search available", "spn model search <q>".green());
    println!();
    println!("{}", "Storage:".bold());
    println!();
    println!("  Models are stored in ~/.ollama/models/");
    println!("  Typical sizes: 1GB - 40GB depending on model");
    println!();
    println!("{}", "Integration with Nika:".bold());
    println!();
    println!("  Nika workflows can use local models via the ollama provider:");
    println!("  {}", "provider: ollama/llama3.2".dimmed());
}

fn show_providers_help() {
    println!("{}", "API Key Management".cyan().bold());
    println!("{}", "==================".cyan());
    println!();
    println!("Securely manage API keys for LLM providers and MCP tools.");
    println!();
    println!("{}", "Security:".bold());
    println!();
    println!("  Keys are stored in your OS keychain (macOS/Windows/Linux).");
    println!("  Protected by your system login credentials.");
    println!("  Never stored in plain text or environment variables.");
    println!();
    println!("{}", "Supported Providers:".bold());
    println!();
    println!("  {}", "LLM Providers".dimmed());
    println!("    {}    Claude API", "anthropic".cyan());
    println!("    {}       GPT-4, etc.", "openai".cyan());
    println!("    {}      Mistral AI", "mistral".cyan());
    println!("    {}         Groq (fast)", "groq".cyan());
    println!("    {}     DeepSeek", "deepseek".cyan());
    println!("    {}       Gemini", "gemini".cyan());
    println!();
    println!("  {}", "MCP Tools".dimmed());
    println!("    {}        Graph database", "neo4j".cyan());
    println!("    {}       GitHub API", "github".cyan());
    println!("    {}   Perplexity search", "perplexity".cyan());
    println!("    {}    Firecrawl scraping", "firecrawl".cyan());
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!(
        "  {}         List all keys (masked)",
        "spn provider list".green()
    );
    println!("  {}  Set a key", "spn provider set <name>".green());
    println!("  {}  Get a key", "spn provider get <name>".green());
    println!("  {}  Remove key", "spn provider delete <name>".green());
    println!("  {}  Validate format", "spn provider test <name>".green());
    println!(
        "  {}       Move env vars to keychain",
        "spn provider migrate".green()
    );
    println!();
    println!("{}", "Key Resolution Priority:".bold());
    println!();
    println!("  1. OS Keychain (most secure)");
    println!("  2. Environment variable");
    println!("  3. .env file (least secure)");
}

fn show_daemon_help() {
    println!("{}", "Background Daemon".cyan().bold());
    println!("{}", "=================".cyan());
    println!();
    println!("The spn daemon is a background service that manages:");
    println!();
    println!("  - Secure credential caching (avoids keychain popups)");
    println!("  - Model lifecycle management");
    println!("  - IPC communication with Nika and MCP servers");
    println!();
    println!("{}", "Architecture:".bold());
    println!();
    println!("  ┌──────────────────────────────────────────┐");
    println!(
        "  │ {} (single keychain accessor)    │",
        "spn daemon".cyan()
    );
    println!("  └────────────────┬─────────────────────────┘");
    println!("                   │ Unix socket");
    println!("       ┌───────────┼───────────┐");
    println!("       ▼           ▼           ▼");
    println!("    ┌─────┐    ┌──────┐    ┌─────┐");
    println!("    │Nika │    │ MCP  │    │ spn │");
    println!("    └─────┘    └──────┘    └─────┘");
    println!();
    println!("{}", "Why a daemon?".bold());
    println!();
    println!("  Without daemon: Each process accessing keychain triggers");
    println!("  a system popup asking for permission. Annoying!");
    println!();
    println!("  With daemon: One auth prompt at daemon start, then silence.");
    println!("  All processes talk to daemon via Unix socket.");
    println!();
    println!("{}", "Commands:".bold());
    println!();
    println!("  {}          Start daemon", "spn daemon start".green());
    println!("  {}           Stop daemon", "spn daemon stop".green());
    println!("  {}         Check status", "spn daemon status".green());
    println!("  {}        Restart", "spn daemon restart".green());
    println!(
        "  {}        Install as system service",
        "spn daemon install".green()
    );
    println!("  {}      Remove service", "spn daemon uninstall".green());
    println!();
    println!("{}", "Files:".bold());
    println!();
    println!("  Socket: ~/.spn/daemon.sock");
    println!("  PID:    ~/.spn/daemon.pid");
    println!("  Logs:   Via system logging (journalctl/Console.app)");
}

fn show_architecture_help() {
    println!("{}", "SuperNovae Architecture".cyan().bold());
    println!("{}", "=======================".cyan());
    println!();
    println!("spn is the unified entry point for the SuperNovae AI toolkit.");
    println!();
    println!("{}", "System Overview:".bold());
    println!();
    println!("                    ┌─────────────────────────────────┐");
    println!(
        "                    │              {}                │",
        "spn".cyan().bold()
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
        "nika".cyan(),
        "novanet".cyan(),
        "ollama".cyan()
    );
    println!("    │ (Engine) │          │ (Brain)  │          │ (Models) │");
    println!("    └──────────┘          └──────────┘          └──────────┘");
    println!();
    println!("{}", "Components:".bold());
    println!();
    println!("  {} SuperNovae Package Manager", "spn".cyan());
    println!("      Packages, secrets, sync, daemon");
    println!();
    println!("  {} Workflow Engine", "nika".cyan());
    println!("      YAML workflows, 5 verbs, DAG execution");
    println!("      Access: spn nk <command>");
    println!();
    println!("  {} Knowledge Graph", "novanet".cyan());
    println!("      Neo4j-based, 61 node types, MCP server");
    println!("      Access: spn nv <command>");
    println!();
    println!("  {} Local Models", "ollama".cyan());
    println!("      LLM inference, model management");
    println!("      Access: spn model <command>");
    println!();
    println!("{}", "Communication:".bold());
    println!();
    println!("  spn → nika      Binary proxy (spn nk → nika)");
    println!("  spn → novanet   Binary proxy (spn nv → novanet)");
    println!("  spn → ollama    IPC via daemon");
    println!("  nika → novanet  MCP protocol (invoke: novanet_*)");
    println!();
    println!("{}", "Storage Locations:".bold());
    println!();
    println!("  ~/.spn/          spn config, packages, daemon");
    println!("  ~/.claude/       Claude Code skills, settings");
    println!("  ~/.ollama/       Local LLM models");
    println!("  ~/.nika/         Nika config, traces");
    println!();
    println!("{}", "Learn More:".bold());
    println!();
    println!(
        "  {}              System health check",
        "spn doctor".green()
    );
    println!("  {}       Detailed status", "spn status --json".green());
    println!("  {}    Interactive onboarding", "spn setup".green());
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
