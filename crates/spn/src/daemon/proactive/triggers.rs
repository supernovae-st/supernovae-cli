//! Context triggers for proactive suggestions.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A condition that can trigger a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// File matching pattern was modified.
    FileModified { pattern: String },
    /// File matching pattern was created.
    FileCreated { pattern: String },
    /// File matching pattern was deleted.
    FileDeleted { pattern: String },
    /// Command was executed.
    CommandExecuted { command: String },
    /// Command failed.
    CommandFailed { command: String, exit_code: Option<i32> },
    /// Error pattern detected.
    ErrorDetected { pattern: String },
    /// Time since last activity.
    TimeSinceActivity { duration: Duration },
    /// Periodic schedule.
    Periodic { interval: Duration },
    /// Memory value changed.
    MemoryChanged { key: String },
    /// Project entered directory.
    ProjectEntered { path: String },
    /// Git status changed.
    GitStatusChanged,
    /// Tests completed.
    TestsCompleted { passed: bool },
    /// Build completed.
    BuildCompleted { success: bool },
    /// Custom condition.
    Custom { name: String, params: serde_json::Value },
}

impl TriggerCondition {
    /// File modified trigger.
    pub fn file_modified(pattern: impl Into<String>) -> Self {
        Self::FileModified {
            pattern: pattern.into(),
        }
    }

    /// Command executed trigger.
    pub fn command_executed(command: impl Into<String>) -> Self {
        Self::CommandExecuted {
            command: command.into(),
        }
    }

    /// Command failed trigger.
    pub fn command_failed(command: impl Into<String>) -> Self {
        Self::CommandFailed {
            command: command.into(),
            exit_code: None,
        }
    }

    /// Error detected trigger.
    pub fn error_detected(pattern: impl Into<String>) -> Self {
        Self::ErrorDetected {
            pattern: pattern.into(),
        }
    }

    /// Periodic trigger.
    pub fn periodic(interval: Duration) -> Self {
        Self::Periodic { interval }
    }

    /// Get a brief description.
    pub fn description(&self) -> String {
        match self {
            TriggerCondition::FileModified { pattern } => format!("file modified: {}", pattern),
            TriggerCondition::FileCreated { pattern } => format!("file created: {}", pattern),
            TriggerCondition::FileDeleted { pattern } => format!("file deleted: {}", pattern),
            TriggerCondition::CommandExecuted { command } => format!("after: {}", command),
            TriggerCondition::CommandFailed { command, .. } => format!("{} failed", command),
            TriggerCondition::ErrorDetected { pattern } => format!("error: {}", pattern),
            TriggerCondition::TimeSinceActivity { duration } => {
                format!("inactive for {:?}", duration)
            }
            TriggerCondition::Periodic { interval } => format!("every {:?}", interval),
            TriggerCondition::MemoryChanged { key } => format!("memory changed: {}", key),
            TriggerCondition::ProjectEntered { path } => format!("entered: {}", path),
            TriggerCondition::GitStatusChanged => "git status changed".to_string(),
            TriggerCondition::TestsCompleted { passed } => {
                format!("tests {}", if *passed { "passed" } else { "failed" })
            }
            TriggerCondition::BuildCompleted { success } => {
                format!("build {}", if *success { "succeeded" } else { "failed" })
            }
            TriggerCondition::Custom { name, .. } => format!("custom: {}", name),
        }
    }
}

/// A complete context trigger definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTrigger {
    /// Unique name for this trigger.
    pub name: String,
    /// Conditions that must be met (all must match).
    pub conditions: Vec<TriggerCondition>,
    /// Whether trigger is enabled.
    pub enabled: bool,
    /// Cooldown before trigger can fire again.
    pub cooldown: Option<Duration>,
    /// Maximum times to fire (None = unlimited).
    pub max_fires: Option<u32>,
    /// Times this trigger has fired.
    pub fire_count: u32,
    /// Last time this trigger fired.
    pub last_fired: Option<std::time::SystemTime>,
}

impl ContextTrigger {
    /// Create a new trigger.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            conditions: Vec::new(),
            enabled: true,
            cooldown: None,
            max_fires: None,
            fire_count: 0,
            last_fired: None,
        }
    }

    /// Add a condition.
    pub fn with_condition(mut self, condition: TriggerCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set cooldown.
    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = Some(cooldown);
        self
    }

    /// Set max fires.
    pub fn with_max_fires(mut self, max: u32) -> Self {
        self.max_fires = Some(max);
        self
    }

    /// Enable/disable.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if trigger can fire.
    pub fn can_fire(&self) -> bool {
        if !self.enabled {
            return false;
        }

        // Check max fires
        if let Some(max) = self.max_fires {
            if self.fire_count >= max {
                return false;
            }
        }

        // Check cooldown
        if let (Some(cooldown), Some(last_fired)) = (self.cooldown, self.last_fired) {
            if let Ok(elapsed) = std::time::SystemTime::now().duration_since(last_fired) {
                if elapsed < cooldown {
                    return false;
                }
            }
        }

        true
    }

    /// Record that trigger fired.
    pub fn record_fire(&mut self) {
        self.fire_count += 1;
        self.last_fired = Some(std::time::SystemTime::now());
    }

    /// Reset fire count.
    pub fn reset(&mut self) {
        self.fire_count = 0;
        self.last_fired = None;
    }
}

/// Event that can be matched against triggers.
#[derive(Debug, Clone)]
pub enum ContextEvent {
    /// File was modified.
    FileModified { path: String },
    /// File was created.
    FileCreated { path: String },
    /// File was deleted.
    FileDeleted { path: String },
    /// Command was executed.
    CommandExecuted {
        command: String,
        exit_code: i32,
        duration: Duration,
    },
    /// Error occurred.
    Error { message: String, source: String },
    /// Project entered.
    ProjectEntered { path: String },
    /// Git status changed.
    GitStatusChanged { status: String },
    /// Tests ran.
    TestsRan { passed: u32, failed: u32 },
    /// Build completed.
    BuildCompleted { success: bool, duration: Duration },
    /// Timer tick.
    TimerTick,
}

impl ContextEvent {
    /// Check if event matches a condition.
    pub fn matches(&self, condition: &TriggerCondition) -> bool {
        match (self, condition) {
            (
                ContextEvent::FileModified { path },
                TriggerCondition::FileModified { pattern },
            ) => glob_match(pattern, path),

            (
                ContextEvent::FileCreated { path },
                TriggerCondition::FileCreated { pattern },
            ) => glob_match(pattern, path),

            (
                ContextEvent::FileDeleted { path },
                TriggerCondition::FileDeleted { pattern },
            ) => glob_match(pattern, path),

            (
                ContextEvent::CommandExecuted { command, exit_code, .. },
                TriggerCondition::CommandExecuted { command: cmd_pattern },
            ) => command.contains(cmd_pattern) && *exit_code == 0,

            (
                ContextEvent::CommandExecuted { command, exit_code, .. },
                TriggerCondition::CommandFailed { command: cmd_pattern, exit_code: expected },
            ) => {
                command.contains(cmd_pattern)
                    && *exit_code != 0
                    && expected.is_none_or(|e| e == *exit_code)
            }

            (
                ContextEvent::Error { message, .. },
                TriggerCondition::ErrorDetected { pattern },
            ) => message.to_lowercase().contains(&pattern.to_lowercase()),

            (
                ContextEvent::ProjectEntered { path },
                TriggerCondition::ProjectEntered { path: expected },
            ) => path.contains(expected),

            (ContextEvent::GitStatusChanged { .. }, TriggerCondition::GitStatusChanged) => true,

            (
                ContextEvent::TestsRan { failed, .. },
                TriggerCondition::TestsCompleted { passed },
            ) => (*failed == 0) == *passed,

            (
                ContextEvent::BuildCompleted { success, .. },
                TriggerCondition::BuildCompleted { success: expected },
            ) => success == expected,

            _ => false,
        }
    }
}

/// Simple glob matching (supports * and **).
fn glob_match(pattern: &str, path: &str) -> bool {
    // Simple implementation: just check contains for now
    // A full implementation would use a glob crate
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            return path.starts_with(parts[0].trim_end_matches('/'))
                && path.ends_with(parts[1].trim_start_matches('/'));
        }
    }

    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return path.starts_with(parts[0]) && path.ends_with(parts[1]);
        }
    }

    path.contains(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_condition_descriptions() {
        let cond = TriggerCondition::file_modified("*.rs");
        assert!(cond.description().contains("*.rs"));

        let cond = TriggerCondition::periodic(Duration::from_secs(3600));
        assert!(cond.description().contains("3600"));
    }

    #[test]
    fn test_context_trigger_creation() {
        let trigger = ContextTrigger::new("test-trigger")
            .with_condition(TriggerCondition::file_modified("*.rs"))
            .with_cooldown(Duration::from_secs(60))
            .with_max_fires(5);

        assert_eq!(trigger.name, "test-trigger");
        assert_eq!(trigger.conditions.len(), 1);
        assert_eq!(trigger.cooldown, Some(Duration::from_secs(60)));
        assert_eq!(trigger.max_fires, Some(5));
        assert!(trigger.can_fire());
    }

    #[test]
    fn test_trigger_fire_count() {
        let mut trigger = ContextTrigger::new("test").with_max_fires(2);

        assert!(trigger.can_fire());
        trigger.record_fire();
        assert!(trigger.can_fire());
        trigger.record_fire();
        assert!(!trigger.can_fire()); // Max fires reached
    }

    #[test]
    fn test_event_matching() {
        let event = ContextEvent::FileModified {
            path: "src/main.rs".into(),
        };

        let cond1 = TriggerCondition::file_modified("*.rs");
        assert!(event.matches(&cond1));

        let cond2 = TriggerCondition::file_modified("*.ts");
        assert!(!event.matches(&cond2));
    }

    #[test]
    fn test_command_event_matching() {
        let success_event = ContextEvent::CommandExecuted {
            command: "cargo test".into(),
            exit_code: 0,
            duration: Duration::from_secs(5),
        };

        let failure_event = ContextEvent::CommandExecuted {
            command: "cargo test".into(),
            exit_code: 1,
            duration: Duration::from_secs(5),
        };

        let executed_cond = TriggerCondition::command_executed("cargo test");
        let failed_cond = TriggerCondition::command_failed("cargo test");

        assert!(success_event.matches(&executed_cond));
        assert!(!success_event.matches(&failed_cond));
        assert!(!failure_event.matches(&executed_cond));
        assert!(failure_event.matches(&failed_cond));
    }

    #[test]
    fn test_glob_matching() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("src/*.rs", "src/main.rs"));
        assert!(!glob_match("*.rs", "main.ts"));
    }
}
