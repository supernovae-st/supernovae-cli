# UX Design System "Waouh" Plan v0.14.0

**Date**: 2026-03-07
**Status**: In Progress
**Scope**: Full adoption + enhanced interactivity

---

## Current State Analysis

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  BEFORE: 1299 raw style() calls across 33 files                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Files using raw console::style():                                              │
│  ├── commands/provider.rs   167 calls  ← Highest                                │
│  ├── commands/setup.rs      225 calls  ← Complex wizard                         │
│  ├── commands/help.rs       206 calls  ← Help system                            │
│  ├── commands/model.rs       71 calls                                           │
│  ├── commands/secrets.rs     64 calls                                           │
│  ├── secrets/wizard.rs       67 calls                                           │
│  ├── welcome.rs              63 calls                                           │
│  ├── commands/mcp.rs         55 calls                                           │
│  ├── commands/config.rs      38 calls                                           │
│  ├── commands/status.rs      39 calls                                           │
│  └── ... 23 more files                                                          │
│                                                                                 │
│  Design System adoption: 3 files (0.1%)                                         │
│  └── design_system.rs, prompts.rs, doctor.rs                                    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Research Summary

### Sources Consulted
- **Indicatif** (Context7): Progress bar templates, custom spinners, color styling
- **Spectre.Console** (Context7): Best practices, tree widgets, progress columns
- **Dialoguer** (docs.rs): Interactive prompts with themes
- **Perplexity**: 2025 CLI UX patterns, Ratatui trends, async visualization

### Key Insights

1. **Semantic Colors > Raw Colors**: Users understand `success()` better than `green()`
2. **Progress Columns**: Multiple info types in one progress line (ETA, speed, percentage)
3. **Fuzzy Select**: Modern users expect type-to-filter on selections
4. **Tree Widgets**: Hierarchical data (packages, dependencies) deserves tree view
5. **Single Live Widget**: Never overlap spinners with progress bars

---

## Architecture Plan

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  TARGET ARCHITECTURE                                                            │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  src/ux/                                                                        │
│  ├── mod.rs              Re-exports + high-level helpers (spinners, messages)   │
│  ├── design_system.rs    Semantic colors, icons, primitives ← EXISTS            │
│  ├── progress.rs         NEW: Multi-progress, download bars with ETA            │
│  ├── tables.rs           NEW: Formatted tables with headers                     │
│  ├── trees.rs            NEW: Dependency tree visualization                     │
│  └── themes.rs           NEW: Custom dialoguer/indicatif themes                 │
│                                                                                 │
│  All 33 files import from ux::design_system or ux::*                            │
│  Zero raw style() calls outside ux/                                             │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Design System Enhancements

### 1.1 New Semantic Functions

```rust
// Add to design_system.rs

/// Provider with security badge
pub fn provider_secure(name: &str) -> String {
    format!("{} {} {}", icon::LOCK, provider(name), style("(keychain)").dim())
}

/// Provider with warning badge
pub fn provider_insecure(name: &str) -> String {
    format!("{} {} {}", icon::WARNING, provider(name), style("(.env)").yellow())
}

/// Progress step indicator
pub fn step_indicator(current: usize, total: usize) -> String {
    format!("[{}/{}]", style(current).cyan().bold(), style(total).dim())
}

/// Tree branch (for dependency visualization)
pub fn tree_branch(is_last: bool) -> &'static str {
    if is_last { "└── " } else { "├── " }
}

/// Tree connector (vertical line continuation)
pub fn tree_continue(depth: usize) -> String {
    "│   ".repeat(depth)
}
```

### 1.2 New Progress Module (progress.rs)

```rust
//! Enhanced progress bars with multi-column support

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Download progress with speed and ETA
pub fn download_bar(total: u64) -> ProgressBar {
    let style = ProgressStyle::default_bar()
        .template("{prefix:.bold} {bar:30.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}) ETA: {eta}")
        .unwrap()
        .progress_chars("━━╺");

    ProgressBar::new(total).with_style(style)
}

/// Multi-step wizard progress
pub fn wizard_progress(steps: &[&str]) -> MultiProgress {
    let mp = MultiProgress::new();
    // Each step gets its own line
    // [✓] Step 1: Configure API keys
    // [○] Step 2: Install dependencies  ← current
    // [ ] Step 3: Verify installation
    mp
}

/// Spinner that transforms into checkmark on success
pub fn transforming_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["◐", "◓", "◑", "◒", "✓"])  // Last one is "done"
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb
}
```

### 1.3 New Tables Module (tables.rs)

```rust
//! ASCII tables for structured data display

/// Simple table with aligned columns
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    // Calculate column widths
    // Print header with underline
    // Print rows aligned
}

/// Provider status table
pub fn provider_table(providers: &[(String, Option<SecretSource>, String)]) -> String {
    // ┌──────────────┬─────────────┬─────────────────────────┐
    // │ Provider     │ Source      │ Status                  │
    // ├──────────────┼─────────────┼─────────────────────────┤
    // │ anthropic    │ ✓ Keychain  │ Secure                  │
    // │ openai       │ ⚠ .env      │ Migrate recommended     │
    // └──────────────┴─────────────┴─────────────────────────┘
}
```

### 1.4 New Trees Module (trees.rs)

```rust
//! Tree visualization for hierarchical data

/// Package dependency tree
pub fn dependency_tree(root: &str, deps: &[(&str, &str)]) -> String {
    // @nika/workflow v1.2.3
    // ├── @spn/core v0.1.0
    // │   └── serde v1.0.0
    // └── @spn/keyring v0.1.1
}

/// Directory tree (for spn doctor output)
pub fn directory_tree(paths: &[&str]) -> String {
    // ~/.spn/
    // ├── config.toml
    // ├── daemon.sock
    // └── packages/
    //     └── @scope/name/
}
```

---

## Phase 2: Migrate Files to Design System

### Priority Order (by complexity/impact)

| Priority | File | Calls | Complexity | Impact |
|----------|------|-------|------------|--------|
| P0 | commands/provider.rs | 167 | High | Core UX |
| P0 | commands/setup.rs | 225 | Very High | First impression |
| P0 | secrets/wizard.rs | 67 | Medium | Security UX |
| P1 | commands/help.rs | 206 | High | Discovery |
| P1 | commands/model.rs | 71 | Medium | AI features |
| P1 | commands/doctor.rs | 27 | Low | Already partial |
| P2 | commands/mcp.rs | 55 | Medium | MCP features |
| P2 | commands/secrets.rs | 64 | Medium | Security |
| P2 | commands/config.rs | 38 | Low | Settings |
| P3 | All remaining 24 files | ~400 | Low-Medium | Consistency |

### Migration Pattern

```rust
// BEFORE
println!("{} {}", style("✓").green().bold(), style("Done").green());
println!("  {}: {}", style("Provider").bold(), style(name).cyan());

// AFTER
use crate::ux::design_system::*;

println!("{}", success_line("Done"));
println!("  {}", key_value("Provider", provider(name)));
```

---

## Phase 3: Waouh Features

### 3.1 Animated Install Sequence

```
$ spn add @nika/workflow

  ◐ Resolving dependencies...
  ✓ Resolved 3 packages

  @nika/workflow v1.2.3
  ├── @spn/core v0.1.0       ━━━━━━━━━━━━━━━━━━━━ 100% 234KB
  └── @spn/keyring v0.1.1    ━━━━━━━━━━━━━━━━━━━━ 100% 89KB

  ✨ Installed successfully in 1.2s

  What's next?
  $ spn sync         Sync to editor configs
  $ spn list         View installed packages
```

### 3.2 Interactive Provider Setup

```
$ spn provider set

  ┌──────────────────────────────────────────┐
  │  API Key Setup                           │
  ├──────────────────────────────────────────┤
  │                                          │
  │  Select providers to configure:          │
  │                                          │
  │  ◉ anthropic    (Required for Claude)    │
  │  ○ openai       (GPT models)             │
  │  ○ mistral      (Mistral AI)             │
  │  ○ groq         (Fast inference)         │
  │                                          │
  └──────────────────────────────────────────┘

  ▸ anthropic selected

  Enter ANTHROPIC_API_KEY: ••••••••••••••••••••

  ◐ Validating key format...
  ✓ Key format valid (sk-ant-***)

  ◐ Storing in macOS Keychain...
  ✓ Stored securely

  ┌──────────────────────────────────────────┐
  │ ✨ Setup Complete                        │
  ├──────────────────────────────────────────┤
  │ Provider: anthropic                      │
  │ Source:   macOS Keychain                 │
  │ Status:   Ready                          │
  └──────────────────────────────────────────┘
```

### 3.3 Rich Doctor Output

```
$ spn doctor

  ┌──────────────────────────────────────────────────────────────┐
  │  spn doctor v0.14.0                                          │
  └──────────────────────────────────────────────────────────────┘

  System Check
  ├── ✓ Rust 1.85.0
  ├── ✓ Cargo installed
  ├── ✓ Git 2.44.0
  └── ✓ Ollama running (localhost:11434)

  Storage
  ├── ~/.spn/
  │   ├── ✓ config.toml (valid)
  │   ├── ✓ daemon.sock (listening)
  │   └── packages/
  │       ├── @nika/workflow (3 versions)
  │       └── @spn/utils (1 version)
  └── Total: 4 packages, 12.3 MB

  Security Audit
  ┌────────────────┬─────────────┬──────────────────────┐
  │ Provider       │ Source      │ Recommendation       │
  ├────────────────┼─────────────┼──────────────────────┤
  │ anthropic      │ ✓ Keychain  │ Secure               │
  │ openai         │ ⚠ .env      │ Run: spn provider    │
  │                │             │      migrate         │
  │ github         │ ✗ Missing   │ Run: spn provider    │
  │                │             │      set github      │
  └────────────────┴─────────────┴──────────────────────┘

  Overall: ✓ 7/8 checks passed

  $ spn provider migrate   Fix security warnings
```

---

## Phase 4: Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_line_contains_icon() {
        let output = success_line("Test");
        assert!(output.contains(icon::SUCCESS));
    }

    #[test]
    fn test_provider_table_formatting() {
        let data = vec![
            ("anthropic".to_string(), Some(SecretSource::Keychain), "Secure".to_string()),
        ];
        let table = provider_table(&data);
        assert!(table.contains("anthropic"));
        assert!(table.contains("Keychain"));
    }

    #[test]
    fn test_tree_structure() {
        let tree = dependency_tree("root", &[("child1", "1.0"), ("child2", "2.0")]);
        assert!(tree.contains("├──"));
        assert!(tree.contains("└──"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_provider_list_uses_design_system() {
    let output = run_command(&["provider", "list"]);
    // Should use semantic colors, not raw ANSI
    assert!(output.contains("✓") || output.contains("✗") || output.contains("⚠"));
}
```

---

## Implementation Order

### Batch 1: Foundation (This Session)
1. [ ] Enhance design_system.rs with new semantic functions
2. [ ] Create progress.rs module
3. [ ] Create tables.rs module
4. [ ] Create trees.rs module
5. [ ] Create themes.rs for custom dialoguer theme

### Batch 2: Core Migration
6. [ ] Migrate commands/provider.rs (167 calls)
7. [ ] Migrate secrets/wizard.rs (67 calls)
8. [ ] Migrate secrets/keyring.rs (10 calls)

### Batch 3: Setup & Help
9. [ ] Migrate commands/setup.rs (225 calls)
10. [ ] Migrate commands/help.rs (206 calls)

### Batch 4: Commands (P1)
11. [ ] Migrate commands/model.rs (71 calls)
12. [ ] Migrate commands/doctor.rs (27 calls)
13. [ ] Migrate commands/mcp.rs (55 calls)

### Batch 5: Commands (P2-P3)
14. [ ] Migrate remaining 24 files (~400 calls)

### Batch 6: Verification
15. [ ] Run full test suite
16. [ ] Manual UX testing
17. [ ] Commit with changelog

---

## Success Criteria

- [ ] Zero `console::style()` calls outside `src/ux/` modules
- [ ] All 33 files import from `crate::ux::design_system`
- [ ] All tests pass (currently 380)
- [ ] Consistent iconography across all commands
- [ ] Tables render correctly in terminals 80-200 cols wide
- [ ] Trees handle depth up to 10 levels
- [ ] Fuzzy select works for provider selection
- [ ] Progress bars show ETA and speed

---

## Estimated Effort

| Phase | Files | Estimated Time |
|-------|-------|----------------|
| Phase 1: Enhancements | 4 new modules | 30 min |
| Phase 2: Core Migration | 3 files | 45 min |
| Phase 3: Waouh Features | Integrated | 30 min |
| Phase 4: Remaining Migration | 30 files | 60 min |
| Phase 5: Testing | All | 20 min |

**Total**: ~3 hours

---

## Risk Mitigation

1. **Breaking Changes**: Each file migrated independently, test after each
2. **Terminal Compatibility**: Emoji fallbacks already in place
3. **Performance**: No runtime overhead (compile-time styling)
4. **Accessibility**: High contrast colors, no color-only meaning

---

*Plan created: 2026-03-07*
*Author: Claude + Thibaut*
