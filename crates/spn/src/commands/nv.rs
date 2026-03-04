//! NovaNet wrapper command implementation.
//!
//! Proxies commands to the novanet binary.

use crate::error::Result;
use crate::interop::binary::{BinaryRunner, BinaryType};
use crate::{DbCommands, McpServerCommands, NovaNetCommands};

use colored::Colorize;

/// Run a novanet command via the binary proxy.
pub async fn run(command: NovaNetCommands) -> Result<()> {
    let runner = BinaryRunner::new(BinaryType::NovaNet);

    if !runner.is_available() {
        eprintln!("{}", "Error: novanet not found".red());
        eprintln!(
            "Install with: {}",
            "brew install supernovae-st/tap/novanet".cyan()
        );
        eprintln!(
            "Or download from: {}",
            "https://github.com/supernovae-st/novanet/releases".cyan()
        );
        return Ok(());
    }

    let args: Vec<String> = match &command {
        NovaNetCommands::Tui => vec!["tui".to_string()],
        NovaNetCommands::Query { query } => vec!["query".to_string(), query.clone()],
        NovaNetCommands::Mcp { command } => match command {
            Some(McpServerCommands::Start) => vec!["mcp".to_string(), "start".to_string()],
            Some(McpServerCommands::Stop) => vec!["mcp".to_string(), "stop".to_string()],
            None => vec!["mcp".to_string()],
        },
        NovaNetCommands::AddNode { name, realm, layer } => {
            vec![
                "node".to_string(),
                "add".to_string(),
                name.clone(),
                "--realm".to_string(),
                realm.clone(),
                "--layer".to_string(),
                layer.clone(),
            ]
        }
        NovaNetCommands::AddArc { name, from, to } => {
            vec![
                "arc".to_string(),
                "add".to_string(),
                name.clone(),
                "--from".to_string(),
                from.clone(),
                "--to".to_string(),
                to.clone(),
            ]
        }
        NovaNetCommands::Override { name, add_property } => {
            let mut args = vec!["override".to_string(), name.clone()];
            if let Some(prop) = add_property {
                args.push("--add-property".to_string());
                args.push(prop.clone());
            }
            args
        }
        NovaNetCommands::Db { command } => match command {
            DbCommands::Start => vec!["db".to_string(), "start".to_string()],
            DbCommands::Seed => vec!["db".to_string(), "seed".to_string()],
            DbCommands::Reset => vec!["db".to_string(), "reset".to_string()],
            DbCommands::Migrate => vec!["db".to_string(), "migrate".to_string()],
        },
    };

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match runner.run(&args_refs) {
        Ok(status) => {
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error running novanet".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_availability_check() {
        let runner = BinaryRunner::new(BinaryType::NovaNet);
        // Binary may or may not be available
        let _ = runner.is_available();
    }
}
