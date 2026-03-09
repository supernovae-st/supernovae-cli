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
use crate::prompts;
use crate::secrets::{
    global_secrets_path, is_gitignored, mask_api_key, migrate_env_to_keyring, mlock_available,
    mlock_limit, project_env_path, provider_env_var, resolve_api_key, run_quick_setup, run_wizard,
    security_audit, store_in_dotenv, store_in_global, validate_key_format, SecretSource,
    SpnKeyring, StorageBackend, MCP_SECRET_TYPES, SUPPORTED_PROVIDERS,
};
use crate::ux::design_system as ds;
use crate::ProviderCommands;

use dialoguer::{Confirm, Password};
#[cfg(unix)]
use spn_client::SpnClient;
use std::io::IsTerminal;
use std::str::FromStr;
#[cfg(unix)]
use tracing::debug;
use zeroize::Zeroizing;

/// Notify the daemon to refresh its cached secret for a provider.
///
/// This is called after storing a secret in the keychain to ensure the daemon
/// has the latest value. If the daemon is not running, this is a no-op.
#[cfg(unix)]
async fn notify_daemon_refresh(provider: &str) {
    match SpnClient::connect().await {
        Ok(mut client) => match client.refresh_secret(provider).await {
            Ok(refreshed) => {
                if refreshed {
                    debug!("Daemon cache refreshed for {}", provider);
                } else {
                    debug!("Provider {} not in daemon cache", provider);
                }
            }
            Err(e) => {
                debug!("Failed to refresh daemon cache for {}: {}", provider, e);
            }
        },
        Err(_) => {
            // Daemon not running, no cache to refresh
            debug!(
                "Daemon not running, skipping cache refresh for {}",
                provider
            );
        }
    }
}

/// No-op on non-Unix platforms.
#[cfg(not(unix))]
async fn notify_daemon_refresh(_provider: &str) {
    // Daemon is Unix-only
}

/// Run a provider management command.
pub async fn run(command: ProviderCommands) -> Result<()> {
    match command {
        ProviderCommands::List { show_source } => run_list(show_source).await,
        ProviderCommands::Set {
            provider,
            key,
            storage,
        } => {
            let provider = match provider {
                Some(p) => p,
                None => prompts::select_provider()?,
            };
            run_set(&provider, key, storage).await
        }
        ProviderCommands::Get {
            provider,
            unmask,
            yes,
        } => {
            let provider = match provider {
                Some(p) => p,
                None => prompts::select_provider()?,
            };
            run_get(&provider, unmask, yes).await
        }
        ProviderCommands::Delete { provider } => {
            let provider = match provider {
                Some(p) => p,
                None => prompts::select_provider()?,
            };
            run_delete(&provider).await
        }
        ProviderCommands::Migrate { yes } => run_migrate(yes).await,
        ProviderCommands::Test { provider } => {
            let provider = match provider {
                Some(p) => p,
                None => prompts::select_provider()?,
            };
            run_test(&provider).await
        }
        ProviderCommands::Status { json } => run_status(json).await,
    }
}

/// List all providers and their key status.
async fn run_list(show_source: bool) -> Result<()> {
    println!("{}", ds::primary("Provider API Keys"));
    println!();

    // LLM Providers
    println!("{}", ds::highlight("LLM Providers:"));
    list_providers(SUPPORTED_PROVIDERS, show_source);

    println!();
    println!("{}", ds::highlight("MCP Secrets:"));
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
        println!("{}", ds::highlight("Security Summary:"));
        println!(
            "  {} {} in OS Keychain (secure)",
            ds::success(ds::icon::LOCK),
            in_keychain
        );
        if in_env > 0 {
            println!(
                "  {} {} in environment variables",
                ds::warning(ds::icon::PACKAGE),
                in_env
            );
        }
        if in_dotenv > 0 {
            println!("  {} {} in .env files", ds::warning("📄"), in_dotenv);
        }

        // Memory protection status
        println!();
        println!("{}", ds::highlight("Memory Protection:"));
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
                ds::success("🔒"),
                ds::success("mlock available"),
                ds::muted(format!("(limit: {})", limit.unwrap_or("unknown".into())))
            );
        } else {
            println!(
                "  {} {}",
                ds::warning(ds::icon::WARNING),
                ds::warning("mlock unavailable (secrets may be swapped to disk)")
            );
        }

        if in_env > 0 || in_dotenv > 0 {
            println!();
            println!(
                "{}",
                ds::hint_line("Run `spn provider migrate` to move keys to OS Keychain")
            );
        }
    } else {
        println!("{}", ds::warning("No API keys configured."));
        println!();
        println!("Set a key with:");
        println!(
            "  {} {}",
            ds::command("spn provider set"),
            ds::muted("<provider>")
        );
        println!();
        println!("Or migrate from environment variables:");
        println!("  {}", ds::command("spn provider migrate"));
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
                    ds::success(&masked),
                    ds::muted(ds::icon::ARROW),
                    ds::muted(source.description())
                )
            } else {
                format!("{} {}", source.icon(), ds::success(&masked))
            }
        } else {
            ds::muted("○ not set").to_string()
        };

        let env_var = provider_env_var(provider);
        println!(
            "  {:12} {} {}",
            ds::highlight(provider),
            status,
            ds::muted(format!("({})", env_var))
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

        // Notify daemon to refresh its cache (if running)
        notify_daemon_refresh(provider).await;

        println!(
            "{} {} {} {}",
            ds::success(ds::icon::SUCCESS),
            ds::success("API key stored in"),
            ds::primary(&result.location),
            ds::success(format!("for {}", provider))
        );
        println!("  {} {}", ds::label("Key:"), ds::muted(&result.masked_key));
        return Ok(());
    }

    // If storage is specified but no key, use simplified prompt
    if storage.is_some() {
        return run_set_with_storage(provider, storage).await;
    }

    // Interactive wizard mode (no --key, no --storage)
    match run_wizard(provider) {
        Ok(Some(result)) => {
            // Notify daemon to refresh its cache (if running)
            notify_daemon_refresh(provider).await;

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
        ds::primary("Setting API key for"),
        ds::highlight(provider),
        ds::muted(format!("({})", env_var))
    );
    println!(
        "{} {} {}",
        backend.emoji(),
        ds::label("Storage:"),
        ds::muted(backend.description())
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

            // Notify daemon to refresh its cache (if running)
            notify_daemon_refresh(provider).await;

            println!(
                "{}",
                ds::success_line(format!(
                    "API key stored in {} for {}",
                    "OS Keychain", provider
                ))
            );
            println!(
                "  {} {}",
                ds::label("Key:"),
                ds::muted(mask_api_key(&api_key))
            );
            println!();
            println!(
                "{}",
                ds::muted("Key is now securely stored and will be used automatically.")
            );
        }
        StorageBackend::Env => {
            let path = project_env_path();

            // Warn if not gitignored
            if !is_gitignored(&path) {
                eprintln!(
                    "{}",
                    ds::warning_line("Warning: .env is not in .gitignore!")
                );
                eprintln!(
                    "  {}",
                    ds::muted("Add '.env' to .gitignore to avoid committing secrets.")
                );
                eprintln!();
            }

            store_in_dotenv(provider, &api_key, &path)
                .map_err(|e| SpnError::CommandFailed(format!("Failed to store key: {}", e)))?;
            println!(
                "{}",
                ds::success_line(format!("API key stored in {} for {}", ".env", provider))
            );
            println!(
                "  {} {}",
                ds::label("Key:"),
                ds::muted(mask_api_key(&api_key))
            );
            println!("  {} {}", ds::label("File:"), ds::path(path.display()));
        }
        StorageBackend::Global => {
            let path = global_secrets_path().map_err(|e| {
                SpnError::CommandFailed(format!("Failed to determine global secrets path: {}", e))
            })?;
            store_in_global(provider, &api_key)
                .map_err(|e| SpnError::CommandFailed(format!("Failed to store key: {}", e)))?;
            println!(
                "{}",
                ds::success_line(format!(
                    "API key stored in {} for {}",
                    "~/.spn/secrets.env", provider
                ))
            );
            println!(
                "  {} {}",
                ds::label("Key:"),
                ds::muted(mask_api_key(&api_key))
            );
            println!("  {} {}", ds::label("File:"), ds::path(path.display()));
        }
        StorageBackend::Shell => {
            println!();
            println!(
                "{} {}",
                ds::warning(format!("{} SECURITY WARNING:", ds::icon::WARNING)),
                ds::warning("Full API key will be displayed below")
            );
            println!(
                "  {}",
                ds::muted("• Key may appear in terminal scrollback history")
            );
            println!(
                "  {}",
                ds::muted("• May be visible in screen recordings/sharing")
            );
            println!(
                "  {}",
                ds::muted("• Will NOT be stored by spn - you must copy it")
            );
            println!();

            println!(
                "{}",
                ds::success_line(format!("Key validated for {}", provider))
            );
            println!();
            println!("{}", ds::highlight("Export command (copy this):"));
            println!();
            println!(
                "  {}",
                ds::command(format!("export {}='{}'", env_var, *api_key))
            );
            println!();
            println!(
                "{}",
                ds::muted("Add this to your ~/.zshrc or ~/.bashrc to persist.")
            );
            println!(
                "{}",
                ds::hint_line("Use --storage keychain for more secure storage.")
            );
        }
    }

    Ok(())
}

/// Get an API key for a provider.
async fn run_get(provider: &str, unmask: bool, skip_confirm: bool) -> Result<()> {
    match resolve_api_key(provider) {
        Some((key, source)) => {
            if unmask {
                // Check if running in interactive mode (TTY)
                let is_interactive = std::io::stdout().is_terminal();

                // Require confirmation in interactive mode unless --yes is passed
                if is_interactive && !skip_confirm {
                    eprintln!();
                    eprintln!(
                        "{} {}",
                        ds::warning(format!("{} SECURITY WARNING:", ds::icon::WARNING)),
                        ds::warning("Full API key will be exposed")
                    );
                    eprintln!(
                        "  {} Key may appear in terminal scrollback history",
                        ds::muted("•")
                    );
                    eprintln!("  {} Visible in screen recordings/sharing", ds::muted("•"));
                    eprintln!();

                    let confirm = Confirm::new()
                        .with_prompt("Are you sure you want to display the full key?")
                        .default(false)
                        .interact()
                        .map_err(|e| SpnError::InvalidInput(e.to_string()))?;

                    if !confirm {
                        eprintln!("{}", ds::info_line("Cancelled. Key was not displayed."));
                        return Ok(());
                    }
                }

                // Key is printed to stdout for scripting
                println!("{}", *key);
                // key is automatically zeroized when it goes out of scope
            } else {
                println!(
                    "{} {} {}",
                    source.icon(),
                    ds::success(mask_api_key(&key)),
                    ds::muted(format!("{} {}", ds::icon::ARROW, source.description()))
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
            "{}",
            ds::warning_line(format!("No key in keychain for: {}", provider))
        );
        return Ok(());
    }

    // Show current key (masked)
    if let Some(masked) = SpnKeyring::get_masked(provider) {
        println!("{} {}", ds::label("Current key:"), ds::muted(&masked));
    }

    // Confirm deletion
    let confirm = Confirm::new()
        .with_prompt(format!("Delete API key for {} from OS Keychain?", provider))
        .default(false)
        .interact()
        .unwrap_or(false);

    if !confirm {
        println!("{}", ds::muted("Cancelled."));
        return Ok(());
    }

    SpnKeyring::delete(provider)
        .map_err(|e| SpnError::CommandFailed(format!("Failed to delete: {}", e)))?;
    println!(
        "{}",
        ds::success_line(format!("Deleted key for {}", provider))
    );

    Ok(())
}

/// Migrate keys from environment variables to OS keychain.
async fn run_migrate(skip_confirm: bool) -> Result<()> {
    println!("{}", ds::primary("Migrate API Keys to OS Keychain"));
    println!();
    println!(
        "{}",
        ds::muted("This will copy API keys from environment variables to the secure OS keychain.")
    );
    println!(
        "{}",
        ds::muted("Your environment variables will NOT be modified.")
    );
    println!();

    // Show security benefits
    println!("{}", ds::highlight("Benefits of OS Keychain:"));
    println!(
        "  {} Encrypted storage managed by your OS",
        ds::success(ds::icon::LOCK)
    );
    println!(
        "  {} Protected by your login password/biometrics",
        ds::success(ds::icon::LOCK)
    );
    println!(
        "  {} Not readable by other processes",
        ds::success(ds::icon::LOCK)
    );
    println!(
        "  {} Not exposed in process listings",
        ds::success(ds::icon::LOCK)
    );
    println!();

    // macOS-specific info about ACL pre-authorization
    #[cfg(target_os = "macos")]
    {
        println!("{}", ds::highlight("macOS Note:"));
        println!(
            "  {} Keys will be pre-authorized for 'spn' - no repeated popups!",
            ds::primary(ds::icon::INFO)
        );
        println!(
            "  {} If you see a keychain prompt, click \"{}\" once.",
            ds::primary(ds::icon::INFO),
            ds::highlight("Always Allow")
        );
        println!();
    }

    if !skip_confirm {
        let confirm = Confirm::new()
            .with_prompt("Proceed with migration?")
            .default(true)
            .interact()
            .unwrap_or(false);

        if !confirm {
            println!("{}", ds::muted("Cancelled."));
            return Ok(());
        }
    }

    println!();
    let report = migrate_env_to_keyring();
    println!();

    // Summary
    if report.migrated > 0 {
        println!(
            "{}",
            ds::success_line(format!("{} keys migrated to OS Keychain", report.migrated))
        );
    }

    if report.skipped > 0 {
        println!(
            "  {} {} {}",
            ds::muted(ds::icon::ARROW),
            ds::muted(format!("{} keys skipped", report.skipped)),
            ds::muted("(already in keychain)")
        );
    }

    if !report.errors.is_empty() {
        eprintln!();
        eprintln!("{}", ds::error("Errors:"));
        for (provider, error) in &report.errors {
            eprintln!("{}", ds::error_line(format!("{}: {}", provider, error)));
        }
    }

    if report.migrated > 0 {
        println!();
        println!("{}", ds::highlight("Next steps:"));
        println!(
            "  1. Verify keys work: {}",
            ds::command("spn provider test all")
        );
        println!("  2. Remove keys from .env files for better security");
        println!("  3. Keys are now automatically used by spn and Nika");
    }

    Ok(())
}

/// Test provider connection.
async fn run_test(provider: &str) -> Result<()> {
    if provider == "all" {
        println!("{}", ds::primary("Testing all providers..."));
        println!();

        println!("{}", ds::highlight("LLM Providers:"));
        for p in SUPPORTED_PROVIDERS {
            test_single_provider(p).await;
        }

        println!();
        println!("{}", ds::highlight("MCP Secrets:"));
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
    print!(
        "  {} {}... ",
        ds::primary("Testing"),
        ds::highlight(provider)
    );

    match resolve_api_key(provider) {
        None => {
            println!("{}", ds::muted("○ Not configured"));
        }
        Some((key, source)) => {
            // Basic validation
            let validation = validate_key_format(provider, &key);
            if !validation.is_valid() {
                println!(
                    "{} {}",
                    ds::error(format!("{} Invalid format:", ds::icon::ERROR)),
                    validation
                );
                return;
            }

            // Show source and masked key
            println!(
                "{} {} {}",
                ds::success(format!("{} Valid", ds::icon::SUCCESS)),
                source.icon(),
                ds::muted(mask_api_key(&key))
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
        ds::primary(
            "╔═══════════════════════════════════════════════════════════════════════════════╗"
        )
    );
    println!(
        "{}",
        ds::primary(
            "║  🔐 SECRETS & PROVIDERS DIAGNOSTIC REPORT                                     ║"
        )
    );
    println!(
        "{}",
        ds::primary(
            "╚═══════════════════════════════════════════════════════════════════════════════╝"
        )
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
    println!("│  {}", ds::highlight("LLM Providers:"));
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
                    ds::success(&masked),
                    ds::muted(format!("({})", source.description()))
                )
            }
            None => ds::muted("○ not configured").to_string(),
        };
        println!("│    {:12} {}", ds::highlight(provider), status);
    }

    println!("│");
    println!("│  {}", ds::highlight("MCP Secrets:"));
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
                    ds::success(&masked),
                    ds::muted(format!("({})", source.description()))
                )
            }
            None => ds::muted("○ not configured").to_string(),
        };
        println!("│    {:12} {}", ds::highlight(provider), status);
    }
    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Section 2: Storage Distribution
    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  📦 STORAGE DISTRIBUTION                                                    │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!(
        "│    {} {} {} {}",
        ds::success("🔐"),
        ds::success(format!("{:>2}", in_keychain)),
        ds::success("in OS Keychain"),
        ds::muted("(most secure)")
    );
    if in_env > 0 {
        println!(
            "│    {} {} {} {}",
            ds::warning("📦"),
            ds::warning(format!("{:>2}", in_env)),
            ds::warning("in environment variables"),
            ds::muted("(moderate security)")
        );
    }
    if in_dotenv > 0 {
        println!(
            "│    {} {} {} {}",
            ds::warning("📄"),
            ds::warning(format!("{:>2}", in_dotenv)),
            ds::warning("in .env files"),
            ds::muted("(check .gitignore!)")
        );
    }
    let total = SUPPORTED_PROVIDERS.len() + MCP_SECRET_TYPES.len();
    let unconfigured = total - configured;
    if unconfigured > 0 {
        println!(
            "│    {} {} {}",
            ds::muted("○"),
            ds::muted(format!("{:>2}", unconfigured)),
            ds::muted("not configured")
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
            ds::success(ds::icon::SUCCESS),
            ds::success("mlock() available"),
            ds::muted(format!("(limit: {})", limit.unwrap_or("unknown".into())))
        );
        println!(
            "│      {}",
            ds::muted("Secrets are locked in memory and won't be swapped to disk.")
        );
    } else {
        println!(
            "│    {} {}",
            ds::error(ds::icon::ERROR),
            ds::error("mlock() unavailable")
        );
        println!(
            "│      {}",
            ds::muted("Secrets may be swapped to disk. Consider increasing RLIMIT_MEMLOCK.")
        );
    }

    println!("│");
    println!(
        "│    {} {}",
        ds::success(ds::icon::SUCCESS),
        ds::success("Zeroize on drop enabled for all secrets")
    );
    println!(
        "│      {}",
        ds::muted("Secrets are cleared from memory when no longer needed.")
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
                ds::success(ds::icon::SUCCESS).to_string()
            } else {
                ds::muted("○").to_string()
            };
            println!(
                "│    {} {} {}",
                status,
                ds::label("Global secrets:"),
                ds::path(path.display())
            );
        }
        Err(_) => {
            println!(
                "│    {} {} {}",
                ds::error(ds::icon::ERROR),
                ds::label("Global secrets:"),
                ds::error("Cannot determine home directory")
            );
        }
    }

    let env_path = project_env_path();
    let env_exists = env_path.exists();
    let gitignored = is_gitignored(&env_path);
    let env_status = if env_exists {
        ds::success(ds::icon::SUCCESS).to_string()
    } else {
        ds::muted("○").to_string()
    };
    let gitignore_status = if gitignored {
        ds::success("(gitignored ✓)").to_string()
    } else if env_exists {
        ds::warning("(NOT gitignored ⚠)").to_string()
    } else {
        String::new()
    };
    println!(
        "│    {} {} {} {}",
        env_status,
        ds::label("Project .env:"),
        ds::path(env_path.display()),
        gitignore_status
    );

    println!("╰─────────────────────────────────────────────────────────────────────────────╯");
    println!();

    // Section 5: Security Score
    let score = calculate_security_score(in_keychain, in_env, in_dotenv, configured);

    println!("╭─────────────────────────────────────────────────────────────────────────────╮");
    println!("│  🏆 SECURITY SCORE                                                          │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    let bar_width = 40;
    let filled = (score * bar_width / 100) as usize;
    let empty = bar_width as usize - filled;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    let (colored_bar, colored_score) = if score >= 80 {
        (ds::success(&bar), ds::success(format!("{}/100", score)))
    } else if score >= 50 {
        (ds::warning(&bar), ds::warning(format!("{}/100", score)))
    } else {
        (ds::error(&bar), ds::error(format!("{}/100", score)))
    };

    println!("│    {} {}", colored_bar, colored_score);
    println!("│");

    // Recommendations
    if in_env > 0 || in_dotenv > 0 {
        println!("│    {}", ds::highlight("Recommendations:"));
        if in_env > 0 {
            println!(
                "│      {} Migrate {} keys from environment to keychain:",
                ds::primary(ds::icon::ARROW),
                in_env
            );
            println!("│        {}", ds::command("spn provider migrate"));
        }
        if in_dotenv > 0 && !gitignored {
            println!(
                "│      {} Add .env to .gitignore to prevent accidental commits",
                ds::primary(ds::icon::ARROW)
            );
        }
    } else if configured == 0 {
        println!("│    {}", ds::highlight("Get started:"));
        println!(
            "│      {} Set up your first API key:",
            ds::primary(ds::icon::ARROW)
        );
        println!("│        {}", ds::command("spn provider set anthropic"));
    } else {
        println!(
            "│    {} All secrets are stored securely in OS Keychain!",
            ds::success(ds::icon::SUCCESS)
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
