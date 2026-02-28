//! MCP server command implementation.

use crate::McpCommands;
use crate::error::Result;

pub async fn run(command: McpCommands) -> Result<()> {
    match command {
        McpCommands::Add { name } => {
            println!("🔌 Adding MCP server: {}", name);
        }
        McpCommands::Remove { name } => {
            println!("🗑️  Removing MCP server: {}", name);
        }
        McpCommands::List => {
            println!("📋 Installed MCP servers:");
        }
        McpCommands::Test { name } => {
            println!("🧪 Testing MCP server: {}", name);
        }
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
