//! MCP server implementation over stdio.

use secrecy::ExposeSecret;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use super::protocol::{
    InitializeResult, McpRequest, McpResponse, ServerCapabilities, ServerInfo, ToolCapabilities,
    METHOD_NOT_FOUND,
};
use super::tools::{list_tools, ModelRunParams, SecretsCheckParams, SecretsGetParams, ToolResult};
use crate::daemon::{ModelManager, SecretManager};

/// MCP server that communicates over stdio.
pub struct McpServer {
    secrets: Arc<SecretManager>,
    models: Arc<ModelManager>,
    version: String,
}

impl McpServer {
    /// Create a new MCP server.
    pub fn new(secrets: Arc<SecretManager>, models: Arc<ModelManager>) -> Self {
        Self {
            secrets,
            models,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Run the MCP server over stdio.
    ///
    /// Reads JSON-RPC requests from stdin, processes them, and writes responses to stdout.
    pub async fn run(self) -> std::io::Result<()> {
        info!("Starting MCP server over stdio");

        let server = Arc::new(Mutex::new(self));
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        let mut stdout = std::io::stdout();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    error!("Error reading stdin: {}", e);
                    break;
                }
            };

            if line.is_empty() {
                continue;
            }

            debug!("Received: {}", line);

            // Parse request
            let request: McpRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let response = McpResponse::error(None, -32700, format!("Parse error: {}", e));
                    let _ = writeln!(stdout, "{}", serde_json::to_string(&response)?);
                    let _ = stdout.flush();
                    continue;
                }
            };

            // Handle request
            let server = server.lock().await;
            let response = server.handle_request(request).await;

            // Write response
            let response_str = serde_json::to_string(&response)?;
            debug!("Sending: {}", response_str);
            writeln!(stdout, "{}", response_str)?;
            stdout.flush()?;
        }

        info!("MCP server shutting down");
        Ok(())
    }

    /// Handle a single MCP request.
    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "initialized" => McpResponse::success(id, json!({})),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tool_call(id, request.params).await,
            "ping" => McpResponse::success(id, json!({})),
            _ => McpResponse::error(
                id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> McpResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".into(),
            capabilities: ServerCapabilities {
                tools: ToolCapabilities {
                    list_changed: false,
                },
            },
            server_info: ServerInfo {
                name: "spn-daemon".into(),
                version: self.version.clone(),
            },
        };

        McpResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_tools_list(&self, id: Option<Value>) -> McpResponse {
        let tools = list_tools();
        McpResponse::success(id, json!({ "tools": tools }))
    }

    async fn handle_tool_call(&self, id: Option<Value>, params: Value) -> McpResponse {
        // Extract tool name and arguments
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = match name {
            "spn_secrets_get" => self.tool_secrets_get(arguments).await,
            "spn_secrets_list" => self.tool_secrets_list().await,
            "spn_secrets_check" => self.tool_secrets_check(arguments).await,
            "spn_model_list" => self.tool_model_list().await,
            "spn_model_run" => self.tool_model_run(arguments).await,
            "spn_status" => self.tool_status().await,
            _ => ToolResult::error(format!("Unknown tool: {}", name)),
        };

        McpResponse::success(id, serde_json::to_value(result).unwrap())
    }

    async fn tool_secrets_get(&self, arguments: Value) -> ToolResult {
        let params: SecretsGetParams = match serde_json::from_value(arguments) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        match self.secrets.get_cached(&params.provider).await {
            Some(secret) => {
                // Return masked secret for security
                let value = secret.expose_secret();
                let masked = if value.len() > 8 {
                    format!("{}...{}", &value[..4], &value[value.len() - 4..])
                } else {
                    "****".to_string()
                };
                ToolResult::text(format!(
                    "Secret for '{}' is configured: {}",
                    params.provider, masked
                ))
            }
            None => ToolResult::error(format!("No secret found for provider: {}", params.provider)),
        }
    }

    async fn tool_secrets_list(&self) -> ToolResult {
        let providers = self.secrets.list_cached().await;

        if providers.is_empty() {
            ToolResult::text("No providers configured")
        } else {
            let list = providers
                .into_iter()
                .map(|p| format!("- {}", p))
                .collect::<Vec<_>>()
                .join("\n");
            ToolResult::text(format!("Configured providers:\n{}", list))
        }
    }

    async fn tool_secrets_check(&self, arguments: Value) -> ToolResult {
        let params: SecretsCheckParams = match serde_json::from_value(arguments) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let exists = self.secrets.has_cached(&params.provider).await;
        if exists {
            ToolResult::text(format!("Provider '{}' is configured", params.provider))
        } else {
            ToolResult::text(format!("Provider '{}' is NOT configured", params.provider))
        }
    }

    async fn tool_model_list(&self) -> ToolResult {
        match self.models.list_models().await {
            Ok(models) => {
                if models.is_empty() {
                    ToolResult::text("No models installed. Run 'spn model pull <name>' to install.")
                } else {
                    let list = models
                        .into_iter()
                        .map(|m| {
                            let size = format_size(m.size);
                            format!("- {} [{}]", m.name, size)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    ToolResult::text(format!("Local models:\n{}", list))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to list models: {}", e)),
        }
    }

    async fn tool_model_run(&self, arguments: Value) -> ToolResult {
        let params: ModelRunParams = match serde_json::from_value(arguments) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        match self
            .models
            .run_inference(
                &params.model,
                &params.prompt,
                params.system,
                params.temperature,
            )
            .await
        {
            Ok(response) => ToolResult::text(response),
            Err(e) => ToolResult::error(format!("Model inference failed: {}", e)),
        }
    }

    async fn tool_status(&self) -> ToolResult {
        let providers = self.secrets.list_cached().await;
        let models = self.models.list_models().await.unwrap_or_default();

        let status = format!(
            "spn daemon v{}\n\nProviders: {} configured\nModels: {} installed",
            self.version,
            providers.len(),
            models.len()
        );

        ToolResult::text(status)
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    }
}
