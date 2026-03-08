//! SuperNovae CLI - Package manager for AI workflows, schemas, skills, and MCP servers.
//!
//! # Usage
//!
//! ```bash
//! spn add @nika/generate-page      # Add workflow
//! spn skill add brainstorming      # Add skill (via skills.sh)
//! spn mcp add neo4j                # Add MCP server (via npm)
//! spn sync                         # Sync to editors
//! spn doctor                       # System diagnostic
//! ```

use clap::builder::{styling::AnsiColor, Styles};
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod daemon;
mod diff;
mod error;
mod first_run;
mod index;
mod interop;
mod manifest;
mod mcp;
mod prompts;
mod secrets;
mod status;
mod storage;
mod suggest;
mod sync;
mod tui;
mod ux;
mod welcome;

use crate::ux::design_system as ds;

// ============================================================================
// CLI STYLES - Colorized help for v0.14.0
// ============================================================================

/// Get the CLI color styles for consistent, readable help output
fn cli_styles() -> Styles {
    Styles::styled()
        // Headers: bright green (matches success indicators)
        .header(AnsiColor::BrightGreen.on_default().bold())
        .usage(AnsiColor::BrightGreen.on_default().bold())
        // Commands/literals: cyan (matches URLs and command hints)
        .literal(AnsiColor::BrightCyan.on_default())
        // Placeholders: yellow (stands out, indicates user input needed)
        .placeholder(AnsiColor::Yellow.on_default())
        // Errors: red (standard)
        .error(AnsiColor::BrightRed.on_default().bold())
        // Valid values: green
        .valid(AnsiColor::BrightGreen.on_default())
        // Invalid values: red
        .invalid(AnsiColor::BrightRed.on_default())
}

/// SuperNovae CLI - AI Development Toolkit
#[derive(Parser)]
#[command(name = "spn")]
#[command(author = "SuperNovae Studio")]
#[command(version)]
#[command(styles = cli_styles())]
#[command(about = "AI Development Toolkit: packages, secrets, and sync for Claude Code & Nika")]
#[command(long_about = r#"
spn - SuperNovae Package Manager

Your AI development toolkit providing:

  📦 PACKAGES    Install AI workflows, schemas, skills, and MCP servers
  🔐 SECRETS     Securely manage API keys for LLM providers and MCP tools
  🔄 SYNC        Sync packages to Claude Code, VS Code, and other editors

QUICK START:
  spn setup              Interactive onboarding wizard
  spn provider set       Set up an API key
  spn mcp add neo4j      Add an MCP server
  spn sync               Sync to your editor

LEARN MORE:
  spn topic              Browse detailed guides
  spn doctor             System health check
  https://spn.supernovae.studio/docs
"#)]
#[command(
    after_help = "Run 'spn setup' for interactive onboarding, or 'spn topic' for detailed guides."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output (-v = info, -vv = debug, -vvv = trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a package to the project
    #[command(after_help = "Related: spn remove, spn list, spn search, spn info")]
    Add {
        /// Package name (e.g., @nika/generate-page)
        package: String,

        /// Package type (workflow, schema, job)
        #[arg(short, long)]
        r#type: Option<String>,
    },

    /// Remove a package from the project
    Remove {
        /// Package name
        package: String,
    },

    /// Install packages from spn.yaml
    Install {
        /// Use exact versions from spn.lock
        #[arg(long)]
        frozen: bool,
    },

    /// Update packages to latest compatible versions
    Update {
        /// Specific package to update
        package: Option<String>,
    },

    /// List outdated packages
    Outdated,

    /// Search packages in the registry
    Search {
        /// Search query
        query: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show package information
    Info {
        /// Package name
        package: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List installed packages
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Publish current package to registry
    Publish {
        /// Simulate without publishing
        #[arg(long)]
        dry_run: bool,
    },

    /// Bump package version
    Version {
        /// Version bump type (major, minor, patch)
        bump: String,
    },

    /// Manage skills from the skills.sh ecosystem
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },

    /// Manage MCP servers
    #[command(visible_alias = "mc")]
    #[command(after_help = "Related: spn sync, spn provider list, spn doctor")]
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },

    /// Nika integration commands
    Nk {
        #[command(subcommand)]
        command: NikaCommands,
    },

    /// NovaNet integration commands
    Nv {
        #[command(subcommand)]
        command: NovaNetCommands,
    },

    /// Sync packages to editor configs
    #[command(visible_alias = "s")]
    Sync {
        /// Enable sync for an editor
        #[arg(long)]
        enable: Option<String>,

        /// Disable sync for an editor
        #[arg(long)]
        disable: Option<String>,

        /// Show sync status
        #[arg(long)]
        status: bool,

        /// Specific target(s) to sync
        #[arg(long)]
        target: Option<String>,

        /// Show diff without modifying
        #[arg(long)]
        dry_run: bool,

        /// Show interactive diff and ask for confirmation
        #[arg(short, long)]
        interactive: bool,
    },

    /// Configuration commands
    #[command(visible_alias = "c")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Manage NovaNet schema
    Schema {
        #[command(subcommand)]
        command: SchemaCommands,
    },

    /// System diagnostic
    #[command(visible_alias = "dr")]
    Doctor,

    /// Show a guided tour of spn features
    Tour,

    /// Manage API keys and secrets for providers
    #[command(visible_alias = "p")]
    #[command(after_help = "Related: spn daemon start, spn secrets doctor, spn mcp add")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },

    /// Show ecosystem status
    #[command(visible_alias = "st")]
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Interactive TUI for exploring resources
    #[command(visible_alias = "ex")]
    Explore,

    /// Get context-aware suggestions
    #[command(visible_alias = "sg")]
    Suggest,

    /// Initialize a new project
    Init {
        /// Create local config template
        #[arg(long)]
        local: bool,

        /// Create MCP config template
        #[arg(long)]
        mcp: bool,

        /// Initialize from a template (nika, novanet)
        #[arg(long)]
        template: Option<String>,
    },

    /// Show detailed help for a topic
    Topic {
        /// Topic name (config, scopes, mcp, sync, workflows, registry)
        name: Option<String>,
    },

    /// Secrets management and diagnostics
    Secrets {
        #[command(subcommand)]
        command: SecretsCommands,
    },

    /// Interactive onboarding wizard for first-time setup
    Setup {
        /// Quick setup: auto-detect and migrate keys
        #[arg(long)]
        quick: bool,

        /// Specific setup command (nika, novanet)
        #[command(subcommand)]
        command: Option<SetupCommands>,
    },

    /// Daemon commands for background service
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },

    /// Manage local LLM models (Ollama)
    #[command(visible_alias = "m")]
    #[command(
        after_help = "Requires: Ollama must be running (ollama serve)\nRelated: spn provider list, spn nk studio"
    )]
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },

    /// Generate shell completions
    #[command(visible_alias = "comp")]
    #[command(
        after_help = "Examples:\n  spn completion install            # Auto-detect and install\n  spn completion install --shell zsh\n  spn completion bash >> ~/.bashrc  # Manual generation\n  spn completion status             # Check installation"
    )]
    Completion {
        #[command(subcommand)]
        command: CompletionCommands,
    },
}

#[derive(Subcommand)]
pub enum CompletionCommands {
    /// Generate bash completions to stdout
    Bash {
        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Generate zsh completions to stdout
    Zsh {
        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Generate fish completions to stdout
    Fish {
        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Generate PowerShell completions to stdout
    #[command(visible_alias = "pwsh")]
    PowerShell {
        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Generate elvish completions to stdout
    Elvish {
        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Install completions to shell config file
    Install {
        /// Target shell (auto-detect if omitted)
        #[arg(short, long)]
        shell: Option<String>,
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove installed completions
    Uninstall {
        /// Target shell (auto-detect if omitted)
        #[arg(short, long)]
        shell: Option<String>,
    },
    /// Show completion installation status
    Status,
}

#[derive(Subcommand)]
enum SkillCommands {
    /// Add a skill from skills.sh
    Add {
        /// Skill name (e.g., brainstorming, superpowers/tdd)
        name: String,
    },
    /// Remove a skill
    Remove {
        /// Skill name to remove
        name: String,
    },
    /// List installed skills
    List,
    /// Search skills on skills.sh
    Search {
        /// Search query (e.g., "tdd", "git")
        query: String,
    },
}

#[derive(Subcommand)]
enum McpCommands {
    /// Add an MCP server from npm
    #[command(visible_alias = "a")]
    Add {
        /// Server alias or npm package
        name: String,

        /// Install to global config (~/.spn/mcp.yaml)
        #[arg(short, long)]
        global: bool,

        /// Install to project config (.spn/mcp.yaml)
        #[arg(short, long)]
        project: bool,

        /// Skip syncing to editors
        #[arg(long)]
        no_sync: bool,

        /// Sync only to specific editors (comma-separated)
        #[arg(long)]
        sync_to: Option<String>,
    },
    /// Remove an MCP server
    #[command(visible_alias = "rm")]
    Remove {
        /// Server name
        name: String,

        /// Remove from global config
        #[arg(short, long)]
        global: bool,

        /// Remove from project config
        #[arg(short, long)]
        project: bool,
    },
    /// List installed MCP servers
    #[command(visible_alias = "l", visible_alias = "ls")]
    List {
        /// Show only global servers
        #[arg(short, long)]
        global: bool,

        /// Show only project servers
        #[arg(short, long)]
        project: bool,

        /// Show as JSON
        #[arg(long)]
        json: bool,
    },
    /// Test MCP server connection
    Test {
        /// Server name (or "all" to test all)
        name: String,
    },
    /// View MCP server logs
    Logs {
        /// Server name (or "all" for all servers)
        name: Option<String>,

        /// Follow log output (like tail -f)
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,

        /// Filter by log level (debug, info, warn, error)
        #[arg(short, long)]
        level: Option<String>,
    },
    /// Start dynamic REST-to-MCP server (loads from ~/.spn/apis/)
    #[command(visible_alias = "s")]
    Serve {
        /// Only load specific API config
        #[arg(long)]
        api: Option<String>,
    },
    /// Manage REST API wrapper configurations
    Apis {
        #[command(subcommand)]
        command: ApisCommands,
    },
}

#[derive(Subcommand)]
pub enum ApisCommands {
    /// List configured REST API wrappers
    #[command(visible_alias = "l", visible_alias = "ls")]
    List {
        /// Show as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate an API configuration
    Validate {
        /// API name to validate
        name: String,
    },
    /// Show API configuration details
    Info {
        /// API name
        name: String,
    },
}

#[derive(Subcommand)]
enum NikaCommands {
    /// Run a workflow
    Run {
        /// Workflow file
        file: String,
    },
    /// Validate a workflow
    Check {
        /// Workflow file
        file: String,
    },
    /// Open Nika Studio TUI
    Studio,
    /// Job commands
    Jobs {
        #[command(subcommand)]
        command: JobCommands,
    },
    /// Create a new workflow from template
    New {
        /// Workflow name
        name: String,
        /// Template to use (default: minimal)
        #[arg(long, short, default_value = "minimal")]
        template: String,
    },
    /// Manage execution traces
    Trace {
        #[command(subcommand)]
        command: TraceCommands,
    },
    /// Nika configuration
    Config {
        #[command(subcommand)]
        command: NikaConfigCommands,
    },
}

#[derive(Subcommand)]
enum JobCommands {
    /// Start the jobs daemon
    Start,
    /// Show jobs status
    Status,
    /// Stop the jobs daemon
    Stop,
}

#[derive(Subcommand)]
enum TraceCommands {
    /// List recent traces
    List {
        /// Number of traces to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show trace details
    Show {
        /// Trace ID
        id: String,
    },
    /// Clean old traces
    Clean {
        /// Keep traces newer than this (e.g., "7d", "24h")
        #[arg(long, default_value = "7d")]
        keep: String,
    },
}

#[derive(Subcommand)]
enum NikaConfigCommands {
    /// List all configuration values
    List,
    /// Get a config value
    Get {
        /// Config key
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
}

#[derive(Subcommand)]
enum NovaNetCommands {
    /// Open NovaNet TUI
    Tui,
    /// Query the knowledge graph
    Query {
        /// Query string
        query: String,
    },
    /// Start MCP server
    Mcp {
        #[command(subcommand)]
        command: Option<McpServerCommands>,
    },
    /// Add a new node type
    AddNode {
        /// Node name
        name: String,
        /// Realm (shared, org)
        #[arg(long)]
        realm: String,
        /// Layer (e.g., config, semantic, foundation, structure, output)
        #[arg(long)]
        layer: String,
    },
    /// Add a new arc type
    AddArc {
        /// Arc name
        name: String,
        /// Source node
        #[arg(long)]
        from: String,
        /// Target node
        #[arg(long)]
        to: String,
    },
    /// Override an existing node
    Override {
        /// Node name to override
        name: String,
        /// Add a property (format: "name:type", e.g., "status:string")
        #[arg(long)]
        add_property: Option<String>,
    },
    /// Database commands
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    /// Search nodes in the knowledge graph
    Search {
        /// Search query
        query: String,
        /// Filter by node kind
        #[arg(long, short)]
        kind: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Entity operations
    Entity {
        #[command(subcommand)]
        command: EntityCommands,
    },
    /// Export subgraph to file
    Export {
        /// Output file path
        #[arg(short, long)]
        output: String,
        /// Export format (cypher, json, yaml)
        #[arg(long, default_value = "cypher")]
        format: String,
        /// Entity key to export from
        #[arg(long)]
        entity: Option<String>,
    },
    /// Locale operations
    Locale {
        #[command(subcommand)]
        command: LocaleCommands,
    },
    /// Knowledge generation commands
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommands,
    },
    /// Show graph statistics
    Stats {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show schema vs database drift
    Diff {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Generate documentation
    Doc {
        /// Output directory
        #[arg(short, long, default_value = "docs/generated")]
        output: String,
        /// Documentation format (markdown, html)
        #[arg(long, default_value = "markdown")]
        format: String,
    },
}

#[derive(Subcommand)]
enum McpServerCommands {
    /// Start the MCP server
    Start,
    /// Stop the MCP server
    Stop,
}

#[derive(Subcommand)]
enum DbCommands {
    /// Seed database
    Seed,
    /// Run migrations
    Migrate,
    /// Reset database (drop + seed)
    Reset,
    /// Verify YAML↔Neo4j arc consistency
    Verify,
}

#[derive(Subcommand)]
enum EntityCommands {
    /// List entities
    List {
        /// Filter by category
        #[arg(long, short)]
        category: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show entity details
    Show {
        /// Entity key
        key: String,
        /// Include native content
        #[arg(long)]
        with_native: bool,
    },
    /// Generate entity content for a locale
    Generate {
        /// Entity key
        key: String,
        /// Target locale
        #[arg(long)]
        locale: String,
    },
}

#[derive(Subcommand)]
enum LocaleCommands {
    /// List available locales
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show locale details
    Show {
        /// Locale code (e.g., fr-FR)
        code: String,
    },
    /// Show locale coverage
    Coverage {
        /// Locale code
        locale: String,
    },
}

#[derive(Subcommand)]
enum KnowledgeCommands {
    /// Generate knowledge atoms
    Generate {
        /// Entity key
        entity: String,
        /// Target locale
        #[arg(long)]
        locale: String,
    },
    /// List knowledge atoms
    List {
        /// Filter by locale
        #[arg(long)]
        locale: Option<String>,
        /// Filter by type (term, expression, pattern)
        #[arg(long, short)]
        r#type: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show resolved configuration
    Show {
        /// Config section to show (providers, paths, sync, ui)
        section: Option<String>,
    },
    /// Show config file locations
    Where,
    /// List config with origins
    List {
        /// Show which file defined each value
        #[arg(long)]
        show_origin: bool,
    },
    /// Get a configuration value
    Get {
        /// Config key (e.g., providers.anthropic.model)
        key: String,
        /// Show which scope defined this value
        #[arg(long)]
        show_origin: bool,
    },
    /// Set a configuration value
    Set {
        /// Config key (e.g., providers.anthropic.model)
        key: String,
        /// Value to set
        value: String,
        /// Scope to set value in (global, team, local)
        #[arg(long, default_value = "global")]
        scope: String,
    },
    /// Edit configuration
    Edit {
        /// Edit local config
        #[arg(long)]
        local: bool,
        /// Edit user config
        #[arg(long)]
        user: bool,
        /// Edit MCP config
        #[arg(long)]
        mcp: bool,
    },
    /// Import configuration from editor config file
    Import {
        /// Path to editor config file (e.g., .claude/settings.json)
        file: String,
        /// Scope to import into (global, team, local)
        #[arg(long, default_value = "team")]
        scope: String,
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum SchemaCommands {
    /// Show schema statistics (JSON output)
    #[command(alias = "status")]
    Stats,
    /// Validate YAML ↔ Neo4j sync
    Validate,
    /// Generate all schema artifacts (TypeScript, Cypher, etc.)
    Generate,
    /// Validate Cypher seed files
    CypherValidate,
}

#[derive(Subcommand)]
enum SecretsCommands {
    /// Run health checks on secrets configuration
    Doctor {
        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,
    },
    /// Export secrets to encrypted file (SOPS format)
    Export {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
        /// Export as plaintext (DANGEROUS)
        #[arg(long)]
        plaintext: bool,
    },
    /// Import secrets from encrypted file
    Import {
        /// Input file path
        file: String,
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum DaemonCommands {
    /// Start the daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Restart the daemon
    Restart,
    /// Install daemon as a system service (auto-start at login)
    Install,
    /// Uninstall daemon system service
    Uninstall,
    /// Run as MCP server over stdio
    #[command(visible_alias = "mcp-server")]
    Mcp,
}

#[derive(Subcommand)]
pub enum ModelCommands {
    /// List installed models
    #[command(visible_alias = "l", visible_alias = "ls")]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Only show loaded models
        #[arg(long)]
        running: bool,
    },

    /// Pull/download a model from Ollama registry
    #[command(visible_alias = "get", visible_alias = "download")]
    Pull {
        /// Model name (e.g., llama3.2:7b, mistral:latest)
        name: String,
    },

    /// Load a model into memory
    Load {
        /// Model name
        name: String,

        /// Keep model loaded indefinitely
        #[arg(long)]
        keep_alive: bool,
    },

    /// Unload a model from memory
    Unload {
        /// Model name
        name: String,
    },

    /// Delete a model
    #[command(visible_alias = "rm")]
    Delete {
        /// Model name
        name: String,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// Show running models and resource usage
    #[command(visible_alias = "ps")]
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Search available models in registry
    Search {
        /// Search query (e.g., "coding", "vision", "reasoning")
        query: String,

        /// Filter by category (code, chat, embed, vision, reasoning)
        #[arg(long, short)]
        category: Option<String>,
    },

    /// Show detailed info about a model
    Info {
        /// Model name (e.g., deepseek-coder, llama3.2)
        name: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Get model recommendations for a use case
    Recommend {
        /// Use case (e.g., "coding", "chat", "embeddings", "vision")
        #[arg(value_name = "USE_CASE")]
        use_case: Option<String>,
    },

    /// Run inference on a model
    #[command(visible_alias = "r")]
    Run {
        /// Model name (e.g., llama3.2, mistral:7b)
        model: String,

        /// Prompt text (use - for stdin, @file for file input)
        prompt: String,

        /// Stream output tokens as they arrive
        #[arg(long)]
        stream: bool,

        /// Temperature (0.0 - 2.0)
        #[arg(long, short = 't', default_value = "0.7")]
        temperature: f32,

        /// System prompt
        #[arg(long, short = 's')]
        system: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum SetupCommands {
    /// Install and configure Nika workflow engine
    Nika {
        /// Skip editor sync after installation
        #[arg(long)]
        no_sync: bool,

        /// Skip LSP server installation
        #[arg(long)]
        no_lsp: bool,

        /// Installation method: cargo, brew, or auto
        #[arg(long, default_value = "auto")]
        method: String,
    },

    /// Install and configure NovaNet knowledge graph
    Novanet {
        /// Skip configuration sync
        #[arg(long)]
        no_sync: bool,
    },

    /// Install SuperNovae Claude Code plugin
    #[command(name = "claude-code")]
    ClaudeCode {
        /// Force reinstall even if plugin is already installed
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ProviderCommands {
    /// List all stored API keys (masked)
    #[command(visible_alias = "l", visible_alias = "ls")]
    List {
        /// Show key sources (keychain, env, .env)
        #[arg(long)]
        show_source: bool,
    },
    /// Set API key for a provider
    #[command(visible_alias = "add")]
    Set {
        /// Provider name (anthropic, openai, gemini, etc.)
        provider: String,
        /// API key value (prompts securely if omitted)
        #[arg(long)]
        key: Option<String>,
        /// Storage backend: keychain (default), env, global, shell
        #[arg(long, short = 's')]
        storage: Option<String>,
    },
    /// Get masked API key for a provider
    Get {
        /// Provider name
        provider: String,
        /// Show full key (DANGEROUS - only for scripts)
        #[arg(long)]
        unmask: bool,
    },
    /// Delete API key for a provider
    Delete {
        /// Provider name
        provider: String,
    },
    /// Migrate keys from env vars to OS keychain
    Migrate {
        /// Don't prompt, just migrate
        #[arg(long)]
        yes: bool,
    },
    /// Test provider connection
    Test {
        /// Provider name (or "all")
        provider: String,
    },
    /// Show full diagnostic status of all secrets and providers
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

// ============================================================================
// FIRST-RUN EXPERIENCE
// ============================================================================

/// Handle the first-run welcome experience
///
/// Shows the welcome screen and handles user choices:
/// - QuickSetup: Runs the setup wizard
/// - TakeTour: Shows feature tour, then loops back
/// - ShowHelp: Shows --help and marks complete
/// - SkipForever: Marks complete and exits
async fn handle_first_run() -> error::Result<()> {
    use welcome::WelcomeAction;

    loop {
        match welcome::show()? {
            WelcomeAction::QuickSetup => {
                // Run the setup wizard
                println!();
                println!("  {} Starting setup...", ds::primary("→"));
                println!();

                // Run setup command
                let result = commands::setup::run(None, false).await;
                if result.is_ok() {
                    welcome::show_setup_complete();
                }
                first_run::mark_complete()?;
                return Ok(());
            }
            WelcomeAction::TakeTour => {
                welcome::show_tour();
                // After tour, loop back to show welcome again
                // User can then choose setup or skip
                continue;
            }
            WelcomeAction::ShowHelp => {
                // Show help and mark as complete
                first_run::mark_complete()?;
                // Re-invoke with --help
                let _ = Cli::try_parse_from(["spn", "--help"]);
                return Ok(());
            }
            WelcomeAction::SkipForever => {
                // Mark as complete and exit
                first_run::mark_complete()?;
                println!();
                println!(
                    "  {} Run {} anytime to get started.",
                    ds::success("✓"),
                    ds::primary("spn setup")
                );
                println!();
                return Ok(());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Check for verbose flag early (before full CLI parsing) to set correct log level
    // This allows debug output during first-run and error handling
    let verbose_count = std::env::args()
        .filter(|arg| arg == "-v" || arg == "--verbose")
        .count();

    // Initialize logging with appropriate level based on verbose flags
    // -v = info, -v -v = debug, -v -v -v = trace
    let log_level = match verbose_count {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(format!("spn={}", log_level))
        .init();

    // Check for first-run experience BEFORE parsing CLI
    // This allows us to show welcome screen when user just types "spn"
    // Use count() instead of collect() to avoid unnecessary allocation
    let has_no_args = std::env::args().count() == 1;
    if has_no_args && first_run::is_first_run() {
        // No subcommand provided and it's first run - show welcome
        if let Err(e) = handle_first_run().await {
            // Use SpnError::print() for consistent error formatting with help
            e.print();
            std::process::exit(1);
        }
        return;
    }

    // Try to parse CLI args, with "did you mean?" suggestions on error
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Check if this is a missing subcommand (user just ran "spn")
            if e.kind() == clap::error::ErrorKind::MissingSubcommand {
                // Not first run but no command - show brief help
                println!();
                println!(
                    "  {} {}",
                    ds::primary("spn"),
                    ds::muted(format!("v{}", env!("CARGO_PKG_VERSION")))
                );
                println!();
                println!("  Run {} to see all commands", ds::primary("spn --help"));
                println!("  Run {} to learn about features", ds::primary("spn topic"));
                println!();
                std::process::exit(0);
            }

            // Check if this is an unrecognized subcommand error
            let error_str = e.to_string();
            if error_str.contains("unrecognized subcommand") {
                // Extract the invalid command from the error
                if let Some(start) = error_str.find('\'') {
                    if let Some(end) = error_str[start + 1..].find('\'') {
                        let invalid_cmd = &error_str[start + 1..start + 1 + end];
                        suggest::print_suggestion(invalid_cmd);
                    }
                }
            }
            e.exit();
        }
    };

    // Verbose flag was already processed during early logging initialization
    // (tracing only allows one global subscriber, so we check args before parsing)

    let result = match cli.command {
        Commands::Add { package, r#type } => commands::add::run(&package, r#type.as_deref()).await,
        Commands::Remove { package } => commands::remove::run(&package).await,
        Commands::Install { frozen } => commands::install::run(frozen).await,
        Commands::Update { package } => commands::update::run(package.as_deref()).await,
        Commands::Outdated => commands::outdated::run().await,
        Commands::Search { query, json } => commands::search::run(&query, json).await,
        Commands::Info { package, json } => commands::info::run(&package, json).await,
        Commands::List { json } => commands::list::run(json).await,
        Commands::Publish { dry_run } => commands::publish::run(dry_run).await,
        Commands::Version { bump } => commands::version::run(&bump).await,
        Commands::Skill { command } => commands::skill::run(command).await,
        Commands::Mcp { command } => commands::mcp::run(command).await,
        Commands::Nk { command } => commands::nk::run(command).await,
        Commands::Nv { command } => commands::nv::run(command).await,
        Commands::Sync {
            enable,
            disable,
            status,
            target,
            dry_run,
            interactive,
        } => commands::sync::run(enable, disable, status, target, dry_run, interactive).await,
        Commands::Config { command } => commands::config::run(command).await,
        Commands::Schema { command } => commands::schema::run(command).await,
        Commands::Doctor => commands::doctor::run().await,
        Commands::Tour => {
            welcome::show_tour();
            Ok(())
        }
        Commands::Provider { command } => commands::provider::run(command).await,
        Commands::Status { json } => commands::status::run(json).await,
        Commands::Explore => commands::explore::run().await,
        Commands::Suggest => commands::suggest::run().await,
        Commands::Init {
            local,
            mcp,
            template,
        } => commands::init::run(local, mcp, template).await,
        Commands::Topic { name } => commands::help::run(name.as_deref()).await,
        Commands::Secrets { command } => commands::secrets::run(command).await,
        Commands::Setup { quick, command } => commands::setup::run(command, quick).await,
        Commands::Daemon { command } => commands::daemon::run(command).await,
        Commands::Model { command } => commands::model::execute(command).await,
        Commands::Completion { command } => match command {
            CompletionCommands::Bash { output } => commands::completion::run("bash", output).await,
            CompletionCommands::Zsh { output } => commands::completion::run("zsh", output).await,
            CompletionCommands::Fish { output } => commands::completion::run("fish", output).await,
            CompletionCommands::PowerShell { output } => {
                commands::completion::run("powershell", output).await
            }
            CompletionCommands::Elvish { output } => {
                commands::completion::run("elvish", output).await
            }
            CompletionCommands::Install { shell, dry_run } => {
                commands::completion::install(shell.as_deref(), dry_run).await
            }
            CompletionCommands::Uninstall { shell } => {
                commands::completion::uninstall(shell.as_deref()).await
            }
            CompletionCommands::Status => commands::completion::status().await,
        },
    };

    // Handle errors with helpful messages
    if let Err(e) = result {
        e.print();
        std::process::exit(1);
    }
}
