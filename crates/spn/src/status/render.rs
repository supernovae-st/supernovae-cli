//! ASCII renderer for status dashboard.
//!
//! Renders the complete status dashboard with:
//! - Box drawing characters
//! - Progress bars
//! - Color coding

use crate::ux::design_system as ds;

use super::{
    credentials::{CredentialStatus, CredentialType, Source, Status as CredStatus},
    daemon::DaemonStatus,
    mcp::McpServerStatus,
    ollama::OllamaStatus,
    StatusSummary, SystemStatus,
};

/// Width of the main content area.
const WIDTH: usize = 78;

/// Render the complete status dashboard.
pub fn render(status: &SystemStatus) {
    render_header();
    println!();
    render_ollama(&status.ollama);
    println!();
    render_credentials(&status.credentials);
    println!();
    render_mcp_servers(&status.mcp_servers);
    println!();
    render_daemon(&status.daemon);
    println!();
    render_footer(&status.summary());
}

fn render_header() {
    println!(
        "{}",
        ds::primary(
            "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓"
        )
    );
    println!(
        "{}",
        ds::primary(
            "┃  ✦ spn status                                    The Agentic AI Toolkit  ✦  ┃"
        )
    );
    println!(
        "{}",
        ds::primary(
            "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛"
        )
    );
}

fn render_ollama(ollama: &OllamaStatus) {
    println!(
        "┌─ {} ────────────────────────────────────────────────────────────────┐",
        ds::primary("🦙 LOCAL MODELS")
    );
    println!("│{:width$}│", "", width = WIDTH);

    if ollama.running {
        let status = format!(
            "  Ollama → {}                            {}",
            ds::highlight(&ollama.endpoint),
            ds::success("✅ running")
        );
        println!("│{:<width$}│", status, width = WIDTH);

        // Memory bar
        if ollama.memory_used > 0 {
            let used_gb = ollama.memory_used as f64 / 1024.0 / 1024.0 / 1024.0;
            let total_gb = ollama.memory_total as f64 / 1024.0 / 1024.0 / 1024.0;
            let percent = (ollama.memory_used as f64 / ollama.memory_total as f64 * 100.0) as usize;
            let bar = progress_bar(percent, 16);
            let mem_line = format!(
                "  Memory  {:.1} / {:.1} GB                    {}  {}%",
                used_gb, total_gb, bar, percent
            );
            println!("│{:<width$}│", mem_line, width = WIDTH);
        }

        println!("│{:width$}│", "", width = WIDTH);

        if ollama.models.is_empty() {
            println!(
                "│  {}│",
                ds::muted(
                    "No models installed. Run: spn model pull llama3.2                       "
                )
            );
        } else {
            println!(
                "│  {}│",
                ds::highlight(
                    "Models                                                                    "
                )
            );
            for (i, model) in ollama.models.iter().enumerate() {
                let prefix = if i == ollama.models.len() - 1 {
                    "└──"
                } else {
                    "├──"
                };
                let indicator = if model.loaded { "●" } else { "○" };
                let size_mb = model.size / 1024 / 1024;
                let size_str = if size_mb > 1024 {
                    format!("{:.1} GB", size_mb as f64 / 1024.0)
                } else {
                    format!("{} MB", size_mb)
                };
                let loaded_str = if model.loaded { "  ← active" } else { "" };
                let line = format!(
                    "  {} {} {:<18} {:>8}{}",
                    prefix, indicator, model.name, size_str, loaded_str
                );
                println!("│{:<width$}│", line, width = WIDTH);
            }
        }
    } else {
        println!(
            "│  Ollama → {}                              {}│",
            ds::muted("localhost:11434"),
            ds::warning("❌ not running")
        );
        println!("│{:width$}│", "", width = WIDTH);
        println!(
            "│  {}│",
            ds::muted("Start with: ollama serve                                                  ")
        );
    }

    println!("│{:width$}│", "", width = WIDTH);
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
}

fn render_credentials(credentials: &[CredentialStatus]) {
    println!(
        "┌─ {} ─────────────────────────────────────────────────────────────────┐",
        ds::primary("🔑 CREDENTIALS")
    );
    println!("│{:width$}│", "", width = WIDTH);
    println!(
        "│  {:<14}{:<7}{:<12}{:<12}{:<30}│",
        ds::muted("Name"),
        ds::muted("Type"),
        ds::muted("Status"),
        ds::muted("Source"),
        ds::muted("Endpoint")
    );
    println!(
        "│  {}│",
        ds::muted("──────────────────────────────────────────────────────────────────────────")
    );

    // Separate LLM and MCP credentials
    let llm_creds: Vec<_> = credentials
        .iter()
        .filter(|c| c.credential_type == CredentialType::Llm)
        .collect();
    let mcp_creds: Vec<_> = credentials
        .iter()
        .filter(|c| c.credential_type == CredentialType::Mcp)
        .collect();

    for cred in &llm_creds {
        render_credential_line(cred);
    }

    if !llm_creds.is_empty() && !mcp_creds.is_empty() {
        println!(
            "│  {}│",
            ds::muted("──────────────────────────────────────────────────────────────────────────")
        );
    }

    for cred in &mcp_creds {
        render_credential_line(cred);
    }

    println!("│{:width$}│", "", width = WIDTH);

    // Summary
    let configured = credentials
        .iter()
        .filter(|c| c.status == CredStatus::Ready || c.status == CredStatus::Local)
        .count();
    let total = credentials.len();

    let keychain_count = credentials
        .iter()
        .filter(|c| c.source == Some(Source::Keychain))
        .count();
    let env_count = credentials
        .iter()
        .filter(|c| c.source == Some(Source::Env))
        .count();
    let dotenv_count = credentials
        .iter()
        .filter(|c| c.source == Some(Source::DotEnv))
        .count();
    let local_count = credentials
        .iter()
        .filter(|c| c.source == Some(Source::Local))
        .count();

    let summary = format!(
        "  {}/{} configured   │   🔐 {} keychain   📦 {} env   📄 {} .env   🦙 {} local",
        configured, total, keychain_count, env_count, dotenv_count, local_count
    );
    println!("│{:<width$}│", summary, width = WIDTH);

    println!("│{:width$}│", "", width = WIDTH);
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
}

fn render_credential_line(cred: &CredentialStatus) {
    let status_icon = match cred.status {
        CredStatus::Ready => ds::success("✅ ready"),
        CredStatus::Local => ds::success("✅ local"),
        CredStatus::NotSet => ds::muted("❌ ──"),
    };

    let source_str = cred
        .source
        .map(|s| format!("{} {}", s.icon(), s.label()))
        .unwrap_or_else(|| "──".to_string());

    let endpoint = cred.endpoint.as_deref().unwrap_or("──");

    let type_str = match cred.credential_type {
        CredentialType::Llm => "LLM",
        CredentialType::Mcp => "MCP",
    };

    let line = format!(
        "  {:<14}{:<7}{:<12}{:<12}{:<30}",
        cred.name, type_str, status_icon, source_str, endpoint
    );
    println!("│{}│", line);
}

fn render_mcp_servers(servers: &[McpServerStatus]) {
    println!(
        "┌─ {} ───────────────────────────────────────────────────────────────┐",
        ds::primary("🔌 MCP SERVERS")
    );
    println!("│{:width$}│", "", width = WIDTH);

    if servers.is_empty() {
        println!(
            "│  {}│",
            ds::muted("No MCP servers configured. Run: spn mcp add neo4j                        ")
        );
    } else {
        println!(
            "│  {:<14}{:<12}{:<12}{:<20}{:<16}│",
            ds::muted("Server"),
            ds::muted("Status"),
            ds::muted("Transport"),
            ds::muted("Command"),
            ds::muted("Credential")
        );
        println!(
            "│  {}│",
            ds::muted("──────────────────────────────────────────────────────────────────────────")
        );

        for server in servers {
            let status_str = format!("{} {}", server.status.icon(), server.status.label());
            let transport = match server.transport {
                super::mcp::Transport::Stdio => "stdio",
                super::mcp::Transport::Http => "http",
                super::mcp::Transport::Websocket => "ws",
            };
            let cred = server
                .credential
                .as_ref()
                .map(|c| format!("→ {}", c))
                .unwrap_or_else(|| "(no key)".to_string());

            // Truncate command if too long
            let cmd = if server.command.len() > 18 {
                format!("{}…", &server.command[..17])
            } else {
                server.command.clone()
            };

            let line = format!(
                "  {:<14}{:<12}{:<12}{:<20}{:<16}",
                server.name, status_str, transport, cmd, cred
            );
            println!("│{}│", line);
        }

        println!("│{:width$}│", "", width = WIDTH);

        let active = servers
            .iter()
            .filter(|s| !matches!(s.status, super::mcp::ServerStatus::Disabled))
            .count();
        let total = servers.len();
        println!(
            "│  {}/{} active{:width$}│",
            active,
            total,
            "",
            width = WIDTH - 12
        );
    }

    println!("│{:width$}│", "", width = WIDTH);
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
}

fn render_daemon(daemon: &DaemonStatus) {
    println!(
        "┌─ {} ──────────────────────────────────────────────────────────────────┐",
        ds::primary("📡 DAEMON")
    );
    println!("│{:width$}│", "", width = WIDTH);

    if daemon.running {
        let pid_str = daemon
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        let line = format!(
            "  spn daemon {} {}   PID {}   {}   Uptime {}",
            ds::success("✅"),
            ds::success("running"),
            pid_str,
            daemon.socket_path.display(),
            daemon.uptime_display()
        );
        println!("│{:<width$}│", line, width = WIDTH);
    } else {
        let line = format!(
            "  spn daemon {} {}   Start with: spn daemon start",
            ds::warning("❌"),
            ds::muted("not running")
        );
        println!("│{:<width$}│", line, width = WIDTH);
    }

    println!("│{:width$}│", "", width = WIDTH);
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
}

fn render_footer(summary: &StatusSummary) {
    let daemon_status = if summary.daemon_running {
        ds::success("Daemon OK")
    } else {
        ds::warning("Daemon OFF")
    };

    println!(
        "  🔑 {}/{} Keys    🔌 {}/{} MCPs    🦙 {} Models    📡 {}",
        summary.credentials_configured,
        summary.credentials_total,
        summary.mcp_active,
        summary.mcp_total,
        summary.models_count,
        daemon_status
    );
}

/// Create a progress bar.
fn progress_bar(percent: usize, width: usize) -> String {
    let filled = (percent * width) / 100;
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0, 10), "░░░░░░░░░░");
        assert_eq!(progress_bar(50, 10), "█████░░░░░");
        assert_eq!(progress_bar(100, 10), "██████████");
    }
}
