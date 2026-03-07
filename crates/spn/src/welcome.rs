//! Welcome screen for first-time users (v0.14.0).
//!
//! Provides an engaging, helpful introduction to spn
//! with guided options for getting started.

use console::style;
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
        style("╭─────────────────────────────────────────────────────────────╮").cyan()
    );
    println!(
        "{}",
        style("│                                                             │").cyan()
    );
    println!(
        "{}  {} {}                {}",
        style("│").cyan(),
        style("🚀").bold(),
        style("Welcome to spn").cyan().bold(),
        style("│").cyan()
    );
    println!(
        "{}     {}      {}",
        style("│").cyan(),
        style("SuperNovae Package Manager").dim(),
        style("│").cyan()
    );
    println!(
        "{}                                                             {}",
        style("│").cyan(),
        style("│").cyan()
    );
    println!(
        "{}{}{}",
        style("├").cyan(),
        style("─────────────────────────────────────────────────────────────").cyan(),
        style("┤").cyan()
    );
    println!(
        "{}                                                             {}",
        style("│").cyan(),
        style("│").cyan()
    );
    println!(
        "{}   Your AI development toolkit for:                         {}",
        style("│").cyan(),
        style("│").cyan()
    );
    println!(
        "{}                                                             {}",
        style("│").cyan(),
        style("│").cyan()
    );
    println!(
        "{}   {} Managing AI workflows and schemas                  {}",
        style("│").cyan(),
        style("📦").bold(),
        style("│").cyan()
    );
    println!(
        "{}   {} Securing API keys for LLM providers                {}",
        style("│").cyan(),
        style("🔐").bold(),
        style("│").cyan()
    );
    println!(
        "{}   {} Syncing tools to your favorite editor              {}",
        style("│").cyan(),
        style("🔄").bold(),
        style("│").cyan()
    );
    println!(
        "{}                                                             {}",
        style("│").cyan(),
        style("│").cyan()
    );
    println!(
        "{}",
        style("╰─────────────────────────────────────────────────────────────╯").cyan()
    );
    println!();

    let choices = vec![
        format!(
            "{} {} {}",
            style("🎯").bold(),
            style("Quick Setup").bold(),
            style("(5 min) - Configure providers, add essential tools").dim()
        ),
        format!(
            "{} {} {}",
            style("📖").bold(),
            style("Take a Tour").bold(),
            style("- Learn what spn can do").dim()
        ),
        format!(
            "{} {} {}",
            style("❔").bold(),
            style("Show Help").bold(),
            style("- See all commands").dim()
        ),
        format!(
            "{} {} {}",
            style("⏭️").bold(),
            style("Skip").bold(),
            style("- I'll explore on my own").dim()
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
    println!("{}", style("🧭 spn Feature Tour").cyan().bold());
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
        (
            "Diagnostics",
            "Check your setup is working",
            "spn doctor",
        ),
    ];

    for (i, (name, desc, cmd)) in features.iter().enumerate() {
        println!(
            "  {} {}",
            style(format!("{}.", i + 1)).cyan().bold(),
            style(*name).bold()
        );
        println!("     {}", style(*desc).dim());
        println!(
            "     {} {}",
            style("$").dim(),
            style(*cmd).cyan()
        );
        println!();
    }

    println!(
        "{}",
        style("Run `spn topic` to explore any of these in detail.").dim()
    );
    println!();
}

/// Display post-setup success message
pub fn show_setup_complete() {
    println!();
    println!(
        "{} {}",
        style("✨").bold(),
        style("Setup complete!").green().bold()
    );
    println!();
    println!("{}", style("What's next?").bold());
    println!();
    println!(
        "  {} {}  {}",
        style("$").dim(),
        style("spn doctor").cyan(),
        style("Verify your setup").dim()
    );
    println!(
        "  {} {}  {}",
        style("$").dim(),
        style("spn mcp list").cyan(),
        style("See installed MCP servers").dim()
    );
    println!(
        "  {} {}  {}",
        style("$").dim(),
        style("spn sync").cyan(),
        style("Sync to your editor").dim()
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
