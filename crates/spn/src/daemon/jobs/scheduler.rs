//! Job scheduler for background workflow execution.

#![allow(dead_code)]

use super::store::JobStore;
use super::types::{Job, JobId, JobState, JobStatus};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Maximum concurrent jobs.
const MAX_CONCURRENT_JOBS: usize = 4;

/// Job scheduler for background workflow execution.
pub struct JobScheduler {
    /// Job store.
    store: Arc<JobStore>,
    /// Currently running job handles.
    running: RwLock<Vec<(JobId, JoinHandle<()>)>>,
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
    /// Path to nika binary.
    nika_path: Option<PathBuf>,
}

impl JobScheduler {
    /// Create a new job scheduler.
    pub fn new(store: Arc<JobStore>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        // Try to find nika binary
        let nika_path = which::which("nika").ok().or_else(|| {
            // Check in ~/.spn/bin
            dirs::home_dir()
                .map(|h| h.join(".spn/bin/nika"))
                .filter(|p| p.exists())
        });

        Self {
            store,
            running: RwLock::new(Vec::new()),
            shutdown_tx,
            nika_path,
        }
    }

    /// Check if nika is available.
    pub fn has_nika(&self) -> bool {
        self.nika_path.is_some()
    }

    /// Submit a job for execution.
    pub async fn submit(&self, job: Job) -> JobStatus {
        let status = self.store.add(job).await;
        info!(job_id = %status.job.id, "Job submitted");

        // Try to start immediately if under capacity
        self.maybe_start_next().await;

        status
    }

    /// Get job status.
    pub async fn status(&self, id: &JobId) -> Option<JobStatus> {
        self.store.get(id).await
    }

    /// List all jobs.
    pub async fn list(&self) -> Vec<JobStatus> {
        self.store.list().await
    }

    /// Cancel a job.
    pub async fn cancel(&self, id: &JobId) -> bool {
        // Check if job exists
        let mut status = match self.store.get(id).await {
            Some(s) => s,
            None => return false,
        };

        match status.state {
            JobState::Pending => {
                // Just mark as cancelled
                status.cancel();
                self.store.update(status).await;
                info!(job_id = %id, "Pending job cancelled");
                true
            }
            JobState::Running => {
                // Find and kill the running task
                let mut running = self.running.write().await;
                if let Some(idx) = running.iter().position(|(job_id, _)| job_id == id) {
                    let (_, handle) = running.remove(idx);
                    handle.abort();
                    drop(running);

                    status.cancel();
                    self.store.update(status).await;
                    info!(job_id = %id, "Running job cancelled");
                    true
                } else {
                    warn!(job_id = %id, "Job marked running but no handle found");
                    false
                }
            }
            _ => {
                // Already terminal
                debug!(job_id = %id, state = %status.state, "Cannot cancel terminal job");
                false
            }
        }
    }

    /// Try to start the next pending job if under capacity.
    async fn maybe_start_next(&self) {
        let running = self.running.read().await;
        if running.len() >= MAX_CONCURRENT_JOBS {
            debug!("At max capacity, not starting new job");
            return;
        }
        drop(running);

        // Get next pending job
        let pending = self.store.list_pending().await;
        if let Some(mut status) = pending.into_iter().next() {
            let job_id = status.job.id;

            // Mark as running
            status.start();
            self.store.update(status.clone()).await;

            // Start execution
            let store = Arc::clone(&self.store);
            let nika_path = self.nika_path.clone();
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            let handle = tokio::spawn(async move {
                tokio::select! {
                    result = execute_job(&status.job, nika_path) => {
                        // Update status based on result
                        let mut status = store.get(&job_id).await.unwrap_or(status);
                        match result {
                            Ok(output) => {
                                status.complete(Some(output));
                                info!(job_id = %job_id, "Job completed");
                            }
                            Err(e) => {
                                status.fail(e.to_string());
                                error!(job_id = %job_id, error = %e, "Job failed");
                            }
                        }
                        store.update(status).await;
                    }
                    _ = shutdown_rx.recv() => {
                        // Graceful shutdown
                        let mut status = store.get(&job_id).await.unwrap_or(status);
                        status.cancel();
                        store.update(status).await;
                        info!(job_id = %job_id, "Job cancelled due to shutdown");
                    }
                }
            });

            // Track the running job
            let mut running = self.running.write().await;
            running.push((job_id, handle));
            info!(job_id = %job_id, "Job started");
        }
    }

    /// Process completed jobs and start new ones.
    pub async fn tick(&self) {
        // Clean up completed handles
        let mut running = self.running.write().await;
        running.retain(|(_, handle)| !handle.is_finished());
        drop(running);

        // Try to start more jobs
        self.maybe_start_next().await;
    }

    /// Shutdown the scheduler gracefully.
    pub async fn shutdown(&self) {
        info!("Shutting down job scheduler");

        // Signal all running jobs to stop
        let _ = self.shutdown_tx.send(());

        // Wait for all running jobs
        let mut running = self.running.write().await;
        for (job_id, handle) in running.drain(..) {
            debug!(job_id = %job_id, "Waiting for job to finish");
            let _ = handle.await;
        }

        info!("Job scheduler shutdown complete");
    }

    /// Get scheduler statistics.
    pub async fn stats(&self) -> SchedulerStats {
        let jobs = self.store.list().await;
        let running = self.running.read().await;

        SchedulerStats {
            total: jobs.len(),
            pending: jobs.iter().filter(|s| s.state == JobState::Pending).count(),
            running: running.len(),
            completed: jobs
                .iter()
                .filter(|s| s.state == JobState::Completed)
                .count(),
            failed: jobs.iter().filter(|s| s.state == JobState::Failed).count(),
            cancelled: jobs
                .iter()
                .filter(|s| s.state == JobState::Cancelled)
                .count(),
            has_nika: self.has_nika(),
        }
    }
}

/// Scheduler statistics.
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub has_nika: bool,
}

/// Execute a job (run nika workflow).
async fn execute_job(job: &Job, nika_path: Option<PathBuf>) -> Result<String, String> {
    let nika = match nika_path {
        Some(p) => p,
        None => return Err("Nika not found. Install with: spn setup nika".into()),
    };

    if !job.workflow.exists() {
        return Err(format!("Workflow not found: {}", job.workflow.display()));
    }

    let mut cmd = Command::new(&nika);
    cmd.arg("run")
        .arg(&job.workflow)
        .args(&job.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    debug!(workflow = %job.workflow.display(), "Executing workflow");

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to execute nika: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Workflow failed: {}", stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_stats() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-sched-test-{}", uuid::Uuid::new_v4()));
        let store = Arc::new(JobStore::new(&temp_dir));
        store.init().await.unwrap();

        let scheduler = JobScheduler::new(store);
        let stats = scheduler.stats().await;

        assert_eq!(stats.total, 0);
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.running, 0);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
