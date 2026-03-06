//! .env file storage backend for secrets.
//!
//! Provides functions to store, read, and delete secrets from .env files.
//! Supports both project-local .env and global ~/.spn/secrets.env files.
//!
//! TODO(v0.14): Integrate with `spn provider migrate` and storage backend selection

#![allow(dead_code)]

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use anyhow::{Context, Result};
use zeroize::Zeroizing;

use super::types::provider_env_var;
use super::storage::global_secrets_path;

/// RAII guard to clean up temp files on error.
struct TempFileGuard<'a>(&'a std::path::Path);

impl Drop for TempFileGuard<'_> {
    fn drop(&mut self) {
        // Best effort cleanup - ignore errors
        let _ = std::fs::remove_file(self.0);
    }
}

/// Store a secret in a .env file.
///
/// If the key already exists, it will be updated.
/// Uses atomic write (temp file + rename) to prevent corruption.
/// Sets file permissions to 0o600 (owner read/write only) on Unix.
pub fn store_in_dotenv(provider: &str, key: &str, path: &Path) -> Result<()> {
    let env_var = provider_env_var(provider);
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;

    // Read existing content if file exists
    if path.exists() {
        let file = File::open(path).context("Failed to open .env file for reading")?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.context("Failed to read line from .env file")?;
            if line.starts_with(&format!("{}=", env_var)) {
                // Replace existing key
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

    // Atomic write: write to temp file, then rename
    let temp_path = path.with_extension("env.tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).context("Failed to create parent directory")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                    .context("Failed to set directory permissions")?;
            }
        }
    }

    // RAII guard ensures temp file is cleaned up on error
    let _guard = TempFileGuard(&temp_path);

    // Write to temp file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)
        .context("Failed to create temp file")?;

    // Set secure permissions BEFORE writing sensitive data (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
            .context("Failed to set temp file permissions")?;
    }

    for line in &lines {
        writeln!(file, "{}", line).context("Failed to write to temp file")?;
    }
    file.sync_all().context("Failed to sync temp file")?;

    // Atomic rename (guard is still active, will clean up if rename fails)
    fs::rename(&temp_path, path).context("Failed to rename temp file to .env")?;

    // Forget the guard - temp file no longer exists after successful rename
    std::mem::forget(_guard);

    // Ensure final file has correct permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .context("Failed to set file permissions")?;
    }

    Ok(())
}

/// Read a secret from a .env file.
///
/// Returns the value wrapped in Zeroizing<String> for automatic memory clearing.
pub fn read_from_dotenv(provider: &str, path: &Path) -> Result<Zeroizing<String>> {
    let env_var = provider_env_var(provider);

    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    let content = fs::read_to_string(path).context("Failed to read .env file")?;

    for line in content.lines() {
        // Skip comments and empty lines
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for our env var
        if let Some(value) = line.strip_prefix(&format!("{}=", env_var)) {
            // Handle quoted values
            let value = value.trim();
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };
            return Ok(Zeroizing::new(value.to_string()));
        }
    }

    anyhow::bail!("Key {} not found in {}", env_var, path.display())
}

/// Delete a secret from a .env file.
///
/// Removes the line containing the key, preserving all other content.
pub fn delete_from_dotenv(provider: &str, path: &Path) -> Result<()> {
    let env_var = provider_env_var(provider);

    if !path.exists() {
        return Ok(()); // Nothing to delete
    }

    let content = fs::read_to_string(path).context("Failed to read .env file")?;
    let lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.starts_with(&format!("{}=", env_var)))
        .collect();

    // Atomic write
    let temp_path = path.with_extension("env.tmp");
    let new_content = lines.join("\n");
    let new_content = if new_content.is_empty() {
        String::new()
    } else {
        format!("{}\n", new_content)
    };

    fs::write(&temp_path, &new_content).context("Failed to write temp file")?;
    fs::rename(&temp_path, path).context("Failed to rename temp file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Store in the global ~/.spn/secrets.env file.
pub fn store_in_global(provider: &str, key: &str) -> Result<()> {
    let path = global_secrets_path()?;
    store_in_dotenv(provider, key, &path)
}

/// Read from the global ~/.spn/secrets.env file.
pub fn read_from_global(provider: &str) -> Result<Zeroizing<String>> {
    let path = global_secrets_path()?;
    read_from_dotenv(provider, &path)
}

/// Delete from the global ~/.spn/secrets.env file.
pub fn delete_from_global(provider: &str) -> Result<()> {
    let path = global_secrets_path()?;
    delete_from_dotenv(provider, &path)
}

/// Check if a .env file is in .gitignore.
///
/// This helps warn users if they're about to store secrets in a file
/// that might be accidentally committed.
pub fn is_gitignored(env_path: &Path) -> bool {
    let gitignore = env_path
        .parent()
        .map(|p| p.join(".gitignore"))
        .filter(|p| p.exists());

    if let Some(gitignore_path) = gitignore {
        if let Ok(content) = fs::read_to_string(gitignore_path) {
            return content.lines().any(|line| {
                let trimmed = line.trim();
                // Common .env gitignore patterns
                trimmed == ".env"
                    || trimmed == "*.env"
                    || trimmed == ".env*"
                    || trimmed == ".env.local"
                    || trimmed == ".env.*.local"
            });
        }
    }
    false
}

/// Check if a .env file exists at the given path.
pub fn dotenv_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Check if the global secrets file exists.
pub fn global_secrets_exists() -> bool {
    global_secrets_path()
        .map(|path| dotenv_exists(&path))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_in_dotenv_creates_file() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        store_in_dotenv("anthropic", "sk-ant-test123", &env_path).unwrap();

        assert!(env_path.exists());
        let content = fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-test123"));
    }

    #[test]
    fn test_store_in_dotenv_appends() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        // Create existing file
        fs::write(&env_path, "EXISTING_VAR=value\n").unwrap();

        store_in_dotenv("anthropic", "sk-ant-test123", &env_path).unwrap();

        let content = fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("EXISTING_VAR=value"));
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-test123"));
    }

    #[test]
    fn test_store_in_dotenv_updates_existing() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        // Create with old value
        fs::write(&env_path, "ANTHROPIC_API_KEY=old-key\nOTHER=value\n").unwrap();

        store_in_dotenv("anthropic", "sk-ant-new123", &env_path).unwrap();

        let content = fs::read_to_string(&env_path).unwrap();
        assert!(!content.contains("old-key"));
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-new123"));
        assert!(content.contains("OTHER=value"));
    }

    #[test]
    fn test_read_from_dotenv() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        fs::write(&env_path, "ANTHROPIC_API_KEY=sk-ant-test123\n").unwrap();

        let key = read_from_dotenv("anthropic", &env_path).unwrap();
        assert_eq!(key.as_str(), "sk-ant-test123");
    }

    #[test]
    fn test_read_from_dotenv_quoted_values() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        // Double quotes
        fs::write(&env_path, "ANTHROPIC_API_KEY=\"sk-ant-test123\"\n").unwrap();
        let key = read_from_dotenv("anthropic", &env_path).unwrap();
        assert_eq!(key.as_str(), "sk-ant-test123");

        // Single quotes
        fs::write(&env_path, "ANTHROPIC_API_KEY='sk-ant-test456'\n").unwrap();
        let key = read_from_dotenv("anthropic", &env_path).unwrap();
        assert_eq!(key.as_str(), "sk-ant-test456");
    }

    #[test]
    fn test_read_from_dotenv_skips_comments() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        fs::write(
            &env_path,
            "# This is a comment\n\nANTHROPIC_API_KEY=sk-ant-test123\n",
        )
        .unwrap();

        let key = read_from_dotenv("anthropic", &env_path).unwrap();
        assert_eq!(key.as_str(), "sk-ant-test123");
    }

    #[test]
    fn test_read_from_dotenv_not_found() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        fs::write(&env_path, "OTHER_KEY=value\n").unwrap();

        let result = read_from_dotenv("anthropic", &env_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_read_from_dotenv_file_missing() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        let result = read_from_dotenv("anthropic", &env_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_delete_from_dotenv() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        fs::write(&env_path, "ANTHROPIC_API_KEY=test\nOTHER=value\n").unwrap();

        delete_from_dotenv("anthropic", &env_path).unwrap();

        let content = fs::read_to_string(&env_path).unwrap();
        assert!(!content.contains("ANTHROPIC_API_KEY"));
        assert!(content.contains("OTHER=value"));
    }

    #[test]
    fn test_delete_from_dotenv_file_missing() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        // Should not error if file doesn't exist
        let result = delete_from_dotenv("anthropic", &env_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_gitignored_true() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        let gitignore_path = temp.path().join(".gitignore");

        fs::write(&gitignore_path, ".env\nnode_modules/\n").unwrap();

        assert!(is_gitignored(&env_path));
    }

    #[test]
    fn test_is_gitignored_false() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        let gitignore_path = temp.path().join(".gitignore");

        fs::write(&gitignore_path, "node_modules/\n").unwrap();

        assert!(!is_gitignored(&env_path));
    }

    #[test]
    fn test_is_gitignored_no_gitignore() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        assert!(!is_gitignored(&env_path));
    }

    #[test]
    fn test_is_gitignored_wildcard_patterns() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        let gitignore_path = temp.path().join(".gitignore");

        // Test *.env pattern
        fs::write(&gitignore_path, "*.env\n").unwrap();
        assert!(is_gitignored(&env_path));

        // Test .env* pattern
        fs::write(&gitignore_path, ".env*\n").unwrap();
        assert!(is_gitignored(&env_path));
    }

    #[test]
    fn test_dotenv_exists() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        assert!(!dotenv_exists(&env_path));

        fs::write(&env_path, "KEY=value\n").unwrap();
        assert!(dotenv_exists(&env_path));
    }

    #[test]
    #[cfg(unix)]
    fn test_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");

        store_in_dotenv("anthropic", "sk-ant-test123", &env_path).unwrap();

        let metadata = fs::metadata(&env_path).unwrap();
        let mode = metadata.permissions().mode();
        // Check that only owner can read/write (0o600)
        assert_eq!(mode & 0o777, 0o600);
    }
}
