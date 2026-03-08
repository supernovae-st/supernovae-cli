# Master Implementation Plan v0.15.0

**Status:** Active
**Author:** Claude + Thibaut
**Date:** 2026-03-08
**Approach:** Test-Driven Development (RED → GREEN → REFACTOR)
**Related:** ADR-001, ADR-002, ADR-003, spn-v015-v018-roadmap.md

---

## TDD Philosophy

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  TDD CYCLE                                                                    ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  1. RED    — Write a failing test that defines expected behavior              ║
║  2. GREEN  — Write MINIMAL code to make the test pass                         ║
║  3. REFACTOR — Improve code quality without changing behavior                 ║
║                                                                               ║
║  Tests are written FIRST. No code without a failing test.                     ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

---

## Implementation Order (v0.15.0)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  IMPLEMENTATION SEQUENCE                                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  1. spn model run       (P0) — 8 tests, ~150 lines                              │
│  2. spn completion      (P0) — 12 tests, ~200 lines                             │
│  3. spn --verbose       (P1) — 10 tests, ~100 lines                             │
│  4. spn mcp logs        (P1) — 14 tests, ~250 lines                             │
│  5. spn topic           (P2) — 6 tests, ~80 lines                               │
│                                                                                 │
│  Total: 50 tests, ~780 lines of production code                                 │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Feature 1: `spn model run`

### 1.1 Test Specification

```rust
// crates/spn/src/commands/model/run_tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    // === RED: Basic prompt execution ===
    #[tokio::test]
    async fn test_run_basic_prompt() {
        // Given: Ollama is running with llama3.2 loaded
        // When: spn model run llama3.2 "Hello"
        // Then: Returns non-empty response
    }

    #[tokio::test]
    async fn test_run_model_not_found() {
        // Given: Model "nonexistent" does not exist
        // When: spn model run nonexistent "Hello"
        // Then: Error with "model not found" message
    }

    #[tokio::test]
    async fn test_run_ollama_not_running() {
        // Given: Ollama service is not running
        // When: spn model run llama3.2 "Hello"
        // Then: Error with "Ollama not running" suggestion
    }

    // === RED: Streaming mode ===
    #[tokio::test]
    async fn test_run_stream_output() {
        // Given: Model loaded
        // When: spn model run --stream llama3.2 "Count to 5"
        // Then: Tokens arrive incrementally (test with callback)
    }

    // === RED: Input modes ===
    #[tokio::test]
    async fn test_run_stdin_input() {
        // Given: Prompt piped via stdin
        // When: echo "Hello" | spn model run llama3.2 -
        // Then: Uses stdin as prompt
    }

    #[tokio::test]
    async fn test_run_file_input() {
        // Given: File exists at /tmp/test.txt
        // When: spn model run llama3.2 "Review:" @/tmp/test.txt
        // Then: File content appended to prompt
    }

    // === RED: Options ===
    #[tokio::test]
    async fn test_run_json_output() {
        // Given: Model loaded
        // When: spn model run --json llama3.2 "List 3 colors"
        // Then: Output is valid JSON
    }

    #[tokio::test]
    async fn test_run_with_temperature() {
        // Given: Model loaded
        // When: spn model run --temp 0.0 llama3.2 "Say hello"
        // Then: Deterministic output (same every time)
    }
}
```

### 1.2 CLI Specification

```rust
// crates/spn/src/commands/model.rs

/// Run inference on a model
#[derive(Args)]
pub struct RunArgs {
    /// Model name (e.g., llama3.2, mistral:7b)
    pub model: String,

    /// Prompt text (use - for stdin, @file for file input)
    pub prompt: String,

    /// Stream output tokens as they arrive
    #[arg(short, long)]
    pub stream: bool,

    /// Context window size
    #[arg(long, default_value = "4096")]
    pub context: u32,

    /// Temperature (0.0 - 2.0)
    #[arg(long, short = 't', default_value = "0.7")]
    pub temperature: f32,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// System prompt
    #[arg(long, short = 's')]
    pub system: Option<String>,
}
```

### 1.3 Implementation Steps

```
Step 1: Write test_run_model_not_found (RED)
Step 2: Add Run variant to ModelCommands enum
Step 3: Implement error case for model not found (GREEN)
Step 4: Write test_run_basic_prompt (RED)
Step 5: Implement basic prompt execution (GREEN)
Step 6: Write test_run_stream_output (RED)
Step 7: Implement streaming (GREEN)
Step 8: Write remaining tests and implement (RED → GREEN)
Step 9: REFACTOR - extract common patterns
```

### 1.4 Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `crates/spn/src/commands/model.rs` | Modify | Add `Run` variant |
| `crates/spn/src/commands/model/run.rs` | Create | Run implementation |
| `crates/spn/src/commands/model/run_tests.rs` | Create | Tests |
| `crates/spn-ollama/src/chat.rs` | Modify | Add chat/generate |

---

## Feature 2: `spn completion install`

### 2.1 Test Specification

```rust
// crates/spn/src/commands/completion_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // === RED: Shell detection ===
    #[test]
    fn test_detect_shell_bash() {
        std::env::set_var("SHELL", "/bin/bash");
        assert_eq!(detect_shell(), Some(Shell::Bash));
    }

    #[test]
    fn test_detect_shell_zsh() {
        std::env::set_var("SHELL", "/usr/local/bin/zsh");
        assert_eq!(detect_shell(), Some(Shell::Zsh));
    }

    #[test]
    fn test_detect_shell_fish() {
        std::env::set_var("SHELL", "/opt/homebrew/bin/fish");
        assert_eq!(detect_shell(), Some(Shell::Fish));
    }

    // === RED: Generation ===
    #[test]
    fn test_generate_bash_completions() {
        let script = generate_completions(Shell::Bash);
        assert!(script.contains("complete"));
        assert!(script.contains("spn"));
    }

    #[test]
    fn test_generate_zsh_completions() {
        let script = generate_completions(Shell::Zsh);
        assert!(script.contains("#compdef"));
        assert!(script.contains("spn"));
    }

    #[test]
    fn test_generate_fish_completions() {
        let script = generate_completions(Shell::Fish);
        assert!(script.contains("complete -c spn"));
    }

    // === RED: Installation ===
    #[test]
    fn test_install_bash_writes_to_bashrc() {
        let dir = tempdir().unwrap();
        let bashrc = dir.path().join(".bashrc");
        std::fs::write(&bashrc, "# existing content\n").unwrap();

        install_completion(Shell::Bash, &bashrc).unwrap();

        let content = std::fs::read_to_string(&bashrc).unwrap();
        assert!(content.contains("# spn completions"));
        assert!(content.contains("# existing content"));
    }

    #[test]
    fn test_install_is_idempotent() {
        let dir = tempdir().unwrap();
        let bashrc = dir.path().join(".bashrc");
        std::fs::write(&bashrc, "").unwrap();

        install_completion(Shell::Bash, &bashrc).unwrap();
        install_completion(Shell::Bash, &bashrc).unwrap();

        let content = std::fs::read_to_string(&bashrc).unwrap();
        let count = content.matches("# spn completions").count();
        assert_eq!(count, 1, "Should not duplicate");
    }

    #[test]
    fn test_install_fish_creates_file() {
        let dir = tempdir().unwrap();
        let fish_dir = dir.path().join("fish/completions");
        let fish_file = fish_dir.join("spn.fish");

        install_completion_fish(&fish_file).unwrap();

        assert!(fish_file.exists());
    }

    // === RED: Uninstall ===
    #[test]
    fn test_uninstall_removes_lines() {
        let dir = tempdir().unwrap();
        let bashrc = dir.path().join(".bashrc");
        std::fs::write(&bashrc, "before\n# spn completions\nCOMPLETION_CODE\n# end spn\nafter").unwrap();

        uninstall_completion(Shell::Bash, &bashrc).unwrap();

        let content = std::fs::read_to_string(&bashrc).unwrap();
        assert!(!content.contains("# spn completions"));
        assert!(content.contains("before"));
        assert!(content.contains("after"));
    }

    // === RED: Status ===
    #[test]
    fn test_status_shows_installed() {
        // Given: Bash completion is installed
        // When: spn completion status
        // Then: Shows "bash: installed"
    }
}
```

### 2.2 CLI Specification

```rust
// crates/spn/src/commands/completion.rs

#[derive(Subcommand)]
pub enum CompletionCommands {
    /// Generate completion script (print to stdout)
    Bash,
    Zsh,
    Fish,
    PowerShell,

    /// Install completions to shell config
    Install {
        /// Target shell (auto-detect if omitted)
        shell: Option<Shell>,

        /// Show what would be done
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove installed completions
    Uninstall {
        /// Target shell
        shell: Shell,
    },

    /// Show completion installation status
    Status,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}
```

### 2.3 Installation Paths

| Shell | Path | Method |
|-------|------|--------|
| Bash | `~/.bashrc` | Append eval block |
| Zsh | `~/.zshrc` | Append eval block |
| Fish | `~/.config/fish/completions/spn.fish` | Write file |
| PowerShell | `$PROFILE` | Append block |

### 2.4 Implementation Steps

```
Step 1: Write test_generate_bash_completions (RED)
Step 2: Implement generate_completions() using clap_complete (GREEN)
Step 3: Write test_detect_shell_* (RED)
Step 4: Implement detect_shell() (GREEN)
Step 5: Write test_install_* (RED)
Step 6: Implement install_completion() (GREEN)
Step 7: Write test_uninstall_* (RED)
Step 8: Implement uninstall_completion() (GREEN)
Step 9: REFACTOR - clean up file handling
```

---

## Feature 3: `spn --verbose`

### 3.1 Test Specification

```rust
// crates/spn/src/verbosity_tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    // === RED: Verbosity levels ===
    #[test]
    fn test_verbosity_default_is_normal() {
        let args = parse_args(&["spn", "provider", "list"]);
        assert_eq!(args.verbosity, Verbosity::Normal);
    }

    #[test]
    fn test_verbosity_v_is_verbose() {
        let args = parse_args(&["spn", "-v", "provider", "list"]);
        assert_eq!(args.verbosity, Verbosity::Verbose);
    }

    #[test]
    fn test_verbosity_vv_is_debug() {
        let args = parse_args(&["spn", "-vv", "provider", "list"]);
        assert_eq!(args.verbosity, Verbosity::Debug);
    }

    #[test]
    fn test_verbosity_quiet_suppresses() {
        let args = parse_args(&["spn", "--quiet", "provider", "list"]);
        assert_eq!(args.verbosity, Verbosity::Quiet);
    }

    // === RED: Output filtering ===
    #[test]
    fn test_verbose_shows_timing() {
        let output = capture_output(|| {
            with_verbosity(Verbosity::Verbose, || {
                timed("Test operation", || {
                    std::thread::sleep(Duration::from_millis(10));
                });
            });
        });
        assert!(output.contains("completed in"));
    }

    #[test]
    fn test_normal_hides_timing() {
        let output = capture_output(|| {
            with_verbosity(Verbosity::Normal, || {
                timed("Test operation", || {
                    std::thread::sleep(Duration::from_millis(10));
                });
            });
        });
        assert!(!output.contains("completed in"));
    }

    #[test]
    fn test_quiet_hides_info() {
        let output = capture_output(|| {
            with_verbosity(Verbosity::Quiet, || {
                info("This should be hidden");
                error("This should be shown");
            });
        });
        assert!(!output.contains("This should be hidden"));
        assert!(output.contains("This should be shown"));
    }

    // === RED: Integration ===
    #[test]
    fn test_verbose_with_provider_list() {
        // Run actual command with -v
        // Verify extra output appears
    }
}
```

### 3.2 Implementation

```rust
// crates/spn/src/verbosity.rs

use std::cell::RefCell;

thread_local! {
    static VERBOSITY: RefCell<Verbosity> = RefCell::new(Verbosity::Normal);
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,   // -q
    Normal,  // default
    Verbose, // -v
    Debug,   // -vv
}

pub fn set_verbosity(v: Verbosity) {
    VERBOSITY.with(|cell| *cell.borrow_mut() = v);
}

pub fn get_verbosity() -> Verbosity {
    VERBOSITY.with(|cell| *cell.borrow())
}

pub fn verbose(msg: impl std::fmt::Display) {
    if get_verbosity() >= Verbosity::Verbose {
        eprintln!("{} {}", console::style("[VERBOSE]").dim(), msg);
    }
}

pub fn debug(msg: impl std::fmt::Display) {
    if get_verbosity() >= Verbosity::Debug {
        eprintln!("{} {}", console::style("[DEBUG]").cyan().dim(), msg);
    }
}

pub fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = std::time::Instant::now();
    let result = f();
    verbose(format!("{} completed in {:?}", label, start.elapsed()));
    result
}
```

### 3.3 CLI Integration

```rust
// crates/spn/src/main.rs

#[derive(Parser)]
#[command(name = "spn")]
pub struct Cli {
    /// Increase verbosity (-v for verbose, -vv for debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

fn main() {
    let cli = Cli::parse();

    let verbosity = if cli.quiet {
        Verbosity::Quiet
    } else {
        match cli.verbose {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    };

    set_verbosity(verbosity);

    // ... run command
}
```

---

## Feature 4: `spn mcp logs`

### 4.1 Test Specification

```rust
// crates/spn/src/mcp/logs_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // === RED: Log file management ===
    #[test]
    fn test_log_path_by_date() {
        let manager = LogManager::new(PathBuf::from("/tmp"));
        let path = manager.log_path("neo4j", date!(2026-03-08));
        assert_eq!(path, PathBuf::from("/tmp/neo4j-2026-03-08.log"));
    }

    #[test]
    fn test_current_log_path() {
        let manager = LogManager::new(PathBuf::from("/tmp"));
        let path = manager.current_log_path("neo4j");
        assert!(path.to_string_lossy().contains("neo4j-"));
        assert!(path.to_string_lossy().contains(".log"));
    }

    // === RED: Reading logs ===
    #[test]
    fn test_tail_last_n_lines() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("test.log");
        std::fs::write(&log, "line1\nline2\nline3\nline4\nline5\n").unwrap();

        let manager = LogManager::new(dir.path().to_path_buf());
        let lines = manager.tail(&log, 3).unwrap();

        assert_eq!(lines, vec!["line3", "line4", "line5"]);
    }

    #[test]
    fn test_tail_empty_file() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("empty.log");
        std::fs::write(&log, "").unwrap();

        let manager = LogManager::new(dir.path().to_path_buf());
        let lines = manager.tail(&log, 10).unwrap();

        assert!(lines.is_empty());
    }

    #[test]
    fn test_tail_file_not_found() {
        let manager = LogManager::new(PathBuf::from("/nonexistent"));
        let result = manager.tail(&PathBuf::from("/nonexistent/foo.log"), 10);
        assert!(result.is_err());
    }

    // === RED: Filtering ===
    #[test]
    fn test_filter_by_level() {
        let lines = vec![
            "[INFO] Starting",
            "[ERROR] Failed",
            "[DEBUG] Details",
            "[ERROR] Another error",
        ];

        let filtered = filter_by_level(&lines, LogLevel::Error);
        assert_eq!(filtered.len(), 2);
        assert!(filtered[0].contains("Failed"));
    }

    #[test]
    fn test_filter_by_time_range() {
        let lines = vec![
            "[2026-03-08T10:00:00Z] Early",
            "[2026-03-08T14:00:00Z] Middle",
            "[2026-03-08T18:00:00Z] Late",
        ];

        let since = DateTime::parse_from_rfc3339("2026-03-08T12:00:00Z").unwrap();
        let filtered = filter_by_time(&lines, since);
        assert_eq!(filtered.len(), 2);
    }

    // === RED: Log rotation ===
    #[test]
    fn test_rotate_deletes_old_logs() {
        let dir = tempdir().unwrap();
        let manager = LogManager::new(dir.path().to_path_buf());

        // Create old logs
        std::fs::write(dir.path().join("neo4j-2026-02-01.log"), "old").unwrap();
        std::fs::write(dir.path().join("neo4j-2026-03-07.log"), "recent").unwrap();
        std::fs::write(dir.path().join("neo4j-2026-03-08.log"), "today").unwrap();

        manager.rotate(7).unwrap(); // Keep 7 days

        assert!(!dir.path().join("neo4j-2026-02-01.log").exists());
        assert!(dir.path().join("neo4j-2026-03-07.log").exists());
        assert!(dir.path().join("neo4j-2026-03-08.log").exists());
    }

    // === RED: Live follow ===
    #[tokio::test]
    async fn test_follow_receives_new_lines() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("follow.log");
        std::fs::write(&log, "initial\n").unwrap();

        let manager = LogManager::new(dir.path().to_path_buf());
        let mut stream = manager.follow(&log).await.unwrap();

        // Write new line
        std::fs::OpenOptions::new()
            .append(true)
            .open(&log)
            .unwrap()
            .write_all(b"new line\n")
            .unwrap();

        // Should receive it
        let line = stream.next().await;
        assert_eq!(line, Some("new line".to_string()));
    }

    // === RED: Clear logs ===
    #[test]
    fn test_clear_logs_for_server() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("neo4j-2026-03-08.log"), "content").unwrap();
        std::fs::write(dir.path().join("github-2026-03-08.log"), "content").unwrap();

        let manager = LogManager::new(dir.path().to_path_buf());
        manager.clear("neo4j").unwrap();

        assert!(!dir.path().join("neo4j-2026-03-08.log").exists());
        assert!(dir.path().join("github-2026-03-08.log").exists());
    }
}
```

### 4.2 CLI Specification

```rust
// crates/spn/src/commands/mcp.rs

#[derive(Subcommand)]
pub enum McpCommands {
    // ... existing commands ...

    /// View MCP server logs
    Logs {
        /// Server name (or --all)
        server: Option<String>,

        /// Show all servers' logs combined
        #[arg(long)]
        all: bool,

        /// Follow log output (like tail -f)
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,

        /// Filter by log level
        #[arg(long)]
        level: Option<LogLevel>,

        /// Show logs since duration (e.g., "1h", "30m", "7d")
        #[arg(long)]
        since: Option<String>,

        /// Clear logs
        #[arg(long)]
        clear: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
```

### 4.3 Log Storage Structure

```
~/.spn/logs/
├── neo4j-2026-03-08.log
├── neo4j-2026-03-07.log
├── github-2026-03-08.log
└── .config.json  # { "rotation_days": 30, "max_size_mb": 100 }
```

### 4.4 Log Format

```
[2026-03-08T14:23:45.123Z] [INFO] neo4j: Connection established
[2026-03-08T14:23:46.456Z] [DEBUG] neo4j: Query: MATCH (n) RETURN count(n)
[2026-03-08T14:23:47.789Z] [ERROR] neo4j: Connection timeout after 30s
```

---

## Feature 5: `spn topic`

### 5.1 Test Specification

```rust
// crates/spn/src/commands/topic_tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_topics() {
        let topics = list_topics();
        assert!(topics.contains(&"secrets"));
        assert!(topics.contains(&"mcp"));
        assert!(topics.contains(&"daemon"));
    }

    #[test]
    fn test_get_topic_secrets() {
        let content = get_topic("secrets").unwrap();
        assert!(content.contains("credential"));
        assert!(content.contains("keychain"));
    }

    #[test]
    fn test_get_topic_unknown() {
        let result = get_topic("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_topic_suggestions() {
        let result = get_topic("secret"); // Without 's'
        if let Err(e) = result {
            assert!(e.to_string().contains("secrets")); // Suggests correct name
        }
    }

    #[test]
    fn test_all_topics_have_content() {
        for name in list_topics() {
            let content = get_topic(name).expect(&format!("Topic {} should exist", name));
            assert!(!content.is_empty(), "Topic {} is empty", name);
        }
    }
}
```

### 5.2 Topics Content

```
crates/spn/assets/topics/
├── secrets.md          # Credential management guide
├── mcp.md              # MCP server setup and troubleshooting
├── sync.md             # Editor synchronization
├── daemon.md           # Daemon architecture
├── troubleshoot.md     # Common issues and solutions
└── models.md           # Local model management
```

---

## Verification Protocol (Ralph Wiggum)

After each feature implementation:

```bash
# 1. Run all tests
cargo test --workspace

# 2. Run clippy (zero warnings)
cargo clippy --workspace -- -D warnings

# 3. Run the new command manually
spn model run llama3.2 "Hello"
spn completion status
spn --verbose provider list

# 4. Check for regressions
cargo test --workspace --no-fail-fast 2>&1 | grep -E "(FAILED|passed)"

# 5. Verify test count increased
cargo test --workspace 2>&1 | tail -1
# Expected: test result: ok. XXX passed; 0 failed
```

---

## Commit Strategy

One commit per logical unit:

```bash
# Feature commits
test(model): add tests for model run command
feat(model): implement spn model run basic prompt
feat(model): add streaming support to model run
feat(model): add file input support (@syntax)

# Don't batch multiple features
# Don't commit broken tests
```

---

## Success Criteria

| Feature | Tests | Passing | Coverage |
|---------|-------|---------|----------|
| model run | 8 | 8/8 | 90%+ |
| completion | 12 | 12/12 | 95%+ |
| --verbose | 10 | 10/10 | 85%+ |
| mcp logs | 14 | 14/14 | 90%+ |
| topic | 6 | 6/6 | 100% |

**Total v0.15.0:** 50+ new tests, 0 regressions, 0 clippy warnings.

---

## Next Steps After v0.15.0

1. Tag release v0.15.0
2. Update CHANGELOG.md
3. Begin v0.16.0 (operational features)
4. ADR-002/003 implementation (MCP bridge, scheduler)
