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
    println!(
        "  {}     Configuration system (spn.yaml, .spn/)",
        "config".cyan()
    );
    println!(
        "  {}     Package scopes (@nika, @novanet, @community)",
        "scopes".cyan()
    );
    println!("  {}        MCP server management", "mcp".cyan());
    println!(
        "  {}       IDE sync (Claude Code, Cursor, VS Code)",
        "sync".cyan()
    );
    println!("  {}  Nika workflow packages", "workflows".cyan());
    println!("  {}   Package registry", "registry".cyan());
    println!();
    println!("Usage: {} <topic>", "spn help".green());
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
    async fn test_help_unknown_topic() {
        let result = run(Some("unknown-topic")).await;
        assert!(result.is_ok());
    }
}
