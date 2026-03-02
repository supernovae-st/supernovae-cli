# Week 1 Day 1-2 — Action Plan

**Date:** 2026-03-02
**Goal:** Fix `spn add` tokio panic
**Time Estimate:** 3.5 hours
**Status:** 🚀 In Progress

---

## 🎯 Objective

Fix the tokio runtime panic that occurs when running `spn add @workflows/name`.

**Error:**
```
thread 'main' panicked at tokio-1.49.0/src/runtime/blocking/shutdown.rs:51:21:
Cannot drop a runtime in a context where blocking is not allowed.
```

---

## 📋 Task Breakdown

### Task 1: Diagnostic (30 min) ⏱️

**Actions:**
1. Run `spn add` with full backtrace to identify exact panic location
2. Analyze `commands/add.rs` for async/blocking context issues
3. Identify where `IndexClient` or `Downloader` creates tokio runtime
4. Document findings

**Expected Output:**
- Clear understanding of where runtime is created/dropped
- Exact call stack leading to panic
- Root cause identified

---

### Task 2: Analyze Current Code (30 min) ⏱️

**Files to Review:**
- `src/commands/add.rs` (main entry point)
- `src/index/client.rs` (IndexClient - likely uses async)
- `src/index/downloader.rs` (Downloader - likely uses reqwest blocking)

**Questions to Answer:**
1. Is `add::run()` async or sync?
2. Does `IndexClient::new()` create a tokio runtime?
3. Does `Downloader::download_entry()` use blocking reqwest?
4. Where is the runtime being dropped in a blocking context?

---

### Task 3: Implement Fix (2 hours) ⏱️

**Strategy:** Ensure consistent async/await usage throughout the chain.

**Option A: Make Everything Async** (Preferred)
```rust
// src/commands/add.rs
pub async fn run(package: String, options: AddOptions) -> Result<(), SpnError> {
    // All async calls
    let client = IndexClient::new();  // Should NOT create runtime
    let entry = client.fetch_latest(&package).await?;

    let downloader = Downloader::new();
    let downloaded = downloader.download_entry(&entry).await?;

    // ...
}

// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    match cli.command {
        Commands::Add { package, .. } => {
            commands::add::run(package, options).await?;
        }
        // ...
    }
}
```

**Option B: Use Runtime Explicitly**
```rust
// src/commands/add.rs
pub fn run(package: String, options: AddOptions) -> Result<(), SpnError> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Async logic here
    })
}
```

**Decision:** Use Option A (make everything properly async).

---

### Task 4: Refactor Code (1.5 hours) ⏱️

**Changes Required:**

#### File: `src/commands/add.rs`

**Current (lines 60-170):**
```rust
pub fn run(package: String, options: AddOptions) -> Result<(), SpnError> {
    // Sync function calling async code = problem
    let client = IndexClient::new();  // Creates runtime?
    let entry = client.fetch_latest(&package)?;  // Blocks?

    let downloader = Downloader::new();
    let downloaded = downloader.download_entry(&entry)?;  // Blocks?
}
```

**Target:**
```rust
pub async fn run(package: String, options: AddOptions) -> Result<(), SpnError> {
    // Properly async
    let client = IndexClient::new();
    let entry = client.fetch_latest(&package).await?;

    let downloader = Downloader::new();
    let downloaded = downloader.download_entry(&entry).await?;
}
```

#### File: `src/index/client.rs`

**Review methods:**
- `fetch_package()` - Should be async?
- `fetch_latest()` - Should be async?
- `fetch_http()` - Currently uses blocking reqwest, change to async

**Current (lines 180-209):**
```rust
fn fetch_http(&self, index_path: &str) -> Result<String, IndexError> {
    let response = client
        .get(&url)
        .send()  // Blocking!
        .map_err(|e| IndexError::HttpError(e.to_string()))?;
}
```

**Target:**
```rust
async fn fetch_http(&self, index_path: &str) -> Result<String, IndexError> {
    let response = client
        .get(&url)
        .send()  // Async!
        .await
        .map_err(|e| IndexError::HttpError(e.to_string()))?;
}
```

#### File: `src/index/downloader.rs`

**Current (lines 144-167):**
```rust
fn fetch_http(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
    let response = reqwest::blocking::Client::new()  // Blocking client!
        .get(url)
        .send()?;
}
```

**Target:**
```rust
async fn fetch_http(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
    let response = reqwest::Client::new()  // Async client!
        .get(url)
        .send()
        .await?;
}
```

#### File: `src/main.rs`

**Add tokio runtime:**
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Existing CLI parsing...

    match cli.command {
        Commands::Add { package, .. } => {
            commands::add::run(package, options).await?;
        }
        // Other commands...
    }

    Ok(())
}
```

---

### Task 5: Update Dependencies (15 min) ⏱️

**File: `Cargo.toml`**

Ensure we have the right dependencies:
```toml
[dependencies]
tokio = { version = "1.49", features = ["full"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
# Remove "blocking" feature from reqwest
```

---

### Task 6: Test Fix (30 min) ⏱️

**Manual Tests:**
```bash
# Test 1: Add package (should not panic)
$ cargo run -- add @workflows/test

# Test 2: Add with version
$ cargo run -- add @workflows/test@1.0.0

# Test 3: Add multiple times (cache behavior)
$ cargo run -- add @workflows/test
$ cargo run -- add @workflows/test
```

**Expected Results:**
- ✅ No panic
- ✅ Package downloaded
- ✅ `spn.yaml` updated
- ✅ `spn.lock` created

---

### Task 7: Add Integration Test (1 hour) ⏱️

**File: `tests/integration/add_test.rs`** (new file)

```rust
#[cfg(test)]
mod add_tests {
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_add_workflow_package() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path();

        // Setup test registry
        std::env::set_var("SPN_REGISTRY_URL", "file://./tests/fixtures/registry");

        // Run add command
        let result = supernovae_cli::commands::add::run(
            "@workflows/test".to_string(),
            Default::default(),
        ).await;

        // Assertions
        assert!(result.is_ok());
        assert!(project_dir.join("spn.yaml").exists());
        assert!(project_dir.join("spn.lock").exists());

        // Verify package installed
        let home = dirs::home_dir().unwrap();
        let pkg_path = home.join(".spn/packages/@workflows/test/1.0.0");
        assert!(pkg_path.exists());
    }

    #[tokio::test]
    async fn test_add_no_panic() {
        // Regression test for tokio panic
        let result = supernovae_cli::commands::add::run(
            "@workflows/test".to_string(),
            Default::default(),
        ).await;

        // Should not panic, even if error
        assert!(result.is_ok() || result.is_err());
    }
}
```

---

## ✅ Acceptance Criteria

**Must Pass:**
- [ ] `cargo run -- add @workflows/test` completes without panic
- [ ] Package downloaded to `~/.spn/packages/`
- [ ] `spn.yaml` created/updated with dependency
- [ ] `spn.lock` created/updated with exact version
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo clippy` has zero errors

**Nice to Have:**
- [ ] Performance: Add completes in < 5 seconds
- [ ] Error messages are clear and actionable
- [ ] Works on Linux, macOS, Windows

---

## 🐛 Debugging Checklist

If fix doesn't work, check:

1. **Is tokio runtime properly initialized?**
   ```rust
   #[tokio::main]
   async fn main() { ... }
   ```

2. **Are all HTTP calls using async reqwest?**
   ```rust
   reqwest::Client::new()  // Not blocking::Client
   .send().await?          // Not .send()?
   ```

3. **Is the entire call chain async?**
   ```
   main() async → add::run() async → client.fetch() async
   ```

4. **Are we mixing blocking and async?**
   - Look for `std::thread::spawn` in async context
   - Look for `block_on()` inside async function

---

## 📝 Commit Message Template

```
fix(add): resolve tokio runtime panic in blocking context

Problem:
- `spn add` panicked when dropping tokio runtime
- Root cause: mixing blocking and async contexts

Solution:
- Converted entire add command chain to async/await
- Changed IndexClient to use async reqwest
- Changed Downloader to use async reqwest
- Added #[tokio::main] to main function

Testing:
- Manual: `spn add @workflows/test` succeeds
- Integration test: test_add_no_panic passes

Closes #XXX

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>
```

---

## 🚀 Next Steps (After This Task)

Once this is done:
1. ✅ Day 1-2 complete
2. → Day 2-3: Fix `nika init` invalid examples
3. → Day 3-4: Create resolver foundation

---

**Ready to execute? Let's fix this bug! 🦸**
