//! Request handler for daemon commands.

use secrecy::ExposeSecret;
use spn_client::{
    ChatMessage, ChatOptions, ForeignMcpInfo, IpcJobState, IpcJobStatus, IpcSchedulerStats,
    RecentProjectInfo, Request, Response, WatcherStatusInfo, PROTOCOL_VERSION,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Timeout for model pull operations (30 minutes).
/// Large models (70B+) can take 15-20 minutes on fast connections.
const MODEL_PULL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

use super::foreign::{ForeignScope, ForeignTracker};
use super::jobs::{Job, JobScheduler, JobState, JobStatus};
use super::recent::RecentProjects;
use super::{ModelManager, SecretManager};

/// Handles incoming daemon requests.
pub struct RequestHandler {
    /// Secret manager
    secrets: Arc<SecretManager>,

    /// Model manager
    models: Arc<ModelManager>,

    /// Job scheduler
    jobs: Arc<JobScheduler>,

    /// Daemon version
    version: String,
}

impl RequestHandler {
    /// Create a new request handler.
    pub fn new(
        secrets: Arc<SecretManager>,
        models: Arc<ModelManager>,
        jobs: Arc<JobScheduler>,
    ) -> Self {
        Self {
            secrets,
            models,
            jobs,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Handle a request and return a response.
    pub async fn handle(&self, request: Request) -> Response {
        debug!("Handling request: {:?}", request);

        match request {
            Request::Ping => self.handle_ping(),
            Request::GetSecret { provider } => self.handle_get_secret(&provider).await,
            Request::HasSecret { provider } => self.handle_has_secret(&provider).await,
            Request::ListProviders => self.handle_list_providers().await,

            // Model commands
            Request::ModelList => self.handle_model_list().await,
            Request::ModelPull { name } => self.handle_model_pull(&name).await,
            Request::ModelLoad { name, config } => self.handle_model_load(&name, config).await,
            Request::ModelUnload { name } => self.handle_model_unload(&name).await,
            Request::ModelStatus => self.handle_model_status().await,
            Request::ModelDelete { name } => self.handle_model_delete(&name).await,
            Request::ModelRun {
                model,
                prompt,
                system,
                temperature,
                stream: _,
            } => {
                self.handle_model_run(&model, &prompt, system, temperature)
                    .await
            }

            // Job commands
            Request::JobSubmit {
                workflow,
                args,
                name,
                priority,
            } => {
                self.handle_job_submit(&workflow, args, name, priority)
                    .await
            }
            Request::JobStatus { job_id } => self.handle_job_status(&job_id).await,
            Request::JobList { state } => self.handle_job_list(state.as_deref()).await,
            Request::JobCancel { job_id } => self.handle_job_cancel(&job_id).await,
            Request::JobStats => self.handle_job_stats().await,

            // Watcher commands
            Request::WatcherStatus => self.handle_watcher_status().await,
        }
    }

    fn handle_ping(&self) -> Response {
        Response::Pong {
            protocol_version: PROTOCOL_VERSION,
            version: self.version.clone(),
        }
    }

    async fn handle_get_secret(&self, provider: &str) -> Response {
        match self.secrets.get_cached(provider).await {
            Some(secret) => {
                // NOTE: Security consideration - the secret is exposed as plain String in the
                // Response for JSON serialization over IPC. This is acceptable because:
                // 1. Unix socket uses peer credential verification (same UID only)
                // 2. Socket has 0600 permissions (owner-only access)
                // 3. The secret exposure is short-lived (serialized immediately, then dropped)
                // A future protocol version could use encrypted payloads if needed.
                Response::Secret {
                    value: secret.expose_secret().to_string(),
                }
            }
            None => {
                warn!("Secret not found for provider: {}", provider);
                Response::Error {
                    message: format!("Secret not found for provider: {}", provider),
                }
            }
        }
    }

    async fn handle_has_secret(&self, provider: &str) -> Response {
        let exists = self.secrets.has_cached(provider).await;
        Response::Exists { exists }
    }

    async fn handle_list_providers(&self) -> Response {
        let providers = self.secrets.list_cached().await;
        Response::Providers { providers }
    }

    // ==================== Model Handlers ====================

    async fn handle_model_list(&self) -> Response {
        match self.models.list_models().await {
            Ok(models) => Response::Models { models },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_pull(&self, name: &str) -> Response {
        match tokio::time::timeout(MODEL_PULL_TIMEOUT, self.models.pull(name)).await {
            Ok(Ok(())) => Response::Success { success: true },
            Ok(Err(e)) => Response::Error {
                message: e.to_string(),
            },
            Err(_) => Response::Error {
                message: format!(
                    "Model pull timed out after {} minutes. \
                     The model may be very large or there may be network issues. \
                     Try again or check your connection.",
                    MODEL_PULL_TIMEOUT.as_secs() / 60
                ),
            },
        }
    }

    async fn handle_model_load(
        &self,
        name: &str,
        config: Option<spn_client::LoadConfig>,
    ) -> Response {
        match self.models.load(name, config).await {
            Ok(()) => Response::Success { success: true },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_unload(&self, name: &str) -> Response {
        match self.models.unload(name).await {
            Ok(()) => Response::Success { success: true },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_status(&self) -> Response {
        match self.models.running_models().await {
            Ok(running) => Response::RunningModels { running },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_delete(&self, name: &str) -> Response {
        match self.models.delete(name).await {
            Ok(()) => Response::Success { success: true },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_run(
        &self,
        model: &str,
        prompt: &str,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> Response {
        // Build messages
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(prompt));

        // Build options
        let options = temperature.map(|temp| ChatOptions::new().with_temperature(temp));

        match self.models.chat(model, messages, options).await {
            Ok(response) => {
                // Build stats JSON
                let stats = serde_json::json!({
                    "tokens_per_second": response.tokens_per_second(),
                    "eval_count": response.eval_count,
                    "prompt_eval_count": response.prompt_eval_count,
                    "total_duration_ns": response.total_duration,
                });

                Response::ModelRunResult {
                    content: response.message.content,
                    stats: Some(stats),
                }
            }
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    // ==================== Job Handlers ====================

    async fn handle_job_submit(
        &self,
        workflow: &str,
        args: Vec<String>,
        name: Option<String>,
        priority: i32,
    ) -> Response {
        let path = PathBuf::from(workflow);

        // Validate workflow file exists
        if !path.exists() {
            return Response::Error {
                message: format!("Workflow file not found: {}", workflow),
            };
        }

        // Security: Canonicalize and validate path to prevent path traversal attacks.
        // We resolve the path to its absolute form (following symlinks) and verify:
        // 1. The path is canonical (no .. components after resolution)
        // 2. The file has a valid workflow extension (.nika.yaml or .yaml)
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return Response::Error {
                    message: format!("Invalid workflow path: {}", e),
                };
            }
        };

        // Validate file extension (must be a Nika workflow file)
        let extension_valid = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .map(|name| name.ends_with(".nika.yaml") || name.ends_with(".yaml"))
            .unwrap_or(false);

        if !extension_valid {
            return Response::Error {
                message: "Workflow file must have .nika.yaml or .yaml extension".to_string(),
            };
        }

        // Security: Validate canonical path is within allowed directories.
        // This prevents symlink attacks where an attacker creates a symlink to
        // a sensitive .yaml file outside the expected workflow locations.
        // Allowed directories:
        // 1. Current working directory and subdirectories
        // 2. User's ~/.spn/workflows/ directory
        // 3. User's home directory (for personal workflows)
        let allowed_bases: Vec<std::path::PathBuf> = vec![
            std::env::current_dir().ok(),
            dirs::home_dir().map(|h| h.join(".spn").join("workflows")),
            dirs::home_dir(),
        ]
        .into_iter()
        .flatten()
        .collect();

        let is_allowed = allowed_bases
            .iter()
            .any(|base| canonical.starts_with(base));

        if !is_allowed {
            return Response::Error {
                message: format!(
                    "Workflow path must be within current directory or home directory: {}",
                    canonical.display()
                ),
            };
        }

        // Create job with optional name and priority
        let mut job = Job::new(canonical)
            .with_args(args)
            .with_priority(priority);
        if let Some(n) = name {
            job = job.with_name(n);
        }

        // Submit to scheduler
        let status = self.jobs.submit(job).await;

        Response::JobSubmitted {
            job: job_status_to_ipc(&status),
        }
    }

    async fn handle_job_status(&self, job_id: &str) -> Response {
        // Parse job ID (first 8 chars of UUID)
        let job = self.find_job_by_short_id(job_id).await;

        Response::JobStatusResult {
            job: job.map(|s| job_status_to_ipc(&s)),
        }
    }

    async fn handle_job_list(&self, state_filter: Option<&str>) -> Response {
        let all_jobs = self.jobs.list().await;

        // Filter by state if specified
        let jobs: Vec<IpcJobStatus> = if let Some(state_str) = state_filter {
            let target_state = match state_str.to_lowercase().as_str() {
                "pending" => Some(JobState::Pending),
                "running" => Some(JobState::Running),
                "completed" => Some(JobState::Completed),
                "failed" => Some(JobState::Failed),
                "cancelled" => Some(JobState::Cancelled),
                _ => None,
            };

            if let Some(state) = target_state {
                all_jobs
                    .into_iter()
                    .filter(|s| s.state == state)
                    .map(|s| job_status_to_ipc(&s))
                    .collect()
            } else {
                // Invalid state filter, return all
                all_jobs.iter().map(job_status_to_ipc).collect()
            }
        } else {
            all_jobs.iter().map(job_status_to_ipc).collect()
        };

        Response::JobListResult { jobs }
    }

    async fn handle_job_cancel(&self, job_id: &str) -> Response {
        // Find and cancel job
        if let Some(status) = self.find_job_by_short_id(job_id).await {
            let cancelled = self.jobs.cancel(&status.job.id).await;
            Response::JobCancelled {
                cancelled,
                job_id: job_id.to_string(),
            }
        } else {
            Response::JobCancelled {
                cancelled: false,
                job_id: job_id.to_string(),
            }
        }
    }

    async fn handle_job_stats(&self) -> Response {
        let stats = self.jobs.stats().await;
        Response::JobStatsResult {
            stats: IpcSchedulerStats {
                total: stats.total,
                pending: stats.pending,
                running: stats.running,
                completed: stats.completed,
                failed: stats.failed,
                cancelled: stats.cancelled,
                has_nika: stats.has_nika,
            },
        }
    }

    // ==================== Watcher Handlers ====================

    async fn handle_watcher_status(&self) -> Response {
        // Load recent projects from persistent storage
        let recent = RecentProjects::load().unwrap_or_default();
        let recent_projects: Vec<RecentProjectInfo> = recent
            .projects
            .iter()
            .map(|p| RecentProjectInfo {
                path: p.path.display().to_string(),
                last_used: p.last_used.to_rfc3339(),
            })
            .collect();

        // Load foreign tracker from persistent storage
        let foreign = ForeignTracker::load().unwrap_or_default();
        let foreign_pending: Vec<ForeignMcpInfo> = foreign
            .pending
            .iter()
            .map(|f| ForeignMcpInfo {
                name: f.name.clone(),
                source: f.source.to_string().to_lowercase().replace(' ', "_"),
                scope: match &f.scope {
                    ForeignScope::Global => "global".into(),
                    ForeignScope::Project(p) => format!("project:{}", p.display()),
                },
                detected: f.detected.to_rfc3339(),
            })
            .collect();

        // Build watched paths list based on default config locations
        // The actual watcher state is in the server, but we can infer from config
        let mut watched_paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            // Global configs always watched
            watched_paths.push(home.join(".spn/mcp.yaml").display().to_string());
            watched_paths.push(home.join(".claude.json").display().to_string());
            watched_paths.push(home.join(".cursor/mcp.json").display().to_string());
            watched_paths.push(
                home.join(".codeium/windsurf/mcp_config.json")
                    .display()
                    .to_string(),
            );
        }

        // Add project-level configs from recent projects
        for proj in &recent.projects {
            watched_paths.push(proj.path.join(".mcp.json").display().to_string());
            watched_paths.push(proj.path.join(".cursor/mcp.json").display().to_string());
        }

        let watched_count = watched_paths.len();

        Response::WatcherStatusResult {
            status: WatcherStatusInfo {
                is_running: true, // If we're responding, daemon is running
                watched_count,
                watched_paths,
                debounce_ms: 500, // Matches DEBOUNCE_MS constant
                recent_projects,
                foreign_pending,
                foreign_ignored: foreign.ignored.clone(),
            },
        }
    }

    /// Find a job by short ID (first 8 chars of UUID).
    async fn find_job_by_short_id(&self, short_id: &str) -> Option<JobStatus> {
        let all_jobs = self.jobs.list().await;
        all_jobs
            .into_iter()
            .find(|s| s.job.id.to_string() == short_id)
    }
}

// ==================== Conversion Helpers ====================

/// Convert daemon JobStatus to IPC-friendly IpcJobStatus.
fn job_status_to_ipc(status: &JobStatus) -> IpcJobStatus {
    IpcJobStatus {
        id: status.job.id.to_string(),
        workflow: status.job.workflow.display().to_string(),
        state: job_state_to_ipc(status.state),
        name: status.job.name.clone(),
        progress: status.progress,
        error: status.error.clone(),
        output: status.output.clone(),
        created_at: system_time_to_millis(status.job.created_at),
        started_at: status.started_at.map(system_time_to_millis),
        ended_at: status.ended_at.map(system_time_to_millis),
    }
}

/// Convert daemon JobState to IPC JobState.
fn job_state_to_ipc(state: JobState) -> IpcJobState {
    match state {
        JobState::Pending => IpcJobState::Pending,
        JobState::Running => IpcJobState::Running,
        JobState::Completed => IpcJobState::Completed,
        JobState::Failed => IpcJobState::Failed,
        JobState::Cancelled => IpcJobState::Cancelled,
    }
}

/// Convert SystemTime to Unix epoch milliseconds.
fn system_time_to_millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::jobs::JobStore;
    use tempfile::tempdir;

    fn create_handler() -> RequestHandler {
        let secrets = Arc::new(SecretManager::new());
        let models = Arc::new(ModelManager::new());
        let dir = tempdir().expect("Failed to create tempdir");
        let store = Arc::new(JobStore::new(dir.path()));
        let jobs = Arc::new(JobScheduler::new(store));
        RequestHandler::new(secrets, models, jobs)
    }

    fn create_handler_with_secrets() -> (RequestHandler, Arc<SecretManager>) {
        let secrets = Arc::new(SecretManager::new());
        let models = Arc::new(ModelManager::new());
        let dir = tempdir().expect("Failed to create tempdir");
        let store = Arc::new(JobStore::new(dir.path()));
        let jobs = Arc::new(JobScheduler::new(store));
        let handler = RequestHandler::new(Arc::clone(&secrets), models, jobs);
        (handler, secrets)
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let handler = create_handler();

        let response = handler.handle(Request::Ping).await;

        match response {
            Response::Pong {
                protocol_version,
                version,
            } => {
                assert_eq!(protocol_version, PROTOCOL_VERSION);
                assert!(!version.is_empty());
            }
            _ => panic!("Expected Pong response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_secret_found() {
        let (handler, secrets) = create_handler_with_secrets();
        secrets.set_cached("test", "secret-value").await.unwrap();

        let response = handler
            .handle(Request::GetSecret {
                provider: "test".to_string(),
            })
            .await;

        match response {
            Response::Secret { value } => {
                assert_eq!(value, "secret-value");
            }
            _ => panic!("Expected Secret response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_secret_not_found() {
        let handler = create_handler();

        let response = handler
            .handle(Request::GetSecret {
                provider: "nonexistent".to_string(),
            })
            .await;

        match response {
            Response::Error { message } => {
                assert!(message.contains("nonexistent"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_has_secret() {
        let (handler, secrets) = create_handler_with_secrets();
        secrets.set_cached("test", "value").await.unwrap();

        // Existing secret
        let response = handler
            .handle(Request::HasSecret {
                provider: "test".to_string(),
            })
            .await;
        assert!(matches!(response, Response::Exists { exists: true }));

        // Non-existing secret
        let response = handler
            .handle(Request::HasSecret {
                provider: "nonexistent".to_string(),
            })
            .await;
        assert!(matches!(response, Response::Exists { exists: false }));
    }

    #[tokio::test]
    async fn test_handle_list_providers() {
        let (handler, secrets) = create_handler_with_secrets();
        secrets.set_cached("anthropic", "key1").await.unwrap();
        secrets.set_cached("openai", "key2").await.unwrap();

        let response = handler.handle(Request::ListProviders).await;

        match response {
            Response::Providers { providers } => {
                assert_eq!(providers.len(), 2);
                assert!(providers.contains(&"anthropic".to_string()));
                assert!(providers.contains(&"openai".to_string()));
            }
            _ => panic!("Expected Providers response"),
        }
    }

    // ==================== Job Handler Tests ====================

    #[tokio::test]
    async fn test_handle_job_stats() {
        let handler = create_handler();

        let response = handler.handle(Request::JobStats).await;

        match response {
            Response::JobStatsResult { stats } => {
                assert_eq!(stats.total, 0);
                assert_eq!(stats.pending, 0);
                assert_eq!(stats.running, 0);
                assert_eq!(stats.completed, 0);
                assert_eq!(stats.failed, 0);
                assert_eq!(stats.cancelled, 0);
            }
            _ => panic!("Expected JobStatsResult response"),
        }
    }

    #[tokio::test]
    async fn test_handle_job_list_empty() {
        let handler = create_handler();

        let response = handler.handle(Request::JobList { state: None }).await;

        match response {
            Response::JobListResult { jobs } => {
                assert!(jobs.is_empty());
            }
            _ => panic!("Expected JobListResult response"),
        }
    }

    #[tokio::test]
    async fn test_handle_job_submit_workflow_not_found() {
        let handler = create_handler();

        let response = handler
            .handle(Request::JobSubmit {
                workflow: "/nonexistent/workflow.nika.yaml".to_string(),
                args: vec![],
                name: None,
                priority: 0,
            })
            .await;

        match response {
            Response::Error { message } => {
                assert!(message.contains("not found"));
            }
            _ => panic!("Expected Error response for missing workflow"),
        }
    }

    #[tokio::test]
    async fn test_handle_job_status_not_found() {
        let handler = create_handler();

        let response = handler
            .handle(Request::JobStatus {
                job_id: "nonexistent".to_string(),
            })
            .await;

        match response {
            Response::JobStatusResult { job } => {
                assert!(job.is_none());
            }
            _ => panic!("Expected JobStatusResult response"),
        }
    }

    #[tokio::test]
    async fn test_handle_job_cancel_not_found() {
        let handler = create_handler();

        let response = handler
            .handle(Request::JobCancel {
                job_id: "nonexistent".to_string(),
            })
            .await;

        match response {
            Response::JobCancelled { cancelled, job_id } => {
                assert!(!cancelled);
                assert_eq!(job_id, "nonexistent");
            }
            _ => panic!("Expected JobCancelled response"),
        }
    }
}
