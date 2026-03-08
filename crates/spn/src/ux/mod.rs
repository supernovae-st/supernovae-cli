//! UX utilities for consistent, delightful CLI experience.
//!
//! Provides themed spinners, progress bars, messages, and visual elements
//! with a subtle cosmic/supernova aesthetic that's developer-friendly.
//!
//! # Modules
//!
//! - `design_system`: Semantic color system and styling primitives
//! - `progress`: Enhanced progress bars with multi-column support
//! - `tables`: ASCII tables for structured data display
//! - `trees`: Tree visualization for hierarchical data
//! - `themes`: Custom themes for dialoguer prompts
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::ux::{design_system as ds, progress, tables, trees};
//!
//! // Semantic styling
//! println!("{}", ds::success_line("Package installed"));
//! println!("{}", ds::key_value("Version", "1.2.3"));
//!
//! // Progress bars
//! let spinner = progress::transforming_spinner("Installing...");
//! spinner.finish_success("Installed!");
//!
//! // Tables
//! let table = tables::provider_table(&providers);
//! println!("{}", table);
//!
//! // Trees
//! let tree = trees::package_tree("@nika/workflow", "1.2.3", &deps);
//! println!("{}", tree);
//! ```

#![allow(dead_code)]

pub mod design_system;
pub mod help_screen;
pub mod progress;
pub mod tables;
pub mod themes;
pub mod trees;

use console::{style, Emoji, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// Note: For icons, use `ds::icon::SUCCESS` etc. via `use crate::ux::design_system as ds;`

// Note: design_system provides styling functions that return StyledObject
// This module (ux) provides print functions that output to stdout
// Use `use crate::ux::design_system as ds` for direct styling access

// ============================================================================
// EMOJI & SYMBOLS (with fallbacks for terminals without emoji support)
// ============================================================================

pub static STAR: Emoji<'_, '_> = Emoji("✦ ", "* ");
pub static NOVA: Emoji<'_, '_> = Emoji("🌟 ", "* ");
pub static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "> ");
pub static CHECK: Emoji<'_, '_> = Emoji("✔ ", "v ");
pub static CROSS: Emoji<'_, '_> = Emoji("✘ ", "x ");
pub static WARN: Emoji<'_, '_> = Emoji("⚠ ", "! ");
pub static INFO: Emoji<'_, '_> = Emoji("ℹ ", "i ");
pub static SEARCH: Emoji<'_, '_> = Emoji("🔍 ", "> ");
pub static KEY: Emoji<'_, '_> = Emoji("🔑 ", "* ");
pub static PACKAGE: Emoji<'_, '_> = Emoji("📦 ", "# ");
pub static SYNC: Emoji<'_, '_> = Emoji("🔄 ", "~ ");
pub static ANCHOR: Emoji<'_, '_> = Emoji("⚓ ", "@ ");
pub static COMPASS: Emoji<'_, '_> = Emoji("🧭 ", "> ");
pub static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", "~ ");

// ============================================================================
// SPINNERS - Cosmic themed spinners for async operations
// ============================================================================

/// Spinner themes for different operations
pub enum SpinnerTheme {
    /// Default spinner for general operations
    Default,
    /// For network/download operations
    Download,
    /// For search/discovery operations
    Search,
    /// For installation/setup operations
    Install,
    /// For sync operations
    Sync,
}

impl SpinnerTheme {
    fn frames(&self) -> &[&str] {
        match self {
            SpinnerTheme::Default => &["◐", "◓", "◑", "◒"],
            SpinnerTheme::Download => &[
                "    ", "=   ", "==  ", "=== ", " ===", "  ==", "   =", "    ",
            ],
            SpinnerTheme::Search => &["◜", "◠", "◝", "◞", "◡", "◟"],
            SpinnerTheme::Install => &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▱▰▰", "▱▱▰"],
            SpinnerTheme::Sync => &["↻", "↺"],
        }
    }
}

/// Create a themed spinner with a message
pub fn spinner(message: &str) -> ProgressBar {
    spinner_with_theme(message, SpinnerTheme::Default)
}

/// Create a spinner with a specific theme
pub fn spinner_with_theme(message: &str, theme: SpinnerTheme) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    let frames = theme.frames().to_vec();

    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&frames)
            .template(&format!("{{spinner:.cyan}} {}", message))
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a progress bar for downloads
pub fn download_progress(total: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "{}{{bar:30.cyan/blue}} {{bytes}}/{{total_bytes}} ({{eta}})",
                prefix
            ))
            .unwrap()
            .progress_chars("━━╺"),
    );
    pb
}

/// Create a progress bar for multi-step operations
pub fn step_progress(steps: u64) -> ProgressBar {
    let pb = ProgressBar::new(steps);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{prefix:.bold} [{bar:25.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("▰▰▱"),
    );
    pb
}

// ============================================================================
// MESSAGES - Consistent message formatting
// ============================================================================

/// Print a success message
pub fn success(message: &str) {
    println!("{} {}", style(CHECK).green().bold(), style(message).green());
}

/// Print a success message with details
pub fn success_with_detail(message: &str, detail: &str) {
    println!(
        "{} {} {}",
        style(CHECK).green().bold(),
        style(message).green(),
        style(format!("({})", detail)).dim()
    );
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("{} {}", style(CROSS).red().bold(), style(message).red());
}

/// Print an error message with suggestion
pub fn error_with_hint(message: &str, hint: &str) {
    eprintln!("{} {}", style(CROSS).red().bold(), style(message).red());
    eprintln!("  {} {}", style("hint:").dim(), style(hint).dim());
}

/// Print a warning message
pub fn warn(message: &str) {
    println!(
        "{} {}",
        style(WARN).yellow().bold(),
        style(message).yellow()
    );
}

/// Print an info message
pub fn info(message: &str) {
    println!("{} {}", style(INFO).cyan().bold(), style(message));
}

/// Print a dimmed/muted message
pub fn muted(message: &str) {
    println!("  {}", style(message).dim());
}

/// Print a step in a process
pub fn step(number: usize, message: &str) {
    println!(
        "{} {}",
        style(format!("[{}/{}]", number, number)).cyan().bold(),
        message
    );
}

// ============================================================================
// BANNERS & HEADERS
// ============================================================================

/// Print the main spn banner (subtle, not overwhelming)
pub fn banner() {
    println!();
    println!(
        "  {}{}{}",
        style("spn").cyan().bold(),
        style(" · ").dim(),
        style("SuperNovae Package Manager").dim()
    );
    println!();
}

/// Print a section header
pub fn header(title: &str) {
    println!();
    println!("{}", style(title).bold().underlined());
    println!();
}

/// Print a compact header for sub-sections
pub fn subheader(title: &str) {
    println!("{}", style(title).bold());
}

/// Print a boxed message (for important notices)
pub fn boxed(title: &str, lines: &[&str]) {
    let term = Term::stdout();
    let width = term.size().1 as usize;
    let box_width = std::cmp::min(width - 4, 60);

    let border = "─".repeat(box_width);

    println!();
    println!("  ┌{}┐", border);
    println!(
        "  │ {:<width$} │",
        style(title).bold(),
        width = box_width - 2
    );
    println!("  ├{}┤", border);
    for line in lines {
        println!("  │ {:<width$} │", line, width = box_width - 2);
    }
    println!("  └{}┘", border);
    println!();
}

// ============================================================================
// LISTS & TABLES
// ============================================================================

/// Print a bullet point list item
pub fn bullet(text: &str) {
    println!("  {} {}", style("•").cyan(), text);
}

/// Print a numbered list item
pub fn numbered(num: usize, text: &str) {
    println!("  {}. {}", style(num).cyan().bold(), text);
}

/// Print a key-value pair
pub fn kv(key: &str, value: &str) {
    println!("  {}: {}", style(key).bold(), value);
}

/// Print a key-value pair with the value dimmed
pub fn kv_muted(key: &str, value: &str) {
    println!("  {}: {}", style(key).bold(), style(value).dim());
}

// ============================================================================
// INTERACTIVE HELPERS
// ============================================================================

/// Clear the current line (useful after spinners)
pub fn clear_line() {
    let term = Term::stdout();
    let _ = term.clear_line();
}

/// Move cursor up N lines
pub fn move_up(lines: u16) {
    print!("\x1B[{}A", lines);
}

/// Check if running in a terminal (vs piped)
pub fn is_interactive() -> bool {
    Term::stdout().is_term()
}

// ============================================================================
// SUCCESS CELEBRATIONS (subtle)
// ============================================================================

/// Celebrate a successful operation (subtle, not annoying)
pub fn celebrate(message: &str) {
    println!();
    println!(
        "{} {} {}",
        style(SPARKLE).cyan(),
        style(message).green().bold(),
        style(SPARKLE).cyan()
    );
    println!();
}

/// Show a "what's next" section after successful operations
pub fn next_steps(steps: &[(&str, &str)]) {
    println!();
    println!("{}", style("What's next?").bold());
    println!();
    for (cmd, desc) in steps {
        println!(
            "  {} {}  {}",
            style("$").dim(),
            style(*cmd).cyan(),
            style(*desc).dim()
        );
    }
    println!();
}

// ============================================================================
// THEMED CONFIRMATIONS
// ============================================================================

use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};

/// Get a styled theme for dialoguer prompts
pub fn theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

/// Ask for confirmation
pub fn confirm(prompt: &str, default: bool) -> std::result::Result<bool, dialoguer::Error> {
    Confirm::with_theme(&theme())
        .with_prompt(prompt)
        .default(default)
        .interact()
}

/// Ask for text input
pub fn input(prompt: &str) -> std::result::Result<String, dialoguer::Error> {
    Input::with_theme(&theme()).with_prompt(prompt).interact()
}

/// Ask for text input with default value
pub fn input_with_default(
    prompt: &str,
    default: &str,
) -> std::result::Result<String, dialoguer::Error> {
    Input::with_theme(&theme())
        .with_prompt(prompt)
        .default(default.to_string())
        .interact()
}

/// Select from a list of options
pub fn select<T: ToString>(
    prompt: &str,
    items: &[T],
) -> std::result::Result<usize, dialoguer::Error> {
    Select::with_theme(&theme())
        .with_prompt(prompt)
        .items(items)
        .interact()
}

/// Multi-select from a list of options
pub fn multi_select<T: ToString>(
    prompt: &str,
    items: &[T],
) -> std::result::Result<Vec<usize>, dialoguer::Error> {
    MultiSelect::with_theme(&theme())
        .with_prompt(prompt)
        .items(items)
        .interact()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let pb = spinner("Testing...");
        pb.finish_and_clear();
    }

    #[test]
    fn test_is_interactive() {
        // Just verify it doesn't panic
        let _ = is_interactive();
    }
}
