# spn-cli Pro-Grade Roadmap v0.15+

**Date:** 2026-03-07
**Status:** 🔵 Planning
**Version:** spn-cli v0.14.2 → v0.15.0+

---

## Executive Summary

Cette roadmap consolide les analyses de 5 agents de recherche pour transformer spn-cli en outil "production-grade" comparable à `gh`, `cargo`, et `ripgrep`.

**Impact estimé:**
- Performance: **2-3x faster** (HTTP pooling, ETag caching, parallel downloads)
- Security: **SLSA Level 2+** (signatures, SBOM, audit CI)
- DX: **Pro-grade** (shell completions, man pages, offline mode)

---

## Quick Wins (< 2h each)

| Priority | Task | Effort | Impact | File |
|----------|------|--------|--------|------|
| 🔴 | Add `cargo audit` to CI | 15 min | Critical | `.github/workflows/test.yml` |
| 🔴 | Wire verbose flag to log level | 1-2h | High | `main.rs:1091-1093` |
| 🔴 | Global `--non-interactive` flag | 1-2h | High | `main.rs` (Cli struct) |
| 🟡 | Add panic hook for crash context | 1-2h | Medium | NEW: `crash.rs` |
| 🟡 | Reject symlinks in tarballs | 1h | Medium | `storage/local.rs:290+` |
| 🟡 | Add short command aliases | 30 min | Low | `main.rs` Commands enum |

---

## Part 1: Security Hardening (v0.15.0)

### 1.1 Dependency Auditing (CRITICAL)

**Gap:** Zero CVE scanning in CI/CD

```yaml
# .github/workflows/test.yml - ADD THIS
- name: Security audit
  run: |
    cargo install cargo-audit --locked
    cargo audit --deny warnings
```

**Effort:** 15 min | **Impact:** Critical

### 1.2 SBOM Generation

**Gap:** No Software Bill of Materials

```yaml
# .github/workflows/release.yml - ADD THIS
- name: Generate SBOM
  run: |
    curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh
    syft ./target/release/spn -o cyclonedx-json > sbom.cyclonedx.json
```

**Effort:** 30 min | **Impact:** High (compliance)

### 1.3 Symlink Rejection in Tarballs

**Location:** `crates/spn/src/storage/local.rs:290+`

```rust
// ADD: Check if entry is symlink
if entry.header().entry_type().is_symlink() {
    return Err(StorageError::InvalidPath(format!(
        "Symlinks not allowed in packages: {}",
        path.display()
    )));
}
```

**Effort:** 1h | **Impact:** Medium (defense-in-depth)

### 1.4 Ed25519 Signature Verification (v0.16.0)

**Gap:** SHA256 checksums but no asymmetric signatures

**Scope:**
- Create: `crates/spn/src/index/signer.rs`
- Add: `ed25519-dalek` or `ring` crate
- Modify: `downloader.rs` to verify signatures

**Effort:** 4-6h | **Impact:** High (supply chain)

---

## Part 2: Performance Optimization (v0.15.0)

### 2.1 Shared HTTP Client Pool (HIGH PRIORITY)

**Gap:** Each module creates separate `ClientWithMiddleware`

```rust
// NEW: crates/spn-client/src/http.rs
use once_cell::sync::Lazy;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

pub static HTTP_CLIENT: Lazy<ClientWithMiddleware> = Lazy::new(|| {
    let retry = RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(3)
    );
    ClientBuilder::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap()
    )
    .with(retry)
    .build()
});
```

**Impact:** 1.25x faster, 3MB memory savings
**Effort:** 2-3h

### 2.2 ETag Caching for Registry

**Location:** `crates/spn/src/index/client.rs`

```rust
async fn fetch_index_file(&self, index_path: &str) -> Result<String> {
    let cache_path = self.cache_path(index_path);
    let etag = self.load_etag(&cache_path)?;

    let mut req = HTTP_CLIENT.get(&url);
    if let Some(tag) = etag {
        req = req.header("If-None-Match", tag);
    }

    let response = req.send().await?;
    match response.status() {
        StatusCode::NOT_MODIFIED => std::fs::read_to_string(&cache_path),
        StatusCode::OK => {
            let etag = response.headers().get("etag").cloned();
            let content = response.text().await?;
            self.save_etag(&cache_path, &etag)?;
            std::fs::write(&cache_path, &content)?;
            Ok(content)
        }
        _ => Err(...)
    }
}
```

**Impact:** 5x faster on repeat `spn search/info` calls
**Effort:** 3-4h

### 2.3 Parallel Tarball Downloads

**Location:** `crates/spn/src/index/downloader.rs`

```rust
use futures::stream::{self, StreamExt};
const MAX_CONCURRENT: usize = 4;

// BEFORE: Sequential
for pkg in packages {
    downloader.download_entry(&pkg).await?;
}

// AFTER: Parallel
stream::iter(packages)
    .map(|pkg| async { downloader.download_entry(&pkg).await })
    .buffer_unordered(MAX_CONCURRENT)
    .collect::<Vec<_>>()
    .await
```

**Impact:** 3-4x faster for multi-package installs
**Effort:** 2-3h

---

## Part 3: Error Handling & Observability (v0.15.0)

### 3.1 Wire Verbose Flag to Log Level

**Location:** `crates/spn/src/main.rs:1091-1093`

```rust
// BEFORE: verbose flag ignored
let _ = cli.verbose;

// AFTER: Connect to tracing
let level = match cli.verbose {
    0 => "warn",
    1 => "info",
    2 => "debug",
    _ => "trace",
};
tracing_subscriber::fmt()
    .with_env_filter(format!("spn={}", level))
    .init();
```

**Effort:** 1-2h | **Impact:** High (debugging)

### 3.2 Panic Hook for Crash Context

```rust
// NEW: crates/spn/src/crash.rs
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let msg = panic_info.payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown panic".to_string());

        eprintln!("\n  ✗ Internal Error: {}", msg);
        eprintln!("  Report: https://github.com/supernovae-st/supernovae-cli/issues\n");
        default_hook(panic_info);
    }));
}
```

**Effort:** 1-2h | **Impact:** Medium (user trust)

### 3.3 Error Codes & Categories

**Location:** `crates/spn/src/error.rs`

```rust
pub enum ErrorCategory {
    Permanent,   // Don't retry
    Transient,   // Safe to retry
    UserError,   // User misconfiguration
    Internal,    // Bug
}

impl SpnError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PackageNotFound(_) => "E001",
            Self::NetworkError(_) => "E002",
            // ...
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::NetworkError(_) => ErrorCategory::Transient,
            Self::PackageNotFound(_) => ErrorCategory::UserError,
            // ...
        }
    }
}
```

**Effort:** 2-3h | **Impact:** Medium (machine parsing)

---

## Part 4: CLI Patterns (v0.15.0)

### 4.1 Global Automation Flags

**Location:** `crates/spn/src/main.rs` (Cli struct)

```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Skip confirmation prompts (assume yes)
    #[arg(long, short = 'y', global = true)]
    force: bool,

    /// Non-interactive mode (error if input needed)
    #[arg(long, global = true, env = "SPN_NON_INTERACTIVE")]
    non_interactive: bool,

    /// Output format (text, json, yaml)
    #[arg(long, short = 'o', global = true)]
    output: Option<OutputFormat>,
}
```

**Effort:** 1-2h | **Impact:** High (CI/CD)

### 4.2 Unified Output Format

```rust
// NEW: crates/spn/src/output.rs
pub enum OutputFormat {
    Text,       // Default (colored)
    Json,       // Structured JSON
    JsonCompact,// JSON without formatting
    Yaml,       // YAML format
}

// In commands - replace scattered --json flags
pub async fn run(format: OutputFormat) -> Result<()> {
    let result = perform_operation()?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputFormat::Text => println!("{}", result.to_human_readable()),
        // ...
    }
}
```

**Effort:** 2-3h | **Impact:** High (consistency)

### 4.3 Short Command Aliases

**Location:** `crates/spn/src/main.rs` (Commands enum)

```rust
#[command(visible_alias = "a")]
Add { ... }

#[command(visible_alias = "rm")]
Remove { ... }

#[command(visible_alias = "i")]
Install { ... }

#[command(visible_alias = "ls", visible_alias = "l")]
List { ... }
```

**Effort:** 30 min | **Impact:** Low (convenience)

### 4.4 Progress Indicators for Network Commands

**Location:** `commands/add.rs`, `commands/search.rs`

```rust
use crate::ux::progress::transforming_spinner;

pub async fn run_add(package: &str) -> Result<()> {
    let spinner = transforming_spinner("Resolving dependencies...");
    let deps = resolve_dependencies(package).await?;
    spinner.finish_success("Dependencies resolved");
    // ...
}
```

**Effort:** 1-2h | **Impact:** Medium (UX)

---

## Part 5: Developer Experience (v0.15.0-v0.16.0)

### 5.1 Shell Completions (HIGH PRIORITY)

**Note:** `clap_complete` is already a dependency but NOT implemented!

```rust
// NEW: crates/spn/src/commands/completion.rs
use clap_complete::{generate, shells};

pub fn run(shell: &str) -> Result<()> {
    let shell = match shell {
        "bash" => shells::Bash,
        "zsh" => shells::Zsh,
        "fish" => shells::Fish,
        "powershell" => shells::PowerShell,
        _ => return Err(SpnError::InvalidShell),
    };
    generate(shell, &mut Cli::command(), "spn", &mut std::io::stdout());
    Ok(())
}
```

**Usage:**
```bash
spn completion bash >> ~/.bashrc
spn completion zsh >> ~/.zshrc
spn completion fish > ~/.config/fish/completions/spn.fish
```

**Effort:** 2-3h | **Impact:** High (UX)

### 5.2 Man Page Generation

```toml
# Cargo.toml - ADD
[build-dependencies]
clap_mangen = "0.2"
```

```rust
// build.rs - ADD
fn main() {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut file = std::fs::File::create("target/spn.1").unwrap();
    man.render(&mut file).unwrap();
}
```

**Effort:** 2-3h | **Impact:** Medium (discoverability)

### 5.3 XDG Config Support (Linux)

```rust
// NEW: crates/spn/src/config/paths.rs
fn config_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().join(".config"))
            .join("spn")
    }
    #[cfg(target_os = "macos")]
    { dirs::preference_dir().join("spn") }
    #[cfg(target_os = "windows")]
    { dirs::config_dir().join("spn") }
}
```

**Effort:** 1-2h | **Impact:** Medium (Linux users)

### 5.4 Offline Mode

```rust
// In Cli struct
#[arg(long, global = true)]
offline: bool,

// In index client
pub async fn fetch_package(&self, name: &str) -> Result<Vec<IndexEntry>> {
    if self.offline {
        return self.load_from_cache(name);
    }
    // Normal network fetch...
}
```

**Effort:** 2-3h | **Impact:** Medium (airgapped envs)

---

## Implementation Roadmap

### v0.15.0 (Week 1-2) - Security & Performance

| Task | Effort | Priority |
|------|--------|----------|
| `cargo audit` in CI | 15 min | 🔴 Critical |
| Verbose flag wiring | 1-2h | 🔴 High |
| Shared HTTP client | 2-3h | 🔴 High |
| ETag caching | 3-4h | 🔴 High |
| Global `--non-interactive` | 1-2h | 🔴 High |
| Panic hook | 1-2h | 🟡 Medium |
| Symlink rejection | 1h | 🟡 Medium |
| SBOM generation | 30 min | 🟡 Medium |

**Total:** ~12-17h

### v0.16.0 (Week 3-4) - DX & CLI Polish

| Task | Effort | Priority |
|------|--------|----------|
| Shell completions | 2-3h | 🔴 High |
| Parallel downloads | 2-3h | 🔴 High |
| Unified output format | 2-3h | 🟡 Medium |
| Man pages | 2-3h | 🟡 Medium |
| Error codes | 2-3h | 🟡 Medium |
| Progress indicators | 1-2h | 🟡 Medium |
| Command aliases | 30 min | 🟢 Low |

**Total:** ~12-17h

### v0.17.0+ (Future) - Advanced

| Task | Effort | Priority |
|------|--------|----------|
| Ed25519 signatures | 4-6h | 🔴 High |
| VS Code extension | 4-6h | 🟡 Medium |
| Plugin system | 8-10h | 🟡 Medium |
| Offline mode | 2-3h | 🟡 Medium |
| MCP capability sandbox | 6-8h | 🟡 Medium |
| Audit logging | 3-4h | 🟢 Low |

---

## Performance Benchmarks (Expected)

| Operation | Current | After v0.15 | Speedup |
|-----------|---------|-------------|---------|
| `spn search` (cold) | 250ms | 200ms | 1.25x |
| `spn search` (warm) | 250ms | 50ms | 5x |
| `spn install` (4 pkgs) | 8-20s | 2-5s | 3-4x |
| Memory usage | ~6MB | ~3MB | 2x |

---

## Files to Create/Modify

### New Files

```
crates/spn/src/
├── crash.rs              # Panic hook
├── output.rs             # Unified output format
├── commands/
│   └── completion.rs     # Shell completions
└── config/
    └── paths.rs          # XDG support

crates/spn-client/src/
└── http.rs               # Shared HTTP client

.github/workflows/
└── test.yml              # Add cargo-audit
```

### Modified Files

```
crates/spn/src/
├── main.rs               # Global flags, verbose wiring
├── error.rs              # Error codes/categories
├── index/client.rs       # ETag caching
├── index/downloader.rs   # Parallel downloads
└── storage/local.rs      # Symlink rejection

Cargo.toml                # Add clap_mangen
build.rs                  # Man page generation
```

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| CI security scanning | ❌ None | ✅ cargo-audit |
| Shell completions | ❌ None | ✅ bash/zsh/fish |
| Man pages | ❌ None | ✅ `man spn` |
| SBOM | ❌ None | ✅ CycloneDX |
| HTTP connection reuse | ❌ None | ✅ Pooled |
| ETag caching | ❌ None | ✅ Enabled |
| Test count | 706 | 750+ |

---

## References

- **Agent 1:** CLI Patterns (ripgrep, gh, cargo comparison)
- **Agent 2:** Error Handling & Observability
- **Agent 3:** Performance & Caching
- **Agent 4:** Security Hardening
- **Agent 5:** Developer Experience

**Research Date:** 2026-03-07
