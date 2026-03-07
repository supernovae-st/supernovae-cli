//! Command suggestion system for typo correction.
//!
//! Uses Levenshtein distance to find similar commands when the user
//! makes a typo, providing helpful "did you mean?" suggestions.

use crate::ux::design_system as ds;

/// All available top-level commands.
const COMMANDS: &[&str] = &[
    "add", "remove", "install", "update", "outdated", "search", "info", "list", "publish",
    "version", "init", "mcp", "skill", "model", "provider", "nk", "nv", "schema", "config", "sync",
    "secrets", "daemon", "doctor", "status", "topic", "setup", "help",
];

/// Calculate Levenshtein distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *cell = j;
    }

    for (i, ca) in a.chars().enumerate() {
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Find the closest matching command for a given input.
///
/// Returns `Some(suggestion)` if a close match is found (distance <= 3),
/// otherwise returns `None`.
pub fn suggest_command(input: &str) -> Option<&'static str> {
    let input_lower = input.to_lowercase();

    // First, check for prefix matches (e.g., "mod" -> "model")
    let prefix_matches: Vec<_> = COMMANDS
        .iter()
        .filter(|cmd| cmd.starts_with(&input_lower))
        .copied()
        .collect();

    if prefix_matches.len() == 1 {
        return Some(prefix_matches[0]);
    }

    // Find closest by Levenshtein distance
    let mut best_match: Option<&str> = None;
    let mut best_distance = usize::MAX;

    for cmd in COMMANDS {
        let distance = levenshtein(&input_lower, cmd);
        if distance < best_distance {
            best_distance = distance;
            best_match = Some(cmd);
        }
    }

    // Only suggest if distance is reasonable (max 3 edits for short commands)
    let threshold = if input.len() <= 3 { 2 } else { 3 };
    if best_distance <= threshold {
        best_match
    } else {
        None
    }
}

/// Print a "did you mean?" suggestion for an unrecognized command.
pub fn print_suggestion(input: &str) {
    if let Some(suggestion) = suggest_command(input) {
        eprintln!();
        eprintln!(
            "  {} '{}'",
            ds::warning("Did you mean?").bold(),
            ds::primary(format!("spn {}", suggestion))
        );
        eprintln!();
    }
}

/// Get all available commands (for help text).
#[allow(dead_code)]
pub fn available_commands() -> &'static [&'static str] {
    COMMANDS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(levenshtein("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_one_edit() {
        assert_eq!(levenshtein("hello", "hallo"), 1);
        assert_eq!(levenshtein("hello", "hell"), 1);
        assert_eq!(levenshtein("hello", "helloo"), 1);
    }

    #[test]
    fn test_levenshtein_multiple_edits() {
        assert_eq!(levenshtein("hello", "hola"), 3);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn test_suggest_typo_model() {
        assert_eq!(suggest_command("modle"), Some("model"));
        assert_eq!(suggest_command("mdoel"), Some("model"));
    }

    #[test]
    fn test_suggest_typo_install() {
        assert_eq!(suggest_command("instal"), Some("install"));
        assert_eq!(suggest_command("isntall"), Some("install"));
    }

    #[test]
    fn test_suggest_typo_config() {
        assert_eq!(suggest_command("confg"), Some("config"));
        assert_eq!(suggest_command("conifg"), Some("config"));
    }

    #[test]
    fn test_suggest_typo_provider() {
        assert_eq!(suggest_command("provder"), Some("provider"));
        assert_eq!(suggest_command("provier"), Some("provider"));
    }

    #[test]
    fn test_suggest_prefix() {
        // Single prefix match should return the command
        assert_eq!(suggest_command("doc"), Some("doctor"));
        assert_eq!(suggest_command("mod"), Some("model"));
    }

    #[test]
    fn test_suggest_no_match() {
        // Completely unrelated input
        assert_eq!(suggest_command("xyzabc"), None);
        assert_eq!(suggest_command("foobar"), None);
    }

    #[test]
    fn test_suggest_exact_match() {
        // Exact matches should return themselves
        assert_eq!(suggest_command("model"), Some("model"));
        assert_eq!(suggest_command("config"), Some("config"));
    }

    #[test]
    fn test_available_commands() {
        let cmds = available_commands();
        assert!(cmds.contains(&"model"));
        assert!(cmds.contains(&"config"));
        assert!(cmds.contains(&"nk"));
        assert!(cmds.contains(&"nv"));
    }
}
