//! Help command implementation.

use crate::error::Result;

pub async fn run(topic: Option<&str>) -> Result<()> {
    match topic {
        Some("config") => {
            println!("📖 Configuration Help");
            println!("====================");
            println!("spn uses a layered configuration system...");
        }
        Some("scopes") => {
            println!("📖 Package Scopes");
            println!("=================");
            println!("@nika/*    - Nika workflow packages");
            println!("@novanet/* - NovaNet packages");
            println!("@community/* - Community packages");
        }
        Some("mcp") => {
            println!("📖 MCP Servers");
            println!("==============");
            println!("MCP (Model Context Protocol) servers...");
        }
        Some(t) => {
            println!("❓ Unknown topic: {}", t);
        }
        None => {
            println!("📖 Available help topics:");
            println!("  config  - Configuration system");
            println!("  scopes  - Package scopes");
            println!("  mcp     - MCP servers");
        }
    }
    Ok(())
}
