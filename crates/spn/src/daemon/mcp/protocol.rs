//! MCP JSON-RPC 2.0 protocol types.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version (always "2.0").
    #[allow(dead_code)]
    pub jsonrpc: String,
    /// Request ID.
    pub id: Option<Value>,
    /// Method name.
    pub method: String,
    /// Method parameters.
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,
    /// Request ID (echoed from request).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    /// Result (if success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error (if failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// JSON-RPC 2.0 error.
#[derive(Debug, Clone, Serialize)]
pub struct McpError {
    /// Error code.
    pub code: i32,
    /// Error message.
    pub message: String,
    /// Additional data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl McpResponse {
    /// Create a success response.
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(McpError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    /// Create a notification response (no id).
    pub fn notification(result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: None,
            result: Some(result),
            error: None,
        }
    }
}

// Standard JSON-RPC error codes
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

/// MCP server info for initialize response.
#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP capabilities.
#[derive(Debug, Clone, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolCapabilities,
}

/// Tool capabilities.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCapabilities {
    /// Whether tool list changes.
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// MCP initialize result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request_deserialize_with_params() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "test"}
        }"#;

        let request: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "tools/call");
        assert_eq!(request.id, Some(serde_json::json!(1)));
        assert_eq!(request.params["name"], "test");
    }

    #[test]
    fn test_mcp_request_deserialize_without_params() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "abc-123",
            "method": "initialize"
        }"#;

        let request: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "initialize");
        assert_eq!(request.id, Some(serde_json::json!("abc-123")));
        assert!(request.params.is_null());
    }

    #[test]
    fn test_mcp_request_deserialize_notification() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }"#;

        let request: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "notifications/initialized");
        assert!(request.id.is_none());
    }

    #[test]
    fn test_mcp_response_success_serialize() {
        let response = McpResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"status": "ok"}),
        );

        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["status"], "ok");
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_mcp_response_error_serialize() {
        let response = McpResponse::error(Some(serde_json::json!(2)), PARSE_ERROR, "Invalid JSON");

        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 2);
        assert_eq!(parsed["error"]["code"], PARSE_ERROR);
        assert_eq!(parsed["error"]["message"], "Invalid JSON");
        assert!(parsed.get("result").is_none());
    }

    #[test]
    fn test_mcp_response_notification_serialize() {
        let response = McpResponse::notification(serde_json::json!({"event": "ready"}));

        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert!(parsed.get("id").is_none());
        assert_eq!(parsed["result"]["event"], "ready");
    }

    #[test]
    fn test_error_codes_are_standard_jsonrpc() {
        // JSON-RPC 2.0 standard error codes
        assert_eq!(PARSE_ERROR, -32700);
        assert_eq!(INVALID_REQUEST, -32600);
        assert_eq!(METHOD_NOT_FOUND, -32601);
        assert_eq!(INVALID_PARAMS, -32602);
        assert_eq!(INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_initialize_result_serialize() {
        let result = InitializeResult {
            protocol_version: "2024-11-05".into(),
            capabilities: ServerCapabilities {
                tools: ToolCapabilities {
                    list_changed: false,
                },
            },
            server_info: ServerInfo {
                name: "spn".into(),
                version: "0.15.2".into(),
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["protocolVersion"], "2024-11-05");
        assert_eq!(parsed["serverInfo"]["name"], "spn");
        assert_eq!(parsed["capabilities"]["tools"]["listChanged"], false);
    }
}
