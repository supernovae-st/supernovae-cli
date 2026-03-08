//! OpenAPI 3.0+ parser.
//!
//! Parses OpenAPI specifications and converts them to [`ApiConfig`].

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::config::{ApiConfig, AuthConfig, AuthType, ParamDef, ParamType, ToolDef};

/// Errors that can occur when parsing OpenAPI specs.
#[derive(Debug, Error)]
pub enum OpenApiError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unsupported OpenAPI version: {0}. Only 3.0+ is supported.")]
    UnsupportedVersion(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Result type for OpenAPI operations.
pub type Result<T> = std::result::Result<T, OpenApiError>;

/// OpenAPI 3.0+ specification (partial - only what we need).
#[derive(Debug, Clone, Deserialize)]
pub struct OpenApiSpec {
    /// OpenAPI version (must be 3.0+)
    pub openapi: String,

    /// API info
    pub info: OpenApiInfo,

    /// Servers (for base URL)
    #[serde(default)]
    pub servers: Vec<OpenApiServer>,

    /// Path definitions
    #[serde(default)]
    pub paths: HashMap<String, PathItem>,

    /// Security definitions
    #[serde(default)]
    pub components: Option<Components>,

    /// Top-level security requirements
    #[serde(default)]
    pub security: Vec<SecurityRequirement>,
}

/// API information.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Server definition.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenApiServer {
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Path item containing operations.
#[derive(Debug, Clone, Deserialize)]
pub struct PathItem {
    #[serde(default)]
    pub get: Option<Operation>,
    #[serde(default)]
    pub post: Option<Operation>,
    #[serde(default)]
    pub put: Option<Operation>,
    #[serde(default)]
    pub patch: Option<Operation>,
    #[serde(default)]
    pub delete: Option<Operation>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

/// Operation definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub request_body: Option<RequestBody>,
    #[serde(default)]
    pub security: Vec<SecurityRequirement>,
}

/// Parameter definition.
#[derive(Debug, Clone, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String, // path, query, header, cookie
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub schema: Option<SchemaRef>,
}

/// Schema reference or inline schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SchemaRef {
    Ref {
        #[serde(rename = "$ref")]
        reference: String,
    },
    Inline(Schema),
}

/// Inline schema definition.
#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    #[serde(rename = "type", default)]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub items: Option<Box<Schema>>,
}

/// Request body definition.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestBody {
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub content: HashMap<String, MediaType>,
}

/// Media type definition.
#[derive(Debug, Clone, Deserialize)]
pub struct MediaType {
    #[serde(default)]
    pub schema: Option<SchemaRef>,
}

/// Components section.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    #[serde(default)]
    pub security_schemes: HashMap<String, SecurityScheme>,
}

/// Security scheme definition.
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: String,
    #[serde(default)]
    pub scheme: Option<String>, // For http type: basic, bearer
    #[serde(default)]
    pub name: Option<String>, // For apiKey: header/query param name
    #[serde(rename = "in", default)]
    pub location: Option<String>, // For apiKey: header, query
}

/// Security requirement.
pub type SecurityRequirement = HashMap<String, Vec<String>>;

/// Parse an OpenAPI spec from a file.
pub fn parse_openapi(path: &Path) -> Result<OpenApiSpec> {
    let content = std::fs::read_to_string(path)?;

    // Detect format by extension or content
    let spec: OpenApiSpec =
        if path.extension().is_some_and(|e| e == "json") || content.trim().starts_with('{') {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

    // Validate version
    if !spec.openapi.starts_with("3.") {
        return Err(OpenApiError::UnsupportedVersion(spec.openapi));
    }

    Ok(spec)
}

impl OpenApiSpec {
    /// Convert the OpenAPI spec to an ApiConfig.
    pub fn to_api_config(&self, api_name: Option<&str>) -> ApiConfig {
        let name = api_name
            .map(String::from)
            .unwrap_or_else(|| slugify(&self.info.title));

        let base_url = self
            .servers
            .first()
            .map(|s| s.url.clone())
            .unwrap_or_default();

        let auth = self.detect_auth();
        let tools = self.extract_tools(&name);

        ApiConfig {
            name: name.clone(),
            version: "1.0".to_string(),
            base_url,
            description: self.info.description.clone(),
            auth,
            rate_limit: None,
            headers: None,
            tools,
        }
    }

    /// Detect authentication from security schemes.
    fn detect_auth(&self) -> AuthConfig {
        // Check components.securitySchemes
        if let Some(components) = &self.components {
            for (name, scheme) in &components.security_schemes {
                match scheme.scheme_type.as_str() {
                    "http" => {
                        if scheme.scheme.as_deref() == Some("bearer") {
                            return AuthConfig {
                                auth_type: AuthType::Bearer,
                                credential: name.clone(),
                                location: None,
                                key_name: None,
                            };
                        } else if scheme.scheme.as_deref() == Some("basic") {
                            return AuthConfig {
                                auth_type: AuthType::Basic,
                                credential: name.clone(),
                                location: None,
                                key_name: None,
                            };
                        }
                    }
                    "apiKey" => {
                        let location = match scheme.location.as_deref() {
                            Some("query") => Some(crate::config::ApiKeyLocation::Query),
                            _ => Some(crate::config::ApiKeyLocation::Header),
                        };
                        return AuthConfig {
                            auth_type: AuthType::ApiKey,
                            credential: name.clone(),
                            location,
                            key_name: scheme.name.clone(),
                        };
                    }
                    _ => {}
                }
            }
        }

        // Default to bearer token
        AuthConfig {
            auth_type: AuthType::Bearer,
            credential: "api_key".to_string(),
            location: None,
            key_name: None,
        }
    }

    /// Extract tools from paths.
    fn extract_tools(&self, api_name: &str) -> Vec<ToolDef> {
        let mut tools = Vec::new();

        for (path, item) in &self.paths {
            // Collect path-level parameters
            let path_params: Vec<_> = item.parameters.iter().collect();

            // Process each HTTP method
            if let Some(op) = &item.get {
                tools.push(self.operation_to_tool(api_name, "GET", path, op, &path_params));
            }
            if let Some(op) = &item.post {
                tools.push(self.operation_to_tool(api_name, "POST", path, op, &path_params));
            }
            if let Some(op) = &item.put {
                tools.push(self.operation_to_tool(api_name, "PUT", path, op, &path_params));
            }
            if let Some(op) = &item.patch {
                tools.push(self.operation_to_tool(api_name, "PATCH", path, op, &path_params));
            }
            if let Some(op) = &item.delete {
                tools.push(self.operation_to_tool(api_name, "DELETE", path, op, &path_params));
            }
        }

        // Sort by name for consistent ordering
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }

    /// Convert an operation to a ToolDef.
    fn operation_to_tool(
        &self,
        api_name: &str,
        method: &str,
        path: &str,
        op: &Operation,
        path_params: &[&Parameter],
    ) -> ToolDef {
        // Generate tool name
        let name = op
            .operation_id
            .clone()
            .unwrap_or_else(|| generate_tool_name(api_name, method, path));

        // Merge path-level and operation-level parameters
        let mut params = Vec::new();
        for param in path_params.iter().copied() {
            params.push(parameter_to_param_def(param));
        }
        for param in &op.parameters {
            params.push(parameter_to_param_def(param));
        }

        // Use summary or description
        let description = op.summary.clone().or_else(|| op.description.clone());

        ToolDef {
            name,
            description,
            method: method.to_string(),
            path: path.to_string(),
            body_template: None,
            params,
            response: None,
        }
    }

    /// Get tools filtered by tag.
    pub fn tools_by_tag(&self, tag: &str) -> Vec<(&str, &str, &Operation)> {
        let mut results = Vec::new();

        for (path, item) in &self.paths {
            let ops = [
                ("GET", &item.get),
                ("POST", &item.post),
                ("PUT", &item.put),
                ("PATCH", &item.patch),
                ("DELETE", &item.delete),
            ];

            for (method, op_opt) in ops {
                if let Some(op) = op_opt {
                    if op.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                        results.push((path.as_str(), method, op));
                    }
                }
            }
        }

        results
    }

    /// Get all unique tags.
    pub fn tags(&self) -> Vec<String> {
        let mut tags = std::collections::HashSet::new();

        for item in self.paths.values() {
            let ops: [&Option<Operation>; 5] =
                [&item.get, &item.post, &item.put, &item.patch, &item.delete];
            for op_opt in ops.into_iter().flatten() {
                for tag in &op_opt.tags {
                    tags.insert(tag.clone());
                }
            }
        }

        let mut sorted: Vec<_> = tags.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Count total endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.paths
            .values()
            .map(|item| {
                [
                    item.get.is_some(),
                    item.post.is_some(),
                    item.put.is_some(),
                    item.patch.is_some(),
                    item.delete.is_some(),
                ]
                .iter()
                .filter(|&&b| b)
                .count()
            })
            .sum()
    }
}

/// Convert a Parameter to ParamDef.
fn parameter_to_param_def(param: &Parameter) -> ParamDef {
    let param_type = param
        .schema
        .as_ref()
        .map(schema_to_param_type)
        .unwrap_or(ParamType::String);

    let required = param.location == "path" || param.required.unwrap_or(false);

    ParamDef {
        name: param.name.clone(),
        param_type,
        items: None,
        required,
        default: None,
        description: param.description.clone(),
    }
}

/// Convert schema type to ParamType.
fn schema_to_param_type(schema: &SchemaRef) -> ParamType {
    match schema {
        SchemaRef::Ref { .. } => ParamType::Object,
        SchemaRef::Inline(s) => match s.schema_type.as_deref() {
            Some("integer") => ParamType::Integer,
            Some("number") => ParamType::Number,
            Some("boolean") => ParamType::Boolean,
            Some("array") => ParamType::Array,
            Some("object") => ParamType::Object,
            _ => ParamType::String,
        },
    }
}

/// Generate a tool name from method and path.
fn generate_tool_name(api_name: &str, method: &str, path: &str) -> String {
    let path_part: String = path
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{'))
        .collect::<Vec<_>>()
        .join("_");

    let method_prefix = match method.to_uppercase().as_str() {
        "GET" => "get",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "call",
    };

    let name = format!("{}_{}", method_prefix, path_part);
    let name = name.trim_matches('_');

    // If empty, use api name
    if name.is_empty() {
        format!("{}_{}", api_name, method.to_lowercase())
    } else {
        name.to_string()
    }
}

/// Convert a string to a slug (lowercase, underscores).
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("GitHub API"), "github_api");
        assert_eq!(slugify("My-Cool_API v2"), "my_cool_api_v2");
        assert_eq!(slugify("  spaces  "), "spaces");
    }

    #[test]
    fn test_generate_tool_name() {
        assert_eq!(
            generate_tool_name("github", "GET", "/repos/{owner}/{repo}"),
            "get_repos"
        );
        assert_eq!(
            generate_tool_name("github", "POST", "/repos/{owner}/{repo}/issues"),
            "create_repos_issues"
        );
        assert_eq!(
            generate_tool_name("github", "DELETE", "/repos/{owner}/{repo}"),
            "delete_repos"
        );
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test API
  version: "1.0"
servers:
  - url: https://api.example.com
paths:
  /users:
    get:
      operationId: listUsers
      summary: List all users
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
"#;
        let spec: OpenApiSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.paths.len(), 1);
        assert!(spec.paths.get("/users").unwrap().get.is_some());
    }

    #[test]
    fn test_to_api_config() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test API
  version: "1.0"
servers:
  - url: https://api.example.com
paths:
  /users:
    get:
      operationId: listUsers
      summary: List users
  /users/{id}:
    get:
      operationId: getUser
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
"#;
        let spec: OpenApiSpec = serde_yaml::from_str(yaml).unwrap();
        let config = spec.to_api_config(None);

        assert_eq!(config.name, "test_api");
        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.tools.len(), 2);
    }

    #[test]
    fn test_detect_bearer_auth() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0"
paths: {}
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
"#;
        let spec: OpenApiSpec = serde_yaml::from_str(yaml).unwrap();
        let config = spec.to_api_config(None);

        assert_eq!(config.auth.auth_type, AuthType::Bearer);
        assert_eq!(config.auth.credential, "bearerAuth");
    }

    #[test]
    fn test_detect_api_key_auth() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0"
paths: {}
components:
  securitySchemes:
    apiKey:
      type: apiKey
      name: X-API-Key
      in: header
"#;
        let spec: OpenApiSpec = serde_yaml::from_str(yaml).unwrap();
        let config = spec.to_api_config(None);

        assert_eq!(config.auth.auth_type, AuthType::ApiKey);
        assert_eq!(config.auth.key_name, Some("X-API-Key".to_string()));
    }

    #[test]
    fn test_endpoint_count() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0"
paths:
  /a:
    get: {}
    post: {}
  /b:
    delete: {}
"#;
        let spec: OpenApiSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.endpoint_count(), 3);
    }

    #[test]
    fn test_tags() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0"
paths:
  /users:
    get:
      tags: [users, admin]
  /posts:
    get:
      tags: [posts]
"#;
        let spec: OpenApiSpec = serde_yaml::from_str(yaml).unwrap();
        let tags = spec.tags();
        assert_eq!(tags, vec!["admin", "posts", "users"]);
    }
}
