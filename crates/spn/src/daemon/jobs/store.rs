//! Persistent job storage.

#![allow(dead_code)]

use super::types::{Job, JobId, JobStatus};
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Persistent job store.
#[derive(Debug)]
pub struct JobStore {
    /// In-memory job index.
    jobs: RwLock<FxHashMap<JobId, JobStatus>>,
    /// Storage directory.
    storage_dir: PathBuf,
}

impl JobStore {
    /// Create a new job store.
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        let storage_dir = storage_dir.into();
        Self {
            jobs: RwLock::new(FxHashMap::default()),
            storage_dir,
        }
    }

    /// Initialize the store (create directory, load existing jobs).
    pub async fn init(&self) -> std::io::Result<()> {
        // Create storage directory
        tokio::fs::create_dir_all(&self.storage_dir).await?;

        // Load existing jobs from disk
        self.load_jobs().await?;

        Ok(())
    }

    /// Load jobs from disk.
    async fn load_jobs(&self) -> std::io::Result<()> {
        let jobs_file = self.storage_dir.join("jobs.json");
        if !jobs_file.exists() {
            debug!("No existing jobs file found");
            return Ok(());
        }

        match tokio::fs::read_to_string(&jobs_file).await {
            Ok(content) => match serde_json::from_str::<Vec<JobStatus>>(&content) {
                Ok(job_list) => {
                    let mut jobs = self.jobs.write().await;
                    for status in job_list {
                        jobs.insert(status.job.id, status);
                    }
                    debug!(count = jobs.len(), "Loaded jobs from disk");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse jobs file");
                }
            },
            Err(e) => {
                warn!(error = %e, "Failed to read jobs file");
            }
        }

        Ok(())
    }

    /// Save jobs to disk.
    async fn save_jobs(&self) -> std::io::Result<()> {
        let jobs = self.jobs.read().await;
        let job_list: Vec<_> = jobs.values().cloned().collect();
        drop(jobs);

        let content = serde_json::to_string_pretty(&job_list).map_err(std::io::Error::other)?;

        let jobs_file = self.storage_dir.join("jobs.json");
        tokio::fs::write(&jobs_file, content).await?;

        debug!("Saved jobs to disk");
        Ok(())
    }

    /// Add a new job.
    pub async fn add(&self, job: Job) -> JobStatus {
        let status = JobStatus::new(job.clone());
        let id = job.id;

        let mut jobs = self.jobs.write().await;
        jobs.insert(id, status.clone());
        drop(jobs);

        // Persist to disk
        if let Err(e) = self.save_jobs().await {
            error!(error = %e, "Failed to save jobs");
        }

        debug!(job_id = %id, "Added job to store");
        status
    }

    /// Get a job by ID.
    pub async fn get(&self, id: &JobId) -> Option<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.get(id).cloned()
    }

    /// Update a job status.
    pub async fn update(&self, status: JobStatus) {
        let id = status.job.id;
        let mut jobs = self.jobs.write().await;
        jobs.insert(id, status);
        drop(jobs);

        // Persist to disk
        if let Err(e) = self.save_jobs().await {
            error!(error = %e, "Failed to save jobs after update");
        }
    }

    /// List all jobs.
    pub async fn list(&self) -> Vec<JobStatus> {
        let jobs = self.jobs.read().await;
        let mut list: Vec<_> = jobs.values().cloned().collect();
        // Sort by creation time (newest first)
        list.sort_by(|a, b| b.job.created_at.cmp(&a.job.created_at));
        list
    }

    /// List pending jobs (sorted by priority).
    pub async fn list_pending(&self) -> Vec<JobStatus> {
        let jobs = self.jobs.read().await;
        let mut pending: Vec<_> = jobs
            .values()
            .filter(|s| s.state == super::types::JobState::Pending)
            .cloned()
            .collect();
        // Sort by priority (highest first), then by creation time (oldest first)
        pending.sort_by(|a, b| {
            b.job
                .priority
                .cmp(&a.job.priority)
                .then_with(|| a.job.created_at.cmp(&b.job.created_at))
        });
        pending
    }

    /// List running jobs.
    pub async fn list_running(&self) -> Vec<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.values()
            .filter(|s| s.state == super::types::JobState::Running)
            .cloned()
            .collect()
    }

    /// Remove old completed jobs (older than given duration).
    pub async fn cleanup(&self, max_age_secs: u64) -> usize {
        let cutoff = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(max_age_secs))
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let mut jobs = self.jobs.write().await;
        let initial_count = jobs.len();

        jobs.retain(|_, status| {
            if status.is_terminal() {
                if let Some(ended) = status.ended_at {
                    return ended > cutoff;
                }
            }
            true
        });

        let removed = initial_count - jobs.len();
        drop(jobs);

        if removed > 0 {
            if let Err(e) = self.save_jobs().await {
                error!(error = %e, "Failed to save jobs after cleanup");
            }
            debug!(removed, "Cleaned up old jobs");
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_job_store_basic() {
        let temp_dir = std::env::temp_dir().join(format!("spn-jobs-test-{}", uuid::Uuid::new_v4()));
        let store = JobStore::new(&temp_dir);
        store.init().await.unwrap();

        // Add a job
        let job = Job::new(PathBuf::from("test.yaml")).with_name("Test");
        let status = store.add(job.clone()).await;
        assert_eq!(status.job.id, job.id);

        // Get the job
        let retrieved = store.get(&job.id).await.unwrap();
        assert_eq!(retrieved.job.name, Some("Test".into()));

        // List jobs
        let list = store.list().await;
        assert_eq!(list.len(), 1);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
