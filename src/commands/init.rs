//! Init command implementation.

use crate::error::Result;

pub async fn run(local: bool, mcp: bool) -> Result<()> {
    if local {
        println!("📁 Creating local config template...");
    } else if mcp {
        println!("🔌 Creating MCP config template...");
    } else {
        println!("🚀 Initializing new project...");
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
