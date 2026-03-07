//! Shell completion generation.
//!
//! Generate shell completions for bash, zsh, fish, and PowerShell.
//!
//! # Usage
//!
//! ```bash
//! # Bash
//! spn completion bash >> ~/.bashrc
//!
//! # Zsh
//! spn completion zsh >> ~/.zshrc
//!
//! # Fish
//! spn completion fish > ~/.config/fish/completions/spn.fish
//!
//! # PowerShell
//! spn completion powershell >> $PROFILE
//! ```

use crate::error::{Result, SpnError};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;
use std::path::PathBuf;

/// Run the completion command.
pub async fn run(shell: &str, output: Option<PathBuf>) -> Result<()> {
    let shell = parse_shell(shell)?;

    // Build the CLI command structure
    let mut cmd = crate::Cli::command();

    // Generate completions
    if let Some(path) = output {
        let mut file = std::fs::File::create(&path)?;
        generate(shell, &mut cmd, "spn", &mut file);
        println!("Completions written to: {}", path.display());
    } else {
        generate(shell, &mut cmd, "spn", &mut io::stdout());
    }

    Ok(())
}

/// Parse shell name to clap_complete Shell enum.
fn parse_shell(shell: &str) -> Result<Shell> {
    match shell.to_lowercase().as_str() {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "powershell" | "pwsh" => Ok(Shell::PowerShell),
        "elvish" => Ok(Shell::Elvish),
        _ => Err(SpnError::InvalidInput(format!(
            "Unknown shell '{}'. Supported: bash, zsh, fish, powershell, elvish",
            shell
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shell_bash() {
        assert!(matches!(parse_shell("bash"), Ok(Shell::Bash)));
        assert!(matches!(parse_shell("BASH"), Ok(Shell::Bash)));
    }

    #[test]
    fn test_parse_shell_zsh() {
        assert!(matches!(parse_shell("zsh"), Ok(Shell::Zsh)));
    }

    #[test]
    fn test_parse_shell_fish() {
        assert!(matches!(parse_shell("fish"), Ok(Shell::Fish)));
    }

    #[test]
    fn test_parse_shell_powershell() {
        assert!(matches!(parse_shell("powershell"), Ok(Shell::PowerShell)));
        assert!(matches!(parse_shell("pwsh"), Ok(Shell::PowerShell)));
    }

    #[test]
    fn test_parse_shell_invalid() {
        assert!(parse_shell("invalid").is_err());
    }
}
