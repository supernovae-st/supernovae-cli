//! NovaNet wrapper command implementation.

use crate::NovaNetCommands;
use crate::error::Result;

pub async fn run(command: NovaNetCommands) -> Result<()> {
    match command {
        NovaNetCommands::Tui => {
            println!("🖥️  Opening NovaNet TUI...");
        }
        NovaNetCommands::Query { query } => {
            println!("🔍 Query: {}", query);
        }
        NovaNetCommands::Mcp { command: _ } => {
            println!("🔌 MCP server management...");
        }
        NovaNetCommands::AddNode { name, realm, layer } => {
            println!("➕ Adding node: {} ({}/{})", name, realm, layer);
        }
        NovaNetCommands::AddArc { name, from, to } => {
            println!("🔗 Adding arc: {} ({} → {})", name, from, to);
        }
        NovaNetCommands::Override {
            name,
            add_property: _,
        } => {
            println!("✏️  Overriding node: {}", name);
        }
        NovaNetCommands::Db { command: _ } => {
            println!("🗄️  Database management...");
        }
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
