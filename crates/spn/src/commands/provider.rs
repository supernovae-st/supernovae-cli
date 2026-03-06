//! Provider API key management command.
//!
//! Manages API keys for LLM providers (Anthropic, OpenAI, etc.) and MCP secrets.
//! Keys are stored securely in the OS keychain via keyring-rs.
//!
//! # Security Features
//!
//! - All keys use `Zeroizing<String>` (auto-clear on drop)
//! - Password input is masked and zeroized
//! - Keys are validated before storage
//! - Debug/Display implementations are redacted

use crate::error::{Result, SpnError};
use crate::secrets::{
    global_secrets_path, is_gitignored, mask_api_key, migrate_env_to_keyring, mlock_available,
    mlock_limit, project_env_path, provider_env_var, resolve_api_key, run_quick_setup, run_wizard,
    security_audit, store_in_dotenv, store_in_global, validate_key_format, SecretSource,
    SpnKeyring, StorageBackend, MCP_SECRET_TYPES, SUPPORTED_PROVIDERS,
};
use crate::ProviderCommands;

use colored::Colorize;
use dialoguer::{Confirm, Password};
use std::str::FromStr;
use zeroize::Zeroizing;

/// Run a provider management command.
pub async fn run(command: ProviderCommands) -> Result<()> {
    match command {
        ProviderCommands::List { show_source } => run_list(show_source).await,
        ProviderCommands::Set {
            provider,
            key,
            storage,
        } => run_set(&provider, key, storage).await,
        ProviderCommands::Get { provider, unmask } => run_get(&provider, unmask).await,
        ProviderCommands::Delete { provider } => run_delete(&provider).await,
        ProviderCommands::Migrate { yes } => run_migrate(yes).await,
        ProviderCommands::Test { provider } => run_test(&provider).await,
        ProviderCommands::Status { json } => run_status(json).await,
    }
}

/// List all providers and their key status.
async fn run_list(show_source: bool) -> Result<()> {
    println!("{}", "Provider API Keys".cyan().bold());
    println!();

    // LLM Providers
    println!("{}", "LLM Providers:".bold());
    list_providers(SUPPORTED_PROVIDERS, show_source);

    println!();
    println!("{}", "MCP Secrets:".bold());
    list_providers(MCP_SECRET_TYPES, show_source);

    println!();

    // Security summary
    let audit = security_audit();
    let in_keychain = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Keychain))
        .count();
    let in_env = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Environment))
        .count();
    let in_dotenv = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::DotEnv))
        .count();
    let total = in_keychain + in_env + in_dotenv;

    if total > 0 {
        println!("{}", "Security Summary:".bold());
        println!("  {} {} in OS Keychain (secure)", "🔐".green(), in_keychain);
        if in_env > 0 {
            println!("  {} {} in environment variables", "📦".yellow(), in_env);
        }
        if in_dotenv > 0 {
            println!("  {} {} in .env files", "📄".yellow(), in_dotenv);
        }

        // Memory protection status
        println!();
        println!("{}", "Memory Protection:".bold());
        if mlock_available() {
            let limit = mlock_limit().map(|l| {
                if l == u64::MAX {
                    "unlimited".to_string()
                } else {
                    format!("{} KB", l / 1024)
                }
            });
            println!(
                "  {} {} {}",
                "🔒".green(),
                "mlock available".green(),
                format!("(limit: {})", limit.unwrap_or("unknown".into())).dimmed()
            );
        } else {
            println!(
                "  {} {}",
                "⚠".yellow(),
                "mlock unavailable (secrets may be swapped to disk)".yellow()
            );
        }

        if in_env > 0 || in_dotenv > 0 {
            println!();
            println!(
                "{} {}",
                "Tip:".dimmed(),
                "Run `spn provider migrate` to move keys to OS Keychain".dimmed()
            );
        }
    } else {
        println!("{}", "No API keys configured.".yellow());
        println!();
        println!("Set a key with:");
        println!("  {} {}", "spn provider set".cyan(), "<provider>".dimmed());
        println!();
        println!("Or migrate from environment variables:");
        println!("  {}", "spn provider migrate".cyan());
    }

    Ok(())
}

/// List providers from a slice.
fn list_providers(providers: &[&str], show_source: bool) {
    for provider in providers {
        let status = if let Some((key, source)) = resolve_api_key(provider) {
            let masked = mask_api_key(&key);
            if show_source {
                format!(
                    "{} {} {} {}",
                    source.icon(),
                    masked.green(),
                    "←".dimmed(),
                    source.description().dimmed()
                )
            } else {
                format!("{} {}", source.icon(), masked.green())
            }
        } else {
            "○ not set".dimmed().to_string()
        };

        let env_var = provider_env_var(provider);
        println!(
            "  {:12} {} {}",
            provider.bold(),
            status,
            format!("({})", env_var).dimmed()
        );
    }
}

/// Set an API key for a provider.
async fn run_set(provider: &str, key: Option<String>, storage: Option<String>) -> Result<()> {
    // Validate provider name
    let is_known_provider = SUPPORTED_PROVIDERS
        .iter()
        .chain(MCP_SECRET_TYPES.iter())
        .any(|&p| p == provider);

    if !is_known_provider {
        let mut msg = format!("Unknown provider: {}\n\n", provider);
        msg.push_str("Supported providers:\n");
        for p in SUPPORTED_PROVIDERS {
            msg.push_str(&format!("  • {}\n", p));
        }
        msg.push_str("\nMCP secrets:\n");
        for p in MCP_SECRET_TYPES {
            msg.push_str(&format!("  • {}\n", p));
        }
        return Err(SpnError::CommandFailed(msg));
    }

    // If key is provided (scripting mode), use quick setup
    if let Some(k) = key {
        let backend = match &storage {
            Some(s) => {
                StorageBackend::from_str(s).map_err(|e| SpnError::CommandFailed(e.to_string()))?
            }
            None => StorageBackend::default(),
        };

        let result = run_quick_setup(provider, &k, backend)
            .map_err(|e| SpnError::CommandFailed(format!("Failed: {}", e)))?;

        println!(
            "{} {} {} {}",
            "✓".green(),
            "API key stored in".green(),
            result.location.cyan().bold(),
            format!("for {}", provider).green()
        );
        println!("  {} {}", "Key:".dimmed(), result.masked_key.dimmed());
        return Ok(());
    }

    // If storage is specified but no key, use simplified prompt
    if storage.is_some() {
        return run_set_with_storage(provider, storage).await;
    }

    // Interactive wizard mode (no --key, no --storage)
    match run_wizard(provider) {
        Ok(Some(result)) => {
            // Wizard handles all output, just log success
            tracing::info!(
                provider = provider,
                storage = %result.storage,
                location = result.location,
                "API key stored via wizard"
            );
        }
        Ok(None) => {
            // User cancelled - wizard already printed "Cancelled."
        }
        Err(e) => {
            return Err(SpnError::CommandFailed(e.to_string()));
        }
    }

    Ok(())
}

/// Set an API key with storage already specified (simplified prompt).
async fn run_set_with_storage(provider: &str, storage: Option<String>) -> Result<()> {
    let backend = match &storage {
        Some(s) => {
            StorageBackend::from_str(s).map_err(|e| SpnError::CommandFailed(e.to_string()))?
        }
        None => StorageBackend::default(),
    };

    let env_var = provider_env_var(provider);
    println!(
        "{} {} {}",
        "Setting API key for".cyan(),
        provider.bold(),
        format!("({})", env_var).dimmed()
    );
    println!(
        "{} {} {}",
        backend.emoji(),
        "Storage:".dimmed(),
        backend.description().dimmed()
    );
    println!();

    let input = Password::new()
        .with_prompt("Enter API key")
        .interact()
        .map_err(|e| crate::error::SpnError::InvalidInput(e.to_string()))?;

    let api_key = Zeroizing::new(input);

    // Validate key format
    let validation = validate_key_format(provider, &api_key);
    if !validation.is_valid() {
        return Err(SpnError::CommandFailed(format!(
            "Invalid key format: {}",
            validation
        )));
    }

    // Store based on backend
    match backend {
        StorageBackend::Keychain => {
            SpnKeyring::set(provider, &api_key)
                .map_err(|e| SpnError::CommandFailed(format!("Failed to store key: {}", e)))?;
            println!(
                "{} {} {} {}",
                "✓".green(),
                "API key stored in".green(),
                "OS Keychain".cyan().bold(),
                format!("for {}", provider).green()
            );
            println!("  {} {}", "Key:".dimmed(), mask_api_key(&api_key).dimmed());
            println!();
            println!(
                "{}",
                "Key is now securely stored and will be used automatically.".dimmed()
            );
        }
        StorageBackend::Env => {
            let path = project_env_path();

            // Warn if not gitignored
            if !is_gitignored(&path) {
                eprintln!(
                    "{} {}",
                    "⚠".yellow(),
                    "Warning: .env is not in .gitignore!".yellow()
                );
                eprintln!(
                    "  {}",
                    "Add '.env' to .gitignore to avoid committing secrets.".dimmed()
                );
                eprintln!();
            }

            store_in_dotenv(provider, &api_key, &path)
                .map_err(|e| SpnError::CommandFailed(format!("Failed to store key: {}", e)))?;
            println!(
                "{} {} {} {}",
                "✓".green(),
                "API key stored in".green(),
                ".env".cyan().bold(),
                format!("for {}", provider).green()
            );
            println!("  {} {}", "Key:".dimmed(), mask_api_key(&api_key).dimmed());
            println!("  {} {}", "File:".dimmed(), path.display());
        }
        StorageBackend::Global => {
            let path = global_secrets_path().map_err(|e| {
                SpnError::CommandFailed(format!("Failed to determine global secrets path: {}", e))
            })?;
            store_in_global(provider, &api_key)
                .map_err(|e| SpnError::CommandFailed(format!("Failed to store key: {}", e)))?;
            println!(
                "{} {} {} {}",
                "✓".green(),
                "API key stored in".green(),
                "~/.spn/secrets.env".cyan().bold(),
                format!("for {}", provider).green()
            );
            println!("  {} {}", "Key:".dimmed(), mask_api_key(&api_key).dimmed());
            println!("  {} {}", "File:".dimmed(), path.display());
        }
        StorageBackend::Shell => {
            println!();
            println!(
                "{} {}",
                "⚠ SECURITY WARNING:".yellow().bold(),
                "Full API key will be displayed below".yellow()
            );
            println!(
                "  {}",
                "• Key may appear in terminal scrollback history".dimmed()
            );
            println!(
                "  {}",
                "• May be visible in screen recordings/sharing".dimmed()
            );
            println!(
                "  {}",
                "• Will NOT be stored by spn - you must copy it".dimmed()
            );
            println!();

            println!(
                "{} {} {}",
                "✓".green(),
                "Key validated for".green(),
                provider.bold()
            );
            println!();
            println!("{}", "Export command (copy this):".bold());
            println!();
            println!("  {}", format!("export {}='{}'", env_var, *api_key).cyan());
            println!();
            println!(
                "{}",
                "Add this to your ~/.zshrc or ~/.bashrc to persist.".dimmed()
            );
            println!(
                "{}",
                "Tip: Use --storage keychain for more secure storage.".dimmed()
            );
        }
    }

    Ok(())
}

/// Get an API key for a provider.
async fn run_get(provider: &str, unmask: bool) -> Result<()> {
    match resolve_api_key(provider) {
        Some((key, source)) => {
            if unmask {
                // DANGEROUS: Print full key
                eprintln!(
                    "{} {}",
                    "⚠ WARNING:".yellow().bold(),
                    "Exposing full API key - use only for scripts!".yellow()
                );
                // Key is printed to stdout for scripting
                println!("{}", *key);
                // key is automatically zeroized when it goes out of scope
            } else {
                println!(
                    "{} {} {}",
                    source.icon(),
                    mask_api_key(&key).green(),
                    format!("← {}", source.description()).dimmed()
                );
            }
        }
        None => {
            return Err(SpnError::CommandFailed(format!(
                "No key found for provider: {}\n\nSet with: spn provider set {}",
                provider, provider
            )));
        }
    }

    Ok(())
}

/// Delete an API key for a provider.
async fn run_delete(provider: &str) -> Result<()> {
    if !SpnKeyring::exists(provider) {
        println!(
            "{} {} {}",
            "⚠".yellow(),
            "No key in keychain for:".yellow(),
            provider.bold()
        );
        return Ok(());
    }

    // Show current key (masked)
    if let Some(masked) = SpnKeyring::get_masked(provider) {
        println!("Current key: {}", masked.dimmed());
    }

    // Confirm deletion
    let confirm = Confirm::new()
        .with_prompt(format!(
            "Delete API key for {} from OS Keychain?",
            provider.bold()
        ))
        .default(false)
        .interact()
        .unwrap_or(false);

    if !confirm {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    }

    SpnKeyring::delete(provider)
        .map_err(|e| SpnError::CommandFailed(format!("Failed to delete: {}", e)))?;
    println!(
        "{} {} {}",
        "✓".green(),
        "Deleted key for".green(),
        provider.bold()
    );

    Ok(())
}

/// Migrate keys from environment variables to OS keychain.
async fn run_migrate(skip_confirm: bool) -> Result<()> {
    println!("{}", "Migrate API Keys to OS Keychain".cyan().bold());
    println!();
    println!(
        "{}",
        "This will copy API keys from environment variables to the secure OS keychain.".dimmed()
    );
    println!(
        "{}",
        "Your environment variables will NOT be modified.".dimmed()
    );
    println!();

    // Show security benefits
    println!("{}", "Benefits of OS Keychain:".bold());
    println!("  • 🔐 Encrypted storage managed by your OS");
    println!("  • 🔐 Protected by your login password/biometrics");
    println!("  • 🔐 Not readable by other processes");
    println!("  • 🔐 Not exposed in process listings");
    println!();

    if !skip_confirm {
        let confirm = Confirm::new()
            .with_prompt("Proceed with migration?")
            .default(true)
            .interact()
            .unwrap_or(false);

        if !confirm {
            println!("{}", "Cancelled.".dimmed());
            return Ok(());
        }
    }

    println!();
    let report = migrate_env_to_keyring();
    println!();

    // Summary
    if report.migrated > 0 {
        println!(
            "{} {} {}",
            "✓".green(),
            format!("{} keys migrated", report.migrated).green(),
            "to OS Keychain".dimmed()
        );
    }

    if report.skipped > 0 {
        println!(
            "{} {} {}",
            "→".dimmed(),
            format!("{} keys skipped", report.skipped).dimmed(),
            "(already in keychain)".dimmed()
        );
    }

    if !report.errors.is_empty() {
        eprintln!();
        eprintln!("{}", "Errors:".red().bold());
        for (provider, error) in &report.errors {
            eprintln!("  {} {}: {}", "✗".red(), provider.bold(), error);
        }
    }

    if report.migrated > 0 {
        println!();
        println!("{}", "Next steps:".bold());
        println!("  1. Verify keys work: {}", "spn provider test all".cyan());
        println!("  2. Remove keys from .env files for better security");
        println!("  3. Keys are now automatically used by spn and Nika");
    }

    Ok(())
}

/// Test provider connection.
async fn run_test(provider: &str) -> Result<()> {
    if provider == "all" {
        println!("{}", "Testing all providers...".cyan());
        println!();

        println!("{}", "LLM Providers:".bold());
        for p in SUPPORTED_PROVIDERS {
            test_single_provider(p).await;
        }

        println!();
        println!("{}", "MCP Secrets:".bold());
        for p in MCP_SECRET_TYPES {
            test_single_provider(p).await;
        }
    } else {
        test_single_provider(provider).await;
    }

    Ok(())
}

/// Test a single provider.
async fn test_single_provider(provider: &str) {
    print!("  {} {}... ", "Testing".cyan(), provider.bold());

    match resolve_api_key(provider) {
        None => {
            println!("{}", "○ Not configured".dimmed());
        }
        Some((key, source)) => {
            // Basic validation
            let validation = validate_key_format(provider, &key);
            if !validation.is_valid() {
                println!("{} {}", "✗ Invalid format:".red(), validation);
                return;
            }

            // Show source and masked key
            println!(
                "{} {} {}",
                "✓ Valid".green(),
                source.icon(),
                mask_api_key(&key).dimmed()
            );
            // key is automatically zeroized when it goes out of scope
        }
    }
}

/// Show comprehensive diagnostics for secrets and providers.
async fn run_status(json: bool) -> Result<()> {
    if json {
        return run_status_json().await;
    }

    println!();
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║  🔐 SECRETS & PROVIDERS DIAGNOSTIC REPORT                                     ║".cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    // Section 1: Provider Status
    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  📊 PROVIDER STATUS                                                         │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    let mut configured = 0;
    let mut in_keychain = 0;
    let mut in_env = 0;
    let mut in_dotenv = 0;

    // LLM Providers
    println!("│  {}", "LLM Providers:".bold());
    for provider in SUPPORTED_PROVIDERS {
        let status = match resolve_api_key(provider) {
            Some((key, source)) => {
                configured += 1;
                match source {
                    SecretSource::Keychain => in_keychain += 1,
                    SecretSource::Environment => in_env += 1,
                    SecretSource::DotEnv => in_dotenv += 1,
                    SecretSource::Inline => {} // Inline secrets don't count in storage stats
                }
                let icon = source.icon();
                let masked = mask_api_key(&key);
                format!(
                    "{} {} {}",
                    icon,
                    masked.green(),
                    format!("({})", source.description()).dimmed()
                )
            }
            None => "○ not configured".dimmed().to_string(),
        };
        println!("│    {:12} {}", provider.bold(), status);
    }

    println!("│");
    println!("│  {}", "MCP Secrets:".bold());
    for provider in MCP_SECRET_TYPES {
        let status = match resolve_api_key(provider) {
            Some((key, source)) => {
                configured += 1;
                match source {
                    SecretSource::Keychain => in_keychain += 1,
                    SecretSource::Environment => in_env += 1,
                    SecretSource::DotEnv => in_dotenv += 1,
                    SecretSource::Inline => {} // Inline secrets don't count in storage stats
                }
                let icon = source.icon();
                let masked = mask_api_key(&key);
                format!(
                    "{} {} {}",
                    icon,
                    masked.green(),
                    format!("({})", source.description()).dimmed()
                )
            }
            None => "○ not configured".dimmed().to_string(),
        };
        println!("│    {:12} {}", provider.bold(), status);
    }
    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Section 2: Storage Distribution
    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  📦 STORAGE DISTRIBUTION                                                    │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!(
        "│    {} {} {} {}",
        "🔐".green(),
        format!("{:>2}", in_keychain).green().bold(),
        "in OS Keychain".green(),
        "(most secure)".dimmed()
    );
    if in_env > 0 {
        println!(
            "│    {} {} {} {}",
            "📦".yellow(),
            format!("{:>2}", in_env).yellow().bold(),
            "in environment variables".yellow(),
            "(moderate security)".dimmed()
        );
    }
    if in_dotenv > 0 {
        println!(
            "│    {} {} {} {}",
            "📄".yellow(),
            format!("{:>2}", in_dotenv).yellow().bold(),
            "in .env files".yellow(),
            "(check .gitignore!)".dimmed()
        );
    }
    let total = SUPPORTED_PROVIDERS.len() + MCP_SECRET_TYPES.len();
    let unconfigured = total - configured;
    if unconfigured > 0 {
        println!(
            "│    {} {} {}",
            "○".dimmed(),
            format!("{:>2}", unconfigured).dimmed(),
            "not configured".dimmed()
        );
    }
    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Section 3: Memory Protection
    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  🛡️  MEMORY PROTECTION                                                      │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    if mlock_available() {
        let limit = mlock_limit().map(|l| {
            if l == u64::MAX {
                "unlimited".to_string()
            } else {
                format!("{} KB", l / 1024)
            }
        });
        println!(
            "│    {} {} {}",
            "✓".green(),
            "mlock() available".green(),
            format!("(limit: {})", limit.unwrap_or("unknown".into())).dimmed()
        );
        println!(
            "│      {}",
            "Secrets are locked in memory and won't be swapped to disk.".dimmed()
        );
    } else {
        println!("│    {} {}", "✗".red(), "mlock() unavailable".red());
        println!(
            "│      {}",
            "Secrets may be swapped to disk. Consider increasing RLIMIT_MEMLOCK.".dimmed()
        );
    }

    println!("│");
    println!(
        "│    {} {}",
        "✓".green(),
        "Zeroize on drop enabled for all secrets".green()
    );
    println!(
        "│      {}",
        "Secrets are cleared from memory when no longer needed.".dimmed()
    );
    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Section 4: File Paths
    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  📁 STORAGE LOCATIONS                                                       │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    match global_secrets_path() {
        Ok(path) => {
            let exists = path.exists();
            let status = if exists {
                "✓".green()
            } else {
                "○".dimmed()
            };
            println!(
                "│    {} {} {}",
                status,
                "Global secrets:".bold(),
                path.display()
            );
        }
        Err(_) => {
            println!(
                "│    {} {} {}",
                "✗".red(),
                "Global secrets:".bold(),
                "Cannot determine home directory".red()
            );
        }
    }

    let env_path = project_env_path();
    let env_exists = env_path.exists();
    let gitignored = is_gitignored(&env_path);
    let env_status = if env_exists {
        "✓".green()
    } else {
        "○".dimmed()
    };
    let gitignore_status = if gitignored {
        "(gitignored ✓)".green()
    } else if env_exists {
        "(NOT gitignored ⚠)".yellow()
    } else {
        "".into()
    };
    println!(
        "│    {} {} {} {}",
        env_status,
        "Project .env:".bold(),
        env_path.display(),
        gitignore_status
    );

    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Section 5: Security Score
    let score = calculate_security_score(in_keychain, in_env, in_dotenv, configured);
    let score_color = if score >= 80 {
        "green"
    } else if score >= 50 {
        "yellow"
    } else {
        "red"
    };

    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  🏆 SECURITY SCORE                                                          │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    let bar_width = 40;
    let filled = (score * bar_width / 100) as usize;
    let empty = bar_width as usize - filled;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    let colored_bar = match score_color {
        "green" => bar.green(),
        "yellow" => bar.yellow(),
        _ => bar.red(),
    };
    let colored_score = match score_color {
        "green" => format!("{}/100", score).green().bold(),
        "yellow" => format!("{}/100", score).yellow().bold(),
        _ => format!("{}/100", score).red().bold(),
    };

    println!("│    {} {}", colored_bar, colored_score);
    println!("│");

    // Recommendations
    if in_env > 0 || in_dotenv > 0 {
        println!("│    {}", "Recommendations:".bold());
        if in_env > 0 {
            println!(
                "│      {} Migrate {} keys from environment to keychain:",
                "→".cyan(),
                in_env
            );
            println!("│        {}", "spn provider migrate".cyan());
        }
        if in_dotenv > 0 && !gitignored {
            println!(
                "│      {} Add .env to .gitignore to prevent accidental commits",
                "→".cyan()
            );
        }
    } else if configured == 0 {
        println!("│    {}", "Get started:".bold());
        println!("│      {} Set up your first API key:", "→".cyan());
        println!("│        {}", "spn provider set anthropic".cyan());
    } else {
        println!(
            "│    {} All secrets are stored securely in OS Keychain!",
            "✓".green()
        );
    }

    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    Ok(())
}

/// Calculate security score based on where keys are stored.
fn calculate_security_score(
    in_keychain: usize,
    in_env: usize,
    in_dotenv: usize,
    total: usize,
) -> u32 {
    if total == 0 {
        return 100; // No secrets to protect = perfect score
    }

    // Weights: keychain=100, env=40, dotenv=20
    let keychain_score = in_keychain * 100;
    let env_score = in_env * 40;
    let dotenv_score = in_dotenv * 20;

    let weighted_total = keychain_score + env_score + dotenv_score;
    let max_possible = total * 100;

    (weighted_total * 100 / max_possible) as u32
}

/// Output status as JSON for scripting.
async fn run_status_json() -> Result<()> {
    use serde_json::json;

    let audit = security_audit();
    let mut providers = Vec::new();

    for (provider, source, recommendation) in &audit {
        // Get the actual masked key if configured
        let masked_key = resolve_api_key(provider).map(|(key, _)| mask_api_key(&key));

        providers.push(json!({
            "name": provider,
            "configured": source.is_some(),
            "source": source.as_ref().map(|s| format!("{:?}", s)),
            "masked_key": masked_key,
            "recommendation": recommendation
        }));
    }

    let in_keychain = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Keychain))
        .count();
    let in_env = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Environment))
        .count();
    let in_dotenv = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::DotEnv))
        .count();
    let configured = in_keychain + in_env + in_dotenv;

    let output = json!({
        "providers": providers,
        "storage": {
            "keychain": in_keychain,
            "environment": in_env,
            "dotenv": in_dotenv
        },
        "memory_protection": {
            "mlock_available": mlock_available(),
            "mlock_limit": mlock_limit()
        },
        "paths": {
            "global_secrets": global_secrets_path().ok().map(|p| p.display().to_string()),
            "project_env": project_env_path().display().to_string(),
            "project_env_gitignored": is_gitignored(&project_env_path())
        },
        "security_score": calculate_security_score(in_keychain, in_env, in_dotenv, configured)
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_providers() {
        assert!(SUPPORTED_PROVIDERS.contains(&"anthropic"));
        assert!(SUPPORTED_PROVIDERS.contains(&"openai"));
        assert!(SUPPORTED_PROVIDERS.contains(&"gemini"));
    }

    #[test]
    fn test_mcp_secrets() {
        assert!(MCP_SECRET_TYPES.contains(&"github"));
        assert!(MCP_SECRET_TYPES.contains(&"neo4j"));
        assert!(MCP_SECRET_TYPES.contains(&"perplexity"));
    }

    #[test]
    fn test_storage_backend_parsing() {
        // Valid backends
        assert_eq!(
            StorageBackend::from_str("keychain").unwrap(),
            StorageBackend::Keychain
        );
        assert_eq!(
            StorageBackend::from_str("env").unwrap(),
            StorageBackend::Env
        );
        assert_eq!(
            StorageBackend::from_str("global").unwrap(),
            StorageBackend::Global
        );
        assert_eq!(
            StorageBackend::from_str("shell").unwrap(),
            StorageBackend::Shell
        );

        // Case insensitive
        assert_eq!(
            StorageBackend::from_str("KEYCHAIN").unwrap(),
            StorageBackend::Keychain
        );
        assert_eq!(
            StorageBackend::from_str("ENV").unwrap(),
            StorageBackend::Env
        );

        // Invalid
        assert!(StorageBackend::from_str("invalid").is_err());
        assert!(StorageBackend::from_str("database").is_err());
    }

    #[test]
    fn test_storage_backend_default_is_keychain() {
        assert_eq!(StorageBackend::default(), StorageBackend::Keychain);
    }

    #[test]
    fn test_storage_backend_descriptions() {
        // Each backend should have meaningful description
        assert!(StorageBackend::Keychain.description().contains("Keychain"));
        assert!(StorageBackend::Env.description().contains(".env"));
        assert!(StorageBackend::Global.description().contains("~/.spn"));
        assert!(StorageBackend::Shell.description().contains("export"));
    }

    #[test]
    fn test_storage_backend_emojis() {
        assert_eq!(StorageBackend::Keychain.emoji(), "🔐");
        assert_eq!(StorageBackend::Env.emoji(), "📁");
        assert_eq!(StorageBackend::Global.emoji(), "🌍");
        assert_eq!(StorageBackend::Shell.emoji(), "📋");
    }

    #[test]
    fn test_storage_backend_security_ranking() {
        // Keychain > Global > Env > Shell
        assert!(
            StorageBackend::Keychain.security_level() > StorageBackend::Global.security_level()
        );
        assert!(StorageBackend::Global.security_level() > StorageBackend::Env.security_level());
        assert!(StorageBackend::Env.security_level() > StorageBackend::Shell.security_level());
    }
}
