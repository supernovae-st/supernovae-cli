//! Proactive suggestion types.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Unique identifier for a suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SuggestionId(Uuid);

impl SuggestionId {
    /// Create a new random suggestion ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for SuggestionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SuggestionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category of proactive suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuggestionCategory {
    /// Security-related suggestion.
    Security,
    /// Performance optimization.
    Performance,
    /// Code quality improvement.
    Quality,
    /// Testing suggestion.
    Testing,
    /// Documentation suggestion.
    Documentation,
    /// Workflow optimization.
    Workflow,
    /// Resource management.
    Resources,
    /// Learning/improvement.
    Learning,
}

impl SuggestionCategory {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SuggestionCategory::Security => "security",
            SuggestionCategory::Performance => "performance",
            SuggestionCategory::Quality => "quality",
            SuggestionCategory::Testing => "testing",
            SuggestionCategory::Documentation => "documentation",
            SuggestionCategory::Workflow => "workflow",
            SuggestionCategory::Resources => "resources",
            SuggestionCategory::Learning => "learning",
        }
    }

    /// Get icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            SuggestionCategory::Security => "🔐",
            SuggestionCategory::Performance => "⚡",
            SuggestionCategory::Quality => "✨",
            SuggestionCategory::Testing => "🧪",
            SuggestionCategory::Documentation => "📝",
            SuggestionCategory::Workflow => "🔄",
            SuggestionCategory::Resources => "💾",
            SuggestionCategory::Learning => "📚",
        }
    }
}

impl std::fmt::Display for SuggestionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Priority level for suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SuggestionPriority {
    /// Urgent action needed.
    Critical = 1,
    /// High priority, act soon.
    High = 2,
    /// Medium priority, when convenient.
    Medium = 3,
    /// Low priority, nice to have.
    Low = 4,
    /// Informational only.
    Info = 5,
}

impl SuggestionPriority {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SuggestionPriority::Critical => "critical",
            SuggestionPriority::High => "high",
            SuggestionPriority::Medium => "medium",
            SuggestionPriority::Low => "low",
            SuggestionPriority::Info => "info",
        }
    }

    /// Get indicator icon.
    pub fn indicator(&self) -> &'static str {
        match self {
            SuggestionPriority::Critical => "🔴",
            SuggestionPriority::High => "🟠",
            SuggestionPriority::Medium => "🟡",
            SuggestionPriority::Low => "🟢",
            SuggestionPriority::Info => "🔵",
        }
    }
}

impl std::fmt::Display for SuggestionPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Source that triggered the suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionSource {
    /// Based on file changes.
    FileChange { path: String },
    /// Based on command history.
    CommandHistory { command: String },
    /// Based on error patterns.
    ErrorPattern { pattern: String },
    /// Based on time/schedule.
    Scheduled { trigger: String },
    /// Based on project analysis.
    ProjectAnalysis { aspect: String },
    /// Based on memory/preferences.
    MemoryBased { key: String },
    /// Based on external event.
    External { source: String },
    /// Manual/explicit trigger.
    Manual,
}

impl SuggestionSource {
    /// Get a brief description.
    pub fn description(&self) -> String {
        match self {
            SuggestionSource::FileChange { path } => format!("file changed: {}", path),
            SuggestionSource::CommandHistory { command } => format!("after command: {}", command),
            SuggestionSource::ErrorPattern { pattern } => format!("error pattern: {}", pattern),
            SuggestionSource::Scheduled { trigger } => format!("scheduled: {}", trigger),
            SuggestionSource::ProjectAnalysis { aspect } => format!("project analysis: {}", aspect),
            SuggestionSource::MemoryBased { key } => format!("based on: {}", key),
            SuggestionSource::External { source } => format!("external: {}", source),
            SuggestionSource::Manual => "manual".to_string(),
        }
    }
}

/// A proactive suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveSuggestion {
    /// Unique ID.
    pub id: SuggestionId,
    /// Category.
    pub category: SuggestionCategory,
    /// Priority.
    pub priority: SuggestionPriority,
    /// Short title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Source that triggered this.
    pub source: SuggestionSource,
    /// Suggested action/command.
    pub action: Option<SuggestionAction>,
    /// When this was generated.
    pub created_at: SystemTime,
    /// When this expires (if any).
    pub expires_at: Option<SystemTime>,
    /// Whether user has seen this.
    pub seen: bool,
    /// Whether user has acted on this.
    pub acted: bool,
    /// Whether user dismissed this.
    pub dismissed: bool,
    /// Confidence score (0.0-1.0).
    pub confidence: f32,
    /// Related context.
    pub context: Option<serde_json::Value>,
    /// Tags for filtering.
    pub tags: Vec<String>,
}

impl ProactiveSuggestion {
    /// Create a new suggestion.
    pub fn new(
        category: SuggestionCategory,
        priority: SuggestionPriority,
        title: impl Into<String>,
        description: impl Into<String>,
        source: SuggestionSource,
    ) -> Self {
        Self {
            id: SuggestionId::new(),
            category,
            priority,
            title: title.into(),
            description: description.into(),
            source,
            action: None,
            created_at: SystemTime::now(),
            expires_at: None,
            seen: false,
            acted: false,
            dismissed: false,
            confidence: 1.0,
            context: None,
            tags: Vec::new(),
        }
    }

    /// Set the action.
    pub fn with_action(mut self, action: SuggestionAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Set expiration.
    pub fn with_expiration(mut self, expires_at: SystemTime) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set context.
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Mark as seen.
    pub fn mark_seen(&mut self) {
        self.seen = true;
    }

    /// Mark as acted upon.
    pub fn mark_acted(&mut self) {
        self.acted = true;
        self.seen = true;
    }

    /// Dismiss the suggestion.
    pub fn dismiss(&mut self) {
        self.dismissed = true;
        self.seen = true;
    }

    /// Check if expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Check if still active (not expired, not dismissed, not acted).
    pub fn is_active(&self) -> bool {
        !self.is_expired() && !self.dismissed && !self.acted
    }

    /// Get display summary.
    pub fn summary(&self) -> String {
        format!(
            "{} {} [{}] {}",
            self.priority.indicator(),
            self.category.icon(),
            self.category,
            self.title
        )
    }
}

/// Action to take for a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionAction {
    /// Run a CLI command.
    RunCommand { command: String, args: Vec<String> },
    /// Open a file.
    OpenFile { path: String, line: Option<u32> },
    /// Run a workflow.
    RunWorkflow { workflow: String },
    /// Spawn an agent.
    SpawnAgent { role: String, task: String },
    /// Show information.
    ShowInfo { content: String },
    /// Navigate to URL.
    OpenUrl { url: String },
    /// Custom action.
    Custom { action_type: String, data: serde_json::Value },
}

impl SuggestionAction {
    /// Create run command action.
    pub fn run_command(command: impl Into<String>) -> Self {
        Self::RunCommand {
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// Create run command with args.
    pub fn run_command_with_args(command: impl Into<String>, args: Vec<String>) -> Self {
        Self::RunCommand {
            command: command.into(),
            args,
        }
    }

    /// Create open file action.
    pub fn open_file(path: impl Into<String>) -> Self {
        Self::OpenFile {
            path: path.into(),
            line: None,
        }
    }

    /// Create open file at line action.
    pub fn open_file_at(path: impl Into<String>, line: u32) -> Self {
        Self::OpenFile {
            path: path.into(),
            line: Some(line),
        }
    }

    /// Create spawn agent action.
    pub fn spawn_agent(role: impl Into<String>, task: impl Into<String>) -> Self {
        Self::SpawnAgent {
            role: role.into(),
            task: task.into(),
        }
    }

    /// Get a brief description.
    pub fn description(&self) -> String {
        match self {
            SuggestionAction::RunCommand { command, args } => {
                if args.is_empty() {
                    format!("run: {}", command)
                } else {
                    format!("run: {} {}", command, args.join(" "))
                }
            }
            SuggestionAction::OpenFile { path, line } => {
                if let Some(line) = line {
                    format!("open: {}:{}", path, line)
                } else {
                    format!("open: {}", path)
                }
            }
            SuggestionAction::RunWorkflow { workflow } => format!("workflow: {}", workflow),
            SuggestionAction::SpawnAgent { role, .. } => format!("agent: {}", role),
            SuggestionAction::ShowInfo { .. } => "show info".to_string(),
            SuggestionAction::OpenUrl { url } => format!("open: {}", url),
            SuggestionAction::Custom { action_type, .. } => format!("custom: {}", action_type),
        }
    }
}

/// Summary of a suggestion for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionSummary {
    /// ID.
    pub id: SuggestionId,
    /// Category.
    pub category: SuggestionCategory,
    /// Priority.
    pub priority: SuggestionPriority,
    /// Title.
    pub title: String,
    /// Created timestamp.
    pub created_at: SystemTime,
    /// Whether seen.
    pub seen: bool,
    /// Confidence.
    pub confidence: f32,
}

impl From<&ProactiveSuggestion> for SuggestionSummary {
    fn from(s: &ProactiveSuggestion) -> Self {
        Self {
            id: s.id,
            category: s.category,
            priority: s.priority,
            title: s.title.clone(),
            created_at: s.created_at,
            seen: s.seen,
            confidence: s.confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_id() {
        let id1 = SuggestionId::new();
        let id2 = SuggestionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_suggestion_categories() {
        assert_eq!(SuggestionCategory::Security.as_str(), "security");
        assert_eq!(SuggestionCategory::Performance.icon(), "⚡");
    }

    #[test]
    fn test_suggestion_priority_ordering() {
        assert!(SuggestionPriority::Critical < SuggestionPriority::High);
        assert!(SuggestionPriority::High < SuggestionPriority::Medium);
        assert!(SuggestionPriority::Medium < SuggestionPriority::Low);
    }

    #[test]
    fn test_proactive_suggestion_creation() {
        let suggestion = ProactiveSuggestion::new(
            SuggestionCategory::Security,
            SuggestionPriority::High,
            "Update dependencies",
            "Some dependencies have security updates available",
            SuggestionSource::ProjectAnalysis {
                aspect: "dependencies".into(),
            },
        )
        .with_confidence(0.85)
        .with_action(SuggestionAction::run_command("cargo update"));

        assert_eq!(suggestion.category, SuggestionCategory::Security);
        assert_eq!(suggestion.priority, SuggestionPriority::High);
        assert_eq!(suggestion.confidence, 0.85);
        assert!(suggestion.action.is_some());
        assert!(suggestion.is_active());
    }

    #[test]
    fn test_suggestion_lifecycle() {
        let mut suggestion = ProactiveSuggestion::new(
            SuggestionCategory::Testing,
            SuggestionPriority::Medium,
            "Run tests",
            "Tests haven't been run recently",
            SuggestionSource::Scheduled {
                trigger: "daily".into(),
            },
        );

        assert!(!suggestion.seen);
        suggestion.mark_seen();
        assert!(suggestion.seen);
        assert!(suggestion.is_active());

        suggestion.mark_acted();
        assert!(suggestion.acted);
        assert!(!suggestion.is_active());
    }

    #[test]
    fn test_suggestion_dismissal() {
        let mut suggestion = ProactiveSuggestion::new(
            SuggestionCategory::Quality,
            SuggestionPriority::Low,
            "Format code",
            "Code could be formatted",
            SuggestionSource::Manual,
        );

        assert!(suggestion.is_active());
        suggestion.dismiss();
        assert!(suggestion.dismissed);
        assert!(!suggestion.is_active());
    }

    #[test]
    fn test_suggestion_action_descriptions() {
        let action1 = SuggestionAction::run_command("cargo test");
        assert!(action1.description().contains("cargo test"));

        let action2 = SuggestionAction::open_file_at("src/main.rs", 42);
        assert!(action2.description().contains("src/main.rs:42"));

        let action3 = SuggestionAction::spawn_agent("reviewer", "Review changes");
        assert!(action3.description().contains("reviewer"));
    }

    #[test]
    fn test_suggestion_summary() {
        let suggestion = ProactiveSuggestion::new(
            SuggestionCategory::Documentation,
            SuggestionPriority::Info,
            "Add docs",
            "Some functions lack documentation",
            SuggestionSource::Manual,
        );

        let summary = SuggestionSummary::from(&suggestion);
        assert_eq!(summary.title, "Add docs");
        assert_eq!(summary.category, SuggestionCategory::Documentation);
    }
}
