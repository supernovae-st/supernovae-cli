//! Box-drawing character abstraction with Unicode/ASCII fallback.
//!
//! Provides consistent box rendering across terminals with different
//! capabilities, automatically falling back to ASCII when needed.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::ux::boxes::{BoxChars, BoxBuilder};
//!
//! let chars = BoxChars::detect();
//! let builder = BoxBuilder::new(60, chars);
//!
//! println!("{}", builder.top_border());
//! println!("{}", builder.content_line("Hello, World!"));
//! println!("{}", builder.bottom_border());
//! ```

use console::measure_text_width;

use super::terminal::TerminalCaps;

/// Box-drawing characters with Unicode and ASCII variants.
#[derive(Debug, Clone, Copy)]
pub struct BoxChars {
    // Single-line box characters
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,

    // T-junctions
    pub t_down: char,  // top edge with line going down
    pub t_up: char,    // bottom edge with line going up
    pub t_right: char, // left edge with line going right
    pub t_left: char,  // right edge with line going left

    // Arrows
    pub arrow_right: char,
    pub arrow_down: char,

    // Bullets
    pub bullet: char,
    pub bullet_hollow: char,
}

impl BoxChars {
    /// Unicode box-drawing characters (single line).
    pub const UNICODE: Self = Self {
        top_left: '\u{250C}',     // ┌
        top_right: '\u{2510}',    // ┐
        bottom_left: '\u{2514}',  // └
        bottom_right: '\u{2518}', // ┘
        horizontal: '\u{2500}',   // ─
        vertical: '\u{2502}',     // │
        t_down: '\u{252C}',       // ┬
        t_up: '\u{2534}',         // ┴
        t_right: '\u{251C}',      // ├
        t_left: '\u{2524}',       // ┤
        arrow_right: '\u{2192}',  // →
        arrow_down: '\u{2193}',   // ↓
        bullet: '\u{2022}',       // •
        bullet_hollow: '\u{25E6}', // ◦
    };

    /// ASCII fallback characters.
    pub const ASCII: Self = Self {
        top_left: '+',
        top_right: '+',
        bottom_left: '+',
        bottom_right: '+',
        horizontal: '-',
        vertical: '|',
        t_down: '+',
        t_up: '+',
        t_right: '+',
        t_left: '+',
        arrow_right: '>',
        arrow_down: 'v',
        bullet: '*',
        bullet_hollow: 'o',
    };

    /// Detect appropriate character set based on terminal capabilities.
    #[must_use]
    pub fn detect() -> Self {
        let caps = TerminalCaps::detect();
        if caps.unicode {
            Self::UNICODE
        } else {
            Self::ASCII
        }
    }

    /// Get characters for specific unicode support setting.
    #[must_use]
    pub fn for_unicode(unicode: bool) -> Self {
        if unicode {
            Self::UNICODE
        } else {
            Self::ASCII
        }
    }
}

impl Default for BoxChars {
    fn default() -> Self {
        Self::UNICODE
    }
}

/// Box style variants.
#[derive(Debug, Clone, Copy, Default)]
pub enum BoxStyle {
    /// Single-line border (default)
    #[default]
    Single,
    /// No visible border (content only)
    None,
}

/// Builder for constructing box elements with dynamic width.
#[derive(Debug, Clone)]
pub struct BoxBuilder {
    /// Width of the box interior (excluding borders).
    pub width: u16,
    /// Box-drawing characters to use.
    pub chars: BoxChars,
    /// Box style.
    pub style: BoxStyle,
}

impl BoxBuilder {
    /// Create a new box builder with specified width.
    #[must_use]
    pub fn new(width: u16, chars: BoxChars) -> Self {
        Self {
            width,
            chars,
            style: BoxStyle::Single,
        }
    }

    /// Create a box builder from terminal capabilities.
    #[must_use]
    pub fn from_caps(caps: &TerminalCaps) -> Self {
        Self::new(caps.box_width(), BoxChars::for_unicode(caps.unicode))
    }

    /// Set box style.
    #[must_use]
    pub fn with_style(mut self, style: BoxStyle) -> Self {
        self.style = style;
        self
    }

    /// Generate top border line.
    #[must_use]
    pub fn top_border(&self) -> String {
        match self.style {
            BoxStyle::Single => {
                let c = &self.chars;
                format!(
                    "{}{}{}",
                    c.top_left,
                    c.horizontal.to_string().repeat(self.width as usize),
                    c.top_right
                )
            }
            BoxStyle::None => String::new(),
        }
    }

    /// Generate bottom border line.
    #[must_use]
    pub fn bottom_border(&self) -> String {
        match self.style {
            BoxStyle::Single => {
                let c = &self.chars;
                format!(
                    "{}{}{}",
                    c.bottom_left,
                    c.horizontal.to_string().repeat(self.width as usize),
                    c.bottom_right
                )
            }
            BoxStyle::None => String::new(),
        }
    }

    /// Generate separator line (horizontal divider within box).
    #[must_use]
    pub fn separator(&self) -> String {
        match self.style {
            BoxStyle::Single => {
                let c = &self.chars;
                format!(
                    "{}{}{}",
                    c.t_right,
                    c.horizontal.to_string().repeat(self.width as usize),
                    c.t_left
                )
            }
            BoxStyle::None => String::new(),
        }
    }

    /// Generate a content line with left/right borders.
    ///
    /// Content is left-aligned with padding to fill the width.
    #[must_use]
    pub fn content_line(&self, content: &str) -> String {
        match self.style {
            BoxStyle::Single => {
                let c = &self.chars;
                let content_width = measure_text_width(content);
                let padding = (self.width as usize).saturating_sub(content_width);
                format!(
                    "{}{}{}{}",
                    c.vertical,
                    content,
                    " ".repeat(padding),
                    c.vertical
                )
            }
            BoxStyle::None => content.to_string(),
        }
    }

    /// Generate a content line with text centered.
    #[must_use]
    pub fn centered_line(&self, content: &str) -> String {
        match self.style {
            BoxStyle::Single => {
                let c = &self.chars;
                let content_width = measure_text_width(content);
                let total_padding = (self.width as usize).saturating_sub(content_width);
                let left_pad = total_padding / 2;
                let right_pad = total_padding - left_pad;
                format!(
                    "{}{}{}{}{}",
                    c.vertical,
                    " ".repeat(left_pad),
                    content,
                    " ".repeat(right_pad),
                    c.vertical
                )
            }
            BoxStyle::None => content.to_string(),
        }
    }

    /// Generate an empty line within the box.
    #[must_use]
    pub fn empty_line(&self) -> String {
        match self.style {
            BoxStyle::Single => {
                let c = &self.chars;
                format!(
                    "{}{}{}",
                    c.vertical,
                    " ".repeat(self.width as usize),
                    c.vertical
                )
            }
            BoxStyle::None => String::new(),
        }
    }

    /// Generate a line with a key-value pair.
    ///
    /// Key is displayed, followed by spacing, then value right-aligned.
    #[must_use]
    pub fn key_value_line(&self, key: &str, value: &str) -> String {
        let key_width = measure_text_width(key);
        let value_width = measure_text_width(value);
        let spacing = (self.width as usize)
            .saturating_sub(key_width)
            .saturating_sub(value_width);

        let content = format!("{}{}{}", key, " ".repeat(spacing), value);
        self.content_line(&content)
    }
}

/// Create a simple box around content lines.
pub fn simple_box(lines: &[&str], chars: BoxChars, width: u16) -> String {
    let builder = BoxBuilder::new(width, chars);
    let mut result = Vec::with_capacity(lines.len() + 2);

    result.push(builder.top_border());
    for line in lines {
        result.push(builder.content_line(line));
    }
    result.push(builder.bottom_border());

    result.join("\n")
}

/// Create a box with a title header.
pub fn titled_box(title: &str, lines: &[&str], chars: BoxChars, width: u16) -> String {
    let builder = BoxBuilder::new(width, chars);
    let mut result = Vec::with_capacity(lines.len() + 4);

    result.push(builder.top_border());
    result.push(builder.centered_line(title));
    result.push(builder.separator());
    for line in lines {
        result.push(builder.content_line(line));
    }
    result.push(builder.bottom_border());

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_chars_unicode() {
        let chars = BoxChars::UNICODE;
        assert_eq!(chars.top_left, '┌');
        assert_eq!(chars.horizontal, '─');
        assert_eq!(chars.vertical, '│');
    }

    #[test]
    fn test_box_chars_ascii() {
        let chars = BoxChars::ASCII;
        assert_eq!(chars.top_left, '+');
        assert_eq!(chars.horizontal, '-');
        assert_eq!(chars.vertical, '|');
    }

    #[test]
    fn test_box_chars_detect() {
        // Should not panic
        let _chars = BoxChars::detect();
    }

    #[test]
    fn test_box_builder_top_border() {
        let builder = BoxBuilder::new(10, BoxChars::UNICODE);
        let border = builder.top_border();
        assert_eq!(border, "┌──────────┐");
    }

    #[test]
    fn test_box_builder_top_border_ascii() {
        let builder = BoxBuilder::new(10, BoxChars::ASCII);
        let border = builder.top_border();
        assert_eq!(border, "+----------+");
    }

    #[test]
    fn test_box_builder_content_line() {
        let builder = BoxBuilder::new(10, BoxChars::UNICODE);
        let line = builder.content_line("Hello");
        assert_eq!(line, "│Hello     │");
    }

    #[test]
    fn test_box_builder_centered_line() {
        let builder = BoxBuilder::new(10, BoxChars::UNICODE);
        let line = builder.centered_line("Hi");
        assert_eq!(line, "│    Hi    │");
    }

    #[test]
    fn test_box_builder_empty_line() {
        let builder = BoxBuilder::new(10, BoxChars::UNICODE);
        let line = builder.empty_line();
        assert_eq!(line, "│          │");
    }

    #[test]
    fn test_box_builder_key_value_line() {
        let builder = BoxBuilder::new(20, BoxChars::UNICODE);
        let line = builder.key_value_line("Key:", "Value");
        assert!(line.starts_with('│'));
        assert!(line.ends_with('│'));
        assert!(line.contains("Key:"));
        assert!(line.contains("Value"));
    }

    #[test]
    fn test_simple_box() {
        let result = simple_box(&["Line 1", "Line 2"], BoxChars::UNICODE, 10);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with('┌'));
        assert!(lines[1].starts_with('│'));
        assert!(lines[3].starts_with('└'));
    }

    #[test]
    fn test_titled_box() {
        let result = titled_box("Title", &["Content"], BoxChars::UNICODE, 10);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(lines[0].starts_with('┌'));
        assert!(lines[1].contains("Title"));
        assert!(lines[2].starts_with('├')); // separator
        assert!(lines[3].contains("Content"));
        assert!(lines[4].starts_with('└'));
    }

    #[test]
    fn test_box_style_none() {
        let builder = BoxBuilder::new(10, BoxChars::UNICODE).with_style(BoxStyle::None);
        assert_eq!(builder.top_border(), "");
        assert_eq!(builder.content_line("Hello"), "Hello");
    }
}
