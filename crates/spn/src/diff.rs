//! Diff utilities for displaying file changes.
//!
//! Provides interactive diff previews with colored output.

use colored::Colorize;
use dialoguer::Confirm;
use similar::{ChangeTag, TextDiff};
use std::path::Path;

/// Display a unified diff between old and new content.
pub fn display_diff(path: &Path, old_content: &str, new_content: &str) {
    let diff = TextDiff::from_lines(old_content, new_content);

    println!("{} {}", "📄".cyan(), path.display().to_string().bold());
    println!();

    let mut additions = 0;
    let mut deletions = 0;

    for change in diff.iter_all_changes() {
        let line = change.to_string_lossy();
        match change.tag() {
            ChangeTag::Delete => {
                print!("{}", format!("- {}", line).red());
                deletions += 1;
            }
            ChangeTag::Insert => {
                print!("{}", format!("+ {}", line).green());
                additions += 1;
            }
            ChangeTag::Equal => {
                print!("  {}", line);
            }
        }
    }

    println!();
    println!(
        "  {} +{} additions, {} deletions",
        "📊".dimmed(),
        additions.to_string().green(),
        deletions.to_string().red()
    );
    println!();
}

/// Display diff and ask for confirmation.
pub fn confirm_changes(path: &Path, old_content: &str, new_content: &str) -> bool {
    display_diff(path, old_content, new_content);

    Confirm::new()
        .with_prompt("Apply these changes?")
        .default(true)
        .interact()
        .unwrap_or(false)
}

/// Display multiple file diffs and ask for batch confirmation.
pub struct DiffBatch {
    diffs: Vec<FileDiff>,
}

pub struct FileDiff {
    pub path: String,
    pub old_content: String,
    pub new_content: String,
}

impl DiffBatch {
    pub fn new() -> Self {
        Self { diffs: Vec::new() }
    }

    pub fn add(&mut self, path: String, old_content: String, new_content: String) {
        self.diffs.push(FileDiff {
            path,
            old_content,
            new_content,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.diffs.is_empty()
    }

    pub fn display(&self) {
        println!("{} Files to be changed:", "📋".cyan().bold());
        println!();

        for diff in &self.diffs {
            let path = Path::new(&diff.path);
            display_diff(path, &diff.old_content, &diff.new_content);
        }
    }

    pub fn confirm(&self) -> bool {
        if self.is_empty() {
            return true;
        }

        self.display();

        println!(
            "{} {} file(s) will be modified",
            "⚠️".yellow(),
            self.diffs.len()
        );

        Confirm::new()
            .with_prompt("Apply all changes?")
            .default(true)
            .interact()
            .unwrap_or(false)
    }
}

impl Default for DiffBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_batch() {
        let mut batch = DiffBatch::new();
        assert!(batch.is_empty());

        batch.add(
            "test.txt".to_string(),
            "old content\n".to_string(),
            "new content\n".to_string(),
        );

        assert!(!batch.is_empty());
        assert_eq!(batch.diffs.len(), 1);
    }
}
