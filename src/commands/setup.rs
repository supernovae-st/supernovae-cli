//! Onboarding setup wizard for first-time users.
//!
//! Provides a consumer-grade onboarding experience that:
//! 1. Explains what spn is and what it does
//! 2. Detects existing API keys in environment
//! 3. Offers to migrate to secure storage
//! 4. Sets up the primary provider
//! 5. Shows next steps and what spn can do

use crate::error::{Result, SpnError};
use crate::secrets::{
    mask_api_key, migrate_env_to_keyring, mlock_available, provider_env_var, resolve_api_key,
    run_wizard, security_audit, SecretSource, SpnKeyring, MCP_SECRET_TYPES, SUPPORTED_PROVIDERS,
};

use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect, Select};

/// Helper to convert dialoguer errors to SpnError.
fn dialog_err(e: dialoguer::Error) -> SpnError {
    SpnError::InvalidInput(e.to_string())
}

/// Provider info with signup URL.
struct ProviderInfo {
    name: &'static str,
    display_name: &'static str,
    signup_url: &'static str,
    description: &'static str,
    free_tier: bool,
}

const PROVIDER_INFO: &[ProviderInfo] = &[
    ProviderInfo {
        name: "anthropic",
        display_name: "Anthropic (Claude)",
        signup_url: "https://console.anthropic.com/settings/keys",
        description: "Best for complex reasoning, coding, and extended thinking",
        free_tier: false,
    },
    ProviderInfo {
        name: "openai",
        display_name: "OpenAI (GPT-4)",
        signup_url: "https://platform.openai.com/api-keys",
        description: "General purpose, vision, and code generation",
        free_tier: false,
    },
    ProviderInfo {
        name: "gemini",
        display_name: "Google Gemini",
        signup_url: "https://aistudio.google.com/app/apikey",
        description: "Multimodal with generous free tier",
        free_tier: true,
    },
    ProviderInfo {
        name: "groq",
        display_name: "Groq (Llama)",
        signup_url: "https://console.groq.com/keys",
        description: "Ultra-fast inference, free tier available",
        free_tier: true,
    },
    ProviderInfo {
        name: "mistral",
        display_name: "Mistral AI",
        signup_url: "https://console.mistral.ai/api-keys",
        description: "European provider, excellent for code",
        free_tier: false,
    },
    ProviderInfo {
        name: "deepseek",
        display_name: "DeepSeek",
        signup_url: "https://platform.deepseek.com/api_keys",
        description: "Cost-effective, strong reasoning",
        free_tier: true,
    },
    ProviderInfo {
        name: "ollama",
        display_name: "Ollama (Local)",
        signup_url: "https://ollama.ai/download",
        description: "Run models locally, no API key needed",
        free_tier: true,
    },
];

/// Run the onboarding setup wizard.
pub async fn run(quick: bool) -> Result<()> {
    let theme = ColorfulTheme::default();

    // Welcome banner
    print_welcome_banner();

    if quick {
        return run_quick_setup().await;
    }

    // Step 1: Explain what spn is
    println!();
    println!("{}", "WHAT IS SPN?".bold().underline());
    println!();
    println!(
        "{}",
        "spn (SuperNovae Package Manager) is your AI development toolkit:".dimmed()
    );
    println!();
    println!("  {} {}", "📦".cyan(), "Package Manager".bold());
    println!(
        "     {}",
        "Install AI workflows, schemas, skills, and MCP servers".dimmed()
    );
    println!();
    println!("  {} {}", "🔐".cyan(), "Secrets Manager".bold());
    println!(
        "     {}",
        "Securely store API keys for LLM providers and MCP tools".dimmed()
    );
    println!();
    println!("  {} {}", "🔄".cyan(), "Sync Manager".bold());
    println!(
        "     {}",
        "Sync packages to Claude Code, VS Code, and other editors".dimmed()
    );
    println!();

    let proceed = Confirm::with_theme(&theme)
        .with_prompt("Ready to set up spn?")
        .default(true)
        .interact().map_err(dialog_err)?;

    if !proceed {
        println!("{}", "Setup cancelled. Run `spn setup` anytime to continue.".dimmed());
        return Ok(());
    }

    println!();

    // Step 2: Detect existing keys
    println!("{}", "STEP 1/3: Detecting Existing Keys".bold().underline());
    println!();

    let audit = security_audit();
    let in_env: Vec<_> = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Environment))
        .collect();
    let in_keychain: Vec<_> = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Keychain))
        .collect();

    if !in_env.is_empty() {
        println!(
            "  {} Found {} API keys in environment variables:",
            "🔍".yellow(),
            in_env.len()
        );
        for (provider, _, _) in &in_env {
            if let Some((key, _)) = resolve_api_key(provider) {
                println!(
                    "     {} {} {}",
                    "•".dimmed(),
                    provider.bold(),
                    mask_api_key(&key).dimmed()
                );
            }
        }
        println!();

        // Offer to migrate
        println!(
            "{}",
            "╭─────────────────────────────────────────────────────────────────────────────╮"
                .yellow()
        );
        println!(
            "{}",
            "│  💡 RECOMMENDATION: Migrate to OS Keychain                                  │"
                .yellow()
        );
        println!(
            "{}",
            "├─────────────────────────────────────────────────────────────────────────────┤"
                .yellow()
        );
        println!(
            "{}",
            "│  Environment variables are convenient but less secure:                      │"
                .yellow()
        );
        println!(
            "{}",
            "│  • Visible to all processes                                                │"
                .yellow()
        );
        println!(
            "{}",
            "│  • May appear in logs and crash reports                                    │"
                .yellow()
        );
        println!(
            "{}",
            "│  • Not encrypted at rest                                                   │"
                .yellow()
        );
        println!(
            "{}",
            "│                                                                             │"
                .yellow()
        );
        println!(
            "{}",
            "│  OS Keychain provides:                                                      │"
                .yellow()
        );
        println!(
            "{}",
            "│  • Encrypted storage protected by your login                               │"
                .yellow()
        );
        println!(
            "{}",
            "│  • Not visible to other processes                                          │"
                .yellow()
        );
        println!(
            "{}",
            "│  • Automatic cleanup on logout                                             │"
                .yellow()
        );
        println!(
            "{}",
            "╰─────────────────────────────────────────────────────────────────────────────╯"
                .yellow()
        );
        println!();

        let migrate = Confirm::with_theme(&theme)
            .with_prompt("Migrate keys to secure OS Keychain?")
            .default(true)
            .interact().map_err(dialog_err)?;

        if migrate {
            println!();
            let report = migrate_env_to_keyring();
            if report.migrated > 0 {
                println!(
                    "  {} {} keys migrated to OS Keychain",
                    "✓".green(),
                    report.migrated
                );
            }
            if !report.errors.is_empty() {
                for (provider, error) in &report.errors {
                    println!("  {} {}: {}", "✗".red(), provider, error);
                }
            }
        }
    } else if !in_keychain.is_empty() {
        println!(
            "  {} Found {} API keys already in OS Keychain:",
            "✓".green(),
            in_keychain.len()
        );
        for (provider, _, _) in &in_keychain {
            if let Some((key, _)) = resolve_api_key(provider) {
                println!(
                    "     {} {} {}",
                    "🔐".dimmed(),
                    provider.bold(),
                    mask_api_key(&key).dimmed()
                );
            }
        }
        println!();
        println!("  {}", "Your keys are already securely stored!".green());
    } else {
        println!("  {} No existing API keys detected.", "ℹ".dimmed());
        println!(
            "  {}",
            "Let's set up your first provider in the next step.".dimmed()
        );
    }

    println!();

    // Step 3: Set up providers
    println!("{}", "STEP 2/3: Set Up LLM Providers".bold().underline());
    println!();
    println!(
        "{}",
        "Which LLM providers would you like to configure?".dimmed()
    );
    println!(
        "{}",
        "You can add more later with `spn provider set <name>`".dimmed()
    );
    println!();

    // Build selection list with provider info
    let items: Vec<String> = PROVIDER_INFO
        .iter()
        .map(|p| {
            let configured = resolve_api_key(p.name).is_some();
            let status = if configured {
                "✓ configured".green().to_string()
            } else if p.free_tier {
                "○ (free tier)".dimmed().to_string()
            } else {
                "○".dimmed().to_string()
            };
            format!(
                "{} {} {}\n      {}",
                status,
                p.display_name.bold(),
                p.description.dimmed(),
                p.signup_url.cyan().underline()
            )
        })
        .collect();

    let selections = MultiSelect::with_theme(&theme)
        .with_prompt("Select providers to configure (Space to select, Enter to confirm)")
        .items(&items)
        .interact_opt().map_err(dialog_err)?;

    if let Some(indices) = selections {
        for idx in indices {
            let provider = &PROVIDER_INFO[idx];

            // Skip if already configured
            if resolve_api_key(provider.name).is_some() {
                let reconfigure = Confirm::with_theme(&theme)
                    .with_prompt(format!(
                        "{} is already configured. Reconfigure?",
                        provider.display_name
                    ))
                    .default(false)
                    .interact().map_err(dialog_err)?;

                if !reconfigure {
                    continue;
                }
            }

            println!();
            println!(
                "{}",
                format!("━━━ {} ━━━", provider.display_name).cyan().bold()
            );
            println!();

            if provider.name == "ollama" {
                println!(
                    "{}",
                    "Ollama runs models locally - no API key needed!".green()
                );
                println!("  1. Download Ollama: {}", provider.signup_url.cyan());
                println!("  2. Run: {}", "ollama pull llama3.2".cyan());
                println!("  3. Set base URL (optional):");
                println!("     {}", "export OLLAMA_API_BASE_URL=http://localhost:11434".cyan());
                println!();
                continue;
            }

            println!("  Get your API key at:");
            println!("  {}", provider.signup_url.cyan().underline());
            println!();

            // Run the interactive wizard
            match run_wizard(provider.name) {
                Ok(Some(_)) => {
                    println!();
                }
                Ok(None) => {
                    println!("{}", "Skipped.".dimmed());
                }
                Err(e) => {
                    println!("{} {}", "Error:".red(), e);
                }
            }
        }
    }

    println!();

    // Step 4: Summary & Next Steps
    println!("{}", "STEP 3/3: Setup Complete!".bold().underline());
    println!();

    // Recount configured keys
    let audit = security_audit();
    let total_configured = audit.iter().filter(|(_, s, _)| s.is_some()).count();
    let in_keychain_count = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Keychain))
        .count();

    print_summary(total_configured, in_keychain_count);

    Ok(())
}

/// Quick setup - just migrate existing keys and show status.
async fn run_quick_setup() -> Result<()> {
    println!();
    println!("{}", "QUICK SETUP".bold().underline());
    println!();

    // Detect and migrate
    let audit = security_audit();
    let in_env: Vec<_> = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Environment))
        .collect();

    if !in_env.is_empty() {
        println!(
            "  {} Found {} keys in environment, migrating to keychain...",
            "→".cyan(),
            in_env.len()
        );
        let report = migrate_env_to_keyring();
        if report.migrated > 0 {
            println!(
                "  {} {} keys migrated",
                "✓".green(),
                report.migrated
            );
        }
    }

    // Show status
    let total_configured = audit.iter().filter(|(_, s, _)| s.is_some()).count();
    let in_keychain_count = audit
        .iter()
        .filter(|(_, s, _)| *s == Some(SecretSource::Keychain))
        .count();

    print_summary(total_configured, in_keychain_count);

    Ok(())
}

/// Print welcome banner.
fn print_welcome_banner() {
    println!();
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════════════════════════╗"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║                                                                               ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║   ███████╗██████╗ ███╗   ██╗                                                 ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║   ██╔════╝██╔══██╗████╗  ██║     SuperNovae Package Manager                  ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║   ███████╗██████╔╝██╔██╗ ██║     AI Development Toolkit                      ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║   ╚════██║██╔═══╝ ██║╚██╗██║                                                 ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║   ███████║██║     ██║ ╚████║     📦 Packages  🔐 Secrets  🔄 Sync            ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║   ╚══════╝╚═╝     ╚═╝  ╚═══╝                                                 ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║                                                                               ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════════════════════╝"
            .cyan()
            .bold()
    );
}

/// Print summary and next steps.
fn print_summary(total_configured: usize, in_keychain: usize) {
    println!(
        "{}",
        "╭─────────────────────────────────────────────────────────────────────────────╮"
            .green()
    );
    println!(
        "{}",
        "│  ✅ SETUP COMPLETE                                                          │".green()
    );
    println!(
        "{}",
        "├─────────────────────────────────────────────────────────────────────────────┤"
            .green()
    );
    println!(
        "{}",
        format!(
            "│  {} API keys configured ({} in secure keychain)                          │",
            total_configured, in_keychain
        )
        .green()
    );
    println!(
        "{}",
        "╰─────────────────────────────────────────────────────────────────────────────╯"
            .green()
    );
    println!();

    println!("{}", "🚀 WHAT'S NEXT?".bold());
    println!();
    println!("  {} {}", "1.".cyan().bold(), "Test your providers:");
    println!("     {}", "spn provider test all".cyan());
    println!();
    println!("  {} {}", "2.".cyan().bold(), "Add an MCP server:");
    println!("     {}", "spn mcp add neo4j".cyan());
    println!();
    println!("  {} {}", "3.".cyan().bold(), "Sync to Claude Code:");
    println!("     {}", "spn sync --enable claude-code".cyan());
    println!();
    println!("  {} {}", "4.".cyan().bold(), "Run a Nika workflow:");
    println!("     {}", "nika chat".cyan());
    println!();
    println!(
        "{}",
        "Need help? Run `spn topic` for detailed guides.".dimmed()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info_complete() {
        // All providers should have info
        for provider in SUPPORTED_PROVIDERS {
            let info = PROVIDER_INFO.iter().find(|p| p.name == *provider);
            assert!(info.is_some(), "Missing info for provider: {}", provider);
        }
    }

    #[test]
    fn test_provider_info_urls_valid() {
        for info in PROVIDER_INFO {
            assert!(
                info.signup_url.starts_with("https://"),
                "Invalid URL for {}: {}",
                info.name,
                info.signup_url
            );
        }
    }

    #[test]
    fn test_provider_info_descriptions_non_empty() {
        for info in PROVIDER_INFO {
            assert!(
                !info.description.is_empty(),
                "Empty description for {}",
                info.name
            );
            assert!(
                !info.display_name.is_empty(),
                "Empty display_name for {}",
                info.name
            );
        }
    }

    #[test]
    fn test_at_least_one_free_tier() {
        let free_count = PROVIDER_INFO.iter().filter(|p| p.free_tier).count();
        assert!(free_count >= 1, "Should have at least one free tier provider");
    }

    #[test]
    fn test_anthropic_is_first() {
        // Anthropic should be first (primary provider)
        assert_eq!(PROVIDER_INFO[0].name, "anthropic");
    }
}
