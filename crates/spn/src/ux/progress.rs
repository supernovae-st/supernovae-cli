//! Enhanced progress bars with multi-column support.
//!
//! Provides themed progress bars optimized for different operations:
//! - Downloads with speed and ETA
//! - Multi-step wizards with step tracking
//! - Transforming spinners (spinner → checkmark on success)
//!
//! # Best Practices (from Spectre.Console research)
//!
//! 1. Never overlap multiple live widgets
//! 2. Keep rendering on main thread
//! 3. Use semantic templates, not raw ANSI

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

use super::design_system::icon;

// ============================================================================
// DOWNLOAD PROGRESS
// ============================================================================

/// Create a download progress bar with speed and ETA.
///
/// Template: `{prefix} {bar:30} {bytes}/{total_bytes} ({bytes_per_sec}) ETA: {eta}`
pub fn download_bar(total: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "{prefix} {{bar:30.cyan/blue}} {{bytes}}/{{total_bytes}} ({{bytes_per_sec}}) ETA: {{eta}}"
            ))
            .unwrap()
            .progress_chars("━━╺"),
    );
    pb.set_prefix(prefix.to_string());
    pb
}

/// Create a simple progress bar for known-length operations.
pub fn progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:25.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("▰▰▱"),
    );
    pb.set_message(message.to_string());
    pb
}

// ============================================================================
// SPINNER VARIANTS
// ============================================================================

/// Spinner frames for different themes
pub mod frames {
    /// Default circular spinner
    pub const CIRCULAR: &[&str] = &["◐", "◓", "◑", "◒"];

    /// Braille dots (smoother animation)
    pub const BRAILLE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    /// Arc spinner (subtle)
    pub const ARC: &[&str] = &["◜", "◠", "◝", "◞", "◡", "◟"];

    /// Download wave
    pub const WAVE: &[&str] = &["    ", "=   ", "==  ", "=== ", " ===", "  ==", "   =", "    "];

    /// Install blocks
    pub const BLOCKS: &[&str] = &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▱▰▰", "▱▱▰"];

    /// Sync arrows
    pub const SYNC: &[&str] = &["↻", "↺"];
}

/// Create a spinner with braille animation (smoothest).
pub fn spinner(message: &str) -> ProgressBar {
    spinner_with_frames(message, frames::BRAILLE)
}

/// Create a spinner with custom frames.
pub fn spinner_with_frames(message: &str, spinner_frames: &[&str]) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(spinner_frames)
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a spinner that transforms into a checkmark on finish.
///
/// Call `finish_success()` or `finish_error()` to show the final state.
pub fn transforming_spinner(message: &str) -> TransformingSpinner {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(frames::BRAILLE)
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    TransformingSpinner { pb }
}

/// A spinner that can transform into success/error icons.
pub struct TransformingSpinner {
    pb: ProgressBar,
}

impl TransformingSpinner {
    /// Finish with success checkmark.
    pub fn finish_success(&self, message: &str) {
        self.pb.finish_with_message(format!(
            "{} {}",
            style(icon::SUCCESS).green().bold(),
            style(message).green()
        ));
    }

    /// Finish with error cross.
    pub fn finish_error(&self, message: &str) {
        self.pb.finish_with_message(format!(
            "{} {}",
            style(icon::ERROR).red().bold(),
            style(message).red()
        ));
    }

    /// Finish with warning icon.
    pub fn finish_warning(&self, message: &str) {
        self.pb.finish_with_message(format!(
            "{} {}",
            style(icon::WARNING).yellow().bold(),
            style(message).yellow()
        ));
    }

    /// Update the message while spinning.
    pub fn set_message(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }

    /// Get the underlying progress bar for advanced use.
    pub fn inner(&self) -> &ProgressBar {
        &self.pb
    }
}

// ============================================================================
// MULTI-PROGRESS (PARALLEL DOWNLOADS)
// ============================================================================

/// Create a multi-progress manager for parallel operations.
pub fn multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// Add a download bar to a multi-progress.
pub fn add_download(mp: &MultiProgress, total: u64, name: &str) -> ProgressBar {
    let pb = mp.add(ProgressBar::new(total));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{prefix:>20} {bar:20.cyan/blue} {bytes:>10}/{total_bytes:<10} {msg}")
            .unwrap()
            .progress_chars("━━╺"),
    );
    pb.set_prefix(name.to_string());
    pb
}

// ============================================================================
// STEP PROGRESS (WIZARD-STYLE)
// ============================================================================

/// Create a step-by-step progress tracker.
///
/// Shows: `[1/5] Current step message`
pub fn step_progress(current: usize, total: usize, message: &str) -> String {
    format!(
        "{} {}",
        style(format!("[{}/{}]", current, total)).cyan().bold(),
        message
    )
}

/// Print a completed step.
pub fn step_done(step: usize, total: usize, message: &str) {
    println!(
        "{} {} {}",
        style(format!("[{}/{}]", step, total)).cyan().bold(),
        style(icon::SUCCESS).green().bold(),
        style(message).green()
    );
}

/// Print a pending step.
pub fn step_pending(step: usize, total: usize, message: &str) {
    println!(
        "{} {} {}",
        style(format!("[{}/{}]", step, total)).dim(),
        style("○").dim(),
        style(message).dim()
    );
}

/// Print current step (in progress).
pub fn step_current(step: usize, total: usize, message: &str) {
    println!(
        "{} {} {}",
        style(format!("[{}/{}]", step, total)).cyan().bold(),
        style("▸").cyan().bold(),
        style(message).bold()
    );
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
    fn test_transforming_spinner() {
        let ts = transforming_spinner("Loading...");
        ts.set_message("Still loading...");
        ts.finish_success("Done!");
    }

    #[test]
    fn test_progress_bar() {
        let pb = progress_bar(100, "Processing");
        pb.inc(50);
        pb.finish_and_clear();
    }

    #[test]
    fn test_step_progress_format() {
        let s = step_progress(1, 5, "First step");
        assert!(s.contains("[1/5]"));
        assert!(s.contains("First step"));
    }

    #[test]
    fn test_frames_defined() {
        assert!(!frames::CIRCULAR.is_empty());
        assert!(!frames::BRAILLE.is_empty());
        assert!(!frames::ARC.is_empty());
    }
}
