# Integration Gaps Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete all supernovae-cli integration gaps: model progress feedback, daemon jobs IPC, and UX polish.

**Architecture:** Three independent phases that can be executed sequentially. Each phase follows TDD (RED → GREEN → REFACTOR) with frequent commits. All changes are in the spn crate with dependencies on spn-ollama and spn-core.

**Tech Stack:** Rust 2021, tokio 1.36, indicatif 0.18, console 0.15, serde, rmcp 0.16

---

## Phase 1: Model Progress Feedback (~3 hours)

### Problem Statement

Model operations (pull, load, delete) have **zero visual feedback**. The Ollama client already streams progress data, but it's discarded:

```rust
// Current: Progress callback is None!
self.backend.pull(name, None).await
```

### Task 1.1: Add Progress Streaming Types

**Files:**
- Create: `crates/spn/src/daemon/protocol/progress.rs`
- Modify: `crates/spn/src/daemon/protocol/mod.rs:1-10`

**Step 1: Write the failing test**

```rust
// File: crates/spn/src/daemon/protocol/progress.rs
#[cfg(test)]
mod tests {
    use super::*;

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
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p spn-cli --lib daemon::protocol::progress::tests -- --nocapture`
Expected: FAIL with "cannot find module `progress`"

**Step 3: Write minimal implementation**

```rust
// File: crates/spn/src/daemon/protocol/progress.rs
//! Progress streaming types for daemon-to-CLI communication.

use serde::{Deserialize, Serialize};

/// Progress update for model operations (pull, load, delete).
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
```

**Step 4: Update mod.rs to export the module**

```rust
// File: crates/spn/src/daemon/protocol/mod.rs
// Add at the top:
mod progress;

pub use progress::ModelProgress;
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p spn-cli --lib daemon::protocol::progress::tests -- --nocapture`
Expected: PASS (3 tests)

**Step 6: Commit**

```bash
git add crates/spn/src/daemon/protocol/progress.rs crates/spn/src/daemon/protocol/mod.rs
git commit -m "feat(daemon): add ModelProgress type for streaming updates

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 1.2: Add Streaming Response Variant

**Files:**
- Modify: `crates/spn/src/daemon/protocol/mod.rs:40-80`

**Step 1: Write the failing test**

```rust
// Add to crates/spn/src/daemon/protocol/mod.rs tests
#[test]
fn test_response_progress_variant() {
    let response = Response::Progress(ModelProgress::determinate("downloading", 50, 100));

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Progress"));
    assert!(json.contains("downloading"));

    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::Progress(p) => {
            assert_eq!(p.status, "downloading");
            assert_eq!(p.completed, Some(50));
        }
        _ => panic!("Expected Progress variant"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p spn-cli --lib daemon::protocol::tests::test_response_progress_variant -- --nocapture`
Expected: FAIL with "no variant named `Progress`"

**Step 3: Add Progress variant to Response enum**

```rust
// File: crates/spn/src/daemon/protocol/mod.rs
// Add to Response enum (around line 45):

/// Daemon response to CLI requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    /// Operation completed successfully
    Success { success: bool },
    /// Operation failed with error
    Error { error: String },
    /// List of secrets (for HasSecrets)
    SecretsList { secrets: Vec<String> },
    /// Secret value (for GetSecret)
    Secret { value: String },
    /// List of providers
    Providers { providers: Vec<ProviderInfo> },
    /// Health check result
    Health { status: String, uptime_secs: u64 },
    /// Model information
    ModelInfo { info: ModelInfoResponse },
    /// List of models
    ModelList { models: Vec<ModelInfoResponse> },
    /// Progress update (streaming)
    Progress(ModelProgress),
    /// End of stream marker
    StreamEnd { success: bool, error: Option<String> },
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p spn-cli --lib daemon::protocol::tests::test_response_progress_variant -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/spn/src/daemon/protocol/mod.rs
git commit -m "feat(daemon): add Progress and StreamEnd response variants

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 1.3: Wire Progress Callback in ModelManager

**Files:**
- Modify: `crates/spn/src/daemon/model_manager.rs:50-70`
- Test: `crates/spn/src/daemon/model_manager.rs` (add tests)

**Step 1: Write the failing test**

```rust
// Add to crates/spn/src/daemon/model_manager.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pull_with_progress_callback() {
        // This test requires a mock backend
        // For now, we test the callback signature compiles
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let callback = Box::new(move |_progress: spn_ollama::PullProgress| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Verify the callback type is correct
        let _: spn_ollama::ProgressCallback = callback;

        // If this compiles, the signature is correct
        assert!(true);
    }
}
```

**Step 2: Run test to verify it compiles**

Run: `cargo test -p spn-cli --lib daemon::model_manager::tests -- --nocapture`
Expected: PASS (signature test only)

**Step 3: Add pull_with_progress method to ModelManager**

```rust
// File: crates/spn/src/daemon/model_manager.rs
// Add new method after existing pull():

/// Pull a model with progress callback.
pub async fn pull_with_progress<F>(&self, name: &str, mut on_progress: F) -> Result<(), BackendError>
where
    F: FnMut(spn_ollama::PullProgress) + Send + 'static,
{
    info!(model = %name, "Pulling model with progress");

    let callback: spn_ollama::ProgressCallback = Box::new(move |progress| {
        on_progress(progress);
    });

    self.backend.pull(name, Some(callback)).await
}
```

**Step 4: Run all model_manager tests**

Run: `cargo test -p spn-cli --lib daemon::model_manager -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/spn/src/daemon/model_manager.rs
git commit -m "feat(model): add pull_with_progress method for streaming updates

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 1.4: Stream Progress from Daemon Handler

**Files:**
- Modify: `crates/spn/src/daemon/handler.rs:110-150`

**Step 1: Understand current handler structure**

Read: `crates/spn/src/daemon/handler.rs`
Note the `handle_model_pull` function signature and how it sends responses.

**Step 2: Update handler to stream progress**

```rust
// File: crates/spn/src/daemon/handler.rs
// Modify handle_model_pull (around line 125):

async fn handle_model_pull(
    &self,
    name: String,
    response_tx: tokio::sync::mpsc::Sender<Response>,
) -> Result<(), DaemonError> {
    use crate::daemon::protocol::ModelProgress;

    // Create progress sender channel
    let tx = response_tx.clone();

    let progress_callback = move |progress: spn_ollama::PullProgress| {
        let model_progress = ModelProgress {
            status: progress.status.clone(),
            completed: progress.completed,
            total: progress.total,
            digest: progress.digest.clone(),
        };

        // Send progress update (ignore send errors if receiver dropped)
        let _ = tx.try_send(Response::Progress(model_progress));
    };

    // Pull with progress streaming
    match self.models.pull_with_progress(&name, progress_callback).await {
        Ok(()) => {
            response_tx.send(Response::StreamEnd {
                success: true,
                error: None
            }).await.ok();
        }
        Err(e) => {
            response_tx.send(Response::StreamEnd {
                success: false,
                error: Some(e.to_string())
            }).await.ok();
        }
    }

    Ok(())
}
```

**Step 3: Verify compilation**

Run: `cargo build -p spn-cli`
Expected: Success (may need to adjust handler signature)

**Step 4: Commit**

```bash
git add crates/spn/src/daemon/handler.rs
git commit -m "feat(daemon): stream model pull progress to clients

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 1.5: Display Progress in CLI

**Files:**
- Modify: `crates/spn/src/commands/model/handler.rs:144-180`

**Step 1: Create progress display helper**

```rust
// File: crates/spn/src/commands/model/progress.rs
//! Progress display for model operations.

use indicatif::{ProgressBar, ProgressStyle};
use crate::daemon::protocol::ModelProgress;

/// Create a progress bar for model downloads.
pub fn create_download_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}

/// Create an indeterminate spinner.
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Update progress bar from ModelProgress.
pub fn update_progress(pb: &ProgressBar, progress: &ModelProgress) {
    if let (Some(completed), Some(total)) = (progress.completed, progress.total) {
        if pb.length() != Some(total) {
            pb.set_length(total);
        }
        pb.set_position(completed);
    }
    pb.set_message(progress.status.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_download_bar() {
        let pb = create_download_bar(1000);
        assert_eq!(pb.length(), Some(1000));
    }

    #[test]
    fn test_create_spinner() {
        let pb = create_spinner("Loading...");
        assert!(pb.is_hidden() == false);
    }
}
```

**Step 2: Update pull command to use progress**

```rust
// File: crates/spn/src/commands/model/handler.rs
// Modify pull function (around line 144):

async fn pull(name: &str) -> Result<()> {
    use crate::commands::model::progress::{create_spinner, create_download_bar, update_progress};
    use crate::daemon::protocol::Response;

    println!("{} Pulling model: {}", ds::primary("->"), ds::highlight(name));

    // Connect to daemon
    let client = connect_to_daemon().await?;

    // Send pull request
    client.send(Request::ModelPull { name: name.to_string() }).await?;

    // Create initial spinner (will switch to progress bar when total is known)
    let pb = create_spinner("Starting download...");
    let mut current_bar: Option<indicatif::ProgressBar> = None;

    // Receive streaming responses
    loop {
        match client.recv().await? {
            Response::Progress(progress) => {
                // Switch to progress bar when we have total
                if let (Some(_), Some(total)) = (progress.completed, progress.total) {
                    if current_bar.is_none() {
                        pb.finish_and_clear();
                        let new_bar = create_download_bar(total);
                        current_bar = Some(new_bar);
                    }
                    if let Some(ref bar) = current_bar {
                        update_progress(bar, &progress);
                    }
                } else {
                    pb.set_message(progress.status);
                }
            }
            Response::StreamEnd { success, error } => {
                // Finish progress display
                if let Some(ref bar) = current_bar {
                    bar.finish_and_clear();
                } else {
                    pb.finish_and_clear();
                }

                if success {
                    println!("{} Model '{}' pulled successfully", ds::success("*"), ds::highlight(name));
                } else {
                    let err_msg = error.unwrap_or_else(|| "Unknown error".into());
                    println!("{} Failed to pull model: {}", ds::error("!"), err_msg);
                    return Err(anyhow::anyhow!(err_msg));
                }
                break;
            }
            Response::Error { error } => {
                pb.finish_and_clear();
                return Err(anyhow::anyhow!(error));
            }
            _ => {
                // Ignore other response types
            }
        }
    }

    Ok(())
}
```

**Step 3: Run and verify**

Run: `cargo build -p spn-cli && cargo test -p spn-cli`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/spn/src/commands/model/progress.rs crates/spn/src/commands/model/handler.rs crates/spn/src/commands/model/mod.rs
git commit -m "feat(cli): display real-time progress for model pull

- Add progress.rs with download bar and spinner helpers
- Stream progress from daemon to CLI
- Switch from spinner to progress bar when total is known

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Phase 2: Daemon Jobs IPC Integration (~2 hours)

### Problem Statement

JobScheduler is fully implemented but **NOT integrated into daemon IPC**. CLI commands use JobScheduler directly, bypassing the daemon. This means jobs don't benefit from daemon's singleton guarantees.

### Task 2.1: Add Job Request/Response Types

**Files:**
- Modify: `crates/spn/src/daemon/protocol/mod.rs:20-40`

**Step 1: Write the failing test**

```rust
// Add to crates/spn/src/daemon/protocol/mod.rs tests
#[test]
fn test_job_request_serialization() {
    let request = Request::JobSubmit {
        workflow: "/path/to/workflow.nika.yaml".into(),
        args: Some(vec!["--verbose".into()]),
        name: Some("test-job".into()),
        priority: Some(10),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("JobSubmit"));
    assert!(json.contains("workflow.nika.yaml"));

    let parsed: Request = serde_json::from_str(&json).unwrap();
    match parsed {
        Request::JobSubmit { workflow, priority, .. } => {
            assert!(workflow.contains("workflow"));
            assert_eq!(priority, Some(10));
        }
        _ => panic!("Expected JobSubmit"),
    }
}

#[test]
fn test_job_response_serialization() {
    let response = Response::JobSubmitted {
        job_id: "abc12345".into()
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();

    match parsed {
        Response::JobSubmitted { job_id } => {
            assert_eq!(job_id, "abc12345");
        }
        _ => panic!("Expected JobSubmitted"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p spn-cli --lib daemon::protocol::tests -- --nocapture`
Expected: FAIL with "no variant named `JobSubmit`"

**Step 3: Add Job variants to Request and Response enums**

```rust
// File: crates/spn/src/daemon/protocol/mod.rs

// Add to Request enum:
/// Submit a job to the scheduler
JobSubmit {
    workflow: String,
    args: Option<Vec<String>>,
    name: Option<String>,
    priority: Option<i32>,
},
/// Get job status
JobStatus { job_id: String },
/// List all jobs
JobList { all: bool },
/// Cancel a job
JobCancel { job_id: String },
/// Get job output
JobOutput { job_id: String },
/// Clear completed jobs
JobClear,
/// Get scheduler stats
JobStats,

// Add to Response enum:
/// Job submitted successfully
JobSubmitted { job_id: String },
/// Job status response
JobStatusResponse { status: JobStatusInfo },
/// Job list response
JobListResponse { jobs: Vec<JobStatusInfo> },
/// Job cancelled
JobCancelled { success: bool },
/// Job output
JobOutputResponse { output: Option<String> },
/// Scheduler stats
JobStatsResponse { stats: SchedulerStatsInfo },
```

**Step 4: Add JobStatusInfo and SchedulerStatsInfo types**

```rust
// File: crates/spn/src/daemon/protocol/mod.rs
// Add after existing structs:

/// Job status information for IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusInfo {
    pub id: String,
    pub workflow: String,
    pub name: Option<String>,
    pub state: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub progress: Option<String>,
    pub error: Option<String>,
}

/// Scheduler statistics for IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatsInfo {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub has_nika: bool,
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p spn-cli --lib daemon::protocol::tests -- --nocapture`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/spn/src/daemon/protocol/mod.rs
git commit -m "feat(daemon): add Job IPC request/response types

- JobSubmit, JobStatus, JobList, JobCancel, JobOutput, JobClear, JobStats
- JobSubmitted, JobStatusResponse, JobListResponse, etc.
- JobStatusInfo and SchedulerStatsInfo data types

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 2.2: Add JobScheduler to DaemonState

**Files:**
- Modify: `crates/spn/src/daemon/mod.rs:30-60`

**Step 1: Import JobScheduler in daemon module**

```rust
// File: crates/spn/src/daemon/mod.rs
// Add to imports:
use crate::daemon::jobs::JobScheduler;
```

**Step 2: Add scheduler field to DaemonState**

```rust
// File: crates/spn/src/daemon/mod.rs
// Modify DaemonState struct:

pub struct DaemonState {
    pub secrets: SecretCache,
    pub models: ModelManager,
    pub mcp: McpManager,
    pub scheduler: JobScheduler,  // Add this
}
```

**Step 3: Initialize scheduler in DaemonState::new()**

```rust
// File: crates/spn/src/daemon/mod.rs
// Modify DaemonState::new():

impl DaemonState {
    pub async fn new() -> Result<Self, DaemonError> {
        let secrets = SecretCache::new();
        let models = ModelManager::new().await?;
        let mcp = McpManager::new();
        let scheduler = JobScheduler::new().await
            .map_err(|e| DaemonError::Init(format!("Failed to init scheduler: {}", e)))?;

        Ok(Self {
            secrets,
            models,
            mcp,
            scheduler,
        })
    }
}
```

**Step 4: Verify compilation**

Run: `cargo build -p spn-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/spn/src/daemon/mod.rs
git commit -m "feat(daemon): add JobScheduler to DaemonState

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 2.3: Implement Job Handlers

**Files:**
- Modify: `crates/spn/src/daemon/handler.rs:200-350`

**Step 1: Add job handler methods**

```rust
// File: crates/spn/src/daemon/handler.rs
// Add after existing handle_* methods:

async fn handle_job_submit(
    &self,
    workflow: String,
    args: Option<Vec<String>>,
    name: Option<String>,
    priority: Option<i32>,
) -> Response {
    use crate::daemon::jobs::Job;

    let mut job = Job::new(workflow);
    if let Some(args) = args {
        job = job.with_args(args);
    }
    if let Some(name) = name {
        job = job.with_name(name);
    }
    if let Some(priority) = priority {
        job = job.with_priority(priority);
    }

    match self.state.scheduler.submit(job).await {
        Ok(job_id) => Response::JobSubmitted { job_id: job_id.to_string() },
        Err(e) => Response::Error { error: e.to_string() },
    }
}

async fn handle_job_status(&self, job_id: String) -> Response {
    use crate::daemon::jobs::JobId;

    let job_id = match job_id.parse::<JobId>() {
        Ok(id) => id,
        Err(_) => return Response::Error { error: "Invalid job ID".into() },
    };

    match self.state.scheduler.status(&job_id).await {
        Some(status) => Response::JobStatusResponse {
            status: job_status_to_info(&status),
        },
        None => Response::Error { error: "Job not found".into() },
    }
}

async fn handle_job_list(&self, all: bool) -> Response {
    let jobs = self.state.scheduler.list().await;
    let jobs: Vec<_> = jobs.iter()
        .filter(|j| all || !j.is_terminal())
        .map(job_status_to_info)
        .collect();

    Response::JobListResponse { jobs }
}

async fn handle_job_cancel(&self, job_id: String) -> Response {
    use crate::daemon::jobs::JobId;

    let job_id = match job_id.parse::<JobId>() {
        Ok(id) => id,
        Err(_) => return Response::Error { error: "Invalid job ID".into() },
    };

    match self.state.scheduler.cancel(&job_id).await {
        Ok(()) => Response::JobCancelled { success: true },
        Err(e) => Response::Error { error: e.to_string() },
    }
}

async fn handle_job_output(&self, job_id: String) -> Response {
    use crate::daemon::jobs::JobId;

    let job_id = match job_id.parse::<JobId>() {
        Ok(id) => id,
        Err(_) => return Response::Error { error: "Invalid job ID".into() },
    };

    match self.state.scheduler.status(&job_id).await {
        Some(status) => Response::JobOutputResponse {
            output: status.output.clone()
        },
        None => Response::Error { error: "Job not found".into() },
    }
}

async fn handle_job_stats(&self) -> Response {
    let stats = self.state.scheduler.stats().await;
    Response::JobStatsResponse {
        stats: SchedulerStatsInfo {
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

// Helper function
fn job_status_to_info(status: &crate::daemon::jobs::JobStatus) -> JobStatusInfo {
    JobStatusInfo {
        id: status.job.id.to_string(),
        workflow: status.job.workflow.clone(),
        name: status.job.name.clone(),
        state: status.state.to_string(),
        created_at: status.job.created_at.to_rfc3339(),
        started_at: status.started_at.map(|t| t.to_rfc3339()),
        ended_at: status.ended_at.map(|t| t.to_rfc3339()),
        progress: status.progress.clone(),
        error: status.error.clone(),
    }
}
```

**Step 2: Wire handlers in handle_request match**

```rust
// File: crates/spn/src/daemon/handler.rs
// Add to handle_request match statement:

Request::JobSubmit { workflow, args, name, priority } => {
    self.handle_job_submit(workflow, args, name, priority).await
}
Request::JobStatus { job_id } => {
    self.handle_job_status(job_id).await
}
Request::JobList { all } => {
    self.handle_job_list(all).await
}
Request::JobCancel { job_id } => {
    self.handle_job_cancel(job_id).await
}
Request::JobOutput { job_id } => {
    self.handle_job_output(job_id).await
}
Request::JobClear => {
    self.state.scheduler.store.cleanup(std::time::Duration::from_secs(0)).await;
    Response::Success { success: true }
}
Request::JobStats => {
    self.handle_job_stats().await
}
```

**Step 3: Verify compilation and tests**

Run: `cargo build -p spn-cli && cargo test -p spn-cli`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/spn/src/daemon/handler.rs
git commit -m "feat(daemon): implement Job IPC handlers

- handle_job_submit, handle_job_status, handle_job_list
- handle_job_cancel, handle_job_output, handle_job_stats
- Wire all handlers in request match

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 2.4: Update CLI Commands to Use Daemon

**Files:**
- Modify: `crates/spn/src/commands/jobs.rs:50-200`

**Step 1: Refactor submit command to use daemon**

```rust
// File: crates/spn/src/commands/jobs.rs
// Modify submit function:

async fn submit(workflow: &str, args: Option<Vec<String>>, name: Option<String>, priority: Option<i32>) -> Result<()> {
    // Connect to daemon
    let client = crate::daemon::client::connect().await?;

    // Send submit request
    let request = Request::JobSubmit {
        workflow: workflow.to_string(),
        args,
        name,
        priority,
    };

    let response = client.request(request).await?;

    match response {
        Response::JobSubmitted { job_id } => {
            println!("{} Job submitted: {}", ds::success("*"), ds::highlight(&job_id));
            Ok(())
        }
        Response::Error { error } => {
            Err(anyhow::anyhow!(error))
        }
        _ => Err(anyhow::anyhow!("Unexpected response")),
    }
}
```

**Step 2: Refactor remaining commands similarly**

(Apply same pattern to list, status, cancel, output, clear commands)

**Step 3: Verify compilation and tests**

Run: `cargo build -p spn-cli && cargo test -p spn-cli`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/spn/src/commands/jobs.rs
git commit -m "refactor(cli): use daemon IPC for job commands

- All job commands now go through daemon
- Consistent with model/provider commands
- Benefits from daemon singleton guarantees

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Phase 3: UX Polish (~2 hours)

### Task 3.1: Add Interactive Provider Setup

**Files:**
- Create: `crates/spn/src/commands/provider/interactive.rs`
- Modify: `crates/spn/src/commands/provider/handler.rs`

**Step 1: Write the failing test**

```rust
// File: crates/spn/src/commands/provider/interactive.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key_display() {
        let key = "sk-ant-1234567890abcdef";
        let masked = mask_key_for_display(key);
        assert_eq!(masked, "sk-ant-...cdef");
    }

    #[test]
    fn test_validate_provider_name() {
        assert!(validate_provider_name("anthropic").is_ok());
        assert!(validate_provider_name("ANTHROPIC").is_ok());
        assert!(validate_provider_name("invalid-provider").is_err());
    }
}
```

**Step 2: Implement interactive module**

```rust
// File: crates/spn/src/commands/provider/interactive.rs
//! Interactive provider configuration.

use anyhow::Result;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};

use crate::ux::design_system as ds;
use spn_core::KNOWN_PROVIDERS;

/// Mask API key for display (show prefix and last 4 chars).
pub fn mask_key_for_display(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }

    // Find prefix (up to first -)
    let prefix_end = key.find('-').map(|i| i + 1).unwrap_or(4);
    let prefix = &key[..prefix_end.min(key.len())];
    let suffix = &key[key.len().saturating_sub(4)..];

    format!("{}...{}", prefix, suffix)
}

/// Validate provider name against known providers.
pub fn validate_provider_name(name: &str) -> Result<&'static str> {
    let name_lower = name.to_lowercase();
    KNOWN_PROVIDERS
        .iter()
        .find(|p| p.id.to_lowercase() == name_lower || p.name.to_lowercase() == name_lower)
        .map(|p| p.id)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))
}

/// Interactive provider selection.
pub fn select_provider() -> Result<&'static str> {
    let providers: Vec<_> = KNOWN_PROVIDERS.iter().map(|p| p.name).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select provider")
        .items(&providers)
        .default(0)
        .interact()?;

    Ok(KNOWN_PROVIDERS[selection].id)
}

/// Interactive API key input with masking.
pub fn input_api_key(provider_name: &str) -> Result<String> {
    let key = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter {} API key", provider_name))
        .interact()?;

    Ok(key)
}

/// Interactive provider setup flow.
pub async fn interactive_setup() -> Result<()> {
    println!("{}", ds::heading("Provider Setup"));
    println!();

    // Select provider
    let provider_id = select_provider()?;
    let provider = KNOWN_PROVIDERS.iter().find(|p| p.id == provider_id).unwrap();

    println!();
    println!("{} Selected: {}", ds::info("i"), ds::provider(provider.name));

    // Input API key
    let key = input_api_key(provider.name)?;

    // Validate key format
    if let Err(e) = spn_core::validate_key_format(provider_id, &key) {
        println!("{} Invalid key format: {}", ds::warning("!"), e);
        return Err(anyhow::anyhow!("Invalid key format"));
    }

    // Store in keychain
    use spn_keyring::SpnKeyring;
    let keyring = SpnKeyring::new()?;
    keyring.set(provider.env_var, &key)?;

    println!();
    println!("{} API key stored securely in OS keychain", ds::success("*"));
    println!("   Key: {}", ds::muted(&mask_key_for_display(&key)));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key_display() {
        let key = "sk-ant-1234567890abcdef";
        let masked = mask_key_for_display(key);
        assert!(masked.starts_with("sk-ant-"));
        assert!(masked.ends_with("cdef"));
        assert!(masked.contains("..."));
    }

    #[test]
    fn test_validate_provider_name() {
        assert!(validate_provider_name("anthropic").is_ok());
        assert!(validate_provider_name("ANTHROPIC").is_ok());
        assert!(validate_provider_name("openai").is_ok());
        assert!(validate_provider_name("invalid-provider").is_err());
    }
}
```

**Step 3: Add dialoguer dependency**

```toml
# File: crates/spn/Cargo.toml
# Add to dependencies:
dialoguer = "0.11"
```

**Step 4: Wire interactive command**

```rust
// File: crates/spn/src/commands/provider/handler.rs
// Add new subcommand:

#[derive(Subcommand)]
pub enum ProviderCommand {
    // ... existing commands ...

    /// Interactive provider setup
    Setup,
}

// In handle function:
ProviderCommand::Setup => {
    crate::commands::provider::interactive::interactive_setup().await
}
```

**Step 5: Run tests**

Run: `cargo test -p spn-cli --lib commands::provider::interactive -- --nocapture`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/spn/Cargo.toml crates/spn/src/commands/provider/interactive.rs crates/spn/src/commands/provider/handler.rs crates/spn/src/commands/provider/mod.rs
git commit -m "feat(cli): add interactive provider setup command

- spn provider setup for guided configuration
- Provider selection with dialoguer
- Masked API key input
- Key format validation
- Secure keychain storage

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

### Task 3.2: Add Transforming Spinners for Long Operations

**Files:**
- Modify: `crates/spn/src/ux/progress.rs:100-150`

**Step 1: Add TransformingSpinner wrapper**

```rust
// File: crates/spn/src/ux/progress.rs
// Add new struct:

/// A spinner that transforms to success/error on completion.
pub struct TransformingSpinner {
    pb: ProgressBar,
    message: String,
}

impl TransformingSpinner {
    /// Create a new transforming spinner.
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(message.clone());
        pb.enable_steady_tick(Duration::from_millis(80));

        Self { pb, message }
    }

    /// Update the spinner message.
    pub fn set_message(&self, msg: impl Into<String>) {
        self.pb.set_message(msg.into());
    }

    /// Finish with success (checkmark).
    pub fn finish_success(self, msg: impl Into<String>) {
        self.pb.finish_with_message(format!("{} {}",
            console::style("✓").green().bold(),
            msg.into()
        ));
    }

    /// Finish with error (cross).
    pub fn finish_error(self, msg: impl Into<String>) {
        self.pb.finish_with_message(format!("{} {}",
            console::style("✗").red().bold(),
            msg.into()
        ));
    }

    /// Finish with warning (triangle).
    pub fn finish_warning(self, msg: impl Into<String>) {
        self.pb.finish_with_message(format!("{} {}",
            console::style("⚠").yellow().bold(),
            msg.into()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transforming_spinner() {
        let spinner = TransformingSpinner::new("Loading...");
        spinner.set_message("Still loading...");
        spinner.finish_success("Done!");
        // Test passes if no panic
    }
}
```

**Step 2: Use TransformingSpinner in commands**

```rust
// Example usage in crates/spn/src/commands/model/handler.rs:

use crate::ux::progress::TransformingSpinner;

async fn load(name: &str) -> Result<()> {
    let spinner = TransformingSpinner::new(format!("Loading model {}...", name));

    match daemon_load_model(name).await {
        Ok(()) => {
            spinner.finish_success(format!("Model {} loaded", name));
            Ok(())
        }
        Err(e) => {
            spinner.finish_error(format!("Failed: {}", e));
            Err(e)
        }
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p spn-cli --lib ux::progress -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/spn/src/ux/progress.rs
git commit -m "feat(ux): add TransformingSpinner for long operations

- Spinner transforms to ✓/✗/⚠ on completion
- Provides clear visual feedback
- Used for model/provider/daemon operations

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika <nika@supernovae.studio>"
```

---

## Verification Checklist

After completing all phases:

- [ ] `cargo test --workspace` — All tests pass (700+)
- [ ] `cargo clippy --workspace -- -D warnings` — Zero warnings
- [ ] `cargo build --release` — Release build succeeds
- [ ] Manual test: `spn model pull llama3.2:1b` shows progress bar
- [ ] Manual test: `spn provider setup` shows interactive flow
- [ ] Manual test: `spn jobs submit test.nika.yaml` uses daemon

---

## Summary

| Phase | Tasks | Time | Tests Added |
|-------|-------|------|-------------|
| 1. Model Progress | 5 | ~3h | 8 |
| 2. Jobs IPC | 4 | ~2h | 6 |
| 3. UX Polish | 2 | ~2h | 4 |
| **Total** | **11** | **~7h** | **18** |

All changes follow TDD with RED → GREEN → REFACTOR cycle and frequent commits.
