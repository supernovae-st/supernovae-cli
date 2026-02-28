//! Config command implementation.

use crate::ConfigCommands;
use crate::error::Result;

pub async fn run(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show { section } => match section {
            Some(s) => println!("⚙️  Config section: {}", s),
            None => println!("⚙️  All configuration:"),
        },
        ConfigCommands::Where => {
            println!("📁 Config file locations:");
        }
        ConfigCommands::List { show_origin } => {
            if show_origin {
                println!("📋 Config with origins:");
            } else {
                println!("📋 Config list:");
            }
        }
        ConfigCommands::Edit { local, user, mcp } => {
            if local {
                println!("✏️  Editing local config...");
            } else if user {
                println!("✏️  Editing user config...");
            } else if mcp {
                println!("✏️  Editing MCP config...");
            }
        }
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
