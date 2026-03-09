//! MCP tool definitions for spn daemon.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// MCP tool definition.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// All available tool names.
#[allow(dead_code)]
pub const TOOL_NAMES: &[&str] = &[
    "spn_secrets_get",
    "spn_secrets_list",
    "spn_secrets_check",
    "spn_model_list",
    "spn_model_run",
    "spn_status",
];

/// Get tool definitions for MCP.
pub fn list_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "spn_secrets_get".into(),
            description: "Get an API secret for a provider from the secure keychain".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "provider": {
                        "type": "string",
                        "description": "Provider name (e.g., anthropic, openai, neo4j)"
                    }
                },
                "required": ["provider"]
            }),
        },
        Tool {
            name: "spn_secrets_list".into(),
            description: "List all configured providers and their status".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "spn_secrets_check".into(),
            description: "Check if a provider has a secret configured".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "provider": {
                        "type": "string",
                        "description": "Provider name to check"
                    }
                },
                "required": ["provider"]
            }),
        },
        Tool {
            name: "spn_model_list".into(),
            description: "List local Ollama models".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "spn_model_run".into(),
            description: "Run inference on a local Ollama model".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "model": {
                        "type": "string",
                        "description": "Model name (e.g., llama3.2:3b)"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Prompt to send to the model"
                    },
                    "system": {
                        "type": "string",
                        "description": "Optional system prompt"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "Temperature (0.0-2.0, default 0.7)"
                    }
                },
                "required": ["model", "prompt"]
            }),
        },
        Tool {
            name: "spn_status".into(),
            description: "Get spn daemon status and configuration".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Tool call parameters.
#[derive(Debug, Deserialize)]
pub struct SecretsGetParams {
    pub provider: String,
}

#[derive(Debug, Deserialize)]
pub struct SecretsCheckParams {
    pub provider: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelRunParams {
    pub model: String,
    pub prompt: String,
    pub system: Option<String>,
    pub temperature: Option<f32>,
}

/// Tool call result.
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
}

impl ToolResult {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: content.into(),
            }],
            is_error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: message.into(),
            }],
            is_error: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tools_returns_all_tools() {
        let tools = list_tools();

        assert_eq!(tools.len(), 6, "Expected 6 tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"spn_secrets_get"));
        assert!(names.contains(&"spn_secrets_list"));
        assert!(names.contains(&"spn_secrets_check"));
        assert!(names.contains(&"spn_model_list"));
        assert!(names.contains(&"spn_model_run"));
        assert!(names.contains(&"spn_status"));
    }

    #[test]
    fn test_tool_names_constant_matches_list_tools() {
        let tools = list_tools();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        for name in TOOL_NAMES {
            assert!(
                tool_names.contains(name),
                "TOOL_NAMES contains '{}' but list_tools() doesn't return it",
                name
            );
        }

        for name in &tool_names {
            assert!(
                TOOL_NAMES.contains(name),
                "list_tools() returns '{}' but TOOL_NAMES doesn't contain it",
                name
            );
        }
    }

    #[test]
    fn test_tool_schemas_are_valid_json_schema() {
        let tools = list_tools();

        for tool in &tools {
            // Every schema must be an object
            assert!(
                tool.input_schema.is_object(),
                "Tool '{}' schema is not an object",
                tool.name
            );

            // Every schema must have "type": "object"
            assert_eq!(
                tool.input_schema["type"], "object",
                "Tool '{}' schema type is not 'object'",
                tool.name
            );

            // Every schema must have "properties"
            assert!(
                tool.input_schema.get("properties").is_some(),
                "Tool '{}' schema has no 'properties' field",
                tool.name
            );
        }
    }

    #[test]
    fn test_secrets_get_requires_provider() {
        let tools = list_tools();
        let tool = tools.iter().find(|t| t.name == "spn_secrets_get").unwrap();

        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("provider")));
    }

    #[test]
    fn test_model_run_requires_model_and_prompt() {
        let tools = list_tools();
        let tool = tools.iter().find(|t| t.name == "spn_model_run").unwrap();

        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("model")));
        assert!(required.contains(&serde_json::json!("prompt")));
    }

    #[test]
    fn test_tool_result_text_serialize() {
        let result = ToolResult::text("Hello, world!");
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Hello, world!");
        assert!(parsed.get("isError").is_none());
    }

    #[test]
    fn test_tool_result_error_serialize() {
        let result = ToolResult::error("Something went wrong");
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Something went wrong");
        assert_eq!(parsed["isError"], true);
    }

    #[test]
    fn test_secrets_get_params_deserialize() {
        let json = r#"{"provider": "anthropic"}"#;
        let params: SecretsGetParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.provider, "anthropic");
    }

    #[test]
    fn test_model_run_params_deserialize() {
        let json = r#"{
            "model": "llama3.2:3b",
            "prompt": "Hello",
            "system": "Be helpful",
            "temperature": 0.7
        }"#;
        let params: ModelRunParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.model, "llama3.2:3b");
        assert_eq!(params.prompt, "Hello");
        assert_eq!(params.system, Some("Be helpful".to_string()));
        assert_eq!(params.temperature, Some(0.7));
    }

    #[test]
    fn test_model_run_params_optional_fields() {
        let json = r#"{"model": "llama3.2:3b", "prompt": "Hello"}"#;
        let params: ModelRunParams = serde_json::from_str(json).unwrap();

        assert!(params.system.is_none());
        assert!(params.temperature.is_none());
    }
}
