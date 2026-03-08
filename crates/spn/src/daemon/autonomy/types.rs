//! Autonomy system types.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// State of the autonomy orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestratorState {
    /// Not running.
    Stopped,
    /// Starting up.
    Starting,
    /// Running and processing.
    Running,
    /// Paused, waiting for user.
    Paused,
    /// Gracefully shutting down.
    ShuttingDown,
}

impl OrchestratorState {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            OrchestratorState::Stopped => "stopped",
            OrchestratorState::Starting => "starting",
            OrchestratorState::Running => "running",
            OrchestratorState::Paused => "paused",
            OrchestratorState::ShuttingDown => "shutting_down",
        }
    }

    /// Check if operational.
    pub fn is_operational(&self) -> bool {
        matches!(self, OrchestratorState::Running | OrchestratorState::Paused)
    }
}

impl std::fmt::Display for OrchestratorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An autonomous task to be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousTask {
    /// Task ID.
    pub id: Uuid,
    /// Task description.
    pub description: String,
    /// Source that triggered this task.
    pub source: TaskSource,
    /// Priority (1-10, lower = higher).
    pub priority: u8,
    /// Required approval level.
    pub approval_level: super::ApprovalLevel,
    /// Whether approved.
    pub approved: bool,
    /// Assigned agent ID.
    pub agent_id: Option<crate::daemon::AgentId>,
    /// Associated job ID.
    pub job_id: Option<crate::daemon::JobId>,
    /// When created.
    pub created_at: SystemTime,
    /// When started.
    pub started_at: Option<SystemTime>,
    /// When completed.
    pub completed_at: Option<SystemTime>,
    /// Current status.
    pub status: TaskStatus,
    /// Result (if completed).
    pub result: Option<TaskResult>,
    /// Estimated duration.
    pub estimated_duration: Option<Duration>,
    /// Context data.
    pub context: Option<serde_json::Value>,
}

impl AutonomousTask {
    /// Create a new task.
    pub fn new(description: impl Into<String>, source: TaskSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            source,
            priority: 5,
            approval_level: super::ApprovalLevel::Auto,
            approved: false,
            agent_id: None,
            job_id: None,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            status: TaskStatus::Pending,
            result: None,
            estimated_duration: None,
            context: None,
        }
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 10);
        self
    }

    /// Set approval level.
    pub fn with_approval_level(mut self, level: super::ApprovalLevel) -> Self {
        self.approval_level = level;
        self
    }

    /// Set estimated duration.
    pub fn with_estimated_duration(mut self, duration: Duration) -> Self {
        self.estimated_duration = Some(duration);
        self
    }

    /// Set context.
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// Approve the task.
    pub fn approve(&mut self) {
        self.approved = true;
    }

    /// Reject the task.
    pub fn reject(&mut self) {
        self.status = TaskStatus::Rejected;
    }

    /// Start the task.
    pub fn start(&mut self, agent_id: crate::daemon::AgentId) {
        self.agent_id = Some(agent_id);
        self.started_at = Some(SystemTime::now());
        self.status = TaskStatus::Running;
    }

    /// Complete the task.
    pub fn complete(&mut self, result: TaskResult) {
        self.completed_at = Some(SystemTime::now());
        self.status = if result.success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        self.result = Some(result);
    }

    /// Cancel the task.
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(SystemTime::now());
    }

    /// Check if task needs approval.
    pub fn needs_approval(&self) -> bool {
        !self.approved && self.approval_level != super::ApprovalLevel::Auto
    }

    /// Get duration.
    pub fn duration(&self) -> Option<Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => end.duration_since(start).ok(),
            (Some(start), None) => SystemTime::now().duration_since(start).ok(),
            _ => None,
        }
    }
}

/// Source of an autonomous task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskSource {
    /// From a proactive suggestion.
    Suggestion {
        suggestion_id: crate::daemon::SuggestionId,
    },
    /// From a scheduled job.
    Job { job_id: crate::daemon::JobId },
    /// From user request.
    UserRequest { request_id: Uuid },
    /// From agent delegation.
    AgentDelegation {
        parent_agent: crate::daemon::AgentId,
    },
    /// From error recovery.
    ErrorRecovery { error: String },
    /// System-initiated.
    System { reason: String },
}

impl TaskSource {
    /// Get a brief description.
    pub fn description(&self) -> String {
        match self {
            TaskSource::Suggestion { .. } => "from suggestion".to_string(),
            TaskSource::Job { .. } => "from job".to_string(),
            TaskSource::UserRequest { .. } => "user request".to_string(),
            TaskSource::AgentDelegation { .. } => "agent delegation".to_string(),
            TaskSource::ErrorRecovery { error } => format!("error recovery: {}", error),
            TaskSource::System { reason } => format!("system: {}", reason),
        }
    }
}

/// Status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Waiting in queue.
    Pending,
    /// Waiting for approval.
    AwaitingApproval,
    /// Currently running.
    Running,
    /// Successfully completed.
    Completed,
    /// Failed.
    Failed,
    /// Cancelled.
    Cancelled,
    /// Rejected by user.
    Rejected,
}

impl TaskStatus {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::AwaitingApproval => "awaiting_approval",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Rejected => "rejected",
        }
    }

    /// Check if terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled
                | TaskStatus::Rejected
        )
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Result of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Whether successful.
    pub success: bool,
    /// Output or error message.
    pub message: String,
    /// Detailed output.
    pub output: Option<serde_json::Value>,
    /// Tokens used.
    pub tokens_used: u32,
    /// Turns/iterations used.
    pub turns_used: u32,
    /// Files modified.
    pub files_modified: Vec<String>,
    /// Follow-up tasks.
    pub follow_up_tasks: Vec<String>,
}

impl TaskResult {
    /// Create a success result.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            output: None,
            tokens_used: 0,
            turns_used: 0,
            files_modified: Vec::new(),
            follow_up_tasks: Vec::new(),
        }
    }

    /// Create a failure result.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            output: None,
            tokens_used: 0,
            turns_used: 0,
            files_modified: Vec::new(),
            follow_up_tasks: Vec::new(),
        }
    }

    /// Set output.
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Set token usage.
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = tokens;
        self
    }

    /// Set turns.
    pub fn with_turns(mut self, turns: u32) -> Self {
        self.turns_used = turns;
        self
    }

    /// Set modified files.
    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files_modified = files;
        self
    }

    /// Add follow-up tasks.
    pub fn with_follow_ups(mut self, tasks: Vec<String>) -> Self {
        self.follow_up_tasks = tasks;
        self
    }
}

/// A decision made by the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Decision ID.
    pub id: Uuid,
    /// Description of what was decided.
    pub description: String,
    /// Options considered.
    pub options: Vec<String>,
    /// Selected option index.
    pub selected: usize,
    /// Reasoning for the decision.
    pub reasoning: String,
    /// Confidence level (0.0-1.0).
    pub confidence: f32,
    /// When decided.
    pub decided_at: SystemTime,
    /// Outcome (if known).
    pub outcome: Option<DecisionOutcome>,
}

impl Decision {
    /// Create a new decision.
    pub fn new(
        description: impl Into<String>,
        options: Vec<String>,
        selected: usize,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            options,
            selected,
            reasoning: reasoning.into(),
            confidence: 1.0,
            decided_at: SystemTime::now(),
            outcome: None,
        }
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Get the selected option.
    pub fn selected_option(&self) -> Option<&str> {
        self.options.get(self.selected).map(|s| s.as_str())
    }

    /// Record outcome.
    pub fn record_outcome(&mut self, outcome: DecisionOutcome) {
        self.outcome = Some(outcome);
    }
}

/// Outcome of a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    /// Whether the decision was good.
    pub positive: bool,
    /// Description of outcome.
    pub description: String,
    /// When outcome was observed.
    pub observed_at: SystemTime,
}

impl DecisionOutcome {
    /// Create a positive outcome.
    pub fn positive(description: impl Into<String>) -> Self {
        Self {
            positive: true,
            description: description.into(),
            observed_at: SystemTime::now(),
        }
    }

    /// Create a negative outcome.
    pub fn negative(description: impl Into<String>) -> Self {
        Self {
            positive: false,
            description: description.into(),
            observed_at: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_state() {
        assert!(OrchestratorState::Running.is_operational());
        assert!(OrchestratorState::Paused.is_operational());
        assert!(!OrchestratorState::Stopped.is_operational());
    }

    #[test]
    fn test_autonomous_task_creation() {
        let task = AutonomousTask::new(
            "Run tests",
            TaskSource::System {
                reason: "scheduled".into(),
            },
        )
        .with_priority(2)
        .with_estimated_duration(Duration::from_secs(60));

        assert_eq!(task.priority, 2);
        assert!(task.estimated_duration.is_some());
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = AutonomousTask::new(
            "Test task",
            TaskSource::System {
                reason: "test".into(),
            },
        );

        assert!(task.needs_approval() == false); // Auto level
        task.approval_level = super::super::ApprovalLevel::Required;
        assert!(task.needs_approval());

        task.approve();
        assert!(!task.needs_approval());

        let agent_id = crate::daemon::AgentId::new();
        task.start(agent_id);
        assert_eq!(task.status, TaskStatus::Running);
        assert!(task.started_at.is_some());

        task.complete(TaskResult::success("Done"));
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_result() {
        let result = TaskResult::success("Task completed")
            .with_tokens(1000)
            .with_turns(5)
            .with_files(vec!["src/main.rs".into()])
            .with_follow_ups(vec!["Run tests".into()]);

        assert!(result.success);
        assert_eq!(result.tokens_used, 1000);
        assert_eq!(result.files_modified.len(), 1);
        assert_eq!(result.follow_up_tasks.len(), 1);
    }

    #[test]
    fn test_decision() {
        let mut decision = Decision::new(
            "Which approach to use?",
            vec!["Option A".into(), "Option B".into()],
            0,
            "Option A is more efficient",
        )
        .with_confidence(0.85);

        assert_eq!(decision.selected_option(), Some("Option A"));
        assert_eq!(decision.confidence, 0.85);

        decision.record_outcome(DecisionOutcome::positive("Worked well"));
        assert!(decision.outcome.is_some());
        assert!(decision.outcome.unwrap().positive);
    }

    #[test]
    fn test_task_status() {
        assert!(!TaskStatus::Running.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }
}
