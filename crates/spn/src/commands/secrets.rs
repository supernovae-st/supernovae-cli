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

use colored::Colorize;
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
        "╔═══════════════════════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║  🩺 SECRETS DOCTOR                                                            ║".cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    let mut checks: Vec<CheckResult> = Vec::new();
    let mut warnings = 0;
    let mut errors = 0;

    // Check 1: Keychain Access
    println!("{}", "Checking keychain access...".dimmed());
    match check_keychain_access() {
        Ok(result) => checks.push(result),
        Err(e) => checks.push(CheckResult::Error(format!("Keychain check failed: {}", e))),
    }

    // Check 2: Global secrets file permissions
    println!("{}", "Checking file permissions...".dimmed());
    match check_file_permissions() {
        Ok(result) => checks.push(result),
        Err(e) => checks.push(CheckResult::Error(format!(
            "Permission check failed: {}",
            e
        ))),
    }

    // Check 3: .gitignore for .env
    println!("{}", "Checking .gitignore...".dimmed());
    checks.push(check_gitignore());

    // Check 4: Memory protection
    println!("{}", "Checking memory protection...".dimmed());
    checks.push(check_memory_protection());

    // Check 5: Key format validation
    println!("{}", "Validating key formats...".dimmed());
    checks.extend(check_key_formats());

    // Check 6: Duplicate keys
    println!("{}", "Checking for duplicate keys...".dimmed());
    checks.extend(check_duplicate_keys());

    println!();
    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  📋 HEALTH CHECK RESULTS                                                    │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    for check in &checks {
        let icon = match check {
            CheckResult::Pass(_) => check.icon().green(),
            CheckResult::Warning(_) => {
                warnings += 1;
                check.icon().yellow()
            }
            CheckResult::Error(_) => {
                errors += 1;
                check.icon().red()
            }
            CheckResult::Info(_) => check.icon().blue(),
        };

        let message = match check {
            CheckResult::Pass(m) => m.green().to_string(),
            CheckResult::Warning(m) => m.yellow().to_string(),
            CheckResult::Error(m) => m.red().to_string(),
            CheckResult::Info(m) => m.blue().to_string(),
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
            "✓".green().bold(),
            "All health checks passed!".green().bold()
        );
    } else {
        println!(
            "{} {} ({} warning{}, {} error{})",
            "Issues found:".bold(),
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
                "Tip:".dimmed(),
                "Run `spn secrets doctor --fix` to auto-fix where possible.".dimmed()
            );
        }
    }

    // Auto-fix if requested
    if fix && total_issues > 0 {
        println!();
        println!("{}", "Attempting auto-fix...".cyan());
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
                println!("  {} Adding .env to .gitignore...", "→".cyan());

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
                    println!("    {} Added .env to .gitignore", "✓".green());
                }
            }
            CheckResult::Warning(msg) if msg.contains("permissions") => {
                println!("  {} Fixing file permissions...", "→".cyan());

                if let Ok(path) = global_secrets_path() {
                    if path.exists() {
                        let mut perms = fs::metadata(&path)?.permissions();
                        perms.set_mode(0o600);
                        fs::set_permissions(&path, perms)?;
                        println!("    {} Set permissions to 600", "✓".green());
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
            "╭─────────────────────────────────────────────────────────────────────────────╮".red()
        );
        println!(
            "{}",
            "│  ⚠️  WARNING: PLAINTEXT EXPORT                                              │".red()
        );
        println!(
            "{}",
            "│                                                                             │".red()
        );
        println!(
            "{}",
            "│  You are about to export secrets in PLAINTEXT. This is dangerous!          │".red()
        );
        println!(
            "{}",
            "│  The output will contain unencrypted API keys.                             │".red()
        );
        println!(
            "{}",
            "│                                                                             │".red()
        );
        println!(
            "{}",
            "│  Only use this for:                                                        │".red()
        );
        println!(
            "{}",
            "│    • Migrating to another machine you control                              │".red()
        );
        println!(
            "{}",
            "│    • Backup to encrypted storage                                           │".red()
        );
        println!(
            "{}",
            "╰─────────────────────────────────────────────────────────────────────────────╯".red()
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
        println!("{}", "No secrets configured to export.".yellow());
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
                    "✓".green(),
                    export.get("providers").map(|p| p.len()).unwrap_or(0),
                    path.cyan()
                );
                println!();
                println!(
                    "{} {}",
                    "⚠".yellow(),
                    "Remember to delete this file after importing!".yellow()
                );
            }
            None => {
                println!("{}", yaml);
            }
        }
    } else {
        // SOPS encryption
        println!("{}", "SOPS encryption not yet implemented.".yellow());
        println!();
        println!("For now, use {} to export plaintext:", "--plaintext".cyan());
        println!(
            "  {} {}",
            "spn secrets export --plaintext -o".cyan(),
            "secrets.yaml".dimmed()
        );
        println!();
        println!("Then encrypt with SOPS:");
        println!(
            "  {} {}",
            "sops encrypt".cyan(),
            "secrets.yaml > secrets.enc.yaml".dimmed()
        );
    }

    Ok(())
}

/// Import secrets from encrypted file.
async fn run_import(file: &str, yes: bool) -> Result<()> {
    use serde_yaml;
    use std::collections::BTreeMap;

    // Check if file exists
    if !std::path::Path::new(file).exists() {
        eprintln!("{} File not found: {}", "✗".red(), file);
        std::process::exit(1);
    }

    // Read and parse
    let content = fs::read_to_string(file)?;

    // Try to parse as YAML
    let data: BTreeMap<String, BTreeMap<String, String>> = serde_yaml::from_str(&content)?;

    let providers = match data.get("providers") {
        Some(p) => p,
        None => {
            eprintln!("{} Invalid format: missing 'providers' key", "✗".red());
            std::process::exit(1);
        }
    };

    println!();
    println!(
        "{}",
        "╭─────────────────────────────────────────────────────────────────────────────╮".cyan()
    );
    println!(
        "{}",
        "│  📥 SECRETS IMPORT                                                          │".cyan()
    );
    println!(
        "{}",
        "├─────────────────────────────────────────────────────────────────────────────┤".cyan()
    );

    println!("│  Found {} secrets to import:", providers.len());
    for (provider, value) in providers {
        // Check if it's encrypted (starts with ENC[)
        let is_encrypted = value.starts_with("ENC[");
        let status = if is_encrypted {
            "encrypted".yellow()
        } else {
            "plaintext".green()
        };
        println!("│    {} {} ({})", "•".dimmed(), provider.bold(), status);
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
            println!("{}", "Cancelled.".dimmed());
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
                "⚠".yellow(),
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
                    "✓".green(),
                    provider.bold()
                );
                imported += 1;
            }
            Err(e) => {
                println!("  {} {}: Failed - {}", "✗".red(), provider.bold(), e);
            }
        }
    }

    println!();
    println!(
        "{} Imported {} secrets, skipped {}",
        "Summary:".bold(),
        imported,
        skipped
    );

    if skipped > 0 {
        println!();
        println!(
            "{} {}",
            "Tip:".dimmed(),
            "For encrypted files, run `sops decrypt file.yaml` first.".dimmed()
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
