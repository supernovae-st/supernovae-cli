//! Install command implementation.

use crate::error::Result;

pub async fn run(frozen: bool) -> Result<()> {
    if frozen {
        println!("📦 Installing from spn.lock (frozen)...");
    } else {
        println!("📦 Installing dependencies...");
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
