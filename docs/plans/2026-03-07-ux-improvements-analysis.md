# UX Improvements Plan - spn-cli

**Date:** 2026-03-07
**Status:** Ready for Implementation
**Based on:** New user testing + Perplexity/Context7 CLI UX research

---

## Executive Summary

spn-cli v0.12.5 has solid foundations but lacks polish compared to best-in-class CLIs (gh, ripgrep). This plan details 5 UX improvements across two releases.

---

## Test Results Summary

### All Commands Tested (100% Pass Rate)

```
✓ spn --help              Clear structure, emojis, quick start
✓ spn --version           v0.12.5
✓ spn doctor              12 checks in 560ms
✓ spn topic [8 topics]    Progressive disclosure pattern
✓ spn config show/where   Fixed: helpful message when empty
✓ spn mcp list            8 servers displayed correctly
✓ spn provider list       6 providers configured
✓ spn model list          1 model (llama3.2:1b)
✓ spn nk [7 commands]     trace, new, config all work
✓ spn nv [14 commands]    search, entity, stats, diff all work
✓ "Did you mean?"         Typo suggestions for all commands
```

### CI/CD Status

| Check | Status |
|-------|--------|
| cargo test | **830 tests PASSED** |
| cargo clippy | **PASSED** |
| cargo fmt | **PASSED** |
| crates.io | v0.12.5 published |
| Homebrew | v0.12.0 (update pending for v0.12.5) |

---

## v0.13.0: Quick Wins (2 Features)

### Feature 1: Shell Completions

**Priority:** P0 (Highest Impact)
**Effort:** 2-3 hours
**Impact:** Dramatically improves discoverability and typing speed

#### User Experience

```bash
# After installation
spn completions bash >> ~/.bashrc
spn completions zsh >> ~/.zshrc
spn completions fish > ~/.config/fish/completions/spn.fish
spn completions powershell >> $PROFILE

# Usage
spn m<TAB>        → spn mcp / model
spn mcp a<TAB>    → spn mcp add
spn provider <TAB> → list set get delete migrate test status
```

#### Implementation Plan

**Step 1: Add dependency**

```toml
# crates/spn/Cargo.toml
[dependencies]
clap_complete = "4"
```

**Step 2: Add command enum**

```rust
// crates/spn/src/main.rs

#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: CompletionShell,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}
```

**Step 3: Add command handler**

```rust
// crates/spn/src/commands/completions.rs

use clap::CommandFactory;
use clap_complete::{generate, Shell};

pub fn run(shell: CompletionShell) -> Result<()> {
    let mut cmd = crate::Cli::command();
    let shell = match shell {
        CompletionShell::Bash => Shell::Bash,
        CompletionShell::Zsh => Shell::Zsh,
        CompletionShell::Fish => Shell::Fish,
        CompletionShell::PowerShell => Shell::PowerShell,
        CompletionShell::Elvish => Shell::Elvish,
    };
    generate(shell, &mut cmd, "spn", &mut std::io::stdout());
    Ok(())
}
```

**Step 4: Update main.rs**

```rust
Commands::Completions { shell } => commands::completions::run(shell),
```

#### Files to Modify

| File | Change |
|------|--------|
| `crates/spn/Cargo.toml` | Add `clap_complete = "4"` |
| `crates/spn/src/main.rs` | Add `Completions` command + `CompletionShell` enum |
| `crates/spn/src/commands/mod.rs` | Add `pub mod completions;` |
| `crates/spn/src/commands/completions.rs` | New file with handler |

#### Testing

```bash
# Generate and test
./target/release/spn completions bash > /tmp/spn.bash
source /tmp/spn.bash
spn <TAB><TAB>  # Should list all commands

# Verify all shells generate without error
for shell in bash zsh fish powershell elvish; do
    ./target/release/spn completions $shell > /dev/null && echo "✓ $shell"
done
```

---

### Feature 2: Colorized Help

**Priority:** P1 (Visual Polish)
**Effort:** 1 hour
**Impact:** Professional appearance, better readability

#### Current vs Target

```
BEFORE (gray monochrome):
Usage: spn [OPTIONS] <COMMAND>

Commands:
  add       Add a package to the project
  remove    Remove a package from the project

AFTER (colorized):
Usage: spn [OPTIONS] <COMMAND>    ← Green header

Commands:
  add       Add a package...      ← Cyan command name
  remove    Remove a package...   ← Cyan command name

Options:
  -v, --verbose    ← Yellow placeholder
  -h, --help       ← Yellow placeholder
```

#### Implementation Plan

**Step 1: Define styles**

```rust
// crates/spn/src/ux.rs (add to existing file)

use clap::builder::{styling::AnsiColor, Styles};

/// CLI color scheme matching SuperNovae brand
pub fn cli_styles() -> Styles {
    Styles::styled()
        // Headers: bright green (matches ✓ success indicators)
        .header(AnsiColor::BrightGreen.on_default().bold())
        .usage(AnsiColor::BrightGreen.on_default().bold())
        // Commands/literals: cyan (matches URL and command hints)
        .literal(AnsiColor::BrightCyan.on_default())
        // Placeholders: yellow (stands out, indicates user input needed)
        .placeholder(AnsiColor::Yellow.on_default())
        // Errors: red (standard error color)
        .error(AnsiColor::BrightRed.on_default().bold())
        // Valid values: green
        .valid(AnsiColor::BrightGreen.on_default())
        // Invalid values: red
        .invalid(AnsiColor::BrightRed.on_default())
}
```

**Step 2: Apply to CLI**

```rust
// crates/spn/src/main.rs

#[derive(Parser)]
#[command(name = "spn")]
#[command(styles = ux::cli_styles())]  // Add this line
#[command(author = "SuperNovae Studio")]
// ... rest unchanged
```

#### Files to Modify

| File | Change |
|------|--------|
| `crates/spn/src/ux.rs` | Add `cli_styles()` function |
| `crates/spn/src/main.rs` | Add `#[command(styles = ux::cli_styles())]` |

#### Testing

```bash
# Visual inspection
./target/release/spn --help
./target/release/spn mcp --help
./target/release/spn notacommand  # Error should be red

# Verify NO_COLOR is respected
NO_COLOR=1 ./target/release/spn --help  # Should be plain
```

---

## v0.14.0: Unified Component Model (3 Features)

### Feature 3: Progress Indicators

**Priority:** P1
**Effort:** 4-6 hours
**Impact:** Essential for long operations (model downloads)

#### User Experience

```
$ spn model pull llama3.2:7b
Downloading llama3.2:7b
  [████████████████████░░░░░░░░░░░░░░░░░░░░] 52% (2.1/4.0 GB) ETA: 3m 42s

$ spn add @novanet/full-schema
Downloading @novanet/full-schema v1.2.0
  [████████████████████████████████████████] 100% (1.2 MB)
Installing dependencies...
  [██████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 25%
```

#### Implementation Plan

**Step 1: Add dependency**

```toml
# crates/spn/Cargo.toml
[dependencies]
indicatif = "0.17"
```

**Step 2: Create progress module**

```rust
// crates/spn/src/progress.rs

use indicatif::{ProgressBar, ProgressStyle};

pub struct DownloadProgress {
    bar: ProgressBar,
}

impl DownloadProgress {
    pub fn new(total: u64, name: &str) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n  [{bar:40.cyan/blue}] {percent}% ({bytes}/{total_bytes}) ETA: {eta}")
                .unwrap()
                .progress_chars("█░")
        );
        bar.set_message(format!("Downloading {}", name));
        Self { bar }
    }

    pub fn update(&self, downloaded: u64) {
        self.bar.set_position(downloaded);
    }

    pub fn finish(&self) {
        self.bar.finish_with_message("✓ Download complete");
    }
}

pub struct SpinnerProgress {
    bar: ProgressBar,
}

impl SpinnerProgress {
    pub fn new(message: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self { bar }
    }

    pub fn finish(&self, message: &str) {
        self.bar.finish_with_message(format!("✓ {}", message));
    }
}
```

**Step 3: Integrate with model pull**

```rust
// crates/spn/src/commands/model.rs

async fn pull_model(name: &str) -> Result<()> {
    let client = OllamaBackend::new()?;

    // Get model info for size
    let info = client.model_info(name).await?;
    let progress = DownloadProgress::new(info.size, name);

    // Pull with progress callback
    client.pull_with_progress(name, |downloaded, _total| {
        progress.update(downloaded);
    }).await?;

    progress.finish();
    Ok(())
}
```

#### Files to Create/Modify

| File | Change |
|------|--------|
| `crates/spn/Cargo.toml` | Add `indicatif = "0.17"` |
| `crates/spn/src/progress.rs` | New file with progress types |
| `crates/spn/src/commands/model.rs` | Use DownloadProgress |
| `crates/spn/src/commands/add.rs` | Use DownloadProgress for packages |

---

### Feature 4: Interactive Prompts

**Priority:** P2
**Effort:** 4-6 hours
**Impact:** Better UX for missing required arguments

#### User Experience

```
$ spn mcp add
? Which MCP server would you like to add?
  ❯ neo4j        - Graph database (recommended)
    github       - GitHub API
    perplexity   - AI search
    firecrawl    - Web scraping
    supadata     - Transcripts & crawling
    (custom)     - Enter npm package name

$ spn provider set
? Which provider would you like to configure?
  ❯ anthropic    - Claude API (sk-ant-...)
    openai       - GPT-4, etc. (sk-proj-...)
    mistral      - Mistral AI
    groq         - Groq (fast)
```

#### Implementation Plan

**Step 1: Add dependency**

```toml
# crates/spn/Cargo.toml
[dependencies]
dialoguer = { version = "0.11", features = ["fuzzy-select"] }
```

**Step 2: Create prompts module**

```rust
// crates/spn/src/prompts.rs

use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input, Password};

pub fn select_mcp_server() -> Result<String> {
    let servers = vec![
        ("neo4j", "Graph database (recommended)"),
        ("github", "GitHub API"),
        ("perplexity", "AI search"),
        ("firecrawl", "Web scraping"),
        ("supadata", "Transcripts & crawling"),
    ];

    let items: Vec<String> = servers.iter()
        .map(|(name, desc)| format!("{:<15} - {}", name, desc))
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Which MCP server would you like to add?")
        .items(&items)
        .default(0)
        .interact()?;

    if selection == items.len() - 1 {
        // Custom option
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter npm package name")
            .interact_text()
    } else {
        Ok(servers[selection].0.to_string())
    }
}

pub fn prompt_api_key(provider: &str) -> Result<String> {
    Password::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter {} API key", provider))
        .interact()
        .map_err(Into::into)
}
```

**Step 3: Update commands**

```rust
// crates/spn/src/commands/mcp.rs

async fn add(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => prompts::select_mcp_server()?,
    };
    // ... rest of implementation
}
```

#### Files to Create/Modify

| File | Change |
|------|--------|
| `crates/spn/Cargo.toml` | Add `dialoguer = "0.11"` |
| `crates/spn/src/prompts.rs` | New file with prompt functions |
| `crates/spn/src/commands/mcp.rs` | Use prompts when arg missing |
| `crates/spn/src/commands/provider.rs` | Use prompts when arg missing |

---

### Feature 5: First Run Experience

**Priority:** P2
**Effort:** 2-3 hours
**Impact:** Better onboarding for new users

#### User Experience

```
$ spn

╭─────────────────────────────────────────────────────────────╮
│                                                             │
│   Welcome to spn - SuperNovae Package Manager!              │
│                                                             │
│   Looks like this is your first time. Here's how to start: │
│                                                             │
│   1. spn setup          Configure your environment          │
│   2. spn provider set   Add API keys securely               │
│   3. spn mcp add        Install MCP servers                 │
│   4. spn doctor         Verify installation                 │
│                                                             │
│   Press Enter to run 'spn setup' or 'q' to see help...      │
│                                                             │
╰─────────────────────────────────────────────────────────────╯

$ # User presses Enter
$ spn setup
Welcome to SuperNovae! Let's configure your environment...
```

#### Implementation Plan

**Step 1: Create first run detection**

```rust
// crates/spn/src/first_run.rs

use std::fs;
use std::path::PathBuf;

fn marker_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("spn")
        .join(".first_run_complete")
}

pub fn is_first_run() -> bool {
    !marker_path().exists()
}

pub fn mark_complete() -> std::io::Result<()> {
    let path = marker_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "")
}
```

**Step 2: Create welcome screen**

```rust
// crates/spn/src/welcome.rs

use dialoguer::{theme::ColorfulTheme, Select};
use console::style;

pub fn show() -> Result<WelcomeAction> {
    println!();
    println!("╭─────────────────────────────────────────────────────────────╮");
    println!("│                                                             │");
    println!("│   {} - SuperNovae Package Manager!              │",
             style("Welcome to spn").cyan().bold());
    println!("│                                                             │");
    println!("│   Looks like this is your first time. Here's how to start: │");
    println!("│                                                             │");
    println!("│   {} {}          │",
             style("1.").dim(), style("spn setup").cyan());
    println!("│   {} {}   │",
             style("2.").dim(), style("spn provider set").cyan());
    println!("│   {} {}        │",
             style("3.").dim(), style("spn mcp add").cyan());
    println!("│   {} {}         │",
             style("4.").dim(), style("spn doctor").cyan());
    println!("│                                                             │");
    println!("╰─────────────────────────────────────────────────────────────╯");
    println!();

    let choices = vec![
        "Run 'spn setup' (recommended)",
        "Show help",
        "Skip welcome (don't show again)",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&choices)
        .default(0)
        .interact()?;

    Ok(match selection {
        0 => WelcomeAction::RunSetup,
        1 => WelcomeAction::ShowHelp,
        _ => WelcomeAction::Skip,
    })
}

pub enum WelcomeAction {
    RunSetup,
    ShowHelp,
    Skip,
}
```

**Step 3: Integrate in main**

```rust
// crates/spn/src/main.rs

fn main() {
    // Check for first run (only if no args provided)
    if std::env::args().len() == 1 && first_run::is_first_run() {
        match welcome::show() {
            Ok(WelcomeAction::RunSetup) => {
                first_run::mark_complete().ok();
                // Run setup
            }
            Ok(WelcomeAction::ShowHelp) => {
                // Show help
            }
            Ok(WelcomeAction::Skip) => {
                first_run::mark_complete().ok();
            }
            Err(_) => {}
        }
        return;
    }

    // Normal CLI flow
    let cli = Cli::parse();
    // ...
}
```

#### Files to Create/Modify

| File | Change |
|------|--------|
| `crates/spn/Cargo.toml` | Add `console = "0.15"` (if not present) |
| `crates/spn/src/first_run.rs` | New file with detection logic |
| `crates/spn/src/welcome.rs` | New file with welcome screen |
| `crates/spn/src/main.rs` | Integrate first run check |

---

## Implementation Schedule

### v0.13.0 (Target: 1-2 days)

| Task | Effort | Owner |
|------|--------|-------|
| Shell completions | 2-3h | - |
| Colorized help | 1h | - |
| Tests + docs | 1h | - |
| Release | 30m | - |

### v0.14.0 (Target: with Unified Model)

| Task | Effort | Depends On |
|------|--------|------------|
| Progress indicators | 4-6h | Unified Model RFC |
| Interactive prompts | 4-6h | dialoguer |
| First run experience | 2-3h | Progress + prompts |
| Tests + docs | 2h | All above |

---

## Success Criteria

### v0.13.0

- [ ] `spn completions bash/zsh/fish/powershell/elvish` generates valid scripts
- [ ] `spn --help` displays colorized output
- [ ] `NO_COLOR=1 spn --help` displays plain output
- [ ] All 830+ tests pass
- [ ] No clippy warnings

### v0.14.0

- [ ] `spn model pull` shows progress bar with ETA
- [ ] `spn mcp add` (no args) shows interactive selection
- [ ] `spn provider set` (no args) shows interactive selection
- [ ] First `spn` invocation shows welcome screen
- [ ] Welcome screen can be skipped permanently

---

## Comparison with Best-in-Class CLIs

| Feature | spn v0.12 | spn v0.13 | spn v0.14 | gh | ripgrep |
|---------|-----------|-----------|-----------|-----|---------|
| Shell completions | No | **Yes** | Yes | Yes | Yes |
| Colorized help | No | **Yes** | Yes | Yes | Yes |
| Progress bars | No | No | **Yes** | Yes | N/A |
| Interactive prompts | No | No | **Yes** | Yes | No |
| First run experience | No | No | **Yes** | Yes | No |
| "Did you mean?" | Yes | Yes | Yes | Yes | No |
| Help topics | Yes | Yes | Yes | Yes | No |

**Target:** Match `gh` CLI quality by v0.14.0
