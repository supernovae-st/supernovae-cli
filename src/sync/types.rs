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

/// Integration configuration for package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Whether this package requires filesystem sync to editors.
    #[serde(default)]
    pub requires_sync: bool,

    /// List of editors this package supports.
    #[serde(default)]
    pub editors: Vec<String>,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            requires_sync: false,
            editors: vec![],
        }
    }
}

/// Package type derived from name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageType {
    /// @skills/ - Requires sync
    Skills,
    /// @workflows/ - No sync needed (standalone execution)
    Workflows,
    /// @agents/ - No sync needed (CLI subagents)
    Agents,
    /// @prompts/ - No sync needed
    Prompts,
    /// @jobs/ - No sync needed
    Jobs,
    /// @schemas/ - No sync needed
    Schemas,
    /// Unknown package type
    Unknown,
}

impl PackageType {
    /// Parse package type from name.
    pub fn from_name(name: &str) -> Self {
        if name.starts_with("@skills/") {
            Self::Skills
        } else if name.starts_with("@workflows/") {
            Self::Workflows
        } else if name.starts_with("@agents/") {
            Self::Agents
        } else if name.starts_with("@prompts/") {
            Self::Prompts
        } else if name.starts_with("@jobs/") {
            Self::Jobs
        } else if name.starts_with("@schemas/") {
            Self::Schemas
        } else {
            Self::Unknown
        }
    }

    /// Get default requires_sync value for this package type.
    pub fn default_requires_sync(&self) -> bool {
        match self {
            Self::Skills => true,  // Skills need .claude/skills/
            Self::Workflows => false,  // Standalone nika execution
            Self::Agents => false,  // nika CLI subagents
            Self::Prompts => false,  // No editor integration
            Self::Jobs => false,  // No editor integration
            Self::Schemas => false,  // No editor integration
            Self::Unknown => false,  // Conservative default
        }
    }
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

    /// Integration configuration.
    #[serde(default)]
    pub integration: IntegrationConfig,
}

impl PackageManifest {
    /// Check if this package has any IDE integrations.
    pub fn has_integrations(&self) -> bool {
        self.mcp.is_some() || !self.skills.is_empty() || !self.hooks.is_empty()
    }

    /// Check if this package requires sync to editors.
    ///
    /// Uses explicit integration config if present, otherwise falls back to
    /// package type default.
    pub fn requires_sync(&self) -> bool {
        // If integration config explicitly sets requires_sync, use that
        if self.integration.requires_sync {
            return true;
        }

        // Otherwise, check package type default
        let package_type = PackageType::from_name(&self.name);
        package_type.default_requires_sync()
    }

    /// Get package type from name.
    pub fn package_type(&self) -> PackageType {
        PackageType::from_name(&self.name)
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

    #[test]
    fn test_package_type_from_name() {
        assert_eq!(
            PackageType::from_name("@skills/brainstorming"),
            PackageType::Skills
        );
        assert_eq!(
            PackageType::from_name("@workflows/generate-page"),
            PackageType::Workflows
        );
        assert_eq!(
            PackageType::from_name("@agents/code-reviewer"),
            PackageType::Agents
        );
        assert_eq!(
            PackageType::from_name("@prompts/system"),
            PackageType::Prompts
        );
        assert_eq!(PackageType::from_name("@jobs/daily"), PackageType::Jobs);
        assert_eq!(
            PackageType::from_name("@schemas/api"),
            PackageType::Schemas
        );
        assert_eq!(
            PackageType::from_name("unknown-package"),
            PackageType::Unknown
        );
    }

    #[test]
    fn test_package_type_default_requires_sync() {
        assert!(PackageType::Skills.default_requires_sync());
        assert!(!PackageType::Workflows.default_requires_sync());
        assert!(!PackageType::Agents.default_requires_sync());
        assert!(!PackageType::Prompts.default_requires_sync());
        assert!(!PackageType::Jobs.default_requires_sync());
        assert!(!PackageType::Schemas.default_requires_sync());
        assert!(!PackageType::Unknown.default_requires_sync());
    }

    #[test]
    fn test_package_manifest_requires_sync() {
        // Skills package should sync by default
        let skills = PackageManifest {
            name: "@skills/brainstorming".to_string(),
            ..Default::default()
        };
        assert!(skills.requires_sync());

        // Workflows package should NOT sync by default
        let workflow = PackageManifest {
            name: "@workflows/generate-page".to_string(),
            ..Default::default()
        };
        assert!(!workflow.requires_sync());

        // Explicit integration config overrides default
        let workflow_with_sync = PackageManifest {
            name: "@workflows/with-sync".to_string(),
            integration: IntegrationConfig {
                requires_sync: true,
                editors: vec!["claude-code".to_string()],
            },
            ..Default::default()
        };
        assert!(workflow_with_sync.requires_sync());
    }
}
