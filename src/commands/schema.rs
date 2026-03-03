//! Schema command implementation.
//!
//! Manages NovaNet schemas (node classes, arc classes).
//! This is a proxy to `novanet schema` commands.

use crate::error::{CliError, Result};
use crate::SchemaCommands;

pub async fn run(command: SchemaCommands) -> Result<()> {
    // Check if novanet is available
    let novanet_available = which::which("novanet").is_ok();

    if !novanet_available {
        println!("⚠️  NovaNet CLI not found");
        println!();
        println!("   Schema commands require NovaNet to be installed:");
        println!("   • brew install supernovae-st/tap/novanet (builds from source)");
        println!("   • cargo install --git https://github.com/supernovae-st/novanet.git");
        println!();
        return Ok(());
    }

    match command {
        SchemaCommands::Status => schema_status().await,
        SchemaCommands::Validate => schema_validate().await,
        SchemaCommands::Resolve => schema_resolve().await,
        SchemaCommands::Diff => schema_diff().await,
        SchemaCommands::Exclude { name } => schema_exclude(&name).await,
        SchemaCommands::Include { name } => schema_include(&name).await,
    }
}

async fn schema_status() -> Result<()> {
    println!("📊 Schema status:\n");

    // Run novanet schema status
    let output = std::process::Command::new("novanet")
        .args(["schema", "status"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                print!("{}", String::from_utf8_lossy(&out.stdout));
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("not found") || stderr.contains("No schema") {
                    println!("   No schema configured in this project.");
                    println!();
                    println!("   To add a schema package:");
                    println!("   • spn add @novanet/core-schema");
                } else {
                    print!("{}", stderr);
                }
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema status: {}",
                e
            )));
        }
    }

    Ok(())
}

async fn schema_validate() -> Result<()> {
    println!("✅ Validating schema...\n");

    let output = std::process::Command::new("novanet")
        .args(["schema", "validate"])
        .output();

    match output {
        Ok(out) => {
            print!("{}", String::from_utf8_lossy(&out.stdout));
            if !out.status.success() {
                print!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema validate: {}",
                e
            )));
        }
    }

    Ok(())
}

async fn schema_resolve() -> Result<()> {
    println!("🔗 Resolving merged schema...\n");

    let output = std::process::Command::new("novanet")
        .args(["schema", "resolve"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                print!("{}", String::from_utf8_lossy(&out.stdout));
            } else {
                print!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema resolve: {}",
                e
            )));
        }
    }

    Ok(())
}

async fn schema_diff() -> Result<()> {
    println!("📝 Schema diff:\n");

    let output = std::process::Command::new("novanet")
        .args(["schema", "diff"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim().is_empty() {
                    println!("   No changes detected.");
                } else {
                    print!("{}", stdout);
                }
            } else {
                print!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema diff: {}",
                e
            )));
        }
    }

    Ok(())
}

async fn schema_exclude(name: &str) -> Result<()> {
    println!("➖ Excluding node: {}\n", name);

    let output = std::process::Command::new("novanet")
        .args(["schema", "exclude", name])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("   ✓ Node '{}' excluded from schema", name);
            } else {
                print!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema exclude: {}",
                e
            )));
        }
    }

    Ok(())
}

async fn schema_include(name: &str) -> Result<()> {
    println!("➕ Including node: {}\n", name);

    let output = std::process::Command::new("novanet")
        .args(["schema", "include", name])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("   ✓ Node '{}' re-included in schema", name);
            } else {
                print!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema include: {}",
                e
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_commands_handle_missing_novanet() {
        // This test verifies the command doesn't panic when novanet is missing
        // The actual behavior depends on whether novanet is installed
        let result = run(SchemaCommands::Status).await;
        assert!(result.is_ok());
    }
}
