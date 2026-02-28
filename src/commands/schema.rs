//! Schema command implementation.

use crate::SchemaCommands;
use crate::error::Result;

pub async fn run(command: SchemaCommands) -> Result<()> {
    match command {
        SchemaCommands::Status => {
            println!("📊 Schema status:");
        }
        SchemaCommands::Validate => {
            println!("✅ Validating schema...");
        }
        SchemaCommands::Resolve => {
            println!("🔗 Resolving schema...");
        }
        SchemaCommands::Diff => {
            println!("📝 Schema diff:");
        }
        SchemaCommands::Exclude { name } => {
            println!("➖ Excluding node: {}", name);
        }
        SchemaCommands::Include { name } => {
            println!("➕ Including node: {}", name);
        }
    }
    println!("⚠️  Command not yet implemented");
    Ok(())
}
