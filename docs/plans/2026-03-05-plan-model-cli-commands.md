# Plan: Model CLI Commands Implementation

**Date**: 2026-03-05
**Version**: v0.11.0
**Status**: READY TO IMPLEMENT
**Effort**: 4-6 hours
**Author**: Claude + Thibaut

---

## Executive Summary

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  MODEL CLI IMPLEMENTATION STATUS                                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ✅ COMPLETE (Backend)                    ❌ MISSING (CLI)                      │
│  ├── spn-core types (ModelInfo, etc.)     └── spn model list                   │
│  ├── spn-ollama (OllamaBackend)               spn model pull <name>            │
│  ├── spn-client IPC protocol                  spn model load <name>            │
│  ├── daemon/handler.rs (6 handlers)           spn model unload <name>          │
│  └── daemon/model_manager.rs                  spn model delete <name>          │
│                                               spn model status                 │
│  Tests: 270 passing                           spn model info <name>            │
│  Coverage: ~80%                                                                 │
│                                                                                 │
│  DEPENDENCY CHAIN:                                                              │
│  spn-core → spn-ollama → spn-client → daemon → CLI (THIS PLAN)                 │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Current Architecture

### What's Already Implemented

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  LAYER 6: CLI Commands                        ❌ NOT IMPLEMENTED             │
│  crates/spn/src/commands/model.rs                                            │
├──────────────────────────────────────────────────────────────────────────────┤
│  LAYER 5: CLI Routing                         ❌ NOT IMPLEMENTED             │
│  crates/spn/src/main.rs (Commands enum)                                      │
├──────────────────────────────────────────────────────────────────────────────┤
│  LAYER 4: IPC Client                          ✅ COMPLETE                    │
│  crates/spn-client/src/protocol.rs                                           │
│  • Request::ModelList, ModelPull, ModelLoad, ModelUnload, ModelDelete        │
│  • Request::ModelStatus                                                      │
│  • Response::Models, RunningModels, Success, Error                           │
├──────────────────────────────────────────────────────────────────────────────┤
│  LAYER 3: Daemon Handlers                     ✅ COMPLETE                    │
│  crates/spn/src/daemon/handler.rs                                            │
│  • handle_model_list(), handle_model_pull(), handle_model_load()             │
│  • handle_model_unload(), handle_model_delete(), handle_model_status()       │
├──────────────────────────────────────────────────────────────────────────────┤
│  LAYER 2: Model Manager                       ✅ COMPLETE                    │
│  crates/spn/src/daemon/model_manager.rs                                      │
│  • ModelManager wrapping Arc<dyn DynModelBackend>                            │
├──────────────────────────────────────────────────────────────────────────────┤
│  LAYER 1: Backend Implementation              ✅ COMPLETE                    │
│  crates/spn-ollama/src/                                                      │
│  • OllamaBackend with 13 trait methods                                       │
│  • HTTP client with streaming progress                                       │
├──────────────────────────────────────────────────────────────────────────────┤
│  LAYER 0: Shared Types                        ✅ COMPLETE                    │
│  crates/spn-core/src/backend.rs                                              │
│  • ModelInfo, RunningModel, LoadConfig, PullProgress, BackendError           │
└──────────────────────────────────────────────────────────────────────────────┘
```

### IPC Protocol (Already Defined)

```rust
// crates/spn-client/src/protocol.rs

pub enum Request {
    // Model commands (ready to use)
    ModelList,                                      // → Response::Models
    ModelPull { name: String },                     // → Response::Success
    ModelLoad { name: String, config: Option<LoadConfig> }, // → Response::Success
    ModelUnload { name: String },                   // → Response::Success
    ModelDelete { name: String },                   // → Response::Success
    ModelStatus,                                    // → Response::RunningModels
}

pub enum Response {
    Models { models: Vec<ModelInfo> },
    RunningModels { running: Vec<RunningModel> },
    Success { success: bool },
    Error { message: String },
}
```

---

## 2. Implementation Checklist

### Phase 1: CLI Scaffolding (30 min)

- [ ] **Task 1.1**: Add `ModelCommands` enum to `main.rs`
  - Location: `crates/spn/src/main.rs` after line 251
  - Pattern: Follow `McpCommands` structure

- [ ] **Task 1.2**: Add `Model` variant to `Commands` enum
  - Location: `crates/spn/src/main.rs` around line 118

- [ ] **Task 1.3**: Add handler in `main()` match
  - Location: `crates/spn/src/main.rs` around line 656

- [ ] **Task 1.4**: Create `commands/model.rs` skeleton
  - Location: `crates/spn/src/commands/model.rs`

- [ ] **Task 1.5**: Add module declaration
  - Location: `crates/spn/src/commands/mod.rs`

### Phase 2: Core Commands (2 hours)

- [ ] **Task 2.1**: Implement `spn model list`
  - Connect to daemon via `SpnClient`
  - Send `Request::ModelList`
  - Format output (text + JSON modes)
  - Handle daemon not running

- [ ] **Task 2.2**: Implement `spn model status`
  - Send `Request::ModelStatus`
  - Show loaded models with VRAM usage
  - Indicate which models are "hot"

- [ ] **Task 2.3**: Implement `spn model pull <name>`
  - Send `Request::ModelPull`
  - Show progress indicator (spinner for now)
  - Note: Real progress requires protocol extension

- [ ] **Task 2.4**: Implement `spn model load <name>`
  - Send `Request::ModelLoad`
  - Support `--keep-alive` flag
  - Show success/failure message

- [ ] **Task 2.5**: Implement `spn model unload <name>`
  - Send `Request::ModelUnload`
  - Confirm unload completed

- [ ] **Task 2.6**: Implement `spn model delete <name>`
  - Send `Request::ModelDelete`
  - Add confirmation prompt (unless `--yes`)

### Phase 3: Enhanced Features (1 hour)

- [ ] **Task 3.1**: Add `spn model info <name>`
  - New IPC request needed: `Request::ModelInfo { name }`
  - Add handler in daemon
  - Show detailed model metadata

- [ ] **Task 3.2**: Add Ollama auto-start
  - If daemon running but Ollama not → offer to start
  - `spn model list --start-ollama`

- [ ] **Task 3.3**: Add table formatting
  - Use aligned columns for list output
  - Human-readable sizes (GB)
  - Color coding for status

### Phase 4: Tests & Documentation (1 hour)

- [ ] **Task 4.1**: Unit tests for command parsing
- [ ] **Task 4.2**: Integration tests (with mock daemon)
- [ ] **Task 4.3**: Update CLI help text in main.rs
- [ ] **Task 4.4**: Update README.md with model commands
- [ ] **Task 4.5**: Update CLAUDE.md architecture diagram

---

## 3. Detailed Implementation

### 3.1 main.rs Additions

```rust
// After line 251 (after ProviderCommands)

/// Local model management commands
#[derive(Subcommand)]
pub enum ModelCommands {
    /// List installed models
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Only show loaded models
        #[arg(long)]
        running: bool,
    },

    /// Pull/download a model from Ollama registry
    Pull {
        /// Model name (e.g., llama3.2:7b, mistral:latest)
        name: String,
    },

    /// Load a model into memory
    Load {
        /// Model name
        name: String,

        /// Keep model loaded indefinitely
        #[arg(long)]
        keep_alive: bool,
    },

    /// Unload a model from memory
    Unload {
        /// Model name
        name: String,
    },

    /// Delete a model
    Delete {
        /// Model name
        name: String,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// Show running models and resource usage
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

// In Commands enum, add:
/// Manage local LLM models (Ollama)
Model {
    #[command(subcommand)]
    command: ModelCommands,
},

// In main() match, add:
Commands::Model { command } => commands::model::run(command).await,
```

### 3.2 commands/model.rs

```rust
//! Model CLI commands.
//!
//! Manage local LLM models via the spn daemon + Ollama.

use crate::error::Result;
use crate::ModelCommands;
use colored::Colorize;
use dialoguer::Confirm;
use spn_client::{Request, Response, SpnClient};

pub async fn run(command: ModelCommands) -> Result<()> {
    match command {
        ModelCommands::List { json, running } => list(json, running).await,
        ModelCommands::Pull { name } => pull(&name).await,
        ModelCommands::Load { name, keep_alive } => load(&name, keep_alive).await,
        ModelCommands::Unload { name } => unload(&name).await,
        ModelCommands::Delete { name, yes } => delete(&name, yes).await,
        ModelCommands::Status { json } => status(json).await,
    }
}

// ============================================================================
// List Models
// ============================================================================

async fn list(json: bool, running_only: bool) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    let request = if running_only {
        Request::ModelStatus
    } else {
        Request::ModelList
    };

    let response = client.send_request(request).await?;

    match response {
        Response::Models { models } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&models)?);
                return Ok(());
            }

            if models.is_empty() {
                println!("{}", "No models installed.".yellow());
                println!();
                println!("Get started:");
                println!("  {} spn model pull llama3.2", "•".cyan());
                println!("  {} spn model pull mistral:7b", "•".cyan());
                return Ok(());
            }

            println!("{}", "🤖 Installed Models".bold());
            println!();

            // Header
            println!(
                "  {:<30} {:>10} {:>10}",
                "NAME".dimmed(),
                "SIZE".dimmed(),
                "QUANT".dimmed()
            );
            println!("  {}", "─".repeat(52));

            // Models
            for model in &models {
                let size = format_size(model.size);
                let quant = model.quantization.as_deref().unwrap_or("-");
                println!("  {:<30} {:>10} {:>10}", model.name, size, quant);
            }

            println!();
            println!("  {} model(s) installed", models.len());
        }

        Response::RunningModels { running } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&running)?);
                return Ok(());
            }

            if running.is_empty() {
                println!("{}", "No models currently loaded.".yellow());
                return Ok(());
            }

            println!("{}", "🔥 Running Models".bold());
            println!();

            for model in &running {
                let vram = model
                    .vram_used
                    .map(|v| format!("{:.1} GB VRAM", v as f64 / 1_073_741_824.0))
                    .unwrap_or_else(|| "-".to_string());

                println!("  {} {} ({})", "✓".green(), model.name, vram);
            }
        }

        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }

        _ => {
            eprintln!("{}", "Unexpected response from daemon".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Pull Model
// ============================================================================

async fn pull(name: &str) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    println!("{} Pulling model: {}", "📥".cyan(), name.bold());
    println!("   This may take a while...");

    let response = client.send_request(Request::ModelPull {
        name: name.to_string()
    }).await?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' pulled successfully", "✓".green(), name);
        }
        Response::Success { success: false } => {
            eprintln!("{} Pull failed", "✗".red());
            std::process::exit(1);
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Load Model
// ============================================================================

async fn load(name: &str, keep_alive: bool) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    println!("{} Loading model: {}", "🚀".cyan(), name.bold());

    let config = if keep_alive {
        Some(spn_client::LoadConfig {
            gpu_ids: vec![],
            gpu_layers: -1,
            context_size: None,
            keep_alive: true,
        })
    } else {
        None
    };

    let response = client.send_request(Request::ModelLoad {
        name: name.to_string(),
        config,
    }).await?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' loaded", "✓".green(), name);
            if keep_alive {
                println!("   Model will stay loaded until manually unloaded");
            }
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Unload Model
// ============================================================================

async fn unload(name: &str) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    println!("{} Unloading model: {}", "💤".cyan(), name.bold());

    let response = client.send_request(Request::ModelUnload {
        name: name.to_string()
    }).await?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' unloaded", "✓".green(), name);
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Delete Model
// ============================================================================

async fn delete(name: &str, skip_confirm: bool) -> Result<()> {
    if !skip_confirm {
        let confirm = Confirm::new()
            .with_prompt(format!("Delete model '{}'?", name))
            .default(false)
            .interact()?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let mut client = connect_to_daemon().await?;

    println!("{} Deleting model: {}", "🗑️".cyan(), name.bold());

    let response = client.send_request(Request::ModelDelete {
        name: name.to_string()
    }).await?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' deleted", "✓".green(), name);
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Status
// ============================================================================

async fn status(json: bool) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    let response = client.send_request(Request::ModelStatus).await?;

    match response {
        Response::RunningModels { running } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&running)?);
                return Ok(());
            }

            println!("{}", "🤖 Model Status".bold());
            println!();

            if running.is_empty() {
                println!("  {} No models loaded", "○".dimmed());
                println!();
                println!("  Load a model with: {} spn model load llama3.2", "→".cyan());
            } else {
                println!(
                    "  {:<30} {:>12}",
                    "MODEL".dimmed(),
                    "VRAM".dimmed()
                );
                println!("  {}", "─".repeat(44));

                let mut total_vram: u64 = 0;

                for model in &running {
                    let vram = model.vram_used.unwrap_or(0);
                    total_vram += vram;

                    let vram_str = if vram > 0 {
                        format!("{:.1} GB", vram as f64 / 1_073_741_824.0)
                    } else {
                        "-".to_string()
                    };

                    println!(
                        "  {} {:<28} {:>12}",
                        "●".green(),
                        model.name,
                        vram_str
                    );
                }

                if total_vram > 0 {
                    println!("  {}", "─".repeat(44));
                    println!(
                        "  {:<30} {:>12}",
                        "Total VRAM",
                        format!("{:.1} GB", total_vram as f64 / 1_073_741_824.0)
                    );
                }
            }
        }

        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }

        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

async fn connect_to_daemon() -> Result<SpnClient> {
    match SpnClient::connect().await {
        Ok(client) => Ok(client),
        Err(_) => {
            eprintln!("{} Daemon is not running", "✗".red());
            eprintln!();
            eprintln!("Start the daemon with: {} spn daemon start", "→".cyan());
            std::process::exit(1);
        }
    }
}

fn format_size(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
        assert_eq!(format_size(4_500_000_000), "4.2 GB");
        assert_eq!(format_size(500_000_000), "477 MB");
        assert_eq!(format_size(1000), "1000 B");
    }
}
```

### 3.3 commands/mod.rs Addition

```rust
pub mod model;
```

---

## 4. Output Examples

### `spn model list`

```
🤖 Installed Models

  NAME                           SIZE      QUANT
  ────────────────────────────────────────────────
  llama3.2:7b                    4.1 GB    Q4_K_M
  mistral:7b                     4.1 GB    Q4_K_M
  qwen2.5:14b                    8.5 GB    Q4_K_M
  deepseek-coder:6.7b            3.8 GB    Q4_0

  4 model(s) installed
```

### `spn model list --json`

```json
[
  {
    "name": "llama3.2:7b",
    "size": 4400000000,
    "quantization": "Q4_K_M",
    "parameters": "7B",
    "digest": "sha256:abc123..."
  }
]
```

### `spn model status`

```
🤖 Model Status

  MODEL                          VRAM
  ────────────────────────────────────────────
  ● llama3.2:7b                    4.0 GB
  ● mistral:7b                     3.8 GB
  ────────────────────────────────────────────
  Total VRAM                       7.8 GB
```

### `spn model pull llama3.2`

```
📥 Pulling model: llama3.2
   This may take a while...
✓ Model 'llama3.2' pulled successfully
```

### Error: Daemon not running

```
✗ Daemon is not running

→ Start the daemon with: spn daemon start
```

---

## 5. Consistency with Existing Commands

| Pattern | mcp list | skill list | provider list | model list |
|---------|----------|------------|---------------|------------|
| Emoji header | ✓ | ✓ | ✓ | ✓ |
| Status icons | ✓ ○ | • | 🔐 📦 ○ | ● ○ |
| JSON output | --json | ❌ | --json | --json |
| Legend | ✓ | ❌ | ✓ | ✓ |
| Empty state | ✓ | ✓ | ✓ | ✓ |
| Data source | YAML | Filesystem | Keychain | Daemon IPC |

---

## 6. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
        assert_eq!(format_size(4_500_000_000), "4.2 GB");
    }

    #[tokio::test]
    async fn test_list_models_daemon_not_running() {
        // Should exit gracefully with helpful message
    }
}
```

### Integration Tests (Requires Daemon)

```bash
# Start daemon
spn daemon start

# Test commands
spn model list
spn model list --json
spn model status
spn model pull tinyllama  # Small model for testing
spn model load tinyllama
spn model status
spn model unload tinyllama
spn model delete tinyllama --yes

# Stop daemon
spn daemon stop
```

---

## 7. Future Enhancements (Post v0.11.0)

### v0.12.0: Progress Reporting

- Extend IPC protocol for streaming progress
- Add progress bars for pull operations
- Show ETA for downloads

### v0.13.0: Multi-Backend Support

- Add `--backend` flag (ollama, llamacpp)
- Abstract backend selection
- Per-model backend configuration

### v0.14.0: Model Profiles

- `spn model profile save <name>` - Save LoadConfig preset
- `spn model profile list` - Show saved profiles
- `spn model load <model> --profile <name>`

---

## 8. Validation Checklist

Before marking complete:

- [ ] All 6 commands work end-to-end
- [ ] JSON output is valid and parseable
- [ ] Error messages are helpful
- [ ] Daemon-not-running case handled gracefully
- [ ] `--help` shows clear descriptions
- [ ] Tests pass (cargo test)
- [ ] Clippy clean (cargo clippy)
- [ ] Format clean (cargo fmt --check)
- [ ] CLAUDE.md updated with commands
- [ ] README.md updated if needed

---

## 9. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Daemon not running | Users confused | Clear error message + instructions |
| Ollama not installed | Commands fail | Detect and suggest installation |
| Large model pull timeout | Bad UX | Show "this may take a while" |
| Model in use (delete) | Data loss | Confirmation prompt |
| Network issues (pull) | Partial download | Ollama handles resumption |

---

## 10. Dependencies

### Internal
- `spn-client` v0.2.0 (IPC protocol)
- `spn-core` v0.1.0 (types)
- `spn-ollama` v0.1.0 (backend)

### External
- `colored` (terminal colors)
- `dialoguer` (confirmation prompts)
- `serde_json` (JSON output)

### Runtime
- spn daemon running
- Ollama server running (or auto-started)

---

## Summary

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  IMPLEMENTATION SUMMARY                                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Files to Create:                                                               │
│  └── crates/spn/src/commands/model.rs (~250 LOC)                               │
│                                                                                 │
│  Files to Modify:                                                               │
│  ├── crates/spn/src/main.rs (+50 LOC)                                          │
│  └── crates/spn/src/commands/mod.rs (+1 LOC)                                   │
│                                                                                 │
│  Commands Added:                                                                │
│  ├── spn model list [--json] [--running]                                       │
│  ├── spn model pull <name>                                                     │
│  ├── spn model load <name> [--keep-alive]                                      │
│  ├── spn model unload <name>                                                   │
│  ├── spn model delete <name> [--yes]                                           │
│  └── spn model status [--json]                                                 │
│                                                                                 │
│  Estimated Effort: 4-6 hours                                                    │
│  Risk Level: LOW (all backend code complete)                                    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

**Ready to implement. All infrastructure is in place.**
