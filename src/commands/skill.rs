//! Skill command implementation (via skills.sh).

use crate::SkillCommands;
use crate::error::Result;

pub async fn run(command: SkillCommands) -> Result<()> {
    match command {
        SkillCommands::Add { name } => {
            println!("📚 Adding skill: {}", name);
        }
        SkillCommands::Remove { name } => {
            println!("🗑️  Removing skill: {}", name);
        }
        SkillCommands::List => {
            println!("📋 Installed skills:");
        }
        SkillCommands::Search { query } => {
            println!("🔍 Searching skills: {}", query);
        }
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
