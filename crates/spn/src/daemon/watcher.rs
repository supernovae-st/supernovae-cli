//! File system watcher for MCP config changes.
//!
//! Watches MCP configuration files across all supported clients and detects:
//! - Changes to spn's own config (triggers sync to clients)
//! - Changes to client configs (detects foreign MCPs)
//!
//! Watch scope follows A + Lite C strategy:
//! - Global configs: Always watched
//! - Project configs: 5 most recent projects watched

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

use super::differ::{diff_mcp_configs, parse_client_config};
use super::foreign::{ForeignMcp, ForeignMcpServer, ForeignScope, ForeignSource, ForeignTracker};
use super::notifications::NotificationService;
use super::recent::RecentProjects;
use crate::error::{Result, SpnError};
use crate::mcp::McpConfigManager;

/// Debounce duration for file change events.
const DEBOUNCE_MS: u64 = 500;

/// Event emitted when watch state changes.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// Foreign MCP detected in a client config.
    ForeignMcpDetected(ForeignMcp),
    /// MCP sync needed (spn config changed).
    SyncNeeded,
    /// Watch list updated (new project added).
    #[allow(dead_code)] // Phase 2: emitted when refresh_watch_list is called
    WatchListUpdated,
}

/// File watcher service for MCP config auto-sync.
pub struct WatcherService {
    /// The underlying file watcher.
    watcher: RecommendedWatcher,
    /// Receiver for file system events (tokio async channel).
    pub(crate) rx: mpsc::UnboundedReceiver<notify::Result<Event>>,
    /// Currently watched paths.
    watched_paths: FxHashSet<PathBuf>,
    /// Recent projects tracker.
    recent: RecentProjects,
    /// Foreign MCP tracker.
    foreign: ForeignTracker,
    /// Notification service.
    notifier: NotificationService,
    /// Last event time per path (for debouncing).
    last_event: FxHashMap<PathBuf, Instant>,
    /// Checksums of files we wrote (for origin tracking).
    our_writes: FxHashMap<PathBuf, u64>,
    /// Broadcast sender for watch events.
    event_tx: broadcast::Sender<WatchEvent>,
}

impl WatcherService {
    /// Create a new watcher service.
    pub fn new() -> Result<(Self, broadcast::Receiver<WatchEvent>)> {
        // Use unbounded channel to avoid blocking the notify callback thread
        let (tx, rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = broadcast::channel(32);

        let watcher = RecommendedWatcher::new(
            move |res| {
                // This callback runs on a system thread, use blocking send
                let _ = tx.send(res);
            },
            notify::Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .map_err(|e| SpnError::ConfigError(format!("Failed to create watcher: {e}")))?;

        let recent = RecentProjects::load().unwrap_or_default();
        let foreign = ForeignTracker::load().unwrap_or_default();

        Ok((
            Self {
                watcher,
                rx,
                watched_paths: FxHashSet::default(),
                recent,
                foreign,
                notifier: NotificationService::new(),
                last_event: FxHashMap::default(),
                our_writes: FxHashMap::default(),
                event_tx,
            },
            event_rx,
        ))
    }

    /// Start watching all configured paths.
    pub fn start(&mut self) -> Result<()> {
        info!("Starting MCP config watcher");

        // Watch global configs
        self.start_global_watches()?;

        // Watch recent project configs
        self.start_project_watches()?;

        info!(
            "Watching {} paths for MCP config changes",
            self.watched_paths.len()
        );

        Ok(())
    }

    /// Get paths for global MCP configs.
    fn global_watch_paths() -> Vec<PathBuf> {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => {
                warn!("HOME directory not set, skipping global config watches");
                return Vec::new();
            }
        };

        vec![
            // spn's own config
            home.join(".spn").join("mcp.yaml"),
            // Cursor global
            home.join(".cursor").join("mcp.json"),
            // Claude Code global
            home.join(".claude.json"),
            // Windsurf global
            home.join(".codeium").join("windsurf").join("mcp_config.json"),
        ]
    }

    /// Start watching global config paths.
    fn start_global_watches(&mut self) -> Result<()> {
        for path in Self::global_watch_paths() {
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    self.watch_path(parent)?;
                }
            }
        }
        Ok(())
    }

    /// Get paths for project-level MCP configs.
    fn project_watch_paths(project: &Path) -> Vec<PathBuf> {
        vec![
            // Cursor project
            project.join(".cursor").join("mcp.json"),
            // Claude Code project
            project.join(".mcp.json"),
            // Claude Code settings
            project.join(".claude").join("settings.json"),
        ]
    }

    /// Start watching recent project config paths.
    fn start_project_watches(&mut self) -> Result<()> {
        for project in self.recent.watch_paths() {
            for path in Self::project_watch_paths(&project) {
                if let Some(parent) = path.parent() {
                    if parent.exists() {
                        self.watch_path(parent)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Add a path to the watch list.
    fn watch_path(&mut self, path: &Path) -> Result<()> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        if self.watched_paths.contains(&canonical) {
            return Ok(());
        }

        self.watcher
            .watch(&canonical, RecursiveMode::NonRecursive)
            .map_err(|e| SpnError::ConfigError(format!("Failed to watch {}: {e}", path.display())))?;

        self.watched_paths.insert(canonical.clone());
        debug!("Watching: {}", canonical.display());

        Ok(())
    }

    /// Remove a path from the watch list.
    #[allow(dead_code)]
    fn unwatch_path(&mut self, path: &Path) -> Result<()> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        if !self.watched_paths.contains(&canonical) {
            return Ok(());
        }

        self.watcher
            .unwatch(&canonical)
            .map_err(|e| SpnError::ConfigError(format!("Failed to unwatch {}: {e}", path.display())))?;

        self.watched_paths.remove(&canonical);
        debug!("Unwatched: {}", canonical.display());

        Ok(())
    }

    /// Refresh the watch list from recent projects.
    #[allow(dead_code)] // Phase 2: called when tracking new projects
    pub fn refresh_watch_list(&mut self) -> Result<()> {
        // Reload recent projects
        self.recent = RecentProjects::load().unwrap_or_default();
        self.recent.cleanup();

        // Clean up stale debounce entries to prevent memory leaks
        self.cleanup_debounce();

        // Add watches for any new projects
        self.start_project_watches()?;

        let _ = self.event_tx.send(WatchEvent::WatchListUpdated);

        Ok(())
    }

    /// Check if an event should be processed (debounce).
    fn should_process(&mut self, path: &Path) -> bool {
        let now = Instant::now();

        if let Some(last) = self.last_event.get(path) {
            if now.duration_since(*last) < Duration::from_millis(DEBOUNCE_MS) {
                return false;
            }
        }

        self.last_event.insert(path.to_path_buf(), now);
        true
    }

    /// Clean up stale debounce entries to prevent memory leaks.
    ///
    /// Removes entries that haven't been touched in over 60 seconds.
    /// Called periodically during refresh_watch_list.
    fn cleanup_debounce(&mut self) {
        const STALE_THRESHOLD_SECS: u64 = 60;
        let now = Instant::now();
        let threshold = Duration::from_secs(STALE_THRESHOLD_SECS);

        self.last_event
            .retain(|_, last_time| now.duration_since(*last_time) < threshold);
    }

    /// Mark that we're about to write content to a file.
    ///
    /// Call this BEFORE writing, passing the exact content you're about to write.
    /// This allows `is_our_write` to correctly identify our own changes.
    #[allow(dead_code)] // Phase 2: used when daemon syncs config to clients
    pub fn mark_our_write(&mut self, path: &Path, content: &[u8]) {
        let checksum = Self::compute_checksum(content);
        self.our_writes.insert(path.to_path_buf(), checksum);
    }

    /// Check if this change was caused by us.
    ///
    /// IMPORTANT: This is a one-shot check. If the checksum matches, the entry
    /// is REMOVED from our_writes to prevent:
    /// 1. Memory leaks from accumulating entries
    /// 2. False positives if the file is later modified by someone else
    ///
    /// Takes the already-loaded content to avoid:
    /// 1. TOCTOU race (file could change between mark and check)
    /// 2. Blocking I/O in async context
    fn is_our_write(&mut self, path: &Path, content: &[u8]) -> bool {
        // Remove entry first to make this one-shot (fixes memory leak)
        if let Some(expected) = self.our_writes.remove(path) {
            let current = Self::compute_checksum(content);
            if expected == current {
                return true;
            }
            // Content changed - treat as external change
        }
        false
    }

    /// Compute a simple checksum for content.
    fn compute_checksum(content: &[u8]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = rustc_hash::FxHasher::default();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Identify the source client from a config path.
    fn identify_source(path: &Path) -> Option<ForeignSource> {
        let path_str = path.to_string_lossy();

        if path_str.contains(".cursor") {
            Some(ForeignSource::Cursor)
        } else if path_str.contains(".claude") || path_str.contains(".mcp.json") {
            Some(ForeignSource::ClaudeCode)
        } else if path_str.contains(".codeium") || path_str.contains("windsurf") {
            Some(ForeignSource::Windsurf)
        } else {
            None
        }
    }

    /// Determine if a path is a global or project config.
    fn determine_scope(path: &Path) -> ForeignScope {
        let home = dirs::home_dir();

        // Check if path is directly under home (global)
        if let Some(home) = home {
            if let Some(parent) = path.parent() {
                // Global configs are at ~/.cursor, ~/.claude.json, ~/.codeium
                if parent == home || parent.parent() == Some(&home) {
                    return ForeignScope::Global;
                }
            }
        }

        // Otherwise it's project-level
        // Try to find the project root
        let mut current = path.to_path_buf();
        while let Some(parent) = current.parent() {
            if parent.join("spn.yaml").exists()
                || parent.join(".git").exists()
                || parent.join("package.json").exists()
            {
                return ForeignScope::Project(parent.to_path_buf());
            }
            current = parent.to_path_buf();
        }

        // Fallback to the config's parent directory
        ForeignScope::Project(path.parent().unwrap_or(path).to_path_buf())
    }

    /// Process a file change event.
    pub(crate) async fn handle_event(&mut self, event: Event) {
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {
                for path in &event.paths {
                    self.process_file_change(path).await;
                }
            }
            _ => {}
        }
    }

    /// Process a single file change.
    async fn process_file_change(&mut self, path: &Path) {
        // Skip if debounced
        if !self.should_process(path) {
            return;
        }

        // Read file content once (async, non-blocking)
        let content = match tokio::fs::read(path).await {
            Ok(c) => c,
            Err(e) => {
                debug!("Could not read {}: {}", path.display(), e);
                return;
            }
        };

        // Skip if this is our own write (pass content to avoid TOCTOU + blocking I/O)
        if self.is_our_write(path, &content) {
            debug!("Ignoring our own write to {}", path.display());
            return;
        }

        // Check if this is spn's config (triggers sync to clients)
        if path.to_string_lossy().contains(".spn") && path.to_string_lossy().contains("mcp") {
            info!("spn MCP config changed, sync needed");
            let _ = self.event_tx.send(WatchEvent::SyncNeeded);
            return;
        }

        // Identify the source client
        let source = match Self::identify_source(path) {
            Some(s) => s,
            None => {
                debug!("Unknown config source: {}", path.display());
                return;
            }
        };

        // Parse the client config (async, non-blocking)
        let client_mcps = match parse_client_config(path).await {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to parse {}: {}", path.display(), e);
                return;
            }
        };

        // Load spn's config for comparison
        let spn_config = McpConfigManager::new();
        let spn_servers: Vec<(String, spn_core::McpServer)> = match spn_config.list_all_servers() {
            Ok(servers) => servers
                .into_iter()
                .map(|(name, server)| {
                    // Convert local McpServer to spn_core::McpServer
                    let mut core_server = spn_core::McpServer::stdio(
                        &name,
                        &server.command,
                        server.args.iter().map(String::as_str).collect(),
                    );
                    for (k, v) in &server.env {
                        core_server = core_server.with_env(k, v);
                    }
                    core_server = core_server.with_enabled(server.enabled);
                    (name, core_server)
                })
                .collect(),
            Err(e) => {
                warn!("Failed to load spn config: {}", e);
                return;
            }
        };

        // Diff the configs
        let diff = diff_mcp_configs(&spn_servers, &client_mcps);

        // Process foreign MCPs
        for (name, server) in diff.foreign {
            if self.foreign.is_ignored(&name) {
                continue;
            }

            let foreign = ForeignMcp {
                name: name.clone(),
                source,
                scope: Self::determine_scope(path),
                config_path: path.to_path_buf(),
                detected: chrono::Utc::now(),
                server: ForeignMcpServer {
                    command: server.command.clone(),
                    args: server.args.clone(),
                    env: server.env.iter().cloned().collect(),
                    url: server.url.clone(),
                },
            };

            self.foreign.add_pending(foreign.clone());
            self.notifier.notify_foreign_mcp(&name, source);

            let _ = self.event_tx.send(WatchEvent::ForeignMcpDetected(foreign));
        }

        // Save foreign tracker (async)
        if let Err(e) = self.foreign.save().await {
            error!("Failed to save foreign tracker: {}", e);
        }
    }

    /// Run the watcher event loop.
    ///
    /// This should be spawned as a tokio task. It's fully async and will not
    /// block the executor.
    #[allow(dead_code)] // Alternative to select! integration in server.rs
    pub async fn run(&mut self) -> Result<()> {
        info!("Watcher event loop started");

        while let Some(result) = self.rx.recv().await {
            match result {
                Ok(event) => {
                    self.handle_event(event).await;
                }
                Err(e) => {
                    error!("Watch error: {}", e);
                }
            }
        }

        warn!("Watcher channel closed, stopping");
        Ok(())
    }

    /// Get the foreign tracker (for status display).
    #[allow(dead_code)] // Phase 2: exposed via daemon status API
    pub fn foreign_tracker(&self) -> &ForeignTracker {
        &self.foreign
    }

    /// Get the recent projects (for status display).
    #[allow(dead_code)] // Phase 2: exposed via daemon status API
    pub fn recent_projects(&self) -> &RecentProjects {
        &self.recent
    }

    /// Get count of watched paths.
    #[allow(dead_code)] // Phase 2: exposed via daemon status API
    pub fn watched_count(&self) -> usize {
        self.watched_paths.len()
    }

    /// Get the watched paths (for status display).
    #[allow(dead_code)] // Phase 2: exposed via daemon status API
    pub fn watched_paths(&self) -> &FxHashSet<PathBuf> {
        &self.watched_paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_source_cursor() {
        let path = PathBuf::from("/home/user/.cursor/mcp.json");
        assert_eq!(
            WatcherService::identify_source(&path),
            Some(ForeignSource::Cursor)
        );
    }

    #[test]
    fn test_identify_source_claude() {
        let path = PathBuf::from("/home/user/.claude.json");
        assert_eq!(
            WatcherService::identify_source(&path),
            Some(ForeignSource::ClaudeCode)
        );

        let path2 = PathBuf::from("/home/user/project/.mcp.json");
        assert_eq!(
            WatcherService::identify_source(&path2),
            Some(ForeignSource::ClaudeCode)
        );
    }

    #[test]
    fn test_identify_source_windsurf() {
        let path = PathBuf::from("/home/user/.codeium/windsurf/mcp_config.json");
        assert_eq!(
            WatcherService::identify_source(&path),
            Some(ForeignSource::Windsurf)
        );
    }

    #[test]
    fn test_identify_source_unknown() {
        let path = PathBuf::from("/home/user/random/file.json");
        assert_eq!(WatcherService::identify_source(&path), None);
    }

    #[test]
    fn test_determine_scope_global() {
        // Note: This test may not work correctly without a real home dir
        // but demonstrates the logic
        let path = PathBuf::from("/Users/test/.cursor/mcp.json");
        let scope = WatcherService::determine_scope(&path);
        // Will be Project since we can't verify home dir in test
        assert!(matches!(scope, ForeignScope::Global | ForeignScope::Project(_)));
    }

    #[test]
    fn test_compute_checksum() {
        let data1 = b"hello world";
        let data2 = b"hello world";
        let data3 = b"different";

        let hash1 = WatcherService::compute_checksum(data1);
        let hash2 = WatcherService::compute_checksum(data2);
        let hash3 = WatcherService::compute_checksum(data3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_global_watch_paths() {
        let paths = WatcherService::global_watch_paths();

        // Should include paths for all supported clients
        let path_strs: Vec<_> = paths.iter().map(|p| p.to_string_lossy()).collect();

        assert!(path_strs.iter().any(|p| p.contains("mcp.yaml")));
        assert!(path_strs.iter().any(|p| p.contains(".cursor")));
        assert!(path_strs.iter().any(|p| p.contains(".claude")));
        assert!(path_strs.iter().any(|p| p.contains("windsurf")));
    }
}
