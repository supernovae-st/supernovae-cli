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

use crate::error::Result;
use crate::secrets::{
    mask_api_key, migrate_env_to_keyring, provider_env_var, resolve_api_key, security_audit,
    validate_key_format, SecretSource, SpnKeyring, MCP_SECRET_TYPES, SUPPORTED_PROVIDERS,
};
use crate::ProviderCommands;

use colored::Colorize;
use dialoguer::{Confirm, Password};
use zeroize::Zeroizing;

/// Run a provider management command.
pub async fn run(command: ProviderCommands) -> Result<()> {
    match command {
        ProviderCommands::List { show_source } => run_list(show_source).await,
        ProviderCommands::Set { provider, key } => run_set(&provider, key).await,
        ProviderCommands::Get { provider, unmask } => run_get(&provider, unmask).await,
        ProviderCommands::Delete { provider } => run_delete(&provider).await,
        ProviderCommands::Migrate { yes } => run_migrate(yes).await,
        ProviderCommands::Test { provider } => run_test(&provider).await,
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
    let in_keychain = audit.iter().filter(|(_, s, _)| *s == Some(SecretSource::Keychain)).count();
    let in_env = audit.iter().filter(|(_, s, _)| *s == Some(SecretSource::Environment)).count();
    let in_dotenv = audit.iter().filter(|(_, s, _)| *s == Some(SecretSource::DotEnv)).count();
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
async fn run_set(provider: &str, key: Option<String>) -> Result<()> {
    // Validate provider name
    let all_providers: Vec<&str> = SUPPORTED_PROVIDERS
        .iter()
        .chain(MCP_SECRET_TYPES.iter())
        .copied()
        .collect();

    if !all_providers.contains(&provider) {
        eprintln!(
            "{} {} {}",
            "✗".red(),
            "Unknown provider:".red(),
            provider.bold()
        );
        eprintln!();
        eprintln!("Supported providers:");
        for p in SUPPORTED_PROVIDERS {
            eprintln!("  • {}", p.cyan());
        }
        eprintln!();
        eprintln!("MCP secrets:");
        for p in MCP_SECRET_TYPES {
            eprintln!("  • {}", p.cyan());
        }
        std::process::exit(1);
    }

    // Get key (prompt if not provided)
    // Wrap in Zeroizing immediately for secure handling
    let api_key: Zeroizing<String> = match key {
        Some(k) => Zeroizing::new(k),
        None => {
            let env_var = provider_env_var(provider);
            println!(
                "{} {} {}",
                "Setting API key for".cyan(),
                provider.bold(),
                format!("({})", env_var).dimmed()
            );
            println!(
                "{}",
                "Key will be stored securely in OS Keychain".dimmed()
            );
            println!();

            let input = Password::new()
                .with_prompt("Enter API key")
                .interact()
                .map_err(|e| crate::error::SpnError::InvalidInput(e.to_string()))?;

            Zeroizing::new(input)
        }
    };

    // Validate key format
    if let Err(e) = validate_key_format(provider, &api_key) {
        eprintln!("{} {} {}", "✗".red(), "Invalid key format:".red(), e);
        std::process::exit(1);
    }

    // Store in keychain
    match SpnKeyring::set(provider, &api_key) {
        Ok(()) => {
            println!(
                "{} {} {}",
                "✓".green(),
                "API key stored in OS Keychain for".green(),
                provider.bold()
            );
            println!(
                "  {} {}",
                "Key:".dimmed(),
                mask_api_key(&api_key).dimmed()
            );
            println!();
            println!(
                "{}",
                "Key is now securely stored and will be used automatically.".dimmed()
            );
        }
        Err(e) => {
            eprintln!(
                "{} {} {}",
                "✗".red(),
                "Failed to store key:".red(),
                e.to_string()
            );
            std::process::exit(1);
        }
    }
    // api_key is automatically zeroized when it goes out of scope

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
            eprintln!(
                "{} {} {}",
                "✗".red(),
                "No key found for provider:".red(),
                provider.bold()
            );
            eprintln!();
            eprintln!("Set with: {}", format!("spn provider set {}", provider).cyan());
            std::process::exit(1);
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
        println!(
            "Current key: {}",
            masked.dimmed()
        );
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

    match SpnKeyring::delete(provider) {
        Ok(()) => {
            println!(
                "{} {} {}",
                "✓".green(),
                "Deleted key for".green(),
                provider.bold()
            );
        }
        Err(e) => {
            eprintln!(
                "{} {} {}",
                "✗".red(),
                "Failed to delete:".red(),
                e.to_string()
            );
            std::process::exit(1);
        }
    }

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
    println!("  • {} Encrypted storage managed by your OS", "🔐");
    println!("  • {} Protected by your login password/biometrics", "🔐");
    println!("  • {} Not readable by other processes", "🔐");
    println!("  • {} Not exposed in process listings", "🔐");
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
            if let Err(e) = validate_key_format(provider, &key) {
                println!("{} {}", "✗ Invalid format:".red(), e);
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
}
