//! Platform-specific utilities for native inference.
//!
//! This module provides:
//! - RAM detection for auto-quantization selection
//! - Default model storage directory

use std::path::PathBuf;

// ============================================================================
// Storage Location
// ============================================================================

/// Default model storage directory.
///
/// Models are stored in `~/.spn/models/` by default.
///
/// # Example
///
/// ```
/// use spn_native::default_model_dir;
///
/// let dir = default_model_dir();
/// assert!(dir.to_string_lossy().contains("models"));
/// ```
#[must_use]
pub fn default_model_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".spn").join("models"))
        .unwrap_or_else(|| PathBuf::from(".spn/models"))
}

// ============================================================================
// RAM Detection
// ============================================================================

/// Detect available system RAM in gigabytes.
///
/// Returns a conservative estimate if detection fails.
///
/// # Platform Support
///
/// - **macOS**: Uses `sysctl hw.memsize`
/// - **Linux**: Reads `/proc/meminfo`
/// - **Windows**: Returns 16GB default (TODO: implement via winapi)
/// - **Other**: Returns 8GB default
///
/// # Example
///
/// ```
/// use spn_native::detect_available_ram_gb;
///
/// let ram = detect_available_ram_gb();
/// println!("System has {}GB RAM", ram);
/// assert!(ram >= 1); // At least 1GB
/// ```
#[cfg(target_os = "macos")]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    use std::process::Command;

    let output = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok();

    output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|bytes| (bytes / 1_073_741_824) as u32) // bytes to GB
        .unwrap_or(8) // Conservative default
}

/// Detect available system RAM in gigabytes.
#[cfg(target_os = "linux")]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    use std::fs;

    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("MemTotal:"))
                .and_then(|line| {
                    line.split_whitespace()
                        .nth(1)
                        .and_then(|kb| kb.parse::<u64>().ok())
                })
        })
        .map(|kb| (kb / 1_048_576) as u32) // KB to GB
        .unwrap_or(8)
}

/// Detect available system RAM in gigabytes.
#[cfg(target_os = "windows")]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    // TODO: Use winapi to get actual RAM
    // For now, assume 16GB on Windows
    16
}

/// Detect available system RAM in gigabytes.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    8 // Conservative default
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_dir() {
        let dir = default_model_dir();
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.contains(".spn") || dir_str.contains("spn"));
        assert!(dir_str.contains("models"));
    }

    #[test]
    fn test_detect_ram() {
        let ram = detect_available_ram_gb();
        // Should return a reasonable value (at least 1GB, no more than 1TB)
        assert!(ram >= 1);
        assert!(ram <= 1024);
    }

    #[test]
    fn test_detect_ram_consistency() {
        // Multiple calls should return the same value
        let ram1 = detect_available_ram_gb();
        let ram2 = detect_available_ram_gb();
        assert_eq!(ram1, ram2);
    }
}
