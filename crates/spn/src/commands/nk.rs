//! Nika wrapper command implementation.
//!
//! Proxies commands to the nika binary with lazy install support.

use std::io::IsTerminal;

use crate::error::{Result, SpnError};
use crate::interop::binary::{BinaryRunner, BinaryType};
use crate::interop::detect::{install_nika, EcosystemTools, InstallMethod};
use crate::{JobCommands, NikaCommands, NikaConfigCommands, TraceCommands};

use crate::ux::design_system as ds;
use dialoguer::Confirm;

/// Run a nika command via the binary proxy.
pub async fn run(command: NikaCommands) -> Result<()> {
    let runner = BinaryRunner::new(BinaryType::Nika);

    if !runner.is_available() {
        // Check if we're in an interactive terminal
        if std::io::stdin().is_terminal() {
            eprintln!();
            eprintln!("{}", ds::warning("⚠️  Nika is not installed"));
            eprintln!();

            let install = Confirm::new()
                .with_prompt("Install Nika now?")
                .default(true)
                .interact()
                .map_err(|e| SpnError::InvalidInput(e.to_string()))?;

            if install {
                let method = InstallMethod::best_available().ok_or_else(|| {
                    SpnError::NotFound(
                        "No installation method available (cargo or brew required)".into(),
                    )
                })?;

                eprintln!();
                eprintln!(
                    "  {} Installing Nika via {}...",
                    ds::primary("→"),
                    ds::muted(method.display_name())
                );

                match install_nika(method).await {
                    Ok(()) => {
                        eprintln!("  {} Nika installed successfully", ds::command("✓"));
                        eprintln!();
                        // Re-detect after install
                        let tools = EcosystemTools::detect();
                        if !tools.nika.is_installed() {
                            eprintln!(
                                "{} Installation completed but nika not found in PATH.",
                                ds::warning("⚠️")
                            );
                            eprintln!(
                                "  You may need to restart your shell or run: {}",
                                ds::primary("source ~/.bashrc")
                            );
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        eprintln!("  {} Installation failed: {}", ds::error("✗"), e);
                        return Ok(());
                    }
                }
            } else {
                eprintln!("{}", ds::muted("Install later with: spn setup nika"));
                return Ok(());
            }
        } else {
            // Non-interactive mode - show error
            eprintln!("{}", ds::error("Error: nika not found"));
            eprintln!("Install with: {}", ds::primary("spn setup nika"));
            return Ok(());
        }
    }

    let args: Vec<String> = match &command {
        NikaCommands::Run { file } => vec!["run".to_string(), file.clone()],
        NikaCommands::Check { file } => vec!["check".to_string(), file.clone()],
        NikaCommands::Studio => vec!["studio".to_string()],
        NikaCommands::Jobs { command } => match command {
            JobCommands::Start => vec!["jobs".to_string(), "start".to_string()],
            JobCommands::Status => vec!["jobs".to_string(), "status".to_string()],
            JobCommands::Stop => vec!["jobs".to_string(), "stop".to_string()],
        },
        NikaCommands::New { name, template } => {
            vec![
                "new".to_string(),
                name.clone(),
                "--template".to_string(),
                template.clone(),
            ]
        }
        NikaCommands::Trace { command } => match command {
            TraceCommands::List { limit } => {
                vec![
                    "trace".to_string(),
                    "list".to_string(),
                    "--limit".to_string(),
                    limit.to_string(),
                ]
            }
            TraceCommands::Show { id } => {
                vec!["trace".to_string(), "show".to_string(), id.clone()]
            }
            TraceCommands::Clean { keep } => {
                vec![
                    "trace".to_string(),
                    "clean".to_string(),
                    "--keep".to_string(),
                    keep.clone(),
                ]
            }
        },
        NikaCommands::Config { command } => match command {
            NikaConfigCommands::List => vec!["config".to_string(), "list".to_string()],
            NikaConfigCommands::Get { key } => {
                vec!["config".to_string(), "get".to_string(), key.clone()]
            }
            NikaConfigCommands::Set { key, value } => {
                vec![
                    "config".to_string(),
                    "set".to_string(),
                    key.clone(),
                    value.clone(),
                ]
            }
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
            eprintln!("{}: {}", ds::error("Error running nika"), e);
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
        let runner = BinaryRunner::new(BinaryType::Nika);
        // Binary may or may not be available
        let _ = runner.is_available();
    }
}
