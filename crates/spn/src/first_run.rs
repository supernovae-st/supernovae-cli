//! First-run detection and management for v0.14.0.
//!
//! Detects when spn is run for the first time and provides
//! a guided onboarding experience.

use std::fs;
use std::path::PathBuf;

/// Get the path to the first-run marker file
fn marker_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("spn")
        .join(".first_run_complete")
}

/// Check if this is the first run (marker doesn't exist)
pub fn is_first_run() -> bool {
    !marker_path().exists()
}

/// Mark the first run as complete
pub fn mark_complete() -> std::io::Result<()> {
    let path = marker_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("completed: {}", chrono::Utc::now().to_rfc3339()))
}

/// Reset the first-run state (for testing)
#[allow(dead_code)]
pub fn reset() -> std::io::Result<()> {
    let path = marker_path();
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_path_exists() {
        let path = marker_path();
        assert!(path.to_string_lossy().contains("spn"));
    }
}
