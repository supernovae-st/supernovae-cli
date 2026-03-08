//! Trace storage and retrieval.

#![allow(dead_code)]

use super::types::{ReasoningTrace, TraceId, TraceSummary};
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Storage for reasoning traces.
#[derive(Debug)]
pub struct TraceStore {
    /// In-memory cache of active traces.
    active: RwLock<FxHashMap<TraceId, ReasoningTrace>>,
    /// In-memory cache of completed traces (limited size).
    completed: RwLock<Vec<ReasoningTrace>>,
    /// Storage directory.
    storage_dir: PathBuf,
    /// Maximum completed traces to keep in memory.
    max_in_memory: usize,
    /// Maximum traces to keep on disk.
    max_on_disk: usize,
}

impl TraceStore {
    /// Create a new trace store.
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            active: RwLock::new(FxHashMap::default()),
            completed: RwLock::new(Vec::new()),
            storage_dir: storage_dir.into(),
            max_in_memory: 100,
            max_on_disk: 1000,
        }
    }

    /// Initialize the store.
    pub async fn init(&self) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.storage_dir).await?;
        self.load_recent().await?;
        Ok(())
    }

    /// Load recent traces from disk.
    async fn load_recent(&self) -> std::io::Result<()> {
        let index_file = self.storage_dir.join("traces_index.json");
        if !index_file.exists() {
            return Ok(());
        }

        match tokio::fs::read_to_string(&index_file).await {
            Ok(content) => {
                match serde_json::from_str::<Vec<TraceSummary>>(&content) {
                    Ok(summaries) => {
                        debug!(count = summaries.len(), "Loaded trace index");
                        // We only load the index, actual traces loaded on demand
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to parse trace index");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to read trace index");
            }
        }

        Ok(())
    }

    /// Start a new trace.
    pub async fn start(&self, trace: ReasoningTrace) -> TraceId {
        let id = trace.id;
        let mut active = self.active.write().await;
        active.insert(id, trace);
        debug!(?id, "Started new trace");
        id
    }

    /// Get an active trace for modification.
    pub async fn get_active(&self, id: &TraceId) -> Option<ReasoningTrace> {
        let active = self.active.read().await;
        active.get(id).cloned()
    }

    /// Update an active trace.
    pub async fn update(&self, trace: ReasoningTrace) {
        let mut active = self.active.write().await;
        if active.contains_key(&trace.id) {
            active.insert(trace.id, trace);
        }
    }

    /// Complete a trace (move from active to completed).
    pub async fn complete(&self, id: &TraceId) -> Option<ReasoningTrace> {
        let mut active = self.active.write().await;
        if let Some(mut trace) = active.remove(id) {
            trace.complete();
            drop(active);

            // Save to disk
            if let Err(e) = self.save_trace(&trace).await {
                error!(error = %e, "Failed to save completed trace");
            }

            // Add to completed cache
            let mut completed = self.completed.write().await;
            completed.push(trace.clone());

            // Enforce memory limit
            if completed.len() > self.max_in_memory {
                completed.remove(0);
            }

            Some(trace)
        } else {
            None
        }
    }

    /// Fail a trace.
    pub async fn fail(&self, id: &TraceId, error: impl Into<String>) -> Option<ReasoningTrace> {
        let mut active = self.active.write().await;
        if let Some(mut trace) = active.remove(id) {
            trace.fail(error);
            drop(active);

            // Save to disk
            if let Err(e) = self.save_trace(&trace).await {
                tracing::error!(error = %e, "Failed to save failed trace");
            }

            // Add to completed cache
            let mut completed = self.completed.write().await;
            completed.push(trace.clone());

            Some(trace)
        } else {
            None
        }
    }

    /// Save a trace to disk.
    async fn save_trace(&self, trace: &ReasoningTrace) -> std::io::Result<()> {
        let trace_file = self.storage_dir.join(format!("{}.json", trace.id));
        let content = serde_json::to_string_pretty(trace).map_err(std::io::Error::other)?;
        tokio::fs::write(&trace_file, content).await?;

        // Update index
        self.update_index().await?;

        debug!(id = %trace.id, "Saved trace to disk");
        Ok(())
    }

    /// Update the traces index file.
    async fn update_index(&self) -> std::io::Result<()> {
        let completed = self.completed.read().await;
        let summaries: Vec<TraceSummary> = completed.iter().map(TraceSummary::from).collect();
        drop(completed);

        let index_file = self.storage_dir.join("traces_index.json");
        let content = serde_json::to_string_pretty(&summaries).map_err(std::io::Error::other)?;
        tokio::fs::write(&index_file, content).await?;

        Ok(())
    }

    /// Load a trace from disk.
    pub async fn load(&self, id: &TraceId) -> std::io::Result<Option<ReasoningTrace>> {
        // Check completed cache first
        {
            let completed = self.completed.read().await;
            if let Some(trace) = completed.iter().find(|t| &t.id == id) {
                return Ok(Some(trace.clone()));
            }
        }

        // Check active
        {
            let active = self.active.read().await;
            if let Some(trace) = active.get(id) {
                return Ok(Some(trace.clone()));
            }
        }

        // Load from disk
        let trace_file = self.storage_dir.join(format!("{}.json", id));
        if !trace_file.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&trace_file).await?;
        let trace: ReasoningTrace =
            serde_json::from_str(&content).map_err(std::io::Error::other)?;

        Ok(Some(trace))
    }

    /// List all traces (summaries).
    pub async fn list(&self, limit: Option<usize>) -> Vec<TraceSummary> {
        let completed = self.completed.read().await;
        let limit = limit.unwrap_or(50);

        completed
            .iter()
            .rev() // Most recent first
            .take(limit)
            .map(TraceSummary::from)
            .collect()
    }

    /// List active traces.
    pub async fn list_active(&self) -> Vec<TraceSummary> {
        let active = self.active.read().await;
        active.values().map(TraceSummary::from).collect()
    }

    /// Search traces by task description.
    pub async fn search(&self, query: &str) -> Vec<TraceSummary> {
        let query_lower = query.to_lowercase();
        let completed = self.completed.read().await;

        completed
            .iter()
            .filter(|t| {
                t.metadata.task.to_lowercase().contains(&query_lower)
                    || t.metadata
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(TraceSummary::from)
            .collect()
    }

    /// Search traces by model.
    pub async fn search_by_model(&self, model: &str) -> Vec<TraceSummary> {
        let completed = self.completed.read().await;

        completed
            .iter()
            .filter(|t| t.metadata.model.contains(model))
            .map(TraceSummary::from)
            .collect()
    }

    /// Search traces by provider.
    pub async fn search_by_provider(&self, provider: &str) -> Vec<TraceSummary> {
        let completed = self.completed.read().await;

        completed
            .iter()
            .filter(|t| t.metadata.provider == provider)
            .map(TraceSummary::from)
            .collect()
    }

    /// Get statistics about traces.
    pub async fn stats(&self) -> TraceStats {
        let active = self.active.read().await;
        let completed = self.completed.read().await;

        let total_tokens: u32 = completed.iter().filter_map(|t| t.total_tokens).sum();
        let total_duration: f64 = completed
            .iter()
            .filter_map(|t| t.total_duration)
            .map(|d| d.as_secs_f64())
            .sum();
        let success_count = completed.iter().filter(|t| t.success).count();
        let failure_count = completed.iter().filter(|t| !t.success).count();

        // Count by provider
        let mut by_provider = FxHashMap::default();
        for trace in completed.iter() {
            *by_provider
                .entry(trace.metadata.provider.clone())
                .or_insert(0) += 1;
        }

        // Count by model
        let mut by_model = FxHashMap::default();
        for trace in completed.iter() {
            *by_model.entry(trace.metadata.model.clone()).or_insert(0) += 1;
        }

        TraceStats {
            active_count: active.len(),
            completed_count: completed.len(),
            success_count,
            failure_count,
            total_tokens,
            total_duration_secs: total_duration,
            by_provider,
            by_model,
        }
    }

    /// Cleanup old traces.
    pub async fn cleanup(&self, keep_count: usize) -> std::io::Result<usize> {
        let mut completed = self.completed.write().await;

        if completed.len() <= keep_count {
            return Ok(0);
        }

        let to_remove = completed.len() - keep_count;
        let removed: Vec<_> = completed.drain(..to_remove).collect();

        // Delete files
        for trace in &removed {
            let trace_file = self.storage_dir.join(format!("{}.json", trace.id));
            if trace_file.exists() {
                tokio::fs::remove_file(&trace_file).await?;
            }
        }

        drop(completed);

        // Update index
        self.update_index().await?;

        debug!(removed = removed.len(), "Cleaned up old traces");
        Ok(removed.len())
    }

    /// Delete a specific trace.
    pub async fn delete(&self, id: &TraceId) -> std::io::Result<bool> {
        // Remove from completed cache
        {
            let mut completed = self.completed.write().await;
            if let Some(pos) = completed.iter().position(|t| &t.id == id) {
                completed.remove(pos);
            }
        }

        // Delete file
        let trace_file = self.storage_dir.join(format!("{}.json", id));
        if trace_file.exists() {
            tokio::fs::remove_file(&trace_file).await?;
            self.update_index().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Statistics about traces.
#[derive(Debug, Clone)]
pub struct TraceStats {
    /// Active trace count.
    pub active_count: usize,
    /// Completed trace count.
    pub completed_count: usize,
    /// Successful trace count.
    pub success_count: usize,
    /// Failed trace count.
    pub failure_count: usize,
    /// Total tokens used.
    pub total_tokens: u32,
    /// Total duration in seconds.
    pub total_duration_secs: f64,
    /// Count by provider.
    pub by_provider: FxHashMap<String, usize>,
    /// Count by model.
    pub by_model: FxHashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::traces::types::{TraceMetadata, TraceStep, TraceStepKind};

    #[tokio::test]
    async fn test_trace_store_basic() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-traces-test-{}", uuid::Uuid::new_v4()));
        let store = TraceStore::new(&temp_dir);
        store.init().await.unwrap();

        // Start a trace
        let metadata = TraceMetadata::new("claude-3", "anthropic", "Test task");
        let mut trace = ReasoningTrace::new(metadata);
        trace.add_step(TraceStep::new(TraceStepKind::Thinking, "Analyzing..."));
        let id = store.start(trace).await;

        // Get active
        let active = store.get_active(&id).await;
        assert!(active.is_some());

        // List active
        let active_list = store.list_active().await;
        assert_eq!(active_list.len(), 1);

        // Complete
        let completed = store.complete(&id).await;
        assert!(completed.is_some());
        assert!(completed.unwrap().success);

        // List completed
        let list = store.list(None).await;
        assert_eq!(list.len(), 1);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_trace_store_failure() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-traces-test-{}", uuid::Uuid::new_v4()));
        let store = TraceStore::new(&temp_dir);
        store.init().await.unwrap();

        let metadata = TraceMetadata::new("gpt-4", "openai", "Failing task");
        let trace = ReasoningTrace::new(metadata);
        let id = store.start(trace).await;

        let failed = store.fail(&id, "Something went wrong").await;
        assert!(failed.is_some());
        assert!(!failed.unwrap().success);

        let stats = store.stats().await;
        assert_eq!(stats.failure_count, 1);

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_trace_store_search() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-traces-test-{}", uuid::Uuid::new_v4()));
        let store = TraceStore::new(&temp_dir);
        store.init().await.unwrap();

        // Add traces with different tasks
        for task in &["Build feature", "Fix bug", "Review code"] {
            let metadata = TraceMetadata::new("claude-3", "anthropic", *task);
            let trace = ReasoningTrace::new(metadata);
            let id = store.start(trace).await;
            store.complete(&id).await;
        }

        // Search
        let results = store.search("bug").await;
        assert_eq!(results.len(), 1);
        assert!(results[0].task.contains("bug"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_trace_store_stats() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-traces-test-{}", uuid::Uuid::new_v4()));
        let store = TraceStore::new(&temp_dir);
        store.init().await.unwrap();

        // Add traces from different providers
        for (model, provider) in &[("claude-3", "anthropic"), ("gpt-4", "openai")] {
            let metadata = TraceMetadata::new(*model, *provider, "Task");
            let mut trace = ReasoningTrace::new(metadata);
            trace.add_step(TraceStep::new(TraceStepKind::Thinking, "...").with_tokens(100));
            let id = store.start(trace).await;
            store.complete(&id).await;
        }

        let stats = store.stats().await;
        assert_eq!(stats.completed_count, 2);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.total_tokens, 200);
        assert_eq!(stats.by_provider.len(), 2);

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
