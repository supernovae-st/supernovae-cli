//! Config command implementation.
//!
//! Manages configuration across three scopes: Global, Team, Local.

use std::env;
use std::path::Path;

use crate::config::{global, local, resolver::format_value, scope::ScopeType, team, ConfigResolver};
use crate::error::{Result, SpnError};
use crate::ux::design_system as ds;
use crate::ConfigCommands;

/// Validate that an editor command is safe to execute.
///
/// Returns the validated editor command, or an error if the command is invalid.
/// This prevents command injection attacks via malicious EDITOR env vars.
fn validate_editor(editor: &str) -> Result<&str> {
    // Check for empty editor
    if editor.is_empty() {
        return Err(SpnError::ConfigError("EDITOR is empty".to_string()));
    }

    // Shell metacharacters that could enable command injection
    const SHELL_METACHARACTERS: &[char] = &[
        ';', '|', '&', '$', '`', '(', ')', '{', '}', '[', ']', '<', '>', '\n', '\r', '\0',
    ];

    // Check for shell metacharacters
    if editor.chars().any(|c| SHELL_METACHARACTERS.contains(&c)) {
        return Err(SpnError::ConfigError(format!(
            "EDITOR contains invalid characters: {}",
            editor
        )));
    }

    // Split editor command (may have args like "code --wait")
    let editor_cmd = editor.split_whitespace().next().unwrap_or(editor);

    // If absolute path, verify it exists
    if editor_cmd.starts_with('/') && !Path::new(editor_cmd).exists() {
        return Err(SpnError::ConfigError(format!(
            "Editor not found: {}",
            editor_cmd
        )));
    }

    Ok(editor)
}

pub async fn run(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show { section } => show_config(section).await,
        ConfigCommands::Where => show_locations().await,
        ConfigCommands::List { show_origin } => list_config(show_origin).await,
        ConfigCommands::Get { key, show_origin } => get_value(&key, show_origin).await,
        ConfigCommands::Set { key, value, scope } => set_value(&key, &value, &scope).await,
        ConfigCommands::Edit { local, user, mcp } => edit_config(local, user, mcp).await,
        ConfigCommands::Import { file, scope, yes } => import_config(&file, &scope, yes).await,
    }
}

async fn show_config(_section: Option<String>) -> Result<()> {
    let resolver = ConfigResolver::load()?;
    let config = resolver.resolved();
    let scopes = resolver.get_scope_paths()?;

    println!("{}", ds::primary("⚙️  Resolved Configuration"));
    println!();

    let mut has_config = false;

    // Show providers
    if !config.providers.is_empty() {
        has_config = true;
        println!("{}", ds::highlight("Providers:"));
        for (name, provider) in &config.providers {
            if let Some(model) = &provider.model {
                println!("  {} model = {}", ds::primary(name), model);
            }
            if let Some(endpoint) = &provider.endpoint {
                println!("  {} endpoint = {}", ds::primary(name), endpoint);
            }
        }
        println!();
    }

    // Show sync config
    if !config.sync.enabled_editors.is_empty() || config.sync.auto_sync {
        has_config = true;
        println!("{}", ds::highlight("Sync:"));
        println!("  enabled_editors = {:?}", config.sync.enabled_editors);
        println!("  auto_sync = {}", config.sync.auto_sync);
        println!();
    }

    // Show MCP servers
    if !config.servers.is_empty() {
        has_config = true;
        println!("{}", ds::highlight("MCP Servers:"));
        for (name, server) in &config.servers {
            let status = if server.disabled { "(disabled)" } else { "" };
            println!(
                "  {} {} {}",
                ds::primary(name),
                server.command,
                ds::muted(status)
            );
        }
        println!();
    }

    // Show message if no config found
    if !has_config {
        println!("  {}", ds::muted("No configuration found."));
        println!();
        println!("{}", ds::highlight("Config File Locations:"));
        for scope in &scopes {
            let status = if scope.exists {
                ds::success("✓")
            } else {
                ds::muted("○")
            };
            println!("  {} {}", status, scope.display_name());
        }
        println!();
        println!("{}", ds::highlight("Quick Start:"));
        println!("  {} Create project manifest", ds::primary("spn init"));
        println!("  {} Add MCP server", ds::primary("spn mcp add <name>"));
        println!("  {} Set API key", ds::primary("spn provider set <name>"));
        println!();
    }

    Ok(())
}

async fn show_locations() -> Result<()> {
    let resolver = ConfigResolver::load()?;
    let scopes = resolver.get_scope_paths()?;

    println!("{}", ds::primary("📁 Config File Locations"));
    println!();
    println!("   {}", ds::muted("Precedence: Local > Team > Global"));
    println!();

    for scope in scopes {
        let status = if scope.exists {
            ds::success("✓")
        } else {
            ds::muted("○")
        };
        println!("   {} {}", status, scope.display_name());
    }

    println!();
    println!(
        "   {} = exists, {} = not found",
        ds::success("✓"),
        ds::muted("○")
    );

    Ok(())
}

async fn list_config(show_origin: bool) -> Result<()> {
    let resolver = ConfigResolver::load()?;
    let config = resolver.resolved();

    println!("{}", ds::primary("📋 Configuration Values"));
    println!();

    // List providers
    if !config.providers.is_empty() {
        for (name, provider) in &config.providers {
            if let Some(model) = &provider.model {
                if show_origin {
                    // TODO: Implement proper origin tracking
                    println!("  providers.{}.model = {} (global)", name, model);
                } else {
                    println!("  providers.{}.model = {}", name, model);
                }
            }
        }
    }

    // List sync config
    if !config.sync.enabled_editors.is_empty() {
        println!("  sync.enabled_editors = {:?}", config.sync.enabled_editors);
    }
    if config.sync.auto_sync {
        println!("  sync.auto_sync = true");
    }

    // List servers
    if !config.servers.is_empty() {
        for name in config.servers.keys() {
            println!("  servers.{} = <configured>", name);
        }
    }

    if show_origin {
        println!();
        println!(
            "   {}",
            ds::muted("Use 'spn config get <key> --show-origin' for detailed origin info")
        );
    }

    Ok(())
}

async fn get_value(key: &str, show_origin: bool) -> Result<()> {
    let resolver = ConfigResolver::load()?;

    // Get value from resolved config
    match resolver.get_value(key) {
        Some(value) => {
            let formatted = format_value(&value);
            println!("{}", formatted);

            if show_origin {
                // Show which scope defined this value
                if let Some(origin) = resolver.get_origin(key) {
                    println!();
                    println!(
                        "   {} Defined in: {}",
                        ds::muted("↳"),
                        ds::highlight(origin.to_string())
                    );
                }
            }
        }
        None => {
            println!(
                "{} Key not found: {}",
                ds::warning("⚠"),
                ds::highlight(key)
            );
            println!();
            println!(
                "   {} Available top-level keys: providers, sync, secrets, servers",
                ds::muted("→")
            );
            println!(
                "   {} Example: spn config get providers.anthropic.model",
                ds::muted("→")
            );
        }
    }

    Ok(())
}

async fn set_value(key: &str, value: &str, scope: &str) -> Result<()> {
    use crate::config::types::ProviderConfig;
    use rustc_hash::FxHashMap;

    let scope_type = ScopeType::from_str(scope).ok_or_else(|| {
        SpnError::ConfigError(format!(
            "Invalid scope: {}. Use: global, team, or local",
            scope
        ))
    })?;

    let cwd = env::current_dir()?;

    // Parse key path (e.g., "providers.anthropic.model")
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() {
        return Err(SpnError::ConfigError("Empty key".to_string()));
    }

    // Load config for the scope
    let mut config = match scope_type {
        ScopeType::Global => global::load()?,
        ScopeType::Team => {
            // Team scope doesn't support all config types
            return Err(SpnError::ConfigError(
                "Team scope only supports MCP servers. Use 'spn mcp add' instead.".to_string(),
            ));
        }
        ScopeType::Local => local::load(&cwd)?,
    };

    // Apply the value based on the key path
    match parts.as_slice() {
        // providers.<name>.model
        ["providers", provider_name, "model"] => {
            let provider = config
                .providers
                .entry(provider_name.to_string())
                .or_insert_with(|| ProviderConfig {
                    model: None,
                    endpoint: None,
                    extra: FxHashMap::default(),
                });
            provider.model = Some(value.to_string());
        }
        // providers.<name>.endpoint
        ["providers", provider_name, "endpoint"] => {
            let provider = config
                .providers
                .entry(provider_name.to_string())
                .or_insert_with(|| ProviderConfig {
                    model: None,
                    endpoint: None,
                    extra: FxHashMap::default(),
                });
            provider.endpoint = Some(value.to_string());
        }
        // sync.auto_sync
        ["sync", "auto_sync"] => {
            config.sync.auto_sync = value.parse::<bool>().map_err(|_| {
                SpnError::ConfigError(format!(
                    "Invalid boolean value: {}. Use 'true' or 'false'.",
                    value
                ))
            })?;
        }
        // sync.enabled_editors (comma-separated)
        ["sync", "enabled_editors"] => {
            config.sync.enabled_editors = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        // secrets.default_storage
        ["secrets", "default_storage"] => {
            let valid = ["keychain", "env", "global"];
            if !valid.contains(&value) {
                return Err(SpnError::ConfigError(format!(
                    "Invalid storage: {}. Use: keychain, env, or global",
                    value
                )));
            }
            config.secrets.default_storage = value.to_string();
        }
        // secrets.auto_migrate
        ["secrets", "auto_migrate"] => {
            config.secrets.auto_migrate = value.parse::<bool>().map_err(|_| {
                SpnError::ConfigError(format!(
                    "Invalid boolean value: {}. Use 'true' or 'false'.",
                    value
                ))
            })?;
        }
        _ => {
            return Err(SpnError::ConfigError(format!(
                "Unknown key: {}. Supported keys:\n\
                 - providers.<name>.model\n\
                 - providers.<name>.endpoint\n\
                 - sync.auto_sync\n\
                 - sync.enabled_editors\n\
                 - secrets.default_storage\n\
                 - secrets.auto_migrate",
                key
            )));
        }
    }

    // Save config
    match scope_type {
        ScopeType::Global => global::save(&config)?,
        ScopeType::Local => {
            local::save(&cwd, &config)?;
            local::ensure_gitignored(&cwd)?;
        }
        ScopeType::Team => unreachable!(),
    }

    println!(
        "{} Set {} = {} in {} scope",
        ds::success("✓"),
        ds::highlight(key),
        value,
        scope_type
    );

    Ok(())
}

async fn edit_config(local_flag: bool, user: bool, mcp: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    let path = if local_flag {
        local::config_path(&cwd)
    } else if user {
        global::config_path()?
    } else if mcp {
        team::mcp_config_path(&cwd)
    } else {
        team::package_config_path(&cwd)
    };

    // Determine and validate editor
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());
    let editor = validate_editor(&editor)?;

    if !path.exists() {
        println!("⚠️  File does not exist: {}", path.display());
        if local_flag {
            println!("   Creating local config...");
            local::save(&cwd, &Default::default())?;
            local::ensure_gitignored(&cwd)?;
        } else if user {
            println!("   Creating global config...");
            global::save(&Default::default())?;
        } else if mcp {
            println!("   Creating MCP config...");
            team::save_mcp(&cwd, &Default::default())?;
        } else {
            println!("   Run 'spn init' to create it.");
            return Ok(());
        }
    }

    println!("✏️  Opening {} with {}...", path.display(), editor);

    // Open editor
    std::process::Command::new(editor).arg(&path).status()?;

    println!("   Config saved.");

    Ok(())
}

async fn import_config(file: &str, scope: &str, skip_confirm: bool) -> Result<()> {
    use dialoguer::Confirm;
    use rustc_hash::FxHashMap;
    use std::fs;
    use std::path::Path;

    let scope_type = ScopeType::from_str(scope).ok_or_else(|| {
        SpnError::ConfigError(format!(
            "Invalid scope: {}. Use: global, team, or local",
            scope
        ))
    })?;

    println!(
        "{} Importing configuration from {}",
        ds::primary("📥"),
        ds::highlight(file)
    );
    println!("   Target scope: {}", scope_type);
    println!();

    // Check if file exists
    let path = Path::new(file);
    if !path.exists() {
        return Err(SpnError::ConfigError(format!("File not found: {}", file)));
    }

    // Read and parse file
    let content = fs::read_to_string(path)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| SpnError::ConfigError(format!("Failed to parse JSON: {}", e)))?;

    // Extract MCP servers
    let mcp_servers = if let Some(servers_obj) = parsed.get("mcpServers") {
        if let Some(obj) = servers_obj.as_object() {
            let mut servers = FxHashMap::default();
            for (name, config) in obj {
                if let Some(command) = config.get("command").and_then(|v| v.as_str()) {
                    let args = config
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let env = config
                        .get("env")
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            obj.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect()
                        })
                        .unwrap_or_default();

                    servers.insert(
                        name.clone(),
                        crate::config::types::McpServerConfig {
                            command: command.to_string(),
                            args,
                            env,
                            disabled: false,
                        },
                    );
                }
            }
            servers
        } else {
            FxHashMap::default()
        }
    } else {
        FxHashMap::default()
    };

    if mcp_servers.is_empty() {
        println!("{}", ds::warning("⚠️  No MCP servers found in file"));
        return Ok(());
    }

    // Show what will be imported
    println!("{}", ds::highlight("MCP Servers to import:"));
    for (name, server) in &mcp_servers {
        println!(
            "  {} {} {}",
            ds::primary("•"),
            ds::highlight(name),
            server.command
        );
        if !server.args.is_empty() {
            println!("    args: {:?}", server.args);
        }
        if !server.env.is_empty() {
            println!("    env: {} variables", server.env.len());
        }
    }
    println!();

    // Ask for confirmation
    if !skip_confirm {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Import {} servers into {} scope?",
                mcp_servers.len(),
                scope_type
            ))
            .default(true)
            .interact()
            .unwrap_or(false);

        if !confirmed {
            println!("{}", ds::warning("❌ Import cancelled"));
            return Ok(());
        }
    }

    // Import based on scope
    let cwd = env::current_dir()?;
    match scope_type {
        ScopeType::Global => {
            let mut config = global::load()?;
            config.servers = mcp_servers;
            global::save(&config)?;
            println!(
                "{} Imported to {}",
                ds::success("✅"),
                global::config_path()?.display()
            );
        }
        ScopeType::Team => {
            team::save_mcp(&cwd, &mcp_servers)?;
            println!(
                "{} Imported to {}",
                ds::success("✅"),
                team::mcp_config_path(&cwd).display()
            );
        }
        ScopeType::Local => {
            let mut config = local::load(&cwd)?;
            config.servers = mcp_servers;
            local::save(&cwd, &config)?;
            local::ensure_gitignored(&cwd)?;
            println!(
                "{} Imported to {}",
                ds::success("✅"),
                local::config_path(&cwd).display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_show_locations_runs() {
        let result = show_locations().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_editor_accepts_valid_editors() {
        // Simple command names
        assert!(validate_editor("vi").is_ok());
        assert!(validate_editor("vim").is_ok());
        assert!(validate_editor("nano").is_ok());
        assert!(validate_editor("code").is_ok());

        // Commands with flags
        assert!(validate_editor("code --wait").is_ok());
        assert!(validate_editor("vim -c startinsert").is_ok());
    }

    #[test]
    fn test_validate_editor_rejects_empty() {
        let result = validate_editor("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_editor_rejects_shell_metacharacters() {
        // Command injection attempts
        assert!(validate_editor("vi; rm -rf /").is_err());
        assert!(validate_editor("vi | cat /etc/passwd").is_err());
        assert!(validate_editor("vi && malicious").is_err());
        assert!(validate_editor("vi $(whoami)").is_err());
        assert!(validate_editor("vi `id`").is_err());

        // Various dangerous characters
        assert!(validate_editor("vi>output").is_err());
        assert!(validate_editor("vi<input").is_err());
        assert!(validate_editor("vi(subshell)").is_err());
        assert!(validate_editor("vi{block}").is_err());
        assert!(validate_editor("vi\nmalicious").is_err());
    }

    #[test]
    fn test_validate_editor_rejects_nonexistent_absolute_path() {
        let result = validate_editor("/nonexistent/path/to/editor");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
