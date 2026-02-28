//! Remove command implementation.

use crate::error::Result;

pub async fn run(package: &str) -> Result<()> {
    println!("🗑️  Removing package: {}", package);
    println!("⚠️  Command not yet implemented");
    Ok(())
}
