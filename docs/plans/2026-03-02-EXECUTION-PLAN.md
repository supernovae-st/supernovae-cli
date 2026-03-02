# SuperNovae CLI v0.7.0 — Execution Plan

**Date:** 2026-03-02
**Status:** 🎯 Ready to Execute (Validated)
**Target:** 4 weeks (28 days)
**Total Estimate:** ~750 LOC + 400 LOC tests = 1,150 LOC

---

## ✅ Validated Decisions (2026-03-02)

**From VISION-SUMMARY.md questions:**

1. **Storage Architecture:** **Option B** (Direct Lookup with caching)
   - ✅ No symlinks in `.nika/.cache/`
   - ✅ Direct lookup from `~/.spn/packages/`
   - ✅ DashMap cache for 99% faster subsequent lookups
   - ✅ Cross-platform compatible (no Windows symlink issues)

2. **CLI Interactive:** **Now (Week 4)** ✅
   - Interactive `spn add` with dialoguer
   - Interactive `nika init` with templates
   - Better UX for discovery

3. **@prompts Format:** **Markdown with frontmatter** ✅
   - Same format as agents
   - YAML frontmatter for metadata
   - Markdown body for prompt template

4. **Include Support:** **Now (Week 2-3)** ✅
   - `include: { pkg: @workflows/name }` in v0.7.0
   - Enables workflow composition from packages

---

## 🎯 Three-Sentence Vision

**SuperNovae = npm for AI. Nika = Node.js for AI.**

- Users discover AI workflows/agents/prompts in a registry
- Install with `spn add @workflows/name`
- Run with `nika run @workflows/name` in 30 seconds

---

## 📊 Progress Dashboard

```
┌─────────────────────────────────────────────────────────────┐
│  WEEK 1: Bug Fixes + Foundation         [░░░░░░░░░░] 0%    │
│  WEEK 2: Package Resolution              [░░░░░░░░░░] 0%    │
│  WEEK 3: Includes + Sync                 [░░░░░░░░░░] 0%    │
│  WEEK 4: Interactive CLI + Polish        [░░░░░░░░░░] 0%    │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔴 Week 1: Bug Fixes + Foundation (Days 1-7)

**Goal:** Zero panics, valid examples, clean foundation

### Day 1-2: Fix `spn add` Tokio Panic 🔴 P0

**Problem:**
```
thread 'main' panicked at tokio-1.49.0/src/runtime/blocking/shutdown.rs:51:21:
Cannot drop a runtime in a context where blocking is not allowed.
```

**Root Cause:** `IndexClient::new()` creates tokio runtime that gets dropped in blocking context

**Tasks:**

1. **Diagnostic** (30 min)
   - [ ] Run `spn add @workflows/test` with `RUST_BACKTRACE=full`
   - [ ] Identify exact line where runtime is created/dropped
   - [ ] Document call stack in issue tracker

2. **Refactor `add.rs`** (2 hours)
   - [ ] File: `supernovae-cli/src/commands/add.rs`
   - [ ] Change signature: `pub async fn run(...)` stays async
   - [ ] Wrap blocking code in `tokio::task::spawn_blocking()`
   - [ ] OR: Create runtime explicitly with `Runtime::new()?`
   - [ ] Test: `cargo run -- add @workflows/test` (should NOT panic)

3. **Add Integration Test** (1 hour)
   - [ ] File: `supernovae-cli/tests/integration/add_test.rs`
   - [ ] Test case: Add package, verify no panic, verify installed
   - [ ] Run: `cargo test test_add_workflow`
   - [ ] Coverage: Must pass on Linux/macOS/Windows

**Acceptance Criteria:**
- ✅ `spn add @workflows/name` completes without panic
- ✅ Package downloaded to `~/.spn/packages/`
- ✅ `spn.yaml` updated with dependency
- ✅ `spn.lock` created/updated
- ✅ Integration test passes

**Estimate:** 3.5 hours
**Files Modified:** 1 (`add.rs`)
**LOC:** ~50 lines
**Tests Added:** 1 integration test (~80 LOC)

---

### Day 2-3: Fix `nika init` Invalid Examples 🔴 P0

**Problem:**
```
× [NIKA-005] Schema validation failed: 3 errors
  [/tasks/0/output] Additional properties not allowed ('use.summary' was unexpected)
```

**Root Cause:** Hardcoded workflow templates use deprecated `output.use.*` syntax

**Tasks:**

1. **Audit All Templates** (1 hour)
   - [ ] File: `nika/tools/nika/src/main.rs` (lines 1312-1900)
   - [ ] Find all hardcoded YAML strings (search `"schema: nika/workflow@0.9"`)
   - [ ] Count: 4 templates (content-generation, code-automation, research, empty)
   - [ ] Document invalid syntax patterns

2. **Update Content Generation Template** (30 min)
   - [ ] Fix `output.use.*` → separate `use:` task
   - [ ] Fix `output.result` → use `id` + `use:` reference
   - [ ] Validate against schema: `cargo run -- check workflow-content-gen.nika.yaml`

3. **Update Code Automation Template** (30 min)
   - [ ] Same fixes as content-gen
   - [ ] Test: `nika init --template code-automation && nika check example.nika.yaml`

4. **Update Research Template** (30 min)
   - [ ] Fix agent syntax if needed
   - [ ] Test: `nika init --template research && nika check research.nika.yaml`

5. **Update Empty Template** (15 min)
   - [ ] Verify minimal valid workflow
   - [ ] Should be 5-10 lines max

6. **Add Template Validation Test** (1 hour)
   - [ ] File: `nika/tools/nika/tests/init_templates_test.rs`
   - [ ] For each template: init → check → run dry-run
   - [ ] Must pass schema validation

**Acceptance Criteria:**
- ✅ `nika init` generates valid workflows (all templates)
- ✅ `nika check` passes on all generated files
- ✅ No schema validation errors
- ✅ Tests cover all 4 templates

**Estimate:** 3.5 hours
**Files Modified:** 1 (`main.rs` in nika)
**LOC:** ~100 lines (template strings)
**Tests Added:** 1 test file (~120 LOC)

---

### Day 3-4: Add `resolve_package_path()` Foundation 🟡 P1

**Goal:** Create the core package resolution function that will be used everywhere

**Tasks:**

1. **Create Resolver Module** (2 hours)
   - [ ] New file: `nika/tools/nika/src/registry/resolver.rs`
   - [ ] Add to `mod.rs`: `pub mod resolver;`
   - [ ] Define types:
     ```rust
     pub struct PackageRef {
         pub scope: String,        // "@workflows"
         pub name: String,          // "seo-audit"
         pub version: Option<String>, // "1.2.0" or None
     }

     pub struct ResolvedPackage {
         pub path: PathBuf,         // Full path to package dir
         pub manifest: PackageManifest,
         pub version: String,
     }
     ```

2. **Implement `parse_package_ref()`** (1 hour)
   - [ ] Parse `@workflows/seo-audit` → `PackageRef`
   - [ ] Parse `@workflows/seo-audit@1.2.0` → `PackageRef` with version
   - [ ] Handle errors: invalid format, missing scope
   - [ ] Unit tests: 10 test cases (valid/invalid inputs)

3. **Implement `resolve_package_path()`** (2 hours)
   - [ ] Algorithm:
     ```rust
     1. Parse package reference
     2. If no version specified:
        - Check spn.lock for pinned version
        - OR use latest installed from ~/.spn/packages/
     3. Build path: ~/.spn/packages/{scope}/{name}/{version}/
     4. Verify package exists
     5. Return ResolvedPackage
     ```
   - [ ] Handle errors: package not found, invalid manifest
   - [ ] Unit tests: 8 test cases

4. **Add Lockfile Integration** (1.5 hours)
   - [ ] New file: `nika/tools/nika/src/registry/lockfile.rs`
   - [ ] Function: `read_lockfile() -> Option<SpnLockfile>`
   - [ ] Function: `get_locked_version(name: &str) -> Option<String>`
   - [ ] Read `spn.lock` from current directory
   - [ ] Parse YAML, extract version for given package
   - [ ] Unit tests: 5 test cases

5. **Integration Test** (1 hour)
   - [ ] File: `nika/tools/nika/tests/resolver_test.rs`
   - [ ] Setup: Create fake `~/.spn/packages/` structure
   - [ ] Test: Resolve `@workflows/test` → correct path
   - [ ] Test: Resolve with version → correct path
   - [ ] Test: Resolve with lockfile → uses locked version
   - [ ] Test: Package not found → error

**Acceptance Criteria:**
- ✅ `resolve_package_path("@workflows/test")` returns correct path
- ✅ Respects `spn.lock` if present
- ✅ Falls back to latest installed if no lock
- ✅ Errors gracefully on missing packages
- ✅ All unit tests pass (23 tests)

**Estimate:** 7.5 hours (2 days)
**Files Created:** 2 (`resolver.rs`, `lockfile.rs`)
**LOC:** ~200 lines
**Tests Added:** 23 unit tests + 1 integration (~180 LOC)

---

### Day 5-7: Weekend Buffer + Documentation

**Tasks:**

1. **Write Developer Guide** (2 hours)
   - [ ] File: `docs/development/PACKAGE_RESOLUTION.md`
   - [ ] Explain resolver architecture
   - [ ] Code examples for extending resolver
   - [ ] Testing guidelines

2. **Update CHANGELOG** (30 min)
   - [ ] Add v0.7.0-alpha.1 entry
   - [ ] List bug fixes
   - [ ] Document new resolver module

3. **Code Review Checkpoint** (1 hour)
   - [ ] Review all Week 1 changes
   - [ ] Run: `cargo fmt && cargo clippy`
   - [ ] Run: `cargo test --all`
   - [ ] Verify 100% pass rate

**Deliverables:**
- ✅ Zero panics in `spn add`
- ✅ Valid templates in `nika init`
- ✅ Core resolver foundation ready
- ✅ All tests passing (35+ new tests)
- ✅ Documentation updated

---

## 🟡 Week 2: Package Resolution (Days 8-14)

**Goal:** `nika run @workflows/name` resolves and executes packages

### Day 8-9: Integrate Resolver into `nika run`

**Tasks:**

1. **Modify `run_workflow()` Function** (2 hours)
   - [ ] File: `nika/tools/nika/src/main.rs` (line ~727)
   - [ ] Add resolution logic before file read:
     ```rust
     let resolved_path = if file.starts_with('@') {
         resolver::resolve_package_path(file).await?.path
     } else if !file.ends_with(".nika.yaml") && !file.contains('/') {
         // Try .nika/workflows/ first
         let local = Path::new(".nika/workflows").join(format!("{}.nika.yaml", file));
         if local.exists() { local } else { PathBuf::from(file) }
     } else {
         PathBuf::from(file)
     };
     ```
   - [ ] Test: `nika run @workflows/test`

2. **Add Search Priority** (1 hour)
   - [ ] Search order: `.nika/workflows/` → `~/.spn/packages/` → filesystem
   - [ ] Log which location was used (debug output)
   - [ ] Test: Local file overrides package

3. **Add Caching** (2 hours)
   - [ ] Use `DashMap<String, ResolvedPackage>` for in-memory cache
   - [ ] Cache TTL: 60 seconds (configurable)
   - [ ] Invalidate on `spn install/update`
   - [ ] Benchmark: cached lookup < 100ns

4. **Error Messages** (1 hour)
   - [ ] Package not found: "Package @workflows/name not installed. Run: spn add @workflows/name"
   - [ ] Workflow file missing: "Package exists but missing workflow.nika.yaml"
   - [ ] Version conflict: "Locked version X but installed Y. Run: spn install"

**Acceptance Criteria:**
- ✅ `nika run @workflows/test` resolves from `~/.spn/packages/`
- ✅ Local files take precedence over packages
- ✅ Clear error messages for missing packages
- ✅ Cache reduces lookup time by 99%

**Estimate:** 6 hours
**Files Modified:** 1 (`main.rs` in nika)
**LOC:** ~80 lines
**Tests Added:** 6 integration tests (~150 LOC)

---

### Day 10-11: Support `@agents/` Resolution

**Tasks:**

1. **Add Agent Resolution** (2 hours)
   - [ ] File: `nika/tools/nika/src/registry/resolver.rs`
   - [ ] Function: `resolve_agent(ref: &str) -> Result<PathBuf>`
   - [ ] Look for `agent.md` in package
   - [ ] Same resolution logic as workflows

2. **Integrate into `agent:` Verb** (2 hours)
   - [ ] File: `nika/tools/nika/src/ast/task.rs` (agent handling)
   - [ ] Parse `pkg: @agents/researcher`
   - [ ] Resolve to file path
   - [ ] Load agent markdown
   - [ ] Test: Run workflow with agent package

3. **Add Agent Tests** (1.5 hours)
   - [ ] Test: `agent: { pkg: @agents/test }` resolves
   - [ ] Test: Agent not found → error
   - [ ] Test: Invalid agent.md → parse error

**Acceptance Criteria:**
- ✅ Workflows can use `agent: { pkg: @agents/name }`
- ✅ Agent packages resolve correctly
- ✅ Error handling for missing agents

**Estimate:** 5.5 hours
**LOC:** ~60 lines
**Tests Added:** 3 tests (~90 LOC)

---

### Day 12-13: Support `@prompts/` and `@jobs/`

**Tasks:**

1. **Add Prompt Resolution** (1.5 hours)
   - [ ] Function: `resolve_prompt(ref: &str) -> Result<PathBuf>`
   - [ ] Look for `prompt.md` in package
   - [ ] Parse frontmatter (variables)
   - [ ] Template substitution support

2. **Add Job Resolution** (1 hour)
   - [ ] Function: `resolve_job(ref: &str) -> Result<PathBuf>`
   - [ ] Look for `job.nika.yaml` in package
   - [ ] Same resolution as workflows

3. **Integration Tests** (1.5 hours)
   - [ ] Test prompts: `@prompts/seo-meta`
   - [ ] Test jobs: `@jobs/daily-report`
   - [ ] Test all 4 types together in one workflow

**Acceptance Criteria:**
- ✅ All 4 package types resolve: workflows, agents, prompts, jobs
- ✅ Resolution logic unified (DRY)
- ✅ Comprehensive test coverage

**Estimate:** 4 hours
**LOC:** ~40 lines
**Tests Added:** 5 tests (~120 LOC)

---

### Day 14: Integration Testing + Documentation

**Tasks:**

1. **End-to-End Test** (2 hours)
   - [ ] Create test workflow that uses all 4 types:
     ```yaml
     tasks:
       - id: research
         agent: { pkg: @agents/test }

       - id: content
         include: { pkg: @workflows/test-content }

       - id: generate
         infer: "@prompts/test-template with {{research}}"

       - id: schedule
         invoke: @jobs/test-job
     ```
   - [ ] Verify all packages resolve
   - [ ] Verify execution succeeds

2. **Performance Benchmark** (1 hour)
   - [ ] Measure cold lookup time (first run)
   - [ ] Measure cached lookup time
   - [ ] Target: cold < 5ms, cached < 100ns
   - [ ] Document in `PERFORMANCE.md`

3. **Update Documentation** (1 hour)
   - [ ] File: `docs/guides/USING_PACKAGES.md`
   - [ ] Examples for each package type
   - [ ] Resolution algorithm explained
   - [ ] Troubleshooting section

**Deliverables:**
- ✅ `nika run @workflows/name` works
- ✅ `nika run @agents/name` works
- ✅ `@prompts/` and `@jobs/` resolve
- ✅ All integration tests pass
- ✅ Performance targets met

---

## 🟢 Week 3: Includes + Sync (Days 15-21)

**Goal:** Workflow composition from packages + local caching

### Day 15-16: Add `pkg:` Support to Includes

**Tasks:**

1. **Modify `IncludeSpec` Enum** (1 hour)
   - [ ] File: `nika/tools/nika/src/ast/include.rs`
   - [ ] Add variant:
     ```rust
     Package {
         pkg: String,
         prefix: Option<String>,
     }
     ```
   - [ ] Update serde deserializer

2. **Modify `expand_includes_recursive()`** (2 hours)
   - [ ] File: `nika/tools/nika/src/ast/include_loader.rs`
   - [ ] Handle `IncludeSpec::Package`:
     ```rust
     match include_spec {
         Path { path, .. } => base_path.join(path),
         Package { pkg, .. } => resolve_package_path(pkg).await?.path,
     }
     ```
   - [ ] Test: Include workflow from package

3. **Add Prefix Support** (1 hour)
   - [ ] Prefix all task IDs from included workflow
   - [ ] Test: `prefix: seo_` → task `audit` becomes `seo_audit`

4. **Circular Dependency Detection** (1.5 hours)
   - [ ] Detect `A includes B includes A`
   - [ ] Max include depth: 10 levels
   - [ ] Error: "Circular include detected: A → B → A"

5. **Integration Tests** (1.5 hours)
   - [ ] Test: Include from local path
   - [ ] Test: Include from package
   - [ ] Test: Mixed local + package includes
   - [ ] Test: Circular include error

**Acceptance Criteria:**
- ✅ `include: { pkg: @workflows/name }` works
- ✅ Prefix correctly applied
- ✅ Circular dependencies detected
- ✅ 5 integration tests pass

**Estimate:** 7 hours (2 days)
**LOC:** ~30 lines
**Tests Added:** 5 tests (~130 LOC)

---

### Day 17-18: Cache Optimization (Direct Lookup Strategy)

**Decision:** Using **Option B** (direct lookup) instead of symlinks for simplicity and cross-platform compatibility.

**Tasks:**

1. **Implement DashMap Cache** (3 hours)
   - [ ] File: `nika/tools/nika/src/registry/cache.rs`
   - [ ] Use `DashMap<String, ResolvedPackage>` for thread-safe cache
   - [ ] Cache key: `@scope/name@version`
   - [ ] Cache entry includes: path, manifest, timestamp
   - [ ] TTL: 60 seconds (configurable)

2. **Cache Invalidation Strategy** (2 hours)
   - [ ] Invalidate on `spn install/update/remove`
   - [ ] Invalidate on file system changes (watch ~/.spn/packages/)
   - [ ] Automatic expiry after TTL
   - [ ] Manual clear: `nika cache clear`

3. **Performance Optimization** (2 hours)
   - [ ] Benchmark: cold lookup (first time)
   - [ ] Benchmark: cached lookup (subsequent)
   - [ ] Target: cold < 5ms, cached < 100ns
   - [ ] Use `xxHash` for fast hashing

4. **Tests** (1 hour)
   - [ ] Test: Cold lookup → populates cache
   - [ ] Test: Cached lookup → fast retrieval
   - [ ] Test: Cache expiry → re-lookup
   - [ ] Test: Cache invalidation on install

**Acceptance Criteria:**
- ✅ Direct lookup from `~/.spn/packages/` works
- ✅ Cache reduces lookup time by 99%
- ✅ No Windows compatibility issues (no symlinks)
- ✅ Automatic cache management

**Estimate:** 8 hours (2 days)
**LOC:** ~120 lines (cache module)
**Tests Added:** 4 tests (~100 LOC)

---

### Day 19-21: Polish + Edge Cases

**Tasks:**

1. **Path Traversal Security** (2 hours)
   - [ ] Validate package names don't contain `..`
   - [ ] Canonicalize paths before resolving
   - [ ] Test: `@workflows/../../etc/passwd` → error

2. **Checksum Verification** (2 hours)
   - [ ] Read checksum from `spn.lock`
   - [ ] Compute SHA256 of installed package
   - [ ] Error if mismatch: "Package corrupted. Run: spn install --force"

3. **Concurrent Access Safety** (2 hours)
   - [ ] Use DashMap for thread-safe cache access
   - [ ] Test: Multiple threads resolving same package
   - [ ] Test: Concurrent install + resolve operations

4. **Documentation** (2 hours)
   - [ ] File: `docs/architecture/PACKAGE_RESOLUTION.md`
   - [ ] Explain resolution algorithm
   - [ ] Document cache structure
   - [ ] Security considerations

**Deliverables:**
- ✅ Workflow includes from packages work
- ✅ Cache optimization complete (99% faster)
- ✅ Security issues fixed (path traversal, checksums, concurrency)
- ✅ Comprehensive documentation

---

## 🚀 Week 4: Interactive CLI + Polish (Days 22-28)

**Goal:** Guided discovery experience + production-ready release

### Day 22-24: Interactive `spn add`

**Tasks:**

1. **Add `dialoguer` Dependency** (15 min)
   - [ ] File: `supernovae-cli/Cargo.toml`
   - [ ] Add: `dialoguer = "0.11"`

2. **Implement Interactive Mode** (4 hours)
   - [ ] File: `supernovae-cli/src/commands/add.rs`
   - [ ] Detect if no package argument provided
   - [ ] Show menu:
     ```
     ? What type of package?
       ❯ Workflow - Complete Nika workflows
         Agent - Multi-turn agents
         Prompt - Prompt templates
         Job - Background tasks
         Skill - Claude Code skills
         MCP Server - Model Context Protocol
     ```
   - [ ] Prompt for search query
   - [ ] Search registry (use existing `spn search`)
   - [ ] Show results with arrow key selection
   - [ ] Install selected package
   - [ ] Ask: "Run now? (Y/n)"

3. **Styling** (1 hour)
   - [ ] Use colors: `colored` crate
   - [ ] Format: `⭐ 45` for stars
   - [ ] Format: `v1.2.0` for versions
   - [ ] Truncate long descriptions

4. **Tests** (2 hours)
   - [ ] Mock stdin/stdout for tests
   - [ ] Test: Select workflow → install
   - [ ] Test: Cancel at menu → exit
   - [ ] Test: No results → retry or exit

**Acceptance Criteria:**
- ✅ `spn add` (no args) shows interactive menu
- ✅ Search, select, install flow works
- ✅ Pretty formatting with colors
- ✅ Tests cover all paths

**Estimate:** 7 hours (3 days)
**LOC:** ~150 lines
**Tests Added:** 3 tests (~80 LOC)

---

### Day 24-25: Interactive `nika init`

**Tasks:**

1. **Add Project Type Templates** (2 hours)
   - [ ] File: `nika/tools/nika/src/templates/mod.rs`
   - [ ] Define 4 templates: empty, content-gen, code-automation, research
   - [ ] Each template: list of recommended packages

2. **Implement Interactive Mode** (3 hours)
   - [ ] File: `nika/tools/nika/src/main.rs` (init command)
   - [ ] Show menu:
     ```
     ? Project type:
       ❯ Empty - Minimal config only
         Content Generation - SEO, blog posts
         Code Automation - Review, refactor, tests
         Research Pipeline - Web scraping, analysis
     ```
   - [ ] Multi-select workflow packages
   - [ ] Install selected packages (call `spn add`)
   - [ ] Create `.nika/` structure
   - [ ] Show next steps

3. **Tests** (1.5 hours)
   - [ ] Test: Empty template → minimal .nika/
   - [ ] Test: Content-gen template → 2 workflows installed
   - [ ] Test: Multi-select packages → all installed

**Acceptance Criteria:**
- ✅ `nika init` shows interactive menu
- ✅ Templates install recommended packages
- ✅ Multi-select works
- ✅ 3 tests pass

**Estimate:** 6.5 hours (1.5 days)
**LOC:** ~100 lines
**Tests Added:** 3 tests (~90 LOC)

---

### Day 26-27: Final Polish

**Tasks:**

1. **Error Message Audit** (2 hours)
   - [ ] Review all error messages
   - [ ] Make actionable: "Package not found. Try: spn search <query>"
   - [ ] Consistent format

2. **Performance Optimization** (2 hours)
   - [ ] Profile `nika run @workflows/name`
   - [ ] Optimize cache lookups
   - [ ] Lazy load registry index

3. **Help Text** (1 hour)
   - [ ] Update `--help` for all commands
   - [ ] Add examples:
     ```
     EXAMPLES:
       spn add @workflows/seo-audit
       nika run @workflows/seo-audit
       nika run local-workflow.nika.yaml
     ```

4. **Cross-Platform Testing** (3 hours)
   - [ ] Test on Linux (Ubuntu 22.04)
   - [ ] Test on macOS (Intel + Apple Silicon)
   - [ ] Test on Windows 11
   - [ ] Document platform-specific issues

**Deliverables:**
- ✅ Clear, actionable error messages
- ✅ Optimized performance
- ✅ Excellent help documentation
- ✅ Cross-platform verified

---

### Day 28: Release Preparation

**Tasks:**

1. **Final Test Suite** (2 hours)
   - [ ] Run: `cargo test --all`
   - [ ] Run: `cargo clippy -- -D warnings`
   - [ ] Run: `cargo fmt --check`
   - [ ] All must pass

2. **Update CHANGELOG** (1 hour)
   - [ ] Document all v0.7.0 changes
   - [ ] Breaking changes (none expected)
   - [ ] Bug fixes
   - [ ] New features

3. **Update Documentation** (2 hours)
   - [ ] Update README.md
   - [ ] Update ROADMAP.md
   - [ ] Add migration guide (if needed)

4. **Tag Release** (30 min)
   - [ ] Git tag: `v0.7.0`
   - [ ] Push: `git push origin v0.7.0`
   - [ ] GitHub release with notes

5. **Announce** (30 min)
   - [ ] GitHub Discussions post
   - [ ] Update project status
   - [ ] Celebrate! 🎉

**Final Checklist:**
- ✅ All 11 integration tests pass
- ✅ Zero clippy warnings
- ✅ Documentation complete
- ✅ CHANGELOG updated
- ✅ Cross-platform verified
- ✅ v0.7.0 tagged and released

---

## 📊 Success Metrics

**At the end of 4 weeks:**

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to First Workflow | < 30 sec | `spn add` → `nika run` |
| Commands to Productivity | 3 | `search`, `add`, `run` |
| Package Types Supported | 6 | workflows, agents, prompts, jobs, skills, mcp |
| Test Coverage | 80%+ | `cargo tarpaulin` |
| Integration Tests | 35+ | `cargo test --test '*'` |
| Cross-Platform | 3 | Linux, macOS, Windows |
| Zero Panics | ✅ | No runtime panics in normal usage |

**User Story Validation:**

```bash
# New user, zero knowledge, 30 seconds
$ spn search seo
🔍 Found @workflows/seo-audit

$ spn add @workflows/seo-audit
✓ Installed

$ nika run @workflows/seo-audit --url https://qrcode-ai.com
✓ SEO Score: 95/100

# ✅ Success in 3 commands, 28 seconds
```

---

## 🔧 Troubleshooting Guide

### If Behind Schedule:

**Cut Scope (in priority order):**
1. ❌ CLI interactive (Week 4) → Move to v0.8.0
2. ❌ `.nika/.cache/` sync (Week 3) → Optional optimization
3. ❌ @prompts support → Move to v0.8.0
4. ✅ KEEP: Bug fixes + workflow/agent resolution (CRITICAL)

**Add Resources:**
- Pair programming on blockers
- Split work: one person on spn, one on nika
- Use rust-async-expert agent for async issues

### If Tests Fail:

1. **Identify pattern:** Is it one module or widespread?
2. **Isolate:** Run single test with `cargo test <name> -- --nocapture`
3. **Debug:** Add `dbg!()` macros
4. **Ask for help:** Include test output in issue

### If Windows Issues:

- Symlinks fail → Use copy fallback
- Path separators → Use `std::path::Path` everywhere
- Line endings → Configure git: `autocrlf=true`

---

## 📝 Daily Standup Format

**Template for tracking progress:**

```markdown
## Day X Standup (YYYY-MM-DD)

### Completed Yesterday:
- [ ] Task 1
- [ ] Task 2

### Today's Plan:
- [ ] Task 3 (Est: 2h)
- [ ] Task 4 (Est: 3h)

### Blockers:
- None / Issue with X, need help

### Metrics:
- Tests: XX passing
- LOC: +XXX/-XXX
- Clippy: X warnings
```

---

## 🎯 Definition of Done

**For each task to be "done":**

1. ✅ Code implemented and formatted (`cargo fmt`)
2. ✅ No clippy warnings (`cargo clippy`)
3. ✅ Unit tests pass
4. ✅ Integration test added (if applicable)
5. ✅ Documentation updated
6. ✅ Manually tested (smoke test)
7. ✅ Committed with descriptive message
8. ✅ Pushed to branch

**For week to be "done":**

1. ✅ All tasks completed
2. ✅ All tests passing (`cargo test --all`)
3. ✅ Code reviewed (self or peer)
4. ✅ Documentation updated
5. ✅ CHANGELOG entry added
6. ✅ Demo prepared (if end of phase)

---

## 🚀 Let's Ship v0.7.0!

**This is the roadmap. Let's build it.** 🦸

**Next Steps:**
1. Review this execution plan
2. Answer 4 questions in VISION-SUMMARY.md
3. Create GitHub milestone for v0.7.0
4. Start Day 1: Fix `spn add` tokio panic

**Ready to code?** 💻
