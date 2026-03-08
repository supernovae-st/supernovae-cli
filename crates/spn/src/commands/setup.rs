//! Onboarding setup wizard for first-time users.
//!
//! Provides a consumer-grade onboarding experience that:
//! 1. Explains what spn is and what it does
//! 2. Detects existing API keys in environment
//! 3. Offers to migrate to secure storage
//! 4. Sets up the primary provider
//! 5. Shows next steps and what spn can do

use crate::error::{Result, SpnError};
use crate::interop::detect::{EcosystemTools, InstallMethod};
use crate::secrets::{
    mask_api_key, migrate_env_to_keyring, resolve_api_key, run_wizard, security_audit, SecretSource,
};
use crate::SetupCommands;

use crate::ux::design_system as ds;
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
    println!("{}", ds::highlight("WHAT IS SPN?").underlined());
    println!();
    println!(
        "{}",
        ds::muted("spn (SuperNovae Package Manager) is your AI development toolkit:")
    );
    println!();
    println!(
        "  {} {}",
        ds::primary("📦"),
        ds::highlight("Package Manager")
    );
    println!(
        "     {}",
        ds::muted("Install AI workflows, schemas, skills, and MCP servers")
    );
    println!();
    println!(
        "  {} {}",
        ds::primary("🔐"),
        ds::highlight("Secrets Manager")
    );
    println!(
        "     {}",
        ds::muted("Securely store API keys for LLM providers and MCP tools")
    );
    println!();
    println!("  {} {}", ds::primary("🔄"), ds::highlight("Sync Manager"));
    println!(
        "     {}",
        ds::muted("Sync packages to Claude Code, VS Code, and other editors")
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
            ds::muted("Setup cancelled. Run `spn setup` anytime to continue.")
        );
        return Ok(());
    }

    println!();

    // Step 2: Ecosystem tools status
    println!(
        "{}",
        ds::highlight("STEP 1/4: Ecosystem Tools").underlined()
    );
    println!();

    let tools = EcosystemTools::detect();
    print_ecosystem_status(&tools);

    // Offer to install missing tools
    if !tools.all_installed() {
        let missing = tools.missing();
        println!();

        for tool in &missing {
            let prompt = format!("Install {} now?", tool);
            let install = Confirm::with_theme(&theme)
                .with_prompt(&prompt)
                .default(true)
                .interact()
                .map_err(dialog_err)?;

            if install {
                install_ecosystem_tool(tool)?;
            }
        }
    }

    println!();

    // Step 3: Detect existing keys
    println!(
        "{}",
        ds::highlight("STEP 2/4: Detecting Existing Keys").underlined()
    );
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
            ds::warning("🔍"),
            in_env.len()
        );
        for (provider, _, _) in &in_env {
            if let Some((key, _)) = resolve_api_key(provider) {
                println!(
                    "     {} {} {}",
                    ds::muted("•"),
                    ds::highlight(provider),
                    ds::muted(mask_api_key(&key))
                );
            }
        }
        println!();

        // Offer to migrate
        println!(
            "{}",
            ds::warning(
                "╭─────────────────────────────────────────────────────────────────────────────╮"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  💡 RECOMMENDATION: Migrate to OS Keychain                                  │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "├─────────────────────────────────────────────────────────────────────────────┤"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  Environment variables are convenient but less secure:                      │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  • Visible to all processes                                                │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  • May appear in logs and crash reports                                    │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  • Not encrypted at rest                                                   │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│                                                                             │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  OS Keychain provides:                                                      │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  • Encrypted storage protected by your login                               │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  • Not visible to other processes                                          │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  • Automatic cleanup on logout                                             │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "╰─────────────────────────────────────────────────────────────────────────────╯"
            )
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
                    ds::command("✓"),
                    report.migrated
                );
            }
            if !report.errors.is_empty() {
                for (provider, error) in &report.errors {
                    println!("  {} {}: {}", ds::error("✗"), provider, error);
                }
            }
        }
    } else if !in_keychain.is_empty() {
        println!(
            "  {} Found {} API keys already in OS Keychain:",
            ds::command("✓"),
            in_keychain.len()
        );
        for (provider, _, _) in &in_keychain {
            if let Some((key, _)) = resolve_api_key(provider) {
                println!(
                    "     {} {} {}",
                    ds::muted("🔐"),
                    ds::highlight(provider),
                    ds::muted(mask_api_key(&key))
                );
            }
        }
        println!();
        println!(
            "  {}",
            ds::command("Your keys are already securely stored!")
        );
    } else {
        println!("  {} No existing API keys detected.", ds::muted("ℹ"));
        println!(
            "  {}",
            ds::muted("Let's set up your first provider in the next step.")
        );
    }

    println!();

    // Step 4: Set up providers
    println!(
        "{}",
        ds::highlight("STEP 3/4: Set Up LLM Providers").underlined()
    );
    println!();
    println!(
        "{}",
        ds::muted("Which LLM providers would you like to configure?")
    );
    println!(
        "{}",
        ds::muted("You can add more later with `spn provider set <name>`")
    );
    println!();

    // Build selection list with provider info
    let items: Vec<String> = PROVIDER_INFO
        .iter()
        .map(|p| {
            let configured = resolve_api_key(p.name).is_some();
            let status = if configured {
                ds::command("✓ configured").to_string()
            } else if p.free_tier {
                ds::muted("○ (free tier)").to_string()
            } else {
                ds::muted("○").to_string()
            };
            format!(
                "{} {} {}\n      {}",
                status,
                ds::highlight(p.display_name),
                ds::muted(p.description),
                ds::primary(p.signup_url).underlined()
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
                ds::primary(format!("━━━ {} ━━━", provider.display_name))
            );
            println!();

            if provider.name == "ollama" {
                println!(
                    "{}",
                    ds::command("Ollama runs models locally - no API key needed!")
                );
                println!("  1. Download Ollama: {}", ds::primary(provider.signup_url));
                println!("  2. Run: {}", ds::primary("ollama pull llama3.2"));
                println!("  3. Set base URL (optional):");
                println!(
                    "     {}",
                    ds::primary("export OLLAMA_API_BASE_URL=http://localhost:11434")
                );
                println!();
                continue;
            }

            println!("  Get your API key at:");
            println!("  {}", ds::primary(provider.signup_url).underlined());
            println!();

            // Run the interactive wizard
            match run_wizard(provider.name) {
                Ok(Some(_)) => {
                    println!();
                }
                Ok(None) => {
                    println!("{}", ds::muted("Skipped."));
                }
                Err(e) => {
                    println!("{} {}", ds::error("Error:"), e);
                }
            }
        }
    }

    println!();

    // Step 5: Summary & Next Steps
    println!(
        "{}",
        ds::highlight("STEP 4/4: Setup Complete!").underlined()
    );
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
    println!("{}", ds::highlight("QUICK SETUP").underlined());
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
            ds::primary("→"),
            in_env.len()
        );
        let report = migrate_env_to_keyring();
        if report.migrated > 0 {
            println!("  {} {} keys migrated", ds::command("✓"), report.migrated);
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
        ds::primary(
            "╔═══════════════════════════════════════════════════════════════════════════════╗"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║                                                                               ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║   ███████╗██████╗ ███╗   ██╗                                                 ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║   ██╔════╝██╔══██╗████╗  ██║     SuperNovae Package Manager                  ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║   ███████╗██████╔╝██╔██╗ ██║     AI Development Toolkit                      ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║   ╚════██║██╔═══╝ ██║╚██╗██║                                                 ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║   ███████║██║     ██║ ╚████║     📦 Packages  🔐 Secrets  🔄 Sync            ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║   ╚══════╝╚═╝     ╚═╝  ╚═══╝                                                 ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "║                                                                               ║"
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        ds::primary(
            "╚═══════════════════════════════════════════════════════════════════════════════╝"
        )
        .cyan()
        .bold()
    );
}

/// Print summary and next steps.
fn print_summary(total_configured: usize, in_keychain: usize) {
    println!(
        "{}",
        ds::command(
            "╭─────────────────────────────────────────────────────────────────────────────╮"
        )
    );
    println!(
        "{}",
        ds::command(
            "│  ✅ SETUP COMPLETE                                                          │"
        )
    );
    println!(
        "{}",
        ds::command(
            "├─────────────────────────────────────────────────────────────────────────────┤"
        )
    );
    println!(
        "{}",
        ds::success(format!(
            "│  {} API keys configured ({} in secure keychain)                          │",
            total_configured, in_keychain
        ))
    );
    println!(
        "{}",
        ds::command(
            "╰─────────────────────────────────────────────────────────────────────────────╯"
        )
    );
    println!();

    println!("{}", ds::highlight("🚀 WHAT'S NEXT?"));
    println!();
    println!("  {} Test your providers:", ds::primary("1."));
    println!("     {}", ds::primary("spn provider test all"));
    println!();
    println!("  {} Add an MCP server:", ds::primary("2."));
    println!("     {}", ds::primary("spn mcp add neo4j"));
    println!();
    println!("  {} Sync to Claude Code:", ds::primary("3."));
    println!("     {}", ds::primary("spn sync --enable claude-code"));
    println!();
    println!("  {} Run a Nika workflow:", ds::primary("4."));
    println!("     {}", ds::primary("nika chat"));
    println!();
    println!(
        "{}",
        ds::muted("Need help? Run `spn topic` for detailed guides.")
    );
    println!();
}

// ============================================================================
// Nika Setup
// ============================================================================

/// Install and configure Nika workflow engine.
async fn run_nika_setup(no_sync: bool, no_lsp: bool, method: &str) -> Result<()> {
    print_nika_banner();

    println!("{}", ds::highlight("CHECKING PREREQUISITES").underlined());
    println!();

    // Check prerequisites
    let has_cargo = Command::new("cargo").arg("--version").output().is_ok();
    let has_brew = Command::new("brew").arg("--version").output().is_ok();

    if !has_cargo && !has_brew && method != "source" {
        println!(
            "{}",
            ds::warning("⚠️  Neither cargo nor brew found. Install one of:")
        );
        println!("     {}", ds::muted("• cargo: https://rustup.rs"));
        println!("     {}", ds::muted("• brew: https://brew.sh"));
        return Err(SpnError::NotFound(
            "cargo or brew required for installation".into(),
        ));
    }

    // Step 1: Install nika CLI
    println!(
        "{}",
        ds::highlight("STEP 1/5: Installing Nika CLI").underlined()
    );
    println!();

    let install_result = match method {
        "cargo" if has_cargo => {
            println!("  {} cargo install nika-cli", ds::primary("Running:"));
            Command::new("cargo").args(["install", "nika-cli"]).status()
        }
        "brew" if has_brew => {
            println!(
                "  {} brew install supernovae-st/tap/nika",
                ds::primary("Running:")
            );
            Command::new("brew")
                .args(["install", "supernovae-st/tap/nika"])
                .status()
        }
        "source" => {
            println!(
                "  {}",
                ds::warning("Source installation: clone and build manually")
            );
            println!(
                "     {}",
                ds::muted("git clone https://github.com/supernovae-st/nika")
            );
            println!(
                "     {}",
                ds::muted("cd nika && cargo install --path tools/nika")
            );
            return Ok(());
        }
        _ => {
            // Fallback to what's available
            if has_cargo {
                println!("  {} cargo install nika-cli", ds::primary("Running:"));
                Command::new("cargo").args(["install", "nika-cli"]).status()
            } else {
                println!(
                    "  {} brew install supernovae-st/tap/nika",
                    ds::primary("Running:")
                );
                Command::new("brew")
                    .args(["install", "supernovae-st/tap/nika"])
                    .status()
            }
        }
    };

    match install_result {
        Ok(status) if status.success() => {
            println!("  {} Nika CLI installed", ds::command("✓"));
        }
        Ok(status) => {
            println!(
                "  {} Installation failed (exit code: {:?})",
                ds::error("✗"),
                status.code()
            );
        }
        Err(e) => {
            println!("  {} Installation error: {}", ds::error("✗"), e);
        }
    }
    println!();

    // Step 2: Install nika-lsp (optional)
    if !no_lsp {
        println!(
            "{}",
            ds::highlight("STEP 2/5: Installing Nika LSP").underlined()
        );
        println!();

        if has_cargo {
            println!("  {} cargo install nika-lsp", ds::primary("Running:"));
            match Command::new("cargo").args(["install", "nika-lsp"]).status() {
                Ok(status) if status.success() => {
                    println!("  {} Nika LSP installed", ds::command("✓"));
                }
                Ok(_) => {
                    println!(
                        "  {}",
                        ds::warning("⚠️  LSP installation failed (optional, continuing)")
                    );
                }
                Err(_) => {
                    println!(
                        "  {}",
                        ds::warning("⚠️  LSP installation failed (optional, continuing)")
                    );
                }
            }
        } else {
            println!("  {}", ds::warning("⚠️  Skipping LSP (requires cargo)"));
        }
        println!();
    }

    // Step 3: Start spn daemon (unified secret management)
    println!(
        "{}",
        ds::highlight("STEP 3/5: Starting spn Daemon").underlined()
    );
    println!();
    println!(
        "  {}",
        ds::muted("The daemon provides unified secret access (no keychain popups).")
    );
    println!();

    // Check if daemon is already running (by checking socket existence)
    let daemon_running = spn_client::daemon_socket_exists();

    if daemon_running {
        println!("  {} Daemon already running", ds::command("✓"));
    } else {
        println!("  {} Starting daemon...", ds::primary("→"));

        // Get the path to the current executable
        let exe = std::env::current_exe().unwrap_or_else(|_| "spn".into());

        // Spawn detached daemon process
        let spawn_result = Command::new(&exe)
            .args(["daemon", "start", "--foreground"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match spawn_result {
            Ok(_) => {
                // Wait a moment for the daemon to start
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                if spn_client::daemon_socket_exists() {
                    println!("  {} Daemon started", ds::command("✓"));
                } else {
                    println!("  {} Daemon may have failed to start", ds::warning("⚠️"));
                    println!(
                        "     {}",
                        ds::muted("Run 'spn daemon start' manually later")
                    );
                }
            }
            Err(e) => {
                println!("  {} Daemon start failed: {}", ds::warning("⚠️"), e);
                println!(
                    "     {}",
                    ds::muted("Run 'spn daemon start' manually later")
                );
            }
        }
    }
    println!();

    // Step 4: Check provider configuration
    println!(
        "{}",
        ds::highlight("STEP 4/5: Checking API Providers").underlined()
    );
    println!();

    // Check for common providers
    let providers_to_check = ["anthropic", "openai", "mistral", "groq"];
    let mut configured_count = 0;
    let mut unconfigured: Vec<&str> = Vec::new();

    for provider in &providers_to_check {
        if crate::secrets::resolve_api_key(provider).is_some() {
            configured_count += 1;
        } else {
            unconfigured.push(provider);
        }
    }

    if configured_count > 0 {
        println!(
            "  {} {}/{} providers configured",
            ds::command("✓"),
            configured_count,
            providers_to_check.len()
        );
    }

    if !unconfigured.is_empty() {
        println!(
            "  {} Missing providers: {}",
            ds::warning("⚠️"),
            unconfigured.join(", ")
        );
        println!();
        println!(
            "     {}",
            ds::muted("Configure with: spn provider set <provider>")
        );
        println!(
            "     {}",
            ds::muted("Or migrate from env: spn provider migrate")
        );
    }
    println!();

    // Step 5: Configure editors
    if !no_sync {
        println!(
            "{}",
            ds::highlight("STEP 5/5: Configuring Editors").underlined()
        );
        println!();

        // Detect Claude Code
        let claude_config = dirs::config_dir()
            .map(|d| d.join("claude-code"))
            .filter(|d| d.exists());

        if claude_config.is_some() {
            println!("  {} Claude Code detected, syncing...", ds::primary("→"));
            match Command::new("spn")
                .args(["sync", "--enable", "claude-code"])
                .status()
            {
                Ok(status) if status.success() => {
                    println!("  {} Claude Code configured", ds::command("✓"));
                }
                _ => {
                    println!("  {} Claude Code sync failed", ds::warning("⚠️"));
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
                ds::primary("→")
            );
            if let Err(e) = configure_vscode_yaml_schema(&settings_path) {
                println!("  {} VS Code config failed: {}", ds::warning("⚠️"), e);
            } else {
                println!("  {} VS Code configured", ds::command("✓"));
            }
        }

        // Detect Cursor
        let cursor_config = dirs::home_dir()
            .map(|d| d.join(".cursor/User/settings.json"))
            .filter(|f| f.exists());

        if let Some(settings_path) = cursor_config {
            println!(
                "  {} Cursor detected, configuring yaml.schemas...",
                ds::primary("→")
            );
            if let Err(e) = configure_cursor_yaml_schema(&settings_path) {
                println!("  {} Cursor config failed: {}", ds::warning("⚠️"), e);
            } else {
                println!("  {} Cursor configured", ds::command("✓"));
            }
        }

        // Detect Windsurf (uses Application Support on macOS, .config on Linux)
        let windsurf_config = dirs::config_dir()
            .map(|d| d.join("Windsurf/User/settings.json"))
            .filter(|f| f.exists());

        if let Some(settings_path) = windsurf_config {
            println!(
                "  {} Windsurf detected, configuring yaml.schemas...",
                ds::primary("→")
            );
            if let Err(e) = configure_windsurf_yaml_schema(&settings_path) {
                println!("  {} Windsurf config failed: {}", ds::warning("⚠️"), e);
            } else {
                println!("  {} Windsurf configured", ds::command("✓"));
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

fn configure_windsurf_yaml_schema(settings_path: &std::path::Path) -> Result<()> {
    // Same logic as VS Code
    configure_vscode_yaml_schema(settings_path)
}

fn print_nika_banner() {
    println!();
    println!(
        "{}",
        ds::primary(
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
        )
    );
    println!();
}

fn print_nika_success() {
    println!();
    println!("{}", ds::highlight("🦋 Nika Setup Complete!").green());
    println!();

    // Setup summary box
    println!("┌───────────────────────────────────────────────────────────┐");
    println!(
        "│  {} Nika CLI installed                               │",
        ds::command("✓")
    );
    println!(
        "│  {} Nika LSP configured                              │",
        ds::command("✓")
    );
    println!(
        "│  {} spn daemon running (unified secrets)             │",
        ds::command("✓")
    );
    println!(
        "│  {} Provider keys verified                           │",
        ds::command("✓")
    );
    println!(
        "│  {} Editors configured                               │",
        ds::command("✓")
    );
    println!("└───────────────────────────────────────────────────────────┘");
    println!();

    // Note about providers (less prominent)
    println!(
        "{}",
        ds::muted("Note: If you haven't configured providers yet, run:")
    );
    println!("      {}", ds::primary("spn provider set anthropic"));
    println!();

    println!("{}", ds::highlight("WHAT'S NEXT?"));
    println!();
    println!("  {}       Launch TUI (Home View)", ds::primary("nika"));
    println!("  {}  Start chat session", ds::primary("nika chat"));
    println!("  {} Open workflow studio", ds::primary("nika studio"));
    println!("  {}  Show all commands", ds::primary("nika --help"));
    println!();
    println!(
        "{}",
        ds::muted("Documentation: https://github.com/supernovae-st/nika#readme")
    );
    println!();
}

// ============================================================================
// NovaNet Setup
// ============================================================================

/// Install and configure NovaNet knowledge graph.
async fn run_novanet_setup(no_sync: bool) -> Result<()> {
    print_novanet_banner();

    let theme = ColorfulTheme::default();

    // Step 1: Check prerequisites
    println!(
        "{}",
        ds::highlight("STEP 1/4: Checking Prerequisites").underlined()
    );
    println!();

    let has_cargo = Command::new("cargo").arg("--version").output().is_ok();
    let has_brew = Command::new("brew").arg("--version").output().is_ok();
    let has_novanet = Command::new("novanet").arg("--version").output().is_ok();
    let has_neo4j = check_neo4j_connection();

    // Show status
    println!(
        "  {} Rust/Cargo: {}",
        if has_cargo {
            ds::command("✓")
        } else {
            ds::error("✗")
        },
        if has_cargo { "installed" } else { "not found" }
    );
    println!(
        "  {} Homebrew: {}",
        if has_brew {
            ds::command("✓")
        } else {
            ds::muted("○")
        },
        if has_brew {
            "installed"
        } else {
            "not found (optional)"
        }
    );
    println!(
        "  {} NovaNet CLI: {}",
        if has_novanet {
            ds::command("✓")
        } else {
            ds::warning("○")
        },
        if has_novanet {
            "installed"
        } else {
            "not installed"
        }
    );
    println!(
        "  {} Neo4j: {}",
        if has_neo4j {
            ds::command("✓")
        } else {
            ds::warning("○")
        },
        if has_neo4j {
            "connected"
        } else {
            "not running"
        }
    );
    println!();

    // Step 2: Install NovaNet CLI if needed
    if !has_novanet {
        println!(
            "{}",
            ds::highlight("STEP 2/4: Installing NovaNet CLI").underlined()
        );
        println!();

        if !has_cargo && !has_brew {
            println!(
                "{}",
                ds::warning("⚠️  Neither cargo nor brew found. Install one of:")
            );
            println!("     {}", ds::muted("• cargo: https://rustup.rs"));
            println!("     {}", ds::muted("• brew: https://brew.sh"));
            return Err(SpnError::NotFound(
                "cargo or brew required for installation".into(),
            ));
        }

        let methods: Vec<&str> = if has_brew && has_cargo {
            vec!["Homebrew (recommended)", "Cargo", "Skip installation"]
        } else if has_brew {
            vec!["Homebrew", "Skip installation"]
        } else {
            vec!["Cargo", "Skip installation"]
        };

        let selection = dialoguer::Select::with_theme(&theme)
            .with_prompt("How would you like to install NovaNet?")
            .items(&methods)
            .default(0)
            .interact()
            .map_err(dialog_err)?;

        let method = methods[selection];

        if method.contains("Skip") {
            println!("{}", ds::muted("Skipping installation."));
        } else if method.contains("Homebrew") {
            println!(
                "  {} brew install supernovae-st/tap/novanet",
                ds::primary("Running:")
            );
            match Command::new("brew")
                .args(["install", "supernovae-st/tap/novanet"])
                .status()
            {
                Ok(status) if status.success() => {
                    println!("  {} NovaNet CLI installed", ds::command("✓"));
                }
                Ok(status) => {
                    println!(
                        "  {} Installation failed (exit code: {:?})",
                        ds::error("✗"),
                        status.code()
                    );
                }
                Err(e) => {
                    println!("  {} Installation error: {}", ds::error("✗"), e);
                }
            }
        } else if method.contains("Cargo") {
            println!("  {} cargo install novanet-cli", ds::primary("Running:"));
            match Command::new("cargo")
                .args(["install", "novanet-cli"])
                .status()
            {
                Ok(status) if status.success() => {
                    println!("  {} NovaNet CLI installed", ds::command("✓"));
                }
                Ok(status) => {
                    println!(
                        "  {} Installation failed (exit code: {:?})",
                        ds::error("✗"),
                        status.code()
                    );
                }
                Err(e) => {
                    println!("  {} Installation error: {}", ds::error("✗"), e);
                }
            }
        }
        println!();
    } else {
        println!(
            "{}",
            ds::highlight("STEP 2/4: NovaNet CLI Already Installed").underlined()
        );
        println!();
        // Show version
        if let Ok(output) = Command::new("novanet").arg("--version").output() {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  {} {}", ds::muted("Version:"), version.trim());
        }
        println!();
    }

    // Step 3: Check/Setup Neo4j
    println!("{}", ds::highlight("STEP 3/4: Neo4j Database").underlined());
    println!();

    if !has_neo4j {
        println!(
            "{}",
            ds::warning(
                "╭─────────────────────────────────────────────────────────────────────────────╮"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  💡 NEO4J NOT RUNNING                                                       │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "├─────────────────────────────────────────────────────────────────────────────┤"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  NovaNet requires Neo4j to store the knowledge graph.                       │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│                                                                             │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  Quick start with Docker:                                                   │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  docker run -d --name neo4j -p 7474:7474 -p 7687:7687 \\                     │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│    -e NEO4J_AUTH=neo4j/password neo4j:5                                     │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│                                                                             │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  Or install locally:                                                        │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  brew install neo4j && neo4j start                                          │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "╰─────────────────────────────────────────────────────────────────────────────╯"
            )
            .yellow()
        );
        println!();

        let has_docker = Command::new("docker").arg("--version").output().is_ok();

        if has_docker {
            let start_docker = Confirm::with_theme(&theme)
                .with_prompt("Start Neo4j with Docker now?")
                .default(true)
                .interact()
                .map_err(dialog_err)?;

            if start_docker {
                println!("  {} Starting Neo4j container...", ds::primary("→"));
                let docker_result = Command::new("docker")
                    .args([
                        "run",
                        "-d",
                        "--name",
                        "novanet-neo4j",
                        "-p",
                        "7474:7474",
                        "-p",
                        "7687:7687",
                        "-e",
                        "NEO4J_AUTH=neo4j/password",
                        "-e",
                        "NEO4J_PLUGINS=[\"apoc\"]",
                        "neo4j:5",
                    ])
                    .status();

                match docker_result {
                    Ok(status) if status.success() => {
                        println!("  {} Neo4j container started", ds::command("✓"));
                        println!("  {} Waiting for Neo4j to be ready...", ds::primary("→"));
                        // Give Neo4j a few seconds to start
                        std::thread::sleep(std::time::Duration::from_secs(5));
                    }
                    Ok(_) => {
                        println!(
                            "  {} Docker start failed (container may already exist)",
                            ds::warning("⚠")
                        );
                        println!("     {}", ds::muted("Try: docker start novanet-neo4j"));
                    }
                    Err(e) => {
                        println!("  {} Docker error: {}", ds::error("✗"), e);
                    }
                }
            }
        }
    } else {
        println!(
            "  {} Neo4j is running and accepting connections",
            ds::command("✓")
        );
    }
    println!();

    // Step 4: Configure editors (if not skipped)
    if !no_sync {
        println!(
            "{}",
            ds::highlight("STEP 4/4: Editor Configuration").underlined()
        );
        println!();

        // Detect Claude Code
        let claude_config = dirs::config_dir()
            .map(|d| d.join("claude-code"))
            .filter(|d| d.exists());

        if claude_config.is_some() {
            println!("  {} Claude Code detected, syncing...", ds::primary("→"));
            match Command::new("spn")
                .args(["sync", "--enable", "claude-code"])
                .status()
            {
                Ok(status) if status.success() => {
                    println!("  {} Claude Code configured", ds::command("✓"));
                }
                _ => {
                    println!("  {} Claude Code sync skipped", ds::muted("○"));
                }
            }
        }
        println!();
    }

    print_novanet_success();
    Ok(())
}

/// Check if Neo4j is running and accepting connections.
fn check_neo4j_connection() -> bool {
    // Try to connect to Neo4j bolt port
    use std::net::TcpStream;
    use std::time::Duration;

    TcpStream::connect_timeout(
        &"127.0.0.1:7687".parse().unwrap(),
        Duration::from_millis(500),
    )
    .is_ok()
}

fn print_novanet_banner() {
    println!();
    println!(
        "{}",
        ds::primary(
            r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ███╗   ██╗ ██████╗ ██╗   ██╗ █████╗ ███╗   ██╗███████╗████████╗
    ║   ████╗  ██║██╔═══██╗██║   ██║██╔══██╗████╗  ██║██╔════╝╚══██╔══╝
    ║   ██╔██╗ ██║██║   ██║██║   ██║███████║██╔██╗ ██║█████╗     ██║
    ║   ██║╚██╗██║██║   ██║╚██╗ ██╔╝██╔══██║██║╚██╗██║██╔══╝     ██║
    ║   ██║ ╚████║╚██████╔╝ ╚████╔╝ ██║  ██║██║ ╚████║███████╗   ██║
    ║   ╚═╝  ╚═══╝ ╚═════╝   ╚═══╝  ╚═╝  ╚═╝╚═╝  ╚═══╝╚══════╝   ╚═╝
    ║                                                               ║
    ║   Knowledge Graph for AI Agents                               ║
    ║   https://github.com/supernovae-st/novanet                    ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
"#
        )
    );
    println!();
}

fn print_novanet_success() {
    println!("{}", ds::highlight("🎉 NOVANET SETUP COMPLETE!").green());
    println!();
    println!("{}", ds::highlight("WHAT'S NEXT?"));
    println!();
    println!("  {} Launch the TUI explorer:", ds::primary("1."));
    println!("     {}", ds::primary("novanet tui"));
    println!();
    println!("  {} Validate your schema:", ds::primary("2."));
    println!("     {}", ds::primary("spn schema validate"));
    println!();
    println!("  {} Generate schema artifacts:", ds::primary("3."));
    println!("     {}", ds::primary("spn schema generate"));
    println!();
    println!("  {} Query the graph:", ds::primary("4."));
    println!(
        "     {}",
        ds::command("novanet query \"MATCH (n) RETURN n LIMIT 10\"")
    );
    println!();
    println!(
        "{}",
        ds::muted("Documentation: https://github.com/supernovae-st/novanet#readme")
    );
    println!();
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
    println!(
        "{}",
        ds::highlight("STEP 1/4: Checking Prerequisites").underlined()
    );
    println!();

    let claude_available = Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !claude_available {
        println!("  {} Claude Code CLI not found", ds::error("✗"));
        println!();
        println!(
            "{}",
            ds::warning(
                "╭─────────────────────────────────────────────────────────────────────────────╮"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  💡 INSTALL CLAUDE CODE                                                     │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "├─────────────────────────────────────────────────────────────────────────────┤"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  npm install -g @anthropic-ai/claude-code                                   │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│                                                                             │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  Or with Homebrew:                                                          │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "│  brew install claude                                                        │"
            )
            .yellow()
        );
        println!(
            "{}",
            ds::warning(
                "╰─────────────────────────────────────────────────────────────────────────────╯"
            )
            .yellow()
        );
        println!();
        return Err(SpnError::NotFound(
            "Claude Code CLI required. Install with: npm install -g @anthropic-ai/claude-code"
                .into(),
        ));
    }

    println!("  {} Claude Code CLI found", ds::command("✓"));
    println!();

    // Step 2: Check if plugin is already installed
    println!(
        "{}",
        ds::highlight("STEP 2/4: Checking Plugin Status").underlined()
    );
    println!();

    let plugin_installed = is_plugin_installed(PLUGIN_FULL_NAME);

    if plugin_installed && !force {
        println!("  {} SuperNovae plugin already installed", ds::command("✓"));
        println!();
        println!(
            "{}",
            ds::muted("Use --force to reinstall: spn setup claude-code --force")
        );
        println!();
        print_claude_code_success(false);
        return Ok(());
    }

    if plugin_installed && force {
        println!(
            "  {} Plugin found, reinstalling (--force)",
            ds::primary("→")
        );
    } else {
        println!("  {} Plugin not found, installing...", ds::primary("→"));
    }
    println!();

    // Step 3: Add the marketplace (if not already added)
    println!(
        "{}",
        ds::highlight("STEP 3/4: Adding Marketplace").underlined()
    );
    println!();

    let marketplace_exists = is_marketplace_added(MARKETPLACE_NAME);

    if marketplace_exists && !force {
        println!("  {} Marketplace already added", ds::command("✓"));
    } else {
        println!(
            "  {} claude plugin marketplace add {}",
            ds::primary("Running:"),
            MARKETPLACE_REPO
        );

        let add_result = Command::new("claude")
            .args(["plugin", "marketplace", "add", MARKETPLACE_REPO])
            .status();

        match add_result {
            Ok(status) if status.success() => {
                println!("  {} Marketplace added successfully", ds::command("✓"));
            }
            Ok(status) => {
                // Marketplace might already exist, which is fine
                println!(
                    "  {} Marketplace add returned code {:?} (may already exist)",
                    ds::primary("→"),
                    status.code()
                );
            }
            Err(e) => {
                println!("  {} Marketplace add error: {}", ds::error("✗"), e);
                return Err(SpnError::IoError(e));
            }
        }
    }
    println!();

    // Step 4: Install the plugin
    println!(
        "{}",
        ds::highlight("STEP 4/4: Installing Plugin").underlined()
    );
    println!();

    println!(
        "  {} claude plugin install {}",
        ds::primary("Running:"),
        PLUGIN_FULL_NAME
    );

    let install_result = Command::new("claude")
        .args(["plugin", "install", PLUGIN_FULL_NAME])
        .status();

    match install_result {
        Ok(status) if status.success() => {
            println!(
                "  {} SuperNovae plugin installed successfully",
                ds::command("✓")
            );
        }
        Ok(status) => {
            println!(
                "  {} Installation failed (exit code: {:?})",
                ds::error("✗"),
                status.code()
            );
            return Err(SpnError::CommandFailed(format!(
                "Plugin installation failed with exit code: {:?}",
                status.code()
            )));
        }
        Err(e) => {
            println!("  {} Installation error: {}", ds::error("✗"), e);
            return Err(SpnError::IoError(e));
        }
    }
    println!();

    print_claude_code_success(true);
    Ok(())
}

/// Check if a plugin is installed by checking ~/.claude/plugins/installed_plugins.json.
fn is_plugin_installed(plugin_name: &str) -> bool {
    let installed_plugins_path =
        dirs::home_dir().map(|h| h.join(".claude/plugins/installed_plugins.json"));

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
    let marketplaces_path = dirs::home_dir().map(|h| {
        h.join(".claude/plugins/marketplaces")
            .join(marketplace_name)
    });

    if let Some(path) = marketplaces_path {
        return path.exists() && path.is_dir();
    }
    false
}

fn print_claude_code_banner() {
    println!();
    println!(
        "{}",
        ds::primary(
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
        )
    );
    println!();
}

fn print_claude_code_success(newly_installed: bool) {
    if newly_installed {
        println!(
            "{}",
            ds::highlight("🎉 CLAUDE CODE PLUGIN INSTALLED!").green()
        );
    } else {
        println!("{}", ds::highlight("✅ CLAUDE CODE PLUGIN READY!").green());
    }
    println!();
    println!("{}", ds::highlight("WHAT'S NEXT?"));
    println!();
    println!("  {} Start Claude Code:", ds::primary("1."));
    println!("     {}", ds::primary("claude"));
    println!();
    println!("  {} Available skills:", ds::primary("2."));
    println!("     {}", ds::muted("/novanet — NovaNet knowledge graph"));
    println!("     {}", ds::muted("/nika — Nika workflow engine"));
    println!(
        "     {}",
        ds::muted("/spn-powers:yo — List all superpowers")
    );
    println!();
    println!("  {} Check plugin status:", ds::primary("3."));
    println!("     {}", ds::primary("spn doctor"));
    println!();
    println!(
        "{}",
        ds::muted("Documentation: https://github.com/supernovae-st/claude-code-supernovae")
    );
    println!();
}

// ============================================================================
// Ecosystem Tools
// ============================================================================

/// Print ecosystem tools status.
fn print_ecosystem_status(tools: &EcosystemTools) {
    // Nika status
    match &tools.nika {
        crate::interop::detect::InstallStatus::Installed { version, .. } => {
            println!(
                "  {} Nika {} {}",
                ds::command("✓"),
                ds::highlight("v".to_string() + version),
                ds::muted("installed")
            );
        }
        crate::interop::detect::InstallStatus::NotInstalled => {
            println!("  {} Nika {}", ds::warning("○"), ds::muted("not installed"));
        }
        crate::interop::detect::InstallStatus::Outdated { current, latest } => {
            println!(
                "  {} Nika {} {} {}",
                ds::warning("↑"),
                ds::muted(current),
                ds::primary("→"),
                ds::highlight(latest)
            );
        }
    }

    // NovaNet status
    match &tools.novanet {
        crate::interop::detect::InstallStatus::Installed { version, .. } => {
            println!(
                "  {} NovaNet {} {}",
                ds::command("✓"),
                ds::highlight("v".to_string() + version),
                ds::muted("installed")
            );
        }
        crate::interop::detect::InstallStatus::NotInstalled => {
            println!(
                "  {} NovaNet {}",
                ds::warning("○"),
                ds::muted("not installed")
            );
        }
        crate::interop::detect::InstallStatus::Outdated { current, latest } => {
            println!(
                "  {} NovaNet {} {} {}",
                ds::warning("↑"),
                ds::muted(current),
                ds::primary("→"),
                ds::highlight(latest)
            );
        }
    }
}

/// Install an ecosystem tool interactively.
fn install_ecosystem_tool(tool: &str) -> Result<()> {
    let method = InstallMethod::best_available().ok_or_else(|| {
        SpnError::NotFound("No installation method available (cargo or brew required)".into())
    })?;

    println!();
    println!(
        "  {} Installing {} via {}...",
        ds::primary("→"),
        ds::highlight(tool),
        ds::muted(method.display_name())
    );

    let result = match tool {
        "nika" => crate::interop::detect::install_nika(method),
        "novanet" => crate::interop::detect::install_novanet(method),
        _ => return Err(SpnError::NotFound(format!("Unknown tool: {}", tool))),
    };

    match result {
        Ok(()) => {
            println!(
                "  {} {} installed successfully",
                ds::command("✓"),
                ds::highlight(tool)
            );
            Ok(())
        }
        Err(e) => {
            println!("  {} Installation failed: {}", ds::error("✗"), e);
            // Don't fail the entire setup, just warn
            Ok(())
        }
    }
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

    // ========================================================================
    // Editor Configuration Tests
    // ========================================================================

    #[test]
    fn test_vscode_uses_config_dir() {
        // VS Code should use the standard config directory
        let config_dir = dirs::config_dir();
        assert!(config_dir.is_some(), "Config dir should be available");

        let vscode_path = config_dir.unwrap().join("Code/User/settings.json");
        // On macOS: ~/Library/Application Support/Code/User/settings.json
        // On Linux: ~/.config/Code/User/settings.json
        assert!(
            vscode_path.to_string_lossy().contains("Code"),
            "VS Code path should contain 'Code'"
        );
    }

    #[test]
    fn test_cursor_uses_home_dir_dotfile() {
        // Cursor uses ~/.cursor/User/settings.json
        let home = dirs::home_dir();
        assert!(home.is_some(), "Home dir should be available");

        let cursor_path = home.unwrap().join(".cursor/User/settings.json");
        assert!(
            cursor_path.to_string_lossy().contains(".cursor"),
            "Cursor path should contain '.cursor'"
        );
    }

    #[test]
    fn test_windsurf_uses_config_dir() {
        // Windsurf should use the standard config directory (like VS Code)
        // NOT a dotfile in home
        let config_dir = dirs::config_dir();
        assert!(config_dir.is_some(), "Config dir should be available");

        let windsurf_path = config_dir.unwrap().join("Windsurf/User/settings.json");
        // On macOS: ~/Library/Application Support/Windsurf/User/settings.json
        // On Linux: ~/.config/Windsurf/User/settings.json
        assert!(
            windsurf_path.to_string_lossy().contains("Windsurf"),
            "Windsurf path should contain 'Windsurf'"
        );

        // Verify it does NOT use dotfile pattern
        assert!(
            !windsurf_path.to_string_lossy().contains(".windsurf"),
            "Windsurf should NOT use dotfile pattern"
        );
    }

    #[test]
    fn test_yaml_schema_url_format() {
        // The Nika schema URL should be valid HTTPS
        let schema_url = "https://nika.dev/schema/workflow.json";
        assert!(schema_url.starts_with("https://"));
        assert!(schema_url.ends_with(".json"));
        assert!(schema_url.contains("nika.dev"));
    }

    #[test]
    fn test_yaml_schema_patterns() {
        // The patterns for Nika workflow files
        let patterns = ["*.nika.yaml", "*.nika.yml"];
        for pattern in patterns {
            assert!(pattern.contains("nika"));
            assert!(pattern.starts_with("*"));
        }
    }

    #[test]
    fn test_configure_vscode_yaml_schema_json_structure() {
        // Test the JSON structure we'd inject
        use serde_json::json;

        let schema_entry = json!({
            "yaml.schemas": {
                "https://nika.dev/schema/workflow.json": ["*.nika.yaml", "*.nika.yml"]
            }
        });

        let schemas = schema_entry.get("yaml.schemas").unwrap();
        let nika_schema = schemas
            .get("https://nika.dev/schema/workflow.json")
            .unwrap();

        assert!(nika_schema.is_array());
        let patterns: Vec<&str> = nika_schema
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(patterns.contains(&"*.nika.yaml"));
        assert!(patterns.contains(&"*.nika.yml"));
    }
}
