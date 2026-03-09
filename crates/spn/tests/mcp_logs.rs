//! Tests for MCP log functionality.

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Test level filtering logic.
///
/// Note: This test duplicates the should_show_line logic from mcp.rs
/// since the function is private. If refactored to a utility, update this test.
#[test]
fn test_level_filtering_logic() {
    // Helper function that mirrors should_show_line
    fn should_show_line(line: &str, level_filter: Option<&str>) -> bool {
        let Some(filter) = level_filter else {
            return true;
        };

        let line_upper = line.to_uppercase();

        // Helper to check for log level markers (e.g., [INFO], INFO:, INFO])
        let has_level = |level: &str| {
            line_upper.contains(&format!("[{}]", level))
                || line_upper.contains(&format!("[{}:", level))
                || line_upper.contains(&format!("{}]", level))
                || line_upper.contains(&format!("{}: ", level))
                || line_upper.contains(&format!(" {} ", level))
        };

        let filter_upper = filter.to_uppercase();

        match filter_upper.as_str() {
            "TRACE" => true,
            "DEBUG" => !has_level("TRACE"),
            "INFO" => has_level("INFO") || has_level("WARN") || has_level("ERROR"),
            "WARN" => has_level("WARN") || has_level("ERROR"),
            "ERROR" => has_level("ERROR"),
            _ => true,
        }
    }

    // No filter - show all
    assert!(should_show_line("[INFO] Test message", None));
    assert!(should_show_line("[DEBUG] Test message", None));
    assert!(should_show_line("[ERROR] Test message", None));

    // ERROR filter - only errors
    assert!(should_show_line("[ERROR] Something failed", Some("error")));
    assert!(!should_show_line("[WARN] Something warned", Some("error")));
    assert!(!should_show_line("[INFO] Something happened", Some("error")));

    // WARN filter - warn and error
    assert!(should_show_line("[ERROR] Something failed", Some("warn")));
    assert!(should_show_line("[WARN] Something warned", Some("warn")));
    assert!(!should_show_line("[INFO] Something happened", Some("warn")));

    // INFO filter - info, warn, error
    assert!(should_show_line("[ERROR] Something failed", Some("info")));
    assert!(should_show_line("[WARN] Something warned", Some("info")));
    assert!(should_show_line("[INFO] Something happened", Some("info")));
    assert!(!should_show_line("[DEBUG] Debug info", Some("info"))); // "Debug info" should NOT match [INFO]

    // Case insensitive
    assert!(should_show_line("[error] lowercase", Some("ERROR")));
    assert!(should_show_line("[ERROR] uppercase", Some("error")));

    // Should not match substrings in message text
    assert!(!should_show_line("[DEBUG] Information about something", Some("info")));
    assert!(!should_show_line("[DEBUG] Warning message text", Some("warn")));
}

/// Test log file creation for follow mode.
#[test]
fn test_log_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    // Create log file with content
    let mut file = File::create(&log_path).unwrap();
    writeln!(file, "[2024-01-01T10:00:00Z] INFO: Server started").unwrap();
    writeln!(file, "[2024-01-01T10:00:01Z] DEBUG: Processing request").unwrap();
    writeln!(file, "[2024-01-01T10:00:02Z] ERROR: Connection failed").unwrap();
    file.flush().unwrap();

    // Verify file exists and has content
    assert!(log_path.exists());

    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("INFO: Server started"));
    assert!(content.contains("DEBUG: Processing request"));
    assert!(content.contains("ERROR: Connection failed"));
}

/// Test log directory structure.
#[test]
fn test_log_directory_structure() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("logs").join("mcp");

    // Create directory structure
    std::fs::create_dir_all(&logs_dir).unwrap();

    // Create multiple server log files
    for server in &["neo4j", "github", "perplexity"] {
        let log_file = logs_dir.join(format!("{}.log", server));
        let mut file = File::create(&log_file).unwrap();
        writeln!(file, "[INFO] {} server log", server).unwrap();
    }

    // Verify all files exist
    assert!(logs_dir.join("neo4j.log").exists());
    assert!(logs_dir.join("github.log").exists());
    assert!(logs_dir.join("perplexity.log").exists());
}
