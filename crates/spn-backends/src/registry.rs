//! Backend registry for managing multiple backends.
//!
//! The registry holds references to all available backends and provides
//! lookup by kind or identifier.

use crate::cloud::BoxedCloudBackend;
use crate::traits::BoxedUnifiedBackend;
use crate::{BackendKind, BackendsError, BoxedBackend};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing multiple backends.
///
/// The registry allows registering local model backends (Ollama, llama.cpp)
/// and cloud backends (Anthropic, OpenAI, etc.), then looking them up
/// by kind or identifier.
///
/// # Thread Safety
///
/// The registry uses `Arc<RwLock<...>>` internally for safe concurrent access.
/// Multiple threads can read from the registry simultaneously, while writes
/// are exclusive.
///
/// # Example
///
/// ```rust,ignore
/// use spn_backends::{BackendRegistry, BackendKind};
///
/// let mut registry = BackendRegistry::new();
///
/// // Register backends
/// registry.register_cloud(anthropic_backend);
/// registry.register_cloud(openai_backend);
///
/// // Lookup by kind
/// if let Some(backend) = registry.get_cloud(BackendKind::Anthropic) {
///     let response = backend.chat("claude-sonnet-4-20250514", &messages, None).await?;
/// }
/// ```
#[derive(Default)]
pub struct BackendRegistry {
    /// Local model backends (Ollama, llama.cpp).
    local_backends: FxHashMap<BackendKind, BoxedBackend>,

    /// Cloud backends (Anthropic, OpenAI, etc.).
    cloud_backends: FxHashMap<BackendKind, BoxedCloudBackend>,

    /// Unified backends for generic access.
    unified_backends: FxHashMap<BackendKind, BoxedUnifiedBackend>,
}

impl BackendRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a local model backend.
    pub fn register_local(&mut self, kind: BackendKind, backend: BoxedBackend) {
        self.local_backends.insert(kind, backend);
    }

    /// Register a cloud backend.
    pub fn register_cloud(&mut self, kind: BackendKind, backend: BoxedCloudBackend) {
        self.cloud_backends.insert(kind, backend);
    }

    /// Register a unified backend.
    pub fn register_unified(&mut self, kind: BackendKind, backend: BoxedUnifiedBackend) {
        self.unified_backends.insert(kind, backend);
    }

    /// Get a local backend by kind.
    #[must_use]
    pub fn get_local(&self, kind: BackendKind) -> Option<&BoxedBackend> {
        self.local_backends.get(&kind)
    }

    /// Get a cloud backend by kind.
    #[must_use]
    pub fn get_cloud(&self, kind: BackendKind) -> Option<&BoxedCloudBackend> {
        self.cloud_backends.get(&kind)
    }

    /// Get a unified backend by kind.
    #[must_use]
    pub fn get_unified(&self, kind: BackendKind) -> Option<&BoxedUnifiedBackend> {
        self.unified_backends.get(&kind)
    }

    /// Check if a backend is registered.
    #[must_use]
    pub fn has_backend(&self, kind: BackendKind) -> bool {
        self.local_backends.contains_key(&kind)
            || self.cloud_backends.contains_key(&kind)
            || self.unified_backends.contains_key(&kind)
    }

    /// List all registered backend kinds.
    #[must_use]
    pub fn list_backends(&self) -> Vec<BackendKind> {
        let mut kinds: Vec<_> = self
            .local_backends
            .keys()
            .chain(self.cloud_backends.keys())
            .chain(self.unified_backends.keys())
            .copied()
            .collect();
        kinds.sort_by_key(|k| k.id());
        kinds.dedup();
        kinds
    }

    /// List all registered local backend kinds.
    #[must_use]
    pub fn list_local_backends(&self) -> Vec<BackendKind> {
        self.local_backends.keys().copied().collect()
    }

    /// List all registered cloud backend kinds.
    #[must_use]
    pub fn list_cloud_backends(&self) -> Vec<BackendKind> {
        self.cloud_backends.keys().copied().collect()
    }

    /// Remove a backend from the registry.
    pub fn remove(&mut self, kind: BackendKind) -> bool {
        let local = self.local_backends.remove(&kind).is_some();
        let cloud = self.cloud_backends.remove(&kind).is_some();
        let unified = self.unified_backends.remove(&kind).is_some();
        local || cloud || unified
    }

    /// Clear all registered backends.
    pub fn clear(&mut self) {
        self.local_backends.clear();
        self.cloud_backends.clear();
        self.unified_backends.clear();
    }

    /// Get the number of registered backends.
    #[must_use]
    pub fn len(&self) -> usize {
        self.local_backends.len() + self.cloud_backends.len() + self.unified_backends.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.local_backends.is_empty()
            && self.cloud_backends.is_empty()
            && self.unified_backends.is_empty()
    }
}

/// Thread-safe shared registry using Arc<RwLock>.
pub type SharedRegistry = Arc<RwLock<BackendRegistry>>;

/// Create a new shared registry.
#[must_use]
pub fn shared_registry() -> SharedRegistry {
    Arc::new(RwLock::new(BackendRegistry::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = BackendRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_has_backend() {
        let registry = BackendRegistry::new();
        assert!(!registry.has_backend(BackendKind::Anthropic));
        assert!(!registry.has_backend(BackendKind::Ollama));
    }

    #[test]
    fn test_registry_list_backends() {
        let registry = BackendRegistry::new();
        assert!(registry.list_backends().is_empty());
        assert!(registry.list_local_backends().is_empty());
        assert!(registry.list_cloud_backends().is_empty());
    }

    #[test]
    fn test_shared_registry() {
        let registry = shared_registry();
        assert!(registry.try_read().unwrap().is_empty());
    }
}
