//! Job types and states.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

/// Unique job identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(Uuid);

impl JobId {
    /// Create a new random job ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

/// Job state in the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    /// Job is queued, waiting to run.
    Pending,
    /// Job is currently running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed with an error.
    Failed,
    /// Job was cancelled.
    Cancelled,
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobState::Pending => write!(f, "pending"),
            JobState::Running => write!(f, "running"),
            JobState::Completed => write!(f, "completed"),
            JobState::Failed => write!(f, "failed"),
            JobState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Job definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier.
    pub id: JobId,
    /// Workflow file path.
    pub workflow: PathBuf,
    /// Optional workflow arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Job priority (higher = more urgent).
    #[serde(default)]
    pub priority: i32,
    /// Creation timestamp.
    pub created_at: SystemTime,
    /// Optional job name for display.
    pub name: Option<String>,
}

impl Job {
    /// Create a new job for a workflow.
    pub fn new(workflow: PathBuf) -> Self {
        Self {
            id: JobId::new(),
            workflow,
            args: Vec::new(),
            priority: 0,
            created_at: SystemTime::now(),
            name: None,
        }
    }

    /// Set job arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set job priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set job name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Job status with execution details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    /// The job definition.
    pub job: Job,
    /// Current state.
    pub state: JobState,
    /// Start timestamp (if started).
    pub started_at: Option<SystemTime>,
    /// End timestamp (if finished).
    pub ended_at: Option<SystemTime>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Output from the workflow.
    pub output: Option<String>,
    /// Progress percentage (0-100).
    pub progress: u8,
}

impl JobStatus {
    /// Create a new pending job status.
    pub fn new(job: Job) -> Self {
        Self {
            job,
            state: JobState::Pending,
            started_at: None,
            ended_at: None,
            error: None,
            output: None,
            progress: 0,
        }
    }

    /// Mark job as running.
    pub fn start(&mut self) {
        self.state = JobState::Running;
        self.started_at = Some(SystemTime::now());
    }

    /// Mark job as completed.
    pub fn complete(&mut self, output: Option<String>) {
        self.state = JobState::Completed;
        self.ended_at = Some(SystemTime::now());
        self.output = output;
        self.progress = 100;
    }

    /// Mark job as failed.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.state = JobState::Failed;
        self.ended_at = Some(SystemTime::now());
        self.error = Some(error.into());
    }

    /// Mark job as cancelled.
    pub fn cancel(&mut self) {
        self.state = JobState::Cancelled;
        self.ended_at = Some(SystemTime::now());
    }

    /// Update progress.
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = progress.min(100);
    }

    /// Check if job is terminal (completed/failed/cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            JobState::Completed | JobState::Failed | JobState::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_id_display() {
        let id = JobId::new();
        assert_eq!(id.to_string().len(), 8);
    }

    #[test]
    fn test_job_creation() {
        let job = Job::new(PathBuf::from("test.yaml"))
            .with_name("Test Job")
            .with_priority(5)
            .with_args(vec!["--verbose".into()]);

        assert_eq!(job.name, Some("Test Job".into()));
        assert_eq!(job.priority, 5);
        assert_eq!(job.args.len(), 1);
    }

    #[test]
    fn test_job_status_lifecycle() {
        let job = Job::new(PathBuf::from("test.yaml"));
        let mut status = JobStatus::new(job);

        assert_eq!(status.state, JobState::Pending);
        assert!(status.started_at.is_none());

        status.start();
        assert_eq!(status.state, JobState::Running);
        assert!(status.started_at.is_some());

        status.complete(Some("Success".into()));
        assert_eq!(status.state, JobState::Completed);
        assert!(status.is_terminal());
    }
}
