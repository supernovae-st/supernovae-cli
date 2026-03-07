//! NovaNet wrapper command implementation.
//!
//! Proxies commands to the novanet binary.

use crate::error::Result;
use crate::interop::binary::{BinaryRunner, BinaryType};
use crate::{
    DbCommands, EntityCommands, KnowledgeCommands, LocaleCommands, McpServerCommands,
    NovaNetCommands,
};

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
            DbCommands::Seed => vec!["db".to_string(), "seed".to_string()],
            DbCommands::Migrate => vec!["db".to_string(), "migrate".to_string()],
            DbCommands::Reset => vec!["db".to_string(), "reset".to_string()],
            DbCommands::Verify => vec!["db".to_string(), "verify".to_string()],
        },
        NovaNetCommands::Search { query, kind, json } => {
            let mut args = vec!["search".to_string(), query.clone()];
            if let Some(k) = kind {
                args.push("--kind".to_string());
                args.push(k.clone());
            }
            if *json {
                args.push("--json".to_string());
            }
            args
        }
        NovaNetCommands::Entity { command } => match command {
            EntityCommands::List { category, json } => {
                let mut args = vec!["entity".to_string(), "list".to_string()];
                if let Some(c) = category {
                    args.push("--category".to_string());
                    args.push(c.clone());
                }
                if *json {
                    args.push("--json".to_string());
                }
                args
            }
            EntityCommands::Show { key, with_native } => {
                let mut args = vec!["entity".to_string(), "show".to_string(), key.clone()];
                if *with_native {
                    args.push("--with-native".to_string());
                }
                args
            }
            EntityCommands::Generate { key, locale } => {
                vec![
                    "entity".to_string(),
                    "generate".to_string(),
                    key.clone(),
                    "--locale".to_string(),
                    locale.clone(),
                ]
            }
        },
        NovaNetCommands::Export {
            output,
            format,
            entity,
        } => {
            let mut args = vec![
                "export".to_string(),
                "--output".to_string(),
                output.clone(),
                "--format".to_string(),
                format.clone(),
            ];
            if let Some(e) = entity {
                args.push("--entity".to_string());
                args.push(e.clone());
            }
            args
        }
        NovaNetCommands::Locale { command } => match command {
            LocaleCommands::List { json } => {
                let mut args = vec!["locale".to_string(), "list".to_string()];
                if *json {
                    args.push("--json".to_string());
                }
                args
            }
            LocaleCommands::Show { code } => {
                vec!["locale".to_string(), "show".to_string(), code.clone()]
            }
            LocaleCommands::Coverage { locale } => {
                vec!["locale".to_string(), "coverage".to_string(), locale.clone()]
            }
        },
        NovaNetCommands::Knowledge { command } => match command {
            KnowledgeCommands::Generate { entity, locale } => {
                vec![
                    "knowledge".to_string(),
                    "generate".to_string(),
                    entity.clone(),
                    "--locale".to_string(),
                    locale.clone(),
                ]
            }
            KnowledgeCommands::List { locale, r#type } => {
                let mut args = vec!["knowledge".to_string(), "list".to_string()];
                if let Some(l) = locale {
                    args.push("--locale".to_string());
                    args.push(l.clone());
                }
                if let Some(t) = r#type {
                    args.push("--type".to_string());
                    args.push(t.clone());
                }
                args
            }
        },
        NovaNetCommands::Stats { json } => {
            let mut args = vec!["stats".to_string()];
            if *json {
                args.push("--json".to_string());
            }
            args
        }
        NovaNetCommands::Diff { json } => {
            let mut args = vec!["diff".to_string()];
            if *json {
                args.push("--json".to_string());
            }
            args
        }
        NovaNetCommands::Doc { output, format } => {
            vec![
                "doc".to_string(),
                "--output".to_string(),
                output.clone(),
                "--format".to_string(),
                format.clone(),
            ]
        }
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
