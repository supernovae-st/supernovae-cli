//! Skill command implementation (via skills.sh).
//!
//! Manages skills from skills.sh registry.

use crate::error::{Result, SpnError};
use crate::interop::skills::SkillsClient;
use crate::SkillCommands;

use crate::ux::design_system as ds;

/// Run a skill management command.
pub async fn run(command: SkillCommands) -> Result<()> {
    let client = SkillsClient::new();

    match command {
        SkillCommands::Add { name } => {
            println!("{} {}", ds::primary("Installing skill:"), name);

            let path = client
                .install(&name)
                .map_err(|e| SpnError::CommandFailed(format!("Failed to install skill: {}", e)))?;
            println!(
                "{} {}",
                ds::success("✓"),
                ds::success("Skill installed successfully")
            );
            println!("  Location: {}", path.display());
        }
        SkillCommands::Remove { name } => {
            println!("{} {}", ds::primary("Removing skill:"), name);

            client
                .remove(&name)
                .map_err(|e| SpnError::CommandFailed(format!("Failed to remove skill: {}", e)))?;
            println!(
                "{} {}",
                ds::success("✓"),
                ds::success("Skill removed successfully")
            );
        }
        SkillCommands::List => {
            let skills = client
                .list_installed()
                .map_err(|e| SpnError::CommandFailed(format!("Failed to list skills: {}", e)))?;
            if skills.is_empty() {
                println!("{}", ds::warning("No skills installed"));
                println!("Install with: {}", ds::primary("spn skill add <name>"));
            } else {
                println!("{}", ds::primary("Installed skills:"));
                for skill in &skills {
                    println!("  • {}", skill);
                }
                println!("\n{} {} skill(s)", ds::muted("Total:"), skills.len());
            }
        }
        SkillCommands::Search { query } => {
            let url = client.search_url(&query);
            println!("{} {}", ds::primary("Search on skills.sh:"), query);
            println!("\nOpen in browser: {}", ds::primary(&url));

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
