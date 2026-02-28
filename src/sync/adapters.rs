//! IDE-specific sync adapters.
//!
//! Each adapter knows how to write package configurations to IDE-specific formats.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::types::{IdeTarget, McpConfig, PackageManifest, SyncResult, SyncedItem};

/// Trait for IDE-specific sync adapters.
pub trait IdeAdapter {
    /// Get the target IDE.
    fn target(&self) -> IdeTarget;

    /// Check if this IDE is available in the given directory.
    fn is_available(&self, project_root: &Path) -> bool;

    /// Sync a package to this IDE's configuration.
    fn sync_package(
        &self,
        project_root: &Path,
        package_name: &str,
        package_path: &Path,
        manifest: &PackageManifest,
    ) -> SyncResult;

    /// Get the config file path for this IDE.
    fn config_path(&self, project_root: &Path) -> PathBuf;
}

/// Get the appropriate adapter for an IDE target.
pub fn get_adapter(target: IdeTarget) -> Box<dyn IdeAdapter> {
    match target {
        IdeTarget::ClaudeCode => Box::new(ClaudeCodeAdapter),
        IdeTarget::Cursor => Box::new(CursorAdapter),
        IdeTarget::VsCode => Box::new(VsCodeAdapter),
        IdeTarget::Windsurf => Box::new(WindsurfAdapter),
    }
}

/// Detect available IDEs in a project.
pub fn detect_ides(project_root: &Path) -> Vec<IdeTarget> {
    IdeTarget::all()
        .into_iter()
        .filter(|target| {
            let adapter = get_adapter(*target);
            adapter.is_available(project_root)
        })
        .collect()
}

// =============================================================================
// Claude Code Adapter
// =============================================================================

/// Adapter for Claude Code (.claude/).
pub struct ClaudeCodeAdapter;

impl IdeAdapter for ClaudeCodeAdapter {
    fn target(&self) -> IdeTarget {
        IdeTarget::ClaudeCode
    }

    fn is_available(&self, project_root: &Path) -> bool {
        project_root.join(".claude").exists()
    }

    fn config_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(".claude").join("settings.json")
    }

    fn sync_package(
        &self,
        project_root: &Path,
        package_name: &str,
        package_path: &Path,
        manifest: &PackageManifest,
    ) -> SyncResult {
        let mut synced = Vec::new();
        let config_path = self.config_path(project_root);

        // Load existing settings
        let mut settings = if config_path.exists() {
            let content = match std::fs::read_to_string(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    return SyncResult {
                        package: package_name.to_string(),
                        target: self.target(),
                        success: false,
                        synced: vec![],
                        error: Some(format!("Failed to read settings: {}", e)),
                    };
                }
            };
            serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        // Add MCP server if present
        if let Some(mcp) = &manifest.mcp {
            let server_name = mcp_server_name(package_name);
            let server_config = build_mcp_config(package_path, mcp);

            if let Some(servers) = settings.get_mut("mcpServers") {
                if let Some(obj) = servers.as_object_mut() {
                    obj.insert(server_name.clone(), server_config);
                }
            } else {
                settings["mcpServers"] = json!({ server_name.clone(): server_config });
            }

            synced.push(SyncedItem::McpServer(server_name));
        }

        // TODO: Add skills sync (symlink to .claude/skills/)
        // TODO: Add hooks sync (symlink to .claude/hooks/)

        // Write updated settings
        if let Err(e) = write_json_file(&config_path, &settings) {
            return SyncResult {
                package: package_name.to_string(),
                target: self.target(),
                success: false,
                synced,
                error: Some(format!("Failed to write settings: {}", e)),
            };
        }

        SyncResult {
            package: package_name.to_string(),
            target: self.target(),
            success: true,
            synced,
            error: None,
        }
    }
}

// =============================================================================
// Cursor Adapter
// =============================================================================

/// Adapter for Cursor (.cursor/).
pub struct CursorAdapter;

impl IdeAdapter for CursorAdapter {
    fn target(&self) -> IdeTarget {
        IdeTarget::Cursor
    }

    fn is_available(&self, project_root: &Path) -> bool {
        project_root.join(".cursor").exists()
    }

    fn config_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(".cursor").join("mcp.json")
    }

    fn sync_package(
        &self,
        project_root: &Path,
        package_name: &str,
        package_path: &Path,
        manifest: &PackageManifest,
    ) -> SyncResult {
        let mut synced = Vec::new();
        let config_path = self.config_path(project_root);

        // Load existing MCP config
        let mut mcp_config = if config_path.exists() {
            let content = match std::fs::read_to_string(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    return SyncResult {
                        package: package_name.to_string(),
                        target: self.target(),
                        success: false,
                        synced: vec![],
                        error: Some(format!("Failed to read mcp.json: {}", e)),
                    };
                }
            };
            serde_json::from_str(&content).unwrap_or_else(|_| json!({"mcpServers": {}}))
        } else {
            json!({"mcpServers": {}})
        };

        // Add MCP server if present
        if let Some(mcp) = &manifest.mcp {
            let server_name = mcp_server_name(package_name);
            let server_config = build_mcp_config(package_path, mcp);

            if let Some(servers) = mcp_config.get_mut("mcpServers") {
                if let Some(obj) = servers.as_object_mut() {
                    obj.insert(server_name.clone(), server_config);
                }
            } else {
                mcp_config["mcpServers"] = json!({ server_name.clone(): server_config });
            }

            synced.push(SyncedItem::McpServer(server_name));
        }

        // Write updated config
        if let Err(e) = write_json_file(&config_path, &mcp_config) {
            return SyncResult {
                package: package_name.to_string(),
                target: self.target(),
                success: false,
                synced,
                error: Some(format!("Failed to write mcp.json: {}", e)),
            };
        }

        SyncResult {
            package: package_name.to_string(),
            target: self.target(),
            success: true,
            synced,
            error: None,
        }
    }
}

// =============================================================================
// VS Code Adapter
// =============================================================================

/// Adapter for VS Code (.vscode/).
pub struct VsCodeAdapter;

impl IdeAdapter for VsCodeAdapter {
    fn target(&self) -> IdeTarget {
        IdeTarget::VsCode
    }

    fn is_available(&self, project_root: &Path) -> bool {
        project_root.join(".vscode").exists()
    }

    fn config_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(".vscode").join("settings.json")
    }

    fn sync_package(
        &self,
        _project_root: &Path,
        package_name: &str,
        _package_path: &Path,
        _manifest: &PackageManifest,
    ) -> SyncResult {
        // VS Code doesn't have native MCP support yet
        SyncResult {
            package: package_name.to_string(),
            target: self.target(),
            success: true,
            synced: vec![],
            error: None,
        }
    }
}

// =============================================================================
// Windsurf Adapter
// =============================================================================

/// Adapter for Windsurf (.windsurf/).
pub struct WindsurfAdapter;

impl IdeAdapter for WindsurfAdapter {
    fn target(&self) -> IdeTarget {
        IdeTarget::Windsurf
    }

    fn is_available(&self, project_root: &Path) -> bool {
        project_root.join(".windsurf").exists()
    }

    fn config_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(".windsurf").join("settings.json")
    }

    fn sync_package(
        &self,
        _project_root: &Path,
        package_name: &str,
        _package_path: &Path,
        _manifest: &PackageManifest,
    ) -> SyncResult {
        // Windsurf sync not yet implemented
        SyncResult {
            package: package_name.to_string(),
            target: self.target(),
            success: true,
            synced: vec![],
            error: None,
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate MCP server name from package name.
fn mcp_server_name(package_name: &str) -> String {
    // @workflows/code-review/git-pr -> workflows-code-review-git-pr
    package_name.trim_start_matches('@').replace('/', "-")
}

/// Build MCP server config JSON.
fn build_mcp_config(package_path: &Path, mcp: &McpConfig) -> Value {
    let mut config = json!({
        "command": mcp.command,
        "args": mcp.args.iter()
            .map(|arg| {
                // Replace ${PACKAGE_PATH} with actual path
                arg.replace("${PACKAGE_PATH}", &package_path.display().to_string())
            })
            .collect::<Vec<_>>()
    });

    if !mcp.env.is_empty() {
        let mut env = HashMap::new();
        for (key, value) in &mcp.env {
            env.insert(
                key.clone(),
                value.replace("${PACKAGE_PATH}", &package_path.display().to_string()),
            );
        }
        config["env"] = json!(env);
    }

    config
}

/// Write JSON to file with pretty formatting.
fn write_json_file(path: &Path, value: &Value) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_mcp_server_name() {
        assert_eq!(
            mcp_server_name("@workflows/code-review/git-pr"),
            "workflows-code-review-git-pr"
        );
        assert_eq!(mcp_server_name("@nika/seo-audit"), "nika-seo-audit");
        assert_eq!(mcp_server_name("simple-name"), "simple-name");
    }

    #[test]
    fn test_detect_ides_empty() {
        let temp = TempDir::new().unwrap();
        let ides = detect_ides(temp.path());
        assert!(ides.is_empty());
    }

    #[test]
    fn test_detect_ides_claude_code() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".claude")).unwrap();

        let ides = detect_ides(temp.path());
        assert_eq!(ides, vec![IdeTarget::ClaudeCode]);
    }

    #[test]
    fn test_claude_code_sync_mcp() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();

        let package_path = temp.path().join("packages/test-pkg/1.0.0");
        std::fs::create_dir_all(&package_path).unwrap();

        let manifest = PackageManifest {
            name: "@test/mcp-server".to_string(),
            version: "1.0.0".to_string(),
            mcp: Some(McpConfig {
                command: "node".to_string(),
                args: vec!["${PACKAGE_PATH}/dist/index.js".to_string()],
                env: HashMap::new(),
            }),
            ..Default::default()
        };

        let adapter = ClaudeCodeAdapter;
        let result =
            adapter.sync_package(temp.path(), "@test/mcp-server", &package_path, &manifest);

        assert!(result.success);
        assert_eq!(result.synced.len(), 1);

        // Verify settings.json was created
        let settings_path = claude_dir.join("settings.json");
        assert!(settings_path.exists());

        let settings: Value =
            serde_json::from_str(&std::fs::read_to_string(settings_path).unwrap()).unwrap();
        assert!(settings["mcpServers"]["test-mcp-server"].is_object());
    }

    #[test]
    fn test_build_mcp_config() {
        let package_path = PathBuf::from("/home/user/.spn/packages/test/1.0.0");
        let mcp = McpConfig {
            command: "node".to_string(),
            args: vec!["${PACKAGE_PATH}/dist/mcp.js".to_string()],
            env: HashMap::from([("API_KEY".to_string(), "secret".to_string())]),
        };

        let config = build_mcp_config(&package_path, &mcp);

        assert_eq!(config["command"], "node");
        assert_eq!(
            config["args"][0],
            "/home/user/.spn/packages/test/1.0.0/dist/mcp.js"
        );
        assert_eq!(config["env"]["API_KEY"], "secret");
    }
}
