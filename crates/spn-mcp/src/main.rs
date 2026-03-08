//! spn-mcp: Dynamic REST-to-MCP wrapper
//!
//! Exposes REST APIs as MCP tools based on YAML configuration files.
//! Credentials are resolved via spn daemon (keychain).

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod config;
mod error;
mod server;

use error::Result;

#[derive(Parser)]
#[command(name = "spn-mcp")]
#[command(about = "Dynamic REST-to-MCP wrapper")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server (stdio transport)
    Serve {
        /// Only load specific API (default: all from ~/.spn/apis/)
        #[arg(short, long)]
        api: Option<String>,
    },

    /// List configured APIs
    List,

    /// Validate API configuration
    Validate {
        /// API name to validate
        api: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("spn_mcp=debug,rmcp=debug")
    } else {
        EnvFilter::new("spn_mcp=info,rmcp=warn")
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(filter)
        .init();

    match cli.command {
        Commands::Serve { api } => serve(api).await,
        Commands::List => list().await,
        Commands::Validate { api } => validate(&api).await,
    }
}

async fn serve(api_filter: Option<String>) -> Result<()> {
    tracing::info!("Loading API configurations...");

    // Load configs
    let configs = if let Some(api_name) = api_filter {
        vec![config::load_api(&api_name)?]
    } else {
        config::load_all_apis()?
    };

    if configs.is_empty() {
        tracing::warn!("No API configurations found in ~/.spn/apis/");
        tracing::info!("Create one with: spn mcp wrap <api-name>");
        return Ok(());
    }

    let tool_count: usize = configs.iter().map(|c| c.tools.len()).sum();
    tracing::info!(
        "Loaded {} APIs with {} tools",
        configs.len(),
        tool_count
    );

    // Start MCP server
    server::run(configs).await
}

async fn list() -> Result<()> {
    let apis_dir = config::apis_dir()?;

    if !apis_dir.exists() {
        println!("No APIs configured.");
        println!("Create one with: spn mcp wrap <api-name>");
        return Ok(());
    }

    let configs = config::load_all_apis()?;

    if configs.is_empty() {
        println!("No APIs configured.");
        println!("Create one with: spn mcp wrap <api-name>");
        return Ok(());
    }

    println!("Configured APIs:\n");
    for config in configs {
        println!(
            "  {} ({} tools) - {}",
            config.name,
            config.tools.len(),
            config.description.as_deref().unwrap_or("")
        );
        for tool in &config.tools {
            println!("    - {}_{}", config.name, tool.name);
        }
    }

    Ok(())
}

async fn validate(api_name: &str) -> Result<()> {
    tracing::info!("Validating configuration for: {}", api_name);

    let config = config::load_api(api_name)?;
    config::validate(&config)?;

    println!("Configuration valid for: {}", api_name);
    println!("  Base URL: {}", config.base_url);
    println!("  Auth: {:?}", config.auth.auth_type);
    println!("  Tools: {}", config.tools.len());

    Ok(())
}
