//! Foreign MCP detection and tracking.
//!
//! Tracks MCPs that were added directly to editors (Claude Code, Cursor, etc.)
//! without going through spn. These are "foreign" MCPs that the user can adopt
//! into spn or permanently ignore.

use chrono::{DateTime, Utc};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Result, SpnError};

/// Where a foreign MCP was discovered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForeignScope {
    /// Global config (e.g., ~/.cursor/mcp.json)
    Global,
    /// Project-level config
    Project(PathBuf),
}

/// Source client where the foreign MCP was found.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForeignSource {
    /// Claude Code (~/.claude.json or .mcp.json)
    ClaudeCode,
    /// Cursor (~/.cursor/mcp.json or .cursor/mcp.json)
    Cursor,
    /// Windsurf (~/.codeium/windsurf/mcp_config.json)
    Windsurf,
}

impl std::fmt::Display for ForeignSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClaudeCode => write!(f, "Claude Code"),
            Self::Cursor => write!(f, "Cursor"),
            Self::Windsurf => write!(f, "Windsurf"),
        }
    }
}

/// Serializable MCP server configuration for persistence.
///
/// This is a simplified version of `spn_core::McpServer` that can be
/// serialized to YAML for the foreign tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignMcpServer {
    /// Command to run (for stdio transport).
    pub command: Option<String>,
    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default)]
    pub env: FxHashMap<String, String>,
    /// URL for SSE/WebSocket transports.
    pub url: Option<String>,
}

impl ForeignMcpServer {
    /// Create from command and args.
    pub fn stdio(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: Some(command.into()),
            args,
            env: FxHashMap::default(),
            url: None,
        }
    }

    /// Create from URL.
    pub fn sse(url: impl Into<String>) -> Self {
        Self {
            command: None,
            args: Vec::new(),
            env: FxHashMap::default(),
            url: Some(url.into()),
        }
    }
}

/// A foreign MCP detected in a client config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignMcp {
    /// Server name (key in the MCP config).
    pub name: String,
    /// Which client it was found in.
    pub source: ForeignSource,
    /// Whether it's global or project-level.
    pub scope: ForeignScope,
    /// Path to the config file where it was found.
    pub config_path: PathBuf,
    /// When it was first detected.
    pub detected: DateTime<Utc>,
    /// The actual server configuration.
    pub server: ForeignMcpServer,
}

/// Tracks foreign MCPs and user decisions about them.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ForeignTracker {
    /// MCP names the user has chosen to ignore.
    #[serde(default)]
    pub ignored: Vec<String>,
    /// MCPs awaiting user decision (adopt or ignore).
    #[serde(default)]
    pub pending: Vec<ForeignMcp>,
}

impl ForeignTracker {
    /// Create a new empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the path to the foreign tracker file (~/.spn/foreign.yaml).
    fn file_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| SpnError::ConfigError("HOME not set".into()))?;
        Ok(home.join(".spn").join("foreign.yaml"))
    }

    /// Load foreign tracker from ~/.spn/foreign.yaml.
    ///
    /// Returns empty tracker if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::file_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| SpnError::ConfigError(format!("Failed to read foreign.yaml: {e}")))?;

        serde_yaml::from_str(&content)
            .map_err(|e| SpnError::ConfigError(format!("Failed to parse foreign.yaml: {e}")))
    }

    /// Save foreign tracker to ~/.spn/foreign.yaml.
    pub fn save(&self) -> Result<()> {
        let path = Self::file_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SpnError::ConfigError(format!("Failed to create .spn dir: {e}")))?;
        }

        let content = serde_yaml::to_string(self)
            .map_err(|e| SpnError::ConfigError(format!("Failed to serialize foreign.yaml: {e}")))?;

        std::fs::write(&path, content)
            .map_err(|e| SpnError::ConfigError(format!("Failed to write foreign.yaml: {e}")))?;

        Ok(())
    }

    /// Check if an MCP name is in the ignore list.
    #[must_use]
    pub fn is_ignored(&self, name: &str) -> bool {
        self.ignored.iter().any(|n| n == name)
    }

    /// Check if an MCP is already pending.
    #[must_use]
    pub fn is_pending(&self, name: &str) -> bool {
        self.pending.iter().any(|m| m.name == name)
    }

    /// Add a foreign MCP to the pending list.
    ///
    /// Does nothing if already pending or ignored.
    pub fn add_pending(&mut self, mcp: ForeignMcp) {
        if self.is_ignored(&mcp.name) || self.is_pending(&mcp.name) {
            return;
        }
        self.pending.push(mcp);
    }

    /// Mark an MCP name as ignored.
    ///
    /// Also removes from pending if present.
    pub fn ignore(&mut self, name: &str) {
        // Remove from pending
        self.pending.retain(|m| m.name != name);

        // Add to ignored (if not already there)
        if !self.is_ignored(name) {
            self.ignored.push(name.to_string());
        }
    }

    /// Remove an MCP from pending (after user adopts it).
    pub fn remove_pending(&mut self, name: &str) {
        self.pending.retain(|m| m.name != name);
    }

    /// Get all pending foreign MCPs.
    #[must_use]
    pub fn pending(&self) -> &[ForeignMcp] {
        &self.pending
    }

    /// Get count of pending MCPs.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending MCPs.
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_foreign_mcp(name: &str) -> ForeignMcp {
        ForeignMcp {
            name: name.to_string(),
            source: ForeignSource::Cursor,
            scope: ForeignScope::Global,
            config_path: PathBuf::from("/home/user/.cursor/mcp.json"),
            detected: Utc::now(),
            server: ForeignMcpServer::stdio("npx", vec!["-y".to_string(), "@example/mcp".to_string()]),
        }
    }

    #[test]
    fn test_new_empty() {
        let tracker = ForeignTracker::new();
        assert!(tracker.ignored.is_empty());
        assert!(tracker.pending.is_empty());
    }

    #[test]
    fn test_add_pending() {
        let mut tracker = ForeignTracker::new();
        let mcp = make_foreign_mcp("test-mcp");

        tracker.add_pending(mcp);

        assert_eq!(tracker.pending_count(), 1);
        assert!(tracker.is_pending("test-mcp"));
    }

    #[test]
    fn test_add_pending_skips_ignored() {
        let mut tracker = ForeignTracker::new();
        tracker.ignore("test-mcp");

        let mcp = make_foreign_mcp("test-mcp");
        tracker.add_pending(mcp);

        assert_eq!(tracker.pending_count(), 0);
    }

    #[test]
    fn test_add_pending_skips_duplicate() {
        let mut tracker = ForeignTracker::new();
        let mcp1 = make_foreign_mcp("test-mcp");
        let mcp2 = make_foreign_mcp("test-mcp");

        tracker.add_pending(mcp1);
        tracker.add_pending(mcp2);

        assert_eq!(tracker.pending_count(), 1);
    }

    #[test]
    fn test_ignore_removes_from_pending() {
        let mut tracker = ForeignTracker::new();
        let mcp = make_foreign_mcp("test-mcp");

        tracker.add_pending(mcp);
        assert!(tracker.is_pending("test-mcp"));

        tracker.ignore("test-mcp");

        assert!(!tracker.is_pending("test-mcp"));
        assert!(tracker.is_ignored("test-mcp"));
    }

    #[test]
    fn test_remove_pending() {
        let mut tracker = ForeignTracker::new();
        let mcp = make_foreign_mcp("test-mcp");

        tracker.add_pending(mcp);
        tracker.remove_pending("test-mcp");

        assert!(!tracker.is_pending("test-mcp"));
        assert!(!tracker.is_ignored("test-mcp"));
    }

    #[test]
    fn test_is_ignored() {
        let mut tracker = ForeignTracker::new();
        assert!(!tracker.is_ignored("foo"));

        tracker.ignore("foo");
        assert!(tracker.is_ignored("foo"));
    }
}
