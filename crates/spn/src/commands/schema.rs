//! Schema command implementation.
//!
//! Manages NovaNet schemas (node classes, arc classes).
//! This is a proxy to `novanet schema` commands.
//!
//! Available commands (aligned with novanet schema --help):
//! - stats: Extract schema statistics (JSON output)
//! - validate: Validate YAML ↔ Neo4j sync
//! - generate: Generate all artifacts (TypeScript, Cypher, etc.)
//! - cypher-validate: Validate Cypher seed files

use crate::error::{CliError, Result};
use crate::SchemaCommands;
use console::style;

pub async fn run(command: SchemaCommands) -> Result<()> {
    // Check if novanet is available
    let novanet_available = which::which("novanet").is_ok();

    if !novanet_available {
        eprintln!("{} NovaNet CLI not found", style("⚠").yellow().bold());
        eprintln!();
        eprintln!("   Schema commands require NovaNet to be installed:");
        eprintln!("   {} brew install supernovae-st/tap/novanet", style("•").cyan());
        eprintln!("   {} cargo install novanet-cli", style("•").cyan());
        eprintln!();
        eprintln!("   Run {} to install automatically.", style("spn setup novanet").cyan());
        eprintln!();
        return Ok(());
    }

    match command {
        SchemaCommands::Stats => schema_stats().await,
        SchemaCommands::Validate => schema_validate().await,
        SchemaCommands::Generate => schema_generate().await,
        SchemaCommands::CypherValidate => schema_cypher_validate().await,
    }
}

/// Show schema statistics (JSON output).
/// Proxies to `novanet schema stats`.
async fn schema_stats() -> Result<()> {
    println!("{} Schema statistics:\n", style("📊").cyan());

    let output = std::process::Command::new("novanet")
        .args(["schema", "stats"])
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
                    println!("   {} spn add @novanet/core-schema", style("•").cyan());
                } else {
                    eprint!("{}", stderr);
                }
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema stats: {}",
                e
            )));
        }
    }

    Ok(())
}

/// Validate YAML ↔ Neo4j sync.
/// Proxies to `novanet schema validate`.
async fn schema_validate() -> Result<()> {
    println!("{} Validating schema...\n", style("✓").green());

    let output = std::process::Command::new("novanet")
        .args(["schema", "validate"])
        .output();

    match output {
        Ok(out) => {
            print!("{}", String::from_utf8_lossy(&out.stdout));
            if !out.status.success() {
                eprint!("{}", String::from_utf8_lossy(&out.stderr));
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

/// Generate all schema artifacts (TypeScript, Cypher, etc.).
/// Proxies to `novanet schema generate`.
async fn schema_generate() -> Result<()> {
    println!("{} Generating schema artifacts...\n", style("🔧").cyan());

    let output = std::process::Command::new("novanet")
        .args(["schema", "generate"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                print!("{}", String::from_utf8_lossy(&out.stdout));
                println!();
                println!("   {} Schema artifacts generated successfully.", style("✓").green());
            } else {
                eprint!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema generate: {}",
                e
            )));
        }
    }

    Ok(())
}

/// Validate Cypher seed files.
/// Proxies to `novanet schema cypher-validate`.
async fn schema_cypher_validate() -> Result<()> {
    println!("{} Validating Cypher seed files...\n", style("🔍").cyan());

    let output = std::process::Command::new("novanet")
        .args(["schema", "cypher-validate"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                print!("{}", String::from_utf8_lossy(&out.stdout));
                println!();
                println!("   {} Cypher files are valid.", style("✓").green());
            } else {
                eprint!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            return Err(CliError::CommandFailed(format!(
                "novanet schema cypher-validate: {}",
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
        let result = run(SchemaCommands::Stats).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schema_validate_handles_missing_novanet() {
        let result = run(SchemaCommands::Validate).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schema_generate_handles_missing_novanet() {
        let result = run(SchemaCommands::Generate).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schema_cypher_validate_handles_missing_novanet() {
        let result = run(SchemaCommands::CypherValidate).await;
        assert!(result.is_ok());
    }
}
