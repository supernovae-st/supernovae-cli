//! Info command implementation.

use crate::error::Result;

pub async fn run(package: &str) -> Result<()> {
    println!("ℹ️  Package info: {}", package);
    println!("⚠️  Command not yet implemented");
    Ok(())
}
