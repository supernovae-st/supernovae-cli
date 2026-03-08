//! Agent delegation types.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Unique identifier for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new random agent ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Role that an agent can fulfill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Explores codebase for information.
    Explorer,
    /// Reviews code for issues.
    Reviewer,
    /// Generates code or content.
    Generator,
    /// Runs and analyzes tests.
    Tester,
    /// Researches external information.
    Researcher,
    /// Refactors existing code.
    Refactorer,
    /// Documents code.
    Documenter,
    /// General-purpose assistant.
    General,
}

impl AgentRole {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Explorer => "explorer",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Generator => "generator",
            AgentRole::Tester => "tester",
            AgentRole::Researcher => "researcher",
            AgentRole::Refactorer => "refactorer",
            AgentRole::Documenter => "documenter",
            AgentRole::General => "general",
        }
    }

    /// Get icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            AgentRole::Explorer => "🔍",
            AgentRole::Reviewer => "👀",
            AgentRole::Generator => "✨",
            AgentRole::Tester => "🧪",
            AgentRole::Researcher => "📚",
            AgentRole::Refactorer => "🔧",
            AgentRole::Documenter => "📝",
            AgentRole::General => "🤖",
        }
    }

    /// Get default model for this role.
    pub fn default_model(&self) -> &'static str {
        match self {
            AgentRole::Explorer => "claude-3-haiku-20240307",
            AgentRole::Reviewer => "claude-3-5-sonnet-20241022",
            AgentRole::Generator => "claude-3-5-sonnet-20241022",
            AgentRole::Tester => "claude-3-haiku-20240307",
            AgentRole::Researcher => "claude-3-5-sonnet-20241022",
            AgentRole::Refactorer => "claude-3-5-sonnet-20241022",
            AgentRole::Documenter => "claude-3-haiku-20240307",
            AgentRole::General => "claude-3-5-sonnet-20241022",
        }
    }
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Current state of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is idle, ready for tasks.
    Idle,
    /// Agent is thinking/planning.
    Thinking,
    /// Agent is executing a task.
    Working,
    /// Agent is waiting for response.
    Waiting,
    /// Agent has completed its task.
    Completed,
    /// Agent encountered an error.
    Failed,
    /// Agent was cancelled.
    Cancelled,
}

impl AgentState {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentState::Idle => "idle",
            AgentState::Thinking => "thinking",
            AgentState::Working => "working",
            AgentState::Waiting => "waiting",
            AgentState::Completed => "completed",
            AgentState::Failed => "failed",
            AgentState::Cancelled => "cancelled",
        }
    }

    /// Check if terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentState::Completed | AgentState::Failed | AgentState::Cancelled
        )
    }

    /// Check if active.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            AgentState::Thinking | AgentState::Working | AgentState::Waiting
        )
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration for spawning an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Role for the agent.
    pub role: AgentRole,
    /// Model to use (overrides role default).
    pub model: Option<String>,
    /// Maximum iterations/turns.
    pub max_turns: u32,
    /// Timeout for the entire task.
    pub timeout: Duration,
    /// System prompt override.
    pub system_prompt: Option<String>,
    /// Tools available to the agent.
    pub tools: Vec<String>,
    /// Whether to run in background.
    pub background: bool,
    /// Parent agent ID (for nested delegation).
    pub parent_id: Option<AgentId>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            role: AgentRole::General,
            model: None,
            max_turns: 10,
            timeout: Duration::from_secs(300), // 5 minutes
            system_prompt: None,
            tools: Vec::new(),
            background: false,
            parent_id: None,
        }
    }
}

impl AgentConfig {
    /// Create config for a specific role.
    pub fn for_role(role: AgentRole) -> Self {
        Self {
            role,
            ..Default::default()
        }
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set max turns.
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = turns;
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set available tools.
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// Run in background.
    pub fn in_background(mut self) -> Self {
        self.background = true;
        self
    }

    /// Set parent agent.
    pub fn with_parent(mut self, parent_id: AgentId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Get the model (role default or override).
    pub fn effective_model(&self) -> &str {
        self.model
            .as_deref()
            .unwrap_or_else(|| self.role.default_model())
    }
}

/// A task delegated to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    /// Task ID.
    pub id: Uuid,
    /// Task description/prompt.
    pub prompt: String,
    /// Context to include.
    pub context: Option<serde_json::Value>,
    /// Files to include.
    pub files: Vec<String>,
    /// Expected output type.
    pub expected_output: ExpectedOutput,
    /// Priority level.
    pub priority: u8,
}

/// Expected output from an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedOutput {
    /// Plain text response.
    Text,
    /// JSON structured data.
    Json,
    /// Code changes (file edits).
    CodeChanges,
    /// Analysis/review results.
    Analysis,
    /// Boolean decision.
    Decision,
}

impl DelegatedTask {
    /// Create a new task.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            prompt: prompt.into(),
            context: None,
            files: Vec::new(),
            expected_output: ExpectedOutput::Text,
            priority: 5,
        }
    }

    /// Add context.
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// Add files.
    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    /// Set expected output.
    pub fn expecting(mut self, output: ExpectedOutput) -> Self {
        self.expected_output = output;
        self
    }

    /// Set priority (1-10, lower = higher priority).
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 10);
        self
    }
}

/// Status of an agent with detailed information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent ID.
    pub id: AgentId,
    /// Current state.
    pub state: AgentState,
    /// Role.
    pub role: AgentRole,
    /// Current task (if any).
    pub task: Option<String>,
    /// Number of turns used.
    pub turns_used: u32,
    /// Tokens consumed.
    pub tokens_used: u32,
    /// When agent started.
    pub started_at: SystemTime,
    /// Last activity time.
    pub last_activity: SystemTime,
    /// Progress percentage (0-100).
    pub progress: u8,
}

/// An active agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Agent ID.
    pub id: AgentId,
    /// Configuration.
    pub config: AgentConfig,
    /// Current state.
    pub state: AgentState,
    /// Current task.
    pub task: Option<DelegatedTask>,
    /// Result (if completed).
    pub result: Option<AgentResult>,
    /// When created.
    pub created_at: SystemTime,
    /// When last updated.
    pub updated_at: SystemTime,
    /// Turns used.
    pub turns_used: u32,
    /// Tokens used.
    pub tokens_used: u32,
    /// Child agent IDs.
    pub children: Vec<AgentId>,
}

impl Agent {
    /// Create a new agent.
    pub fn new(config: AgentConfig) -> Self {
        let now = SystemTime::now();
        Self {
            id: AgentId::new(),
            config,
            state: AgentState::Idle,
            task: None,
            result: None,
            created_at: now,
            updated_at: now,
            turns_used: 0,
            tokens_used: 0,
            children: Vec::new(),
        }
    }

    /// Assign a task.
    pub fn assign(&mut self, task: DelegatedTask) {
        self.task = Some(task);
        self.state = AgentState::Thinking;
        self.updated_at = SystemTime::now();
    }

    /// Update state.
    pub fn set_state(&mut self, state: AgentState) {
        self.state = state;
        self.updated_at = SystemTime::now();
    }

    /// Record a turn.
    pub fn record_turn(&mut self, tokens: u32) {
        self.turns_used += 1;
        self.tokens_used += tokens;
        self.updated_at = SystemTime::now();
    }

    /// Complete with result.
    pub fn complete(&mut self, result: AgentResult) {
        self.result = Some(result);
        self.state = AgentState::Completed;
        self.updated_at = SystemTime::now();
    }

    /// Fail with error.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.result = Some(AgentResult::Error(error.into()));
        self.state = AgentState::Failed;
        self.updated_at = SystemTime::now();
    }

    /// Cancel the agent.
    pub fn cancel(&mut self) {
        self.state = AgentState::Cancelled;
        self.updated_at = SystemTime::now();
    }

    /// Add a child agent.
    pub fn add_child(&mut self, child_id: AgentId) {
        self.children.push(child_id);
    }

    /// Get status summary.
    pub fn status(&self) -> AgentStatus {
        AgentStatus {
            id: self.id,
            state: self.state,
            role: self.config.role,
            task: self.task.as_ref().map(|t| t.prompt.clone()),
            turns_used: self.turns_used,
            tokens_used: self.tokens_used,
            started_at: self.created_at,
            last_activity: self.updated_at,
            progress: self.estimate_progress(),
        }
    }

    /// Estimate progress percentage.
    fn estimate_progress(&self) -> u8 {
        if self.state.is_terminal() {
            return 100;
        }
        let turn_progress = (self.turns_used as f32 / self.config.max_turns as f32 * 100.0) as u8;
        turn_progress.min(99) // Never show 100% until actually complete
    }

    /// Check if agent has exceeded timeout.
    pub fn is_timed_out(&self) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.created_at) {
            elapsed > self.config.timeout
        } else {
            false
        }
    }

    /// Check if agent has exceeded max turns.
    pub fn is_turn_limited(&self) -> bool {
        self.turns_used >= self.config.max_turns
    }
}

/// Result from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentResult {
    /// Text response.
    Text(String),
    /// JSON data.
    Json(serde_json::Value),
    /// Code changes.
    CodeChanges(Vec<CodeChange>),
    /// Analysis results.
    Analysis(AnalysisResult),
    /// Boolean decision.
    Decision(bool),
    /// Error occurred.
    Error(String),
}

/// A code change made by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    /// File path.
    pub path: String,
    /// Change type.
    pub change_type: ChangeType,
    /// Content (for create/modify).
    pub content: Option<String>,
}

/// Type of code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Create new file.
    Create,
    /// Modify existing file.
    Modify,
    /// Delete file.
    Delete,
}

/// Analysis result from a reviewer agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Summary.
    pub summary: String,
    /// Issues found.
    pub issues: Vec<Issue>,
    /// Suggestions.
    pub suggestions: Vec<String>,
    /// Overall score (0-100).
    pub score: u8,
}

/// An issue found during analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Severity.
    pub severity: IssueSeverity,
    /// Description.
    pub description: String,
    /// Location (file:line).
    pub location: Option<String>,
    /// Suggested fix.
    pub fix: Option<String>,
}

/// Issue severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Critical issue.
    Critical,
    /// High severity.
    High,
    /// Medium severity.
    Medium,
    /// Low severity.
    Low,
    /// Informational.
    Info,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_roles() {
        assert_eq!(AgentRole::Explorer.as_str(), "explorer");
        assert_eq!(AgentRole::Reviewer.icon(), "👀");
        assert!(AgentRole::Generator.default_model().contains("sonnet"));
    }

    #[test]
    fn test_agent_config() {
        let config = AgentConfig::for_role(AgentRole::Tester)
            .with_model("custom-model")
            .with_max_turns(20)
            .in_background();

        assert_eq!(config.role, AgentRole::Tester);
        assert_eq!(config.effective_model(), "custom-model");
        assert_eq!(config.max_turns, 20);
        assert!(config.background);
    }

    #[test]
    fn test_agent_lifecycle() {
        let config = AgentConfig::for_role(AgentRole::Generator);
        let mut agent = Agent::new(config);

        assert_eq!(agent.state, AgentState::Idle);

        let task = DelegatedTask::new("Generate a function");
        agent.assign(task);
        assert_eq!(agent.state, AgentState::Thinking);

        agent.record_turn(100);
        assert_eq!(agent.turns_used, 1);
        assert_eq!(agent.tokens_used, 100);

        agent.complete(AgentResult::Text("Done".into()));
        assert_eq!(agent.state, AgentState::Completed);
        assert!(agent.state.is_terminal());
    }

    #[test]
    fn test_agent_failure() {
        let mut agent = Agent::new(AgentConfig::default());
        agent.fail("Something went wrong");

        assert_eq!(agent.state, AgentState::Failed);
        assert!(matches!(agent.result, Some(AgentResult::Error(_))));
    }

    #[test]
    fn test_delegated_task() {
        let task = DelegatedTask::new("Review this code")
            .with_files(vec!["src/main.rs".into()])
            .expecting(ExpectedOutput::Analysis)
            .with_priority(2);

        assert_eq!(task.files.len(), 1);
        assert_eq!(task.expected_output, ExpectedOutput::Analysis);
        assert_eq!(task.priority, 2);
    }

    #[test]
    fn test_agent_progress() {
        let config = AgentConfig::for_role(AgentRole::General).with_max_turns(10);
        let mut agent = Agent::new(config);

        assert_eq!(agent.estimate_progress(), 0);

        agent.record_turn(10);
        agent.record_turn(10);
        agent.record_turn(10);
        // 3/10 turns = 30%
        assert_eq!(agent.estimate_progress(), 30);

        agent.complete(AgentResult::Text("Done".into()));
        assert_eq!(agent.estimate_progress(), 100);
    }
}
