//! Search command implementation.

use crate::error::Result;

pub async fn run(query: &str) -> Result<()> {
    println!("🔍 Searching for: {}", query);
    println!("⚠️  Command not yet implemented");
    Ok(())
}
