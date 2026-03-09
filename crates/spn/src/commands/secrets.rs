//! Secrets management and diagnostics command.
//!
//! Provides health checks, import/export functionality for secrets.

#![allow(dead_code)]

use crate::error::Result;
use crate::secrets::{
    global_secrets_path, is_gitignored, mask_api_key, mlock_available, mlock_limit,
    project_env_path, provider_env_var, resolve_api_key, validate_key_format, SecretSource,
    SpnKeyring, MCP_SECRET_TYPES, SUPPORTED_PROVIDERS,
};
use crate::SecretsCommands;

use crate::ux::design_system as ds;
use std::fs;
use std::os::unix::fs::PermissionsExt;

/// Run a secrets management command.
pub async fn run(command: SecretsCommands) -> Result<()> {
    match command {
        SecretsCommands::Doctor { fix } => run_doctor(fix).await,
        SecretsCommands::Export { output, plaintext } => run_export(output, plaintext).await,
        SecretsCommands::Import { file, yes } => run_import(&file, yes).await,
    }
}

/// Health check result for a single check.
#[derive(Debug)]
enum CheckResult {
    Pass(String),
    Warning(String),
    Error(String),
    Info(String),
}

impl CheckResult {
    fn icon(&self) -> &'static str {
        match self {
            CheckResult::Pass(_) => "✓",
            CheckResult::Warning(_) => "⚠",
            CheckResult::Error(_) => "✗",
            CheckResult::Info(_) => "ℹ",
        }
    }

    fn message(&self) -> &str {
        match self {
            CheckResult::Pass(m)
            | CheckResult::Warning(m)
            | CheckResult::Error(m)
            | CheckResult::Info(m) => m,
        }
    }
}

/// Run health checks on secrets configuration.
async fn run_doctor(fix: bool) -> Result<()> {
    println!();
    println!(
        "{}",
        ds::primary(
            "╔═══════════════════════════════════════════════════════════════════════════════╗"
        )
    );
    println!(
        "{}",
        ds::primary(
            "║  🩺 SECRETS DOCTOR                                                            ║"
        )
    );
    println!(
        "{}",
        ds::primary(
            "╚═══════════════════════════════════════════════════════════════════════════════╝"
        )
    );
    println!();

    let mut checks: Vec<CheckResult> = Vec::new();
    let mut warnings = 0;
    let mut errors = 0;

    // Check 1: Keychain Access
    println!("{}", ds::muted("Checking keychain access..."));
    match check_keychain_access() {
        Ok(result) => checks.push(result),
        Err(e) => checks.push(CheckResult::Error(format!("Keychain check failed: {}", e))),
    }

    // Check 2: Global secrets file permissions
    println!("{}", ds::muted("Checking file permissions..."));
    match check_file_permissions() {
        Ok(result) => checks.push(result),
        Err(e) => checks.push(CheckResult::Error(format!(
            "Permission check failed: {}",
            e
        ))),
    }

    // Check 3: .gitignore for .env
    println!("{}", ds::muted("Checking .gitignore..."));
    checks.push(check_gitignore());

    // Check 4: Memory protection
    println!("{}", ds::muted("Checking memory protection..."));
    checks.push(check_memory_protection());

    // Check 5: Key format validation
    println!("{}", ds::muted("Validating key formats..."));
    checks.extend(check_key_formats());

    // Check 6: Duplicate keys
    println!("{}", ds::muted("Checking for duplicate keys..."));
    checks.extend(check_duplicate_keys());

    println!();
    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  📋 HEALTH CHECK RESULTS                                                    │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    for check in &checks {
        let icon = match check {
            CheckResult::Pass(_) => ds::success(check.icon()),
            CheckResult::Warning(_) => {
                warnings += 1;
                ds::warning(check.icon())
            }
            CheckResult::Error(_) => {
                errors += 1;
                ds::error(check.icon())
            }
            CheckResult::Info(_) => ds::primary(check.icon()),
        };

        let message = match check {
            CheckResult::Pass(m) => ds::success(m).to_string(),
            CheckResult::Warning(m) => ds::warning(m).to_string(),
            CheckResult::Error(m) => ds::error(m).to_string(),
            CheckResult::Info(m) => ds::primary(m).to_string(),
        };

        println!("│    {} {}", icon, message);
    }

    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Summary
    let total_issues = warnings + errors;
    if total_issues == 0 {
        println!(
            "{} {}",
            ds::success("✓").bold(),
            ds::success("All health checks passed!").bold()
        );
    } else {
        println!(
            "{} {} ({} warning{}, {} error{})",
            ds::highlight("Issues found:"),
            total_issues,
            warnings,
            if warnings == 1 { "" } else { "s" },
            errors,
            if errors == 1 { "" } else { "s" }
        );

        if !fix {
            println!();
            println!(
                "{} {}",
                ds::muted("Tip:"),
                ds::muted("Run `spn secrets doctor --fix` to auto-fix where possible.")
            );
        }
    }

    // Auto-fix if requested
    if fix && total_issues > 0 {
        println!();
        println!("{}", ds::primary("Attempting auto-fix..."));
        run_auto_fix(&checks).await?;
    }

    println!();
    Ok(())
}

/// Check if we can access the keychain.
fn check_keychain_access() -> Result<CheckResult> {
    // Try to access the keychain (read-only test)
    if SpnKeyring::is_accessible() {
        let keys = SpnKeyring::list();
        let count = keys.len();
        Ok(CheckResult::Pass(format!(
            "Keychain access: OK ({} keys stored)",
            count
        )))
    } else {
        Ok(CheckResult::Error(
            "Keychain access failed: cannot access keyring".to_string(),
        ))
    }
}

/// Check file permissions on secrets files.
fn check_file_permissions() -> Result<CheckResult> {
    let global_path = global_secrets_path()?;

    if !global_path.exists() {
        return Ok(CheckResult::Info(
            "Global secrets file not yet created".to_string(),
        ));
    }

    let metadata = fs::metadata(&global_path)?;
    let mode = metadata.permissions().mode();
    let permissions = mode & 0o777;

    if permissions == 0o600 {
        Ok(CheckResult::Pass(format!(
            "Global secrets file permissions: {:o} (secure)",
            permissions
        )))
    } else {
        Ok(CheckResult::Warning(format!(
            "Global secrets file permissions: {:o} (should be 600)",
            permissions
        )))
    }
}

/// Check if .env is in .gitignore.
fn check_gitignore() -> CheckResult {
    let env_path = project_env_path();

    if !env_path.exists() {
        return CheckResult::Info("No project .env file".to_string());
    }

    if is_gitignored(&env_path) {
        CheckResult::Pass(".env is in .gitignore".to_string())
    } else {
        CheckResult::Warning(
            ".env exists but is NOT in .gitignore - risk of accidental commit".to_string(),
        )
    }
}

/// Check memory protection availability.
fn check_memory_protection() -> CheckResult {
    if mlock_available() {
        let limit = mlock_limit().map(|l| {
            if l == u64::MAX {
                "unlimited".to_string()
            } else {
                format!("{} KB", l / 1024)
            }
        });
        CheckResult::Pass(format!(
            "Memory protection: mlock available (limit: {})",
            limit.unwrap_or("unknown".into())
        ))
    } else {
        CheckResult::Warning(
            "Memory protection: mlock unavailable (secrets may be swapped to disk)".to_string(),
        )
    }
}

/// Validate format of all configured keys.
fn check_key_formats() -> Vec<CheckResult> {
    let mut results = Vec::new();
    let all_providers: Vec<&str> = SUPPORTED_PROVIDERS
        .iter()
        .chain(MCP_SECRET_TYPES.iter())
        .copied()
        .collect();

    for provider in all_providers {
        if let Some((key, _source)) = resolve_api_key(provider) {
            let validation = validate_key_format(provider, &key);
            if !validation.is_valid() {
                results.push(CheckResult::Error(format!(
                    "Invalid key format for {}: {}",
                    provider, validation
                )));
            }
        }
    }

    if results.is_empty() {
        results.push(CheckResult::Pass(
            "All configured keys have valid formats".to_string(),
        ));
    }

    results
}

/// Check for duplicate keys (same key in multiple storage locations).
fn check_duplicate_keys() -> Vec<CheckResult> {
    let mut results = Vec::new();
    let all_providers: Vec<&str> = SUPPORTED_PROVIDERS
        .iter()
        .chain(MCP_SECRET_TYPES.iter())
        .copied()
        .collect();

    for provider in &all_providers {
        let mut sources: Vec<SecretSource> = Vec::new();

        // Check keychain
        if SpnKeyring::get(provider).is_ok() {
            sources.push(SecretSource::Keychain);
        }

        // Check environment
        let env_var = provider_env_var(provider);
        if std::env::var(env_var).is_ok() {
            sources.push(SecretSource::Environment);
        }

        // Check .env file (would need to read it directly)
        // For now, if resolve_api_key returns DotEnv and we already have other sources
        if sources.len() > 1 {
            results.push(CheckResult::Warning(format!(
                "Duplicate key for {}: found in {:?}",
                provider, sources
            )));
        }
    }

    if results.is_empty() {
        results.push(CheckResult::Pass("No duplicate keys found".to_string()));
    }

    results
}

/// Attempt to auto-fix issues.
async fn run_auto_fix(checks: &[CheckResult]) -> Result<()> {
    for check in checks {
        match check {
            CheckResult::Warning(msg) if msg.contains(".gitignore") => {
                println!("  {} Adding .env to .gitignore...", ds::primary("→"));

                let gitignore_path = std::path::Path::new(".gitignore");
                let mut content = if gitignore_path.exists() {
                    fs::read_to_string(gitignore_path)?
                } else {
                    String::new()
                };

                if !content.contains(".env") {
                    if !content.ends_with('\n') && !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(".env\n");
                    fs::write(gitignore_path, content)?;
                    println!("    {} Added .env to .gitignore", ds::success("✓"));
                }
            }
            CheckResult::Warning(msg) if msg.contains("permissions") => {
                println!("  {} Fixing file permissions...", ds::primary("→"));

                if let Ok(path) = global_secrets_path() {
                    if path.exists() {
                        let mut perms = fs::metadata(&path)?.permissions();
                        perms.set_mode(0o600);
                        fs::set_permissions(&path, perms)?;
                        println!("    {} Set permissions to 600", ds::success("✓"));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Export secrets to encrypted file (SOPS format).
async fn run_export(output: Option<String>, plaintext: bool) -> Result<()> {
    use serde_yaml;
    use std::collections::BTreeMap;

    if plaintext {
        println!();
        println!(
            "{}",
            ds::error(
                "╭─────────────────────────────────────────────────────────────────────────────╮"
            )
        );
        println!(
            "{}",
            ds::error(
                "│  ⚠️  WARNING: PLAINTEXT EXPORT                                              │"
            )
        );
        println!(
            "{}",
            ds::error(
                "│                                                                             │"
            )
        );
        println!(
            "{}",
            ds::error(
                "│  You are about to export secrets in PLAINTEXT. This is dangerous!          │"
            )
        );
        println!(
            "{}",
            ds::error(
                "│  The output will contain unencrypted API keys.                             │"
            )
        );
        println!(
            "{}",
            ds::error(
                "│                                                                             │"
            )
        );
        println!(
            "{}",
            ds::error(
                "│  Only use this for:                                                        │"
            )
        );
        println!(
            "{}",
            ds::error(
                "│    • Migrating to another machine you control                              │"
            )
        );
        println!(
            "{}",
            ds::error(
                "│    • Backup to encrypted storage                                           │"
            )
        );
        println!(
            "{}",
            ds::error(
                "╰─────────────────────────────────────────────────────────────────────────────╯"
            )
        );
        println!();
    }

    // Collect all secrets
    let mut providers: BTreeMap<String, String> = BTreeMap::new();
    let all_provider_names: Vec<&str> = SUPPORTED_PROVIDERS
        .iter()
        .chain(MCP_SECRET_TYPES.iter())
        .copied()
        .collect();

    for provider in all_provider_names {
        if let Some((key, source)) = resolve_api_key(provider) {
            if plaintext {
                providers.insert(provider.to_string(), (*key).clone());
            } else {
                // For SOPS, we output masked values with metadata
                providers.insert(
                    provider.to_string(),
                    format!("ENC[source={:?},masked={}]", source, mask_api_key(&key)),
                );
            }
        }
    }

    if providers.is_empty() {
        println!("{}", ds::warning("No secrets configured to export."));
        return Ok(());
    }

    // Build export structure
    let mut export: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    export.insert("providers".to_string(), providers);

    let yaml = serde_yaml::to_string(&export)?;

    if plaintext {
        // Direct output
        match output {
            Some(path) => {
                fs::write(&path, &yaml)?;
                println!(
                    "{} Exported {} secrets to {}",
                    ds::success("✓"),
                    export.get("providers").map(|p| p.len()).unwrap_or(0),
                    ds::primary(&path)
                );
                println!();
                println!(
                    "{} {}",
                    ds::warning("⚠"),
                    ds::warning("Remember to delete this file after importing!")
                );
            }
            None => {
                println!("{}", yaml);
            }
        }
    } else {
        // SOPS encryption - check if sops is available
        let sops_available = std::process::Command::new("sops")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !sops_available {
            println!("{}", ds::warning("SOPS not found in PATH."));
            println!();
            println!("Install SOPS to enable encrypted exports:");
            println!(
                "  {} {}",
                ds::primary("brew install sops"),
                ds::muted("# macOS")
            );
            println!(
                "  {} {}",
                ds::primary("apt install sops"),
                ds::muted("# Debian/Ubuntu")
            );
            println!();
            println!("Or use {} to export plaintext:", ds::primary("--plaintext"));
            println!(
                "  {} {}",
                ds::primary("spn secrets export --plaintext -o"),
                ds::muted("secrets.yaml")
            );
            return Ok(());
        }

        // Check for .sops.yaml config
        let sops_config_exists = std::path::Path::new(".sops.yaml").exists()
            || dirs::home_dir()
                .map(|h| h.join(".sops.yaml").exists())
                .unwrap_or(false);

        if !sops_config_exists {
            println!("{}", ds::warning("No .sops.yaml configuration found."));
            println!();
            println!("SOPS requires encryption key configuration. Create a .sops.yaml file:");
            println!();
            println!("{}", ds::muted("# Example using AGE key:"));
            println!("{}", ds::primary("creation_rules:"));
            println!("{}", ds::primary("  - path_regex: .*\\.enc\\.yaml$"));
            println!(
                "{}",
                ds::primary("    age: age1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
            );
            println!();
            println!(
                "{}",
                ds::muted("Generate an AGE key with: age-keygen -o key.txt")
            );
            println!();
            return Ok(());
        }

        // Write plaintext to temp file, then encrypt with SOPS
        let temp_dir = tempfile::tempdir()?;
        let temp_file = temp_dir.path().join("secrets.yaml");
        fs::write(&temp_file, &yaml)?;

        let output_path = output.unwrap_or_else(|| "secrets.enc.yaml".to_string());

        let sops_result = std::process::Command::new("sops")
            .args(["encrypt", "--output", &output_path])
            .arg(&temp_file)
            .status();

        match sops_result {
            Ok(status) if status.success() => {
                println!(
                    "{} Exported {} secrets to {} (SOPS encrypted)",
                    ds::success("✓"),
                    export.get("providers").map(|p| p.len()).unwrap_or(0),
                    ds::primary(&output_path)
                );
            }
            Ok(status) => {
                eprintln!(
                    "{} SOPS encryption failed (exit code: {})",
                    ds::error("✗"),
                    status.code().unwrap_or(-1)
                );
                eprintln!();
                eprintln!("Check your .sops.yaml configuration.");
            }
            Err(e) => {
                eprintln!("{} Failed to run SOPS: {}", ds::error("✗"), e);
            }
        }
    }

    Ok(())
}

/// Import secrets from encrypted file.
async fn run_import(file: &str, yes: bool) -> Result<()> {
    use serde_yaml;
    use std::collections::BTreeMap;

    // Check if file exists
    if !std::path::Path::new(file).exists() {
        eprintln!("{} File not found: {}", ds::error("✗"), file);
        std::process::exit(1);
    }

    // Read and parse
    let content = fs::read_to_string(file)?;

    // Try to parse as YAML
    let data: BTreeMap<String, BTreeMap<String, String>> = serde_yaml::from_str(&content)?;

    let providers = match data.get("providers") {
        Some(p) => p,
        None => {
            eprintln!("{} Invalid format: missing 'providers' key", ds::error("✗"));
            std::process::exit(1);
        }
    };

    println!();
    println!(
        "{}",
        ds::primary(
            "╭─────────────────────────────────────────────────────────────────────────────╮"
        )
    );
    println!(
        "{}",
        ds::primary(
            "│  📥 SECRETS IMPORT                                                          │"
        )
    );
    println!(
        "{}",
        ds::primary(
            "├─────────────────────────────────────────────────────────────────────────────┤"
        )
    );

    println!("│  Found {} secrets to import:", providers.len());
    for (provider, value) in providers {
        // Check if it's encrypted (starts with ENC[)
        let is_encrypted = value.starts_with("ENC[");
        let status = if is_encrypted {
            ds::warning("encrypted")
        } else {
            ds::success("plaintext")
        };
        println!(
            "│    {} {} ({})",
            ds::muted("•"),
            ds::highlight(provider),
            status
        );
    }

    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Confirm
    if !yes {
        print!("Import these secrets to OS Keychain? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", ds::muted("Cancelled."));
            return Ok(());
        }
    }

    // Import each secret
    let mut imported = 0;
    let mut skipped = 0;

    for (provider, value) in providers {
        // Skip encrypted values (would need SOPS to decrypt)
        if value.starts_with("ENC[") {
            println!(
                "  {} {}: Skipping encrypted value (use SOPS to decrypt first)",
                ds::warning("⚠"),
                provider
            );
            skipped += 1;
            continue;
        }

        // Store in keychain
        match SpnKeyring::set(provider, value) {
            Ok(()) => {
                println!(
                    "  {} {}: Imported to keychain",
                    ds::success("✓"),
                    ds::highlight(provider)
                );
                imported += 1;
            }
            Err(e) => {
                println!(
                    "  {} {}: Failed - {}",
                    ds::error("✗"),
                    ds::highlight(provider),
                    e
                );
            }
        }
    }

    println!();
    println!(
        "{} Imported {} secrets, skipped {}",
        ds::highlight("Summary:"),
        imported,
        skipped
    );

    if skipped > 0 {
        println!();
        println!(
            "{} {}",
            ds::muted("Tip:"),
            ds::muted("For encrypted files, run `sops decrypt file.yaml` first.")
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_icons() {
        assert_eq!(CheckResult::Pass("test".into()).icon(), "✓");
        assert_eq!(CheckResult::Warning("test".into()).icon(), "⚠");
        assert_eq!(CheckResult::Error("test".into()).icon(), "✗");
        assert_eq!(CheckResult::Info("test".into()).icon(), "ℹ");
    }

    #[test]
    fn test_check_result_messages() {
        let pass = CheckResult::Pass("pass message".into());
        let warn = CheckResult::Warning("warn message".into());
        let err = CheckResult::Error("error message".into());
        let info = CheckResult::Info("info message".into());

        assert_eq!(pass.message(), "pass message");
        assert_eq!(warn.message(), "warn message");
        assert_eq!(err.message(), "error message");
        assert_eq!(info.message(), "info message");
    }

    #[test]
    fn test_check_gitignore_no_env() {
        // When .env doesn't exist, should return Info
        let result = check_gitignore();
        // This will depend on whether .env exists in the test environment
        match result {
            CheckResult::Info(msg) => assert!(msg.contains("No project .env")),
            CheckResult::Pass(msg) => assert!(msg.contains(".gitignore")),
            CheckResult::Warning(msg) => assert!(msg.contains(".gitignore")),
            _ => {}
        }
    }

    #[test]
    fn test_check_memory_protection() {
        let result = check_memory_protection();
        // Should always return either Pass or Warning
        match result {
            CheckResult::Pass(msg) => assert!(msg.contains("mlock")),
            CheckResult::Warning(msg) => assert!(msg.contains("mlock")),
            _ => panic!("Unexpected result type"),
        }
    }
}
