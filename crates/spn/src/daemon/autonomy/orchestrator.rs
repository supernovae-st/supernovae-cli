//! Autonomy orchestrator for coordinating autonomous operations.

#![allow(dead_code)]

use super::{
    ApprovalLevel, AutonomousTask, AutonomyLevel, AutonomyPolicy, Decision, DecisionOutcome,
    OrchestratorState, TaskResult,
};
use crate::daemon::agents::{AgentId, AgentManager};
use crate::daemon::memory::MemoryStore;
use crate::daemon::proactive::{ProactiveSuggestion, SuggestionAnalyzer};
use crate::daemon::traces::TraceStore;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use uuid::Uuid;

/// Statistics for the orchestrator.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrchestratorStats {
    /// Total tasks processed.
    pub tasks_processed: u64,
    /// Tasks completed successfully.
    pub tasks_completed: u64,
    /// Tasks failed.
    pub tasks_failed: u64,
    /// Tasks cancelled.
    pub tasks_cancelled: u64,
    /// Tasks rejected by user.
    pub tasks_rejected: u64,
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Decisions made.
    pub decisions_made: u64,
    /// Positive decision outcomes.
    pub positive_outcomes: u64,
    /// Auto-approved tasks.
    pub auto_approved: u64,
    /// User-approved tasks.
    pub user_approved: u64,
}

/// Configuration for the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Policy for autonomous operation.
    pub policy: AutonomyPolicy,
    /// Whether to persist decisions.
    pub persist_decisions: bool,
    /// Maximum queued tasks.
    pub max_queued_tasks: usize,
    /// Whether to auto-start on creation.
    pub auto_start: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            policy: AutonomyPolicy::default(),
            persist_decisions: true,
            max_queued_tasks: 100,
            auto_start: false,
        }
    }
}

/// The main autonomy orchestrator.
///
/// Coordinates agents, memory, traces, and proactive suggestions
/// to enable autonomous operation with human oversight.
pub struct AutonomyOrchestrator {
    /// Current state.
    state: RwLock<OrchestratorState>,
    /// Configuration.
    config: RwLock<OrchestratorConfig>,
    /// Task queue.
    tasks: RwLock<FxHashMap<Uuid, AutonomousTask>>,
    /// Decision history.
    decisions: RwLock<Vec<Decision>>,
    /// Statistics.
    stats: RwLock<OrchestratorStats>,
    /// Agent manager (optional, may not be initialized).
    #[allow(dead_code)]
    agent_manager: Option<Arc<AgentManager>>,
    /// Memory store (optional).
    #[allow(dead_code)]
    memory_store: Option<Arc<MemoryStore>>,
    /// Trace store (optional).
    #[allow(dead_code)]
    trace_store: Option<Arc<TraceStore>>,
    /// Suggestion analyzer (optional).
    #[allow(dead_code)]
    suggestion_analyzer: Option<Arc<SuggestionAnalyzer>>,
    /// When started.
    started_at: RwLock<Option<SystemTime>>,
}

impl AutonomyOrchestrator {
    /// Create a new orchestrator with default config.
    pub fn new() -> Self {
        Self::with_config(OrchestratorConfig::default())
    }

    /// Create with custom config.
    pub fn with_config(config: OrchestratorConfig) -> Self {
        let auto_start = config.auto_start;
        let orchestrator = Self {
            state: RwLock::new(OrchestratorState::Stopped),
            config: RwLock::new(config),
            tasks: RwLock::new(FxHashMap::default()),
            decisions: RwLock::new(Vec::new()),
            stats: RwLock::new(OrchestratorStats::default()),
            agent_manager: None,
            memory_store: None,
            trace_store: None,
            suggestion_analyzer: None,
            started_at: RwLock::new(None),
        };

        if auto_start {
            orchestrator.start();
        }

        orchestrator
    }

    /// Set agent manager.
    pub fn with_agent_manager(mut self, manager: Arc<AgentManager>) -> Self {
        self.agent_manager = Some(manager);
        self
    }

    /// Set memory store.
    pub fn with_memory_store(mut self, store: Arc<MemoryStore>) -> Self {
        self.memory_store = Some(store);
        self
    }

    /// Set trace store.
    pub fn with_trace_store(mut self, store: Arc<TraceStore>) -> Self {
        self.trace_store = Some(store);
        self
    }

    /// Set suggestion analyzer.
    pub fn with_suggestion_analyzer(mut self, analyzer: Arc<SuggestionAnalyzer>) -> Self {
        self.suggestion_analyzer = Some(analyzer);
        self
    }

    /// Get current state.
    pub fn state(&self) -> OrchestratorState {
        *self.state.read().unwrap()
    }

    /// Get current autonomy level.
    pub fn autonomy_level(&self) -> AutonomyLevel {
        self.config.read().unwrap().policy.level
    }

    /// Get current policy.
    pub fn policy(&self) -> AutonomyPolicy {
        self.config.read().unwrap().policy.clone()
    }

    /// Set autonomy level.
    pub fn set_autonomy_level(&self, level: AutonomyLevel) {
        self.config.write().unwrap().policy.level = level;
    }

    /// Start the orchestrator.
    pub fn start(&self) {
        let mut state = self.state.write().unwrap();
        if *state == OrchestratorState::Stopped {
            *state = OrchestratorState::Starting;
            *self.started_at.write().unwrap() = Some(SystemTime::now());
            *state = OrchestratorState::Running;
        }
    }

    /// Pause the orchestrator.
    pub fn pause(&self) {
        let mut state = self.state.write().unwrap();
        if *state == OrchestratorState::Running {
            *state = OrchestratorState::Paused;
        }
    }

    /// Resume the orchestrator.
    pub fn resume(&self) {
        let mut state = self.state.write().unwrap();
        if *state == OrchestratorState::Paused {
            *state = OrchestratorState::Running;
        }
    }

    /// Stop the orchestrator.
    pub fn stop(&self) {
        let mut state = self.state.write().unwrap();
        if state.is_operational() {
            *state = OrchestratorState::ShuttingDown;
            // In a real implementation, we would drain tasks here
            *state = OrchestratorState::Stopped;
            *self.started_at.write().unwrap() = None;
        }
    }

    /// Submit a task for autonomous execution.
    pub fn submit_task(&self, mut task: AutonomousTask) -> Result<Uuid, String> {
        // Check state
        if !self.state.read().unwrap().is_operational() {
            return Err("Orchestrator is not running".to_string());
        }

        // Check queue limit
        let config = self.config.read().unwrap();
        if self.tasks.read().unwrap().len() >= config.max_queued_tasks {
            return Err("Task queue is full".to_string());
        }

        // Determine approval level based on policy
        let approval_level = config.policy.approval_level_for(&task.description);
        task.approval_level = approval_level;

        // Auto-approve if allowed
        if approval_level == ApprovalLevel::Auto {
            task.approved = true;
            self.stats.write().unwrap().auto_approved += 1;
        }

        let task_id = task.id;
        self.tasks.write().unwrap().insert(task_id, task);
        self.stats.write().unwrap().tasks_processed += 1;

        Ok(task_id)
    }

    /// Get a task by ID.
    pub fn get_task(&self, task_id: Uuid) -> Option<AutonomousTask> {
        self.tasks.read().unwrap().get(&task_id).cloned()
    }

    /// Approve a task.
    pub fn approve_task(&self, task_id: Uuid) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

        if task.approved {
            return Err("Task already approved".to_string());
        }

        task.approve();
        self.stats.write().unwrap().user_approved += 1;
        Ok(())
    }

    /// Reject a task.
    pub fn reject_task(&self, task_id: Uuid) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

        task.reject();
        self.stats.write().unwrap().tasks_rejected += 1;
        Ok(())
    }

    /// Cancel a task.
    pub fn cancel_task(&self, task_id: Uuid) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

        task.cancel();
        self.stats.write().unwrap().tasks_cancelled += 1;
        Ok(())
    }

    /// Start executing a task.
    pub fn start_task(&self, task_id: Uuid, agent_id: AgentId) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

        if !task.approved {
            return Err("Task not approved".to_string());
        }

        task.start(agent_id);
        Ok(())
    }

    /// Complete a task.
    pub fn complete_task(&self, task_id: Uuid, result: TaskResult) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

        let success = result.success;
        let tokens = result.tokens_used;
        task.complete(result);

        let mut stats = self.stats.write().unwrap();
        stats.total_tokens += tokens as u64;
        if success {
            stats.tasks_completed += 1;
        } else {
            stats.tasks_failed += 1;
        }

        Ok(())
    }

    /// Record a decision.
    pub fn record_decision(&self, decision: Decision) -> Uuid {
        let decision_id = decision.id;
        self.decisions.write().unwrap().push(decision);
        self.stats.write().unwrap().decisions_made += 1;
        decision_id
    }

    /// Record decision outcome.
    pub fn record_outcome(&self, decision_id: Uuid, outcome: DecisionOutcome) -> Result<(), String> {
        let mut decisions = self.decisions.write().unwrap();
        let decision = decisions
            .iter_mut()
            .find(|d| d.id == decision_id)
            .ok_or("Decision not found")?;

        if outcome.positive {
            self.stats.write().unwrap().positive_outcomes += 1;
        }

        decision.record_outcome(outcome);
        Ok(())
    }

    /// Get decisions.
    pub fn decisions(&self) -> Vec<Decision> {
        self.decisions.read().unwrap().clone()
    }

    /// Get pending tasks.
    pub fn pending_tasks(&self) -> Vec<AutonomousTask> {
        self.tasks
            .read()
            .unwrap()
            .values()
            .filter(|t| !t.status.is_terminal())
            .cloned()
            .collect()
    }

    /// Get tasks awaiting approval.
    pub fn tasks_awaiting_approval(&self) -> Vec<AutonomousTask> {
        self.tasks
            .read()
            .unwrap()
            .values()
            .filter(|t: &&AutonomousTask| t.needs_approval())
            .cloned()
            .collect()
    }

    /// Get statistics.
    pub fn stats(&self) -> OrchestratorStats {
        self.stats.read().unwrap().clone()
    }

    /// Process a proactive suggestion.
    pub fn process_suggestion(&self, suggestion: &ProactiveSuggestion) -> Result<Uuid, String> {
        use super::types::TaskSource;

        // Convert priority enum to u8 value
        let priority = match suggestion.priority {
            crate::daemon::proactive::SuggestionPriority::Critical => 1,
            crate::daemon::proactive::SuggestionPriority::High => 2,
            crate::daemon::proactive::SuggestionPriority::Medium => 3,
            crate::daemon::proactive::SuggestionPriority::Low => 4,
            crate::daemon::proactive::SuggestionPriority::Info => 5,
        };

        let task = AutonomousTask::new(
            &suggestion.title,
            TaskSource::Suggestion {
                suggestion_id: suggestion.id,
            },
        )
        .with_priority(priority)
        .with_context(serde_json::json!({
            "suggestion_id": suggestion.id.to_string(),
            "category": suggestion.category.as_str(),
            "description": suggestion.description,
        }));

        self.submit_task(task)
    }

    /// Get uptime.
    pub fn uptime(&self) -> Option<std::time::Duration> {
        self.started_at
            .read()
            .unwrap()
            .and_then(|start| SystemTime::now().duration_since(start).ok())
    }

    /// Clear completed tasks older than given duration.
    pub fn cleanup_old_tasks(&self, max_age: std::time::Duration) {
        let now = SystemTime::now();
        self.tasks.write().unwrap().retain(|_, task| {
            if let Some(completed_at) = task.completed_at {
                if let Ok(age) = now.duration_since(completed_at) {
                    return age < max_age;
                }
            }
            true
        });
    }
}

impl Default for AutonomyOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::autonomy::types::TaskSource;

    #[test]
    fn test_orchestrator_lifecycle() {
        let orchestrator = AutonomyOrchestrator::new();
        assert_eq!(orchestrator.state(), OrchestratorState::Stopped);

        orchestrator.start();
        assert_eq!(orchestrator.state(), OrchestratorState::Running);

        orchestrator.pause();
        assert_eq!(orchestrator.state(), OrchestratorState::Paused);

        orchestrator.resume();
        assert_eq!(orchestrator.state(), OrchestratorState::Running);

        orchestrator.stop();
        assert_eq!(orchestrator.state(), OrchestratorState::Stopped);
    }

    #[test]
    fn test_task_submission() {
        let orchestrator = AutonomyOrchestrator::new();
        orchestrator.start();

        let task = AutonomousTask::new("Run tests", TaskSource::System {
            reason: "test".into(),
        });
        let result = orchestrator.submit_task(task);
        assert!(result.is_ok());

        let stats = orchestrator.stats();
        assert_eq!(stats.tasks_processed, 1);
    }

    #[test]
    fn test_task_requires_stopped_orchestrator() {
        let orchestrator = AutonomyOrchestrator::new();
        // Don't start it

        let task = AutonomousTask::new("Test", TaskSource::System {
            reason: "test".into(),
        });
        let result = orchestrator.submit_task(task);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_approval_flow() {
        let orchestrator = AutonomyOrchestrator::new();
        orchestrator.start();

        // Set to manual mode (all tasks need approval)
        orchestrator.set_autonomy_level(AutonomyLevel::Manual);

        let task = AutonomousTask::new("Test task", TaskSource::System {
            reason: "test".into(),
        });
        let task_id = orchestrator.submit_task(task).unwrap();

        // Task should need approval in manual mode
        let task = orchestrator.get_task(task_id).unwrap();
        assert!(task.needs_approval());

        // Approve it
        orchestrator.approve_task(task_id).unwrap();
        let task = orchestrator.get_task(task_id).unwrap();
        assert!(!task.needs_approval());
    }

    #[test]
    fn test_auto_approval() {
        let orchestrator = AutonomyOrchestrator::new();
        orchestrator.start();

        // Set to full autonomy mode
        orchestrator.set_autonomy_level(AutonomyLevel::Full);

        let task = AutonomousTask::new("Test task", TaskSource::System {
            reason: "test".into(),
        });
        let task_id = orchestrator.submit_task(task).unwrap();

        // Task should be auto-approved in full mode
        let task = orchestrator.get_task(task_id).unwrap();
        assert!(task.approved);

        let stats = orchestrator.stats();
        assert_eq!(stats.auto_approved, 1);
    }

    #[test]
    fn test_decision_recording() {
        let orchestrator = AutonomyOrchestrator::new();

        let decision = Decision::new(
            "Which approach?",
            vec!["A".into(), "B".into()],
            0,
            "A is better",
        );
        let decision_id = orchestrator.record_decision(decision);

        let decisions = orchestrator.decisions();
        assert_eq!(decisions.len(), 1);

        orchestrator
            .record_outcome(decision_id, DecisionOutcome::positive("Worked well"))
            .unwrap();

        let stats = orchestrator.stats();
        assert_eq!(stats.decisions_made, 1);
        assert_eq!(stats.positive_outcomes, 1);
    }

    #[test]
    fn test_task_completion() {
        let orchestrator = AutonomyOrchestrator::new();
        orchestrator.start();
        orchestrator.set_autonomy_level(AutonomyLevel::Full);

        let task = AutonomousTask::new("Test", TaskSource::System {
            reason: "test".into(),
        });
        let task_id = orchestrator.submit_task(task).unwrap();

        let agent_id = AgentId::new();
        orchestrator.start_task(task_id, agent_id).unwrap();

        let result = TaskResult::success("Done").with_tokens(1000);
        orchestrator.complete_task(task_id, result).unwrap();

        let stats = orchestrator.stats();
        assert_eq!(stats.tasks_completed, 1);
        assert_eq!(stats.total_tokens, 1000);
    }

    #[test]
    fn test_uptime() {
        let orchestrator = AutonomyOrchestrator::new();
        assert!(orchestrator.uptime().is_none());

        orchestrator.start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let uptime = orchestrator.uptime();
        assert!(uptime.is_some());
        assert!(uptime.unwrap().as_millis() >= 10);

        orchestrator.stop();
        assert!(orchestrator.uptime().is_none());
    }
}
