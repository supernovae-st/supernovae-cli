//! Doctor command implementation.
//!
//! System health check for SuperNovae ecosystem.
//! Provides clear, actionable diagnostics.

#![allow(dead_code)]

use crate::error::Result;
use crate::interop::binary::{BinaryRunner, BinaryType};
use crate::interop::npm::NpmClient;
use crate::ux;

use console::style;
use std::path::PathBuf;
use std::time::Instant;

/// Check result for a single item.
struct Check {
    name: String,
    status: Status,
    detail: Option<String>,
    hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Ok,
    Warning,
    Error,
}

impl Check {
    fn ok(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: Status::Ok,
            detail: None,
            hint: None,
        }
    }

    fn ok_with(name: &str, detail: &str) -> Self {
        Self {
            name: name.to_string(),
            status: Status::Ok,
            detail: Some(detail.to_string()),
            hint: None,
        }
    }

    fn warn(name: &str, detail: &str) -> Self {
        Self {
            name: name.to_string(),
            status: Status::Warning,
            detail: Some(detail.to_string()),
            hint: None,
        }
    }

    fn warn_with_hint(name: &str, detail: &str, hint: &str) -> Self {
        Self {
            name: name.to_string(),
            status: Status::Warning,
            detail: Some(detail.to_string()),
            hint: Some(hint.to_string()),
        }
    }

    fn error(name: &str, detail: &str) -> Self {
        Self {
            name: name.to_string(),
            status: Status::Error,
            detail: Some(detail.to_string()),
            hint: None,
        }
    }

    fn error_with_hint(name: &str, detail: &str, hint: &str) -> Self {
        Self {
            name: name.to_string(),
            status: Status::Error,
            detail: Some(detail.to_string()),
            hint: Some(hint.to_string()),
        }
    }

    fn print(&self) {
        let (icon, name_style) = match self.status {
            Status::Ok => (style("✓").green().bold(), style(&self.name).green()),
            Status::Warning => (style("!").yellow().bold(), style(&self.name).yellow()),
            Status::Error => (style("✗").red().bold(), style(&self.name).red()),
        };

        if let Some(detail) = &self.detail {
            println!("  {} {} {}", icon, name_style, style(detail).dim());
        } else {
            println!("  {} {}", icon, name_style);
        }

        if let Some(hint) = &self.hint {
            println!("    {} {}", style("→").cyan(), style(hint).cyan());
        }
    }
}

/// Run system health checks.
pub async fn run() -> Result<()> {
    let start = Instant::now();

    // Header
    println!();
    println!(
        "  {}{}{}",
        style("spn doctor").cyan().bold(),
        style(" · ").dim(),
        style("System Health Check").dim()
    );
    println!();

    let mut errors = 0;
    let mut warnings = 0;
    let mut ok_count = 0;

    // ─── TOOLS ───────────────────────────────────────────────────────────────
    println!("{}", style("  Tools").bold());
    println!("  {}", style("─".repeat(50)).dim());

    let tool_checks = check_tools();
    for check in &tool_checks {
        check.print();
        match check.status {
            Status::Ok => ok_count += 1,
            Status::Warning => warnings += 1,
            Status::Error => errors += 1,
        }
    }
    println!();

    // ─── ECOSYSTEM ───────────────────────────────────────────────────────────
    println!("{}", style("  Ecosystem").bold());
    println!("  {}", style("─".repeat(50)).dim());

    let ecosystem_checks = check_ecosystem();
    for check in &ecosystem_checks {
        check.print();
        match check.status {
            Status::Ok => ok_count += 1,
            Status::Warning => warnings += 1,
            Status::Error => errors += 1,
        }
    }
    println!();

    // ─── STORAGE ─────────────────────────────────────────────────────────────
    println!("{}", style("  Storage").bold());
    println!("  {}", style("─".repeat(50)).dim());

    let storage_checks = check_storage();
    for check in &storage_checks {
        check.print();
        match check.status {
            Status::Ok => ok_count += 1,
            Status::Warning => warnings += 1,
            Status::Error => errors += 1,
        }
    }
    println!();

    // ─── PROJECT ─────────────────────────────────────────────────────────────
    println!("{}", style("  Project").bold());
    println!("  {}", style("─".repeat(50)).dim());

    let project_checks = check_project();
    for check in &project_checks {
        check.print();
        match check.status {
            Status::Ok => ok_count += 1,
            Status::Warning => warnings += 1,
            Status::Error => errors += 1,
        }
    }
    println!();

    // ─── SUMMARY ─────────────────────────────────────────────────────────────
    let elapsed = start.elapsed();
    let total = ok_count + warnings + errors;

    println!("  {}", style("─".repeat(50)).dim());

    if errors == 0 && warnings == 0 {
        println!(
            "  {} {} {} {}",
            style("✓").green().bold(),
            style("All systems operational").green().bold(),
            style(format!("({} checks in {:?})", total, elapsed)).dim(),
            style("✦").cyan()
        );
    } else if errors == 0 {
        println!(
            "  {} {} {}",
            style("!").yellow().bold(),
            style(format!("System ready with {} warning(s)", warnings)).yellow(),
            style(format!("({} checks in {:?})", total, elapsed)).dim()
        );
    } else {
        println!(
            "  {} {} {}",
            style("✗").red().bold(),
            style(format!(
                "{} error(s), {} warning(s) found",
                errors, warnings
            ))
            .red()
            .bold(),
            style(format!("({} checks in {:?})", total, elapsed)).dim()
        );
    }

    println!();

    // Show next steps if there are issues
    if errors > 0 || warnings > 0 {
        ux::next_steps(&[
            ("spn setup", "Interactive setup wizard"),
            ("spn topic", "Browse help topics"),
        ]);
    }

    // Exit with error code for CI
    if errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Check required tools.
fn check_tools() -> Vec<Check> {
    let mut checks = Vec::new();

    // Check nika
    let nika = BinaryRunner::new(BinaryType::Nika);
    if nika.is_available() {
        // Try to get version
        if let Ok(output) = std::process::Command::new("nika").arg("--version").output() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.split_whitespace().last().unwrap_or("?");
            checks.push(Check::ok_with("nika", &format!("v{}", version)));
        } else {
            checks.push(Check::ok_with("nika", "installed"));
        }
    } else {
        checks.push(Check::warn_with_hint(
            "nika",
            "not found",
            "brew install supernovae-st/tap/nika",
        ));
    }

    // Check novanet
    let novanet = BinaryRunner::new(BinaryType::NovaNet);
    if novanet.is_available() {
        if let Ok(output) = std::process::Command::new("novanet")
            .arg("--version")
            .output()
        {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.split_whitespace().last().unwrap_or("?");
            checks.push(Check::ok_with("novanet", &format!("v{}", version)));
        } else {
            checks.push(Check::ok_with("novanet", "installed"));
        }
    } else {
        checks.push(Check::warn_with_hint(
            "novanet",
            "not found (optional)",
            "brew install supernovae-st/tap/novanet",
        ));
    }

    // Check npm
    let npm = NpmClient::new();
    if npm.is_available() {
        if let Ok(output) = std::process::Command::new("npm").arg("--version").output() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            checks.push(Check::ok_with("npm", &format!("v{}", version)));
        } else {
            checks.push(Check::ok_with("npm", "installed"));
        }
    } else {
        checks.push(Check::warn_with_hint(
            "npm",
            "not found",
            "https://nodejs.org",
        ));
    }

    // Check git
    if which::which("git").is_ok() {
        if let Ok(output) = std::process::Command::new("git").arg("--version").output() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim().strip_prefix("git version ").unwrap_or("?");
            checks.push(Check::ok_with("git", &format!("v{}", version)));
        } else {
            checks.push(Check::ok_with("git", "installed"));
        }
    } else {
        checks.push(Check::error_with_hint(
            "git",
            "required for publishing",
            "https://git-scm.com",
        ));
    }

    // Check ollama (optional)
    if which::which("ollama").is_ok() {
        if let Ok(output) = std::process::Command::new("ollama")
            .arg("--version")
            .output()
        {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.split_whitespace().last().unwrap_or("?");
            checks.push(Check::ok_with("ollama", &format!("v{}", version)));
        } else {
            checks.push(Check::ok_with("ollama", "installed"));
        }
    }

    checks
}

/// Check ecosystem components.
fn check_ecosystem() -> Vec<Check> {
    let mut checks = Vec::new();

    // Check Claude Code
    if which::which("claude").is_ok() {
        if let Ok(output) = std::process::Command::new("claude")
            .arg("--version")
            .output()
        {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            checks.push(Check::ok_with("claude", &format!("v{}", version)));
        } else {
            checks.push(Check::ok_with("claude", "installed"));
        }

        // Check SuperNovae plugin
        if let Some(home) = dirs::home_dir() {
            let plugins_file = home.join(".claude/plugins/installed_plugins.json");
            if plugins_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&plugins_file) {
                    if content.contains("supernovae") {
                        // Count assets
                        let cache =
                            home.join(".claude/plugins/cache/claude-code-supernovae/supernovae");
                        if cache.exists() {
                            let version_dir = std::fs::read_dir(&cache)
                                .ok()
                                .and_then(|mut e| e.next())
                                .and_then(|e| e.ok())
                                .map(|e| e.path());

                            if let Some(path) = version_dir {
                                let skills = count_in_dir(&path, "skills", ".md");
                                let agents = count_in_dir(&path, "agents", ".md");
                                let cmds = count_in_dir(&path, "commands", ".md");
                                checks.push(Check::ok_with(
                                    "spn-plugin",
                                    &format!(
                                        "{} skills, {} agents, {} commands",
                                        skills, agents, cmds
                                    ),
                                ));
                            } else {
                                checks.push(Check::ok_with("spn-plugin", "active"));
                            }
                        } else {
                            checks.push(Check::ok_with("spn-plugin", "active"));
                        }
                    } else {
                        checks.push(Check::warn_with_hint(
                            "spn-plugin",
                            "not installed",
                            "spn setup claude-code",
                        ));
                    }
                }
            }
        }
    } else {
        checks.push(Check::warn_with_hint(
            "claude",
            "not installed",
            "https://claude.ai/code",
        ));
    }

    checks
}

/// Check storage directories.
fn check_storage() -> Vec<Check> {
    let mut checks = Vec::new();

    // ~/.spn/
    if let Ok(paths) = spn_client::SpnPaths::new() {
        if paths.root().exists() {
            let pkg_count = paths
                .packages_dir()
                .read_dir()
                .map(|d| d.count())
                .unwrap_or(0);
            checks.push(Check::ok_with(
                "~/.spn/",
                &format!("{} packages", pkg_count),
            ));
        } else {
            checks.push(Check::ok_with("~/.spn/", "ready (will create on install)"));
        }
    } else {
        checks.push(Check::error("~/.spn/", "cannot determine home directory"));
    }

    // ~/.claude/
    if let Some(home) = dirs::home_dir() {
        let claude_dir = home.join(".claude");
        if claude_dir.exists() {
            let skills_dir = claude_dir.join("skills");
            let skill_count = skills_dir
                .read_dir()
                .map(|d| d.filter_map(|e| e.ok()).count())
                .unwrap_or(0);
            checks.push(Check::ok_with(
                "~/.claude/",
                &format!("{} skills", skill_count),
            ));
        } else {
            checks.push(Check::ok_with("~/.claude/", "not created yet"));
        }
    }

    checks
}

/// Check current project.
fn check_project() -> Vec<Check> {
    let mut checks = Vec::new();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Check manifest
    let manifest = cwd.join("spn.yaml");
    let manifest_alt = cwd.join(".spn").join("spn.yaml");

    if manifest.exists() {
        checks.push(Check::ok_with("manifest", "spn.yaml"));
    } else if manifest_alt.exists() {
        checks.push(Check::ok_with("manifest", ".spn/spn.yaml"));
    } else {
        checks.push(Check::ok_with("manifest", "none (spn init to create)"));
    }

    // Check IDE configs
    let ides: Vec<(&str, &str)> = vec![
        (".claude", "Claude"),
        (".cursor", "Cursor"),
        (".vscode", "VS Code"),
        (".windsurf", "Windsurf"),
    ];

    let found: Vec<&str> = ides
        .iter()
        .filter(|(dir, _)| cwd.join(dir).exists())
        .map(|(_, name)| *name)
        .collect();

    if found.is_empty() {
        checks.push(Check::ok_with("IDE", "none detected"));
    } else {
        checks.push(Check::ok_with("IDE", &found.join(", ")));
    }

    // Check git
    if cwd.join(".git").exists() {
        checks.push(Check::ok_with("git", "repository"));
    }

    checks
}

/// Count files in a subdirectory matching a pattern.
fn count_in_dir(base: &std::path::Path, subdir: &str, ext: &str) -> usize {
    let dir = base.join(subdir);
    if !dir.exists() {
        return 0;
    }
    walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().to_string_lossy().ends_with(ext))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_ok() {
        let c = Check::ok("test");
        assert_eq!(c.status, Status::Ok);
    }

    #[test]
    fn test_check_warn() {
        let c = Check::warn("test", "detail");
        assert_eq!(c.status, Status::Warning);
    }

    #[test]
    fn test_check_error() {
        let c = Check::error("test", "detail");
        assert_eq!(c.status, Status::Error);
    }

    #[test]
    fn test_check_tools() {
        let checks = check_tools();
        assert!(!checks.is_empty());
    }

    #[test]
    fn test_check_storage() {
        let checks = check_storage();
        assert!(!checks.is_empty());
    }

    #[test]
    fn test_check_project() {
        let checks = check_project();
        assert!(!checks.is_empty());
    }
}
