//! UX Design System for spn CLI v0.14.0+
//!
//! A semantic color and styling system based on best practices from:
//! - Lucas F. Costa's "UX Patterns for CLI Tools"
//! - Spectre.Console's semantic markup system
//! - Katie Cooper's "Semantic Color Sets for Design Systems"
//!
//! # Design Principles
//!
//! 1. **Semantic over Primitive**: Colors are named by meaning, not by hue
//! 2. **Consistent Vocabulary**: Same semantic color across all commands
//! 3. **Accessibility**: High contrast, works on light and dark terminals
//! 4. **Agent-Friendly**: Structured output for AI/automation consumers
//!
//! # Color Taxonomy
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  SEMANTIC COLOR TAXONOMY                                                │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  STATUS (feedback colors)                                               │
//! │  ├── success    Green      ✓ Operation completed                       │
//! │  ├── error      Red        ✗ Operation failed                          │
//! │  ├── warning    Yellow     ⚠ Potential issue                           │
//! │  └── info       Blue       ℹ Informational                             │
//! │                                                                         │
//! │  INTERACTIVE (user action colors)                                       │
//! │  ├── primary    Cyan       Commands, URLs, actionable items             │
//! │  ├── secondary  Dim        Hints, suggestions, examples                 │
//! │  └── highlight  Bold       Emphasis, important values                   │
//! │                                                                         │
//! │  HIERARCHY (content structure)                                          │
//! │  ├── heading    Bold       Section headers                              │
//! │  ├── label      Normal     Field labels                                 │
//! │  ├── value      Cyan       Field values                                 │
//! │  └── muted      Dim        Metadata, timestamps, paths                  │
//! │                                                                         │
//! │  SEMANTIC ELEMENTS                                                      │
//! │  ├── command    Cyan       CLI commands (spn add)                       │
//! │  ├── package    Yellow     Package names (@scope/name)                  │
//! │  ├── version    Green      Version numbers (v1.2.3)                     │
//! │  ├── path       Dim        File paths                                   │
//! │  └── url        Cyan+Underline  URLs                                    │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Icons (Unicode)
//!
//! Consistent iconography across all output:
//!
//! | Icon | Meaning | Usage |
//! |------|---------|-------|
//! | ✓ | Success | Operation completed |
//! | ✗ | Error | Operation failed |
//! | ⚠ | Warning | Potential issue |
//! | → | Action | Suggestion, next step |
//! | • | Bullet | List items |
//! | ◆ | Item | Package/resource |
//! | ↓ | Download | Fetching data |
//! | ↑ | Upload | Sending data |
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::ux::design_system::*;
//!
//! // Status messages
//! println!("{}", success("Package installed"));
//! println!("{}", error("Network error"));
//! println!("{}", warning("Outdated version"));
//! println!("{}", info("Checking dependencies..."));
//!
//! // Semantic elements
//! println!("Run {} to continue", command("spn sync"));
//! println!("Installing {}", package("@nika/workflow"));
//! println!("Version {}", version("1.2.3"));
//!
//! // Structured output
//! println!("{} {}", icon::SUCCESS, success("Done"));
//! println!("{} {}", icon::ARROW, hint("Try running spn doctor"));
//! ```

use console::{style, StyledObject};

// ============================================================================
// ICONS - Consistent Unicode iconography
// ============================================================================

/// Standard icons for CLI output
pub mod icon {
    /// Success indicator (green checkmark)
    pub const SUCCESS: &str = "✓";
    /// Error indicator (red cross)
    pub const ERROR: &str = "✗";
    /// Warning indicator (yellow triangle)
    pub const WARNING: &str = "⚠";
    /// Info indicator
    pub const INFO: &str = "ℹ";
    /// Action/suggestion arrow
    pub const ARROW: &str = "→";
    /// Bullet point
    pub const BULLET: &str = "•";
    /// Item marker (diamond)
    pub const ITEM: &str = "◆";
    /// Download indicator
    pub const DOWNLOAD: &str = "↓";
    /// Upload indicator
    pub const UPLOAD: &str = "↑";
    /// Spinner frames for progress
    pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    /// Package icon
    pub const PACKAGE: &str = "📦";
    /// Lock icon (security)
    pub const LOCK: &str = "🔐";
    /// Sync icon
    pub const SYNC: &str = "🔄";
}

// ============================================================================
// STATUS COLORS - Feedback semantics
// ============================================================================

/// Success message (green)
pub fn success<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).green()
}

/// Error message (red)
pub fn error<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).red()
}

/// Warning message (yellow)
pub fn warning<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).yellow()
}

/// Info message (blue)
pub fn info<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).blue()
}

// ============================================================================
// INTERACTIVE COLORS - User action semantics
// ============================================================================

/// Primary action color (cyan) - commands, URLs, clickable items
pub fn primary<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).cyan()
}

/// Secondary/hint color (dim) - suggestions, examples
pub fn secondary<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).dim()
}

/// Highlighted/emphasized text (bold)
pub fn highlight<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).bold()
}

/// Muted text (dim) - metadata, paths, timestamps
pub fn muted<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).dim()
}

// ============================================================================
// SEMANTIC ELEMENTS - Domain-specific styling
// ============================================================================

/// CLI command styling (cyan)
pub fn command<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).cyan()
}

/// Package name styling (yellow bold)
pub fn package<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).yellow().bold()
}

/// Version number styling (green)
pub fn version<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).green()
}

/// File path styling (dim)
pub fn path<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).dim()
}

/// URL styling (cyan underline)
pub fn url<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).cyan().underlined()
}

/// Provider name styling (magenta)
pub fn provider<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).magenta()
}

/// Key/label styling (bold)
pub fn label<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).bold()
}

/// Value styling (cyan)
pub fn value<D: std::fmt::Display>(text: D) -> StyledObject<String> {
    style(text.to_string()).cyan()
}

// ============================================================================
// COMPOSITE HELPERS - Common patterns
// ============================================================================

/// Success line with icon
pub fn success_line<D: std::fmt::Display>(text: D) -> String {
    format!("  {} {}", style(icon::SUCCESS).green().bold(), success(text))
}

/// Error line with icon
pub fn error_line<D: std::fmt::Display>(text: D) -> String {
    format!("  {} {}", style(icon::ERROR).red().bold(), error(text))
}

/// Warning line with icon
pub fn warning_line<D: std::fmt::Display>(text: D) -> String {
    format!(
        "  {} {}",
        style(icon::WARNING).yellow().bold(),
        warning(text)
    )
}

/// Info line with icon
pub fn info_line<D: std::fmt::Display>(text: D) -> String {
    format!("  {} {}", style(icon::INFO).blue().bold(), info(text))
}

/// Hint/suggestion line with arrow
pub fn hint_line<D: std::fmt::Display>(text: D) -> String {
    format!("  {} {}", style(icon::ARROW).cyan(), secondary(text))
}

/// Bullet point item
pub fn bullet<D: std::fmt::Display>(text: D) -> String {
    format!("  {} {}", style(icon::BULLET).dim(), text)
}

/// Just the bullet icon (for manual formatting)
pub fn bullet_icon() -> StyledObject<&'static str> {
    style(icon::BULLET).dim()
}

// ============================================================================
// FORMATTING HELPERS
// ============================================================================

/// Format a key-value pair
pub fn key_value<K: std::fmt::Display, V: std::fmt::Display>(key: K, val: V) -> String {
    format!("{}: {}", label(key), value(val))
}

/// Format a labeled section header
pub fn section<D: std::fmt::Display>(title: D) -> String {
    format!("\n  {}\n", highlight(title))
}

/// Indent text with standard 2-space prefix
pub fn indent<D: std::fmt::Display>(text: D) -> String {
    format!("  {}", text)
}

/// Double indent (4 spaces)
pub fn indent2<D: std::fmt::Display>(text: D) -> String {
    format!("    {}", text)
}

// ============================================================================
// PROVIDER HELPERS (Domain-specific)
// ============================================================================

/// Provider with security badge (keychain).
pub fn provider_secure<D: std::fmt::Display>(name: D) -> String {
    format!(
        "{} {} {}",
        style(icon::LOCK).green(),
        provider(&name),
        style("(keychain)").dim()
    )
}

/// Provider with warning badge (env var).
pub fn provider_env<D: std::fmt::Display>(name: D) -> String {
    format!(
        "{} {} {}",
        style(icon::WARNING).yellow(),
        provider(&name),
        style("(env var)").dim()
    )
}

/// Provider with insecure badge (.env file).
pub fn provider_insecure<D: std::fmt::Display>(name: D) -> String {
    format!(
        "{} {} {}",
        style(icon::WARNING).yellow(),
        provider(&name),
        style("(.env)").yellow()
    )
}

/// Provider missing (not configured).
pub fn provider_missing<D: std::fmt::Display>(name: D) -> String {
    format!(
        "{} {} {}",
        style(icon::ERROR).red(),
        style(name.to_string()).dim(),
        style("(missing)").red()
    )
}

// ============================================================================
// STEP INDICATORS
// ============================================================================

/// Step indicator with count [1/5].
pub fn step_indicator(current: usize, total: usize) -> String {
    format!(
        "[{}/{}]",
        style(current).cyan().bold(),
        style(total).dim()
    )
}

/// Step with icon (completed).
pub fn step_done<D: std::fmt::Display>(step: usize, total: usize, message: D) -> String {
    format!(
        "{} {} {}",
        step_indicator(step, total),
        style(icon::SUCCESS).green().bold(),
        success(message)
    )
}

/// Step with icon (in progress).
pub fn step_active<D: std::fmt::Display>(step: usize, total: usize, message: D) -> String {
    format!(
        "{} {} {}",
        step_indicator(step, total),
        style("▸").cyan().bold(),
        highlight(message)
    )
}

/// Step with icon (pending).
pub fn step_pending<D: std::fmt::Display>(step: usize, total: usize, message: D) -> String {
    format!(
        "{} {} {}",
        style(format!("[{}/{}]", step, total)).dim(),
        style("○").dim(),
        muted(message)
    )
}

// ============================================================================
// TREE HELPERS
// ============================================================================

/// Tree branch for non-last items.
pub fn tree_branch() -> &'static str {
    "├── "
}

/// Tree branch for last item.
pub fn tree_branch_last() -> &'static str {
    "└── "
}

/// Tree continuation (vertical line).
pub fn tree_continue() -> &'static str {
    "│   "
}

/// Get appropriate branch character.
pub fn branch(is_last: bool) -> &'static str {
    if is_last {
        "└── "
    } else {
        "├── "
    }
}

// ============================================================================
// BOX HELPERS
// ============================================================================

/// Create a boxed header.
pub fn boxed_header<D: std::fmt::Display>(title: D, width: usize) -> String {
    let title_str = title.to_string();
    let inner_width = width.saturating_sub(4);
    let title_padded = format!("{:<width$}", title_str, width = inner_width);

    format!(
        "┌{}┐\n│ {} │\n└{}┘",
        "─".repeat(width - 2),
        highlight(title_padded),
        "─".repeat(width - 2)
    )
}

/// Create a simple horizontal rule.
pub fn hr(width: usize) -> String {
    "─".repeat(width)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_are_defined() {
        assert!(!icon::SUCCESS.is_empty());
        assert!(!icon::ERROR.is_empty());
        assert!(!icon::WARNING.is_empty());
        assert!(!icon::SPINNER.is_empty());
    }

    #[test]
    fn test_status_colors() {
        // Just verify they don't panic
        let _ = success("test");
        let _ = error("test");
        let _ = warning("test");
        let _ = info("test");
    }

    #[test]
    fn test_semantic_elements() {
        let _ = command("spn add");
        let _ = package("@nika/workflow");
        let _ = version("1.2.3");
        let _ = path("/path/to/file");
        let _ = url("https://example.com");
    }

    #[test]
    fn test_composite_helpers() {
        let s = success_line("Done");
        assert!(s.contains(icon::SUCCESS));

        let e = error_line("Failed");
        assert!(e.contains(icon::ERROR));

        let kv = key_value("Name", "value");
        assert!(kv.contains(':'));
    }

    #[test]
    fn test_formatting_helpers() {
        let s = section("Header");
        assert!(s.contains("Header"));

        let i = indent("text");
        assert!(i.starts_with("  "));

        let i2 = indent2("text");
        assert!(i2.starts_with("    "));
    }
}
