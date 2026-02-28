//! Types for IDE synchronization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Supported IDE targets for synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdeTarget {
    /// Claude Code (.claude/)
    ClaudeCode,
    /// Cursor (.cursor/)
    Cursor,
    /// VS Code (.vscode/)
    VsCode,
    /// Windsurf (.windsurf/)
    Windsurf,
}

impl IdeTarget {
    /// Get all supported IDE targets.
    pub fn all() -> Vec<Self> {
        vec![Self::ClaudeCode, Self::Cursor, Self::VsCode, Self::Windsurf]
    }

    /// Get the config directory name for this IDE.
    pub fn config_dir(&self) -> &'static str {
        match self {
            Self::ClaudeCode => ".claude",
            Self::Cursor => ".cursor",
            Self::VsCode => ".vscode",
            Self::Windsurf => ".windsurf",
        }
    }

    /// Get the display name for this IDE.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Cursor => "Cursor",
            Self::VsCode => "VS Code",
            Self::Windsurf => "Windsurf",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude" | "cc" => Some(Self::ClaudeCode),
            "cursor" => Some(Self::Cursor),
            "vscode" | "vs-code" | "code" => Some(Self::VsCode),
            "windsurf" => Some(Self::Windsurf),
            _ => None,
        }
    }
}

impl std::fmt::Display for IdeTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// MCP server configuration from package manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Command to run the MCP server.
    pub command: String,

    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables for the MCP server.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Package manifest (spn.json) with IDE integration info.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Package name.
    #[serde(default)]
    pub name: String,

    /// Package version.
    #[serde(default)]
    pub version: String,

    /// MCP server configuration (if package provides one).
    #[serde(default)]
    pub mcp: Option<McpConfig>,

    /// Skills directories to sync.
    #[serde(default)]
    pub skills: Vec<String>,

    /// Hooks directories to sync.
    #[serde(default)]
    pub hooks: Vec<String>,

    /// Commands to sync.
    #[serde(default)]
    pub commands: Vec<String>,
}

impl PackageManifest {
    /// Check if this package has any IDE integrations.
    pub fn has_integrations(&self) -> bool {
        self.mcp.is_some() || !self.skills.is_empty() || !self.hooks.is_empty()
    }

    /// Load from a spn.json file.
    pub fn from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }
}

/// Result of a sync operation for a single package.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Package name.
    pub package: String,

    /// Target IDE.
    pub target: IdeTarget,

    /// Whether the sync was successful.
    pub success: bool,

    /// What was synced.
    pub synced: Vec<SyncedItem>,

    /// Error message if failed.
    pub error: Option<String>,
}

/// Item that was synced.
#[derive(Debug, Clone)]
pub enum SyncedItem {
    /// MCP server added to config.
    McpServer(String),
    /// Skills directory linked.
    Skills(PathBuf),
    /// Hooks directory linked.
    Hooks(PathBuf),
    /// Command linked.
    Command(String),
}

impl std::fmt::Display for SyncedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::McpServer(name) => write!(f, "MCP: {}", name),
            Self::Skills(path) => write!(f, "Skills: {}", path.display()),
            Self::Hooks(path) => write!(f, "Hooks: {}", path.display()),
            Self::Command(name) => write!(f, "Command: {}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ide_target_from_str() {
        assert_eq!(
            IdeTarget::from_str("claude-code"),
            Some(IdeTarget::ClaudeCode)
        );
        assert_eq!(IdeTarget::from_str("claude"), Some(IdeTarget::ClaudeCode));
        assert_eq!(IdeTarget::from_str("cc"), Some(IdeTarget::ClaudeCode));
        assert_eq!(IdeTarget::from_str("cursor"), Some(IdeTarget::Cursor));
        assert_eq!(IdeTarget::from_str("vscode"), Some(IdeTarget::VsCode));
        assert_eq!(IdeTarget::from_str("vs-code"), Some(IdeTarget::VsCode));
        assert_eq!(IdeTarget::from_str("windsurf"), Some(IdeTarget::Windsurf));
        assert_eq!(IdeTarget::from_str("unknown"), None);
    }

    #[test]
    fn test_ide_target_config_dir() {
        assert_eq!(IdeTarget::ClaudeCode.config_dir(), ".claude");
        assert_eq!(IdeTarget::Cursor.config_dir(), ".cursor");
        assert_eq!(IdeTarget::VsCode.config_dir(), ".vscode");
        assert_eq!(IdeTarget::Windsurf.config_dir(), ".windsurf");
    }

    #[test]
    fn test_package_manifest_has_integrations() {
        let empty = PackageManifest::default();
        assert!(!empty.has_integrations());

        let with_mcp = PackageManifest {
            mcp: Some(McpConfig {
                command: "node".to_string(),
                args: vec!["dist/mcp.js".to_string()],
                env: HashMap::new(),
            }),
            ..Default::default()
        };
        assert!(with_mcp.has_integrations());

        let with_skills = PackageManifest {
            skills: vec!["skills/".to_string()],
            ..Default::default()
        };
        assert!(with_skills.has_integrations());
    }
}
