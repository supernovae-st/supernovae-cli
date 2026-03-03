//! Skill command implementation (via skills.sh).
//!
//! Manages skills from skills.sh registry.

use crate::error::Result;
use crate::interop::skills::SkillsClient;
use crate::SkillCommands;

use colored::Colorize;

/// Run a skill management command.
pub async fn run(command: SkillCommands) -> Result<()> {
    let client = SkillsClient::new();

    match command {
        SkillCommands::Add { name } => {
            println!("{} {}", "Installing skill:".cyan(), name);

            match client.install(&name) {
                Ok(path) => {
                    println!("{} {}", "✓".green(), "Skill installed successfully".green());
                    println!("  Location: {}", path.display());
                }
                Err(e) => {
                    eprintln!("{} {}: {}", "✗".red(), "Failed to install skill".red(), e);
                    std::process::exit(1);
                }
            }
        }
        SkillCommands::Remove { name } => {
            println!("{} {}", "Removing skill:".cyan(), name);

            match client.remove(&name) {
                Ok(()) => {
                    println!("{} {}", "✓".green(), "Skill removed successfully".green());
                }
                Err(e) => {
                    eprintln!("{} {}: {}", "✗".red(), "Failed to remove skill".red(), e);
                    std::process::exit(1);
                }
            }
        }
        SkillCommands::List => match client.list_installed() {
            Ok(skills) => {
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
            Err(e) => {
                eprintln!("{} {}: {}", "✗".red(), "Failed to list skills".red(), e);
                std::process::exit(1);
            }
        },
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
