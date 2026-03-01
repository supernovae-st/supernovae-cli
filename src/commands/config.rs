//! Config command implementation.
//!
//! Shows and manages configuration from multiple sources.

use std::env;
use std::path::PathBuf;

use crate::ConfigCommands;
use crate::error::Result;

/// Config file locations in precedence order (lowest to highest).
#[derive(Debug)]
struct ConfigLocations {
    /// User config (~/.config/spn/config.yaml)
    user: Option<PathBuf>,
    /// Project config (spn.yaml)
    project: Option<PathBuf>,
    /// Local config (spn.local.yaml)
    local: Option<PathBuf>,
    /// MCP config (.mcp.yaml)
    mcp: Option<PathBuf>,
    /// MCP local config (.mcp.local.yaml)
    mcp_local: Option<PathBuf>,
}

impl ConfigLocations {
    fn discover() -> Self {
        let cwd = env::current_dir().unwrap_or_default();

        // User config directory
        let user_config = dirs::config_dir()
            .map(|d| d.join("spn").join("config.yaml"))
            .filter(|p| p.exists());

        // Project configs
        let project = cwd.join("spn.yaml");
        let local = cwd.join("spn.local.yaml");
        let mcp = cwd.join(".mcp.yaml");
        let mcp_local = cwd.join(".mcp.local.yaml");

        Self {
            user: user_config,
            project: project.exists().then_some(project),
            local: local.exists().then_some(local),
            mcp: mcp.exists().then_some(mcp),
            mcp_local: mcp_local.exists().then_some(mcp_local),
        }
    }
}

pub async fn run(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show { section } => show_config(section).await,
        ConfigCommands::Where => show_locations().await,
        ConfigCommands::List { show_origin } => list_config(show_origin).await,
        ConfigCommands::Edit { local, user, mcp } => edit_config(local, user, mcp).await,
    }
}

async fn show_config(section: Option<String>) -> Result<()> {
    let locations = ConfigLocations::discover();

    println!("⚙️  Configuration:\n");

    // Show project config if exists
    if let Some(ref path) = locations.project {
        let content = std::fs::read_to_string(path)?;
        match section {
            Some(ref s) => {
                println!("   Section: {}", s);
                // Parse YAML and extract section
                if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    if let Some(value) = yaml.get(s) {
                        let section_yaml = serde_yaml::to_string(value)?;
                        for line in section_yaml.lines() {
                            println!("   {}", line);
                        }
                    } else {
                        println!("   Section '{}' not found", s);
                    }
                }
            }
            None => {
                println!("   {}:", path.display());
                for line in content.lines().take(20) {
                    println!("   {}", line);
                }
                if content.lines().count() > 20 {
                    println!("   ... ({} more lines)", content.lines().count() - 20);
                }
            }
        }
    } else {
        println!("   No spn.yaml found in current directory.");
        println!();
        println!("   Run 'spn init' to create one.");
    }

    Ok(())
}

async fn show_locations() -> Result<()> {
    let locations = ConfigLocations::discover();
    let cwd = env::current_dir()?;

    println!("📁 Config file locations:\n");

    // User config
    let user_path = dirs::config_dir()
        .map(|d| d.join("spn").join("config.yaml"))
        .unwrap_or_else(|| PathBuf::from("~/.config/spn/config.yaml"));
    let user_status = if locations.user.is_some() { "✓" } else { "○" };
    println!("   {} User:    {}", user_status, user_path.display());

    // Project config
    let project_path = cwd.join("spn.yaml");
    let project_status = if locations.project.is_some() { "✓" } else { "○" };
    println!("   {} Project: {}", project_status, project_path.display());

    // Local config
    let local_path = cwd.join("spn.local.yaml");
    let local_status = if locations.local.is_some() { "✓" } else { "○" };
    println!("   {} Local:   {} (gitignored)", local_status, local_path.display());

    // MCP config
    let mcp_path = cwd.join(".mcp.yaml");
    let mcp_status = if locations.mcp.is_some() { "✓" } else { "○" };
    println!("   {} MCP:     {}", mcp_status, mcp_path.display());

    // MCP local config
    let mcp_local_path = cwd.join(".mcp.local.yaml");
    let mcp_local_status = if locations.mcp_local.is_some() { "✓" } else { "○" };
    println!("   {} MCP:     {} (gitignored)", mcp_local_status, mcp_local_path.display());

    println!();
    println!("   ✓ = exists, ○ = not found");
    println!();
    println!("   Precedence: user < project < local");

    Ok(())
}

async fn list_config(show_origin: bool) -> Result<()> {
    let locations = ConfigLocations::discover();

    println!("📋 Configuration values:\n");

    if locations.project.is_none() {
        println!("   No spn.yaml found. Run 'spn init' to create one.");
        return Ok(());
    }

    // Load and display project config
    if let Some(ref path) = locations.project {
        let content = std::fs::read_to_string(path)?;
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            print_yaml_values(&yaml, "", show_origin, path);
        }
    }

    if show_origin {
        println!();
        println!("   Values from: spn.yaml < spn.local.yaml");
    }

    Ok(())
}

fn print_yaml_values(value: &serde_yaml::Value, prefix: &str, show_origin: bool, source: &PathBuf) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    let new_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    match v {
                        serde_yaml::Value::Mapping(_) => {
                            print_yaml_values(v, &new_prefix, show_origin, source);
                        }
                        serde_yaml::Value::Sequence(seq) => {
                            if show_origin {
                                println!("   {} = [{} items] ({})", new_prefix, seq.len(), source.file_name().unwrap().to_string_lossy());
                            } else {
                                println!("   {} = [{} items]", new_prefix, seq.len());
                            }
                        }
                        _ => {
                            let val_str = match v {
                                serde_yaml::Value::String(s) => s.clone(),
                                serde_yaml::Value::Number(n) => n.to_string(),
                                serde_yaml::Value::Bool(b) => b.to_string(),
                                serde_yaml::Value::Null => "null".to_string(),
                                _ => format!("{:?}", v),
                            };
                            if show_origin {
                                println!("   {} = {} ({})", new_prefix, val_str, source.file_name().unwrap().to_string_lossy());
                            } else {
                                println!("   {} = {}", new_prefix, val_str);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

async fn edit_config(local: bool, user: bool, mcp: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    let path = if local {
        cwd.join("spn.local.yaml")
    } else if user {
        dirs::config_dir()
            .map(|d| d.join("spn").join("config.yaml"))
            .unwrap_or_else(|| PathBuf::from("~/.config/spn/config.yaml"))
    } else if mcp {
        cwd.join(".mcp.yaml")
    } else {
        cwd.join("spn.yaml")
    };

    // Determine editor
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    if !path.exists() {
        println!("⚠️  File does not exist: {}", path.display());
        if local {
            println!("   Run 'spn init --local' to create it.");
        } else if mcp {
            println!("   Run 'spn init --mcp' to create it.");
        } else {
            println!("   Run 'spn init' to create it.");
        }
        return Ok(());
    }

    println!("✏️  Opening {} with {}...", path.display(), editor);

    // Open editor
    std::process::Command::new(&editor)
        .arg(&path)
        .status()?;

    println!("   Config saved.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_show_locations_runs() {
        let result = show_locations().await;
        assert!(result.is_ok());
    }
}
