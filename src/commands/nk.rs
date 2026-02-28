//! Nika wrapper command implementation.

use crate::NikaCommands;
use crate::error::Result;

pub async fn run(command: NikaCommands) -> Result<()> {
    match command {
        NikaCommands::Run { file } => {
            println!("▶️  Running workflow: {}", file);
        }
        NikaCommands::Check { file } => {
            println!("✅ Checking workflow: {}", file);
        }
        NikaCommands::Studio => {
            println!("🎨 Opening Nika Studio...");
        }
        NikaCommands::Jobs { command: _ } => {
            println!("📋 Jobs management...");
        }
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
