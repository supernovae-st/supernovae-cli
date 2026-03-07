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
        output.push_str(&self.render_border(
            chars::BOTTOM_LEFT,
            chars::T_UP,
            chars::BOTTOM_RIGHT,
        ));

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
#[allow(dead_code)]
pub fn terminal_width() -> usize {
    Term::stdout().size().1 as usize
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
}
