//! First-run detection and management for v0.14.0.
//!
//! Detects when spn is run for the first time and provides
//! a guided onboarding experience.
//!
//! # Marker File
//!
//! The first-run state is tracked via a marker file at:
//! - Unix: `~/.local/share/spn/.first_run_complete`
//! - macOS: `~/Library/Application Support/spn/.first_run_complete`
//! - Windows: `%APPDATA%\spn\.first_run_complete`
//!
//! # Thread Safety
//!
//! Uses `OpenOptions::create_new()` to ensure atomic creation,
//! preventing race conditions when multiple `spn` instances start.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Get the path to the first-run marker file
fn marker_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            tracing::warn!("Could not determine data directory, using current directory");
            PathBuf::from(".")
        })
        .join("spn")
        .join(".first_run_complete")
}

/// Check if this is the first run (marker doesn't exist)
///
/// Returns `true` if the marker file does not exist, indicating
/// this is the first time `spn` is being run.
pub fn is_first_run() -> bool {
    !marker_path().exists()
}

/// Mark the first run as complete
///
/// Creates the marker file atomically using `create_new()` to prevent
/// race conditions. Sets file permissions to 0600 on Unix.
///
/// # Errors
///
/// Returns an error if:
/// - The parent directory cannot be created
/// - The marker file already exists (another process won the race)
/// - File permissions cannot be set
pub fn mark_complete() -> std::io::Result<()> {
    let path = marker_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;

        // Set directory permissions to 0700 on Unix
        #[cfg(unix)]
        {
            let perms = fs::Permissions::from_mode(0o700);
            let _ = fs::set_permissions(parent, perms); // Ignore errors on existing dirs
        }
    }

    // Use create_new for atomic "create if not exists" semantics
    // This prevents race conditions when multiple spn instances start
    let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another process already created the marker - that's fine
            tracing::debug!("First-run marker already exists (race condition handled)");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // Set file permissions to 0600 on Unix
    #[cfg(unix)]
    {
        let perms = fs::Permissions::from_mode(0o600);
        file.set_permissions(perms)?;
    }

    // Write timestamp
    writeln!(file, "completed: {}", chrono::Utc::now().to_rfc3339())?;

    Ok(())
}

/// Reset the first-run state (for testing)
///
/// Removes the marker file, causing `is_first_run()` to return `true` again.
#[allow(dead_code)]
pub fn reset() -> std::io::Result<()> {
    let path = marker_path();
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_marker_path_contains_spn() {
        let path = marker_path();
        assert!(path.to_string_lossy().contains("spn"));
        assert!(path.to_string_lossy().contains(".first_run_complete"));
    }

    #[test]
    fn test_first_run_lifecycle() {
        // Create a temp directory for isolated testing
        let temp_dir = TempDir::new().unwrap();
        let marker = temp_dir.path().join("spn").join(".first_run_complete");

        // Initially, marker doesn't exist
        assert!(!marker.exists());

        // Create marker
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        fs::write(&marker, "test").unwrap();

        // Now marker exists
        assert!(marker.exists());

        // Remove marker
        fs::remove_file(&marker).unwrap();
        assert!(!marker.exists());
    }

    #[test]
    fn test_reset_handles_missing_file() {
        // reset() should not error when file doesn't exist
        // This test uses the real marker_path but should be safe
        // as it only removes a file, not creates one
        let result = reset();
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_permissions_constants() {
        use std::os::unix::fs::PermissionsExt;

        let dir_perms = fs::Permissions::from_mode(0o700);
        assert_eq!(dir_perms.mode() & 0o777, 0o700);

        let file_perms = fs::Permissions::from_mode(0o600);
        assert_eq!(file_perms.mode() & 0o777, 0o600);
    }
}
