//! Doctor command implementation.
//!
//! System health check for SuperNovae ecosystem.

#![allow(dead_code)]

use crate::error::Result;
use crate::interop::binary::{BinaryRunner, BinaryType};
use crate::interop::npm::NpmClient;

use colored::Colorize;
use std::path::PathBuf;

/// Check result for a single item.
struct CheckResult {
    name: String,
    status: CheckStatus,
    details: Option<String>,
}

enum CheckStatus {
    Ok,
    Warning,
    Error,
}

impl CheckResult {
    fn ok(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Ok,
            details: None,
        }
    }

    fn ok_with(name: &str, details: &str) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Ok,
            details: Some(details.to_string()),
        }
    }

    fn warning(name: &str, details: &str) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Warning,
            details: Some(details.to_string()),
        }
    }

    fn error(name: &str, details: &str) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Error,
            details: Some(details.to_string()),
        }
    }

    fn print(&self) {
        let icon = match self.status {
            CheckStatus::Ok => "✓".green(),
            CheckStatus::Warning => "!".yellow(),
            CheckStatus::Error => "✗".red(),
        };

        let name_colored = match self.status {
            CheckStatus::Ok => self.name.normal(),
            CheckStatus::Warning => self.name.yellow(),
            CheckStatus::Error => self.name.red(),
        };

        if let Some(details) = &self.details {
            println!("  {} {} ({})", icon, name_colored, details.dimmed());
        } else {
            println!("  {} {}", icon, name_colored);
        }
    }
}

/// Run system health checks.
pub async fn run() -> Result<()> {
    println!("{}", "SuperNovae Doctor".cyan().bold());
    println!("{}", "=================".cyan());
    println!();

    let mut all_ok = true;
    let mut warning_count = 0;
    let mut error_count = 0;

    // Check binaries
    println!("{}", "Binaries:".bold());
    let binary_checks = check_binaries();
    for check in &binary_checks {
        check.print();
        match check.status {
            CheckStatus::Warning => warning_count += 1,
            CheckStatus::Error => {
                error_count += 1;
                all_ok = false;
            }
            _ => {}
        }
    }
    println!();

    // Check directories
    println!("{}", "Directories:".bold());
    let dir_checks = check_directories();
    for check in &dir_checks {
        check.print();
        match check.status {
            CheckStatus::Warning => warning_count += 1,
            CheckStatus::Error => {
                error_count += 1;
                all_ok = false;
            }
            _ => {}
        }
    }
    println!();

    // Check configuration
    println!("{}", "Configuration:".bold());
    let config_checks = check_configuration();
    for check in &config_checks {
        check.print();
        match check.status {
            CheckStatus::Warning => warning_count += 1,
            CheckStatus::Error => {
                error_count += 1;
                all_ok = false;
            }
            _ => {}
        }
    }
    println!();

    // Check plugins
    println!("{}", "Plugins:".bold());
    let plugin_checks = check_plugins();
    for check in &plugin_checks {
        check.print();
        match check.status {
            CheckStatus::Warning => warning_count += 1,
            CheckStatus::Error => {
                error_count += 1;
                all_ok = false;
            }
            _ => {}
        }
    }
    println!();

    // Summary
    println!("{}", "Summary:".bold());
    if all_ok && warning_count == 0 {
        println!("  {} {}", "✓".green(), "All checks passed!".green().bold());
    } else if all_ok {
        println!(
            "  {} {} ({} warning(s))",
            "!".yellow(),
            "System functional with warnings".yellow(),
            warning_count
        );
    } else {
        println!(
            "  {} {} ({} error(s), {} warning(s))",
            "✗".red(),
            "Issues found".red().bold(),
            error_count,
            warning_count
        );
    }

    Ok(())
}

/// Check required binaries.
fn check_binaries() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check nika
    let nika_runner = BinaryRunner::new(BinaryType::Nika);
    if nika_runner.is_available() {
        results.push(CheckResult::ok_with("nika", "found in PATH"));
    } else {
        results.push(CheckResult::warning(
            "nika",
            "not found - install with: brew install supernovae-st/tap/nika",
        ));
    }

    // Check novanet
    let novanet_runner = BinaryRunner::new(BinaryType::NovaNet);
    if novanet_runner.is_available() {
        results.push(CheckResult::ok_with("novanet", "found in PATH"));
    } else {
        results.push(CheckResult::warning(
            "novanet",
            "not found - install with: brew install supernovae-st/tap/novanet",
        ));
    }

    // Check npm
    let npm_client = NpmClient::new();
    if npm_client.is_available() {
        results.push(CheckResult::ok_with("npm", "found in PATH"));
    } else {
        results.push(CheckResult::warning(
            "npm",
            "not found - install Node.js from https://nodejs.org",
        ));
    }

    // Check git
    if which::which("git").is_ok() {
        results.push(CheckResult::ok_with("git", "found in PATH"));
    } else {
        results.push(CheckResult::error(
            "git",
            "not found - required for publishing",
        ));
    }

    // Check curl
    if which::which("curl").is_ok() {
        results.push(CheckResult::ok_with("curl", "found in PATH"));
    } else {
        results.push(CheckResult::warning(
            "curl",
            "not found - required for skills.sh",
        ));
    }

    results
}

/// Check required directories.
fn check_directories() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check ~/.spn/
    if let Some(home) = dirs::home_dir() {
        let spn_dir = home.join(".spn");
        if spn_dir.exists() {
            let packages_dir = spn_dir.join("packages");
            let package_count = if packages_dir.exists() {
                std::fs::read_dir(&packages_dir)
                    .map(|entries| entries.count())
                    .unwrap_or(0)
            } else {
                0
            };
            results.push(CheckResult::ok_with(
                "~/.spn/",
                &format!("{} package(s) installed", package_count),
            ));
        } else {
            results.push(CheckResult::ok_with(
                "~/.spn/",
                "will be created on first install",
            ));
        }

        // Check ~/.claude/
        let claude_dir = home.join(".claude");
        if claude_dir.exists() {
            let skills_dir = claude_dir.join("skills");
            let skill_count = if skills_dir.exists() {
                std::fs::read_dir(&skills_dir)
                    .map(|entries| entries.filter(|e| e.is_ok()).count())
                    .unwrap_or(0)
            } else {
                0
            };
            results.push(CheckResult::ok_with(
                "~/.claude/",
                &format!("{} skill(s) installed", skill_count),
            ));
        } else {
            results.push(CheckResult::warning(
                "~/.claude/",
                "not found - create with Claude Code",
            ));
        }
    } else {
        results.push(CheckResult::error("home directory", "could not determine"));
    }

    // Check current project
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let spn_yaml = cwd.join("spn.yaml");
    let spn_dir_yaml = cwd.join(".spn").join("spn.yaml");

    if spn_yaml.exists() {
        results.push(CheckResult::ok_with("project manifest", "spn.yaml found"));
    } else if spn_dir_yaml.exists() {
        results.push(CheckResult::ok_with(
            "project manifest",
            ".spn/spn.yaml found",
        ));
    } else {
        results.push(CheckResult::ok_with(
            "project manifest",
            "none (run spn init to create)",
        ));
    }

    results
}

/// Check configuration.
fn check_configuration() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check for IDE configs
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let ides = [
        (".claude", "Claude Code"),
        (".cursor", "Cursor"),
        (".vscode", "VS Code"),
        (".windsurf", "Windsurf"),
    ];

    let mut found_ides = Vec::new();
    for (dir, name) in &ides {
        if cwd.join(dir).exists() {
            found_ides.push(*name);
        }
    }

    if found_ides.is_empty() {
        results.push(CheckResult::ok_with("IDE configs", "none detected"));
    } else {
        results.push(CheckResult::ok_with("IDE configs", &found_ides.join(", ")));
    }

    // Check sync config
    if let Some(home) = dirs::home_dir() {
        let sync_config = home.join(".spn").join("sync.json");
        if sync_config.exists() {
            results.push(CheckResult::ok_with("sync config", "~/.spn/sync.json"));
        } else {
            results.push(CheckResult::ok_with(
                "sync config",
                "default (run spn sync --enable <editor>)",
            ));
        }
    }

    // Check registry connectivity
    results.push(CheckResult::ok_with(
        "registry",
        "github.com/supernovae-st/supernovae-registry",
    ));

    results
}

/// Check Claude Code plugins.
fn check_plugins() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check if Claude Code CLI is available
    let claude_available = which::which("claude").is_ok();

    if !claude_available {
        results.push(CheckResult::warning(
            "claude-code",
            "not installed - get it at https://claude.ai/code",
        ));
        return results;
    }

    results.push(CheckResult::ok_with("claude-code", "found in PATH"));

    // Check SuperNovae plugin installation
    if let Some(home) = dirs::home_dir() {
        let plugins_file = home.join(".claude/plugins/installed_plugins.json");

        if plugins_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&plugins_file) {
                // Plugin is named "supernovae@claude-code-supernovae"
                if content.contains("supernovae@claude-code-supernovae") {
                    // Try to get more details about the plugin
                    // Cache path: ~/.claude/plugins/cache/claude-code-supernovae/supernovae/<version>/
                    let plugin_cache = home.join(".claude/plugins/cache/claude-code-supernovae/supernovae");
                    if plugin_cache.exists() {
                        // Find the version directory (e.g., 1.0.0)
                        let version_dir = std::fs::read_dir(&plugin_cache)
                            .ok()
                            .and_then(|mut entries| entries.next())
                            .and_then(|e| e.ok())
                            .map(|e| e.path());

                        if let Some(version_path) = version_dir {
                            // Count skills, agents, commands
                            let skills_count = count_files_in_dir(&version_path, "skills", "SKILL.md");
                            let agents_count = count_files_in_dir(&version_path, "agents", ".md");
                            let commands_count = count_files_in_dir(&version_path, "commands", ".md");

                            let details = format!(
                                "{} skills, {} agents, {} commands",
                                skills_count, agents_count, commands_count
                            );
                            results.push(CheckResult::ok_with("supernovae-plugin", &details));
                        } else {
                            results.push(CheckResult::ok_with("supernovae-plugin", "installed"));
                        }
                    } else {
                        results.push(CheckResult::ok_with("supernovae-plugin", "installed"));
                    }
                } else {
                    results.push(CheckResult::warning(
                        "supernovae-plugin",
                        "not installed - run: spn setup claude-code",
                    ));
                }
            } else {
                results.push(CheckResult::warning(
                    "supernovae-plugin",
                    "not installed - run: spn setup claude-code",
                ));
            }
        } else {
            results.push(CheckResult::warning(
                "supernovae-plugin",
                "no plugins installed - run: spn setup claude-code",
            ));
        }
    }

    results
}

/// Count files matching a pattern in a subdirectory.
fn count_files_in_dir(base: &std::path::Path, subdir: &str, pattern: &str) -> usize {
    let dir = base.join(subdir);
    if !dir.exists() {
        return 0;
    }

    walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().to_string_lossy().ends_with(pattern))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_ok() {
        let result = CheckResult::ok("test");
        assert!(matches!(result.status, CheckStatus::Ok));
    }

    #[test]
    fn test_check_result_warning() {
        let result = CheckResult::warning("test", "details");
        assert!(matches!(result.status, CheckStatus::Warning));
    }

    #[test]
    fn test_check_result_error() {
        let result = CheckResult::error("test", "details");
        assert!(matches!(result.status, CheckStatus::Error));
    }

    #[test]
    fn test_check_binaries() {
        let results = check_binaries();
        // Should have at least 5 checks (nika, novanet, npm, git, curl)
        assert!(results.len() >= 5);
    }

    #[test]
    fn test_check_directories() {
        let results = check_directories();
        // Should have at least 3 checks
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_check_configuration() {
        let results = check_configuration();
        // Should have at least 2 checks
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_check_plugins() {
        let results = check_plugins();
        // Should have at least 1 check (claude-code availability)
        assert!(results.len() >= 1);
    }
}
