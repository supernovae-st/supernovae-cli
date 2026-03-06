//! Interactive wizard for secret management.
//!
//! Provides an intuitive, step-by-step interface for setting up API keys
//! with clear explanations of each storage option's security tradeoffs.

use crate::secrets::{
    global_secrets_path, is_gitignored, mask_api_key, project_env_path, provider_env_var,
    store_in_dotenv, store_in_global, validate_key_format, SpnKeyring, StorageBackend,
};

use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Password, Select};
use zeroize::Zeroizing;

/// Get the API key signup URL for a provider.
fn get_api_key_url(provider: &str) -> Option<&'static str> {
    match provider {
        "anthropic" => Some("https://console.anthropic.com/settings/keys"),
        "openai" => Some("https://platform.openai.com/api-keys"),
        "gemini" => Some("https://aistudio.google.com/app/apikey"),
        "groq" => Some("https://console.groq.com/keys"),
        "mistral" => Some("https://console.mistral.ai/api-keys"),
        "deepseek" => Some("https://platform.deepseek.com/api_keys"),
        "ollama" => Some("https://ollama.ai/download"),
        // MCP secrets
        "github" => Some("https://github.com/settings/tokens"),
        "neo4j" => Some("https://neo4j.com/cloud/aura-free/"),
        "slack" => Some("https://api.slack.com/apps"),
        "perplexity" => Some("https://www.perplexity.ai/settings/api"),
        "firecrawl" => Some("https://firecrawl.dev/app/api-keys"),
        "supadata" => Some("https://supadata.ai/dashboard"),
        _ => None,
    }
}

/// Result of the wizard execution.
#[derive(Debug)]
pub struct WizardResult {
    pub provider: String,
    pub storage: StorageBackend,
    pub masked_key: String,
    pub location: String,
}

/// Storage option with detailed description for the wizard.
struct StorageOption {
    backend: StorageBackend,
    title: &'static str,
    description: &'static str,
    security_note: &'static str,
    recommended: bool,
}

const STORAGE_OPTIONS: &[StorageOption] = &[
    StorageOption {
        backend: StorageBackend::Keychain,
        title: "🔐 OS Keychain (Recommended)",
        description: "Encrypted storage managed by your operating system",
        security_note: "Protected by your login password/biometrics. Most secure option.",
        recommended: true,
    },
    StorageOption {
        backend: StorageBackend::Global,
        title: "🌍 Global secrets file (~/.spn/secrets.env)",
        description: "Shared across all your projects",
        security_note: "File permissions set to 0600 (owner only). Good for multi-project use.",
        recommended: false,
    },
    StorageOption {
        backend: StorageBackend::Env,
        title: "📁 Project .env file",
        description: "Stored in current directory's .env file",
        security_note: "⚠️ Make sure .env is in .gitignore! Good for project-specific keys.",
        recommended: false,
    },
    StorageOption {
        backend: StorageBackend::Shell,
        title: "📋 Shell export (manual)",
        description: "Prints export command for you to copy",
        security_note: "⚠️ Key will be displayed in terminal. You manage storage.",
        recommended: false,
    },
];

/// Run the interactive wizard for setting up an API key.
///
/// This wizard guides users through:
/// 1. Understanding the provider and its environment variable
/// 2. Choosing a storage backend with clear security explanations
/// 3. Entering and validating the API key
/// 4. Confirming the action with a summary
/// 5. Providing post-setup recommendations
pub fn run_wizard(provider: &str) -> Result<Option<WizardResult>> {
    let theme = ColorfulTheme::default();
    let env_var = provider_env_var(provider);

    // Header
    println!();
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║  🔑 API KEY SETUP WIZARD                                                      ║"
            .to_string()
            .cyan()
    );
    println!(
        "{}",
        "╠═══════════════════════════════════════════════════════════════════════════════╣".cyan()
    );
    println!(
        "{}",
        format!(
            "║  Provider: {:<20}  Environment Variable: {:<20}  ║",
            provider.bold(),
            env_var.dimmed()
        )
        .cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    // Show API key URL if available
    if let Some(url) = get_api_key_url(provider) {
        println!(
            "  {} {}",
            "Get your API key at:".dimmed(),
            url.cyan().underline()
        );
        println!();
    }

    // Step 1: Storage Selection
    println!("{}", "STEP 1/3: Choose Storage Location".bold().underline());
    println!();
    println!(
        "{}",
        "Where should this API key be stored? Each option has different security tradeoffs:"
            .dimmed()
    );
    println!();

    // Build selection items with detailed descriptions
    let items: Vec<String> = STORAGE_OPTIONS
        .iter()
        .map(|opt| {
            let rec = if opt.recommended {
                " ← Recommended"
            } else {
                ""
            };
            format!(
                "{}{}\n      {}\n      {}",
                opt.title,
                rec.green().bold(),
                opt.description.dimmed(),
                opt.security_note.dimmed()
            )
        })
        .collect();

    let selection = Select::with_theme(&theme)
        .with_prompt("Select storage backend")
        .items(&items)
        .default(0) // Keychain is default
        .interact_opt()?;

    let storage = match selection {
        Some(idx) => STORAGE_OPTIONS[idx].backend,
        None => {
            println!("{}", "Cancelled.".dimmed());
            return Ok(None);
        }
    };

    println!();
    println!(
        "  {} Selected: {} {}",
        "✓".green(),
        storage.emoji(),
        storage.description().bold()
    );
    println!();

    // Show additional warnings for less secure options
    match storage {
        StorageBackend::Env => {
            let path = project_env_path();
            if !is_gitignored(&path) {
                println!(
                    "{}",
                    "╭─────────────────────────────────────────────────────────────────────────────╮"
                        .yellow()
                );
                println!(
                    "{}",
                    "│  ⚠️  WARNING: .env is NOT in .gitignore!                                    │"
                        .yellow()
                );
                println!(
                    "{}",
                    "│                                                                             │"
                        .yellow()
                );
                println!(
                    "{}",
                    "│  Your API key could be accidentally committed to version control.          │"
                        .yellow()
                );
                println!(
                    "{}",
                    "│  Add '.env' to .gitignore before proceeding, or choose a different option. │"
                        .yellow()
                );
                println!(
                    "{}",
                    "╰─────────────────────────────────────────────────────────────────────────────╯"
                        .yellow()
                );
                println!();

                let proceed = Confirm::with_theme(&theme)
                    .with_prompt("Proceed anyway?")
                    .default(false)
                    .interact()?;

                if !proceed {
                    return Ok(None);
                }
            }
        }
        StorageBackend::Shell => {
            println!(
                "{}",
                "╭─────────────────────────────────────────────────────────────────────────────╮"
                    .yellow()
            );
            println!(
                "{}",
                "│  ⚠️  SHELL EXPORT MODE                                                      │"
                    .yellow()
            );
            println!(
                "{}",
                "│                                                                             │"
                    .yellow()
            );
            println!(
                "{}",
                "│  Your full API key will be displayed in the terminal.                      │"
                    .yellow()
            );
            println!(
                "{}",
                "│  This may be visible in:                                                   │"
                    .yellow()
            );
            println!(
                "{}",
                "│    • Terminal scrollback history                                           │"
                    .yellow()
            );
            println!(
                "{}",
                "│    • Screen recordings or screenshots                                      │"
                    .yellow()
            );
            println!(
                "{}",
                "│    • Shared terminal sessions                                              │"
                    .yellow()
            );
            println!(
                "{}",
                "╰─────────────────────────────────────────────────────────────────────────────╯"
                    .yellow()
            );
            println!();

            let proceed = Confirm::with_theme(&theme)
                .with_prompt("I understand and want to proceed")
                .default(false)
                .interact()?;

            if !proceed {
                return Ok(None);
            }
        }
        _ => {}
    }

    // Step 2: Key Input
    println!("{}", "STEP 2/3: Enter API Key".bold().underline());
    println!();
    println!(
        "{}",
        format!(
            "Enter your {} API key. It will be validated before storage.",
            provider
        )
        .dimmed()
    );
    println!(
        "{}",
        "Your input is hidden and will be securely handled.".dimmed()
    );
    println!();

    let api_key: Zeroizing<String> = loop {
        let input = Password::with_theme(&theme)
            .with_prompt(format!("Enter {} API key", provider))
            .interact()?;

        let key = Zeroizing::new(input);

        // Validate key format
        let validation = validate_key_format(provider, &key);
        if validation.is_valid() {
            println!("  {} Key format valid", "✓".green());
            break key;
        } else {
            println!("  {} Invalid format: {}", "✗".red(), validation);
            println!();

            let retry = Confirm::with_theme(&theme)
                .with_prompt("Try again?")
                .default(true)
                .interact()?;

            if !retry {
                return Ok(None);
            }
            println!();
        }
    };

    println!();

    // Step 3: Confirmation
    println!("{}", "STEP 3/3: Confirm".bold().underline());
    println!();

    let location = match storage {
        StorageBackend::Keychain => "OS Keychain".to_string(),
        StorageBackend::Env => project_env_path().display().to_string(),
        StorageBackend::Global => global_secrets_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.spn/secrets.env".to_string()),
        StorageBackend::Shell => "Terminal (export command)".to_string(),
    };

    let masked = mask_api_key(&api_key);

    println!(
        "{}",
        "╭─────────────────────────────────────────────────────────────────────────────╮".cyan()
    );
    println!(
        "{}",
        "│  📋 SUMMARY                                                                 │".cyan()
    );
    println!(
        "{}",
        "├─────────────────────────────────────────────────────────────────────────────┤".cyan()
    );
    println!(
        "{}",
        format!("│  Provider:  {:<63} │", provider.bold()).cyan()
    );
    println!(
        "{}",
        format!("│  Key:       {:<63} │", masked.dimmed()).cyan()
    );
    println!(
        "{}",
        format!("│  Storage:   {} {:<58} │", storage.emoji(), storage).cyan()
    );
    println!(
        "{}",
        format!("│  Location:  {:<63} │", location.dimmed()).cyan()
    );
    println!(
        "{}",
        format!(
            "│  Security:  {:<63} │",
            format!("Level {}/5", storage.security_level())
        )
        .cyan()
    );
    println!(
        "{}",
        "╰─────────────────────────────────────────────────────────────────────────────╯".cyan()
    );
    println!();

    let confirm = Confirm::with_theme(&theme)
        .with_prompt("Save this API key?")
        .default(true)
        .interact()?;

    if !confirm {
        println!("{}", "Cancelled.".dimmed());
        return Ok(None);
    }

    // Store the key
    println!();
    print!("  {} Storing key... ", "→".cyan());

    let result = match storage {
        StorageBackend::Keychain => SpnKeyring::set(provider, &api_key).map_err(|e| e.into()),
        StorageBackend::Env => {
            let path = project_env_path();
            store_in_dotenv(provider, &api_key, &path)
        }
        StorageBackend::Global => store_in_global(provider, &api_key),
        StorageBackend::Shell => {
            // Shell mode: just print the export command
            println!();
            println!();
            println!("{}", "Export command:".bold());
            println!();
            println!("  {}", format!("export {}='{}'", env_var, *api_key).cyan());
            println!();
            println!(
                "{}",
                "Copy this command and add it to your shell profile:".dimmed()
            );
            println!("  • {} for Zsh", "~/.zshrc".cyan());
            println!("  • {} for Bash", "~/.bashrc".cyan());
            println!();
            return Ok(Some(WizardResult {
                provider: provider.to_string(),
                storage,
                masked_key: masked,
                location,
            }));
        }
    };

    match result {
        Ok(()) => {
            println!("{}", "Done!".green().bold());
            println!();

            // Success message with recommendations
            println!(
                "{}",
                "╭─────────────────────────────────────────────────────────────────────────────╮"
                    .green()
            );
            println!(
                "{}",
                "│  ✅ SUCCESS                                                                 │"
                    .green()
            );
            println!(
                "{}",
                "├─────────────────────────────────────────────────────────────────────────────┤"
                    .green()
            );
            println!(
                "{}",
                format!(
                    "│  Your {} API key is now stored securely.{:width$}│",
                    provider,
                    "",
                    width = 47 - provider.len()
                )
                .green()
            );
            println!(
                "{}",
                "│                                                                             │"
                    .green()
            );
            println!(
                "{}",
                "│  Next steps:                                                                │"
                    .green()
            );
            println!(
                "{}",
                "│    • Run 'spn provider test' to verify the key works                       │"
                    .green()
            );
            println!(
                "{}",
                "│    • Run 'spn provider list --show-source' to see all configured keys      │"
                    .green()
            );
            println!(
                "{}",
                "│    • Run 'spn provider status' for a full diagnostics report               │"
                    .green()
            );
            println!(
                "{}",
                "╰─────────────────────────────────────────────────────────────────────────────╯"
                    .green()
            );
            println!();

            Ok(Some(WizardResult {
                provider: provider.to_string(),
                storage,
                masked_key: masked,
                location,
            }))
        }
        Err(e) => {
            println!("{}", "Failed!".red().bold());
            println!();
            println!("  {} Error: {}", "✗".red(), e);
            Err(e)
        }
    }
}

/// Run a quick non-interactive setup (for scripting).
pub fn run_quick_setup(provider: &str, key: &str, storage: StorageBackend) -> Result<WizardResult> {
    let api_key = Zeroizing::new(key.to_string());

    // Validate
    let validation = validate_key_format(provider, &api_key);
    if !validation.is_valid() {
        return Err(anyhow::anyhow!("Invalid key format: {}", validation));
    }

    let location = match storage {
        StorageBackend::Keychain => "OS Keychain".to_string(),
        StorageBackend::Env => project_env_path().display().to_string(),
        StorageBackend::Global => global_secrets_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.spn/secrets.env".to_string()),
        StorageBackend::Shell => "Terminal".to_string(),
    };

    // Store
    match storage {
        StorageBackend::Keychain => SpnKeyring::set(provider, &api_key)?,
        StorageBackend::Env => {
            let path = project_env_path();
            store_in_dotenv(provider, &api_key, &path)?;
        }
        StorageBackend::Global => {
            store_in_global(provider, &api_key)?;
        }
        StorageBackend::Shell => {
            let env_var = provider_env_var(provider);
            println!("export {}='{}'", env_var, *api_key);
        }
    }

    Ok(WizardResult {
        provider: provider.to_string(),
        storage,
        masked_key: mask_api_key(&api_key),
        location,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_options_count() {
        assert_eq!(STORAGE_OPTIONS.len(), 4);
    }

    #[test]
    fn test_storage_options_have_keychain_first() {
        assert_eq!(STORAGE_OPTIONS[0].backend, StorageBackend::Keychain);
        assert!(STORAGE_OPTIONS[0].recommended);
    }

    #[test]
    fn test_storage_options_only_one_recommended() {
        let recommended_count = STORAGE_OPTIONS.iter().filter(|o| o.recommended).count();
        assert_eq!(recommended_count, 1);
    }

    #[test]
    fn test_all_storage_backends_covered() {
        let backends: Vec<StorageBackend> = STORAGE_OPTIONS.iter().map(|o| o.backend).collect();
        assert!(backends.contains(&StorageBackend::Keychain));
        assert!(backends.contains(&StorageBackend::Env));
        assert!(backends.contains(&StorageBackend::Global));
        assert!(backends.contains(&StorageBackend::Shell));
    }

    #[test]
    fn test_wizard_result_fields() {
        let result = WizardResult {
            provider: "anthropic".to_string(),
            storage: StorageBackend::Keychain,
            masked_key: "sk-ant-...xyz".to_string(),
            location: "OS Keychain".to_string(),
        };

        assert_eq!(result.provider, "anthropic");
        assert_eq!(result.storage, StorageBackend::Keychain);
    }
}
