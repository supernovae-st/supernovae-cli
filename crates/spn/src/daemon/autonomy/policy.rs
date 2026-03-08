//! Autonomy policies and approval levels.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Level of autonomy for the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AutonomyLevel {
    /// Fully manual: all actions require approval.
    Manual = 0,
    /// Assisted: suggestions but no auto-execution.
    Assisted = 1,
    /// Semi-autonomous: safe actions auto-execute, others need approval.
    SemiAutonomous = 2,
    /// Autonomous: most actions auto-execute, major ones need approval.
    Autonomous = 3,
    /// Full: all actions auto-execute, human notified.
    Full = 4,
}

impl AutonomyLevel {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AutonomyLevel::Manual => "manual",
            AutonomyLevel::Assisted => "assisted",
            AutonomyLevel::SemiAutonomous => "semi_autonomous",
            AutonomyLevel::Autonomous => "autonomous",
            AutonomyLevel::Full => "full",
        }
    }

    /// Get description.
    pub fn description(&self) -> &'static str {
        match self {
            AutonomyLevel::Manual => "All actions require explicit approval",
            AutonomyLevel::Assisted => "Suggestions only, no auto-execution",
            AutonomyLevel::SemiAutonomous => "Safe actions auto-execute, others need approval",
            AutonomyLevel::Autonomous => "Most actions auto-execute, major changes need approval",
            AutonomyLevel::Full => "All actions auto-execute, human is notified",
        }
    }
}

impl std::fmt::Display for AutonomyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Approval level required for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalLevel {
    /// Automatically approved (safe action).
    Auto,
    /// Notify user but proceed.
    Notify,
    /// Optional approval (timeout auto-approves).
    Optional,
    /// Required approval (must wait for user).
    Required,
    /// Blocked (cannot proceed even with approval).
    Blocked,
}

impl ApprovalLevel {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalLevel::Auto => "auto",
            ApprovalLevel::Notify => "notify",
            ApprovalLevel::Optional => "optional",
            ApprovalLevel::Required => "required",
            ApprovalLevel::Blocked => "blocked",
        }
    }

    /// Check if this requires waiting for user.
    pub fn requires_wait(&self) -> bool {
        matches!(self, ApprovalLevel::Required | ApprovalLevel::Blocked)
    }
}

impl std::fmt::Display for ApprovalLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Policy for autonomous operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyPolicy {
    /// Current autonomy level.
    pub level: AutonomyLevel,
    /// Maximum tokens per task.
    pub max_tokens_per_task: u32,
    /// Maximum concurrent tasks.
    pub max_concurrent_tasks: usize,
    /// Maximum agent depth.
    pub max_agent_depth: usize,
    /// Allowed operations.
    pub allowed_ops: HashSet<String>,
    /// Blocked operations.
    pub blocked_ops: HashSet<String>,
    /// Operations requiring approval.
    pub approval_ops: HashSet<String>,
    /// Allowed file patterns.
    pub allowed_files: Vec<String>,
    /// Blocked file patterns.
    pub blocked_files: Vec<String>,
    /// Whether to allow network access.
    pub allow_network: bool,
    /// Whether to allow file writes.
    pub allow_file_writes: bool,
    /// Whether to allow command execution.
    pub allow_commands: bool,
    /// Auto-approve timeout (seconds).
    pub auto_approve_timeout: Option<u64>,
}

impl Default for AutonomyPolicy {
    fn default() -> Self {
        Self {
            level: AutonomyLevel::SemiAutonomous,
            max_tokens_per_task: 100_000,
            max_concurrent_tasks: 5,
            max_agent_depth: 3,
            allowed_ops: HashSet::new(),
            blocked_ops: HashSet::from_iter([
                "rm -rf".to_string(),
                "git push --force".to_string(),
                "git reset --hard".to_string(),
            ]),
            approval_ops: HashSet::from_iter([
                "git commit".to_string(),
                "git push".to_string(),
                "cargo publish".to_string(),
            ]),
            allowed_files: vec!["*".to_string()],
            blocked_files: vec![
                "~/.ssh/*".to_string(),
                "~/.gnupg/*".to_string(),
                "*.pem".to_string(),
                "*.key".to_string(),
            ],
            allow_network: true,
            allow_file_writes: true,
            allow_commands: true,
            auto_approve_timeout: Some(30),
        }
    }
}

impl AutonomyPolicy {
    /// Create a minimal (manual) policy.
    pub fn manual() -> Self {
        Self {
            level: AutonomyLevel::Manual,
            allow_file_writes: false,
            allow_commands: false,
            auto_approve_timeout: None,
            ..Default::default()
        }
    }

    /// Create a full autonomy policy.
    pub fn full() -> Self {
        Self {
            level: AutonomyLevel::Full,
            max_tokens_per_task: 500_000,
            max_concurrent_tasks: 10,
            ..Default::default()
        }
    }

    /// Check if an operation is allowed.
    pub fn is_operation_allowed(&self, op: &str) -> bool {
        if self.blocked_ops.iter().any(|b| op.contains(b)) {
            return false;
        }
        if !self.allowed_ops.is_empty() && !self.allowed_ops.iter().any(|a| op.contains(a)) {
            return false;
        }
        true
    }

    /// Get approval level for an operation.
    pub fn approval_level_for(&self, op: &str) -> ApprovalLevel {
        // Check blocked first
        if self.blocked_ops.iter().any(|b| op.contains(b)) {
            return ApprovalLevel::Blocked;
        }

        // Check if requires approval
        if self.approval_ops.iter().any(|a| op.contains(a)) {
            return match self.level {
                AutonomyLevel::Manual | AutonomyLevel::Assisted => ApprovalLevel::Required,
                AutonomyLevel::SemiAutonomous => ApprovalLevel::Required,
                AutonomyLevel::Autonomous => ApprovalLevel::Optional,
                AutonomyLevel::Full => ApprovalLevel::Notify,
            };
        }

        // Default based on level
        match self.level {
            AutonomyLevel::Manual => ApprovalLevel::Required,
            AutonomyLevel::Assisted => ApprovalLevel::Required,
            AutonomyLevel::SemiAutonomous => ApprovalLevel::Optional,
            AutonomyLevel::Autonomous => ApprovalLevel::Auto,
            AutonomyLevel::Full => ApprovalLevel::Auto,
        }
    }

    /// Check if file access is allowed.
    pub fn is_file_allowed(&self, path: &str) -> bool {
        // Check blocked first
        for pattern in &self.blocked_files {
            if glob_match(pattern, path) {
                return false;
            }
        }

        // Check allowed
        for pattern in &self.allowed_files {
            if glob_match(pattern, path) {
                return true;
            }
        }

        false
    }

    /// Validate an action against policy.
    pub fn validate(
        &self,
        action: &str,
        resources: &[String],
    ) -> Result<ApprovalLevel, PolicyViolation> {
        // Check operation
        if !self.is_operation_allowed(action) {
            return Err(PolicyViolation::OperationBlocked(action.to_string()));
        }

        // Check files
        for resource in resources {
            if !self.is_file_allowed(resource) {
                return Err(PolicyViolation::FileBlocked(resource.clone()));
            }
        }

        // Check permissions
        if action.starts_with("write:") && !self.allow_file_writes {
            return Err(PolicyViolation::FileWritesDisabled);
        }

        if action.starts_with("exec:") && !self.allow_commands {
            return Err(PolicyViolation::CommandsDisabled);
        }

        if action.starts_with("network:") && !self.allow_network {
            return Err(PolicyViolation::NetworkDisabled);
        }

        Ok(self.approval_level_for(action))
    }
}

/// Simple glob matching.
fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }
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

/// Policy violation error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PolicyViolation {
    /// Operation is blocked.
    #[error("Operation blocked: {0}")]
    OperationBlocked(String),

    /// File access is blocked.
    #[error("File access blocked: {0}")]
    FileBlocked(String),

    /// File writes are disabled.
    #[error("File writes are disabled")]
    FileWritesDisabled,

    /// Commands are disabled.
    #[error("Command execution is disabled")]
    CommandsDisabled,

    /// Network access is disabled.
    #[error("Network access is disabled")]
    NetworkDisabled,

    /// Token limit exceeded.
    #[error("Token limit exceeded: {used}/{max}")]
    TokenLimitExceeded { used: u32, max: u32 },

    /// Concurrent task limit exceeded.
    #[error("Concurrent task limit exceeded")]
    ConcurrentLimitExceeded,

    /// Agent depth limit exceeded.
    #[error("Agent depth limit exceeded")]
    AgentDepthExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autonomy_levels() {
        assert!(AutonomyLevel::Full > AutonomyLevel::Autonomous);
        assert!(AutonomyLevel::Autonomous > AutonomyLevel::SemiAutonomous);
        assert!(AutonomyLevel::SemiAutonomous > AutonomyLevel::Assisted);
        assert!(AutonomyLevel::Assisted > AutonomyLevel::Manual);
    }

    #[test]
    fn test_approval_level_wait() {
        assert!(ApprovalLevel::Required.requires_wait());
        assert!(ApprovalLevel::Blocked.requires_wait());
        assert!(!ApprovalLevel::Auto.requires_wait());
        assert!(!ApprovalLevel::Notify.requires_wait());
    }

    #[test]
    fn test_default_policy() {
        let policy = AutonomyPolicy::default();
        assert_eq!(policy.level, AutonomyLevel::SemiAutonomous);
        assert!(policy.allow_file_writes);
        assert!(policy.allow_commands);
    }

    #[test]
    fn test_blocked_operations() {
        let policy = AutonomyPolicy::default();

        assert!(!policy.is_operation_allowed("rm -rf /"));
        assert!(!policy.is_operation_allowed("git push --force origin main"));
        assert!(policy.is_operation_allowed("cargo test"));
    }

    #[test]
    fn test_approval_ops() {
        let policy = AutonomyPolicy::default();

        let level = policy.approval_level_for("git commit -m 'test'");
        assert_eq!(level, ApprovalLevel::Required);

        let level = policy.approval_level_for("cargo test");
        assert_eq!(level, ApprovalLevel::Optional);
    }

    #[test]
    fn test_blocked_files() {
        let policy = AutonomyPolicy::default();

        assert!(!policy.is_file_allowed("~/.ssh/id_rsa"));
        assert!(!policy.is_file_allowed("secrets.pem"));
        assert!(policy.is_file_allowed("src/main.rs"));
    }

    #[test]
    fn test_policy_validation() {
        let policy = AutonomyPolicy::default();

        // Allowed operation
        let result = policy.validate("cargo test", &[]);
        assert!(result.is_ok());

        // Blocked operation
        let result = policy.validate("rm -rf /tmp", &[]);
        assert!(matches!(result, Err(PolicyViolation::OperationBlocked(_))));

        // Blocked file
        let result = policy.validate("read", &["~/.ssh/id_rsa".into()]);
        assert!(matches!(result, Err(PolicyViolation::FileBlocked(_))));
    }

    #[test]
    fn test_manual_policy() {
        let policy = AutonomyPolicy::manual();

        assert_eq!(policy.level, AutonomyLevel::Manual);
        assert!(!policy.allow_file_writes);
        assert!(!policy.allow_commands);

        let result = policy.validate("write: test.txt", &[]);
        assert!(matches!(result, Err(PolicyViolation::FileWritesDisabled)));
    }

    #[test]
    fn test_full_policy() {
        let policy = AutonomyPolicy::full();

        assert_eq!(policy.level, AutonomyLevel::Full);
        assert_eq!(policy.max_tokens_per_task, 500_000);

        let level = policy.approval_level_for("git commit");
        assert_eq!(level, ApprovalLevel::Notify);
    }
}
