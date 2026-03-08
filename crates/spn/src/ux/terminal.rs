//! Terminal capability detection for adaptive UI rendering.
//!
//! Detects terminal width, height, unicode support, and color capabilities
//! to enable responsive layouts and graceful degradation.
//!
//! # Layout Modes
//!
//! - `Wide` (>= 100 columns): Full feature display with detailed descriptions
//! - `Normal` (>= 80 columns): Standard display with compact descriptions
//! - `Narrow` (>= 60 columns): Compressed display, abbreviated text
//! - `Minimal` (< 60 columns): Essential info only
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::ux::terminal::{TerminalCaps, LayoutMode};
//!
//! let caps = TerminalCaps::detect();
//! match caps.layout {
//!     LayoutMode::Wide => println!("Full feature display"),
//!     LayoutMode::Normal => println!("Standard display"),
//!     LayoutMode::Narrow => println!("Compressed display"),
//!     LayoutMode::Minimal => println!("Minimal display"),
//! }
//! ```

use console::Term;
use std::env;

/// Terminal layout mode based on width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
    /// Wide terminal (>= 100 columns) - full feature display
    Wide,
    /// Normal terminal (>= 80 columns) - standard display
    #[default]
    Normal,
    /// Narrow terminal (>= 60 columns) - compressed display
    Narrow,
    /// Minimal terminal (< 60 columns) - essential info only
    Minimal,
}

impl LayoutMode {
    /// Determine layout mode from terminal width.
    #[must_use]
    pub fn from_width(width: u16) -> Self {
        match width {
            w if w >= 100 => Self::Wide,
            w if w >= 80 => Self::Normal,
            w if w >= 60 => Self::Narrow,
            _ => Self::Minimal,
        }
    }

    /// Get the effective content width for this layout mode.
    ///
    /// Returns the maximum width for content, accounting for margins.
    #[must_use]
    pub fn content_width(&self, terminal_width: u16) -> u16 {
        let max_width = match self {
            Self::Wide => 100,
            Self::Normal => 80,
            Self::Narrow => 60,
            Self::Minimal => terminal_width.saturating_sub(2),
        };
        terminal_width.min(max_width)
    }

    /// Check if this is a wide layout.
    #[must_use]
    pub fn is_wide(&self) -> bool {
        matches!(self, Self::Wide)
    }

    /// Check if this is at least normal width.
    #[must_use]
    pub fn is_normal_or_wider(&self) -> bool {
        matches!(self, Self::Wide | Self::Normal)
    }
}

/// Terminal capabilities for adaptive rendering.
#[derive(Debug, Clone)]
pub struct TerminalCaps {
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Whether unicode box-drawing characters are supported.
    pub unicode: bool,
    /// Whether color output is supported.
    pub color: bool,
    /// Layout mode based on width.
    pub layout: LayoutMode,
}

impl Default for TerminalCaps {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
            unicode: true,
            color: true,
            layout: LayoutMode::Normal,
        }
    }
}

impl TerminalCaps {
    /// Detect terminal capabilities.
    ///
    /// Checks terminal size, unicode support, and color capabilities.
    #[must_use]
    pub fn detect() -> Self {
        let term = Term::stdout();
        let (height, width) = term.size();

        // Ensure minimum dimensions
        let width = width.max(40);
        let height = height.max(10);

        let unicode = detect_unicode_support();
        let color = detect_color_support(&term);
        let layout = LayoutMode::from_width(width);

        Self {
            width,
            height,
            unicode,
            color,
            layout,
        }
    }

    /// Get the effective content width for rendering.
    #[must_use]
    pub fn content_width(&self) -> u16 {
        self.layout.content_width(self.width)
    }

    /// Get the box width for help screen rendering.
    ///
    /// Returns a width suitable for box drawing, with some margin.
    #[must_use]
    pub fn box_width(&self) -> u16 {
        // Leave 2 chars margin on each side
        self.content_width().saturating_sub(4).max(40)
    }
}

/// Detect if terminal supports unicode characters.
///
/// Checks for:
/// - `SPN_ASCII=1` environment variable (forces ASCII)
/// - `TERM=dumb` (no unicode)
/// - `LC_ALL`, `LC_CTYPE`, `LANG` containing UTF-8 indicators
fn detect_unicode_support() -> bool {
    // Check for forced ASCII mode
    if env::var("SPN_ASCII").map(|v| v == "1").unwrap_or(false) {
        return false;
    }

    // Check for dumb terminal
    if env::var("TERM").map(|v| v == "dumb").unwrap_or(false) {
        return false;
    }

    // Check locale settings for UTF-8 support
    let check_utf8 = |var: &str| -> bool {
        env::var(var)
            .map(|v| {
                let v = v.to_lowercase();
                v.contains("utf-8") || v.contains("utf8")
            })
            .unwrap_or(false)
    };

    // If any locale indicates UTF-8, assume unicode support
    if check_utf8("LC_ALL") || check_utf8("LC_CTYPE") || check_utf8("LANG") {
        return true;
    }

    // Default to unicode on modern systems (most terminals support it)
    // Only fall back to ASCII if explicitly requested
    true
}

/// Detect if terminal supports color output.
fn detect_color_support(term: &Term) -> bool {
    // Check for NO_COLOR environment variable (standard)
    if env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check for dumb terminal
    if env::var("TERM").map(|v| v == "dumb").unwrap_or(false) {
        return false;
    }

    // Use console crate's detection
    term.is_term()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_mode_from_width() {
        assert_eq!(LayoutMode::from_width(120), LayoutMode::Wide);
        assert_eq!(LayoutMode::from_width(100), LayoutMode::Wide);
        assert_eq!(LayoutMode::from_width(99), LayoutMode::Normal);
        assert_eq!(LayoutMode::from_width(80), LayoutMode::Normal);
        assert_eq!(LayoutMode::from_width(79), LayoutMode::Narrow);
        assert_eq!(LayoutMode::from_width(60), LayoutMode::Narrow);
        assert_eq!(LayoutMode::from_width(59), LayoutMode::Minimal);
        assert_eq!(LayoutMode::from_width(40), LayoutMode::Minimal);
    }

    #[test]
    fn test_layout_mode_content_width() {
        assert_eq!(LayoutMode::Wide.content_width(120), 100);
        assert_eq!(LayoutMode::Normal.content_width(90), 80);
        assert_eq!(LayoutMode::Narrow.content_width(70), 60);
        assert_eq!(LayoutMode::Minimal.content_width(50), 48);
    }

    #[test]
    fn test_layout_mode_predicates() {
        assert!(LayoutMode::Wide.is_wide());
        assert!(!LayoutMode::Normal.is_wide());

        assert!(LayoutMode::Wide.is_normal_or_wider());
        assert!(LayoutMode::Normal.is_normal_or_wider());
        assert!(!LayoutMode::Narrow.is_normal_or_wider());
    }

    #[test]
    fn test_terminal_caps_detect() {
        // Should not panic
        let caps = TerminalCaps::detect();
        assert!(caps.width >= 40);
        assert!(caps.height >= 10);
    }

    #[test]
    fn test_terminal_caps_default() {
        let caps = TerminalCaps::default();
        assert_eq!(caps.width, 80);
        assert_eq!(caps.height, 24);
        assert!(caps.unicode);
        assert!(caps.color);
        assert_eq!(caps.layout, LayoutMode::Normal);
    }

    #[test]
    fn test_terminal_caps_box_width() {
        let caps = TerminalCaps {
            width: 100,
            height: 24,
            unicode: true,
            color: true,
            layout: LayoutMode::Normal,
        };
        // box_width = content_width - 4 = 80 - 4 = 76
        assert_eq!(caps.box_width(), 76);
    }
}
