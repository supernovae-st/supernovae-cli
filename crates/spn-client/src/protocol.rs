//! Protocol types for daemon communication.
//!
//! The protocol uses length-prefixed JSON over Unix sockets.
//!
//! ## Wire Format
//!
//! ```text
//! [4 bytes: message length (big-endian u32)][JSON payload]
//! ```
//!
//! ## Example
//!
//! Request:
//! ```json
//! { "cmd": "GET_SECRET", "provider": "anthropic" }
//! ```
//!
//! Response:
//! ```json
//! { "ok": true, "secret": "sk-ant-..." }
//! ```

use serde::{Deserialize, Serialize};
use spn_core::{LoadConfig, ModelInfo, RunningModel};

/// Request sent to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum Request {
    /// Ping the daemon to check it's alive.
    #[serde(rename = "PING")]
    Ping,

    /// Get a secret for a provider.
    #[serde(rename = "GET_SECRET")]
    GetSecret { provider: String },

    /// Check if a secret exists.
    #[serde(rename = "HAS_SECRET")]
    HasSecret { provider: String },

    /// List all available providers.
    #[serde(rename = "LIST_PROVIDERS")]
    ListProviders,

    // ==================== Model Commands ====================

    /// List all installed models.
    #[serde(rename = "MODEL_LIST")]
    ModelList,

    /// Pull/download a model.
    #[serde(rename = "MODEL_PULL")]
    ModelPull { name: String },

    /// Load a model into memory.
    #[serde(rename = "MODEL_LOAD")]
    ModelLoad {
        name: String,
        #[serde(default)]
        config: Option<LoadConfig>,
    },

    /// Unload a model from memory.
    #[serde(rename = "MODEL_UNLOAD")]
    ModelUnload { name: String },

    /// Get status of running models.
    #[serde(rename = "MODEL_STATUS")]
    ModelStatus,

    /// Delete a model.
    #[serde(rename = "MODEL_DELETE")]
    ModelDelete { name: String },
}

/// Response from the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// Successful ping response.
    Pong { version: String },

    /// Secret value response.
    ///
    /// # Security Note
    ///
    /// The secret is transmitted as plain JSON over the Unix socket. This is secure because:
    /// - Unix socket requires peer credential verification (same UID only)
    /// - Socket permissions are 0600 (owner-only)
    /// - Connection is local-only (no network exposure)
    Secret { value: String },

    /// Secret existence check response.
    Exists { exists: bool },

    /// Provider list response.
    Providers { providers: Vec<String> },

    // ==================== Model Responses ====================

    /// List of installed models.
    Models { models: Vec<ModelInfo> },

    /// List of currently running/loaded models.
    RunningModels { running: Vec<RunningModel> },

    /// Generic success response.
    Success { success: bool },

    /// Error response.
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let ping = Request::Ping;
        let json = serde_json::to_string(&ping).unwrap();
        assert_eq!(json, r#"{"cmd":"PING"}"#);

        let get_secret = Request::GetSecret {
            provider: "anthropic".to_string(),
        };
        let json = serde_json::to_string(&get_secret).unwrap();
        assert_eq!(json, r#"{"cmd":"GET_SECRET","provider":"anthropic"}"#);

        let has_secret = Request::HasSecret {
            provider: "openai".to_string(),
        };
        let json = serde_json::to_string(&has_secret).unwrap();
        assert_eq!(json, r#"{"cmd":"HAS_SECRET","provider":"openai"}"#);

        let list = Request::ListProviders;
        let json = serde_json::to_string(&list).unwrap();
        assert_eq!(json, r#"{"cmd":"LIST_PROVIDERS"}"#);
    }

    #[test]
    fn test_response_deserialization() {
        // Pong
        let json = r#"{"version":"0.9.0"}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response, Response::Pong { version } if version == "0.9.0"));

        // Secret
        let json = r#"{"value":"sk-test-123"}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response, Response::Secret { value } if value == "sk-test-123"));

        // Exists
        let json = r#"{"exists":true}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response, Response::Exists { exists } if exists));

        // Providers
        let json = r#"{"providers":["anthropic","openai"]}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(
            matches!(response, Response::Providers { providers } if providers == vec!["anthropic", "openai"])
        );

        // Error
        let json = r#"{"message":"Not found"}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response, Response::Error { message } if message == "Not found"));
    }
}
