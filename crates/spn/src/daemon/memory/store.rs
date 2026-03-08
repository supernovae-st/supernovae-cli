//! Persistent memory store.

#![allow(dead_code)]

use super::types::{MemoryEntry, MemoryKey, MemoryNamespace};
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Persistent memory store.
#[derive(Debug)]
pub struct MemoryStore {
    /// In-memory cache.
    cache: RwLock<FxHashMap<String, MemoryEntry>>,
    /// Storage directory.
    storage_dir: PathBuf,
    /// Maximum entries per namespace.
    max_entries_per_namespace: usize,
}

impl MemoryStore {
    /// Create a new memory store.
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache: RwLock::new(FxHashMap::default()),
            storage_dir: storage_dir.into(),
            max_entries_per_namespace: 1000,
        }
    }

    /// Initialize the store.
    pub async fn init(&self) -> std::io::Result<()> {
        // Create storage directory
        tokio::fs::create_dir_all(&self.storage_dir).await?;

        // Load existing entries
        self.load_entries().await?;

        // Clean expired entries
        self.cleanup_expired().await;

        Ok(())
    }

    /// Load entries from disk.
    async fn load_entries(&self) -> std::io::Result<()> {
        let memory_file = self.storage_dir.join("memory.json");
        if !memory_file.exists() {
            debug!("No existing memory file found");
            return Ok(());
        }

        match tokio::fs::read_to_string(&memory_file).await {
            Ok(content) => match serde_json::from_str::<Vec<MemoryEntry>>(&content) {
                Ok(entries) => {
                    let mut cache = self.cache.write().await;
                    for entry in entries {
                        if !entry.is_expired() {
                            cache.insert(entry.key.path(), entry);
                        }
                    }
                    debug!(count = cache.len(), "Loaded memory entries from disk");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse memory file");
                }
            },
            Err(e) => {
                warn!(error = %e, "Failed to read memory file");
            }
        }

        Ok(())
    }

    /// Save entries to disk.
    async fn save_entries(&self) -> std::io::Result<()> {
        let cache = self.cache.read().await;
        let entries: Vec<_> = cache.values().cloned().collect();
        drop(cache);

        let content = serde_json::to_string_pretty(&entries).map_err(std::io::Error::other)?;

        let memory_file = self.storage_dir.join("memory.json");
        tokio::fs::write(&memory_file, content).await?;

        debug!("Saved memory entries to disk");
        Ok(())
    }

    /// Get an entry by key.
    pub async fn get(&self, key: &MemoryKey) -> Option<MemoryEntry> {
        let path = key.path();
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(&path) {
            if entry.is_expired() {
                return None;
            }
            return Some(entry.clone());
        }
        None
    }

    /// Get an entry and record access.
    pub async fn get_and_touch(&self, key: &MemoryKey) -> Option<MemoryEntry> {
        let path = key.path();
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get_mut(&path) {
            if entry.is_expired() {
                cache.remove(&path);
                return None;
            }
            entry.touch();
            return Some(entry.clone());
        }
        None
    }

    /// Set an entry.
    pub async fn set(&self, entry: MemoryEntry) {
        let path = entry.key.path();
        let mut cache = self.cache.write().await;
        cache.insert(path, entry);
        drop(cache);

        // Persist to disk
        if let Err(e) = self.save_entries().await {
            error!(error = %e, "Failed to save memory entries");
        }
    }

    /// Update an existing entry or create if not exists.
    pub async fn upsert(&self, key: MemoryKey, value: serde_json::Value) -> MemoryEntry {
        let path = key.path();
        let mut cache = self.cache.write().await;

        let entry = if let Some(existing) = cache.get_mut(&path) {
            existing.update(value);
            existing.clone()
        } else {
            let entry = MemoryEntry::new(key, value);
            cache.insert(path, entry.clone());
            entry
        };
        drop(cache);

        // Persist to disk
        if let Err(e) = self.save_entries().await {
            error!(error = %e, "Failed to save memory entries");
        }

        entry
    }

    /// Delete an entry.
    pub async fn delete(&self, key: &MemoryKey) -> bool {
        let path = key.path();
        let mut cache = self.cache.write().await;
        let removed = cache.remove(&path).is_some();
        drop(cache);

        if removed {
            if let Err(e) = self.save_entries().await {
                error!(error = %e, "Failed to save after delete");
            }
        }

        removed
    }

    /// List entries in a namespace.
    pub async fn list(&self, namespace: MemoryNamespace) -> Vec<MemoryEntry> {
        let prefix = format!("{}/", namespace);
        let cache = self.cache.read().await;

        cache
            .iter()
            .filter(|(path, entry)| path.starts_with(&prefix) && !entry.is_expired())
            .map(|(_, entry)| entry.clone())
            .collect()
    }

    /// List entries with specific tags.
    pub async fn list_by_tags(&self, tags: &[String]) -> Vec<MemoryEntry> {
        let cache = self.cache.read().await;

        cache
            .values()
            .filter(|entry| !entry.is_expired() && tags.iter().all(|t| entry.tags.contains(t)))
            .cloned()
            .collect()
    }

    /// Search entries by value content.
    pub async fn search(&self, query: &str) -> Vec<MemoryEntry> {
        let query_lower = query.to_lowercase();
        let cache = self.cache.read().await;

        cache
            .values()
            .filter(|entry| {
                if entry.is_expired() {
                    return false;
                }
                // Search in value JSON
                let value_str = entry.value.to_string().to_lowercase();
                value_str.contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Clean up expired entries.
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_count = cache.len();

        cache.retain(|_, entry| !entry.is_expired());

        let removed = initial_count - cache.len();
        drop(cache);

        if removed > 0 {
            if let Err(e) = self.save_entries().await {
                error!(error = %e, "Failed to save after cleanup");
            }
            debug!(removed, "Cleaned up expired memory entries");
        }

        removed
    }

    /// Enforce namespace limits (keep most recently accessed).
    pub async fn enforce_limits(&self) -> usize {
        let mut cache = self.cache.write().await;
        let mut removed = 0;

        // Group by namespace
        let namespaces = [
            MemoryNamespace::Preferences,
            MemoryNamespace::CommandHistory,
            MemoryNamespace::ProjectContext,
            MemoryNamespace::ConversationSummary,
            MemoryNamespace::Analytics,
        ];

        for ns in namespaces {
            let prefix = format!("{}/", ns);
            let mut ns_entries: Vec<_> = cache
                .iter()
                .filter(|(path, _)| path.starts_with(&prefix))
                .map(|(path, entry)| (path.clone(), entry.updated_at))
                .collect();

            if ns_entries.len() > self.max_entries_per_namespace {
                // Sort by updated_at (oldest first)
                ns_entries.sort_by_key(|(_, updated)| *updated);

                // Remove oldest entries
                let to_remove = ns_entries.len() - self.max_entries_per_namespace;
                for (path, _) in ns_entries.into_iter().take(to_remove) {
                    cache.remove(&path);
                    removed += 1;
                }
            }
        }

        drop(cache);

        if removed > 0 {
            if let Err(e) = self.save_entries().await {
                error!(error = %e, "Failed to save after enforcing limits");
            }
            debug!(removed, "Enforced namespace limits");
        }

        removed
    }

    /// Get statistics about the memory store.
    pub async fn stats(&self) -> MemoryStats {
        let cache = self.cache.read().await;

        let mut by_namespace = FxHashMap::default();
        let mut total_size = 0;
        let mut expired_count = 0;

        for entry in cache.values() {
            if entry.is_expired() {
                expired_count += 1;
                continue;
            }

            let ns = entry.key.namespace;
            *by_namespace.entry(ns).or_insert(0usize) += 1;
            total_size += entry.value.to_string().len();
        }

        MemoryStats {
            total_entries: cache.len() - expired_count,
            expired_entries: expired_count,
            by_namespace,
            total_size_bytes: total_size,
        }
    }
}

/// Statistics about the memory store.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total active entries.
    pub total_entries: usize,
    /// Expired entries pending cleanup.
    pub expired_entries: usize,
    /// Entries by namespace.
    pub by_namespace: FxHashMap<MemoryNamespace, usize>,
    /// Approximate total size in bytes.
    pub total_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_basic() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-memory-test-{}", uuid::Uuid::new_v4()));
        let store = MemoryStore::new(&temp_dir);
        store.init().await.unwrap();

        // Set an entry
        let key = MemoryKey::preference("theme");
        let entry = MemoryEntry::new(key.clone(), serde_json::json!("dark"));
        store.set(entry).await;

        // Get the entry
        let retrieved = store.get(&key).await.unwrap();
        assert_eq!(retrieved.value, serde_json::json!("dark"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_memory_store_upsert() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-memory-test-{}", uuid::Uuid::new_v4()));
        let store = MemoryStore::new(&temp_dir);
        store.init().await.unwrap();

        let key = MemoryKey::preference("counter");

        // First upsert creates
        let entry1 = store.upsert(key.clone(), serde_json::json!(1)).await;
        assert_eq!(entry1.value, serde_json::json!(1));

        // Second upsert updates
        let entry2 = store.upsert(key.clone(), serde_json::json!(2)).await;
        assert_eq!(entry2.value, serde_json::json!(2));

        // Verify
        let retrieved = store.get(&key).await.unwrap();
        assert_eq!(retrieved.value, serde_json::json!(2));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_memory_store_list() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-memory-test-{}", uuid::Uuid::new_v4()));
        let store = MemoryStore::new(&temp_dir);
        store.init().await.unwrap();

        // Add entries to different namespaces
        store
            .set(MemoryEntry::new(
                MemoryKey::preference("a"),
                serde_json::json!("1"),
            ))
            .await;
        store
            .set(MemoryEntry::new(
                MemoryKey::preference("b"),
                serde_json::json!("2"),
            ))
            .await;
        store
            .set(MemoryEntry::new(
                MemoryKey::command("x"),
                serde_json::json!("cmd"),
            ))
            .await;

        // List preferences
        let prefs = store.list(MemoryNamespace::Preferences).await;
        assert_eq!(prefs.len(), 2);

        // List commands
        let cmds = store.list(MemoryNamespace::CommandHistory).await;
        assert_eq!(cmds.len(), 1);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_memory_store_delete() {
        let temp_dir =
            std::env::temp_dir().join(format!("spn-memory-test-{}", uuid::Uuid::new_v4()));
        let store = MemoryStore::new(&temp_dir);
        store.init().await.unwrap();

        let key = MemoryKey::preference("temp");
        store
            .set(MemoryEntry::new(key.clone(), serde_json::json!("value")))
            .await;

        // Verify exists
        assert!(store.get(&key).await.is_some());

        // Delete
        assert!(store.delete(&key).await);

        // Verify gone
        assert!(store.get(&key).await.is_none());

        // Delete non-existent
        assert!(!store.delete(&key).await);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
