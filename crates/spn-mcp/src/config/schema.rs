//! Configuration schema types for API wrappers.
//!
//! These types define the YAML structure for API wrapper configurations.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level API wrapper configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API identifier (used in tool names: {name}_{tool_name})
    pub name: String,

    /// Configuration version
    #[serde(default = "default_version")]
    pub version: String,

    /// Base URL for all API requests
    pub base_url: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,

    /// Default headers for all requests
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,

    /// Tool definitions
    pub tools: Vec<ToolDef>,
}

fn default_version() -> String {
    "1.0".into()
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    #[serde(rename = "type")]
    pub auth_type: AuthType,

    /// Credential name (resolved via spn daemon)
    pub credential: String,

    /// For api_key: header or query
    #[serde(default)]
    pub location: Option<ApiKeyLocation>,

    /// For api_key: header/param name (e.g., "X-API-Key")
    #[serde(default)]
    pub key_name: Option<String>,
}

/// Authentication type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    /// HTTP Basic Authentication (username:password base64)
    Basic,
    /// Bearer token (Authorization: Bearer <token>)
    Bearer,
    /// API key (in header or query param)
    ApiKey,
}

/// API key location.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,

    /// Burst allowance (default: 1)
    #[serde(default = "default_burst")]
    pub burst: u32,
}

fn default_burst() -> u32 {
    1
}

/// Tool definition (maps to a single MCP tool).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// Tool name (combined with API name: {api}_{name})
    pub name: String,

    /// Tool description for MCP
    #[serde(default)]
    pub description: Option<String>,

    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    #[serde(default = "default_method")]
    pub method: String,

    /// API path (appended to base_url)
    pub path: String,

    /// Request body template (Tera syntax)
    #[serde(default)]
    pub body_template: Option<String>,

    /// Parameter definitions
    #[serde(default)]
    pub params: Vec<ParamDef>,

    /// Response handling
    #[serde(default)]
    pub response: Option<ResponseConfig>,
}

fn default_method() -> String {
    "GET".into()
}

/// Parameter definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ParamDef {
    /// Parameter name
    pub name: String,

    /// Parameter type
    #[serde(rename = "type")]
    pub param_type: ParamType,

    /// For array type: item type
    #[serde(default)]
    pub items: Option<ParamType>,

    /// Whether parameter is required
    #[serde(default)]
    pub required: bool,

    /// Default value (JSON)
    #[serde(default)]
    pub default: Option<serde_json::Value>,

    /// Parameter description
    #[serde(default)]
    pub description: Option<String>,
}

/// Parameter type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
}

/// Response handling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseConfig {
    /// JSON path to extract from response
    #[serde(default)]
    pub extract: Option<String>,

    /// Template to transform response
    #[serde(default)]
    pub transform: Option<String>,
}

impl ToolDef {
    /// Generate JSON Schema for this tool's parameters.
    pub fn to_json_schema(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.params {
            let mut prop = serde_json::Map::new();

            // Type
            let type_str = match param.param_type {
                ParamType::String => "string",
                ParamType::Integer => "integer",
                ParamType::Number => "number",
                ParamType::Boolean => "boolean",
                ParamType::Array => "array",
                ParamType::Object => "object",
            };
            prop.insert("type".into(), serde_json::Value::String(type_str.into()));

            // Description
            if let Some(desc) = &param.description {
                prop.insert(
                    "description".into(),
                    serde_json::Value::String(desc.clone()),
                );
            }

            // Default
            if let Some(default) = &param.default {
                prop.insert("default".into(), default.clone());
            }

            // Array items
            if param.param_type == ParamType::Array {
                if let Some(items_type) = &param.items {
                    let items_type_str = match items_type {
                        ParamType::String => "string",
                        ParamType::Integer => "integer",
                        ParamType::Number => "number",
                        ParamType::Boolean => "boolean",
                        ParamType::Array => "array",
                        ParamType::Object => "object",
                    };
                    prop.insert(
                        "items".into(),
                        serde_json::json!({"type": items_type_str}),
                    );
                }
            }

            properties.insert(param.name.clone(), serde_json::Value::Object(prop));

            if param.required {
                required.push(serde_json::Value::String(param.name.clone()));
            }
        }

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
name: test
base_url: https://api.example.com
auth:
  type: bearer
  credential: test
tools:
  - name: get_data
    path: /data
"#;

        let config: ApiConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.auth.auth_type, AuthType::Bearer);
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.tools[0].method, "GET"); // default
    }

    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
name: dataforseo
version: "1.0"
base_url: https://api.dataforseo.com/v3
description: "DataForSEO API"
auth:
  type: basic
  credential: dataforseo
rate_limit:
  requests_per_minute: 12
  burst: 3
headers:
  Content-Type: application/json
tools:
  - name: keyword_ideas
    description: "Get keyword ideas"
    method: POST
    path: /dataforseo_labs/google/keyword_ideas/live
    body_template: |
      [{"keywords": {{ keywords | json }}}]
    params:
      - name: keywords
        type: array
        items: string
        required: true
        description: "Seed keywords"
"#;

        let config: ApiConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "dataforseo");
        assert_eq!(config.auth.auth_type, AuthType::Basic);
        assert!(config.rate_limit.is_some());
        assert_eq!(config.rate_limit.unwrap().requests_per_minute, 12);
        assert_eq!(config.tools[0].params[0].param_type, ParamType::Array);
    }

    #[test]
    fn test_to_json_schema() {
        let tool = ToolDef {
            name: "test".into(),
            description: Some("Test tool".into()),
            method: "POST".into(),
            path: "/test".into(),
            body_template: None,
            params: vec![
                ParamDef {
                    name: "query".into(),
                    param_type: ParamType::String,
                    items: None,
                    required: true,
                    default: None,
                    description: Some("Search query".into()),
                },
                ParamDef {
                    name: "limit".into(),
                    param_type: ParamType::Integer,
                    items: None,
                    required: false,
                    default: Some(serde_json::json!(10)),
                    description: None,
                },
            ],
            response: None,
        };

        let schema = tool.to_json_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"]["type"] == "string");
        assert!(schema["properties"]["limit"]["default"] == 10);
        assert!(schema["required"].as_array().unwrap().contains(&serde_json::json!("query")));
    }
}
