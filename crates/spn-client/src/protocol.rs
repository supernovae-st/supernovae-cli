//! Protocol types for daemon communication.
//!
//! The protocol uses length-prefixed JSON over Unix sockets.
//!
//! ## Wire Format
//!
//! ```text
//! [4 bytes: message length (big-endian u32)][JSON payload]
//! ```
//!
//! ## Protocol Versioning
//!
//! The protocol version is exchanged during the initial PING/PONG handshake.
//! This allows clients and daemons to detect incompatible versions early.
//!
//! - `protocol_version`: Integer version for wire protocol changes
//! - `version`: CLI version string for display purposes
//!
//! When the protocol version doesn't match, clients should warn and may
//! fall back to environment variables.
//!
//! ## Example
//!
//! Request:
//! ```json
//! { "cmd": "GET_SECRET", "provider": "anthropic" }
//! ```
//!
//! Response:
//! ```json
//! { "ok": true, "secret": "sk-ant-..." }
//! ```

use serde::{Deserialize, Serialize};
use spn_core::{LoadConfig, ModelInfo, PullProgress, RunningModel};

// ============================================================================
// JOB TYPES (IPC-friendly versions)
// ============================================================================

/// Job state in the scheduler (IPC version).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IpcJobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for IpcJobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcJobState::Pending => write!(f, "pending"),
            IpcJobState::Running => write!(f, "running"),
            IpcJobState::Completed => write!(f, "completed"),
            IpcJobState::Failed => write!(f, "failed"),
            IpcJobState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Job status for IPC responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcJobStatus {
    /// Job ID (8-char UUID prefix)
    pub id: String,
    /// Workflow path
    pub workflow: String,
    /// Current state
    pub state: IpcJobState,
    /// Optional job name
    pub name: Option<String>,
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Output from the workflow (if completed)
    pub output: Option<String>,
    /// Creation timestamp (Unix epoch millis)
    pub created_at: u64,
    /// Start timestamp (Unix epoch millis, if started)
    pub started_at: Option<u64>,
    /// End timestamp (Unix epoch millis, if finished)
    pub ended_at: Option<u64>,
}

/// Scheduler statistics for IPC responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcSchedulerStats {
    /// Total jobs (all states)
    pub total: usize,
    /// Pending jobs
    pub pending: usize,
    /// Currently running jobs
    pub running: usize,
    /// Completed jobs
    pub completed: usize,
    /// Failed jobs
    pub failed: usize,
    /// Cancelled jobs
    pub cancelled: usize,
    /// Whether nika binary is available
    pub has_nika: bool,
}

/// Progress update for model operations (pull, load, delete).
///
/// Used for streaming progress from daemon to CLI during long-running operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProgress {
    /// Current status message (e.g., "downloading", "verifying", "extracting")
    pub status: String,
    /// Bytes/units completed (optional for indeterminate operations)
    pub completed: Option<u64>,
    /// Total bytes/units (optional for indeterminate operations)
    pub total: Option<u64>,
    /// Model digest (for pull operations)
    pub digest: Option<String>,
}

impl ModelProgress {
    /// Calculate completion percentage (0.0 - 100.0).
    /// Returns None if total is unknown or zero.
    pub fn percentage(&self) -> Option<f64> {
        match (self.completed, self.total) {
            (Some(completed), Some(total)) if total > 0 => {
                Some((completed as f64 / total as f64) * 100.0)
            }
            _ => None,
        }
    }

    /// Create a new indeterminate progress (spinner mode).
    pub fn indeterminate(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            completed: None,
            total: None,
            digest: None,
        }
    }

    /// Create a determinate progress (progress bar mode).
    pub fn determinate(status: impl Into<String>, completed: u64, total: u64) -> Self {
        Self {
            status: status.into(),
            completed: Some(completed),
            total: Some(total),
            digest: None,
        }
    }

    /// Create from PullProgress (from spn_core/spn_ollama).
    pub fn from_pull_progress(p: &PullProgress) -> Self {
        Self {
            status: p.status.clone(),
            completed: Some(p.completed),
            total: Some(p.total),
            digest: None, // PullProgress doesn't have digest field
        }
    }
}

/// Current protocol version.
/// - Adding required fields to requests/responses
/// - Changing the serialization format
/// - Removing commands or response variants
///
/// Do NOT increment for:
/// - Adding new optional fields
/// - Adding new commands (backwards compatible)
pub const PROTOCOL_VERSION: u32 = 1;

/// Default protocol version for backwards compatibility.
/// Old daemons that don't send protocol_version are assumed to be v0.
fn default_protocol_version() -> u32 {
    0
}

/// Request sent to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum Request {
    /// Ping the daemon to check it's alive.
    #[serde(rename = "PING")]
    Ping,

    /// Get a secret for a provider.
    #[serde(rename = "GET_SECRET")]
    GetSecret { provider: String },

    /// Check if a secret exists.
    #[serde(rename = "HAS_SECRET")]
    HasSecret { provider: String },

    /// List all available providers.
    #[serde(rename = "LIST_PROVIDERS")]
    ListProviders,

    // ==================== Model Commands ====================
    /// List all installed models.
    #[serde(rename = "MODEL_LIST")]
    ModelList,

    /// Pull/download a model.
    #[serde(rename = "MODEL_PULL")]
    ModelPull { name: String },

    /// Load a model into memory.
    #[serde(rename = "MODEL_LOAD")]
    ModelLoad {
        name: String,
        #[serde(default)]
        config: Option<LoadConfig>,
    },

    /// Unload a model from memory.
    #[serde(rename = "MODEL_UNLOAD")]
    ModelUnload { name: String },

    /// Get status of running models.
    #[serde(rename = "MODEL_STATUS")]
    ModelStatus,

    /// Delete a model.
    #[serde(rename = "MODEL_DELETE")]
    ModelDelete { name: String },

    /// Run inference on a model.
    #[serde(rename = "MODEL_RUN")]
    ModelRun {
        /// Model name (e.g., llama3.2)
        model: String,
        /// User prompt
        prompt: String,
        /// System prompt (optional)
        #[serde(default)]
        system: Option<String>,
        /// Temperature (0.0 - 2.0)
        #[serde(default)]
        temperature: Option<f32>,
        /// Enable streaming (not yet supported via IPC)
        #[serde(default)]
        stream: bool,
    },

    // ==================== Job Commands ====================
    /// Submit a workflow job for background execution.
    #[serde(rename = "JOB_SUBMIT")]
    JobSubmit {
        /// Path to workflow file
        workflow: String,
        /// Optional workflow arguments
        #[serde(default)]
        args: Vec<String>,
        /// Optional job name for display
        #[serde(default)]
        name: Option<String>,
        /// Job priority (higher = more urgent)
        #[serde(default)]
        priority: i32,
    },

    /// Get status of a specific job.
    #[serde(rename = "JOB_STATUS")]
    JobStatus {
        /// Job ID (8-character short UUID)
        job_id: String,
    },

    /// List all jobs (optionally filtered by state).
    #[serde(rename = "JOB_LIST")]
    JobList {
        /// Filter by state (pending, running, completed, failed, cancelled)
        #[serde(default)]
        state: Option<String>,
    },

    /// Cancel a running or pending job.
    #[serde(rename = "JOB_CANCEL")]
    JobCancel {
        /// Job ID to cancel
        job_id: String,
    },

    /// Get scheduler statistics.
    #[serde(rename = "JOB_STATS")]
    JobStats,
}

/// Response from the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// Successful ping response with version info.
    Pong {
        /// Protocol version for compatibility checking.
        /// Clients should verify this matches PROTOCOL_VERSION.
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
        /// CLI version string for display.
        version: String,
    },

    /// Secret value response.
    ///
    /// # Security Note
    ///
    /// The secret is transmitted as plain JSON over the Unix socket. This is secure because:
    /// - Unix socket requires peer credential verification (same UID only)
    /// - Socket permissions are 0600 (owner-only)
    /// - Connection is local-only (no network exposure)
    Secret { value: String },

    /// Secret existence check response.
    Exists { exists: bool },

    /// Provider list response.
    Providers { providers: Vec<String> },

    // ==================== Model Responses ====================
    /// List of installed models.
    Models { models: Vec<ModelInfo> },

    /// List of currently running/loaded models.
    RunningModels { running: Vec<RunningModel> },

    /// Generic success response.
    Success { success: bool },

    /// Model run result with generated content.
    ModelRunResult {
        /// Generated content from the model.
        content: String,
        /// Optional stats (tokens_per_second, etc.)
        #[serde(default)]
        stats: Option<serde_json::Value>,
    },

    /// Error response.
    Error { message: String },

    // ==================== Streaming Responses ====================
    /// Progress update for model operations (streaming).
    Progress {
        /// Progress details
        progress: ModelProgress,
    },

    /// End of stream marker.
    StreamEnd {
        /// Whether the operation succeeded
        success: bool,
        /// Error message if failed
        #[serde(default)]
        error: Option<String>,
    },

    // ==================== Job Responses ====================
    /// Job submitted response with initial status.
    JobSubmitted {
        /// The job status
        job: IpcJobStatus,
    },

    /// Single job status response.
    JobStatusResult {
        /// The job status (None if job not found)
        job: Option<IpcJobStatus>,
    },

    /// Job list response.
    JobListResult {
        /// List of jobs
        jobs: Vec<IpcJobStatus>,
    },

    /// Job cancelled response.
    JobCancelled {
        /// Whether cancellation succeeded
        cancelled: bool,
        /// Job ID that was cancelled
        job_id: String,
    },

    /// Scheduler statistics response.
    JobStatsResult {
        /// Scheduler stats
        stats: IpcSchedulerStats,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let ping = Request::Ping;
        let json = serde_json::to_string(&ping).unwrap();
        assert_eq!(json, r#"{"cmd":"PING"}"#);

        let get_secret = Request::GetSecret {
            provider: "anthropic".to_string(),
        };
        let json = serde_json::to_string(&get_secret).unwrap();
        assert_eq!(json, r#"{"cmd":"GET_SECRET","provider":"anthropic"}"#);

        let has_secret = Request::HasSecret {
            provider: "openai".to_string(),
        };
        let json = serde_json::to_string(&has_secret).unwrap();
        assert_eq!(json, r#"{"cmd":"HAS_SECRET","provider":"openai"}"#);

        let list = Request::ListProviders;
        let json = serde_json::to_string(&list).unwrap();
        assert_eq!(json, r#"{"cmd":"LIST_PROVIDERS"}"#);
    }

    #[test]
    fn test_response_deserialization() {
        // Pong with protocol version
        let json = r#"{"protocol_version":1,"version":"0.14.2"}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(
            matches!(response, Response::Pong { protocol_version, version }
                if protocol_version == 1 && version == "0.14.2")
        );

        // Pong without protocol version (backwards compatibility)
        let json = r#"{"version":"0.9.0"}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(
            matches!(response, Response::Pong { protocol_version, version }
                if protocol_version == 0 && version == "0.9.0")
        );

        // Secret
        let json = r#"{"value":"sk-test-123"}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response, Response::Secret { value } if value == "sk-test-123"));

        // Exists
        let json = r#"{"exists":true}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response, Response::Exists { exists } if exists));

        // Providers
        let json = r#"{"providers":["anthropic","openai"]}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(
            matches!(response, Response::Providers { providers } if providers == vec!["anthropic", "openai"])
        );

        // Error
        let json = r#"{"message":"Not found"}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response, Response::Error { message } if message == "Not found"));
    }

    #[test]
    fn test_model_progress_serialization() {
        let progress = ModelProgress {
            status: "downloading".into(),
            completed: Some(50),
            total: Some(100),
            digest: Some("sha256:abc123".into()),
        };

        let json = serde_json::to_string(&progress).unwrap();
        let parsed: ModelProgress = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.status, "downloading");
        assert_eq!(parsed.completed, Some(50));
        assert_eq!(parsed.total, Some(100));
    }

    #[test]
    fn test_model_progress_percentage() {
        let progress = ModelProgress {
            status: "downloading".into(),
            completed: Some(75),
            total: Some(100),
            digest: None,
        };

        assert_eq!(progress.percentage(), Some(75.0));

        let no_total = ModelProgress {
            status: "starting".into(),
            completed: None,
            total: None,
            digest: None,
        };

        assert_eq!(no_total.percentage(), None);
    }

    #[test]
    fn test_model_progress_constructors() {
        let indeterminate = ModelProgress::indeterminate("loading");
        assert_eq!(indeterminate.status, "loading");
        assert!(indeterminate.percentage().is_none());

        let determinate = ModelProgress::determinate("downloading", 50, 100);
        assert_eq!(determinate.percentage(), Some(50.0));
    }

    #[test]
    fn test_response_progress_variant() {
        let progress = ModelProgress::determinate("downloading", 50, 100);
        let response = Response::Progress { progress };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("downloading"));
    }

    #[test]
    fn test_response_stream_end_variant() {
        let success_response = Response::StreamEnd {
            success: true,
            error: None,
        };
        let json = serde_json::to_string(&success_response).unwrap();
        assert!(json.contains("success"));

        let error_response = Response::StreamEnd {
            success: false,
            error: Some("Connection lost".into()),
        };
        let json = serde_json::to_string(&error_response).unwrap();
        assert!(json.contains("Connection lost"));
    }

    // ==================== Job Protocol Tests ====================

    #[test]
    fn test_job_request_serialization() {
        let submit = Request::JobSubmit {
            workflow: "/path/to/workflow.yaml".into(),
            args: vec!["--verbose".into()],
            name: Some("Test Job".into()),
            priority: 5,
        };
        let json = serde_json::to_string(&submit).unwrap();
        assert!(json.contains("JOB_SUBMIT"));
        assert!(json.contains("workflow.yaml"));

        let status = Request::JobStatus {
            job_id: "abc12345".into(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("JOB_STATUS"));
        assert!(json.contains("abc12345"));

        let list = Request::JobList { state: None };
        let json = serde_json::to_string(&list).unwrap();
        assert!(json.contains("JOB_LIST"));

        let cancel = Request::JobCancel {
            job_id: "def67890".into(),
        };
        let json = serde_json::to_string(&cancel).unwrap();
        assert!(json.contains("JOB_CANCEL"));

        let stats = Request::JobStats;
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("JOB_STATS"));
    }

    #[test]
    fn test_ipc_job_state_serialization() {
        assert_eq!(
            serde_json::to_string(&IpcJobState::Pending).unwrap(),
            r#""pending""#
        );
        assert_eq!(
            serde_json::to_string(&IpcJobState::Running).unwrap(),
            r#""running""#
        );
        assert_eq!(
            serde_json::to_string(&IpcJobState::Completed).unwrap(),
            r#""completed""#
        );
        assert_eq!(
            serde_json::to_string(&IpcJobState::Failed).unwrap(),
            r#""failed""#
        );
        assert_eq!(
            serde_json::to_string(&IpcJobState::Cancelled).unwrap(),
            r#""cancelled""#
        );
    }

    #[test]
    fn test_ipc_job_status_serialization() {
        let status = IpcJobStatus {
            id: "abc12345".into(),
            workflow: "/path/to/test.yaml".into(),
            state: IpcJobState::Running,
            name: Some("Test Job".into()),
            progress: 50,
            error: None,
            output: None,
            created_at: 1710000000000,
            started_at: Some(1710000001000),
            ended_at: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("abc12345"));
        assert!(json.contains("running"));
        assert!(json.contains("Test Job"));
    }

    #[test]
    fn test_ipc_scheduler_stats_serialization() {
        let stats = IpcSchedulerStats {
            total: 10,
            pending: 2,
            running: 3,
            completed: 4,
            failed: 1,
            cancelled: 0,
            has_nika: true,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: IpcSchedulerStats = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.total, 10);
        assert_eq!(parsed.running, 3);
        assert!(parsed.has_nika);
    }

    #[test]
    fn test_job_response_variants() {
        // JobSubmitted
        let status = IpcJobStatus {
            id: "abc12345".into(),
            workflow: "/test.yaml".into(),
            state: IpcJobState::Pending,
            name: None,
            progress: 0,
            error: None,
            output: None,
            created_at: 1710000000000,
            started_at: None,
            ended_at: None,
        };
        let response = Response::JobSubmitted { job: status };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("abc12345"));

        // JobCancelled
        let response = Response::JobCancelled {
            cancelled: true,
            job_id: "def67890".into(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("cancelled"));
        assert!(json.contains("def67890"));
    }
}
