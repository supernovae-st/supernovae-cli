//! ASCII tables for structured data display.
//!
//! Provides formatted tables with:
//! - Auto-calculated column widths
//! - Header rows with separators
//! - Semantic styling for different data types
//! - Terminal width awareness
//!
//! # Example
//!
//! ```text
//! ┌────────────────┬─────────────┬──────────────────────┐
//! │ Provider       │ Source      │ Status               │
//! ├────────────────┼─────────────┼──────────────────────┤
//! │ anthropic      │ ✓ Keychain  │ Secure               │
//! │ openai         │ ⚠ .env      │ Migrate recommended  │
//! └────────────────┴─────────────┴──────────────────────┘
//! ```

use console::{style, Term};

use super::design_system::icon;

// ============================================================================
// TABLE CHARACTERS (Box Drawing)
// ============================================================================

/// Box drawing characters for tables
pub mod chars {
    pub const TOP_LEFT: &str = "┌";
    pub const TOP_RIGHT: &str = "┐";
    pub const BOTTOM_LEFT: &str = "└";
    pub const BOTTOM_RIGHT: &str = "┘";
    pub const HORIZONTAL: &str = "─";
    pub const VERTICAL: &str = "│";
    pub const T_DOWN: &str = "┬";
    pub const T_UP: &str = "┴";
    pub const T_RIGHT: &str = "├";
    pub const T_LEFT: &str = "┤";
    pub const CROSS: &str = "┼";
}

// ============================================================================
// SIMPLE TABLE
// ============================================================================

/// A simple table builder.
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<usize>,
}

impl Table {
    /// Create a new table with headers.
    pub fn new(headers: &[&str]) -> Self {
        let headers: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
        let col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        Self {
            headers,
            rows: Vec::new(),
            col_widths,
        }
    }

    /// Add a row to the table.
    pub fn add_row(&mut self, row: &[&str]) -> &mut Self {
        let row: Vec<String> = row.iter().map(|s| s.to_string()).collect();

        // Update column widths
        for (i, cell) in row.iter().enumerate() {
            if i < self.col_widths.len() {
                self.col_widths[i] = self.col_widths[i].max(strip_ansi(cell).len());
            }
        }

        self.rows.push(row);
        self
    }

    /// Render the table as a string.
    pub fn render(&self) -> String {
        let mut output = String::new();

        // Top border
        output.push_str(&self.render_border(chars::TOP_LEFT, chars::T_DOWN, chars::TOP_RIGHT));
        output.push('\n');

        // Headers
        output.push_str(&self.render_row(&self.headers, true));
        output.push('\n');

        // Header separator
        output.push_str(&self.render_border(chars::T_RIGHT, chars::CROSS, chars::T_LEFT));
        output.push('\n');

        // Rows
        for row in &self.rows {
            output.push_str(&self.render_row(row, false));
            output.push('\n');
        }

        // Bottom border
        output.push_str(&self.render_border(chars::BOTTOM_LEFT, chars::T_UP, chars::BOTTOM_RIGHT));

        output
    }

    fn render_border(&self, left: &str, mid: &str, right: &str) -> String {
        let mut border = String::from(left);
        for (i, width) in self.col_widths.iter().enumerate() {
            border.push_str(&chars::HORIZONTAL.repeat(width + 2));
            if i < self.col_widths.len() - 1 {
                border.push_str(mid);
            }
        }
        border.push_str(right);
        border
    }

    fn render_row(&self, row: &[String], is_header: bool) -> String {
        let mut line = String::from(chars::VERTICAL);
        for (i, cell) in row.iter().enumerate() {
            let width = self.col_widths.get(i).copied().unwrap_or(10);
            let visible_len = strip_ansi(cell).len();
            let padding = width.saturating_sub(visible_len);

            if is_header {
                line.push_str(&format!(" {}{} ", style(cell).bold(), " ".repeat(padding)));
            } else {
                line.push_str(&format!(" {}{} ", cell, " ".repeat(padding)));
            }
            line.push_str(chars::VERTICAL);
        }
        line
    }
}

// ============================================================================
// SPECIALIZED TABLES
// ============================================================================

/// Provider status table with semantic styling.
pub fn provider_table(
    providers: &[(String, Option<crate::secrets::SecretSource>, String)],
) -> String {
    let mut table = Table::new(&["Provider", "Source", "Status"]);

    for (name, source, status) in providers {
        let source_str = match source {
            Some(crate::secrets::SecretSource::Keychain) => {
                format!("{} Keychain", style(icon::SUCCESS).green())
            }
            Some(crate::secrets::SecretSource::Environment) => {
                format!("{} Env var", style(icon::WARNING).yellow())
            }
            Some(crate::secrets::SecretSource::DotEnv) => {
                format!("{} .env", style(icon::WARNING).yellow())
            }
            Some(crate::secrets::SecretSource::Inline) => {
                format!("{} Inline", style(icon::ERROR).red())
            }
            None => format!("{} Missing", style(icon::ERROR).red()),
        };

        table.add_row(&[name, &source_str, status]);
    }

    table.render()
}

/// Package list table.
pub fn package_table(packages: &[(&str, &str, &str)]) -> String {
    let mut table = Table::new(&["Package", "Version", "Status"]);

    for (name, version, status) in packages {
        let styled_name = format!("{}", style(*name).yellow().bold());
        let styled_version = format!("{}", style(*version).green());
        table.add_row(&[&styled_name, &styled_version, status]);
    }

    table.render()
}

// ============================================================================
// KEY-VALUE LIST (Alternative to table)
// ============================================================================

/// Render a key-value list with aligned values.
///
/// ```text
///   Provider:  anthropic
///   Source:    Keychain
///   Status:    Secure
/// ```
pub fn key_value_list(items: &[(&str, &str)]) -> String {
    if items.is_empty() {
        return String::new();
    }

    // Find max key length
    let max_key_len = items.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

    let mut output = String::new();
    for (key, value) in items {
        let padding = max_key_len - key.len();
        output.push_str(&format!(
            "  {}:{} {}\n",
            style(*key).bold(),
            " ".repeat(padding + 1),
            value
        ));
    }
    output.trim_end().to_string()
}

/// Render a compact status list.
///
/// ```text
///   ✓ anthropic   Keychain
///   ⚠ openai      .env
///   ✗ github      Missing
/// ```
pub fn status_list(items: &[(&str, &str, &str)]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let max_name_len = items.iter().map(|(n, _, _)| n.len()).max().unwrap_or(0);

    let mut output = String::new();
    for (icon_str, name, status) in items {
        let padding = max_name_len - name.len();
        output.push_str(&format!(
            "  {} {}{} {}\n",
            icon_str,
            style(*name).bold(),
            " ".repeat(padding + 1),
            style(*status).dim()
        ));
    }
    output.trim_end().to_string()
}

// ============================================================================
// HELPERS
// ============================================================================

/// Strip ANSI escape codes for width calculation.
fn strip_ansi(s: &str) -> String {
    // Simple regex-free ANSI stripping
    let mut result = String::new();
    let mut in_escape = false;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Get terminal width (with fallback).
pub fn terminal_width() -> usize {
    Term::stdout().size().1 as usize
}

/// Get content width for dashboard rendering.
///
/// Returns a width clamped between MIN_WIDTH and MAX_WIDTH for consistent
/// formatting across different terminal sizes.
pub fn content_width() -> usize {
    const MIN_WIDTH: usize = 60;
    const MAX_WIDTH: usize = 100;

    let width = terminal_width();
    // Leave 2 chars margin on each side
    let usable = width.saturating_sub(4);
    usable.clamp(MIN_WIDTH, MAX_WIDTH)
}

/// Truncate a string to fit within max_len, adding "..." if needed.
pub fn truncate(s: &str, max_len: usize) -> String {
    if max_len <= 3 {
        return "...".to_string();
    }

    let stripped = strip_ansi(s);
    if stripped.len() <= max_len {
        s.to_string()
    } else {
        // Account for "..." suffix
        let truncate_at = max_len.saturating_sub(3);

        // Find a good break point (don't break in middle of word if possible)
        let break_at = s[..truncate_at.min(s.len())]
            .rfind(|c: char| c.is_whitespace() || c == '/')
            .unwrap_or(truncate_at.min(s.len()));

        format!("{}...", &s[..break_at])
    }
}

/// Create a horizontal border line of specified width.
pub fn border_line(width: usize, left: &str, fill: &str, right: &str) -> String {
    format!("{}{}{}", left, fill.repeat(width), right)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table() {
        let mut table = Table::new(&["Name", "Value"]);
        table.add_row(&["foo", "bar"]);
        table.add_row(&["longer_name", "x"]);

        let rendered = table.render();
        assert!(rendered.contains("Name"));
        assert!(rendered.contains("foo"));
        assert!(rendered.contains("┌"));
        assert!(rendered.contains("└"));
    }

    #[test]
    fn test_key_value_list() {
        let items = vec![("Key1", "Value1"), ("LongerKey", "Value2")];
        let output = key_value_list(&items);

        assert!(output.contains("Key1"));
        assert!(output.contains("LongerKey"));
    }

    #[test]
    fn test_strip_ansi() {
        let with_ansi = "\x1b[31mred\x1b[0m";
        let without = strip_ansi(with_ansi);
        assert_eq!(without, "red");
    }

    #[test]
    fn test_empty_table() {
        let table = Table::new(&["A", "B"]);
        let rendered = table.render();
        assert!(rendered.contains("A"));
        assert!(rendered.contains("B"));
    }

    #[test]
    fn test_terminal_width() {
        let width = terminal_width();
        // Should return a reasonable value (at least 20, likely 80+)
        assert!(width >= 20);
    }

    #[test]
    fn test_content_width() {
        let width = content_width();
        // Should be between 60 and 100
        assert!(width >= 60);
        assert!(width <= 100);
    }

    #[test]
    fn test_truncate_short_string() {
        // String shorter than max_len should be unchanged
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        // String longer than max_len should be truncated with "..."
        let result = truncate("this is a very long string", 15);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 15);
    }

    #[test]
    fn test_truncate_at_word_boundary() {
        // Should break at word boundary when possible
        let result = truncate("hello world test", 12);
        assert!(result.ends_with("..."));
        // Should break at "hello" rather than in middle of "world"
        assert_eq!(result, "hello...");
    }

    #[test]
    fn test_border_line() {
        let border = border_line(5, "[", "-", "]");
        assert_eq!(border, "[-----]");
    }
}
