//! Add command implementation.

use crate::error::Result;

pub async fn run(package: &str, r#type: Option<&str>) -> Result<()> {
    println!("📦 Adding package: {}", package);
    if let Some(t) = r#type {
        println!("   Type: {}", t);
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
