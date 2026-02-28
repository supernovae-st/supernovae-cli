//! Update command implementation.

use crate::error::Result;

pub async fn run(package: Option<&str>) -> Result<()> {
    match package {
        Some(p) => println!("🔄 Updating package: {}", p),
        None => println!("🔄 Updating all packages..."),
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
