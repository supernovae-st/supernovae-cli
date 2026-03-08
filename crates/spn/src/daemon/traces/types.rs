//! Types for reasoning trace capture.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Unique identifier for a trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(Uuid);

impl TraceId {
    /// Create a new random trace ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Kind of reasoning step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceStepKind {
    /// Initial thought or analysis.
    Thinking,
    /// Planning next actions.
    Planning,
    /// Tool invocation.
    ToolCall,
    /// Tool result processing.
    ToolResult,
    /// Decision point.
    Decision,
    /// Conclusion or final answer.
    Conclusion,
    /// Error or recovery.
    Error,
    /// Delegation to another agent.
    Delegation,
}

impl TraceStepKind {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TraceStepKind::Thinking => "thinking",
            TraceStepKind::Planning => "planning",
            TraceStepKind::ToolCall => "tool_call",
            TraceStepKind::ToolResult => "tool_result",
            TraceStepKind::Decision => "decision",
            TraceStepKind::Conclusion => "conclusion",
            TraceStepKind::Error => "error",
            TraceStepKind::Delegation => "delegation",
        }
    }

    /// Get icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            TraceStepKind::Thinking => "💭",
            TraceStepKind::Planning => "📋",
            TraceStepKind::ToolCall => "🔧",
            TraceStepKind::ToolResult => "📥",
            TraceStepKind::Decision => "🔀",
            TraceStepKind::Conclusion => "✅",
            TraceStepKind::Error => "❌",
            TraceStepKind::Delegation => "👥",
        }
    }
}

impl std::fmt::Display for TraceStepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single step in the reasoning trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    /// Step index within trace.
    pub index: usize,
    /// Kind of step.
    pub kind: TraceStepKind,
    /// Step content (thinking, tool name, etc).
    pub content: String,
    /// Optional details (tool args, error message, etc).
    pub details: Option<serde_json::Value>,
    /// Duration of this step.
    pub duration: Option<Duration>,
    /// When this step occurred.
    pub timestamp: SystemTime,
    /// Token count for this step.
    pub tokens: Option<u32>,
}

impl TraceStep {
    /// Create a new trace step.
    pub fn new(kind: TraceStepKind, content: impl Into<String>) -> Self {
        Self {
            index: 0,
            kind,
            content: content.into(),
            details: None,
            duration: None,
            timestamp: SystemTime::now(),
            tokens: None,
        }
    }

    /// Add details.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Add duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Add token count.
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Set the step index.
    pub fn with_index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }
}

/// Metadata about a reasoning trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
    /// Model used.
    pub model: String,
    /// Provider (anthropic, openai, etc).
    pub provider: String,
    /// Task or prompt summary.
    pub task: String,
    /// Project context.
    pub project: Option<String>,
    /// Working directory.
    pub cwd: Option<String>,
    /// Custom tags.
    pub tags: Vec<String>,
}

impl TraceMetadata {
    /// Create new metadata.
    pub fn new(model: impl Into<String>, provider: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            provider: provider.into(),
            task: task.into(),
            project: None,
            cwd: None,
            tags: Vec::new(),
        }
    }

    /// Set project.
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Set working directory.
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// A complete reasoning trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTrace {
    /// Unique trace ID.
    pub id: TraceId,
    /// Trace metadata.
    pub metadata: TraceMetadata,
    /// Reasoning steps.
    pub steps: Vec<TraceStep>,
    /// When trace started.
    pub started_at: SystemTime,
    /// When trace ended.
    pub ended_at: Option<SystemTime>,
    /// Total duration.
    pub total_duration: Option<Duration>,
    /// Total tokens used.
    pub total_tokens: Option<u32>,
    /// Whether trace completed successfully.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ReasoningTrace {
    /// Create a new reasoning trace.
    pub fn new(metadata: TraceMetadata) -> Self {
        Self {
            id: TraceId::new(),
            metadata,
            steps: Vec::new(),
            started_at: SystemTime::now(),
            ended_at: None,
            total_duration: None,
            total_tokens: None,
            success: false,
            error: None,
        }
    }

    /// Add a step to the trace.
    pub fn add_step(&mut self, mut step: TraceStep) {
        step.index = self.steps.len();
        self.steps.push(step);
    }

    /// Mark trace as completed successfully.
    pub fn complete(&mut self) {
        self.ended_at = Some(SystemTime::now());
        self.success = true;
        if let Ok(duration) = self.ended_at.unwrap().duration_since(self.started_at) {
            self.total_duration = Some(duration);
        }
        self.calculate_total_tokens();
    }

    /// Mark trace as failed.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.ended_at = Some(SystemTime::now());
        self.success = false;
        self.error = Some(error.into());
        if let Ok(duration) = self.ended_at.unwrap().duration_since(self.started_at) {
            self.total_duration = Some(duration);
        }
        self.calculate_total_tokens();
    }

    /// Calculate total tokens from steps.
    fn calculate_total_tokens(&mut self) {
        let total: u32 = self.steps.iter().filter_map(|s| s.tokens).sum();
        if total > 0 {
            self.total_tokens = Some(total);
        }
    }

    /// Get step count by kind.
    pub fn count_by_kind(&self, kind: TraceStepKind) -> usize {
        self.steps.iter().filter(|s| s.kind == kind).count()
    }

    /// Get all tool calls.
    pub fn tool_calls(&self) -> Vec<&TraceStep> {
        self.steps
            .iter()
            .filter(|s| s.kind == TraceStepKind::ToolCall)
            .collect()
    }

    /// Get all errors.
    pub fn errors(&self) -> Vec<&TraceStep> {
        self.steps
            .iter()
            .filter(|s| s.kind == TraceStepKind::Error)
            .collect()
    }

    /// Check if trace has errors.
    pub fn has_errors(&self) -> bool {
        self.steps.iter().any(|s| s.kind == TraceStepKind::Error)
    }

    /// Get duration of the trace.
    pub fn duration(&self) -> Option<Duration> {
        self.total_duration.or_else(|| {
            if let Some(ended) = self.ended_at {
                ended.duration_since(self.started_at).ok()
            } else {
                SystemTime::now().duration_since(self.started_at).ok()
            }
        })
    }
}

/// Trace summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    /// Trace ID.
    pub id: TraceId,
    /// Task description.
    pub task: String,
    /// Model used.
    pub model: String,
    /// Number of steps.
    pub step_count: usize,
    /// When started.
    pub started_at: SystemTime,
    /// Duration in seconds.
    pub duration_secs: Option<f64>,
    /// Total tokens.
    pub total_tokens: Option<u32>,
    /// Success status.
    pub success: bool,
    /// Tags.
    pub tags: Vec<String>,
}

impl From<&ReasoningTrace> for TraceSummary {
    fn from(trace: &ReasoningTrace) -> Self {
        Self {
            id: trace.id,
            task: trace.metadata.task.clone(),
            model: trace.metadata.model.clone(),
            step_count: trace.steps.len(),
            started_at: trace.started_at,
            duration_secs: trace.duration().map(|d| d.as_secs_f64()),
            total_tokens: trace.total_tokens,
            success: trace.success,
            tags: trace.metadata.tags.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id() {
        let id1 = TraceId::new();
        let id2 = TraceId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_trace_step_kinds() {
        assert_eq!(TraceStepKind::Thinking.as_str(), "thinking");
        assert_eq!(TraceStepKind::ToolCall.icon(), "🔧");
    }

    #[test]
    fn test_reasoning_trace_creation() {
        let metadata = TraceMetadata::new("claude-3-opus", "anthropic", "Implement feature X");
        let mut trace = ReasoningTrace::new(metadata);

        trace.add_step(TraceStep::new(TraceStepKind::Thinking, "Analyzing the request"));
        trace.add_step(TraceStep::new(TraceStepKind::Planning, "Will create 3 files"));
        trace.add_step(
            TraceStep::new(TraceStepKind::ToolCall, "Write")
                .with_details(serde_json::json!({"file": "src/lib.rs"})),
        );

        assert_eq!(trace.steps.len(), 3);
        assert_eq!(trace.steps[0].index, 0);
        assert_eq!(trace.steps[2].index, 2);
    }

    #[test]
    fn test_trace_completion() {
        let metadata = TraceMetadata::new("gpt-4", "openai", "Debug issue");
        let mut trace = ReasoningTrace::new(metadata);

        trace.add_step(TraceStep::new(TraceStepKind::Thinking, "Looking at logs"));
        trace.complete();

        assert!(trace.success);
        assert!(trace.ended_at.is_some());
        assert!(trace.total_duration.is_some());
    }

    #[test]
    fn test_trace_failure() {
        let metadata = TraceMetadata::new("claude-3", "anthropic", "Build project");
        let mut trace = ReasoningTrace::new(metadata);

        trace.add_step(TraceStep::new(TraceStepKind::Error, "Compilation failed"));
        trace.fail("Build error: missing dependency");

        assert!(!trace.success);
        assert_eq!(trace.error, Some("Build error: missing dependency".into()));
        assert!(trace.has_errors());
    }

    #[test]
    fn test_trace_summary() {
        let metadata = TraceMetadata::new("claude-3", "anthropic", "Test task")
            .with_tags(vec!["test".into()]);
        let mut trace = ReasoningTrace::new(metadata);
        trace.add_step(TraceStep::new(TraceStepKind::Thinking, "step 1").with_tokens(100));
        trace.add_step(TraceStep::new(TraceStepKind::Conclusion, "done").with_tokens(50));
        trace.complete();

        let summary = TraceSummary::from(&trace);
        assert_eq!(summary.step_count, 2);
        assert_eq!(summary.total_tokens, Some(150));
        assert!(summary.success);
        assert_eq!(summary.tags, vec!["test".to_string()]);
    }
}
