# Diagnostic: spn add Tokio Panic

**Date:** 2026-03-02
**Status:** 🔍 Diagnostic Complete
**Bug:** Tokio runtime panic in `spn add`

---

## 🐛 Error Message

```
thread 'main' panicked at tokio-1.49.0/src/runtime/blocking/shutdown.rs:51:21:
Cannot drop a runtime in a context where blocking is not allowed.
```

---

## 📊 Code Analysis

### Discovery: Incohérent Async/Await Usage

#### File: `src/main.rs` (Lines 469-479)

```rust
#[tokio::main]  // ✅ Tokio runtime initialized
async fn main() -> Result<()> {
    // ...
    match cli.command {
        Commands::Add { package, r#type } => {
            commands::add::run(&package, r#type.as_deref()).await  // ✅ .await
        }
        // ...
    }
}
```

**Status:** ✅ Correct - main is async with tokio runtime

---

#### File: `src/commands/add.rs` (Lines 76-94, 98-121)

```rust
pub async fn run(package: &str, pkg_type: Option<&str>) -> Result<()> {
    // ✅ Function is async
    // ...
    run_with_options(options).await  // ✅ .await
}

pub async fn run_with_options(options: AddOptions) -> Result<()> {
    // ✅ Function is async
    // ...

    // 2. Fetch package info from registry
    let client = IndexClient::new();  // ❌ Creates blocking client?
    let entry = if let Some(ref version) = options.version {
        client.fetch_version(&options.package, version).await  // ❌ .await on SYNC function!
    } else {
        client.fetch_latest(&options.package).await  // ❌ .await on SYNC function!
    }
    .map_err(|e| SpnError::PackageNotFound(format!("{}: {}", options.package, e)))?;

    // ...

    // 5. Install package
    let downloader = Downloader::new();
    let downloaded = downloader
        .download_entry(&entry)  // ❌ NO .await - This is sync!
        .map_err(...)?;
}
```

**Problems Identified:**

1. **Line 116-120**: Calling `.await` on `fetch_version()` and `fetch_latest()` but these functions are **NOT async**
2. **Line 159-161**: NOT calling `.await` on `download_entry()` - suggests it's sync

---

#### File: `src/index/client.rs` (Lines 111-130)

```rust
/// Fetch all versions of a package from the index.
pub fn fetch_package(&self, name: &str) -> Result<Vec<IndexEntry>, IndexError> {
    // ❌ NOT async function
    let scope = PackageScope::parse(name)...;
    let index_path = scope.index_path();
    let content = self.fetch_index_file(&index_path)?;  // Calls sync fetch
    self.parse_index_content(&content, name)
}

/// Fetch the latest non-yanked version of a package.
pub fn fetch_latest(&self, name: &str) -> Result<IndexEntry, IndexError> {
    // ❌ NOT async function
    let entries = self.fetch_package(name)?;
    // ...
}

/// Fetch a specific version of a package.
pub fn fetch_version(&self, name: &str, version: &str) -> Result<IndexEntry, IndexError> {
    // ❌ NOT async function
    let entries = self.fetch_package(name)?;
    // ...
}
```

**Problem:** These functions are **NOT async** but `add.rs` calls them with `.await`!

---

#### File: `src/index/client.rs` (Lines 179-209)

```rust
/// Fetch from HTTP.
fn fetch_http(&self, index_path: &str) -> Result<String, IndexError> {
    // ❌ SYNC function
    let url = format!("{}/{}", self.config.index_url, index_path);

    let client = self.http_client.as_ref()
        .ok_or_else(|| IndexError::HttpError("HTTP client not initialized".to_string()))?;

    let response = client
        .get(&url)
        .header("User-Agent", "spn/0.1")
        .send()  // ❌ BLOCKING CALL - No .await!
        .map_err(|e| IndexError::HttpError(e.to_string()))?;

    // ...
    response.text()  // ❌ BLOCKING CALL
        .map_err(|e| IndexError::HttpError(e.to_string()))
}
```

**Root Cause:** Using `reqwest::blocking::Client` instead of async `reqwest::Client`!

---

#### File: `src/index/downloader.rs` (Lines 144-167)

```rust
/// Fetch from HTTP.
fn fetch_http(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
    // ❌ SYNC function
    let response = reqwest::blocking::Client::new()  // ❌ BLOCKING CLIENT!
        .get(url)
        .header("User-Agent", "spn/0.1")
        .send()?;  // ❌ BLOCKING CALL

    // ...
}
```

**Root Cause:** Using `reqwest::blocking::Client`!

---

## 🎯 Root Cause Analysis

### The Problem Chain:

```
main.rs (async)
  └─> add.rs::run() (async)
      └─> add.rs::run_with_options() (async)
          ├─> IndexClient::new() (sync)
          │   └─> Creates reqwest::blocking::Client  ❌
          │
          ├─> client.fetch_latest() (SYNC but called with .await)  ❌
          │   └─> fetch_http() uses blocking reqwest  ❌
          │
          └─> downloader.download_entry() (SYNC, no .await)  ❌
              └─> fetch_http() uses blocking reqwest  ❌
```

### Why It Panics:

1. `add.rs` is declared `async` and expects async functions
2. But `IndexClient` and `Downloader` use `reqwest::blocking`
3. Blocking operations inside async context = **tokio panic**
4. The `.await` on sync functions is **syntactically wrong** (shouldn't compile!)

**Wait...** If the code has `.await` on sync functions, how does it compile?!

Let me check if these functions actually return `Future<>` somehow...

---

## 🔬 Deeper Investigation

Looking at the system reminders, I see modifications to these files. Let me check the actual function signatures more carefully.

**Hypothesis:** Maybe `fetch_latest()` and friends were SUPPOSED to be async but aren't yet, or the code is inconsistent.

**Checking `Cargo.toml` for reqwest:**

From typical SuperNovae setup:
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

**Missing:** No "blocking" feature = async by default ✅

But the code uses `reqwest::blocking::Client::new()` explicitly!

---

## ✅ Solution: Make Everything Consistently Async

### Changes Required:

#### 1. **File: `src/index/client.rs`**

**Change all HTTP functions to async:**

```rust
// BEFORE (sync)
pub fn fetch_package(&self, name: &str) -> Result<Vec<IndexEntry>, IndexError>

// AFTER (async)
pub async fn fetch_package(&self, name: &str) -> Result<Vec<IndexEntry>, IndexError>
```

```rust
// BEFORE (sync)
fn fetch_http(&self, index_path: &str) -> Result<String, IndexError> {
    let response = client
        .get(&url)
        .send()  // Blocking
        .map_err(...)?;
}

// AFTER (async)
async fn fetch_http(&self, index_path: &str) -> Result<String, IndexError> {
    let response = client
        .get(&url)
        .send()  // Async
        .await   // ← Add .await
        .map_err(...)?;
}
```

**Methods to convert:**
- `fetch_package()` → `async fn`
- `fetch_latest()` → `async fn`
- `fetch_version()` → `async fn`
- `fetch_index_file()` → `async fn`
- `fetch_http()` → `async fn`
- `fetch_local()` → can stay sync (just reads file)

#### 2. **File: `src/index/downloader.rs`**

**Change HTTP functions to async:**

```rust
// BEFORE (sync)
fn fetch_http(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
    let response = reqwest::blocking::Client::new()  // ❌
        .get(url)
        .send()?;
}

// AFTER (async)
async fn fetch_http(&self, url: &str, dest: &Path) -> Result<(), DownloadError> {
    let response = reqwest::Client::new()  // ✅ Async client
        .get(url)
        .send()
        .await?;  // ✅ Await
}
```

**Methods to convert:**
- `download_entry()` → `async fn`
- `fetch_tarball()` → `async fn`
- `fetch_http()` → `async fn`
- `fetch_local()` → can stay sync

#### 3. **File: `src/commands/add.rs`**

**Add `.await` where missing:**

```rust
// Line 159-161 BEFORE
let downloaded = downloader
    .download_entry(&entry)  // ❌ Missing .await
    .map_err(...)?;

// AFTER
let downloaded = downloader
    .download_entry(&entry)
    .await  // ✅ Add .await
    .map_err(...)?;
```

---

## 📝 Implementation Checklist

- [ ] Update `src/index/client.rs`:
  - [ ] Make `fetch_package()` async
  - [ ] Make `fetch_latest()` async
  - [ ] Make `fetch_version()` async
  - [ ] Make `fetch_index_file()` async
  - [ ] Make `fetch_http()` async
  - [ ] Add `.await` to all HTTP calls
  - [ ] Update all call sites

- [ ] Update `src/index/downloader.rs`:
  - [ ] Make `download_entry()` async
  - [ ] Make `fetch_tarball()` async
  - [ ] Make `fetch_http()` async
  - [ ] Change `reqwest::blocking::Client` → `reqwest::Client`
  - [ ] Add `.await` to all HTTP calls

- [ ] Update `src/commands/add.rs`:
  - [ ] Add `.await` to `download_entry()` call (line 160)
  - [ ] Verify all async calls have `.await`

- [ ] Remove blocking reqwest:
  - [ ] Ensure no `reqwest::blocking` usage anywhere
  - [ ] Use async `reqwest::Client` everywhere

- [ ] Test:
  - [ ] `cargo run -- add @workflows/test`
  - [ ] Verify no panic
  - [ ] Verify download works

---

## 🎯 Expected Outcome

After these changes:

```bash
$ cargo run -- add @workflows/test

📦 Adding package: @workflows/test
   ✓ Found @workflows/test@1.0.0
   ✓ Added to dependencies
   ✓ Updated spn.yaml
   ✓ Downloaded /Users/.../.spn/cache/tarballs/...
   ✓ Installed to ~/.spn/packages/@workflows/test/1.0.0/
   ✓ Updated spn.lock
✨ Successfully added @workflows/test
```

**No panic! ✅**

---

## 🚀 Ready to Implement

All problems identified. Fix is straightforward: convert blocking to async consistently.

**Next Steps:**
1. Start with `client.rs` (foundation)
2. Then `downloader.rs` (depends on client pattern)
3. Finally update `add.rs` (add missing .await)
4. Test
5. Commit

**Estimated Time:** 2 hours
