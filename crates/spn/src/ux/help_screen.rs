//! Full-featured help screen for spn CLI.
//!
//! Displays a comprehensive overview of spn as "The Agentic AI Toolkit"
//! with ASCII art logo, architecture diagram, all commands, and capabilities.

use console::style;

/// Print the full help screen with ASCII art and all features.
pub fn print() {
    print_header();
    print_intro();
    print_how_it_works();
    print_get_started();
    print_commands();
    print_capabilities();
    print_footer();
}

// ============================================================================
// HEADER - ASCII Art Logo
// ============================================================================

fn print_header() {
    let version = env!("CARGO_PKG_VERSION");

    // Box top
    println!(
        "{}",
        style("┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓").cyan()
    );
    println!(
        "{}",
        style("┃                                                                                ┃").cyan()
    );

    // ASCII Art - SPN-CLI
    let logo_lines = [
        "    ███████╗██████╗ ███╗   ██╗       ██████╗██╗     ██╗",
        "    ██╔════╝██╔══██╗████╗  ██║      ██╔════╝██║     ██║",
        "    ███████╗██████╔╝██╔██╗ ██║█████╗██║     ██║     ██║",
        "    ╚════██║██╔═══╝ ██║╚██╗██║╚════╝██║     ██║     ██║",
        "    ███████║██║     ██║ ╚████║      ╚██████╗███████╗██║",
        "    ╚══════╝╚═╝     ╚═╝  ╚═══╝       ╚═════╝╚══════╝╚═╝",
    ];

    for line in logo_lines {
        println!(
            "{}  {}  {}",
            style("┃").cyan(),
            style(line).cyan().bold(),
            style("┃").cyan()
        );
    }

    // Version line
    println!(
        "{}",
        style("┃                                                                                ┃").cyan()
    );
    println!(
        "{}{}{}{}",
        style("┃").cyan(),
        style(format!("                              v{}                              ", version)).dim(),
        style("                ").dim(),
        style("┃").cyan()
    );

    // Tagline
    println!(
        "{}",
        style("┃                                                                                ┃").cyan()
    );
    println!(
        "{}       {}{}",
        style("┃").cyan(),
        style("T h e   A g e n t i c   A I   T o o l k i t").white().bold(),
        style("                        ┃").cyan()
    );
    println!(
        "{}",
        style("┃                                                                                ┃").cyan()
    );

    // Box bottom
    println!(
        "{}",
        style("┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛").cyan()
    );
    println!();
}

// ============================================================================
// INTRO
// ============================================================================

fn print_intro() {
    println!(
        "  {}",
        style("Unified CLI for AI agent development. Manage secrets, models, MCP servers,").dim()
    );
    println!(
        "  {}",
        style("workflows, and packages across your entire AI stack.").dim()
    );
    println!();
}

// ============================================================================
// HOW IT WORKS - Architecture diagram
// ============================================================================

fn print_how_it_works() {
    println!(
        "{}",
        style("┌─ HOW IT WORKS ───────────────────────────────────────────────────────────────┐").cyan()
    );
    println!(
        "{}",
        style("│                                                                              │").cyan()
    );

    // Architecture diagram
    println!(
        "{}   {}      {}      {}      {}        {}",
        style("│").cyan(),
        style("┌──────────┐").white(),
        style("┌──────────┐").white(),
        style("┌──────────┐").white(),
        style("┌──────────┐").white(),
        style("│").cyan()
    );
    println!(
        "{}   {} {} {} {} {} {} {}        {}",
        style("│").cyan(),
        style("│").white(),
        style(" Secrets ").green().bold(),
        style("│ ───▶ │").white(),
        style("  Daemon ").yellow().bold(),
        style("│ ───▶ │").white(),
        style("   MCP   ").blue().bold(),
        style("│ ───▶ │   LLM    │").white(),
        style("│").cyan()
    );
    println!(
        "{}   {} {} {} {} {} {} {}        {}",
        style("│").cyan(),
        style("│").white(),
        style(" Keychain").dim(),
        style("│      │").white(),
        style("(no popup)").dim(),
        style("│      │").white(),
        style(" Servers ").dim(),
        style("│      │  Agents  │").white(),
        style("│").cyan()
    );
    println!(
        "{}   {}      {}      {}      {}        {}",
        style("│").cyan(),
        style("└──────────┘").white(),
        style("└──────────┘").white(),
        style("└──────────┘").white(),
        style("└──────────┘").white(),
        style("│").cyan()
    );

    println!(
        "{}                           {}                                                  {}",
        style("│").cyan(),
        style("│").white(),
        style("│").cyan()
    );
    println!(
        "{}                           {}                                                  {}",
        style("│").cyan(),
        style("▼").white(),
        style("│").cyan()
    );

    // Nika + NovaNet
    println!(
        "{}                     {}      {}                          {}",
        style("│").cyan(),
        style("┌──────────┐").white(),
        style("┌──────────┐").white(),
        style("│").cyan()
    );
    println!(
        "{}                     {} {} {} {} {}                          {}",
        style("│").cyan(),
        style("│").white(),
        style("  Nika   ").magenta().bold(),
        style("│ ◀──▶ │").white(),
        style(" NovaNet ").cyan().bold(),
        style("│").white(),
        style("│").cyan()
    );
    println!(
        "{}                     {} {} {} {} {}                          {}",
        style("│").cyan(),
        style("│").white(),
        style("Workflows").dim(),
        style("│      │").white(),
        style("Knowledge").dim(),
        style("│").white(),
        style("│").cyan()
    );
    println!(
        "{}                     {}      {}                          {}",
        style("│").cyan(),
        style("└──────────┘").white(),
        style("└──────────┘").white(),
        style("│").cyan()
    );

    println!(
        "{}                                                                              {}",
        style("│").cyan(),
        style("│").cyan()
    );
    println!(
        "{}   {}   {}",
        style("│").cyan(),
        style("Store keys once → Daemon serves securely → Nika/MCP consume → LLMs run").dim(),
        style("│").cyan()
    );
    println!(
        "{}                                                                              {}",
        style("│").cyan(),
        style("│").cyan()
    );
    println!(
        "{}",
        style("└──────────────────────────────────────────────────────────────────────────────┘").cyan()
    );
    println!();
}

// ============================================================================
// GET STARTED
// ============================================================================

fn print_get_started() {
    println!(
        "{}",
        style("┌─ 🚀 GET STARTED ─────────────────────────────────────────────────────────────┐").green()
    );
    println!(
        "{}                                                                              {}",
        style("│").green(),
        style("│").green()
    );

    let commands = [
        ("setup", "Interactive wizard (providers + MCP + editors)"),
        ("setup nika", "Install Nika workflow engine"),
        ("setup novanet", "Install NovaNet knowledge graph"),
        ("doctor", "Verify installation health"),
        ("tour", "Guided feature walkthrough"),
    ];

    for (cmd, desc) in commands {
        println!(
            "{}   {}  {}{}",
            style("│").green(),
            style(format!("{:<17}", cmd)).cyan().bold(),
            style(desc).dim(),
            style(" ".repeat(58 - desc.len())).dim(),
        );
    }

    println!(
        "{}                                                                              {}",
        style("│").green(),
        style("│").green()
    );
    println!(
        "{}",
        style("└──────────────────────────────────────────────────────────────────────────────┘").green()
    );
    println!();
}

// ============================================================================
// COMMANDS - All command sections
// ============================================================================

fn print_commands() {
    // Row 1: SECRETS + MODELS
    print_two_columns(
        ("🔐 SECRETS", "yellow", &[
            ("provider list", "Status"),
            ("provider set", "Store"),
            ("provider get", "Fetch"),
            ("provider delete", "Remove"),
            ("provider migrate", "Import"),
            ("provider test", "Verify"),
            ("provider status", "Full"),
            ("", ""),
            ("secrets doctor", "Health"),
            ("secrets export", "Backup"),
            ("secrets import", "Restore"),
        ]),
        ("🦙 MODELS", "magenta", &[
            ("model list", "Show installed"),
            ("model pull", "Download from Ollama"),
            ("model load", "Load into VRAM"),
            ("model unload", "Release memory"),
            ("model delete", "Remove local"),
            ("model status", "Running + VRAM"),
            ("model search", "Browse registry"),
            ("model info", "Details + params"),
            ("model run", "Quick inference"),
            ("model recommend", "Suggest for use case"),
            ("", ""),
        ]),
    );

    // Row 2: MCP + PACKAGES
    print_two_columns(
        ("🔌 MCP SERVERS", "blue", &[
            ("mcp add", "Add server"),
            ("mcp remove", "Remove"),
            ("mcp list", "Show all"),
            ("mcp test", "Connection"),
            ("mcp logs", "View logs"),
            ("mcp serve", "REST→MCP"),
            ("mcp wrap", "API wizard"),
            ("mcp apis", "Manage REST"),
            ("", ""),
            ("", "44 aliases: neo4j github"),
            ("", "slack postgres sqlite..."),
        ]),
        ("📦 PACKAGES", "yellow", &[
            ("add", "Add to project"),
            ("remove", "Remove package"),
            ("install", "From spn.yaml"),
            ("update", "Upgrade to latest"),
            ("list", "Show installed"),
            ("search", "Browse registry"),
            ("info", "Package details"),
            ("outdated", "Check for updates"),
            ("publish", "Publish to registry"),
            ("version", "Bump semver"),
            ("", ""),
        ]),
    );

    // Row 3: JOBS + SKILLS
    print_two_columns(
        ("⚡ JOBS", "cyan", &[
            ("jobs list", "Background"),
            ("jobs submit", "Queue work"),
            ("jobs status", "Check job"),
            ("jobs cancel", "Abort job"),
            ("jobs output", "View stdout"),
            ("jobs clear", "Cleanup old"),
            ("", ""),
            ("", "Max 4 concurrent"),
            ("", "Priority queue"),
            ("", "Nika integration"),
            ("", ""),
        ]),
        ("🎯 SKILLS", "green", &[
            ("skill add", "Install from skills.sh"),
            ("skill remove", "Uninstall"),
            ("skill list", "Show installed"),
            ("skill search", "Browse 57K+ skills"),
            ("", ""),
            ("", "Syncs to:"),
            ("", ".claude/"),
            ("", ".cursor/"),
            ("", ".windsurf/"),
            ("", ""),
            ("", ""),
        ]),
    );

    // Row 4: SYSTEM + ECOSYSTEM
    print_two_columns(
        ("🔧 SYSTEM", "white", &[
            ("status", "Dashboard"),
            ("sync", "Editor sync"),
            ("config show", "Settings"),
            ("config where", "File paths"),
            ("config edit", "Modify"),
            ("config import", "From editor"),
            ("init", "New project"),
            ("explore", "TUI browser"),
            ("suggest", "Smart help"),
            ("", ""),
            ("daemon start|stop|status", ""),
        ]),
        ("🌐 ECOSYSTEM", "cyan", &[
            ("nk <args>", "Proxy to Nika CLI"),
            ("nv <args>", "Proxy to NovaNet CLI"),
            ("", ""),
            ("", "┌────────────────────────┐"),
            ("", "│ Nika    YAML workflows │"),
            ("", "│         5 verbs: infer │"),
            ("", "│         exec fetch     │"),
            ("", "│         invoke agent   │"),
            ("", "├────────────────────────┤"),
            ("", "│ NovaNet Knowledge graph│"),
            ("", "└────────────────────────┘"),
        ]),
    );

    // SYNC & COMPLETION
    println!(
        "{}",
        style("┌─ 🔄 SYNC & COMPLETION ───────────────────────────────────────────────────────┐").white()
    );
    println!(
        "{}                                                                              {}",
        style("│").white(),
        style("│").white()
    );
    println!(
        "{}   {}  {}            {}",
        style("│").white(),
        style("sync --enable <editor>").cyan(),
        style("Enable editor (claude-code cursor windsurf)").dim(),
        style("│").white()
    );
    println!(
        "{}   {}  {}                        {}",
        style("│").white(),
        style("sync --status").cyan(),
        style("Show sync status across editors").dim(),
        style("│").white()
    );
    println!(
        "{}   {}  {}                    {}",
        style("│").white(),
        style("sync --interactive").cyan(),
        style("Diff preview before applying").dim(),
        style("│").white()
    );
    println!(
        "{}                                                                              {}",
        style("│").white(),
        style("│").white()
    );
    println!(
        "{}   {}  {}                              {}",
        style("│").white(),
        style("completion install").cyan(),
        style("Auto-detect shell and install").dim(),
        style("│").white()
    );
    println!(
        "{}   {}  {}                          {}",
        style("│").white(),
        style("completion bash|zsh|fish").cyan(),
        style("Generate for specific shell").dim(),
        style("│").white()
    );
    println!(
        "{}                                                                              {}",
        style("│").white(),
        style("│").white()
    );
    println!(
        "{}",
        style("└──────────────────────────────────────────────────────────────────────────────┘").white()
    );
    println!();
}

/// Print two command sections side by side.
fn print_two_columns(
    left: (&str, &str, &[(&str, &str)]),
    right: (&str, &str, &[(&str, &str)]),
) {
    let (left_title, left_color, left_cmds) = left;
    let (right_title, right_color, right_cmds) = right;

    let left_border = match left_color {
        "yellow" => style("│").yellow(),
        "magenta" => style("│").magenta(),
        "blue" => style("│").blue(),
        "cyan" => style("│").cyan(),
        "green" => style("│").green(),
        _ => style("│").white(),
    };

    let right_border = match right_color {
        "yellow" => style("│").yellow(),
        "magenta" => style("│").magenta(),
        "blue" => style("│").blue(),
        "cyan" => style("│").cyan(),
        "green" => style("│").green(),
        _ => style("│").white(),
    };

    // Headers
    let left_header = match left_color {
        "yellow" => style(format!("┌─ {} ─", left_title)).yellow(),
        "magenta" => style(format!("┌─ {} ─", left_title)).magenta(),
        "blue" => style(format!("┌─ {} ─", left_title)).blue(),
        "cyan" => style(format!("┌─ {} ─", left_title)).cyan(),
        "green" => style(format!("┌─ {} ─", left_title)).green(),
        _ => style(format!("┌─ {} ─", left_title)).white(),
    };

    let right_header = match right_color {
        "yellow" => style(format!("┌─ {} ─", right_title)).yellow(),
        "magenta" => style(format!("┌─ {} ─", right_title)).magenta(),
        "blue" => style(format!("┌─ {} ─", right_title)).blue(),
        "cyan" => style(format!("┌─ {} ─", right_title)).cyan(),
        "green" => style(format!("┌─ {} ─", right_title)).green(),
        _ => style(format!("┌─ {} ─", right_title)).white(),
    };

    // Calculate padding for headers
    let left_title_len = left_title.len() + 4; // "┌─ " + " ─"
    let right_title_len = right_title.len() + 4;
    let left_padding = 36 - left_title_len;
    let right_padding = 40 - right_title_len;

    println!(
        "{}{}┐  {}{}┐",
        left_header,
        "─".repeat(left_padding),
        right_header,
        "─".repeat(right_padding),
    );

    // Empty line
    println!(
        "{}                                    {}  {}                                        {}",
        left_border, left_border, right_border, right_border
    );

    // Commands
    let max_rows = left_cmds.len().max(right_cmds.len());
    for i in 0..max_rows {
        let (left_cmd, left_desc) = left_cmds.get(i).unwrap_or(&("", ""));
        let (right_cmd, right_desc) = right_cmds.get(i).unwrap_or(&("", ""));

        // Left column
        let left_formatted = if left_cmd.is_empty() && left_desc.is_empty() {
            format!("{:34}", "")
        } else if left_cmd.is_empty() {
            // Just a description (like "44 aliases...")
            format!("  {:<32}", style(left_desc).dim())
        } else {
            format!(
                "  {} {}",
                style(format!("{:<17}", left_cmd)).cyan(),
                style(format!("{:<13}", left_desc)).dim()
            )
        };

        // Right column
        let right_formatted = if right_cmd.is_empty() && right_desc.is_empty() {
            format!("{:38}", "")
        } else if right_cmd.is_empty() {
            format!("  {:<36}", style(right_desc).dim())
        } else {
            format!(
                "  {} {}",
                style(format!("{:<17}", right_cmd)).cyan(),
                style(format!("{:<17}", right_desc)).dim()
            )
        };

        println!(
            "{} {} {}  {} {} {}",
            left_border, left_formatted, left_border,
            right_border, right_formatted, right_border
        );
    }

    // Empty line
    println!(
        "{}                                    {}  {}                                        {}",
        left_border, left_border, right_border, right_border
    );

    // Bottom borders
    let left_bottom = match left_color {
        "yellow" => style("└──────────────────────────────────────┘").yellow(),
        "magenta" => style("└──────────────────────────────────────┘").magenta(),
        "blue" => style("└──────────────────────────────────────┘").blue(),
        "cyan" => style("└──────────────────────────────────────┘").cyan(),
        "green" => style("└──────────────────────────────────────┘").green(),
        _ => style("└──────────────────────────────────────┘").white(),
    };

    let right_bottom = match right_color {
        "yellow" => style("└────────────────────────────────────────┘").yellow(),
        "magenta" => style("└────────────────────────────────────────┘").magenta(),
        "blue" => style("└────────────────────────────────────────┘").blue(),
        "cyan" => style("└────────────────────────────────────────┘").cyan(),
        "green" => style("└────────────────────────────────────────┘").green(),
        _ => style("└────────────────────────────────────────┘").white(),
    };

    println!("{}  {}", left_bottom, right_bottom);
    println!();
}

// ============================================================================
// CAPABILITIES
// ============================================================================

fn print_capabilities() {
    println!(
        "{}",
        style("┌─ CAPABILITIES ───────────────────────────────────────────────────────────────┐").white().bold()
    );
    println!(
        "{}                                                                              {}",
        style("│").white(),
        style("│").white()
    );

    let capabilities = [
        (
            "🔐",
            "13 Providers",
            "7 LLM (Anthropic OpenAI Mistral Groq DeepSeek Gemini",
        ),
        (
            "",
            "",
            "Ollama) + 6 MCP (Neo4j GitHub Slack Perplexity",
        ),
        (
            "",
            "",
            "Firecrawl Supadata)",
        ),
        (
            "🔌",
            "44 MCP Aliases",
            "Pre-configured npm packages (neo4j github postgres",
        ),
        (
            "",
            "",
            "sqlite memory puppeteer brave-search fetch...)",
        ),
        (
            "🦙",
            "Ollama Backend",
            "Local models with VRAM tracking and GPU allocation",
        ),
        (
            "🔒",
            "OS Keychain",
            "macOS/Windows/Linux + Daemon (zero popup fatigue)",
        ),
        (
            "📝",
            "4 Editors",
            "Claude Code, Cursor, Windsurf, VS Code sync",
        ),
        (
            "⚡",
            "Job Queue",
            "Background Nika workflows (4 concurrent, priority)",
        ),
        (
            "🎯",
            "57K+ Skills",
            "From skills.sh ecosystem",
        ),
    ];

    for (icon, name, desc) in capabilities {
        if icon.is_empty() {
            // Continuation line
            println!(
                "{}                  {}{}",
                style("│").white(),
                style(desc).dim(),
                style(" ".repeat(56 - desc.len())).dim(),
            );
        } else {
            println!(
                "{}  {} {}  {}{}",
                style("│").white(),
                icon,
                style(format!("{:<14}", name)).cyan().bold(),
                style(desc).dim(),
                style(" ".repeat(44 - desc.len().min(44))).dim(),
            );
        }
    }

    println!(
        "{}                                                                              {}",
        style("│").white(),
        style("│").white()
    );
    println!(
        "{}",
        style("└──────────────────────────────────────────────────────────────────────────────┘").white().bold()
    );
    println!();
}

// ============================================================================
// FOOTER
// ============================================================================

fn print_footer() {
    println!(
        "  {}       {}",
        style("spn <command> --help").cyan(),
        style("Detailed command help").dim()
    );
    println!(
        "  {}                 {}",
        style("spn status").cyan(),
        style("Live system dashboard").dim()
    );
    println!(
        "  {}                   {}",
        style("spn tour").cyan(),
        style("Interactive feature guide").dim()
    );
    println!();
    println!(
        "  {}",
        style("Docs: https://github.com/supernovae-st/supernovae-cli").dim().underlined()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_does_not_panic() {
        // Just verify the print function doesn't panic
        // Output goes to stdout which we don't capture in this test
        print();
    }
}
