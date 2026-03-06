# spn-cli Roadmap: Daemon, Registry, UX

**Status**: Planning | **Author**: Claude + Thibaut | **Date**: 2024-03-06

---

## Overview

Three phases of work to complete spn-cli v0.13.0:

| Phase | Description | Complexity | Value | Commits |
|-------|-------------|------------|-------|---------|
| 5 | Daemon Auto-Start | Haute | Moyenne | 4 |
| 6 | Dependency Resolution | Haute | Haute | 5 |
| 8 | CLI UX Polish | Moyenne | Basse | 3 |

**Total**: 12 commits across 3 phases

---

## Phase 5: Daemon Auto-Start

**Goal**: Allow `spn daemon` to run automatically at login via launchd (macOS) / systemd (Linux).

### Commit 5.1: Add launchd plist template

```
feat(daemon): add launchd plist for macOS auto-start

Files:
- assets/launchd/com.supernovae.spn-daemon.plist (NEW)
- crates/spn/src/daemon/service.rs (NEW)
```

**Implementation**:

```xml
<!-- assets/launchd/com.supernovae.spn-daemon.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "...">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.supernovae.spn-daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>${HOME}/.cargo/bin/spn</string>
        <string>daemon</string>
        <string>start</string>
        <string>--foreground</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>${HOME}/.spn/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>${HOME}/.spn/daemon.err</string>
</dict>
</plist>
```

### Commit 5.2: Add systemd unit template

```
feat(daemon): add systemd unit for Linux auto-start

Files:
- assets/systemd/spn-daemon.service (NEW)
```

**Implementation**:

```ini
# assets/systemd/spn-daemon.service
[Unit]
Description=SuperNovae Package Manager Daemon
After=network.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/spn daemon start --foreground
Restart=on-failure
RestartSec=5
StandardOutput=append:%h/.spn/daemon.log
StandardError=append:%h/.spn/daemon.err

[Install]
WantedBy=default.target
```

### Commit 5.3: Implement `spn daemon install/uninstall`

```
feat(daemon): add install/uninstall commands for service management

Files:
- crates/spn/src/daemon/service.rs (NEW)
- crates/spn/src/commands/daemon.rs (MODIFY)
```

**New Commands**:

```bash
spn daemon install    # Install as system service
spn daemon uninstall  # Remove system service
spn daemon status     # Show service status
```

**Implementation**:

```rust
// crates/spn/src/daemon/service.rs

pub enum ServiceManager {
    Launchd,   // macOS
    Systemd,   // Linux
    None,      // Windows/other
}

impl ServiceManager {
    pub fn detect() -> Self {
        #[cfg(target_os = "macos")]
        return Self::Launchd;

        #[cfg(target_os = "linux")]
        if Path::new("/run/systemd/system").exists() {
            return Self::Systemd;
        }

        Self::None
    }

    pub fn install(&self) -> Result<()> {
        match self {
            Self::Launchd => self.install_launchd(),
            Self::Systemd => self.install_systemd(),
            Self::None => Err(Error::UnsupportedPlatform),
        }
    }

    fn install_launchd(&self) -> Result<()> {
        let plist_content = include_str!("../../../assets/launchd/com.supernovae.spn-daemon.plist");
        let home = dirs::home_dir().unwrap();
        let plist_path = home.join("Library/LaunchAgents/com.supernovae.spn-daemon.plist");

        // Replace ${HOME} placeholder
        let content = plist_content.replace("${HOME}", &home.display().to_string());

        fs::create_dir_all(plist_path.parent().unwrap())?;
        fs::write(&plist_path, content)?;

        // Load the service
        Command::new("launchctl")
            .args(["load", "-w", &plist_path.display().to_string()])
            .status()?;

        Ok(())
    }
}
```

### Commit 5.4: Add tests for service management

```
test(daemon): add service management tests

Files:
- crates/spn/tests/daemon_service.rs (NEW)
```

---

## Phase 6: Dependency Resolution

**Goal**: When installing a package, automatically install its dependencies.

### Commit 6.1: Add dependency graph types

```
feat(registry): add dependency graph types

Files:
- crates/spn/src/index/resolver.rs (NEW)
- crates/spn/src/index/mod.rs (MODIFY)
```

**Implementation**:

```rust
// crates/spn/src/index/resolver.rs

use petgraph::graph::DiGraph;
use petgraph::algo::toposort;

/// A node in the dependency graph.
#[derive(Debug, Clone)]
pub struct DepNode {
    pub name: String,
    pub version: String,
    pub entry: IndexEntry,
}

/// Dependency resolver using topological sort.
pub struct DependencyResolver {
    client: IndexClient,
    graph: DiGraph<DepNode, ()>,
    resolved: HashMap<String, NodeIndex>,
}

impl DependencyResolver {
    pub fn new(client: IndexClient) -> Self {
        Self {
            client,
            graph: DiGraph::new(),
            resolved: HashMap::new(),
        }
    }

    /// Resolve all dependencies for a package.
    /// Returns packages in installation order (dependencies first).
    pub async fn resolve(&mut self, name: &str, version: Option<&str>) -> Result<Vec<DepNode>> {
        self.resolve_recursive(name, version).await?;

        // Topological sort ensures deps come before dependents
        let sorted = toposort(&self.graph, None)
            .map_err(|_| ResolverError::CyclicDependency)?;

        Ok(sorted.into_iter()
            .map(|idx| self.graph[idx].clone())
            .collect())
    }

    async fn resolve_recursive(&mut self, name: &str, version: Option<&str>) -> Result<NodeIndex> {
        // Check if already resolved
        if let Some(&idx) = self.resolved.get(name) {
            return Ok(idx);
        }

        // Fetch package info
        let entry = match version {
            Some(v) => self.client.fetch_version(name, v).await?,
            None => self.client.fetch_latest(name).await?,
        };

        // Add to graph
        let node = DepNode {
            name: name.to_string(),
            version: entry.version.clone(),
            entry: entry.clone(),
        };
        let idx = self.graph.add_node(node);
        self.resolved.insert(name.to_string(), idx);

        // Resolve dependencies
        for dep in &entry.deps {
            if dep.optional {
                continue; // Skip optional deps
            }

            let dep_idx = self.resolve_recursive(&dep.name, Some(&dep.req)).await?;
            self.graph.add_edge(dep_idx, idx, ());
        }

        Ok(idx)
    }
}
```

### Commit 6.2: Add version constraint matching

```
feat(registry): add semver constraint matching

Files:
- crates/spn/src/index/version.rs (NEW)
```

**Implementation**:

```rust
// crates/spn/src/index/version.rs

use semver::{Version, VersionReq};

/// Match a version requirement against available versions.
pub fn find_best_match(
    requirement: &str,
    available: &[IndexEntry],
) -> Option<&IndexEntry> {
    let req = VersionReq::parse(requirement).ok()?;

    available
        .iter()
        .filter(|e| !e.yanked)
        .filter(|e| {
            Version::parse(&e.version)
                .map(|v| req.matches(&v))
                .unwrap_or(false)
        })
        .max_by(|a, b| {
            let va = Version::parse(&a.version).ok();
            let vb = Version::parse(&b.version).ok();
            va.cmp(&vb)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caret_requirement() {
        let entries = vec![
            IndexEntry::new("pkg", "1.0.0", "sha256:a"),
            IndexEntry::new("pkg", "1.1.0", "sha256:b"),
            IndexEntry::new("pkg", "2.0.0", "sha256:c"),
        ];

        let best = find_best_match("^1.0", &entries).unwrap();
        assert_eq!(best.version, "1.1.0"); // Highest 1.x
    }

    #[test]
    fn test_tilde_requirement() {
        let entries = vec![
            IndexEntry::new("pkg", "1.0.0", "sha256:a"),
            IndexEntry::new("pkg", "1.0.5", "sha256:b"),
            IndexEntry::new("pkg", "1.1.0", "sha256:c"),
        ];

        let best = find_best_match("~1.0.0", &entries).unwrap();
        assert_eq!(best.version, "1.0.5"); // Highest 1.0.x
    }
}
```

### Commit 6.3: Integrate resolver into `spn add`

```
feat(add): integrate dependency resolution

Files:
- crates/spn/src/commands/add.rs (MODIFY)
```

**Changes to `add.rs`**:

```rust
pub async fn run(package: &str, options: &AddOptions) -> Result<()> {
    let client = IndexClient::new();
    let mut resolver = DependencyResolver::new(client);

    // Resolve all dependencies
    println!("{} Resolving dependencies...", "→".blue());
    let packages = resolver.resolve(package, options.version.as_deref()).await?;

    // Show what will be installed
    println!();
    println!("   {} packages to install:", packages.len());
    for pkg in &packages {
        let is_direct = pkg.name == package;
        let marker = if is_direct { "•".green() } else { "•".blue() };
        println!("   {} {}@{}", marker, pkg.name, pkg.version);
    }

    // Install in order (deps first)
    for pkg in packages {
        install_package(&pkg.name, &pkg.version, &pkg.entry).await?;
    }

    Ok(())
}
```

### Commit 6.4: Add cycle detection

```
feat(resolver): add cycle detection with helpful errors

Files:
- crates/spn/src/index/resolver.rs (MODIFY)
```

**Enhancement**:

```rust
#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("Cyclic dependency detected: {cycle}")]
    CyclicDependency { cycle: String },

    #[error("Version conflict: {package} required as both {v1} and {v2}")]
    VersionConflict { package: String, v1: String, v2: String },
}

fn format_cycle(graph: &DiGraph<DepNode, ()>, cycle: &[NodeIndex]) -> String {
    cycle
        .iter()
        .map(|&idx| format!("{}@{}", graph[idx].name, graph[idx].version))
        .collect::<Vec<_>>()
        .join(" → ")
}
```

### Commit 6.5: Add resolver tests

```
test(resolver): add comprehensive dependency resolution tests

Files:
- crates/spn/tests/resolver.rs (NEW)
```

---

## Phase 8: CLI UX Polish

**Goal**: Improve user experience with progress indicators and better output.

### Commit 8.1: Add progress bars for downloads

```
feat(cli): add progress bars for model/package downloads

Files:
- crates/spn/src/ui/progress.rs (NEW)
- crates/spn/src/ui/mod.rs (NEW)
- crates/spn/src/commands/model.rs (MODIFY)
- Cargo.toml (add indicatif)
```

**Implementation**:

```rust
// crates/spn/src/ui/progress.rs

use indicatif::{ProgressBar, ProgressStyle};

pub fn download_progress(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb
}

pub fn spinner(message: &str) -> ProgressBar {
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
```

### Commit 8.2: Add `--json` output for scripting

```
feat(cli): add --json output flag for machine-readable output

Files:
- crates/spn/src/output.rs (NEW)
- crates/spn/src/commands/list.rs (MODIFY)
- crates/spn/src/commands/search.rs (MODIFY)
- crates/spn/src/commands/info.rs (MODIFY)
```

**Implementation**:

```rust
// crates/spn/src/output.rs

use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
}

impl OutputFormat {
    pub fn from_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Human }
    }
}

pub fn print<T: Serialize>(data: &T, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => {
            // Default: let caller handle human output
            Ok(())
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(data)?);
            Ok(())
        }
    }
}
```

### Commit 8.3: Improve error messages

```
feat(cli): improve error messages with suggestions

Files:
- crates/spn/src/error.rs (MODIFY)
```

**Enhancement**:

```rust
#[derive(Debug, Error)]
pub enum SpnError {
    #[error("Package not found: {0}")]
    #[diagnostic(
        code(spn::package_not_found),
        help("Did you mean one of these?\n{suggestions}")
    )]
    PackageNotFound {
        name: String,
        suggestions: Vec<String>,
    },

    #[error("Daemon not running")]
    #[diagnostic(
        code(spn::daemon_not_running),
        help("Start the daemon with: spn daemon start")
    )]
    DaemonNotRunning,
}
```

---

## Execution Order

```
Phase 6 (Dependency Resolution) - HIGH VALUE
├── 6.1 Dependency graph types
├── 6.2 Version constraint matching
├── 6.3 Integrate into spn add
├── 6.4 Cycle detection
└── 6.5 Tests

Phase 5 (Daemon Auto-Start) - MEDIUM VALUE
├── 5.1 launchd plist
├── 5.2 systemd unit
├── 5.3 install/uninstall commands
└── 5.4 Tests

Phase 8 (CLI UX) - LOW VALUE
├── 8.1 Progress bars
├── 8.2 --json output
└── 8.3 Better errors
```

**Recommended execution**: Phase 6 → Phase 5 → Phase 8

---

## Dependencies to Add

```toml
# Cargo.toml additions

[dependencies]
petgraph = "0.6"      # Dependency graph
indicatif = "0.17"    # Progress bars
miette = "7"          # Better error diagnostics (optional)
```

---

## Timeline Estimate

| Phase | Commits | Estimated |
|-------|---------|-----------|
| 6 | 5 | 2-3h |
| 5 | 4 | 1-2h |
| 8 | 3 | 1h |
| **Total** | **12** | **4-6h** |

---

## Acceptance Criteria

### Phase 5: Daemon Auto-Start
- [ ] `spn daemon install` works on macOS (launchd)
- [ ] `spn daemon install` works on Linux (systemd)
- [ ] `spn daemon uninstall` cleanly removes service
- [ ] Daemon starts automatically after reboot
- [ ] Logs are written to `~/.spn/daemon.log`

### Phase 6: Dependency Resolution
- [ ] `spn add @pkg/with-deps` installs all dependencies
- [ ] Dependencies installed before dependents (topo sort)
- [ ] Cyclic dependencies detected with clear error
- [ ] Version conflicts reported with both versions
- [ ] `^`, `~`, `>=` semver requirements supported

### Phase 8: CLI UX
- [ ] `spn model pull` shows progress bar
- [ ] `spn list --json` outputs valid JSON
- [ ] `spn add nonexistent` suggests similar packages
