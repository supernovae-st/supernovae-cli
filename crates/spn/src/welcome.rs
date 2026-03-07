//! Welcome screen for first-time users (v0.14.0).
//!
//! Provides an engaging, helpful introduction to spn
//! with guided options for getting started.

use crate::ux::design_system as ds;
use dialoguer::{theme::ColorfulTheme, Select};

/// Actions the user can take from the welcome screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WelcomeAction {
    /// Run the interactive setup wizard
    QuickSetup,
    /// Take a tour of features
    TakeTour,
    /// Show help and skip welcome
    ShowHelp,
    /// Skip welcome, don't show again
    SkipForever,
}

/// Display the welcome screen and get user's choice
pub fn show() -> Result<WelcomeAction, dialoguer::Error> {
    println!();
    println!(
        "{}",
        ds::primary("╭─────────────────────────────────────────────────────────────╮")
    );
    println!(
        "{}",
        ds::primary("│                                                             │")
    );
    println!(
        "{}  {} {}                {}",
        ds::primary("│"),
        ds::highlight("🚀"),
        ds::primary("Welcome to spn"),
        ds::primary("│")
    );
    println!(
        "{}     {}      {}",
        ds::primary("│"),
        ds::muted("SuperNovae Package Manager"),
        ds::primary("│")
    );
    println!(
        "{}                                                             {}",
        ds::primary("│"),
        ds::primary("│")
    );
    println!(
        "{}{}{}",
        ds::primary("├"),
        ds::primary("─────────────────────────────────────────────────────────────"),
        ds::primary("┤")
    );
    println!(
        "{}                                                             {}",
        ds::primary("│"),
        ds::primary("│")
    );
    println!(
        "{}   Your AI development toolkit for:                         {}",
        ds::primary("│"),
        ds::primary("│")
    );
    println!(
        "{}                                                             {}",
        ds::primary("│"),
        ds::primary("│")
    );
    println!(
        "{}   {} Managing AI workflows and schemas                  {}",
        ds::primary("│"),
        ds::highlight("📦"),
        ds::primary("│")
    );
    println!(
        "{}   {} Securing API keys for LLM providers                {}",
        ds::primary("│"),
        ds::highlight("🔐"),
        ds::primary("│")
    );
    println!(
        "{}   {} Syncing tools to your favorite editor              {}",
        ds::primary("│"),
        ds::highlight("🔄"),
        ds::primary("│")
    );
    println!(
        "{}                                                             {}",
        ds::primary("│"),
        ds::primary("│")
    );
    println!(
        "{}",
        ds::primary("╰─────────────────────────────────────────────────────────────╯")
    );
    println!();

    let choices = vec![
        format!(
            "{} {} {}",
            ds::highlight("🎯"),
            ds::highlight("Quick Setup"),
            ds::muted("(5 min) - Configure providers, add essential tools")
        ),
        format!(
            "{} {} {}",
            ds::highlight("📖"),
            ds::highlight("Take a Tour"),
            ds::muted("- Learn what spn can do")
        ),
        format!(
            "{} {} {}",
            ds::highlight("❔"),
            ds::highlight("Show Help"),
            ds::muted("- See all commands")
        ),
        format!(
            "{} {} {}",
            ds::highlight("⏭️"),
            ds::highlight("Skip"),
            ds::muted("- I'll explore on my own")
        ),
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do?")
        .items(&choices)
        .default(0)
        .interact()?;

    Ok(match selection {
        0 => WelcomeAction::QuickSetup,
        1 => WelcomeAction::TakeTour,
        2 => WelcomeAction::ShowHelp,
        _ => WelcomeAction::SkipForever,
    })
}

/// Display the feature tour
pub fn show_tour() {
    println!();
    println!("{}", ds::primary("🧭 spn Feature Tour"));
    println!();

    let features = [
        (
            "Package Management",
            "Install AI workflows, schemas, and tools",
            "spn add @nika/generate-page",
        ),
        (
            "Secure Secrets",
            "Store API keys in your OS keychain",
            "spn provider set anthropic",
        ),
        (
            "MCP Servers",
            "Add Model Context Protocol servers",
            "spn mcp add neo4j",
        ),
        (
            "Local Models",
            "Manage LLMs via Ollama",
            "spn model pull llama3.2",
        ),
        (
            "Editor Sync",
            "Sync packages to Claude Code, VS Code",
            "spn sync",
        ),
        ("Diagnostics", "Check your setup is working", "spn doctor"),
    ];

    for (i, (name, desc, cmd)) in features.iter().enumerate() {
        println!(
            "  {} {}",
            ds::primary(format!("{}.", i + 1)),
            ds::highlight(*name)
        );
        println!("     {}", ds::muted(*desc));
        println!("     {} {}", ds::muted("$"), ds::command(*cmd));
        println!();
    }

    println!(
        "{}",
        ds::muted("Run `spn topic` to explore any of these in detail.")
    );
    println!();
}

/// Display post-setup success message
pub fn show_setup_complete() {
    println!();
    println!("{} {}", ds::highlight("✨"), ds::success("Setup complete!"));
    println!();
    println!("{}", ds::highlight("What's next?"));
    println!();
    println!(
        "  {} {}  {}",
        ds::muted("$"),
        ds::command("spn doctor"),
        ds::muted("Verify your setup")
    );
    println!(
        "  {} {}  {}",
        ds::muted("$"),
        ds::command("spn mcp list"),
        ds::muted("See installed MCP servers")
    );
    println!(
        "  {} {}  {}",
        ds::muted("$"),
        ds::command("spn sync"),
        ds::muted("Sync to your editor")
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_action_variants() {
        assert_ne!(WelcomeAction::QuickSetup, WelcomeAction::TakeTour);
        assert_ne!(WelcomeAction::ShowHelp, WelcomeAction::SkipForever);
    }
}
