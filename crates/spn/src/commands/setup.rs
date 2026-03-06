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
    mask_api_key, migrate_env_to_keyring, resolve_api_key, run_wizard, security_audit, SecretSource,
};
use crate::SetupCommands;

use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect};
use std::process::Command;

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
pub async fn run(command: Option<SetupCommands>, quick: bool) -> Result<()> {
    // Dispatch to specific setup if command provided
    if let Some(cmd) = command {
        return match cmd {
            SetupCommands::Nika {
                no_sync,
                no_lsp,
                method,
            } => run_nika_setup(no_sync, no_lsp, &method).await,
            SetupCommands::Novanet { no_sync } => run_novanet_setup(no_sync).await,
            SetupCommands::ClaudeCode { force } => run_claude_code_setup(force).await,
        };
    }

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
        .interact()
        .map_err(dialog_err)?;

    if !proceed {
        println!(
            "{}",
            "Setup cancelled. Run `spn setup` anytime to continue.".dimmed()
        );
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
            .interact()
            .map_err(dialog_err)?;

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
        .interact_opt()
        .map_err(dialog_err)?;

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
                    .interact()
                    .map_err(dialog_err)?;

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
                println!(
                    "     {}",
                    "export OLLAMA_API_BASE_URL=http://localhost:11434".cyan()
                );
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
            println!("  {} {} keys migrated", "✓".green(), report.migrated);
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
        "╭─────────────────────────────────────────────────────────────────────────────╮".green()
    );
    println!(
        "{}",
        "│  ✅ SETUP COMPLETE                                                          │".green()
    );
    println!(
        "{}",
        "├─────────────────────────────────────────────────────────────────────────────┤".green()
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
        "╰─────────────────────────────────────────────────────────────────────────────╯".green()
    );
    println!();

    println!("{}", "🚀 WHAT'S NEXT?".bold());
    println!();
    println!("  {} Test your providers:", "1.".cyan().bold());
    println!("     {}", "spn provider test all".cyan());
    println!();
    println!("  {} Add an MCP server:", "2.".cyan().bold());
    println!("     {}", "spn mcp add neo4j".cyan());
    println!();
    println!("  {} Sync to Claude Code:", "3.".cyan().bold());
    println!("     {}", "spn sync --enable claude-code".cyan());
    println!();
    println!("  {} Run a Nika workflow:", "4.".cyan().bold());
    println!("     {}", "nika chat".cyan());
    println!();
    println!(
        "{}",
        "Need help? Run `spn topic` for detailed guides.".dimmed()
    );
    println!();
}

// ============================================================================
// Nika Setup
// ============================================================================

/// Install and configure Nika workflow engine.
async fn run_nika_setup(no_sync: bool, no_lsp: bool, method: &str) -> Result<()> {
    print_nika_banner();

    println!("{}", "CHECKING PREREQUISITES".bold().underline());
    println!();

    // Check prerequisites
    let has_cargo = Command::new("cargo").arg("--version").output().is_ok();
    let has_brew = Command::new("brew").arg("--version").output().is_ok();

    if !has_cargo && !has_brew && method != "source" {
        println!(
            "{}",
            "⚠️  Neither cargo nor brew found. Install one of:".yellow()
        );
        println!("     {}", "• cargo: https://rustup.rs".dimmed());
        println!("     {}", "• brew: https://brew.sh".dimmed());
        return Err(SpnError::NotFound(
            "cargo or brew required for installation".into(),
        ));
    }

    // Step 1: Install nika CLI
    println!("{}", "STEP 1/3: Installing Nika CLI".bold().underline());
    println!();

    let install_result = match method {
        "cargo" if has_cargo => {
            println!("  {} cargo install nika-cli", "Running:".cyan());
            Command::new("cargo").args(["install", "nika-cli"]).status()
        }
        "brew" if has_brew => {
            println!(
                "  {} brew install supernovae-st/tap/nika",
                "Running:".cyan()
            );
            Command::new("brew")
                .args(["install", "supernovae-st/tap/nika"])
                .status()
        }
        "source" => {
            println!(
                "  {}",
                "Source installation: clone and build manually".yellow()
            );
            println!(
                "     {}",
                "git clone https://github.com/supernovae-st/nika".dimmed()
            );
            println!(
                "     {}",
                "cd nika && cargo install --path tools/nika".dimmed()
            );
            return Ok(());
        }
        _ => {
            // Fallback to what's available
            if has_cargo {
                println!("  {} cargo install nika-cli", "Running:".cyan());
                Command::new("cargo").args(["install", "nika-cli"]).status()
            } else {
                println!(
                    "  {} brew install supernovae-st/tap/nika",
                    "Running:".cyan()
                );
                Command::new("brew")
                    .args(["install", "supernovae-st/tap/nika"])
                    .status()
            }
        }
    };

    match install_result {
        Ok(status) if status.success() => {
            println!("  {} Nika CLI installed", "✓".green());
        }
        Ok(status) => {
            println!(
                "  {} Installation failed (exit code: {:?})",
                "✗".red(),
                status.code()
            );
        }
        Err(e) => {
            println!("  {} Installation error: {}", "✗".red(), e);
        }
    }
    println!();

    // Step 2: Install nika-lsp (optional)
    if !no_lsp {
        println!("{}", "STEP 2/3: Installing Nika LSP".bold().underline());
        println!();

        if has_cargo {
            println!("  {} cargo install nika-lsp", "Running:".cyan());
            match Command::new("cargo").args(["install", "nika-lsp"]).status() {
                Ok(status) if status.success() => {
                    println!("  {} Nika LSP installed", "✓".green());
                }
                Ok(_) => {
                    println!(
                        "  {}",
                        "⚠️  LSP installation failed (optional, continuing)".yellow()
                    );
                }
                Err(_) => {
                    println!(
                        "  {}",
                        "⚠️  LSP installation failed (optional, continuing)".yellow()
                    );
                }
            }
        } else {
            println!("  {}", "⚠️  Skipping LSP (requires cargo)".yellow());
        }
        println!();
    }

    // Step 3: Configure editors
    if !no_sync {
        println!("{}", "STEP 3/3: Configuring Editors".bold().underline());
        println!();

        // Detect Claude Code
        let claude_config = dirs::config_dir()
            .map(|d| d.join("claude-code"))
            .filter(|d| d.exists());

        if claude_config.is_some() {
            println!("  {} Claude Code detected, syncing...", "→".cyan());
            match Command::new("spn")
                .args(["sync", "--enable", "claude-code"])
                .status()
            {
                Ok(status) if status.success() => {
                    println!("  {} Claude Code configured", "✓".green());
                }
                _ => {
                    println!("  {} Claude Code sync failed", "⚠️".yellow());
                }
            }
        }

        // Detect VS Code
        let vscode_config = dirs::config_dir()
            .map(|d| d.join("Code/User/settings.json"))
            .filter(|f| f.exists());

        if let Some(settings_path) = vscode_config {
            println!(
                "  {} VS Code detected, configuring yaml.schemas...",
                "→".cyan()
            );
            if let Err(e) = configure_vscode_yaml_schema(&settings_path) {
                println!("  {} VS Code config failed: {}", "⚠️".yellow(), e);
            } else {
                println!("  {} VS Code configured", "✓".green());
            }
        }

        // Detect Cursor
        let cursor_config = dirs::home_dir()
            .map(|d| d.join(".cursor/User/settings.json"))
            .filter(|f| f.exists());

        if let Some(settings_path) = cursor_config {
            println!(
                "  {} Cursor detected, configuring yaml.schemas...",
                "→".cyan()
            );
            if let Err(e) = configure_cursor_yaml_schema(&settings_path) {
                println!("  {} Cursor config failed: {}", "⚠️".yellow(), e);
            } else {
                println!("  {} Cursor configured", "✓".green());
            }
        }
        println!();
    }

    print_nika_success();
    Ok(())
}

/// Configure VS Code yaml.schemas for .nika.yaml files.
fn configure_vscode_yaml_schema(settings_path: &std::path::Path) -> Result<()> {
    let content = std::fs::read_to_string(settings_path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| SpnError::InvalidInput(format!("Invalid JSON: {}", e)))?;

    // Add yaml.schemas if not present
    let schemas = settings
        .as_object_mut()
        .ok_or_else(|| SpnError::InvalidInput("settings must be object".into()))?
        .entry("yaml.schemas")
        .or_insert(serde_json::json!({}));

    // Add nika schema
    if let Some(obj) = schemas.as_object_mut() {
        obj.insert(
            "https://nika.dev/schema/workflow.json".into(),
            serde_json::json!(["*.nika.yaml", "*.nika.yml"]),
        );
    }

    let pretty = serde_json::to_string_pretty(&settings)
        .map_err(|e| SpnError::InvalidInput(format!("JSON serialize error: {}", e)))?;
    std::fs::write(settings_path, pretty)?;
    Ok(())
}

/// Configure Cursor yaml.schemas for .nika.yaml files.
fn configure_cursor_yaml_schema(settings_path: &std::path::Path) -> Result<()> {
    // Same logic as VS Code
    configure_vscode_yaml_schema(settings_path)
}

fn print_nika_banner() {
    println!();
    println!(
        "{}",
        r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ███╗   ██╗██╗██╗  ██╗ █████╗                               ║
    ║   ████╗  ██║██║██║ ██╔╝██╔══██╗                              ║
    ║   ██╔██╗ ██║██║█████╔╝ ███████║                              ║
    ║   ██║╚██╗██║██║██╔═██╗ ██╔══██║                              ║
    ║   ██║ ╚████║██║██║  ██╗██║  ██║                              ║
    ║   ╚═╝  ╚═══╝╚═╝╚═╝  ╚═╝╚═╝  ╚═╝                              ║
    ║                                                               ║
    ║   Semantic YAML Workflow Engine                               ║
    ║   https://github.com/supernovae-st/nika                       ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
"#
        .cyan()
    );
    println!();
}

fn print_nika_success() {
    println!("{}", "🎉 NIKA SETUP COMPLETE!".bold().green());
    println!();
    println!("{}", "WHAT'S NEXT?".bold());
    println!();
    println!("  {} Try the TUI:", "1.".cyan().bold());
    println!("     {}", "nika".cyan());
    println!();
    println!("  {} Start a chat:", "2.".cyan().bold());
    println!("     {}", "nika chat".cyan());
    println!();
    println!("  {} Create a workflow:", "3.".cyan().bold());
    println!("     {}", "nika new my-workflow".cyan());
    println!();
    println!("  {} Run a workflow:", "4.".cyan().bold());
    println!("     {}", "nika my-workflow.nika.yaml".cyan());
    println!();
    println!(
        "{}",
        "Documentation: https://github.com/supernovae-st/nika#readme".dimmed()
    );
    println!();
}

// ============================================================================
// NovaNet Setup (placeholder)
// ============================================================================

/// Install and configure NovaNet knowledge graph.
async fn run_novanet_setup(_no_sync: bool) -> Result<()> {
    println!();
    println!("{}", "NovaNet setup is not yet implemented.".yellow());
    println!("{}", "For now, follow the manual setup at:".dimmed());
    println!(
        "  {}",
        "https://github.com/supernovae-st/novanet#readme".cyan()
    );
    println!();
    Ok(())
}

// ============================================================================
// Claude Code Plugin Setup
// ============================================================================

const MARKETPLACE_REPO: &str = "supernovae-st/claude-code-supernovae";
const MARKETPLACE_NAME: &str = "claude-code-supernovae";
#[cfg(test)]
const PLUGIN_NAME: &str = "supernovae";
const PLUGIN_FULL_NAME: &str = "supernovae@claude-code-supernovae";

/// Install SuperNovae Claude Code plugin.
async fn run_claude_code_setup(force: bool) -> Result<()> {
    print_claude_code_banner();

    // Step 1: Check if Claude CLI is available
    println!("{}", "STEP 1/4: Checking Prerequisites".bold().underline());
    println!();

    let claude_available = Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !claude_available {
        println!("  {} Claude Code CLI not found", "✗".red());
        println!();
        println!(
            "{}",
            "╭─────────────────────────────────────────────────────────────────────────────╮".yellow()
        );
        println!(
            "{}",
            "│  💡 INSTALL CLAUDE CODE                                                     │".yellow()
        );
        println!(
            "{}",
            "├─────────────────────────────────────────────────────────────────────────────┤".yellow()
        );
        println!(
            "{}",
            "│  npm install -g @anthropic-ai/claude-code                                   │".yellow()
        );
        println!(
            "{}",
            "│                                                                             │".yellow()
        );
        println!(
            "{}",
            "│  Or with Homebrew:                                                          │".yellow()
        );
        println!(
            "{}",
            "│  brew install claude                                                        │".yellow()
        );
        println!(
            "{}",
            "╰─────────────────────────────────────────────────────────────────────────────╯".yellow()
        );
        println!();
        return Err(SpnError::NotFound(
            "Claude Code CLI required. Install with: npm install -g @anthropic-ai/claude-code".into(),
        ));
    }

    println!("  {} Claude Code CLI found", "✓".green());
    println!();

    // Step 2: Check if plugin is already installed
    println!("{}", "STEP 2/4: Checking Plugin Status".bold().underline());
    println!();

    let plugin_installed = is_plugin_installed(PLUGIN_FULL_NAME);

    if plugin_installed && !force {
        println!("  {} SuperNovae plugin already installed", "✓".green());
        println!();
        println!(
            "{}",
            "Use --force to reinstall: spn setup claude-code --force".dimmed()
        );
        println!();
        print_claude_code_success(false);
        return Ok(());
    }

    if plugin_installed && force {
        println!("  {} Plugin found, reinstalling (--force)", "→".cyan());
    } else {
        println!("  {} Plugin not found, installing...", "→".cyan());
    }
    println!();

    // Step 3: Add the marketplace (if not already added)
    println!("{}", "STEP 3/4: Adding Marketplace".bold().underline());
    println!();

    let marketplace_exists = is_marketplace_added(MARKETPLACE_NAME);

    if marketplace_exists && !force {
        println!("  {} Marketplace already added", "✓".green());
    } else {
        println!(
            "  {} claude plugin marketplace add {}",
            "Running:".cyan(),
            MARKETPLACE_REPO
        );

        let add_result = Command::new("claude")
            .args(["plugin", "marketplace", "add", MARKETPLACE_REPO])
            .status();

        match add_result {
            Ok(status) if status.success() => {
                println!("  {} Marketplace added successfully", "✓".green());
            }
            Ok(status) => {
                // Marketplace might already exist, which is fine
                println!(
                    "  {} Marketplace add returned code {:?} (may already exist)",
                    "→".cyan(),
                    status.code()
                );
            }
            Err(e) => {
                println!("  {} Marketplace add error: {}", "✗".red(), e);
                return Err(SpnError::IoError(e));
            }
        }
    }
    println!();

    // Step 4: Install the plugin
    println!("{}", "STEP 4/4: Installing Plugin".bold().underline());
    println!();

    println!(
        "  {} claude plugin install {}",
        "Running:".cyan(),
        PLUGIN_FULL_NAME
    );

    let install_result = Command::new("claude")
        .args(["plugin", "install", PLUGIN_FULL_NAME])
        .status();

    match install_result {
        Ok(status) if status.success() => {
            println!("  {} SuperNovae plugin installed successfully", "✓".green());
        }
        Ok(status) => {
            println!(
                "  {} Installation failed (exit code: {:?})",
                "✗".red(),
                status.code()
            );
            return Err(SpnError::CommandFailed(format!(
                "Plugin installation failed with exit code: {:?}",
                status.code()
            )));
        }
        Err(e) => {
            println!("  {} Installation error: {}", "✗".red(), e);
            return Err(SpnError::IoError(e));
        }
    }
    println!();

    print_claude_code_success(true);
    Ok(())
}

/// Check if a plugin is installed by checking ~/.claude/plugins/installed_plugins.json.
fn is_plugin_installed(plugin_name: &str) -> bool {
    let installed_plugins_path = dirs::home_dir()
        .map(|h| h.join(".claude/plugins/installed_plugins.json"));

    if let Some(path) = installed_plugins_path {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                // Check if the plugin name appears in the installed plugins
                return content.contains(plugin_name);
            }
        }
    }
    false
}

/// Check if a marketplace is added by checking ~/.claude/plugins/marketplaces/.
fn is_marketplace_added(marketplace_name: &str) -> bool {
    let marketplaces_path = dirs::home_dir()
        .map(|h| h.join(".claude/plugins/marketplaces").join(marketplace_name));

    if let Some(path) = marketplaces_path {
        return path.exists() && path.is_dir();
    }
    false
}

fn print_claude_code_banner() {
    println!();
    println!(
        "{}",
        r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ███████╗██████╗ ███╗   ██╗                                 ║
    ║   ██╔════╝██╔══██╗████╗  ██║  Claude Code Plugin              ║
    ║   ███████╗██████╔╝██╔██╗ ██║  Skills • Agents • MCP           ║
    ║   ╚════██║██╔═══╝ ██║╚██╗██║                                 ║
    ║   ███████║██║     ██║ ╚████║  supernovae-st/claude-code-supernovae
    ║   ╚══════╝╚═╝     ╚═╝  ╚═══╝                                 ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
"#
        .cyan()
    );
    println!();
}

fn print_claude_code_success(newly_installed: bool) {
    if newly_installed {
        println!("{}", "🎉 CLAUDE CODE PLUGIN INSTALLED!".bold().green());
    } else {
        println!("{}", "✅ CLAUDE CODE PLUGIN READY!".bold().green());
    }
    println!();
    println!("{}", "WHAT'S NEXT?".bold());
    println!();
    println!("  {} Start Claude Code:", "1.".cyan().bold());
    println!("     {}", "claude".cyan());
    println!();
    println!("  {} Available skills:", "2.".cyan().bold());
    println!("     {}", "/novanet — NovaNet knowledge graph".dimmed());
    println!("     {}", "/nika — Nika workflow engine".dimmed());
    println!("     {}", "/spn-powers:yo — List all superpowers".dimmed());
    println!();
    println!("  {} Check plugin status:", "3.".cyan().bold());
    println!("     {}", "spn doctor".cyan());
    println!();
    println!(
        "{}",
        "Documentation: https://github.com/supernovae-st/claude-code-supernovae".dimmed()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::SUPPORTED_PROVIDERS;

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
        assert!(
            free_count >= 1,
            "Should have at least one free tier provider"
        );
    }

    #[test]
    fn test_anthropic_is_first() {
        // Anthropic should be first (primary provider)
        assert_eq!(PROVIDER_INFO[0].name, "anthropic");
    }

    #[test]
    fn test_marketplace_repo_is_valid() {
        // Marketplace repo should be in org/repo format
        assert!(MARKETPLACE_REPO.contains('/'));
        assert!(MARKETPLACE_REPO.starts_with("supernovae-st/"));
    }

    #[test]
    fn test_plugin_full_name_format() {
        // Plugin full name should be in plugin@marketplace format
        assert!(PLUGIN_FULL_NAME.contains('@'));
        assert!(PLUGIN_FULL_NAME.starts_with(PLUGIN_NAME));
        assert!(PLUGIN_FULL_NAME.ends_with(MARKETPLACE_NAME));
    }

    #[test]
    fn test_is_plugin_installed_handles_nonexistent_gracefully() {
        // Should not panic and return false for nonexistent plugin
        let result = is_plugin_installed("nonexistent-plugin-xyz-12345");
        // Result depends on whether any plugin file contains this string (unlikely)
        // Main test is that it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_is_marketplace_added_handles_nonexistent() {
        // Should return false for nonexistent marketplace
        let result = is_marketplace_added("nonexistent-marketplace-xyz-12345");
        assert!(!result);
    }
}
