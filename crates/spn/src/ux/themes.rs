//! Custom themes for dialoguer prompts.
//!
//! Provides a cohesive visual style for all interactive prompts:
//! - Confirmation dialogs
//! - Text input
//! - Selection menus
//! - Multi-select
//!
//! Uses the spn design system colors for consistency.

use console::{style, Style};
use dialoguer::theme::Theme;
use std::fmt;

// ============================================================================
// SPN THEME
// ============================================================================

/// Custom theme for spn CLI with cosmic aesthetic.
///
/// Colors:
/// - Cyan for prompts and highlights
/// - Green for success states
/// - Yellow for selections
/// - Dim for hints and defaults
#[derive(Debug, Clone, Default)]
pub struct SpnTheme;

impl Theme for SpnTheme {
    /// Formats a prompt string.
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{} ", style(prompt).bold())
    }

    /// Formats an error message.
    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        write!(f, "{} {}", style("✗").red().bold(), style(err).red())
    }

    /// Formats a confirm prompt.
    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> fmt::Result {
        let hint = match default {
            Some(true) => "[Y/n]",
            Some(false) => "[y/N]",
            None => "[y/n]",
        };
        write!(f, "{} {} ", style(prompt).bold(), style(hint).dim())
    }

    /// Formats a confirm prompt after selection.
    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: Option<bool>,
    ) -> fmt::Result {
        let answer = match selection {
            Some(true) => style("yes").green(),
            Some(false) => style("no").red(),
            None => style("none").dim(),
        };
        write!(f, "{} {}", style(prompt).bold(), answer)
    }

    /// Formats an input prompt.
    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        match default {
            Some(d) => write!(
                f,
                "{} {}: ",
                style(prompt).bold(),
                style(format!("[{}]", d)).dim()
            ),
            None => write!(f, "{}: ", style(prompt).bold()),
        }
    }

    /// Formats an input prompt after submission.
    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        write!(f, "{}: {}", style(prompt).bold(), style(sel).cyan())
    }

    /// Formats a password prompt.
    fn format_password_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}: ", style(prompt).bold())
    }

    /// Formats a password prompt after submission.
    fn format_password_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        write!(f, "{}: {}", style(prompt).bold(), style("********").dim())
    }

    /// Formats a select prompt.
    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}", style(prompt).bold())
    }

    /// Formats a select prompt item.
    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        if active {
            write!(f, "{} {}", style("▸").cyan().bold(), style(text).cyan())
        } else {
            write!(f, "  {}", style(text).dim())
        }
    }

    /// Formats a select prompt after selection.
    fn format_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        write!(f, "{}: {}", style(prompt).bold(), style(sel).cyan())
    }

    /// Formats a multi-select prompt.
    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}", style(prompt).bold())
    }

    /// Formats a multi-select prompt item.
    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let checkbox = if checked {
            style("◉").green()
        } else {
            style("○").dim()
        };

        if active {
            write!(
                f,
                "{} {} {}",
                style("▸").cyan().bold(),
                checkbox,
                style(text).cyan()
            )
        } else {
            write!(f, "  {} {}", checkbox, text)
        }
    }

    /// Formats a multi-select prompt after selection.
    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            style(prompt).bold(),
            style(selections.join(", ")).cyan()
        )
    }

    /// Formats a sort prompt.
    fn format_sort_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}", style(prompt).bold())
    }

    /// Formats a sort prompt item.
    fn format_sort_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        picked: bool,
        active: bool,
    ) -> fmt::Result {
        let prefix = if picked {
            style("↕").yellow()
        } else if active {
            style("▸").cyan()
        } else {
            style(" ").dim()
        };

        if active {
            write!(f, "{} {}", prefix, style(text).cyan())
        } else {
            write!(f, "{} {}", prefix, text)
        }
    }

    /// Formats a sort prompt after selection.
    fn format_sort_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            style(prompt).bold(),
            style(selections.join(", ")).cyan()
        )
    }
}

// ============================================================================
// STYLE HELPERS
// ============================================================================

/// Get the default spn theme.
pub fn spn_theme() -> SpnTheme {
    SpnTheme
}

/// Style definitions for programmatic use.
pub mod styles {
    use super::*;

    /// Prompt text style (bold)
    pub fn prompt() -> Style {
        Style::new().bold()
    }

    /// Active/selected item style (cyan)
    pub fn active() -> Style {
        Style::new().cyan()
    }

    /// Inactive item style (dim)
    pub fn inactive() -> Style {
        Style::new().dim()
    }

    /// Success style (green)
    pub fn success() -> Style {
        Style::new().green()
    }

    /// Error style (red)
    pub fn error() -> Style {
        Style::new().red()
    }

    /// Warning style (yellow)
    pub fn warning() -> Style {
        Style::new().yellow()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = spn_theme();
        let _ = format!("{:?}", theme); // Ensure Debug works
    }

    #[test]
    fn test_format_prompt() {
        let theme = SpnTheme;
        let mut buf = String::new();
        theme.format_prompt(&mut buf, "Test prompt").unwrap();
        assert!(buf.contains("Test prompt"));
    }

    #[test]
    fn test_format_confirm_prompt() {
        let theme = SpnTheme;
        let mut buf = String::new();
        theme
            .format_confirm_prompt(&mut buf, "Continue?", Some(true))
            .unwrap();
        assert!(buf.contains("Continue?"));
        assert!(buf.contains("Y/n"));
    }

    #[test]
    fn test_styles() {
        // Just ensure they don't panic
        let _ = styles::prompt();
        let _ = styles::active();
        let _ = styles::inactive();
        let _ = styles::success();
        let _ = styles::error();
    }
}
