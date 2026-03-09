//! Secret Manager with memory-protected cache.
//!
//! The SecretManager provides:
//! - Single-point keychain access (solves macOS popup issue)
//! - In-memory cache with mlock protection
//! - Automatic preloading at daemon start
//!
//! TODO(v0.16): Integrate stats and advanced cache management

#![allow(dead_code)]

use crate::secrets::{memory::LockedString, SpnKeyring};
use rustc_hash::FxHashMap;
use secrecy::SecretString;
use spn_client::{provider_to_env_var, KNOWN_PROVIDERS};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::DaemonError;

/// Manages secrets with secure in-memory caching.
///
/// The SecretManager is designed to be the sole accessor of the OS keychain,
/// caching secrets in memory to avoid repeated keychain prompts.
pub struct SecretManager {
    /// Cached secrets (provider -> value)
    cache: Arc<RwLock<FxHashMap<String, LockedString>>>,
    /// Whether to load secrets lazily on first request
    lazy_mode: std::sync::atomic::AtomicBool,
}

impl SecretManager {
    /// Create a new SecretManager.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(FxHashMap::default())),
            lazy_mode: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Set lazy loading mode.
    pub fn set_lazy_mode(&self, lazy: bool) {
        self.lazy_mode.store(lazy, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if lazy loading mode is enabled.
    pub fn is_lazy_mode(&self) -> bool {
        self.lazy_mode.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Preload all secrets from keychain into cache.
    ///
    /// This is called at daemon startup to populate the cache.
    /// It triggers a single keychain auth prompt (if needed) rather than
    /// multiple prompts throughout the session.
    pub async fn preload_all(&self) -> Result<usize, DaemonError> {
        let total = KNOWN_PROVIDERS.len();
        info!(
            "Preloading secrets from keychain ({} providers)...",
            total
        );
        let mut loaded = 0;
        let mut not_found = 0;
        let mut errors = 0;

        for (idx, provider) in KNOWN_PROVIDERS.iter().enumerate() {
            debug!(
                "[{}/{}] Loading {}...",
                idx + 1,
                total,
                provider.id
            );
            match self.load_from_keyring(provider.id).await {
                Ok(true) => {
                    loaded += 1;
                    info!("[{}/{}] Loaded: {}", idx + 1, total, provider.id);
                }
                Ok(false) => {
                    not_found += 1;
                    debug!("[{}/{}] Not found: {}", idx + 1, total, provider.id);
                }
                Err(e) => {
                    errors += 1;
                    warn!(
                        "[{}/{}] Error loading {}: {}",
                        idx + 1,
                        total,
                        provider.id,
                        e
                    );
                }
            }
        }

        info!(
            "Preload complete: {} loaded, {} not found, {} errors",
            loaded, not_found, errors
        );
        Ok(loaded)
    }

    /// Load a secret from keyring into cache.
    ///
    /// Returns Ok(true) if loaded, Ok(false) if not found, Err on error.
    ///
    /// Note: Keyring access is blocking, so we use spawn_blocking to avoid
    /// blocking the async runtime.
    async fn load_from_keyring(&self, provider: &str) -> Result<bool, DaemonError> {
        // Clone provider for the blocking task
        let provider_owned = provider.to_string();

        // Run blocking keychain operation in a dedicated thread
        let result = tokio::task::spawn_blocking(move || SpnKeyring::get(&provider_owned))
            .await
            .map_err(|e| DaemonError::KeychainError(format!("Spawn blocking failed: {}", e)))?;

        match result {
            Ok(secret) => {
                // Create locked string (mlock protected)
                // secret is Zeroizing<String>, we need to dereference to get &str
                let locked = LockedString::from_str(&secret)
                    .map_err(|e| DaemonError::MemoryLockFailed(e.to_string()))?;

                // Store in cache
                let mut cache = self.cache.write().await;
                cache.insert(provider.to_string(), locked);

                Ok(true)
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("NotFound") {
                    Ok(false)
                } else {
                    Err(DaemonError::KeychainError(err_str))
                }
            }
        }
    }

    /// Get a cached secret.
    ///
    /// Returns None if the secret is not in the cache.
    pub async fn get_cached(&self, provider: &str) -> Option<SecretString> {
        let cache = self.cache.read().await;
        cache
            .get(provider)
            .map(|locked| SecretString::from(locked.as_str().to_string()))
    }

    /// Get a secret, loading from keychain if not cached (lazy mode).
    ///
    /// This is the primary method for getting secrets in lazy mode.
    /// In eager mode (preloaded), this behaves like `get_cached()`.
    pub async fn get_or_load(&self, provider: &str) -> Result<Option<SecretString>, DaemonError> {
        // First, try to get from cache
        if let Some(secret) = self.get_cached(provider).await {
            return Ok(Some(secret));
        }

        // If not in cache, try to load from keychain (lazy loading)
        debug!("Lazy loading secret for {}", provider);
        match self.load_from_keyring(provider).await {
            Ok(true) => {
                // Now it's in cache, get it
                Ok(self.get_cached(provider).await)
            }
            Ok(false) => Ok(None), // Not found in keychain
            Err(e) => Err(e),
        }
    }

    /// Reload a specific secret from keychain into cache.
    ///
    /// Used for cache invalidation after `spn provider set`.
    /// Returns Ok(true) if reloaded, Ok(false) if not found, Err on error.
    pub async fn reload_secret(&self, provider: &str) -> Result<bool, DaemonError> {
        info!("Reloading secret for {}", provider);

        // Remove from cache first
        {
            let mut cache = self.cache.write().await;
            cache.remove(provider);
        }

        // Load fresh from keyring
        self.load_from_keyring(provider).await
    }

    /// Check if a secret is cached.
    pub async fn has_cached(&self, provider: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(provider)
    }

    /// List all cached provider names.
    pub async fn list_cached(&self) -> Vec<String> {
        let cache = self.cache.read().await;
        cache.keys().cloned().collect()
    }

    /// Build environment variables for a process.
    ///
    /// Returns a map of env var names to values for the requested providers.
    /// Used when spawning MCP servers or other processes that need secrets.
    pub async fn build_env_for_process(&self, needed: &[&str]) -> FxHashMap<String, String> {
        let mut env = FxHashMap::default();
        let cache = self.cache.read().await;

        for provider in needed {
            if let Some(locked) = cache.get(*provider) {
                if let Some(env_var) = provider_to_env_var(provider) {
                    env.insert(env_var.to_string(), locked.as_str().to_string());
                }
            }
        }

        env
    }

    /// Store a secret (for testing or manual set).
    #[cfg(test)]
    pub async fn set_cached(&self, provider: &str, value: &str) -> Result<(), DaemonError> {
        let locked = LockedString::from_str(value)
            .map_err(|e| DaemonError::MemoryLockFailed(e.to_string()))?;

        let mut cache = self.cache.write().await;
        cache.insert(provider.to_string(), locked);
        Ok(())
    }

    /// Clear all cached secrets (for shutdown).
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Secret cache cleared");
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> SecretManagerStats {
        let cache = self.cache.read().await;
        SecretManagerStats {
            cached_count: cache.len(),
            providers: cache.keys().cloned().collect(),
        }
    }
}

impl Default for SecretManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the secret manager.
#[derive(Debug, Clone)]
pub struct SecretManagerStats {
    /// Number of cached secrets
    pub cached_count: usize,
    /// List of cached provider names
    pub providers: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[tokio::test]
    async fn test_secret_manager_cache() {
        let manager = SecretManager::new();

        // Initially empty
        assert!(!manager.has_cached("test").await);
        assert!(manager.get_cached("test").await.is_none());

        // Add a secret
        manager.set_cached("test", "secret-value").await.unwrap();

        // Now it should exist
        assert!(manager.has_cached("test").await);
        let secret = manager.get_cached("test").await.unwrap();
        assert_eq!(secret.expose_secret(), "secret-value");
    }

    #[tokio::test]
    async fn test_list_cached() {
        let manager = SecretManager::new();

        manager.set_cached("anthropic", "key1").await.unwrap();
        manager.set_cached("openai", "key2").await.unwrap();

        let providers = manager.list_cached().await;
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"anthropic".to_string()));
        assert!(providers.contains(&"openai".to_string()));
    }

    #[tokio::test]
    async fn test_build_env_for_process() {
        let manager = SecretManager::new();

        manager
            .set_cached("anthropic", "sk-ant-test")
            .await
            .unwrap();
        manager
            .set_cached("openai", "sk-openai-test")
            .await
            .unwrap();

        let env = manager
            .build_env_for_process(&["anthropic", "openai"])
            .await;

        assert_eq!(
            env.get("ANTHROPIC_API_KEY"),
            Some(&"sk-ant-test".to_string())
        );
        assert_eq!(
            env.get("OPENAI_API_KEY"),
            Some(&"sk-openai-test".to_string())
        );
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let manager = SecretManager::new();

        manager.set_cached("test", "value").await.unwrap();
        assert!(manager.has_cached("test").await);

        manager.clear_cache().await;
        assert!(!manager.has_cached("test").await);
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = SecretManager::new();

        manager.set_cached("anthropic", "key").await.unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.cached_count, 1);
        assert!(stats.providers.contains(&"anthropic".to_string()));
    }
}
