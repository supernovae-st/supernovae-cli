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
mod error;
mod index;
mod interop;
mod manifest;
mod storage;
mod sync;

use error::Result;

/// SuperNovae CLI - AI package manager
#[derive(Parser)]
#[command(name = "spn")]
#[command(author = "SuperNovae Studio")]
#[command(version)]
#[command(about = "Package manager for AI workflows, schemas, skills, and MCP servers")]
#[command(long_about = None)]
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

    /// Skill commands (via skills.sh)
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },

    /// MCP server commands (via npm)
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
    },

    /// Configuration commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Schema commands (for NovaNet)
    Schema {
        #[command(subcommand)]
        command: SchemaCommands,
    },

    /// System diagnostic
    Doctor,

    /// Initialize a new project
    Init {
        /// Create local config template
        #[arg(long)]
        local: bool,

        /// Create MCP config template
        #[arg(long)]
        mcp: bool,
    },

    /// Show detailed help for a topic (config, scopes, mcp, sync, workflows, registry)
    Topic {
        /// Topic name
        name: Option<String>,
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
    Remove { name: String },
    /// List installed skills
    List,
    /// Search skills on skills.sh
    Search { query: String },
}

#[derive(Subcommand)]
enum McpCommands {
    /// Add an MCP server from npm
    Add {
        /// Server alias or npm package
        name: String,
    },
    /// Remove an MCP server
    Remove { name: String },
    /// List installed MCP servers
    List,
    /// Test MCP server connection
    Test { name: String },
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
        /// Layer
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
        /// Add a property
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
        /// Config section to show
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
        } => commands::sync::run(enable, disable, status, target, dry_run).await,
        Commands::Config { command } => commands::config::run(command).await,
        Commands::Schema { command } => commands::schema::run(command).await,
        Commands::Doctor => commands::doctor::run().await,
        Commands::Init { local, mcp } => commands::init::run(local, mcp).await,
        Commands::Topic { name } => commands::help::run(name.as_deref()).await,
    }
}
