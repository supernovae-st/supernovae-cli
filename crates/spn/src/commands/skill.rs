//! Skill command implementation (via skills.sh).
//!
//! Manages skills from skills.sh registry.

use crate::error::{Result, SpnError};
use crate::interop::skills::SkillsClient;
use crate::SkillCommands;

use colored::Colorize;

/// Run a skill management command.
pub async fn run(command: SkillCommands) -> Result<()> {
    let client = SkillsClient::new();

    match command {
        SkillCommands::Add { name } => {
            println!("{} {}", "Installing skill:".cyan(), name);

            let path = client.install(&name).map_err(|e| {
                SpnError::CommandFailed(format!("Failed to install skill: {}", e))
            })?;
            println!("{} {}", "✓".green(), "Skill installed successfully".green());
            println!("  Location: {}", path.display());
        }
        SkillCommands::Remove { name } => {
            println!("{} {}", "Removing skill:".cyan(), name);

            client.remove(&name).map_err(|e| {
                SpnError::CommandFailed(format!("Failed to remove skill: {}", e))
            })?;
            println!("{} {}", "✓".green(), "Skill removed successfully".green());
        }
        SkillCommands::List => {
            let skills = client.list_installed().map_err(|e| {
                SpnError::CommandFailed(format!("Failed to list skills: {}", e))
            })?;
            if skills.is_empty() {
                println!("{}", "No skills installed".yellow());
                println!("Install with: {}", "spn skill add <name>".cyan());
            } else {
                println!("{}", "Installed skills:".cyan());
                for skill in &skills {
                    println!("  • {}", skill);
                }
                println!("\n{} {} skill(s)", "Total:".dimmed(), skills.len());
            }
        }
        SkillCommands::Search { query } => {
            let url = client.search_url(&query);
            println!("{} {}", "Search on skills.sh:".cyan(), query);
            println!("\nOpen in browser: {}", url.cyan());

            // Try to open the URL in the default browser
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&url).spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", &url])
                    .spawn();
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SkillsClient::new();
        assert!(client.target_dir().to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_search_url() {
        let client = SkillsClient::new();
        let url = client.search_url("brainstorming");
        assert!(url.contains("skills.sh"));
    }
}
