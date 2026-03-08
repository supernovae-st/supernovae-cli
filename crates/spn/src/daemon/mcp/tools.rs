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
