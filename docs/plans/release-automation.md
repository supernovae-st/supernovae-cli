# Release Automation Plan

## Current State

| Step | Status | Manual Work |
|------|--------|-------------|
| Version bump | Manual | Edit 5 Cargo.toml files |
| CHANGELOG | Manual | Write markdown |
| Tag creation | Manual | `git tag vX.X.X` |
| Build/Release | Auto | GitHub Actions |
| crates.io | Auto | `cargo publish` |
| Docker | Auto | ghcr.io push |
| Homebrew | Auto | Formula update |

## Target State

```
git push (conventional commits)
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│  release-plz creates PR:                                  │
│  - Version bump (all Cargo.toml)                          │
│  - CHANGELOG update (from commits)                        │
│  - PR with diff for review                                │
└───────────────────────────────────────────────────────────┘
    │
    ▼
Review & Merge PR
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│  Auto-release triggered:                                  │
│  1. Create git tag                                        │
│  2. Publish to crates.io (dependency order)               │
│  3. Create GitHub Release                                 │
│  4. Build binaries (6 targets)                            │
│  5. Push Docker images                                    │
│  6. Update Homebrew formula                               │
└───────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: release-plz.toml Configuration

**File:** `release-plz.toml`

```toml
[workspace]
changelog_config = "cliff.toml"
git_release_enable = true
pr_labels = ["release", "automated"]
semver_check = true
publish_timeout = "10m"

[[package]]
name = "spn-cli"
git_release_enable = true
changelog_path = "CHANGELOG.md"

[[package]]
name = "spn-core"
git_release_enable = false

[[package]]
name = "spn-keyring"
git_release_enable = false

[[package]]
name = "spn-client"
git_release_enable = false

[[package]]
name = "spn-ollama"
git_release_enable = false
```

### Phase 2: cliff.toml Configuration

**File:** `cliff.toml`

Keep a Changelog format with:
- Conventional commit parsing
- Emoji groups matching existing style
- GitHub PR/issue links
- Breaking change highlighting

### Phase 3: GitHub Workflows

**Files:**
- `.github/workflows/release-plz.yml` - PR creation + crates.io publish
- `.github/workflows/release.yml` - Build/Docker/Homebrew (modified)

Trigger chain:
```
push to main → release-plz PR
merge PR → release-plz release → trigger release.yml
```

### Phase 4: Dynamic Badges

**File:** `README.md`

Replace:
```markdown
![Version](https://img.shields.io/badge/version-0.12.2-blue)
```

With:
```markdown
![Crates.io](https://img.shields.io/crates/v/spn-cli)
![Docker](https://img.shields.io/docker/v/supernovae-st/spn?label=docker)
```

### Phase 5: Pre-release Validation

Add to release-plz.yml:
- cargo test
- cargo clippy
- cargo fmt --check
- cargo semver-checks

### Phase 6: Documentation

Update:
- CLAUDE.md with new release workflow
- README.md with contributor guide
- CONTRIBUTING.md (create)

## Verification Checklist

- [ ] release-plz.toml validates
- [ ] cliff.toml generates correct CHANGELOG
- [ ] PR created automatically on push
- [ ] Version bump correct for all crates
- [ ] crates.io publish order correct
- [ ] GitHub release created with correct tag
- [ ] Docker images pushed
- [ ] Homebrew formula updated
- [ ] Badges show current version

## Rollback Plan

If issues occur:
1. Disable release-plz workflow
2. Revert to manual release.yml
3. Fix configuration
4. Re-enable

## Timeline

| Phase | Estimated |
|-------|-----------|
| Phase 1-2 | Config files |
| Phase 3 | Workflows |
| Phase 4 | Badges |
| Phase 5 | Validation |
| Phase 6 | Docs |
| Testing | Dry-run |

---

**Author:** Claude + Nika
**Status:** Implementation in progress
