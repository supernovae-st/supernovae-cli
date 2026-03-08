//! Dynamic MCP handler implementation.
//!
//! Registers tools at runtime based on YAML configurations.
//! Uses rmcp 0.16 API with macro-free dynamic registration.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use reqwest::Client;
use rmcp::model::{
    AnnotateAble, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
    ListPromptsResult, ListResourcesResult, PaginatedRequestParams, Prompt, PromptArgument,
    PromptMessage, PromptMessageRole, RawResource, ReadResourceRequestParams, ReadResourceResult,
    Resource, ResourceContents, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::stdio;
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt};
use secrecy::ExposeSecret;
use serde_json::Value;
use tera::{Context as TeraContext, Tera};

use crate::config::{ApiConfig, ApiKeyLocation, AuthConfig, AuthType, ToolDef};
use crate::error::{Error, Result};

use super::rate_limit::{check_limit, create_limiter, ApiRateLimiter};

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
    /// Per-API rate limiters
    rate_limiters: HashMap<String, Arc<ApiRateLimiter>>,
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
        let mut rate_limiters = HashMap::new();

        for config in configs {
            // Resolve credential for this API
            let credential = resolve_credential(&config.auth).await?;
            credentials.insert(config.name.clone(), credential);

            // Create rate limiter if configured
            if let Some(ref rate_limit) = config.rate_limit {
                let limiter = create_limiter(rate_limit);
                rate_limiters.insert(config.name.clone(), limiter);
                tracing::debug!(
                    "Rate limiter for '{}': {} req/min, burst {}",
                    config.name,
                    rate_limit.requests_per_minute,
                    rate_limit.burst
                );
            }

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
            rate_limiters,
        })
    }

    /// Get list of registered tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get deduplicated list of API configs.
    ///
    /// Multiple tools may share the same API config, so we deduplicate by name.
    fn api_configs(&self) -> Vec<&ApiConfig> {
        let mut seen = HashSet::new();
        self.tools
            .values()
            .filter_map(|entry| {
                if seen.insert(&entry.api_config.name) {
                    Some(entry.api_config.as_ref())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Create a sanitized version of an API config (no credentials exposed).
    fn sanitized_api_config(config: &ApiConfig) -> serde_json::Value {
        let tool_names: Vec<&str> = config.tools.iter().map(|t| t.name.as_str()).collect();
        serde_json::json!({
            "name": config.name,
            "version": config.version,
            "base_url": config.base_url,
            "description": config.description,
            "tools": tool_names,
            "rate_limit": config.rate_limit.as_ref().map(|rl| serde_json::json!({
                "requests_per_minute": rl.requests_per_minute,
                "burst": rl.burst
            }))
        })
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

        let description = entry.tool_def.description.clone().unwrap_or_else(|| {
            format!(
                "{} {} {}",
                entry.tool_def.method, entry.api_config.base_url, entry.tool_def.path
            )
        });

        Tool::new(entry.full_name.clone(), description, input_schema)
    }

    /// Build an MCP prompt from a tool entry.
    fn build_prompt(entry: &ToolEntry) -> Prompt {
        let description = entry.tool_def.description.clone().unwrap_or_else(|| {
            format!(
                "{} {} {}",
                entry.tool_def.method, entry.api_config.base_url, entry.tool_def.path
            )
        });

        let arguments: Vec<PromptArgument> = entry
            .tool_def
            .params
            .iter()
            .map(|param| PromptArgument {
                name: param.name.clone(),
                title: None,
                description: param.description.clone(),
                required: Some(param.required),
            })
            .collect();

        Prompt::new(
            entry.full_name.clone(),
            Some(description),
            if arguments.is_empty() {
                None
            } else {
                Some(arguments)
            },
        )
    }

    /// Build a prompt message with usage instructions for a tool.
    fn build_prompt_message(entry: &ToolEntry) -> PromptMessage {
        let mut message = format!(
            "# Tool: {}\n\n**Endpoint:** {} {}{}\n\n",
            entry.full_name, entry.tool_def.method, entry.api_config.base_url, entry.tool_def.path
        );

        if let Some(desc) = &entry.tool_def.description {
            message.push_str(&format!("**Description:** {}\n\n", desc));
        }

        if !entry.tool_def.params.is_empty() {
            message.push_str("## Parameters\n\n");
            for param in &entry.tool_def.params {
                let required = if param.required { " (required)" } else { "" };
                let param_type = format!("{:?}", param.param_type).to_lowercase();
                message.push_str(&format!("- **{}**: {}{}", param.name, param_type, required));
                if let Some(desc) = &param.description {
                    message.push_str(&format!(" - {}", desc));
                }
                message.push('\n');
            }
            message.push('\n');
        }

        message.push_str("## Usage\n\n");
        message.push_str(&format!(
            "Call this tool with the name `{}` and provide the required parameters.\n",
            entry.full_name
        ));

        PromptMessage::new_text(PromptMessageRole::User, message)
    }

    /// Execute a tool with the given parameters.
    async fn execute_tool(&self, entry: &ToolEntry, params: Value) -> Result<Value> {
        let config = &entry.api_config;
        let tool = &entry.tool_def;

        // Security: Check payload size before processing
        validate_payload_size(&params)?;

        // Security: Validate parameter types match schema
        validate_parameters(&tool.params, &params)?;

        // Check rate limit before making request
        if let Some(limiter) = self.rate_limiters.get(&config.name) {
            check_limit(limiter, &config.name)?;
        }

        // Security: Validate tool path before building URL
        validate_tool_path(&tool.path)?;

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
            _ => {
                return Err(Error::ConfigValidation(format!(
                    "Unknown method: {}",
                    tool.method
                )))
            }
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
            request = request
                .body(body)
                .header("Content-Type", "application/json");
        }

        // Execute request
        tracing::debug!("Executing {} {}", tool.method, url);
        let response = request.send().await.map_err(Error::Http)?;

        // Check status
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| {
                tracing::warn!("Failed to read error response body: {}", e);
                String::new()
            });
            return Err(Error::Mcp(format!("API returned {}: {}", status, body)));
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
    ///
    /// # Security
    /// Validates template doesn't contain dangerous directives that could
    /// lead to file inclusion or code execution.
    fn render_template(&self, template: &str, params: &Value) -> Result<String> {
        // Security: Prevent template injection attacks
        const FORBIDDEN_DIRECTIVES: &[&str] = &[
            "{% include",
            "{% import",
            "{% extends",
            "{% macro",
            "{%include",
            "{%import",
            "{%extends",
            "{%macro",
        ];

        let template_lower = template.to_lowercase();
        for directive in FORBIDDEN_DIRECTIVES {
            if template_lower.contains(directive) {
                return Err(Error::ConfigValidation(format!(
                    "Template contains forbidden directive: {}",
                    directive
                )));
            }
        }

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
                .enable_prompts()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::ListToolsResult, McpError>>
           + Send
           + '_ {
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
    ) -> impl std::future::Future<Output = std::result::Result<CallToolResult, McpError>> + Send + '_
    {
        async move {
            let name = request.name.as_ref();
            let entry = self
                .tools
                .get(name)
                .ok_or_else(|| McpError::invalid_params(format!("Unknown tool: {}", name), None))?;

            let params = request
                .arguments
                .map(|m| Value::Object(m.into_iter().collect()))
                .unwrap_or(Value::Object(Default::default()));

            match self.execute_tool(entry, params).await {
                Ok(result) => {
                    let json = serde_json::to_string_pretty(&result).map_err(|e| {
                        McpError::internal_error(format!("Serialization error: {}", e), None)
                    })?;
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error: {}",
                    e
                ))])),
            }
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<ListPromptsResult, McpError>> + Send + '_
    {
        async move {
            let prompts: Vec<Prompt> = self.tools.values().map(Self::build_prompt).collect();
            Ok(ListPromptsResult::with_all_items(prompts))
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<GetPromptResult, McpError>> + Send + '_
    {
        async move {
            let name = &request.name;
            let entry = self.tools.get(name).ok_or_else(|| {
                McpError::invalid_params(format!("Unknown prompt: {}", name), None)
            })?;

            let message = Self::build_prompt_message(entry);

            Ok(GetPromptResult {
                description: entry.tool_def.description.clone(),
                messages: vec![message],
            })
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<ListResourcesResult, McpError>> + Send + '_
    {
        async move {
            let resources: Vec<Resource> = self
                .api_configs()
                .into_iter()
                .map(|config| {
                    let uri = format!("api://{}", config.name);
                    let description = config
                        .description
                        .clone()
                        .unwrap_or_else(|| format!("{} API configuration", config.name));

                    let mut raw = RawResource::new(uri, config.name.clone());
                    raw.description = Some(description);
                    raw.mime_type = Some("application/json".into());

                    raw.optional_annotate(None)
                })
                .collect();

            Ok(ListResourcesResult::with_all_items(resources))
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<ReadResourceResult, McpError>> + Send + '_
    {
        async move {
            let uri: &str = request.uri.as_ref();

            // Parse URI: api://{name}
            let api_name = uri.strip_prefix("api://").ok_or_else(|| {
                McpError::invalid_params(
                    format!(
                        "Invalid resource URI: {}. Expected format: api://{{name}}",
                        uri
                    ),
                    None,
                )
            })?;

            // Find the API config
            let config = self
                .api_configs()
                .into_iter()
                .find(|c| c.name == api_name)
                .ok_or_else(|| {
                    McpError::invalid_params(format!("Unknown API: {}", api_name), None)
                })?;

            // Return sanitized config (no credentials!)
            let sanitized = Self::sanitized_api_config(config);
            let json = serde_json::to_string_pretty(&sanitized).map_err(|e| {
                McpError::internal_error(format!("Serialization error: {}", e), None)
            })?;

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(json, uri)],
            })
        }
    }
}

/// Resolve credential via spn-client (daemon) or env fallback.
async fn resolve_credential(auth: &AuthConfig) -> Result<String> {
    // Try spn-client first (with auto-fallback to env vars)
    match spn_client::SpnClient::connect_with_fallback().await {
        Ok(mut client) => match client.get_secret(&auth.credential).await {
            Ok(secret) => {
                tracing::debug!("Resolved credential '{}' via spn-client", auth.credential);
                return Ok(secret.expose_secret().to_string());
            }
            Err(e) => {
                tracing::warn!("Failed to resolve '{}': {}", auth.credential, e);
            }
        },
        Err(e) => {
            tracing::debug!("Could not connect to spn daemon: {}", e);
        }
    }

    // Manual env var fallback for non-registered providers
    let env_var = format!(
        "{}_API_KEY",
        auth.credential.to_uppercase().replace('-', "_")
    );
    if let Ok(value) = std::env::var(&env_var) {
        tracing::debug!(
            "Resolved credential '{}' via env {}",
            auth.credential,
            env_var
        );
        return Ok(value);
    }

    Err(Error::Credential(
        auth.credential.clone(),
        format!("Not found in daemon or environment (tried {})", env_var),
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

/// Validate a tool path to prevent URL injection attacks.
///
/// # Security
/// Prevents:
/// - Path traversal (e.g., `/../../../etc/passwd`)
/// - Protocol-relative URLs (e.g., `//attacker.com/api`)
/// - Full URLs in path (e.g., `https://attacker.com`)
fn validate_tool_path(path: &str) -> Result<()> {
    // Path must start with /
    if !path.starts_with('/') {
        return Err(Error::ConfigValidation(format!(
            "Tool path '{}' must start with '/'",
            path
        )));
    }

    // Reject path traversal
    if path.contains("..") {
        return Err(Error::ConfigValidation(format!(
            "Tool path '{}' contains path traversal sequence '..'",
            path
        )));
    }

    // Reject protocol-relative URLs (//host)
    if path.starts_with("//") {
        return Err(Error::ConfigValidation(format!(
            "Tool path '{}' looks like a protocol-relative URL",
            path
        )));
    }

    // Reject embedded URLs
    if path.contains("://") {
        return Err(Error::ConfigValidation(format!(
            "Tool path '{}' contains URL scheme",
            path
        )));
    }

    Ok(())
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

/// Maximum payload size in bytes (10 MB).
const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024;

/// Maximum string parameter length (100 KB).
const MAX_STRING_LENGTH: usize = 100 * 1024;

/// Validate payload size to prevent DoS via large requests.
fn validate_payload_size(params: &Value) -> Result<()> {
    let size = serde_json::to_string(params)
        .map(|s| s.len())
        .unwrap_or(0);

    if size > MAX_PAYLOAD_SIZE {
        return Err(Error::ConfigValidation(format!(
            "Payload too large: {} bytes (max: {} bytes)",
            size, MAX_PAYLOAD_SIZE
        )));
    }

    Ok(())
}

/// Validate parameter types match their definitions.
///
/// # Security
/// Ensures type safety before passing data to APIs, preventing:
/// - Type confusion attacks
/// - Injection via unexpected types
/// - DoS via oversized strings
fn validate_parameters(param_defs: &[crate::config::ParamDef], params: &Value) -> Result<()> {
    let obj = match params.as_object() {
        Some(o) => o,
        None if param_defs.is_empty() => return Ok(()),
        None => {
            return Err(Error::ConfigValidation(
                "Parameters must be an object".into(),
            ))
        }
    };

    for def in param_defs {
        match obj.get(&def.name) {
            Some(value) => {
                // Validate type matches
                validate_param_type(&def.name, &def.param_type, value)?;

                // Validate string length
                if let Value::String(s) = value {
                    if s.len() > MAX_STRING_LENGTH {
                        return Err(Error::ConfigValidation(format!(
                            "Parameter '{}' too long: {} chars (max: {})",
                            def.name,
                            s.len(),
                            MAX_STRING_LENGTH
                        )));
                    }
                }
            }
            None if def.required => {
                return Err(Error::ConfigValidation(format!(
                    "Missing required parameter: {}",
                    def.name
                )));
            }
            None => {} // Optional parameter not provided, OK
        }
    }

    Ok(())
}

/// Validate a single parameter value matches its expected type.
fn validate_param_type(
    name: &str,
    expected: &crate::config::ParamType,
    value: &Value,
) -> Result<()> {
    use crate::config::ParamType;

    let type_matches = match (expected, value) {
        (ParamType::String, Value::String(_)) => true,
        (ParamType::Integer, Value::Number(n)) => n.is_i64(),
        (ParamType::Number, Value::Number(_)) => true,
        (ParamType::Boolean, Value::Bool(_)) => true,
        (ParamType::Array, Value::Array(_)) => true,
        (ParamType::Object, Value::Object(_)) => true,
        _ => false,
    };

    if !type_matches {
        let got_type = match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };
        return Err(Error::ConfigValidation(format!(
            "Parameter '{}' has wrong type: expected {:?}, got {}",
            name, expected, got_type
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ParamDef;

    // Helper to create a test handler with tools
    fn create_test_handler() -> DynamicHandler {
        let mut tools = HashMap::new();

        // Tool 1: Simple tool with no parameters
        let api_config1 = Arc::new(ApiConfig {
            name: "test".to_string(),
            version: "1.0".to_string(),
            base_url: "https://api.example.com".to_string(),
            description: Some("Test API".to_string()),
            auth: AuthConfig {
                auth_type: AuthType::Bearer,
                credential: "test".to_string(),
                location: None,
                key_name: None,
            },
            rate_limit: None,
            headers: None,
            tools: vec![],
        });

        let tool_def1 = ToolDef {
            name: "simple".to_string(),
            description: Some("A simple test tool".to_string()),
            method: "GET".to_string(),
            path: "/simple".to_string(),
            body_template: None,
            params: vec![],
            response: None,
        };

        tools.insert(
            "test_simple".to_string(),
            ToolEntry {
                full_name: "test_simple".to_string(),
                api_config: Arc::clone(&api_config1),
                tool_def: tool_def1,
            },
        );

        // Tool 2: Tool with parameters
        let tool_def2 = ToolDef {
            name: "search".to_string(),
            description: Some("Search for items".to_string()),
            method: "POST".to_string(),
            path: "/search".to_string(),
            body_template: Some(r#"{"query": "{{ query }}"}"#.to_string()),
            params: vec![
                ParamDef {
                    name: "query".to_string(),
                    param_type: crate::config::ParamType::String,
                    items: None,
                    required: true,
                    default: None,
                    description: Some("The search query".to_string()),
                },
                ParamDef {
                    name: "limit".to_string(),
                    param_type: crate::config::ParamType::Integer,
                    items: None,
                    required: false,
                    default: Some(serde_json::json!(10)),
                    description: Some("Maximum number of results".to_string()),
                },
            ],
            response: None,
        };

        tools.insert(
            "test_search".to_string(),
            ToolEntry {
                full_name: "test_search".to_string(),
                api_config: Arc::clone(&api_config1),
                tool_def: tool_def2,
            },
        );

        DynamicHandler {
            tools,
            http_client: Client::new(),
            tera: Tera::default(),
            credentials: HashMap::new(),
            rate_limiters: HashMap::new(),
        }
    }

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

    #[test]
    fn test_validate_tool_path_valid() {
        assert!(validate_tool_path("/api/v1/endpoint").is_ok());
        assert!(validate_tool_path("/").is_ok());
        assert!(validate_tool_path("/users/{id}").is_ok());
        assert!(validate_tool_path("/search?q=test").is_ok());
    }

    #[test]
    fn test_validate_tool_path_rejects_no_leading_slash() {
        assert!(validate_tool_path("api/endpoint").is_err());
        assert!(validate_tool_path("").is_err());
    }

    #[test]
    fn test_validate_tool_path_rejects_path_traversal() {
        assert!(validate_tool_path("/../etc/passwd").is_err());
        assert!(validate_tool_path("/api/../../../secret").is_err());
        assert!(validate_tool_path("/api/..").is_err());
    }

    #[test]
    fn test_validate_tool_path_rejects_protocol_relative() {
        assert!(validate_tool_path("//attacker.com/api").is_err());
        assert!(validate_tool_path("//evil.com").is_err());
    }

    #[test]
    fn test_validate_tool_path_rejects_embedded_url() {
        assert!(validate_tool_path("/redirect?url=https://evil.com").is_err());
        assert!(validate_tool_path("/api/http://bad.com").is_err());
    }

    #[test]
    fn test_render_template_rejects_include_directive() {
        let handler = DynamicHandler {
            tools: HashMap::new(),
            http_client: Client::new(),
            tera: Tera::default(),
            credentials: HashMap::new(),
            rate_limiters: HashMap::new(),
        };

        // Should reject {% include %}
        let result =
            handler.render_template(r#"{% include "secret.txt" %}"#, &serde_json::json!({}));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("forbidden directive"));

        // Should reject {% import %}
        let result = handler.render_template(
            r#"{% import "macros.html" as macros %}"#,
            &serde_json::json!({}),
        );
        assert!(result.is_err());

        // Should reject {% extends %}
        let result =
            handler.render_template(r#"{% extends "base.html" %}"#, &serde_json::json!({}));
        assert!(result.is_err());

        // Should reject case variations
        let result =
            handler.render_template(r#"{% INCLUDE "secret.txt" %}"#, &serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_render_template_allows_safe_directives() {
        let handler = DynamicHandler {
            tools: HashMap::new(),
            http_client: Client::new(),
            tera: Tera::default(),
            credentials: HashMap::new(),
            rate_limiters: HashMap::new(),
        };

        // Should allow {{ }} variable substitution
        let result = handler.render_template(
            r#"{"name": "{{ name }}"}"#,
            &serde_json::json!({"name": "test"}),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"name": "test"}"#);

        // Should allow {% for %} loops
        let result = handler.render_template(
            r#"[{% for item in items %}"{{ item }}"{% if not loop.last %},{% endif %}{% endfor %}]"#,
            &serde_json::json!({"items": ["a", "b"]}),
        );
        assert!(result.is_ok());

        // Should allow {% if %} conditions
        let result = handler.render_template(
            r#"{% if enabled %}on{% else %}off{% endif %}"#,
            &serde_json::json!({"enabled": true}),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "on");
    }

    // ===== Prompt Tests =====

    #[test]
    fn test_list_prompts_returns_all_tools() {
        let handler = create_test_handler();

        // Build prompts from tools
        let prompts: Vec<Prompt> = handler
            .tools
            .values()
            .map(DynamicHandler::build_prompt)
            .collect();

        // Should have 2 prompts (one per tool)
        assert_eq!(prompts.len(), 2);

        // Verify prompt names match tool names
        let prompt_names: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
        assert!(prompt_names.contains(&"test_simple"));
        assert!(prompt_names.contains(&"test_search"));
    }

    #[test]
    fn test_get_prompt_returns_usage_instructions() {
        let handler = create_test_handler();

        // Get the simple tool entry
        let entry = handler.tools.get("test_simple").unwrap();
        let message = DynamicHandler::build_prompt_message(entry);

        // Verify message content
        if let rmcp::model::PromptMessageContent::Text { text } = &message.content {
            assert!(text.contains("test_simple"), "Should contain tool name");
            assert!(text.contains("GET"), "Should contain HTTP method");
            assert!(text.contains("/simple"), "Should contain path");
            assert!(
                text.contains("https://api.example.com"),
                "Should contain base URL"
            );
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_get_prompt_includes_parameters() {
        let handler = create_test_handler();

        // Get the search tool entry (has parameters)
        let entry = handler.tools.get("test_search").unwrap();
        let message = DynamicHandler::build_prompt_message(entry);

        // Verify message content includes parameters
        if let rmcp::model::PromptMessageContent::Text { text } = &message.content {
            assert!(
                text.contains("Parameters"),
                "Should have Parameters section"
            );
            assert!(text.contains("query"), "Should contain query parameter");
            assert!(
                text.contains("(required)"),
                "Should indicate required parameter"
            );
            assert!(text.contains("limit"), "Should contain limit parameter");
            assert!(
                text.contains("The search query"),
                "Should contain parameter description"
            );
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_get_prompt_rejects_unknown_tool() {
        let handler = create_test_handler();

        // Try to get a non-existent tool
        let result = handler.tools.get("unknown_tool");
        assert!(result.is_none(), "Should not find unknown tool");
    }

    #[test]
    fn test_build_prompt_has_correct_structure() {
        let handler = create_test_handler();

        let entry = handler.tools.get("test_search").unwrap();
        let prompt = DynamicHandler::build_prompt(entry);

        assert_eq!(prompt.name, "test_search");
        assert!(prompt.description.is_some());
        assert_eq!(prompt.description.unwrap(), "Search for items");

        // Should have arguments
        assert!(prompt.arguments.is_some());
        let args = prompt.arguments.unwrap();
        assert_eq!(args.len(), 2);

        // First argument: query (required)
        assert_eq!(args[0].name, "query");
        assert_eq!(args[0].required, Some(true));
        assert_eq!(args[0].description, Some("The search query".to_string()));

        // Second argument: limit (optional)
        assert_eq!(args[1].name, "limit");
        assert_eq!(args[1].required, Some(false));
    }

    #[test]
    fn test_build_prompt_no_args_when_empty() {
        let handler = create_test_handler();

        let entry = handler.tools.get("test_simple").unwrap();
        let prompt = DynamicHandler::build_prompt(entry);

        assert_eq!(prompt.name, "test_simple");
        // Should be None when no parameters
        assert!(prompt.arguments.is_none());
    }

    // ===== Validation Tests =====

    #[test]
    fn test_validate_payload_size_accepts_normal_payload() {
        let params = serde_json::json!({
            "query": "test search",
            "limit": 10
        });
        assert!(validate_payload_size(&params).is_ok());
    }

    #[test]
    fn test_validate_payload_size_rejects_oversized() {
        // Create a large payload (> 10MB)
        let large_string = "x".repeat(11 * 1024 * 1024);
        let params = serde_json::json!({ "data": large_string });
        let result = validate_payload_size(&params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Payload too large"));
    }

    #[test]
    fn test_validate_parameters_accepts_correct_types() {
        let params = vec![
            ParamDef {
                name: "query".into(),
                param_type: crate::config::ParamType::String,
                items: None,
                required: true,
                default: None,
                description: None,
            },
            ParamDef {
                name: "limit".into(),
                param_type: crate::config::ParamType::Integer,
                items: None,
                required: false,
                default: None,
                description: None,
            },
        ];

        let args = serde_json::json!({
            "query": "test",
            "limit": 10
        });

        assert!(validate_parameters(&params, &args).is_ok());
    }

    #[test]
    fn test_validate_parameters_rejects_wrong_type() {
        let params = vec![ParamDef {
            name: "count".into(),
            param_type: crate::config::ParamType::Integer,
            items: None,
            required: true,
            default: None,
            description: None,
        }];

        let args = serde_json::json!({ "count": "not-an-integer" });
        let result = validate_parameters(&params, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrong type"));
    }

    #[test]
    fn test_validate_parameters_rejects_missing_required() {
        let params = vec![ParamDef {
            name: "required_field".into(),
            param_type: crate::config::ParamType::String,
            items: None,
            required: true,
            default: None,
            description: None,
        }];

        let args = serde_json::json!({});
        let result = validate_parameters(&params, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing required"));
    }

    #[test]
    fn test_validate_parameters_accepts_missing_optional() {
        let params = vec![ParamDef {
            name: "optional_field".into(),
            param_type: crate::config::ParamType::String,
            items: None,
            required: false,
            default: None,
            description: None,
        }];

        let args = serde_json::json!({});
        assert!(validate_parameters(&params, &args).is_ok());
    }

    #[test]
    fn test_validate_parameters_rejects_oversized_string() {
        let params = vec![ParamDef {
            name: "query".into(),
            param_type: crate::config::ParamType::String,
            items: None,
            required: true,
            default: None,
            description: None,
        }];

        // Create string > 100KB
        let large_string = "x".repeat(101 * 1024);
        let args = serde_json::json!({ "query": large_string });
        let result = validate_parameters(&params, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_validate_param_type_all_types() {
        use crate::config::ParamType;

        // String
        assert!(validate_param_type("s", &ParamType::String, &serde_json::json!("test")).is_ok());
        assert!(validate_param_type("s", &ParamType::String, &serde_json::json!(123)).is_err());

        // Integer
        assert!(validate_param_type("i", &ParamType::Integer, &serde_json::json!(42)).is_ok());
        assert!(validate_param_type("i", &ParamType::Integer, &serde_json::json!(3.14)).is_err());

        // Number
        assert!(validate_param_type("n", &ParamType::Number, &serde_json::json!(3.14)).is_ok());
        assert!(validate_param_type("n", &ParamType::Number, &serde_json::json!(42)).is_ok());
        assert!(validate_param_type("n", &ParamType::Number, &serde_json::json!("42")).is_err());

        // Boolean
        assert!(validate_param_type("b", &ParamType::Boolean, &serde_json::json!(true)).is_ok());
        assert!(validate_param_type("b", &ParamType::Boolean, &serde_json::json!("true")).is_err());

        // Array
        assert!(validate_param_type("a", &ParamType::Array, &serde_json::json!([1, 2, 3])).is_ok());
        assert!(validate_param_type("a", &ParamType::Array, &serde_json::json!("array")).is_err());

        // Object
        assert!(
            validate_param_type("o", &ParamType::Object, &serde_json::json!({"key": "value"}))
                .is_ok()
        );
        assert!(validate_param_type("o", &ParamType::Object, &serde_json::json!([1, 2])).is_err());
    }
}
