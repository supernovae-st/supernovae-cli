# Secret Management Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement complete secret management UX overhaul with 8 features across P0-P4 priorities.

**Architecture:** Modular storage backends with encryption layer, metadata tracking, and interactive CLI.

**Tech Stack:** Rust, keyring v3, secrecy, zeroize, age v0.11, rops v0.1.7, dialoguer, toml

---

## Task 1: Add StorageBackend Enum and Types

**Files:**
- Create: `src/secrets/storage.rs`
- Modify: `src/secrets/mod.rs`
- Test: `src/secrets/storage.rs` (inline tests)

**Step 1: Write the failing test**

```rust
// src/secrets/storage.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_backend_from_str() {
        assert_eq!(StorageBackend::from_str("keychain").unwrap(), StorageBackend::Keychain);
        assert_eq!(StorageBackend::from_str("env").unwrap(), StorageBackend::Env);
        assert_eq!(StorageBackend::from_str("global").unwrap(), StorageBackend::Global);
        assert_eq!(StorageBackend::from_str("shell").unwrap(), StorageBackend::Shell);
        assert!(StorageBackend::from_str("invalid").is_err());
    }

    #[test]
    fn test_storage_backend_display() {
        assert_eq!(StorageBackend::Keychain.to_string(), "keychain");
        assert_eq!(StorageBackend::Env.to_string(), "env");
    }

    #[test]
    fn test_storage_backend_description() {
        let desc = StorageBackend::Keychain.description();
        assert!(desc.contains("OS Keychain"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test storage_backend --lib -- --nocapture`
Expected: FAIL with "cannot find type `StorageBackend`"

**Step 3: Write minimal implementation**

```rust
// src/secrets/storage.rs
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageBackend {
    #[default]
    Keychain,
    Env,
    Global,
    Shell,
}

#[derive(Error, Debug)]
#[error("Invalid storage backend: {0}. Valid: keychain, env, global, shell")]
pub struct InvalidStorageBackend(String);

impl FromStr for StorageBackend {
    type Err = InvalidStorageBackend;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "keychain" => Ok(Self::Keychain),
            "env" => Ok(Self::Env),
            "global" => Ok(Self::Global),
            "shell" => Ok(Self::Shell),
            _ => Err(InvalidStorageBackend(s.to_string())),
        }
    }
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keychain => write!(f, "keychain"),
            Self::Env => write!(f, "env"),
            Self::Global => write!(f, "global"),
            Self::Shell => write!(f, "shell"),
        }
    }
}

impl StorageBackend {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Keychain => "OS Keychain - Most secure, protected by login password",
            Self::Env => "Project .env file - Good for project-specific keys",
            Self::Global => "Global ~/.spn/secrets.env - Shared across projects",
            Self::Shell => "Shell export - Print command only, you handle storage",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Keychain => "🔐",
            Self::Env => "📁",
            Self::Global => "🌍",
            Self::Shell => "📋",
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test storage_backend --lib -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/secrets/storage.rs src/secrets/mod.rs
git commit -m "feat(secrets): add StorageBackend enum with TDD

- Add StorageBackend: Keychain, Env, Global, Shell
- Implement FromStr, Display, description(), emoji()
- 3 unit tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 2: Add --storage Flag to CLI

**Files:**
- Modify: `src/commands/provider.rs:ProviderSetArgs`
- Test: `tests/cli_provider_tests.rs`

**Step 1: Write the failing test**

```rust
// tests/cli_provider_tests.rs
#[test]
fn test_provider_set_storage_flag_keychain() {
    let cmd = Command::cargo_bin("spn").unwrap();
    let output = cmd
        .args(["provider", "set", "anthropic", "--storage", "keychain", "--help"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("--storage"));
}

#[test]
fn test_provider_set_storage_flag_invalid() {
    let cmd = Command::cargo_bin("spn").unwrap();
    let output = cmd
        .args(["provider", "set", "anthropic", "--storage", "invalid"])
        .env("ANTHROPIC_API_KEY", "test")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid storage"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_provider_set_storage --test cli_provider_tests`
Expected: FAIL - no --storage flag exists

**Step 3: Write minimal implementation**

```rust
// src/commands/provider.rs - modify ProviderSetArgs
#[derive(Args)]
pub struct ProviderSetArgs {
    /// Provider name (anthropic, openai, mistral, groq, deepseek, ollama)
    pub provider: String,

    /// API key (prompted if not provided)
    #[arg(long, env)]
    pub key: Option<String>,

    /// Storage backend: keychain (default), env, global, shell
    #[arg(long, short = 's', default_value = "keychain")]
    pub storage: StorageBackend,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_provider_set_storage --test cli_provider_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/provider.rs tests/cli_provider_tests.rs
git commit -m "feat(cli): add --storage flag to provider set

- Add --storage flag with keychain/env/global/shell options
- Default remains keychain for backward compatibility
- 2 CLI tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 3: Implement Env Storage Backend

**Files:**
- Create: `src/secrets/env_storage.rs`
- Modify: `src/secrets/mod.rs`
- Test: inline + integration

**Step 1: Write the failing test**

```rust
// src/secrets/env_storage.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_in_dotenv_creates_file() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        store_in_dotenv("anthropic", "sk-ant-test123", &env_path).unwrap();

        let content = std::fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-test123"));
    }

    #[test]
    fn test_store_in_dotenv_appends() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        std::fs::write(&env_path, "EXISTING=value\n").unwrap();

        store_in_dotenv("anthropic", "sk-ant-test123", &env_path).unwrap();

        let content = std::fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("EXISTING=value"));
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-test123"));
    }

    #[test]
    fn test_store_in_dotenv_updates_existing() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        std::fs::write(&env_path, "ANTHROPIC_API_KEY=old-key\n").unwrap();

        store_in_dotenv("anthropic", "sk-ant-new123", &env_path).unwrap();

        let content = std::fs::read_to_string(&env_path).unwrap();
        assert!(!content.contains("old-key"));
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-new123"));
    }

    #[test]
    fn test_read_from_dotenv() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        std::fs::write(&env_path, "ANTHROPIC_API_KEY=sk-ant-test123\n").unwrap();

        let key = read_from_dotenv("anthropic", &env_path).unwrap();
        assert_eq!(key.as_ref(), "sk-ant-test123");
    }

    #[test]
    fn test_delete_from_dotenv() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        std::fs::write(&env_path, "ANTHROPIC_API_KEY=test\nOTHER=value\n").unwrap();

        delete_from_dotenv("anthropic", &env_path).unwrap();

        let content = std::fs::read_to_string(&env_path).unwrap();
        assert!(!content.contains("ANTHROPIC_API_KEY"));
        assert!(content.contains("OTHER=value"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test env_storage --lib`
Expected: FAIL - functions don't exist

**Step 3: Write minimal implementation**

```rust
// src/secrets/env_storage.rs
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

use super::keyring::provider_env_var;

/// Store a secret in a .env file
pub fn store_in_dotenv(provider: &str, key: &str, path: &Path) -> Result<()> {
    let env_var = provider_env_var(provider);
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;

    // Read existing content
    if path.exists() {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.starts_with(&format!("{}=", env_var)) {
                lines.push(format!("{}={}", env_var, key));
                found = true;
            } else {
                lines.push(line);
            }
        }
    }

    // Append if not found
    if !found {
        lines.push(format!("{}={}", env_var, key));
    }

    // Atomic write
    let temp_path = path.with_extension("env.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)?;

    for line in &lines {
        writeln!(file, "{}", line)?;
    }
    file.sync_all()?;
    fs::rename(&temp_path, path)?;

    // Set permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Read a secret from a .env file
pub fn read_from_dotenv(provider: &str, path: &Path) -> Result<Zeroizing<String>> {
    let env_var = provider_env_var(provider);

    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix(&format!("{}=", env_var)) {
            return Ok(Zeroizing::new(value.to_string()));
        }
    }

    anyhow::bail!("Key {} not found in {}", env_var, path.display())
}

/// Delete a secret from a .env file
pub fn delete_from_dotenv(provider: &str, path: &Path) -> Result<()> {
    let env_var = provider_env_var(provider);

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.starts_with(&format!("{}=", env_var)))
        .collect();

    let temp_path = path.with_extension("env.tmp");
    fs::write(&temp_path, lines.join("\n") + "\n")?;
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Check if .env is in .gitignore
pub fn is_gitignored(path: &Path) -> bool {
    let gitignore = path.parent()
        .map(|p| p.join(".gitignore"))
        .filter(|p| p.exists());

    if let Some(gitignore_path) = gitignore {
        if let Ok(content) = fs::read_to_string(gitignore_path) {
            return content.lines().any(|line| {
                let trimmed = line.trim();
                trimmed == ".env" || trimmed == "*.env" || trimmed == ".env*"
            });
        }
    }
    false
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test env_storage --lib`
Expected: PASS (5 tests)

**Step 5: Commit**

```bash
git add src/secrets/env_storage.rs src/secrets/mod.rs
git commit -m "feat(secrets): add .env file storage backend

- store_in_dotenv() with atomic writes and 0o600 permissions
- read_from_dotenv() with Zeroizing<String> return
- delete_from_dotenv() preserves other entries
- is_gitignored() helper for security warnings
- 5 unit tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 4: Implement Global Storage Backend

**Files:**
- Modify: `src/secrets/env_storage.rs` (add global path helper)
- Test: inline

**Step 1: Write the failing test**

```rust
#[test]
fn test_global_secrets_path() {
    let path = global_secrets_path();
    assert!(path.ends_with(".spn/secrets.env"));
}

#[test]
fn test_store_in_global() {
    // Uses temp HOME for isolation
    let temp = TempDir::new().unwrap();
    std::env::set_var("HOME", temp.path());

    store_in_global("anthropic", "sk-ant-test").unwrap();

    let path = temp.path().join(".spn/secrets.env");
    assert!(path.exists());
    let content = std::fs::read_to_string(path).unwrap();
    assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-test"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test global_secrets --lib`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// Add to src/secrets/env_storage.rs

/// Get path to global secrets file
pub fn global_secrets_path() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".spn")
        .join("secrets.env")
}

/// Store in global ~/.spn/secrets.env
pub fn store_in_global(provider: &str, key: &str) -> Result<()> {
    let path = global_secrets_path();

    // Ensure directory exists with proper permissions
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }

    store_in_dotenv(provider, key, &path)
}

/// Read from global ~/.spn/secrets.env
pub fn read_from_global(provider: &str) -> Result<Zeroizing<String>> {
    read_from_dotenv(provider, &global_secrets_path())
}

/// Delete from global ~/.spn/secrets.env
pub fn delete_from_global(provider: &str) -> Result<()> {
    delete_from_dotenv(provider, &global_secrets_path())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test global_secrets --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add src/secrets/env_storage.rs
git commit -m "feat(secrets): add global ~/.spn/secrets.env storage

- global_secrets_path() helper
- store/read/delete_from_global() functions
- Auto-create ~/.spn/ with 0o700 permissions
- 2 unit tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 5: Implement Storage Router

**Files:**
- Modify: `src/commands/provider.rs`
- Test: integration tests

**Step 1: Write the failing test**

```rust
// tests/provider_storage_tests.rs
#[tokio::test]
async fn test_provider_set_routes_to_keychain() {
    // Mock keychain for testing
    let result = store_secret("test-provider", "test-key", StorageBackend::Keychain);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_provider_set_routes_to_env() {
    let temp = TempDir::new().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();

    let result = store_secret("anthropic", "sk-ant-test", StorageBackend::Env);
    assert!(result.is_ok());

    let env_path = temp.path().join(".env");
    assert!(env_path.exists());
}

#[tokio::test]
async fn test_provider_set_shell_prints_export() {
    let output = store_secret("anthropic", "sk-ant-test", StorageBackend::Shell);
    // Shell mode doesn't store, just returns export command
    assert!(output.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test provider_storage --test provider_storage_tests`
Expected: FAIL - store_secret doesn't exist

**Step 3: Write minimal implementation**

```rust
// src/secrets/storage.rs - add store_secret function

use super::env_storage::{store_in_dotenv, store_in_global};
use super::keyring::SpnKeyring;
use super::types::provider_env_var;
use anyhow::Result;
use std::path::PathBuf;

/// Store a secret using the specified backend
pub fn store_secret(provider: &str, key: &str, backend: StorageBackend) -> Result<StoreResult> {
    match backend {
        StorageBackend::Keychain => {
            SpnKeyring::set(provider, key)?;
            Ok(StoreResult::Stored {
                backend,
                location: "OS Keychain".to_string()
            })
        }
        StorageBackend::Env => {
            let path = PathBuf::from(".env");
            store_in_dotenv(provider, key, &path)?;
            Ok(StoreResult::Stored {
                backend,
                location: path.display().to_string()
            })
        }
        StorageBackend::Global => {
            store_in_global(provider, key)?;
            Ok(StoreResult::Stored {
                backend,
                location: "~/.spn/secrets.env".to_string()
            })
        }
        StorageBackend::Shell => {
            let env_var = provider_env_var(provider);
            Ok(StoreResult::ExportCommand {
                command: format!("export {}='{}'", env_var, key)
            })
        }
    }
}

#[derive(Debug)]
pub enum StoreResult {
    Stored { backend: StorageBackend, location: String },
    ExportCommand { command: String },
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test provider_storage --test provider_storage_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add src/secrets/storage.rs src/commands/provider.rs
git commit -m "feat(secrets): add storage router for all backends

- store_secret() routes to keychain/env/global/shell
- StoreResult enum for different outcomes
- Shell mode returns export command without storing
- 3 integration tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 6: Add Key Metadata Storage

**Files:**
- Create: `src/secrets/metadata.rs`
- Test: inline

**Step 1: Write the failing test**

```rust
// src/secrets/metadata.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_metadata_roundtrip() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("metadata.toml");

        let mut store = MetadataStore::new(&path);
        store.set_key_metadata("anthropic", KeyMetadata {
            key_id: "sk-ant-...xyz".to_string(),
            storage: StorageBackend::Keychain,
            created_at: Utc::now(),
            rotated_at: None,
            last_used: None,
            rotation_days: 90,
        }).unwrap();

        store.save().unwrap();

        // Reload and verify
        let loaded = MetadataStore::load(&path).unwrap();
        let meta = loaded.get_key_metadata("anthropic").unwrap();
        assert_eq!(meta.key_id, "sk-ant-...xyz");
        assert_eq!(meta.rotation_days, 90);
    }

    #[test]
    fn test_rotation_due_calculation() {
        let meta = KeyMetadata {
            key_id: "test".to_string(),
            storage: StorageBackend::Keychain,
            created_at: Utc::now() - chrono::Duration::days(85),
            rotated_at: None,
            last_used: None,
            rotation_days: 90,
        };

        assert_eq!(meta.days_until_rotation(), 5);
        assert!(!meta.is_rotation_due());

        let overdue = KeyMetadata {
            created_at: Utc::now() - chrono::Duration::days(95),
            ..meta
        };
        assert!(overdue.is_rotation_due());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test metadata --lib`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// src/secrets/metadata.rs
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::storage::StorageBackend;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub key_id: String,
    pub storage: StorageBackend,
    pub created_at: DateTime<Utc>,
    pub rotated_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub rotation_days: u32,
}

impl KeyMetadata {
    pub fn days_until_rotation(&self) -> i64 {
        let base_date = self.rotated_at.unwrap_or(self.created_at);
        let due_date = base_date + chrono::Duration::days(self.rotation_days as i64);
        (due_date - Utc::now()).num_days()
    }

    pub fn is_rotation_due(&self) -> bool {
        self.days_until_rotation() <= 0
    }

    pub fn is_rotation_soon(&self, warn_days: i64) -> bool {
        self.days_until_rotation() <= warn_days
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MetadataStore {
    #[serde(skip)]
    path: PathBuf,
    pub providers: HashMap<String, KeyMetadata>,
}

impl MetadataStore {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            providers: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new(path));
        }
        let content = fs::read_to_string(path)?;
        let mut store: Self = toml::from_str(&content)?;
        store.path = path.to_path_buf();
        Ok(store)
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
            }
        }

        let content = toml::to_string_pretty(self)?;
        let temp_path = self.path.with_extension("toml.tmp");
        fs::write(&temp_path, &content)?;
        fs::rename(&temp_path, &self.path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    pub fn set_key_metadata(&mut self, provider: &str, metadata: KeyMetadata) -> Result<()> {
        self.providers.insert(provider.to_string(), metadata);
        Ok(())
    }

    pub fn get_key_metadata(&self, provider: &str) -> Option<&KeyMetadata> {
        self.providers.get(provider)
    }

    pub fn remove_key_metadata(&mut self, provider: &str) {
        self.providers.remove(provider);
    }
}

pub fn metadata_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".spn")
        .join("keys")
        .join("metadata.toml")
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test metadata --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add src/secrets/metadata.rs src/secrets/mod.rs
git commit -m "feat(secrets): add key metadata storage for rotation tracking

- KeyMetadata struct with rotation calculation
- MetadataStore for TOML persistence
- Atomic writes with 0o600 permissions
- days_until_rotation() and is_rotation_due() helpers
- 2 unit tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 7: Add Interactive Wizard

**Files:**
- Create: `src/secrets/wizard.rs`
- Modify: `src/commands/provider.rs`
- Test: manual (interactive)

**Step 1: Write the failing test**

```rust
// src/secrets/wizard.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_options_count() {
        let options = storage_options();
        assert_eq!(options.len(), 4);
    }

    #[test]
    fn test_storage_option_labels() {
        let options = storage_options();
        assert!(options[0].contains("Keychain"));
        assert!(options[1].contains(".env"));
        assert!(options[2].contains("Global"));
        assert!(options[3].contains("clipboard"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test wizard --lib`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// src/secrets/wizard.rs
use dialoguer::{theme::ColorfulTheme, Password, Select};
use console::style;
use anyhow::Result;

use super::storage::StorageBackend;
use super::keyring::mask_api_key;

/// Get storage selection options for the wizard
pub fn storage_options() -> Vec<String> {
    vec![
        format!("{} OS Keychain (recommended)\n      Most secure. Protected by your login password.",
            StorageBackend::Keychain.emoji()),
        format!("{} Project .env file\n      Stored in ./.env (gitignored). Good for project-specific keys.",
            StorageBackend::Env.emoji()),
        format!("{} Global secrets file\n      Stored in ~/.spn/secrets.env. Shared across all projects.",
            StorageBackend::Global.emoji()),
        format!("{} Copy to clipboard\n      Key is validated and copied. You handle storage.",
            StorageBackend::Shell.emoji()),
    ]
}

/// Run interactive storage selection wizard
pub fn select_storage() -> Result<StorageBackend> {
    let options = storage_options();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Where should this key be stored?")
        .items(&options)
        .default(0)
        .interact()?;

    Ok(match selection {
        0 => StorageBackend::Keychain,
        1 => StorageBackend::Env,
        2 => StorageBackend::Global,
        3 => StorageBackend::Shell,
        _ => unreachable!(),
    })
}

/// Prompt for API key with hidden input
pub fn prompt_api_key(provider: &str) -> Result<String> {
    let env_var = super::keyring::provider_env_var(provider);

    println!();
    println!("{}", style(format!("Setting API key for {} ({})", provider, env_var)).bold());
    println!();

    let key = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter your {} API key", provider))
        .interact()?;

    Ok(key)
}

/// Display success message after storing
pub fn display_success(provider: &str, key: &str, backend: StorageBackend, location: &str) {
    println!();
    println!("{} API key stored in {}",
        style("✅").green(),
        style(location).cyan());
    println!("   Masked: {}", mask_api_key(key));
    println!("   Rotation reminder set for 90 days");
    println!();
}

/// Check if running in interactive mode
pub fn is_interactive() -> bool {
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test wizard --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add src/secrets/wizard.rs src/secrets/mod.rs Cargo.toml
git commit -m "feat(secrets): add interactive wizard for storage selection

- select_storage() with dialoguer Select
- prompt_api_key() with hidden Password input
- display_success() with colored output
- is_interactive() TTY detection
- Add atty dependency
- 2 unit tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 8: Add Provider Status Command

**Files:**
- Modify: `src/commands/provider.rs`
- Test: CLI tests

**Step 1: Write the failing test**

```rust
// tests/cli_provider_tests.rs
#[test]
fn test_provider_status_command_exists() {
    let output = Command::cargo_bin("spn")
        .unwrap()
        .args(["provider", "status", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("status"));
}

#[test]
fn test_provider_status_shows_table() {
    let output = Command::cargo_bin("spn")
        .unwrap()
        .args(["provider", "status"])
        .env("ANTHROPIC_API_KEY", "sk-ant-test123")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("Provider"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test provider_status --test cli_provider_tests`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// src/commands/provider.rs - add Status subcommand

#[derive(Subcommand)]
pub enum ProviderCommand {
    // ... existing commands ...

    /// Show status of all configured API keys
    Status(ProviderStatusArgs),
}

#[derive(Args)]
pub struct ProviderStatusArgs {
    /// Only show warnings (for shell hooks)
    #[arg(long, short)]
    pub quiet: bool,

    /// Check rotation status and exit with code 1 if any due
    #[arg(long)]
    pub check_rotation: bool,
}

pub async fn handle_status(args: ProviderStatusArgs) -> Result<()> {
    use crate::secrets::{metadata::{metadata_path, MetadataStore}, SUPPORTED_PROVIDERS};
    use crate::secrets::resolve_api_key;
    use console::style;

    let metadata = MetadataStore::load(&metadata_path()).unwrap_or_default();
    let mut has_warnings = false;

    if !args.quiet {
        println!();
        println!("{}", style("┌─────────────────────────────────────────────────────────────────────────┐").dim());
        println!("{}", style("│  🔑 API KEY STATUS                                                      │").bold());
        println!("{}", style("├─────────────────────────────────────────────────────────────────────────┤").dim());
        println!("{}", style("│                                                                         │").dim());
        println!("{}", style("│  Provider       Storage      Status       Rotation                      │").dim());
        println!("{}", style("│  ─────────────────────────────────────────────────────────────────────  │").dim());
    }

    for provider in SUPPORTED_PROVIDERS {
        let (key, source) = match resolve_api_key(provider) {
            Some(result) => result,
            None => {
                if !args.quiet {
                    println!("│  {:<14} {:<12} {:<12} {}                             │",
                        provider,
                        style("❌ missing").red(),
                        "-",
                        "-"
                    );
                }
                continue;
            }
        };

        let meta = metadata.get_key_metadata(provider);
        let rotation_status = if let Some(m) = meta {
            let days = m.days_until_rotation();
            if days <= 0 {
                has_warnings = true;
                style(format!("⚠️ OVERDUE by {} days", -days)).red().to_string()
            } else if days <= 14 {
                has_warnings = true;
                style(format!("⚠️ {} days", days)).yellow().to_string()
            } else {
                style(format!("✅ {} days", days)).green().to_string()
            }
        } else {
            "No metadata".to_string()
        };

        let storage_display = match source {
            crate::secrets::SecretSource::Keychain => "🔐 keychain",
            crate::secrets::SecretSource::Environment => "🌍 env var",
            crate::secrets::SecretSource::DotEnv => "📁 .env",
            _ => "❓ unknown",
        };

        if !args.quiet {
            println!("│  {:<14} {:<12} {:<12} {:<20}         │",
                provider,
                storage_display,
                style("✅ valid").green(),
                rotation_status
            );
        }
    }

    if !args.quiet {
        println!("{}", style("│                                                                         │").dim());
        println!("{}", style("└─────────────────────────────────────────────────────────────────────────┘").dim());
    }

    if has_warnings {
        if args.quiet {
            eprintln!("{} Some API keys need rotation", style("⚠️").yellow());
        }
        if args.check_rotation {
            std::process::exit(1);
        }
    }

    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test provider_status --test cli_provider_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/provider.rs tests/cli_provider_tests.rs
git commit -m "feat(cli): add 'spn provider status' command

- Shows all providers with storage location and status
- Rotation countdown with color-coded warnings
- --quiet flag for shell hooks
- --check-rotation exits with code 1 if any due
- 2 CLI tests passing

Co-Authored-By: Claude <noreply@anthropic.com>
Co-Authored-By: Nika 🦋 <nika@supernovae.studio>"
```

---

## Task 9-16: Remaining Features (Summary)

Due to length, remaining tasks follow the same TDD pattern:

### Task 9: Wire Wizard into Provider Set Command
### Task 10: Add Config File Support (~/.spn/config.toml)
### Task 11: Add age Crate Dependency
### Task 12: Implement Age Encryption/Decryption
### Task 13: Add rops Crate Dependency
### Task 14: Implement SOPS Import/Export
### Task 15: Add 1Password CLI Detection
### Task 16: Implement Hybrid 1Password Integration

---

## Task 17: Comprehensive Security Test Suite

**Files:**
- Create: `tests/security_tests.rs`

**Tests to implement:**

```rust
// tests/security_tests.rs

#[test]
fn test_secrets_zeroed_on_drop() {
    // Verify Zeroizing<String> clears memory
}

#[test]
fn test_env_file_permissions() {
    // Verify .env files created with 0o600
}

#[test]
fn test_metadata_file_permissions() {
    // Verify metadata.toml created with 0o600
}

#[test]
fn test_global_dir_permissions() {
    // Verify ~/.spn/ created with 0o700
}

#[test]
fn test_atomic_write_no_partial() {
    // Simulate crash during write, verify no corruption
}

#[test]
fn test_no_key_in_logs() {
    // Verify keys never appear in debug output
}

#[test]
fn test_no_key_in_error_messages() {
    // Verify error messages don't leak keys
}

#[test]
fn test_gitignore_warning() {
    // Verify warning when .env not gitignored
}

#[test]
fn test_key_validation_before_storage() {
    // Verify invalid keys rejected before storing
}

#[test]
fn test_encrypted_file_not_readable() {
    // Verify .age/.sops files are actually encrypted
}
```

---

## Execution Order

1. Tasks 1-5: Core storage backends (P0)
2. Tasks 6-8: Metadata and status (P1)
3. Task 9: Wire wizard (P1)
4. Tasks 10: Config file (P2)
5. Tasks 11-14: Encryption (P2)
6. Tasks 15-16: 1Password (P3)
7. Task 17: Security test suite

**Total estimated: 17 tasks, ~120 tests**
