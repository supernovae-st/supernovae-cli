//! Nika wrapper command implementation.
//!
//! Proxies commands to the nika binary.

use crate::error::Result;
use crate::interop::binary::{BinaryRunner, BinaryType};
use crate::{JobCommands, NikaCommands};

use colored::Colorize;

/// Run a nika command via the binary proxy.
pub async fn run(command: NikaCommands) -> Result<()> {
    let runner = BinaryRunner::new(BinaryType::Nika);

    if !runner.is_available() {
        eprintln!("{}", "Error: nika not found".red());
        eprintln!(
            "Install with: {}",
            "brew install supernovae-st/tap/nika".cyan()
        );
        eprintln!(
            "Or download from: {}",
            "https://github.com/supernovae-st/nika/releases".cyan()
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
    };

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match runner.run(&args_refs) {
        Ok(status) => {
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error running nika".red(), e);
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
