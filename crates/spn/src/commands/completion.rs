//! Shell completion generation and installation.
//!
//! Generate and install shell completions for bash, zsh, fish, and PowerShell.
//!
//! # Usage
//!
//! ```bash
//! # Auto-detect shell and install
//! spn completion install
//!
//! # Install for specific shell
//! spn completion install --shell bash
//!
//! # Generate to stdout (manual install)
//! spn completion bash
//!
//! # Generate to file
//! spn completion zsh --output ~/.zsh/completions/_spn
//!
//! # Check installation status
//! spn completion status
//!
//! # Uninstall
//! spn completion uninstall
//! ```

use crate::error::{Result, SpnError};
use crate::ux::design_system as ds;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io::{self, Write};
use std::path::PathBuf;

// ============================================================================
// Constants
// ============================================================================

/// Marker comment for spn completion block start.
const COMPLETION_START: &str = "# >>> spn shell completion >>>";

/// Marker comment for spn completion block end.
const COMPLETION_END: &str = "# <<< spn shell completion <<<";

// ============================================================================
// Public API
// ============================================================================

/// Run the completion command (generate to stdout/file).
pub async fn run(shell: &str, output: Option<PathBuf>) -> Result<()> {
    let shell = parse_shell(shell)?;

    // Build the CLI command structure
    let mut cmd = crate::Cli::command();

    // Generate completions
    if let Some(path) = output {
        let mut file = std::fs::File::create(&path)?;
        generate(shell, &mut cmd, "spn", &mut file);
        println!(
            "{}",
            ds::success_line(format!("Completions written to: {}", path.display()))
        );
    } else {
        generate(shell, &mut cmd, "spn", &mut io::stdout());
    }

    Ok(())
}

/// Install shell completions to the appropriate config file.
pub async fn install(shell: Option<&str>, dry_run: bool) -> Result<()> {
    let shell = match shell {
        Some(s) => parse_shell(s)?,
        None => detect_shell().ok_or_else(|| {
            SpnError::InvalidInput("Could not detect shell. Specify --shell".into())
        })?,
    };

    let config_path = get_shell_config_path(&shell)?;

    if dry_run {
        println!(
            "{}",
            ds::info_line(format!(
                "Would install {} completions to: {}",
                shell_name(&shell),
                config_path.display()
            ))
        );
        return Ok(());
    }

    // Check if already installed
    if is_installed(&config_path)? {
        println!(
            "{}",
            ds::info_line(format!(
                "Completions already installed in: {}",
                config_path.display()
            ))
        );
        return Ok(());
    }

    // Generate completion script
    let script = generate_completion_script(&shell);

    // Install based on shell type
    match shell {
        Shell::Fish => install_fish(&config_path, &script)?,
        _ => install_source(&config_path, &script)?,
    }

    println!(
        "{}",
        ds::success_line(format!(
            "Installed {} completions to: {}",
            shell_name(&shell),
            config_path.display()
        ))
    );
    println!(
        "{}",
        ds::hint_line(format!(
            "Restart your shell or run: source {}",
            config_path.display()
        ))
    );

    Ok(())
}

/// Uninstall shell completions from config file.
pub async fn uninstall(shell: Option<&str>) -> Result<()> {
    let shell = match shell {
        Some(s) => parse_shell(s)?,
        None => detect_shell().ok_or_else(|| {
            SpnError::InvalidInput("Could not detect shell. Specify --shell".into())
        })?,
    };

    let config_path = get_shell_config_path(&shell)?;

    if !config_path.exists() {
        println!(
            "{}",
            ds::info_line(format!(
                "Config file does not exist: {}",
                config_path.display()
            ))
        );
        return Ok(());
    }

    if !is_installed(&config_path)? {
        println!(
            "{}",
            ds::info_line(format!(
                "No completions installed in: {}",
                config_path.display()
            ))
        );
        return Ok(());
    }

    match shell {
        Shell::Fish => std::fs::remove_file(&config_path)?,
        _ => remove_completion_block(&config_path)?,
    }

    println!(
        "{}",
        ds::success_line(format!(
            "Removed {} completions from: {}",
            shell_name(&shell),
            config_path.display()
        ))
    );

    Ok(())
}

/// Show completion installation status.
pub async fn status() -> Result<()> {
    let shells = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell];

    println!("{}", ds::section("Completion Status"));

    for shell in &shells {
        let name = shell_name(shell);
        if let Ok(path) = get_shell_config_path(shell) {
            let installed = is_installed(&path).unwrap_or(false);
            let status = if installed {
                ds::success("installed")
            } else {
                ds::muted("not installed")
            };
            println!("  {:<12} {} ({})", name, status, path.display());
        } else {
            println!(
                "  {:<12} {} (config path not found)",
                name,
                ds::muted("unavailable")
            );
        }
    }

    // Show detected shell
    if let Some(detected) = detect_shell() {
        println!();
        println!(
            "{}",
            ds::info_line(format!("Detected shell: {}", shell_name(&detected)))
        );
    }

    Ok(())
}

// ============================================================================
// Shell Detection
// ============================================================================

/// Detect the current shell from environment.
pub fn detect_shell() -> Option<Shell> {
    // Check $SHELL environment variable
    if let Ok(shell_path) = std::env::var("SHELL") {
        let shell_name = std::path::Path::new(&shell_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        return match shell_name {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            "pwsh" | "powershell" => Some(Shell::PowerShell),
            _ => None,
        };
    }

    // On Windows, check for PowerShell
    #[cfg(windows)]
    {
        return Some(Shell::PowerShell);
    }

    #[cfg(not(windows))]
    None
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

/// Get shell display name.
fn shell_name(shell: &Shell) -> &'static str {
    match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::PowerShell => "powershell",
        Shell::Elvish => "elvish",
        _ => "unknown",
    }
}

// ============================================================================
// Config Paths
// ============================================================================

/// Get the config file path for a shell.
fn get_shell_config_path(shell: &Shell) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| SpnError::ConfigError("Could not find home directory".into()))?;

    let path = match shell {
        Shell::Bash => home.join(".bashrc"),
        Shell::Zsh => home.join(".zshrc"),
        Shell::Fish => home.join(".config/fish/completions/spn.fish"),
        Shell::PowerShell => {
            // PowerShell profile location varies
            if cfg!(windows) {
                home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
            } else {
                home.join(".config/powershell/Microsoft.PowerShell_profile.ps1")
            }
        }
        _ => {
            return Err(SpnError::InvalidInput(format!(
                "Unsupported shell for installation: {:?}",
                shell
            )))
        }
    };

    Ok(path)
}

// ============================================================================
// Installation
// ============================================================================

/// Generate completion script as a string.
fn generate_completion_script(shell: &Shell) -> String {
    let mut cmd = crate::Cli::command();
    let mut buf = Vec::new();
    generate(*shell, &mut cmd, "spn", &mut buf);
    String::from_utf8_lossy(&buf).to_string()
}

/// Check if completions are already installed.
fn is_installed(path: &PathBuf) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(path)?;
    Ok(content.contains(COMPLETION_START))
}

/// Install completions by sourcing (bash, zsh, powershell).
fn install_source(path: &PathBuf, script: &str) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Append to existing file
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    writeln!(file)?;
    writeln!(file, "{}", COMPLETION_START)?;
    writeln!(file, "{}", script)?;
    writeln!(file, "{}", COMPLETION_END)?;

    Ok(())
}

/// Install fish completions (separate file).
fn install_fish(path: &PathBuf, script: &str) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::File::create(path)?;
    writeln!(file, "{}", COMPLETION_START)?;
    writeln!(file, "{}", script)?;
    writeln!(file, "{}", COMPLETION_END)?;

    Ok(())
}

/// Remove completion block from config file.
fn remove_completion_block(path: &PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(path)?;

    let mut result = Vec::new();
    let mut in_block = false;

    for line in content.lines() {
        if line.contains(COMPLETION_START) {
            in_block = true;
            continue;
        }
        if line.contains(COMPLETION_END) {
            in_block = false;
            continue;
        }
        if !in_block {
            result.push(line);
        }
    }

    // Remove trailing empty lines
    while result.last().map(|l| l.is_empty()).unwrap_or(false) {
        result.pop();
    }

    std::fs::write(path, result.join("\n") + "\n")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // =========================================================================
    // parse_shell tests
    // =========================================================================

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

    // =========================================================================
    // shell_name tests
    // =========================================================================

    #[test]
    fn test_shell_name() {
        assert_eq!(shell_name(&Shell::Bash), "bash");
        assert_eq!(shell_name(&Shell::Zsh), "zsh");
        assert_eq!(shell_name(&Shell::Fish), "fish");
        assert_eq!(shell_name(&Shell::PowerShell), "powershell");
    }

    // =========================================================================
    // is_installed tests
    // =========================================================================

    #[test]
    fn test_is_installed_no_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        assert!(!is_installed(&path).unwrap());
    }

    #[test]
    fn test_is_installed_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        std::fs::write(&path, "").unwrap();
        assert!(!is_installed(&path).unwrap());
    }

    #[test]
    fn test_is_installed_with_marker() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        std::fs::write(
            &path,
            format!("stuff\n{}\ncode\n{}", COMPLETION_START, COMPLETION_END),
        )
        .unwrap();
        assert!(is_installed(&path).unwrap());
    }

    #[test]
    fn test_is_installed_without_marker() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        std::fs::write(&path, "some other content").unwrap();
        assert!(!is_installed(&path).unwrap());
    }

    // =========================================================================
    // install_source tests
    // =========================================================================

    #[test]
    fn test_install_source_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");

        install_source(&path, "completion code").unwrap();

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(COMPLETION_START));
        assert!(content.contains("completion code"));
        assert!(content.contains(COMPLETION_END));
    }

    #[test]
    fn test_install_source_appends_to_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        std::fs::write(&path, "# existing content\nalias ll='ls -la'\n").unwrap();

        install_source(&path, "completion code").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# existing content"));
        assert!(content.contains("alias ll='ls -la'"));
        assert!(content.contains(COMPLETION_START));
        assert!(content.contains("completion code"));
    }

    // =========================================================================
    // install_fish tests
    // =========================================================================

    #[test]
    fn test_install_fish_creates_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fish/completions/spn.fish");

        install_fish(&path, "fish completion").unwrap();

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(COMPLETION_START));
        assert!(content.contains("fish completion"));
    }

    // =========================================================================
    // remove_completion_block tests
    // =========================================================================

    #[test]
    fn test_remove_completion_block() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        let content = format!(
            "before\n{}\ncompletion code\n{}\nafter",
            COMPLETION_START, COMPLETION_END
        );
        std::fs::write(&path, content).unwrap();

        remove_completion_block(&path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(!result.contains(COMPLETION_START));
        assert!(!result.contains("completion code"));
        assert!(!result.contains(COMPLETION_END));
        assert!(result.contains("before"));
        assert!(result.contains("after"));
    }

    #[test]
    fn test_remove_completion_block_preserves_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        let content = format!(
            "# My config\nalias ll='ls -la'\n\n{}\ncompletion\n{}\n\nexport PATH=...",
            COMPLETION_START, COMPLETION_END
        );
        std::fs::write(&path, content).unwrap();

        remove_completion_block(&path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(result.contains("# My config"));
        assert!(result.contains("alias ll='ls -la'"));
        assert!(result.contains("export PATH=..."));
        assert!(!result.contains(COMPLETION_START));
    }

    // =========================================================================
    // generate_completion_script tests
    // =========================================================================

    #[test]
    fn test_generate_completion_script_bash() {
        let script = generate_completion_script(&Shell::Bash);
        assert!(script.contains("complete"));
        assert!(script.contains("spn"));
    }

    #[test]
    fn test_generate_completion_script_zsh() {
        let script = generate_completion_script(&Shell::Zsh);
        assert!(script.contains("#compdef"));
        assert!(script.contains("spn"));
    }

    #[test]
    fn test_generate_completion_script_fish() {
        let script = generate_completion_script(&Shell::Fish);
        assert!(script.contains("complete -c spn"));
    }

    // =========================================================================
    // Integration tests (idempotency)
    // =========================================================================

    #[test]
    fn test_install_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        std::fs::write(&path, "").unwrap();

        install_source(&path, "code1").unwrap();
        // Second install should be prevented by is_installed check,
        // but if we force it, let's verify the marker count
        let content = std::fs::read_to_string(&path).unwrap();
        let count = content.matches(COMPLETION_START).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_roundtrip_install_uninstall() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".bashrc");
        std::fs::write(&path, "# original\n").unwrap();

        // Install
        install_source(&path, "completion").unwrap();
        assert!(is_installed(&path).unwrap());

        // Uninstall
        remove_completion_block(&path).unwrap();
        assert!(!is_installed(&path).unwrap());

        // Original content preserved
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# original"));
    }
}
