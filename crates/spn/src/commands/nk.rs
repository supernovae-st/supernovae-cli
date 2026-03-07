//! Nika wrapper command implementation.
//!
//! Proxies commands to the nika binary.

use crate::error::Result;
use crate::interop::binary::{BinaryRunner, BinaryType};
use crate::{JobCommands, NikaCommands, NikaConfigCommands, TraceCommands};

use crate::ux::design_system as ds;

/// Run a nika command via the binary proxy.
pub async fn run(command: NikaCommands) -> Result<()> {
    let runner = BinaryRunner::new(BinaryType::Nika);

    if !runner.is_available() {
        eprintln!("{}", ds::error("Error: nika not found"));
        eprintln!(
            "Install with: {}",
            ds::primary("brew install supernovae-st/tap/nika")
        );
        eprintln!(
            "Or download from: {}",
            ds::primary("https://github.com/supernovae-st/nika/releases")
        );
        return Ok(());
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
            NikaConfigCommands::Show => vec!["config".to_string(), "show".to_string()],
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
