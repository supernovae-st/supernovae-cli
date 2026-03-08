//! Context-aware suggestion wizard.
//!
//! Analyzes project state and suggests relevant next actions
//! to help users get the most out of spn.

use crate::error::Result;
use crate::status::{credentials, mcp, ollama};
use crate::ux::design_system as ds;

/// Suggestion with priority and action.
#[derive(Debug)]
struct Suggestion {
    /// Emoji icon.
    icon: &'static str,
    /// Short title.
    title: String,
    /// Description of why this is suggested.
    reason: String,
    /// Command to run.
    command: String,
    /// Priority (lower = more important).
    priority: u8,
}

/// Run the suggest wizard.
pub async fn run() -> Result<()> {
    println!();
    println!(
        "{}",
        ds::highlight(" spn suggest ").bold()
    );
    println!(
        "{}",
        ds::muted("Analyzing your environment...")
    );
    println!();

    let suggestions = gather_suggestions().await;

    if suggestions.is_empty() {
        println!(
            "  {} {}",
            ds::success("All set!"),
            ds::muted("No suggestions - your environment looks great.")
        );
        println!();
        return Ok(());
    }

    println!(
        "  {} {} suggestion{}:",
        ds::primary("Found"),
        suggestions.len(),
        if suggestions.len() == 1 { "" } else { "s" }
    );
    println!();

    for suggestion in &suggestions {
        println!(
            "  {} {}",
            suggestion.icon,
            ds::primary(&suggestion.title).bold()
        );
        println!(
            "    {}",
            ds::muted(&suggestion.reason)
        );
        println!(
            "    {} {}",
            ds::muted("Run:"),
            ds::highlight(&suggestion.command)
        );
        println!();
    }

    println!(
        "  {}",
        ds::muted("Tip: Run 'spn doctor' for a full system check.")
    );
    println!();

    Ok(())
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
        .filter(|p| matches!(p.status, credentials::Status::Ready | credentials::Status::Local))
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
        });
    } else if !missing.is_empty() && missing.len() <= 3 {
        // Only suggest if just a few are missing
        let missing_names: Vec<_> = missing.iter().map(|p| p.name.as_str()).collect();
        suggestions.push(Suggestion {
            icon: "🔑",
            title: format!("Configure {} provider{}", missing.len(), if missing.len() == 1 { "" } else { "s" }),
            reason: format!("Missing: {}", missing_names.join(", ")),
            command: format!("spn provider set {}", missing_names[0]),
            priority: 5,
        });
    }

    // Check for env vars that could be migrated
    let env_based: Vec<_> = providers
        .iter()
        .filter(|p| matches!(p.source, Some(credentials::Source::Env | credentials::Source::DotEnv)))
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
            title: format!("Fix {} MCP server{}", errored.len(), if errored.len() == 1 { "" } else { "s" }),
            reason: format!(
                "Server{} with errors: {}",
                if errored.len() == 1 { "" } else { "s" },
                errored.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
            ),
            command: format!("spn mcp test {}", errored[0].name),
            priority: 2,
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
        });
    } else if status.models.is_empty() {
        suggestions.push(Suggestion {
            icon: "📥",
            title: "Pull a local model".into(),
            reason: "Ollama is running but no models installed.".into(),
            command: "spn model pull llama3.2:3b".into(),
            priority: 4,
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
            },
            Suggestion {
                icon: "🔐",
                title: "High priority".into(),
                reason: "test".into(),
                command: "test".into(),
                priority: 1,
            },
        ];

        suggestions.sort_by_key(|s| s.priority);

        assert_eq!(suggestions[0].priority, 1);
        assert_eq!(suggestions[1].priority, 10);
    }
}
