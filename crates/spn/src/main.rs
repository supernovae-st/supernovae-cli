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

// Allow dead code during early development - scaffolded API surface for future use
#![allow(dead_code)]

use clap::{Parser, Subcommand};

mod commands;
mod config;
mod daemon;
mod diff;
mod error;
mod index;
mod interop;
mod manifest;
mod mcp;
mod secrets;
mod storage;
mod sync;

use error::Result;

/// SuperNovae CLI - AI Development Toolkit
#[derive(Parser)]
#[command(name = "spn")]
#[command(author = "SuperNovae Studio")]
#[command(version)]
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

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a package to the project
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
    },

    /// Show package information
    Info {
        /// Package name
        package: String,
    },

    /// List installed packages
    List,

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
    Doctor,

    /// Manage API keys and secrets for providers
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },

    /// Show ecosystem status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

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
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },
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
    /// Start Neo4j database
    Start,
    /// Seed database
    Seed,
    /// Reset and reseed database
    Reset,
    /// Run migrations
    Migrate,
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
    /// Show schema status
    Status,
    /// Validate schema
    Validate,
    /// Resolve and show merged schema
    Resolve,
    /// Show diff vs last resolved
    Diff,
    /// Exclude a node from packages
    Exclude {
        /// Node name to exclude
        name: String,
    },
    /// Re-include an excluded node
    Include {
        /// Node name to include
        name: String,
    },
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
}

#[derive(Subcommand)]
pub enum ModelCommands {
    /// List installed models
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Only show loaded models
        #[arg(long)]
        running: bool,
    },

    /// Pull/download a model from Ollama registry
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
    Delete {
        /// Model name
        name: String,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// Show running models and resource usage
    Status {
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
}

#[derive(Subcommand)]
enum ProviderCommands {
    /// List all stored API keys (masked)
    List {
        /// Show key sources (keychain, env, .env)
        #[arg(long)]
        show_source: bool,
    },
    /// Set API key for a provider
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(if cli.verbose { "spn=debug" } else { "spn=info" })
        .init();

    match cli.command {
        Commands::Add { package, r#type } => commands::add::run(&package, r#type.as_deref()).await,
        Commands::Remove { package } => commands::remove::run(&package).await,
        Commands::Install { frozen } => commands::install::run(frozen).await,
        Commands::Update { package } => commands::update::run(package.as_deref()).await,
        Commands::Outdated => commands::outdated::run().await,
        Commands::Search { query } => commands::search::run(&query).await,
        Commands::Info { package } => commands::info::run(&package).await,
        Commands::List => commands::list::run().await,
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
        Commands::Provider { command } => commands::provider::run(command).await,
        Commands::Status { json } => commands::status::run(json).await,
        Commands::Init {
            local,
            mcp,
            template,
        } => commands::init::run(local, mcp, template).await,
        Commands::Topic { name } => commands::help::run(name.as_deref()).await,
        Commands::Secrets { command } => commands::secrets::run(command).await,
        Commands::Setup { quick, command } => commands::setup::run(command, quick).await,
        Commands::Daemon { command } => commands::daemon::run(command).await,
        Commands::Model { command } => commands::model::run(command).await,
    }
}
