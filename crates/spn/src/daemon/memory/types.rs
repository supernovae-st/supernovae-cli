//! Memory types for the persistence layer.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Memory namespace for organizing entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryNamespace {
    /// User preferences and settings.
    Preferences,
    /// Recent commands and their outcomes.
    CommandHistory,
    /// Project-specific context.
    ProjectContext,
    /// LLM conversation summaries.
    ConversationSummary,
    /// Usage patterns and statistics.
    Analytics,
}

impl MemoryNamespace {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryNamespace::Preferences => "preferences",
            MemoryNamespace::CommandHistory => "command_history",
            MemoryNamespace::ProjectContext => "project_context",
            MemoryNamespace::ConversationSummary => "conversation_summary",
            MemoryNamespace::Analytics => "analytics",
        }
    }
}

impl std::fmt::Display for MemoryNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Key for memory entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryKey {
    /// Namespace.
    pub namespace: MemoryNamespace,
    /// Entry identifier within namespace.
    pub id: String,
}

impl MemoryKey {
    /// Create a new memory key.
    pub fn new(namespace: MemoryNamespace, id: impl Into<String>) -> Self {
        Self {
            namespace,
            id: id.into(),
        }
    }

    /// Create a preferences key.
    pub fn preference(id: impl Into<String>) -> Self {
        Self::new(MemoryNamespace::Preferences, id)
    }

    /// Create a command history key.
    pub fn command(id: impl Into<String>) -> Self {
        Self::new(MemoryNamespace::CommandHistory, id)
    }

    /// Create a project context key.
    pub fn project(id: impl Into<String>) -> Self {
        Self::new(MemoryNamespace::ProjectContext, id)
    }

    /// Create a conversation summary key.
    pub fn conversation(id: impl Into<String>) -> Self {
        Self::new(MemoryNamespace::ConversationSummary, id)
    }

    /// Create an analytics key.
    pub fn analytics(id: impl Into<String>) -> Self {
        Self::new(MemoryNamespace::Analytics, id)
    }

    /// Get the storage path for this key.
    pub fn path(&self) -> String {
        format!("{}/{}", self.namespace, self.id)
    }
}

impl std::fmt::Display for MemoryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.id)
    }
}

/// A memory entry with value and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// The key identifying this entry.
    pub key: MemoryKey,
    /// The stored value (JSON).
    pub value: serde_json::Value,
    /// When this entry was created.
    pub created_at: SystemTime,
    /// When this entry was last updated.
    pub updated_at: SystemTime,
    /// When this entry expires (None = never).
    pub expires_at: Option<SystemTime>,
    /// Number of times this entry was accessed.
    pub access_count: u64,
    /// Optional tags for filtering.
    pub tags: Vec<String>,
}

impl MemoryEntry {
    /// Create a new memory entry.
    pub fn new(key: MemoryKey, value: serde_json::Value) -> Self {
        let now = SystemTime::now();
        Self {
            key,
            value,
            created_at: now,
            updated_at: now,
            expires_at: None,
            access_count: 0,
            tags: Vec::new(),
        }
    }

    /// Set expiration time.
    pub fn with_expiration(mut self, expires_at: SystemTime) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set TTL in seconds.
    pub fn with_ttl(mut self, seconds: u64) -> Self {
        self.expires_at = Some(
            SystemTime::now()
                .checked_add(std::time::Duration::from_secs(seconds))
                .unwrap_or(SystemTime::UNIX_EPOCH),
        );
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Check if this entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Record an access.
    pub fn touch(&mut self) {
        self.access_count += 1;
    }

    /// Update the value.
    pub fn update(&mut self, value: serde_json::Value) {
        self.value = value;
        self.updated_at = SystemTime::now();
    }
}

/// Command history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    /// The command that was run.
    pub command: String,
    /// Arguments passed.
    pub args: Vec<String>,
    /// Working directory.
    pub cwd: String,
    /// Exit code (if available).
    pub exit_code: Option<i32>,
    /// When the command was run.
    pub timestamp: SystemTime,
    /// Duration in milliseconds.
    pub duration_ms: Option<u64>,
}

/// Project context entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextEntry {
    /// Project root path.
    pub root: String,
    /// Project name.
    pub name: String,
    /// Detected project type (rust, node, python, etc.).
    pub project_type: Option<String>,
    /// Last accessed.
    pub last_accessed: SystemTime,
    /// Custom metadata.
    pub metadata: serde_json::Value,
}

/// User preference entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceEntry {
    /// Preference key.
    pub key: String,
    /// Preference value.
    pub value: serde_json::Value,
    /// Source (user, default, inferred).
    pub source: PreferenceSource,
}

/// Source of a preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreferenceSource {
    /// Set by user explicitly.
    User,
    /// Default value.
    Default,
    /// Inferred from usage.
    Inferred,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_key_creation() {
        let key = MemoryKey::preference("theme");
        assert_eq!(key.namespace, MemoryNamespace::Preferences);
        assert_eq!(key.id, "theme");
        assert_eq!(key.path(), "preferences/theme");
    }

    #[test]
    fn test_memory_key_display() {
        let key = MemoryKey::command("last");
        assert_eq!(format!("{}", key), "command_history:last");
    }

    #[test]
    fn test_memory_entry_creation() {
        let key = MemoryKey::preference("theme");
        let entry = MemoryEntry::new(key.clone(), serde_json::json!("dark"));

        assert!(!entry.is_expired());
        assert_eq!(entry.access_count, 0);
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_memory_entry_ttl() {
        let key = MemoryKey::preference("temp");
        let entry = MemoryEntry::new(key, serde_json::json!("value"))
            .with_ttl(0); // Expires immediately

        // Should be expired (or very close)
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_memory_entry_touch() {
        let key = MemoryKey::preference("counter");
        let mut entry = MemoryEntry::new(key, serde_json::json!(0));

        assert_eq!(entry.access_count, 0);
        entry.touch();
        assert_eq!(entry.access_count, 1);
        entry.touch();
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_memory_entry_update() {
        let key = MemoryKey::preference("value");
        let mut entry = MemoryEntry::new(key, serde_json::json!("old"));
        let original_updated = entry.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        entry.update(serde_json::json!("new"));

        assert_eq!(entry.value, serde_json::json!("new"));
        assert!(entry.updated_at > original_updated);
    }
}
