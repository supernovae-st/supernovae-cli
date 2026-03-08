//! Dynamic MCP handler implementation.
//!
//! Registers tools at runtime based on YAML configurations.
//! Uses rmcp 0.16 API with macro-free dynamic registration.

use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Client;
use rmcp::model::{
    CallToolResult, Content, PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::stdio;
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt};
use secrecy::ExposeSecret;
use serde_json::Value;
use tera::{Context as TeraContext, Tera};

use crate::config::{ApiConfig, ApiKeyLocation, AuthConfig, AuthType, ToolDef};
use crate::error::{Error, Result};

/// Tool registration entry.
struct ToolEntry {
    /// Full tool name (api_name + _ + tool_name)
    full_name: String,
    /// API configuration
    api_config: Arc<ApiConfig>,
    /// Tool definition
    tool_def: ToolDef,
}

/// Dynamic MCP handler that serves tools based on YAML configurations.
pub struct DynamicHandler {
    /// Registered tools by full name
    tools: HashMap<String, ToolEntry>,
    /// HTTP client for making API requests
    http_client: Client,
    /// Tera template engine
    tera: Tera,
    /// Resolved credentials cache (api_name -> credential value)
    credentials: HashMap<String, String>,
}

impl DynamicHandler {
    /// Create a new handler with the given API configurations.
    pub async fn new(configs: Vec<ApiConfig>) -> Result<Self> {
        let http_client = Client::builder()
            .user_agent("spn-mcp/0.1.0")
            .build()
            .map_err(Error::Http)?;

        let tera = Tera::default();
        let mut tools = HashMap::new();
        let mut credentials = HashMap::new();

        for config in configs {
            // Resolve credential for this API
            let credential = resolve_credential(&config.auth).await?;
            credentials.insert(config.name.clone(), credential);

            let config = Arc::new(config);

            // Register each tool
            for tool_def in &config.tools {
                let full_name = format!("{}_{}", config.name, tool_def.name);

                tools.insert(
                    full_name.clone(),
                    ToolEntry {
                        full_name,
                        api_config: Arc::clone(&config),
                        tool_def: tool_def.clone(),
                    },
                );
            }
        }

        Ok(Self {
            tools,
            http_client,
            tera,
            credentials,
        })
    }

    /// Get list of registered tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Run MCP server on stdio transport.
    pub async fn run(self) -> Result<()> {
        // Create stdio transport and run the MCP server
        let server = self
            .serve(stdio())
            .await
            .map_err(|e| Error::Mcp(e.to_string()))?;

        // Wait for completion
        server
            .waiting()
            .await
            .map_err(|e| Error::Mcp(e.to_string()))?;

        Ok(())
    }

    /// Build an MCP tool from a tool entry.
    fn build_tool(entry: &ToolEntry) -> Tool {
        let schema = entry.tool_def.to_json_schema();
        let input_schema = match schema {
            Value::Object(map) => Arc::new(map),
            _ => Arc::new(serde_json::Map::new()),
        };

        let description = entry
            .tool_def
            .description
            .clone()
            .unwrap_or_else(|| {
                format!(
                    "{} {} {}",
                    entry.tool_def.method, entry.api_config.base_url, entry.tool_def.path
                )
            });

        Tool::new(entry.full_name.clone(), description, input_schema)
    }

    /// Execute a tool with the given parameters.
    async fn execute_tool(&self, entry: &ToolEntry, params: Value) -> Result<Value> {
        let config = &entry.api_config;
        let tool = &entry.tool_def;

        // Build URL
        let url = format!("{}{}", config.base_url, tool.path);

        // Get credential
        let credential = self
            .credentials
            .get(&config.name)
            .ok_or_else(|| Error::Credential(config.name.clone(), "not found".into()))?;

        // Build request
        let mut request = match tool.method.to_uppercase().as_str() {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "DELETE" => self.http_client.delete(&url),
            "PATCH" => self.http_client.patch(&url),
            _ => return Err(Error::ConfigValidation(format!("Unknown method: {}", tool.method))),
        };

        // Add authentication
        request = apply_auth(request, &config.auth, credential);

        // Add default headers
        if let Some(headers) = &config.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        // Add body if template exists
        if let Some(template) = &tool.body_template {
            let body = self.render_template(template, &params)?;
            request = request.body(body).header("Content-Type", "application/json");
        }

        // Execute request
        tracing::debug!("Executing {} {}", tool.method, url);
        let response = request.send().await.map_err(Error::Http)?;

        // Check status
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Mcp(format!(
                "API returned {}: {}",
                status, body
            )));
        }

        // Parse response
        let body: Value = response.json().await.map_err(Error::Http)?;

        // Apply response extraction/transformation
        let result = if let Some(ref response_config) = tool.response {
            if let Some(ref extract) = response_config.extract {
                extract_json_path(&body, extract)?
            } else {
                body
            }
        } else {
            body
        };

        Ok(result)
    }

    /// Render a Tera template with parameters.
    fn render_template(&self, template: &str, params: &Value) -> Result<String> {
        let mut context = TeraContext::new();

        // Add all params to context
        if let Value::Object(map) = params {
            for (key, value) in map {
                context.insert(key, value);
            }
        }

        // Create a one-off template
        let mut tera = self.tera.clone();
        tera.add_raw_template("body", template)?;
        let rendered = tera.render("body", &context)?;

        Ok(rendered)
    }
}

// MCP ServerHandler implementation (rmcp 0.16)
impl ServerHandler for DynamicHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Dynamic REST-to-MCP wrapper. Tools are loaded from ~/.spn/apis/".into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::ListToolsResult, McpError>> + Send + '_ {
        async move {
            let tools: Vec<Tool> = self.tools.values().map(Self::build_tool).collect();
            Ok(rmcp::model::ListToolsResult::with_all_items(tools))
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let name = request.name.as_ref();
            let entry = self.tools.get(name).ok_or_else(|| {
                McpError::invalid_params(format!("Unknown tool: {}", name), None)
            })?;

            let params = request.arguments
                .map(|m| Value::Object(m.into_iter().collect()))
                .unwrap_or(Value::Object(Default::default()));

            match self.execute_tool(entry, params).await {
                Ok(result) => {
                    let json = serde_json::to_string_pretty(&result)
                        .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
                Err(e) => {
                    Ok(CallToolResult::error(vec![Content::text(format!("Error: {}", e))]))
                }
            }
        }
    }
}

/// Resolve credential via spn-client (daemon) or env fallback.
async fn resolve_credential(auth: &AuthConfig) -> Result<String> {
    // Try spn-client first (with auto-fallback to env vars)
    match spn_client::SpnClient::connect_with_fallback().await {
        Ok(mut client) => {
            match client.get_secret(&auth.credential).await {
                Ok(secret) => {
                    tracing::debug!("Resolved credential '{}' via spn-client", auth.credential);
                    return Ok(secret.expose_secret().to_string());
                }
                Err(e) => {
                    tracing::warn!("Failed to resolve '{}': {}", auth.credential, e);
                }
            }
        }
        Err(e) => {
            tracing::debug!("Could not connect to spn daemon: {}", e);
        }
    }

    // Manual env var fallback for non-registered providers
    let env_var = format!("{}_API_KEY", auth.credential.to_uppercase().replace('-', "_"));
    if let Ok(value) = std::env::var(&env_var) {
        tracing::debug!("Resolved credential '{}' via env {}", auth.credential, env_var);
        return Ok(value);
    }

    Err(Error::Credential(
        auth.credential.clone(),
        format!(
            "Not found in daemon or environment (tried {})",
            env_var
        ),
    ))
}

/// Apply authentication to a request.
fn apply_auth(
    request: reqwest::RequestBuilder,
    auth: &AuthConfig,
    credential: &str,
) -> reqwest::RequestBuilder {
    match auth.auth_type {
        AuthType::Basic => {
            // Credential format: "username:password"
            let parts: Vec<&str> = credential.splitn(2, ':').collect();
            if parts.len() == 2 {
                request.basic_auth(parts[0], Some(parts[1]))
            } else {
                // Treat as username with no password
                request.basic_auth(credential, None::<&str>)
            }
        }
        AuthType::Bearer => request.bearer_auth(credential),
        AuthType::ApiKey => {
            let key_name = auth.key_name.as_deref().unwrap_or("X-API-Key");
            match auth.location {
                Some(ApiKeyLocation::Query) => request.query(&[(key_name, credential)]),
                _ => request.header(key_name, credential),
            }
        }
    }
}

/// Extract a value from JSON using a simple dot-notation path.
fn extract_json_path(value: &Value, path: &str) -> Result<Value> {
    let mut current = value.clone();

    for part in path.split('.') {
        // Handle array index: "items[0]"
        if let Some(bracket_pos) = part.find('[') {
            let key = &part[..bracket_pos];
            let index_str = &part[bracket_pos + 1..part.len() - 1];
            let index: usize = index_str
                .parse()
                .map_err(|_| Error::Mcp(format!("Invalid array index: {}", index_str)))?;

            if !key.is_empty() {
                current = current
                    .get(key)
                    .cloned()
                    .ok_or_else(|| Error::Mcp(format!("Path not found: {}", key)))?;
            }
            current = current
                .get(index)
                .cloned()
                .ok_or_else(|| Error::Mcp(format!("Index out of bounds: {}", index)))?;
        } else {
            current = current
                .get(part)
                .cloned()
                .ok_or_else(|| Error::Mcp(format!("Path not found: {}", part)))?;
        }
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_path_simple() {
        let value = serde_json::json!({
            "data": {
                "items": [1, 2, 3]
            }
        });

        let result = extract_json_path(&value, "data.items").unwrap();
        assert_eq!(result, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_extract_json_path_array() {
        let value = serde_json::json!({
            "tasks": [
                {"result": "a"},
                {"result": "b"}
            ]
        });

        let result = extract_json_path(&value, "tasks[0].result").unwrap();
        assert_eq!(result, serde_json::json!("a"));
    }

    #[test]
    fn test_apply_auth_basic() {
        // Just verify it compiles - we can't easily test the actual auth header
        let client = Client::new();
        let request = client.get("http://example.com");
        let auth = AuthConfig {
            auth_type: AuthType::Basic,
            credential: "user:pass".into(),
            location: None,
            key_name: None,
        };
        let _ = apply_auth(request, &auth, "user:pass");
    }
}
