# MCP Auto-Sync Implementation Plan

**Date:** 2026-03-08
**Design:** [2026-03-08-mcp-auto-sync-design.md](./2026-03-08-mcp-auto-sync-design.md)
**Target:** v0.16.0
**Estimated Tasks:** 25

---

## Phase 1: Foundation (7 tasks)

### 1.1 Add Dependencies

**File:** `crates/spn/Cargo.toml`

```toml
[dependencies]
notify = "6"           # File system watching
notify-rust = "4"      # macOS/Linux notifications
```

**Verification:** `cargo check -p spn-cli`

---

### 1.2 Create Recent Projects Manager

**File:** `crates/spn/src/daemon/recent.rs`

```rust
//! Recent projects tracking for Lite C watch scope.

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const MAX_RECENT_PROJECTS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: PathBuf,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecentProjects {
    pub max_projects: usize,
    pub projects: Vec<RecentProject>,
}

impl RecentProjects {
    /// Load from ~/.spn/recent.yaml
    pub fn load() -> Result<Self, Error>;

    /// Save to ~/.spn/recent.yaml
    pub fn save(&self) -> Result<(), Error>;

    /// Add or update a project (moves to top)
    pub fn touch(&mut self, path: PathBuf);

    /// Get all project paths for watching
    pub fn watch_paths(&self) -> Vec<PathBuf>;

    /// Remove stale projects (directories that no longer exist)
    pub fn cleanup(&mut self);
}
```

**Tests:**
- `test_touch_new_project`
- `test_touch_existing_project_moves_to_top`
- `test_max_projects_enforced`
- `test_cleanup_removes_nonexistent`

---

### 1.3 Create Foreign MCP Tracker

**File:** `crates/spn/src/daemon/foreign.rs`

```rust
//! Foreign MCP detection and tracking.

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::mcp::McpServer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignMcp {
    pub name: String,
    pub source: String,        // "cursor", "claude-code", "windsurf"
    pub scope: ForeignScope,   // Global or Project
    pub path: PathBuf,         // Where it was found
    pub detected: DateTime<Utc>,
    pub config: McpServer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForeignScope {
    Global,
    Project(PathBuf),
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ForeignTracker {
    pub ignored: Vec<String>,       // Names to ignore
    pub pending: Vec<ForeignMcp>,   // Awaiting user decision
}

impl ForeignTracker {
    /// Load from ~/.spn/foreign.yaml
    pub fn load() -> Result<Self, Error>;

    /// Save to ~/.spn/foreign.yaml
    pub fn save(&self) -> Result<(), Error>;

    /// Check if MCP name is ignored
    pub fn is_ignored(&self, name: &str) -> bool;

    /// Add to pending list
    pub fn add_pending(&mut self, mcp: ForeignMcp);

    /// Mark as ignored (user chose "Ignore")
    pub fn ignore(&mut self, name: &str);

    /// Remove from pending (user chose "Adopt" - handled elsewhere)
    pub fn remove_pending(&mut self, name: &str);

    /// Get all pending MCPs
    pub fn pending(&self) -> &[ForeignMcp];
}
```

**Tests:**
- `test_add_pending`
- `test_ignore_removes_from_pending`
- `test_is_ignored`
- `test_persistence`

---

### 1.4 Create MCP Differ

**File:** `crates/spn/src/daemon/differ.rs`

```rust
//! Compare MCP configs to detect foreign MCPs.

use crate::mcp::{McpConfig, McpServer};
use rustc_hash::FxHashSet;

pub struct McpDiff {
    /// MCPs in client but not in spn (foreign)
    pub foreign: Vec<(String, McpServer)>,
    /// MCPs in spn but not in client (need sync)
    pub missing: Vec<String>,
    /// MCPs in both (synced)
    pub synced: Vec<String>,
}

/// Compare spn's MCPs with a client's MCPs
pub fn diff_mcp_configs(
    spn_config: &McpConfig,
    client_mcps: &[(String, McpServer)],
) -> McpDiff;

/// Parse client config file and extract MCPs
pub fn parse_client_config(path: &Path) -> Result<Vec<(String, McpServer)>, Error>;
```

**Tests:**
- `test_diff_finds_foreign`
- `test_diff_finds_missing`
- `test_diff_identifies_synced`
- `test_parse_cursor_config`
- `test_parse_claude_config`

---

### 1.5 Create Notification Service

**File:** `crates/spn/src/daemon/notify.rs`

```rust
//! Native notifications for foreign MCP detection.

use notify_rust::Notification;

pub struct NotificationService {
    enabled: bool,
}

impl NotificationService {
    pub fn new() -> Self;

    /// Send native notification
    pub fn notify_foreign_mcp(&self, name: &str, source: &str) -> Result<(), Error> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary("spn")
            .body(&format!("Foreign MCP detected: {}", name))
            .subtitle(&format!("Source: {}", source))
            .show()?;

        Ok(())
    }

    /// Log to daemon stderr
    pub fn log_foreign_mcp(&self, name: &str, source: &str) {
        eprintln!("[WATCH] Foreign MCP detected: {} (source: {})", name, source);
    }
}
```

**Tests:**
- `test_notification_disabled_noop`
- Manual test on macOS

---

### 1.6 Update Module Structure

**File:** `crates/spn/src/daemon/mod.rs`

```rust
// Add new modules
pub mod recent;
pub mod foreign;
pub mod differ;
pub mod notify;
pub mod watcher;  // Phase 2
```

---

### 1.7 Add Recent Touch to CLI Commands

**File:** `crates/spn/src/main.rs` (or command handlers)

After every command execution, if in a project directory:

```rust
// At end of command execution
if let Some(project_root) = find_project_root() {
    if let Ok(mut recent) = RecentProjects::load() {
        recent.touch(project_root);
        let _ = recent.save();
        // Notify daemon to update watch list (via IPC)
        if let Ok(client) = SpnClient::connect() {
            let _ = client.update_watch_list();
        }
    }
}
```

---

## Phase 2: File Watcher Service (8 tasks)

### 2.1 Create WatcherService Struct

**File:** `crates/spn/src/daemon/watcher.rs`

```rust
//! File system watcher for MCP config changes.

use notify::{Watcher, RecursiveMode, RecommendedWatcher, Event};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::broadcast;

pub struct WatcherService {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<Event>>,
    watched_paths: Vec<PathBuf>,
    recent: RecentProjects,
    foreign: ForeignTracker,
    notifier: NotificationService,
}

impl WatcherService {
    pub fn new() -> Result<Self, Error>;

    /// Start watching all paths
    pub fn start(&mut self) -> Result<(), Error>;

    /// Add a path to watch
    pub fn watch(&mut self, path: PathBuf) -> Result<(), Error>;

    /// Remove a path from watch
    pub fn unwatch(&mut self, path: &Path) -> Result<(), Error>;

    /// Update watch list from recent projects
    pub fn refresh_watch_list(&mut self) -> Result<(), Error>;

    /// Process a file change event
    async fn handle_event(&mut self, event: Event);
}
```

---

### 2.2 Implement Global Paths Watching

```rust
impl WatcherService {
    fn global_watch_paths() -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            home.join(".spn").join("mcp.yaml"),
            home.join(".cursor").join("mcp.json"),
            home.join(".claude.json"),
            home.join(".codeium").join("windsurf").join("mcp_config.json"),
        ]
    }

    fn start_global_watches(&mut self) -> Result<(), Error> {
        for path in Self::global_watch_paths() {
            if path.exists() {
                self.watcher.watch(&path, RecursiveMode::NonRecursive)?;
                self.watched_paths.push(path);
            }
        }
        Ok(())
    }
}
```

---

### 2.3 Implement Project Paths Watching

```rust
impl WatcherService {
    fn project_watch_paths(project: &Path) -> Vec<PathBuf> {
        vec![
            project.join(".cursor").join("mcp.json"),
            project.join(".mcp.json"),
            project.join(".claude").join("settings.json"),
        ]
    }

    fn start_project_watches(&mut self) -> Result<(), Error> {
        for project in self.recent.watch_paths() {
            for path in Self::project_watch_paths(&project) {
                if path.exists() {
                    self.watcher.watch(&path, RecursiveMode::NonRecursive)?;
                    self.watched_paths.push(path);
                }
            }
        }
        Ok(())
    }
}
```

---

### 2.4 Implement Debounce Logic

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 500;

impl WatcherService {
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
}
```

---

### 2.5 Implement Origin Tracking

Prevent sync loops (spn writes → triggers event → spn processes → writes again):

```rust
use std::sync::atomic::{AtomicU64, Ordering};

impl WatcherService {
    /// Mark that we're about to write to a file
    fn mark_our_write(&mut self, path: &Path) {
        let checksum = self.compute_checksum(path);
        self.our_writes.insert(path.to_path_buf(), checksum);
    }

    /// Check if this change was caused by us
    fn is_our_write(&self, path: &Path) -> bool {
        if let Some(expected_checksum) = self.our_writes.get(path) {
            let current = self.compute_checksum(path);
            return *expected_checksum == current;
        }
        false
    }
}
```

---

### 2.6 Implement Event Handler

```rust
impl WatcherService {
    async fn handle_event(&mut self, event: Event) {
        use notify::EventKind;

        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {
                for path in &event.paths {
                    if !self.should_process(path) {
                        continue;
                    }

                    if self.is_our_write(path) {
                        continue;
                    }

                    self.process_config_change(path).await;
                }
            }
            _ => {}
        }
    }

    async fn process_config_change(&mut self, path: &Path) {
        // Determine source
        let source = self.identify_source(path);

        // Load spn's config
        let spn_config = match McpConfig::load() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[WATCH] Failed to load spn config: {}", e);
                return;
            }
        };

        // Parse client config
        let client_mcps = match parse_client_config(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[WATCH] Failed to parse {}: {}", path.display(), e);
                return;
            }
        };

        // Diff
        let diff = diff_mcp_configs(&spn_config, &client_mcps);

        // Handle foreign MCPs
        for (name, config) in diff.foreign {
            if self.foreign.is_ignored(&name) {
                continue;
            }

            let foreign = ForeignMcp {
                name: name.clone(),
                source: source.clone(),
                scope: self.determine_scope(path),
                path: path.to_path_buf(),
                detected: Utc::now(),
                config,
            };

            self.foreign.add_pending(foreign);
            self.notifier.notify_foreign_mcp(&name, &source);
            self.notifier.log_foreign_mcp(&name, &source);
        }

        // Save foreign tracker
        let _ = self.foreign.save();
    }
}
```

---

### 2.7 Integrate with Daemon

**File:** `crates/spn/src/daemon/server.rs`

```rust
pub struct Daemon {
    keychain: KeychainService,
    ipc: IpcService,
    watcher: WatcherService,  // NEW
}

impl Daemon {
    pub async fn run(&mut self) -> Result<(), Error> {
        let mut set = JoinSet::new();

        // Existing services
        set.spawn(self.keychain.run());
        set.spawn(self.ipc.run());

        // New watcher service
        set.spawn(self.watcher.run());

        // Wait for any to complete (usually shutdown signal)
        while let Some(result) = set.join_next().await {
            // Handle result
        }

        Ok(())
    }
}
```

---

### 2.8 Add IPC Commands for Watch Updates

**File:** `crates/spn/src/daemon/protocol.rs`

```rust
pub enum DaemonRequest {
    // Existing
    Ping,
    GetSecret { provider: String },

    // New
    UpdateWatchList,
    GetForeignMcps,
}

pub enum DaemonResponse {
    // Existing
    Pong,
    Secret { value: Option<String> },

    // New
    WatchListUpdated,
    ForeignMcps { pending: Vec<ForeignMcp> },
}
```

---

## Phase 3: Status Integration (5 tasks)

### 3.1 Update McpServerStatus

**File:** `crates/spn/src/status/mcp.rs`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct McpServerStatus {
    pub name: String,
    pub emoji: &'static str,        // NEW
    pub status: ServerStatus,
    pub transport: Transport,
    pub credential: Option<String>,
    pub command: String,
    pub client_sync: ClientSyncStatus,  // NEW
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientSyncStatus {
    pub claude_code: SyncState,
    pub cursor: SyncState,
    pub windsurf: SyncState,
    pub nika: SyncState,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum SyncState {
    Synced,      // ●
    Pending,     // ○
    Disabled,    // ⊘
}
```

---

### 3.2 Add MCP Emoji Function

**File:** `crates/spn/src/status/mcp.rs`

```rust
pub fn mcp_emoji(name: &str) -> &'static str {
    match name {
        "neo4j" | "@neo4j/mcp-neo4j" => "🔷",
        "github" | "github-mcp" => "🐙",
        "slack" | "slack-mcp" => "💬",
        "perplexity" | "perplexity-mcp" => "🔮",
        "firecrawl" | "firecrawl-mcp" => "🔥",
        "supadata" | "supadata-mcp" => "📺",
        "dataforseo" => "📊",
        "ahrefs" => "🔗",
        "context7" => "📚",
        "novanet" => "🌐",
        "sequential-thinking" => "🧠",
        "21st" | "magic" => "🎨",
        "spn-mcp" => "⚡",
        _ => "🔌",
    }
}
```

---

### 3.3 Update MCP Render with Client Column

**File:** `crates/spn/src/status/render.rs`

```rust
fn render_mcp_servers(servers: &[McpServerStatus], foreign: &[ForeignMcp]) {
    println!(
        "┌─ {} ───────────────────────────────────────────────────────────────┐",
        ds::primary("🔌 MCP SERVERS")
    );

    // Header with Clients column
    println!(
        "│  {:<3}{:<12}{:<10}{:<10}{:<14}{:<12}│",
        "", "Server", "Status", "Transport", "Credential", "Clients"
    );

    for server in servers {
        let clients = format_client_sync(&server.client_sync);
        println!(
            "│  {} {:<12}{:<10}{:<10}{:<14}{}│",
            server.emoji,
            server.name,
            format_status(&server.status),
            server.transport,
            server.credential.as_deref().unwrap_or("--"),
            clients
        );
    }

    // Foreign MCPs section
    if !foreign.is_empty() {
        println!("│{}│", " ".repeat(WIDTH));
        println!("│  {} FOREIGN MCPs ({} pending){}│",
            "🆕", foreign.len(), " ".repeat(WIDTH - 30));

        for mcp in foreign {
            println!("│  🔌 {} (from {}){}│",
                mcp.name, mcp.source, " ".repeat(WIDTH - mcp.name.len() - 20));
        }
    }
}

fn format_client_sync(sync: &ClientSyncStatus) -> String {
    format!("{}{}{}{} ",
        sync_icon(sync.claude_code),
        sync_icon(sync.cursor),
        sync_icon(sync.nika),
        sync_icon(sync.windsurf),
    )
}

fn sync_icon(state: SyncState) -> &'static str {
    match state {
        SyncState::Synced => "●",
        SyncState::Pending => "○",
        SyncState::Disabled => "⊘",
    }
}
```

---

### 3.4 Add Foreign MCP Adoption Prompt

**File:** `crates/spn/src/commands/status.rs`

```rust
use dialoguer::{Select, theme::ColorfulTheme};

async fn handle_foreign_mcps(foreign: &[ForeignMcp]) -> Result<(), Error> {
    if foreign.is_empty() {
        return Ok(());
    }

    println!();
    println!("{}", ds::primary("🆕 Foreign MCPs detected:"));

    for mcp in foreign {
        println!();
        println!("  {} {}", mcp_emoji(&mcp.name), ds::highlight(&mcp.name));
        println!("  Source: {} ({})", mcp.source, mcp.path.display());
        println!("  Command: {}", mcp.config.command);

        let options = vec!["[A]dopt", "[I]gnore", "[S]kip"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => adopt_mcp(mcp).await?,
            1 => ignore_mcp(mcp).await?,
            2 => {} // Skip
            _ => {}
        }
    }

    Ok(())
}

async fn adopt_mcp(mcp: &ForeignMcp) -> Result<(), Error> {
    // Add to ~/.spn/mcp.yaml
    let mut config = McpConfig::load()?;
    config.servers.insert(mcp.name.clone(), mcp.config.clone());
    config.save()?;

    // Remove from foreign pending
    let mut tracker = ForeignTracker::load()?;
    tracker.remove_pending(&mcp.name);
    tracker.save()?;

    // Trigger sync to all clients
    sync_mcp_to_editors(&[IdeTarget::all()], None)?;

    println!("  {} Adopted and synced to all clients", ds::success("✅"));
    Ok(())
}
```

---

### 3.5 Add `spn status mcp` Subcommand

**File:** `crates/spn/src/commands/status.rs`

```rust
#[derive(Parser)]
pub struct StatusCommand {
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Option<StatusSubcommand>,
}

#[derive(Subcommand)]
pub enum StatusSubcommand {
    /// Detailed MCP servers view
    Mcp,
    /// Detailed clients view
    Clients,
}

pub async fn run(cmd: StatusCommand) -> Result<()> {
    match cmd.command {
        None => run_full_status(cmd.json).await,
        Some(StatusSubcommand::Mcp) => run_mcp_detail().await,
        Some(StatusSubcommand::Clients) => run_clients_detail().await,
    }
}
```

---

## Phase 4: Testing & Polish (5 tasks)

### 4.1 Integration Tests

**File:** `crates/spn/tests/watcher_integration.rs`

```rust
#[tokio::test]
async fn test_foreign_mcp_detection() {
    // Setup temp dirs
    // Write to mock cursor config
    // Verify foreign tracker updated
    // Verify notification logged
}

#[tokio::test]
async fn test_recent_projects_watch() {
    // Touch a project
    // Verify it's in recent list
    // Verify watcher starts watching it
}

#[tokio::test]
async fn test_debounce_prevents_spam() {
    // Rapid file changes
    // Verify only one event processed
}
```

---

### 4.2 Manual Test Checklist

- [ ] `spn daemon start` starts watcher
- [ ] Edit `~/.cursor/mcp.json` → notification appears
- [ ] `spn status` shows foreign MCPs
- [ ] Adopt flow works
- [ ] Ignore flow works
- [ ] Recent projects tracked after `spn sync`
- [ ] Project configs watched after becoming recent

---

### 4.3 Documentation

- [ ] Update README with auto-sync feature
- [ ] Update `spn help` with new subcommands
- [ ] Add troubleshooting guide

---

### 4.4 Error Handling

- [ ] Graceful handling of missing config files
- [ ] Watcher restart on error
- [ ] Notification fallback if native fails

---

### 4.5 Performance Testing

- [ ] Test with 5 recent projects
- [ ] Test with many MCP servers
- [ ] Memory usage profiling

---

## File Summary

| File | Status | Phase |
|------|--------|-------|
| `Cargo.toml` | New deps | 1 |
| `daemon/mod.rs` | Modify | 1 |
| `daemon/recent.rs` | New | 1 |
| `daemon/foreign.rs` | New | 1 |
| `daemon/differ.rs` | New | 1 |
| `daemon/notify.rs` | New | 1 |
| `daemon/watcher.rs` | New | 2 |
| `daemon/server.rs` | Modify | 2 |
| `daemon/protocol.rs` | Modify | 2 |
| `status/mcp.rs` | Modify | 3 |
| `status/render.rs` | Modify | 3 |
| `commands/status.rs` | Modify | 3 |
| `main.rs` | Modify | 1 |

---

## Verification Commands

```bash
# After each phase
cargo check -p spn-cli
cargo test -p spn-cli
cargo clippy -p spn-cli -- -D warnings

# Full verification
cargo test --workspace
spn daemon start
# Edit ~/.cursor/mcp.json manually
# Check for notification
spn status
```
