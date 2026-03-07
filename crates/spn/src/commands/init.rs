//! Init command implementation.
//!
//! Creates spn.yaml manifest and optional config files.

use std::env;
use std::path::Path;

use crate::error::Result;
use crate::manifest::SpnManifest;
use crate::ux::design_system as ds;

/// Template for local config (gitignored).
const LOCAL_CONFIG_TEMPLATE: &str = r#"# Local overrides (gitignored)
# Override any spn.yaml settings here

# Example: Local API keys for MCP servers
# mcp:
#   neo4j:
#     env:
#       NEO4J_PASSWORD: "your-local-password"
"#;

/// Template for MCP config.
const MCP_CONFIG_TEMPLATE: &str = r#"# MCP Server Configuration
# Team-shared MCP servers (committed to git)
# Secrets go in .mcp.local.yaml (gitignored)

servers:
  # Example: Neo4j knowledge graph
  # neo4j:
  #   command: npx
  #   args: ["-y", "@neo4j/mcp-server-neo4j"]
  #   env:
  #     NEO4J_URI: "bolt://localhost:7687"
  #     NEO4J_USER: "neo4j"
  #     # NEO4J_PASSWORD in .mcp.local.yaml
"#;

/// Template for Nika config.
const NIKA_CONFIG_TEMPLATE: &str = r#"# Nika Configuration
# Workflow engine settings and provider configuration

# Default LLM provider
default_provider = "anthropic"

# Provider configurations
[providers.anthropic]
model = "claude-sonnet-4-5-20250929"
# API key retrieved from OS keychain via: spn provider set anthropic

[providers.openai]
model = "gpt-4o"
# API key retrieved from OS keychain via: spn provider set openai

# Workflow execution settings
[execution]
max_retries = 3
timeout_seconds = 300
parallel_jobs = 4

# Output settings
[output]
format = "yaml"  # yaml, json, or pretty
verbose = false
log_level = "info"  # trace, debug, info, warn, error

# Cache settings
[cache]
enabled = true
ttl_seconds = 3600
"#;

pub async fn run(local: bool, mcp: bool, template: Option<String>) -> Result<()> {
    let cwd = env::current_dir()?;

    if local {
        create_local_config(&cwd).await
    } else if mcp {
        create_mcp_config(&cwd).await
    } else if let Some(tpl) = template {
        create_from_template(&cwd, &tpl).await
    } else {
        create_project(&cwd).await
    }
}

/// Create a new spn.yaml project manifest.
async fn create_project(dir: &Path) -> Result<()> {
    let manifest_path = dir.join("spn.yaml");

    if manifest_path.exists() {
        println!(
            "{} {}",
            ds::warning(ds::icon::WARNING),
            ds::warning("spn.yaml already exists")
        );
        return Ok(());
    }

    // Derive project name from directory
    let project_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project")
        .to_string();

    let manifest = SpnManifest {
        name: project_name.clone(),
        version: "0.1.0".to_string(),
        description: Some(format!("{} - SuperNovae project", project_name)),
        authors: Vec::new(),
        license: Some("MIT".to_string()),
        repository: None,
        dependencies: Default::default(),
        dev_dependencies: Default::default(),
    };

    // Write manifest
    manifest.write_to_file(&manifest_path)?;

    // Create .spn directory
    let spn_dir = dir.join(".spn");
    std::fs::create_dir_all(&spn_dir)?;

    // Create .gitignore for .spn if it doesn't exist
    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() {
        // Append to existing .gitignore
        let content = std::fs::read_to_string(&gitignore_path)?;
        if !content.contains(".spn/") {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&gitignore_path)?;
            use std::io::Write;
            writeln!(file, "\n# SuperNovae local state\n.spn/")?;
            writeln!(file, "spn.local.yaml")?;
            writeln!(file, ".mcp.local.yaml")?;
        }
    }

    println!(
        "{} {}",
        ds::success(ds::icon::SUCCESS),
        ds::success("Initialized new SuperNovae project")
    );
    println!();
    println!("  {}", ds::highlight("Created:"));
    println!("  {} {}", ds::tree_branch(), ds::path("spn.yaml         (package manifest)"));
    println!(
        "  {} {}",
        ds::tree_branch_last(),
        ds::path(".spn/            (local state)")
    );
    println!();
    println!("  {}", ds::highlight("Next steps:"));
    println!(
        "  {} {}",
        ds::bullet_icon(),
        ds::command("spn add @workflows/dev-productivity/code-review")
    );
    println!("  {} {}", ds::bullet_icon(), ds::command("spn skill add brainstorming"));
    println!("  {} {}", ds::bullet_icon(), ds::command("spn mcp add neo4j"));
    println!();

    Ok(())
}

/// Create local config template.
async fn create_local_config(dir: &Path) -> Result<()> {
    let local_path = dir.join("spn.local.yaml");

    if local_path.exists() {
        println!(
            "{} {}",
            ds::warning(ds::icon::WARNING),
            ds::warning("spn.local.yaml already exists")
        );
        return Ok(());
    }

    std::fs::write(&local_path, LOCAL_CONFIG_TEMPLATE)?;

    println!(
        "{} {}",
        ds::success(ds::icon::SUCCESS),
        ds::success("Created spn.local.yaml (gitignored)")
    );
    println!("  {}", ds::muted("Use this for local overrides and secrets."));

    Ok(())
}

/// Create MCP config template.
async fn create_mcp_config(dir: &Path) -> Result<()> {
    let mcp_path = dir.join(".mcp.yaml");
    let mcp_local_path = dir.join(".mcp.local.yaml");

    let mut created = Vec::new();

    if !mcp_path.exists() {
        std::fs::write(&mcp_path, MCP_CONFIG_TEMPLATE)?;
        created.push(".mcp.yaml (team config)");
    }

    if !mcp_local_path.exists() {
        std::fs::write(
            &mcp_local_path,
            "# Local MCP secrets (gitignored)\n# Add API keys and passwords here\n",
        )?;
        created.push(".mcp.local.yaml (local secrets)");
    }

    if created.is_empty() {
        println!(
            "{} {}",
            ds::warning(ds::icon::WARNING),
            ds::warning("MCP config files already exist")
        );
    } else {
        println!(
            "{} {}",
            ds::success(ds::icon::SUCCESS),
            ds::success("Created MCP configuration:")
        );
        for file in created {
            println!("  {} {}", ds::bullet_icon(), ds::path(file));
        }
    }

    Ok(())
}

/// Create project from template.
async fn create_from_template(dir: &Path, template: &str) -> Result<()> {
    match template {
        "nika" => create_nika_project(dir).await,
        "novanet" => create_novanet_project(dir).await,
        _ => {
            println!(
                "{} Unknown template: {}",
                ds::error(ds::icon::ERROR),
                ds::error(template)
            );
            println!();
            println!("{}", ds::highlight("Available templates:"));
            println!("  {} {}", ds::bullet_icon(), ds::muted("nika       - Workflow engine project"));
            println!("  {} {}", ds::bullet_icon(), ds::muted("novanet    - Knowledge graph project"));
            Err(crate::error::SpnError::InvalidInput(format!(
                "Unknown template: {}",
                template
            )))
        }
    }
}

/// Create a Nika workflow project.
async fn create_nika_project(dir: &Path) -> Result<()> {
    // First create base project
    create_project(dir).await?;

    // Create .nika directory
    let nika_dir = dir.join(".nika");
    std::fs::create_dir_all(&nika_dir)?;

    // Create nika config
    let config_path = nika_dir.join("config.toml");
    std::fs::write(&config_path, NIKA_CONFIG_TEMPLATE)?;

    // Create workflows directory
    let workflows_dir = dir.join("workflows");
    std::fs::create_dir_all(&workflows_dir)?;

    // Create example workflow
    let example_workflow = workflows_dir.join("hello.yaml");
    let example_content = r#"name: hello
description: Simple hello world workflow

steps:
  - infer: "Say hello"
    model: claude-sonnet-4-5-20250929
    use.output: greeting
"#;
    std::fs::write(&example_workflow, example_content)?;

    // Update .gitignore
    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() {
        let content = std::fs::read_to_string(&gitignore_path)?;
        if !content.contains(".nika/cache") {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&gitignore_path)?;
            use std::io::Write;
            writeln!(file, "\n# Nika cache and state")?;
            writeln!(file, ".nika/cache/")?;
            writeln!(file, ".nika/state/")?;
        }
    } else {
        std::fs::write(
            &gitignore_path,
            "# Nika cache and state\n.nika/cache/\n.nika/state/\n",
        )?;
    }

    println!(
        "{} {}",
        ds::success(ds::icon::SUCCESS),
        ds::success("Initialized Nika workflow project")
    );
    println!();
    println!("  {}", ds::highlight("Created:"));
    println!(
        "  {} {}",
        ds::tree_branch(),
        ds::path("spn.yaml                (package manifest)")
    );
    println!(
        "  {} {}",
        ds::tree_branch(),
        ds::path(".nika/config.toml       (nika config)")
    );
    println!(
        "  {} {}",
        ds::tree_branch(),
        ds::path("workflows/hello.yaml    (example workflow)")
    );
    println!(
        "  {} {}",
        ds::tree_branch_last(),
        ds::path(".spn/                   (local state)")
    );
    println!();
    println!("  {}", ds::highlight("Next steps:"));
    println!(
        "  {} {} {}",
        ds::bullet_icon(),
        ds::command("spn provider set anthropic"),
        ds::muted("# Configure API keys")
    );
    println!(
        "  {} {}",
        ds::bullet_icon(),
        ds::command("spn add @workflows/dev/code-review")
    );
    println!(
        "  {} {}",
        ds::bullet_icon(),
        ds::command("nika run workflows/hello.yaml")
    );
    println!();

    Ok(())
}

/// Create a NovaNet knowledge graph project.
async fn create_novanet_project(dir: &Path) -> Result<()> {
    // First create base project
    create_project(dir).await?;

    // Create brain directory structure
    let brain_dir = dir.join("brain");
    std::fs::create_dir_all(brain_dir.join("models"))?;
    std::fs::create_dir_all(brain_dir.join("seed"))?;
    std::fs::create_dir_all(brain_dir.join("data"))?;

    // Create example node class
    let example_model = brain_dir.join("models").join("entities.yaml");
    let model_content = r#"# Entity definitions
node_classes:
  - name: Entity
    description: Base semantic concept
    properties:
      - name: slug
        type: string
        required: true
      - name: description
        type: string
"#;
    std::fs::write(&example_model, model_content)?;

    println!(
        "{} {}",
        ds::success(ds::icon::SUCCESS),
        ds::success("Initialized NovaNet knowledge graph project")
    );
    println!();
    println!("  {}", ds::highlight("Created:"));
    println!(
        "  {} {}",
        ds::tree_branch(),
        ds::path("spn.yaml                (package manifest)")
    );
    println!(
        "  {} {}",
        ds::tree_branch(),
        ds::path("brain/models/           (schema definitions)")
    );
    println!(
        "  {} {}",
        ds::tree_branch(),
        ds::path("brain/seed/             (seed data)")
    );
    println!(
        "  {} {}",
        ds::tree_branch(),
        ds::path("brain/data/             (graph data)")
    );
    println!(
        "  {} {}",
        ds::tree_branch_last(),
        ds::path(".spn/                   (local state)")
    );
    println!();
    println!("  {}", ds::highlight("Next steps:"));
    println!(
        "  {} {} {}",
        ds::bullet_icon(),
        ds::command("spn mcp add neo4j"),
        ds::muted("# Add Neo4j MCP server")
    );
    println!(
        "  {} {} {}",
        ds::bullet_icon(),
        ds::command("spn provider set neo4j"),
        ds::muted("# Configure Neo4j credentials")
    );
    println!(
        "  {} {} {}",
        ds::bullet_icon(),
        ds::command("novanet schema validate"),
        ds::muted("# Validate schema")
    );
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_creates_manifest() {
        let temp = TempDir::new().unwrap();

        let result = create_project(temp.path()).await;
        assert!(result.is_ok());

        let manifest_path = temp.path().join("spn.yaml");
        assert!(manifest_path.exists());

        let manifest = SpnManifest::from_file(&manifest_path).unwrap();
        assert_eq!(manifest.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_init_local_creates_file() {
        let temp = TempDir::new().unwrap();

        let result = create_local_config(temp.path()).await;
        assert!(result.is_ok());

        let local_path = temp.path().join("spn.local.yaml");
        assert!(local_path.exists());
    }

    #[tokio::test]
    async fn test_init_mcp_creates_files() {
        let temp = TempDir::new().unwrap();

        let result = create_mcp_config(temp.path()).await;
        assert!(result.is_ok());

        assert!(temp.path().join(".mcp.yaml").exists());
        assert!(temp.path().join(".mcp.local.yaml").exists());
    }
}
