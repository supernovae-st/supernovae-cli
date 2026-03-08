//! Context-aware smart suggestion wizard.
//!
//! Analyzes project state and suggests relevant next actions
//! to help users get the most out of spn. Supports interactive
//! mode for direct execution of suggestions.

use crate::error::Result;
use crate::status::{credentials, mcp, ollama};
use crate::ux::design_system as ds;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use std::process::Command;

/// Suggestion category for grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Security,
    Setup,
    Tools,
    Project,
}

impl Category {
    fn icon(&self) -> &'static str {
        match self {
            Category::Security => "🔐",
            Category::Setup => "⚙️",
            Category::Tools => "🔧",
            Category::Project => "📦",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Category::Security => "Security",
            Category::Setup => "Setup",
            Category::Tools => "Tools",
            Category::Project => "Project",
        }
    }
}

/// Suggestion with priority and action.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Suggestion {
    /// Emoji icon.
    pub icon: &'static str,
    /// Short title.
    pub title: String,
    /// Description of why this is suggested.
    pub reason: String,
    /// Command to run.
    pub command: String,
    /// Priority (lower = more important).
    pub priority: u8,
    /// Category for grouping.
    pub category: Category,
    /// Whether this can be run in background.
    pub background: bool,
}

/// Run the suggest wizard.
pub async fn run(interactive: bool, category: Option<String>) -> Result<()> {
    println!();
    print_wizard_banner();

    println!("{}", ds::muted("Analyzing your environment..."));
    println!();

    let mut suggestions = gather_suggestions().await;

    // Filter by category if specified
    if let Some(ref cat) = category {
        suggestions.retain(|s| s.category.name().to_lowercase() == cat.to_lowercase());
    }

    if suggestions.is_empty() {
        println!(
            "{}",
            ds::success(
                "╭─────────────────────────────────────────────────────────────────────────────╮"
            )
        );
        println!(
            "{}",
            ds::success(
                "│  ✅ ALL SET!                                                                │"
            )
        );
        println!(
            "{}",
            ds::success(
                "│                                                                             │"
            )
        );
        println!(
            "{}",
            ds::success(
                "│  No suggestions - your environment looks great.                             │"
            )
        );
        println!(
            "{}",
            ds::success(
                "╰─────────────────────────────────────────────────────────────────────────────╯"
            )
        );
        println!();
        return Ok(());
    }

    // Group by category
    let categories = [
        Category::Security,
        Category::Setup,
        Category::Tools,
        Category::Project,
    ];

    if interactive {
        run_interactive_mode(&suggestions).await
    } else {
        // Print grouped suggestions
        for cat in &categories {
            let cat_suggestions: Vec<_> =
                suggestions.iter().filter(|s| s.category == *cat).collect();

            if cat_suggestions.is_empty() {
                continue;
            }

            println!(
                "  {} {} ({})",
                cat.icon(),
                ds::highlight(cat.name()).bold(),
                cat_suggestions.len()
            );
            println!();

            for suggestion in cat_suggestions {
                println!("    {} {}", suggestion.icon, ds::primary(&suggestion.title));
                println!("      {}", ds::muted(&suggestion.reason));
                println!(
                    "      {} {}",
                    ds::muted("→"),
                    ds::highlight(&suggestion.command)
                );
                println!();
            }
        }

        println!(
            "  {}",
            ds::muted("Tip: Run 'spn suggest --interactive' to execute suggestions directly.")
        );
        println!();

        Ok(())
    }
}

/// Print the wizard banner.
fn print_wizard_banner() {
    println!(
        "{}",
        ds::primary(
            "╭─────────────────────────────────────────────────────────────────────────────╮"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "│  🧙 SMART WIZARD                                                            │"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "│                                                                             │"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "│  Context-aware suggestions to optimize your spn environment                │"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "╰─────────────────────────────────────────────────────────────────────────────╯"
        )
        .cyan()
        .bold()
    );
    println!();
}

/// Run interactive mode where user can select and execute suggestions.
async fn run_interactive_mode(suggestions: &[Suggestion]) -> Result<()> {
    let theme = ColorfulTheme::default();

    println!(
        "  {} {} suggestion{}",
        ds::primary("Found"),
        suggestions.len(),
        if suggestions.len() == 1 { "" } else { "s" }
    );
    println!();

    // Build selection items
    let items: Vec<String> = suggestions
        .iter()
        .map(|s| format!("{} {} - {}", s.icon, s.title, s.reason))
        .collect();

    let selections = MultiSelect::with_theme(&theme)
        .with_prompt("Select suggestions to run (Space to select, Enter to confirm)")
        .items(&items)
        .interact_opt()
        .map_err(|e| crate::error::SpnError::InvalidInput(e.to_string()))?;

    match selections {
        Some(indices) if !indices.is_empty() => {
            println!();
            println!(
                "  {} Running {} suggestion{}...",
                ds::primary("→"),
                indices.len(),
                if indices.len() == 1 { "" } else { "s" }
            );
            println!();

            let mut success = 0;
            let mut failed = 0;

            for idx in indices {
                let suggestion = &suggestions[idx];
                execute_suggestion(suggestion, &mut success, &mut failed);
            }

            println!();
            println!(
                "  {} Completed: {} succeeded, {} failed",
                if failed == 0 {
                    ds::success("✓")
                } else {
                    ds::warning("⚠")
                },
                success,
                failed
            );
        }
        _ => {
            println!();
            println!("{}", ds::muted("No suggestions selected."));
        }
    }

    println!();
    Ok(())
}

/// Execute a single suggestion.
fn execute_suggestion(suggestion: &Suggestion, success: &mut usize, failed: &mut usize) {
    println!(
        "  {} {} {}",
        ds::primary("Running:"),
        suggestion.icon,
        ds::highlight(&suggestion.command)
    );

    // Parse command
    let parts: Vec<&str> = suggestion.command.split_whitespace().collect();
    if parts.is_empty() {
        println!("    {} Invalid command", ds::error("✗"));
        *failed += 1;
        return;
    }

    let cmd = parts[0];
    let args = &parts[1..];

    match Command::new(cmd).args(args).status() {
        Ok(status) if status.success() => {
            println!("    {} Success", ds::success("✓"));
            *success += 1;
        }
        Ok(status) => {
            println!(
                "    {} Failed (exit code: {:?})",
                ds::error("✗"),
                status.code()
            );
            *failed += 1;
        }
        Err(e) => {
            println!("    {} Error: {}", ds::error("✗"), e);
            *failed += 1;
        }
    }
}

/// Gather suggestions based on current environment.
async fn gather_suggestions() -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    // Check providers
    check_providers(&mut suggestions).await;

    // Check MCP servers
    check_mcp_servers(&mut suggestions).await;

    // Check Ollama
    check_ollama(&mut suggestions).await;

    // Check project setup
    check_project(&mut suggestions);

    // Sort by priority
    suggestions.sort_by_key(|s| s.priority);

    suggestions
}

/// Check provider configuration.
async fn check_providers(suggestions: &mut Vec<Suggestion>) {
    let providers = credentials::collect().await;

    // Count configured vs missing
    let configured = providers
        .iter()
        .filter(|p| {
            matches!(
                p.status,
                credentials::Status::Ready | credentials::Status::Local
            )
        })
        .count();
    let missing: Vec<_> = providers
        .iter()
        .filter(|p| matches!(p.status, credentials::Status::NotSet))
        .collect();

    if configured == 0 {
        suggestions.push(Suggestion {
            icon: "🔐",
            title: "Set up API keys".into(),
            reason: "No API providers configured. You need at least one LLM provider.".into(),
            command: "spn setup".into(),
            priority: 1,
            category: Category::Security,
            background: false,
        });
    } else if !missing.is_empty() && missing.len() <= 3 {
        // Only suggest if just a few are missing
        let missing_names: Vec<_> = missing.iter().map(|p| p.name.as_str()).collect();
        suggestions.push(Suggestion {
            icon: "🔑",
            title: format!(
                "Configure {} provider{}",
                missing.len(),
                if missing.len() == 1 { "" } else { "s" }
            ),
            reason: format!("Missing: {}", missing_names.join(", ")),
            command: format!("spn provider set {}", missing_names[0]),
            priority: 5,
            category: Category::Security,
            background: false,
        });
    }

    // Check for env vars that could be migrated
    let env_based: Vec<_> = providers
        .iter()
        .filter(|p| {
            matches!(
                p.source,
                Some(credentials::Source::Env | credentials::Source::DotEnv)
            )
        })
        .collect();

    if !env_based.is_empty() {
        suggestions.push(Suggestion {
            icon: "🔒",
            title: "Migrate API keys to keychain".into(),
            reason: format!(
                "{} key{} stored in env vars. Keychain is more secure.",
                env_based.len(),
                if env_based.len() == 1 { "" } else { "s" }
            ),
            command: "spn provider migrate".into(),
            priority: 6,
            category: Category::Security,
            background: false,
        });
    }
}

/// Check MCP server configuration.
async fn check_mcp_servers(suggestions: &mut Vec<Suggestion>) {
    let servers = mcp::collect().await;

    if servers.is_empty() {
        suggestions.push(Suggestion {
            icon: "🔌",
            title: "Add an MCP server".into(),
            reason: "No MCP servers configured. MCP extends your AI capabilities.".into(),
            command: "spn mcp add neo4j".into(),
            priority: 3,
            category: Category::Tools,
            background: false,
        });
    }

    // Check for servers with errors
    let errored: Vec<_> = servers
        .iter()
        .filter(|s| matches!(s.status, mcp::ServerStatus::Error))
        .collect();

    if !errored.is_empty() {
        suggestions.push(Suggestion {
            icon: "⚠️",
            title: format!(
                "Fix {} MCP server{}",
                errored.len(),
                if errored.len() == 1 { "" } else { "s" }
            ),
            reason: format!(
                "Server{} with errors: {}",
                if errored.len() == 1 { "" } else { "s" },
                errored
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            command: format!("spn mcp test {}", errored[0].name),
            priority: 2,
            category: Category::Tools,
            background: false,
        });
    }
}

/// Check Ollama status.
async fn check_ollama(suggestions: &mut Vec<Suggestion>) {
    let status = ollama::collect().await;

    if !status.running {
        suggestions.push(Suggestion {
            icon: "🦙",
            title: "Start Ollama".into(),
            reason: "Ollama is not running. Local models require Ollama.".into(),
            command: "ollama serve".into(),
            priority: 4,
            category: Category::Tools,
            background: true,
        });
    } else if status.models.is_empty() {
        suggestions.push(Suggestion {
            icon: "📥",
            title: "Pull a local model".into(),
            reason: "Ollama is running but no models installed.".into(),
            command: "spn model pull llama3.2:3b".into(),
            priority: 4,
            category: Category::Tools,
            background: true,
        });
    }
}

/// Check project configuration.
fn check_project(suggestions: &mut Vec<Suggestion>) {
    // Check for spn.yaml
    let spn_yaml = std::path::Path::new("spn.yaml");
    let spn_dir = std::path::Path::new(".spn");

    if !spn_yaml.exists() && !spn_dir.exists() {
        suggestions.push(Suggestion {
            icon: "📦",
            title: "Initialize project".into(),
            reason: "No spn.yaml found. Initialize to track packages.".into(),
            command: "spn init".into(),
            priority: 7,
            category: Category::Project,
            background: false,
        });
    }

    // Check for editor sync
    let claude_dir = std::path::Path::new(".claude");
    let vscode_dir = std::path::Path::new(".vscode");

    if !claude_dir.exists() && !vscode_dir.exists() {
        suggestions.push(Suggestion {
            icon: "🔄",
            title: "Sync to your editor".into(),
            reason: "No editor config found. Sync MCP servers to your IDE.".into(),
            command: "spn sync --interactive".into(),
            priority: 8,
            category: Category::Setup,
            background: false,
        });
    }

    // Check for nika workflows
    check_nika_workflows(suggestions);

    // Check for novanet schema
    check_novanet_schema(suggestions);
}

/// Check for Nika workflow files.
fn check_nika_workflows(suggestions: &mut Vec<Suggestion>) {
    // Look for .nika.yaml files in current directory
    let has_nika = std::fs::read_dir(".")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().ends_with(".nika.yaml"))
        })
        .unwrap_or(false);

    // Check if nika is installed
    let has_nika_cli = std::process::Command::new("nika")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_nika && !has_nika_cli {
        suggestions.push(Suggestion {
            icon: "🦋",
            title: "Install Nika".into(),
            reason: "Found .nika.yaml files but Nika is not installed.".into(),
            command: "spn setup nika".into(),
            priority: 3,
            category: Category::Setup,
            background: false,
        });
    }
}

/// Check for NovaNet schema files.
fn check_novanet_schema(suggestions: &mut Vec<Suggestion>) {
    // Look for brain/ directory or schema files
    let has_schema = std::path::Path::new("brain/models").exists()
        || std::path::Path::new("packages/core/models").exists();

    // Check if novanet is installed
    let has_novanet_cli = std::process::Command::new("novanet")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_schema && !has_novanet_cli {
        suggestions.push(Suggestion {
            icon: "🧠",
            title: "Install NovaNet".into(),
            reason: "Found schema files but NovaNet is not installed.".into(),
            command: "spn setup novanet".into(),
            priority: 3,
            category: Category::Setup,
            background: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_priority_order() {
        let mut suggestions = vec![
            Suggestion {
                icon: "📦",
                title: "Low priority".into(),
                reason: "test".into(),
                command: "test".into(),
                priority: 10,
                category: Category::Project,
                background: false,
            },
            Suggestion {
                icon: "🔐",
                title: "High priority".into(),
                reason: "test".into(),
                command: "test".into(),
                priority: 1,
                category: Category::Security,
                background: false,
            },
        ];

        suggestions.sort_by_key(|s| s.priority);

        assert_eq!(suggestions[0].priority, 1);
        assert_eq!(suggestions[1].priority, 10);
    }

    #[test]
    fn test_category_icons() {
        assert_eq!(Category::Security.icon(), "🔐");
        assert_eq!(Category::Setup.icon(), "⚙️");
        assert_eq!(Category::Tools.icon(), "🔧");
        assert_eq!(Category::Project.icon(), "📦");
    }

    #[test]
    fn test_category_names() {
        assert_eq!(Category::Security.name(), "Security");
        assert_eq!(Category::Setup.name(), "Setup");
        assert_eq!(Category::Tools.name(), "Tools");
        assert_eq!(Category::Project.name(), "Project");
    }
}
