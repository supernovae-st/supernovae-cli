# Plan: v0.14.x Release Notes Enhancement

**Created**: 2026-03-08
**Status**: Ready for execution
**Effort**: ~30 minutes

---

## Problem

The v0.14.x release notes (v0.14.0, v0.14.1, v0.14.2, v0.14.3) are minimal and inconsistent with the rich visual style established in v0.10.0+.

### Current Issues

| Version | Issue |
|---------|-------|
| v0.14.0 | Duplicated "Added" sections, mixed formatting |
| v0.14.1 | Only 2 bullets, no highlights |
| v0.14.2 | Only 2 bullets, no highlights |
| v0.14.3 | Better but lacks signature visual banner |

---

## Target Format

Each release should follow the v0.10.0+ template:

```markdown
## [0.14.X] - YYYY-MM-DD

╔═══════════════════════════════════════════════════════════════════════════════╗
║  🎨 SPN v0.14.X — THE DELIGHT RELEASE                                         ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🎨 UX  │  📦 Icons  │  ⚡ Feature  │  🔧 Fix                                 ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🎨 Feature Name** | ✅ New | Description |

### Added / Changed / Fixed / etc.
```

---

## Implementation Steps

### Step 1: Research What Changed

Read git history for each version to understand actual changes:

```bash
git log v0.12.5..v0.14.0 --oneline
git log v0.14.0..v0.14.1 --oneline
git log v0.14.1..v0.14.2 --oneline
git log v0.14.2..v0.14.3 --oneline
```

### Step 2: v0.14.0 — "The Delight Release"

This was the major UX overhaul. Key features to highlight:

- **Semantic Design System**: Unified UX module with theming
- **Phase 2-5 UX Improvements**: Progressive enhancement
- **spn-ollama Enhancements**: `chat_stream()`, retry logic, timeouts
- **Error Handling Fix**: All `exit(1)` replaced with `SpnError::CommandFailed`

**Target:**
```markdown
╔═══════════════════════════════════════════════════════════════════════════════╗
║  🎨 SPN v0.14.0 — THE DELIGHT RELEASE                                         ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🎨 UX  │  🦙 Streaming  │  🔧 Error Handling  │  ⚡ Performance              ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🎨 Semantic Design System** | ✅ New | Unified UX with theming |
| **🦙 Streaming Chat** | ✅ New | `chat_stream()` in spn-ollama |
| **🔧 Error Handling** | ✅ Fixed | All CLI commands use Result types |
| **⏱️ Configurable Timeouts** | ✅ New | ClientConfig with retry logic |
```

### Step 3: v0.14.1

- **Version Alignment**: All crates versioned consistently
- **CLI Fix**: `spn nk config` aligned with nika subcommands
- **crates.io**: Automated publishing enabled

**Target:**
```markdown
### 🔧 Housekeeping Release

Minor release focused on ecosystem alignment.

### Fixed
- **cli**: Align `spn nk config` with nika CLI subcommands

### Changed
- **release**: Enable automated crates.io publishing
- **docs**: Align version references across all crates
```

### Step 4: v0.14.2

- **UX Enhancements**: Human-readable formatters
- **Help Improvements**: Enhanced CLI help text

**Target:**
```markdown
### ✨ UX Enhancements

| Feature | Description |
|---------|-------------|
| **Human-Readable Formatters** | File sizes, durations, counts |
| **Enhanced Help** | Improved `--help` output with examples |
```

### Step 5: v0.14.3

- **Shell Completions**: Install/uninstall/status commands
- **Protocol Versioning**: IPC version negotiation
- **Performance**: FxHashMap, minimal tokio features

**Target:**
```markdown
╔═══════════════════════════════════════════════════════════════════════════════╗
║  ⚡ SPN v0.14.3 — POLISH & PERFORMANCE                                        ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  🐚 Completions  │  📊 Protocol  │  ⚡ FxHashMap  │  📦 Deps                  ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝

### ✨ Highlights

| Feature | Status | Impact |
|---------|--------|--------|
| **🐚 Shell Completions** | ✅ New | install/uninstall/status commands |
| **📊 IPC Protocol** | ✅ Enhanced | Version negotiation for daemon |
| **⚡ FxHashMap** | ✅ Perf | Faster hashing (~20% in hot paths) |
| **📦 Dependencies** | ✅ Updated | reqwest 0.13, indicatif 0.18 |
```

---

## Verification

After editing CHANGELOG.md:

1. Ensure no duplicate sections
2. Check all links at bottom are correct
3. Verify visual alignment (box characters)
4. Run `cargo fmt` to ensure no file issues

---

## Commit

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): enhance v0.14.x release notes with visual style

- Add signature banners to v0.14.0 and v0.14.3
- Fix duplicate 'Added' sections in v0.14.0
- Expand minimal v0.14.1 and v0.14.2 entries
- Add highlights tables with feature impacts

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```
