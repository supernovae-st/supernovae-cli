//! Tree visualization for hierarchical data.
//!
//! Provides ASCII tree rendering for:
//! - Package dependencies
//! - Directory structures
//! - Diagnostic checks
//!
//! # Example Output
//!
//! ```text
//! @nika/workflow v1.2.3
//! ├── @spn/core v0.1.0
//! │   └── serde v1.0.0
//! └── @spn/keyring v0.1.1
//! ```

use console::style;

use super::design_system::{icon, package, version};

// ============================================================================
// TREE CHARACTERS
// ============================================================================

/// Tree branch for non-last items
pub const BRANCH: &str = "├── ";
/// Tree branch for last item
pub const BRANCH_LAST: &str = "└── ";
/// Vertical continuation line
pub const VERTICAL: &str = "│   ";
/// Empty space (for after last items)
pub const SPACE: &str = "    ";

// ============================================================================
// TREE NODE
// ============================================================================

/// A node in a tree structure.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Display content for this node
    pub content: String,
    /// Child nodes
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Create a new tree node with content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            children: Vec::new(),
        }
    }

    /// Add a child node.
    pub fn add_child(&mut self, child: TreeNode) -> &mut Self {
        self.children.push(child);
        self
    }

    /// Add a child with content (convenience method).
    pub fn child(&mut self, content: impl Into<String>) -> &mut TreeNode {
        self.children.push(TreeNode::new(content));
        self.children.last_mut().unwrap()
    }

    /// Render the tree as a string.
    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str(&self.content);
        output.push('\n');
        self.render_children(&mut output, "");
        output.trim_end().to_string()
    }

    fn render_children(&self, output: &mut String, prefix: &str) {
        let count = self.children.len();
        for (i, child) in self.children.iter().enumerate() {
            let is_last = i == count - 1;
            let branch = if is_last { BRANCH_LAST } else { BRANCH };
            let continuation = if is_last { SPACE } else { VERTICAL };

            output.push_str(prefix);
            output.push_str(branch);
            output.push_str(&child.content);
            output.push('\n');

            if !child.children.is_empty() {
                child.render_children(output, &format!("{}{}", prefix, continuation));
            }
        }
    }
}

// ============================================================================
// SPECIALIZED TREES
// ============================================================================

/// Render a package dependency tree.
///
/// ```text
/// @nika/workflow v1.2.3
/// ├── @spn/core v0.1.0
/// └── @spn/keyring v0.1.1
/// ```
pub fn package_tree(root_name: &str, root_version: &str, deps: &[(&str, &str)]) -> String {
    let root_content = format!("{} {}", package(root_name), version(root_version));
    let mut root = TreeNode::new(root_content);

    for (name, ver) in deps {
        let content = format!("{} {}", package(name), version(ver));
        root.add_child(TreeNode::new(content));
    }

    root.render()
}

/// Render a directory tree.
///
/// ```text
/// ~/.spn/
/// ├── config.toml
/// ├── daemon.sock
/// └── packages/
///     └── @scope/name/
/// ```
pub fn directory_tree(root: &str, entries: &[(&str, bool)]) -> String {
    let mut tree = TreeNode::new(style(root).bold().to_string());

    for (path, is_dir) in entries {
        let content = if *is_dir {
            format!("{}/", style(*path).cyan())
        } else {
            path.to_string()
        };
        tree.add_child(TreeNode::new(content));
    }

    tree.render()
}

/// Render a check/diagnostic tree.
///
/// ```text
/// System Check
/// ├── ✓ Rust 1.85.0
/// ├── ✓ Cargo installed
/// ├── ✓ Git 2.44.0
/// └── ✗ Ollama not running
/// ```
pub fn check_tree(title: &str, checks: &[(bool, &str)]) -> String {
    let mut tree = TreeNode::new(style(title).bold().to_string());

    for (passed, message) in checks {
        let content = if *passed {
            format!("{} {}", style(icon::SUCCESS).green().bold(), message)
        } else {
            format!("{} {}", style(icon::ERROR).red().bold(), message)
        };
        tree.add_child(TreeNode::new(content));
    }

    tree.render()
}

/// Render a status tree with icons.
///
/// ```text
/// Security Audit
/// ├── ✓ anthropic (Keychain)
/// ├── ⚠ openai (.env)
/// └── ✗ github (Missing)
/// ```
pub fn status_tree(title: &str, items: &[(&str, &str, &str)]) -> String {
    let mut tree = TreeNode::new(style(title).bold().to_string());

    for (icon_char, name, detail) in items {
        let content = format!("{} {} {}", icon_char, style(*name).bold(), style(format!("({})", detail)).dim());
        tree.add_child(TreeNode::new(content));
    }

    tree.render()
}

// ============================================================================
// INLINE TREE HELPERS
// ============================================================================

/// Get the appropriate branch character.
pub fn branch(is_last: bool) -> &'static str {
    if is_last {
        BRANCH_LAST
    } else {
        BRANCH
    }
}

/// Get the continuation prefix for a given depth and position.
pub fn continuation(depth: usize, is_last_at_each_level: &[bool]) -> String {
    let mut prefix = String::new();
    for (i, &is_last) in is_last_at_each_level.iter().enumerate() {
        if i < depth {
            prefix.push_str(if is_last { SPACE } else { VERTICAL });
        }
    }
    prefix
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_node_render() {
        let mut root = TreeNode::new("root");
        root.child("child1");
        root.child("child2");

        let output = root.render();
        assert!(output.contains("root"));
        assert!(output.contains("├── child1"));
        assert!(output.contains("└── child2"));
    }

    #[test]
    fn test_nested_tree() {
        let mut root = TreeNode::new("root");
        {
            let child = root.child("child");
            child.child("grandchild");
        }

        let output = root.render();
        assert!(output.contains("└── child"));
        assert!(output.contains("    └── grandchild"));
    }

    #[test]
    fn test_package_tree() {
        let output = package_tree("@test/pkg", "1.0.0", &[("dep1", "0.1.0"), ("dep2", "0.2.0")]);
        assert!(output.contains("@test/pkg"));
        assert!(output.contains("1.0.0"));
        assert!(output.contains("dep1"));
    }

    #[test]
    fn test_check_tree() {
        let checks = vec![(true, "Passed check"), (false, "Failed check")];
        let output = check_tree("Tests", &checks);

        assert!(output.contains("Tests"));
        assert!(output.contains(icon::SUCCESS));
        assert!(output.contains(icon::ERROR));
    }

    #[test]
    fn test_branch_helpers() {
        assert_eq!(branch(false), "├── ");
        assert_eq!(branch(true), "└── ");
    }
}
