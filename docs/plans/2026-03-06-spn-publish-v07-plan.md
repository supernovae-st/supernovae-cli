# spn publish Implementation Plan - v0.7.0

**Date:** 2026-03-06
**Status:** In Progress
**Author:** Claude + Thibaut

## Executive Summary

Implement the `spn publish` command to allow publishing packages to the supernovae-registry via Git-based workflow (PR to GitHub).

## Current Status

| Command | Status | Notes |
|---------|--------|-------|
| `spn list` | ✅ Working | Scans state.json + filesystem for manifest.yaml |
| `spn search` | ✅ Working | Searches registry.json from GitHub |
| `spn init --template` | ✅ Working | nika + novanet templates |
| `spn publish` | ⚠️ Partial | Only dry-run validation works |

## Registry Structure

```
supernovae-registry/
├── registry.json              # Full catalog for search
├── index/
│   └── @<scope>/<package>     # NDJSON version metadata
└── releases/
    └── @<scope>/<package>/
        └── <version>.tar.gz   # Package tarballs
```

**Example index file** (`index/@w/dev-productivity/code-review`):
```json
{"name":"@workflows/dev-productivity/code-review","version":"1.0.0","checksum":"sha256:abc...","yanked":false}
```

## Implementation Tasks

### Task 1: Create Tarball (1-2h)

**File:** `crates/spn/src/commands/publish.rs`

```rust
fn create_tarball(dir: &Path, manifest: &SpnManifest) -> Result<(PathBuf, String)> {
    // 1. Create temp file for tarball
    // 2. Include: manifest.yaml, *.yaml, *.md, LICENSE
    // 3. Exclude: .git, .spn, node_modules, target
    // 4. Calculate SHA256 checksum
    // 5. Return (tarball_path, checksum)
}
```

**Files to include:**
- `manifest.yaml` or `spn.yaml` (required)
- `*.nika.yaml`, `*.yaml` (workflows/configs)
- `README.md`, `LICENSE` (documentation)
- `src/**` (if applicable)

**Files to exclude:**
- `.git/`, `.spn/`, `.claude/`
- `node_modules/`, `target/`
- `*.local.yaml`, `.env*`

### Task 2: Git-Based Publishing (2-3h)

**Workflow:**
1. Clone/update supernovae-registry fork
2. Create branch: `publish/<package>/<version>`
3. Add tarball to `releases/@<scope>/<package>/<version>.tar.gz`
4. Update index file `index/@<scope>/<package>`
5. Update `registry.json` (if new package)
6. Commit and push
7. Open PR via `gh pr create`

**Dependencies:**
- `gh` CLI for PR creation
- User must have fork of supernovae-registry

**Config needed:**
```toml
# ~/.spn/config.toml
[publish]
registry_fork = "username/supernovae-registry"
```

### Task 3: Update Index File (1h)

```rust
fn update_index(
    registry_path: &Path,
    name: &str,
    version: &str,
    checksum: &str,
) -> Result<()> {
    // 1. Parse scope: @workflows/dev/foo → @w/dev/foo
    // 2. Create/append to index/@w/dev/foo (NDJSON)
    // 3. Add entry: {"name":"...","version":"...","checksum":"...","yanked":false}
}
```

### Task 4: Update registry.json (1h)

```rust
fn update_registry_json(
    registry_path: &Path,
    manifest: &SpnManifest,
) -> Result<()> {
    // 1. Load registry.json
    // 2. Add/update package entry
    // 3. Write back registry.json
}
```

## Implementation Order

1. **Task 1** - Create tarball function with checksum
2. **Task 2** - Git operations (clone, branch, commit, push)
3. **Task 3** - Index file updates
4. **Task 4** - registry.json updates
5. **Integration** - Wire it all together in `publish_package()`
6. **Testing** - End-to-end test with real package

## CLI Flow

```
$ spn publish

📤 Publishing @workflows/my-workflow@1.0.0...

   ✓ Validating package...
   ✓ Creating tarball (2.3 KB)...
   ✓ Checksum: sha256:abc123...

   📋 Git workflow:
   ✓ Cloning registry fork...
   ✓ Creating branch: publish/@workflows/my-workflow/1.0.0
   ✓ Adding tarball to releases/
   ✓ Updating index file
   ✓ Committing changes
   ✓ Pushing to origin

   🔗 Creating pull request...
   ✓ PR created: https://github.com/supernovae-st/supernovae-registry/pull/42

   📦 Package submitted for review!
   Track status: gh pr view 42
```

## Error Handling

| Error | Message | Action |
|-------|---------|--------|
| No gh CLI | "GitHub CLI not found" | `brew install gh` |
| Not logged in | "Not authenticated" | `gh auth login` |
| No fork | "Fork not found" | Link to fork creation |
| Version exists | "Version already published" | Bump version |
| Invalid package | "Validation failed" | Show errors |

## Testing Strategy

1. **Unit tests:** Tarball creation, checksum, index parsing
2. **Integration test:** Full publish flow with test package
3. **E2E test:** `spn add @workflows/test-pkg` + `nika run`

## Files Modified

```
crates/spn/src/
├── commands/
│   └── publish.rs    # Main implementation
├── index/
│   └── mod.rs        # Add index file helpers
└── Cargo.toml        # Add tar, sha2 deps (already have)
```

## Dependencies (Already Present)

- `tar` - Tarball creation
- `flate2` - Gzip compression
- `sha2` - SHA256 checksums
- `reqwest` - HTTP client (for future direct upload)

## Timeline

| Task | Estimate | Status |
|------|----------|--------|
| Task 1: Tarball | 1-2h | Pending |
| Task 2: Git ops | 2-3h | Pending |
| Task 3: Index | 1h | Pending |
| Task 4: registry.json | 1h | Pending |
| Integration | 1h | Pending |
| Testing | 1h | Pending |
| **Total** | **7-9h** | |

## Success Criteria

- [ ] `spn publish --dry-run` validates and shows what would happen
- [ ] `spn publish` creates tarball with correct contents
- [ ] `spn publish` opens PR to supernovae-registry
- [ ] Published package installable via `spn add`
- [ ] Installed package runnable via `nika run`
