//! Init command implementation.
//!
//! Creates spn.yaml manifest and optional config files.

use std::env;
use std::path::Path;

use crate::error::Result;
use crate::manifest::SpnManifest;

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

pub async fn run(local: bool, mcp: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    if local {
        create_local_config(&cwd).await
    } else if mcp {
        create_mcp_config(&cwd).await
    } else {
        create_project(&cwd).await
    }
}

/// Create a new spn.yaml project manifest.
async fn create_project(dir: &Path) -> Result<()> {
    let manifest_path = dir.join("spn.yaml");

    if manifest_path.exists() {
        println!("⚠️  spn.yaml already exists");
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

    println!("🚀 Initialized new SuperNovae project");
    println!();
    println!("   Created:");
    println!("   ├── spn.yaml         (package manifest)");
    println!("   └── .spn/            (local state)");
    println!();
    println!("   Next steps:");
    println!("   • spn add @workflows/dev-productivity/code-review");
    println!("   • spn skill add brainstorming");
    println!("   • spn mcp add neo4j");
    println!();

    Ok(())
}

/// Create local config template.
async fn create_local_config(dir: &Path) -> Result<()> {
    let local_path = dir.join("spn.local.yaml");

    if local_path.exists() {
        println!("⚠️  spn.local.yaml already exists");
        return Ok(());
    }

    std::fs::write(&local_path, LOCAL_CONFIG_TEMPLATE)?;

    println!("📁 Created spn.local.yaml (gitignored)");
    println!("   Use this for local overrides and secrets.");

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
        println!("⚠️  MCP config files already exist");
    } else {
        println!("🔌 Created MCP configuration:");
        for file in created {
            println!("   • {}", file);
        }
    }

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
