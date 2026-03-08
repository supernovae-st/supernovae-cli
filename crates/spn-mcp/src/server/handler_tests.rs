//! Additional tests for the DynamicHandler.
//!
//! This module provides comprehensive test coverage for the handler's
//! critical paths including JSON path extraction, template rendering,
//! authentication, and tool building.

use super::*;
use crate::config::{ApiConfig, AuthConfig, AuthType, ToolDef};
use std::collections::HashMap;
use std::sync::Arc;

// ===== TEST UTILITIES =====

/// Create a minimal valid ApiConfig for testing.
fn mock_api_config(name: &str) -> ApiConfig {
    ApiConfig {
        name: name.into(),
        version: "1.0".into(),
        base_url: "https://api.example.com".into(),
        description: Some("Test API".into()),
        auth: AuthConfig {
            auth_type: AuthType::Bearer,
            credential: name.into(),
            location: None,
            key_name: None,
        },
        rate_limit: None,
        headers: None,
        tools: vec![],
    }
}

/// Create a tool definition for testing.
fn mock_tool_def(name: &str, method: &str, path: &str) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: Some(format!("Test tool: {}", name)),
        method: method.into(),
        path: path.into(),
        body_template: None,
        params: vec![],
        response: None,
    }
}

/// Create a tool entry for testing.
fn mock_tool_entry(api_name: &str, tool_name: &str, method: &str, path: &str) -> ToolEntry {
    let config = Arc::new(mock_api_config(api_name));
    ToolEntry {
        full_name: format!("{}_{}", api_name, tool_name),
        api_config: config,
        tool_def: mock_tool_def(tool_name, method, path),
    }
}

/// Create a handler with pre-configured credentials.
fn mock_handler_with_creds(creds: Vec<(&str, &str)>) -> DynamicHandler {
    let credentials: HashMap<String, String> = creds
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();

    DynamicHandler {
        tools: HashMap::new(),
        http_client: Client::new(),
        tera: Tera::default(),
        credentials,
    }
}

// ===== TOOL BUILDING TESTS =====

#[test]
fn test_build_tool_generates_valid_schema() {
    let entry = mock_tool_entry("test", "search", "GET", "/search");
    let tool = DynamicHandler::build_tool(&entry);

    assert_eq!(tool.name.as_ref(), "test_search");
    let desc = tool.description.as_deref().expect("Tool should have description");
    assert!(
        desc.contains("search") || desc.contains("Test"),
        "Description should mention the tool: {}",
        desc
    );
}

#[test]
fn test_build_tool_auto_description() {
    let config = Arc::new(mock_api_config("myapi"));
    let entry = ToolEntry {
        full_name: "myapi_fetch".into(),
        api_config: config,
        tool_def: ToolDef {
            name: "fetch".into(),
            description: None,
            method: "GET".into(),
            path: "/data".into(),
            body_template: None,
            params: vec![],
            response: None,
        },
    };

    let tool = DynamicHandler::build_tool(&entry);
    let desc = tool.description.as_deref().expect("Tool should have auto-generated description");
    assert!(desc.contains("GET"));
    assert!(desc.contains("/data"));
}

#[test]
fn test_handler_tool_names() {
    let mut handler = mock_handler_with_creds(vec![]);

    handler.tools.insert(
        "api_tool1".into(),
        mock_tool_entry("api", "tool1", "GET", "/t1"),
    );
    handler.tools.insert(
        "api_tool2".into(),
        mock_tool_entry("api", "tool2", "POST", "/t2"),
    );
    handler.tools.insert(
        "other_action".into(),
        mock_tool_entry("other", "action", "PUT", "/a"),
    );

    let names = handler.tool_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"api_tool1".to_string()));
    assert!(names.contains(&"api_tool2".to_string()));
    assert!(names.contains(&"other_action".to_string()));
}

// ===== JSON PATH EXTRACTION TESTS =====

#[test]
fn test_extract_json_path_nested() {
    let json = serde_json::json!({
        "response": {
            "data": {
                "items": [{"id": 1}, {"id": 2}]
            }
        }
    });

    let result = extract_json_path(&json, "response.data.items[0].id").unwrap();
    assert_eq!(result, serde_json::json!(1));
}

#[test]
fn test_extract_json_path_missing_key() {
    let json = serde_json::json!({"foo": "bar"});
    let result = extract_json_path(&json, "missing.path");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_extract_json_path_invalid_index() {
    let json = serde_json::json!({"items": [1, 2, 3]});
    let result = extract_json_path(&json, "items[99]");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of bounds"));
}

#[test]
fn test_extract_json_path_invalid_index_format() {
    let json = serde_json::json!({"items": [1, 2, 3]});
    let result = extract_json_path(&json, "items[abc]");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid array index"));
}

#[test]
fn test_extract_json_path_root_array() {
    let json = serde_json::json!([{"name": "first"}, {"name": "second"}]);
    let result = extract_json_path(&json, "[0].name").unwrap();
    assert_eq!(result, serde_json::json!("first"));
}

#[test]
fn test_extract_json_path_second_array_element() {
    let json = serde_json::json!({
        "data": [
            {"value": "first"},
            {"value": "second"},
            {"value": "third"}
        ]
    });
    let result = extract_json_path(&json, "data[1].value").unwrap();
    assert_eq!(result, serde_json::json!("second"));
}

#[test]
fn test_extract_json_path_deeply_nested() {
    let json = serde_json::json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "value": 42
                    }
                }
            }
        }
    });
    let result = extract_json_path(&json, "level1.level2.level3.level4.value").unwrap();
    assert_eq!(result, serde_json::json!(42));
}

// ===== AUTHENTICATION TESTS =====

#[test]
fn test_apply_auth_bearer() {
    let client = Client::new();
    let request = client.get("http://example.com");
    let auth = AuthConfig {
        auth_type: AuthType::Bearer,
        credential: "token".into(),
        location: None,
        key_name: None,
    };
    let _ = apply_auth(request, &auth, "my-secret-token");
}

#[test]
fn test_apply_auth_api_key_header() {
    let client = Client::new();
    let request = client.get("http://example.com");
    let auth = AuthConfig {
        auth_type: AuthType::ApiKey,
        credential: "key".into(),
        location: Some(ApiKeyLocation::Header),
        key_name: Some("X-Custom-Key".into()),
    };
    let _ = apply_auth(request, &auth, "secret123");
}

#[test]
fn test_apply_auth_api_key_query() {
    let client = Client::new();
    let request = client.get("http://example.com");
    let auth = AuthConfig {
        auth_type: AuthType::ApiKey,
        credential: "key".into(),
        location: Some(ApiKeyLocation::Query),
        key_name: Some("api_key".into()),
    };
    let _ = apply_auth(request, &auth, "secret123");
}

#[test]
fn test_apply_auth_api_key_default_header_name() {
    let client = Client::new();
    let request = client.get("http://example.com");
    let auth = AuthConfig {
        auth_type: AuthType::ApiKey,
        credential: "key".into(),
        location: None,
        key_name: None,
    };
    let _ = apply_auth(request, &auth, "secret");
}

#[test]
fn test_apply_auth_basic_with_colon() {
    let client = Client::new();
    let request = client.get("http://example.com");
    let auth = AuthConfig {
        auth_type: AuthType::Basic,
        credential: "basic".into(),
        location: None,
        key_name: None,
    };
    let _ = apply_auth(request, &auth, "user:password");
}

#[test]
fn test_apply_auth_basic_without_colon() {
    let client = Client::new();
    let request = client.get("http://example.com");
    let auth = AuthConfig {
        auth_type: AuthType::Basic,
        credential: "basic".into(),
        location: None,
        key_name: None,
    };
    let _ = apply_auth(request, &auth, "onlyusername");
}

// ===== TEMPLATE RENDERING TESTS =====

#[test]
fn test_render_template_with_json_filter() {
    let handler = mock_handler_with_creds(vec![]);
    let result = handler.render_template(
        r#"{"items": {{ items | json_encode() }}}"#,
        &serde_json::json!({"items": ["a", "b", "c"]}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains(r#"["a","b","c"]"#) || output.contains(r#"["a", "b", "c"]"#));
}

#[test]
fn test_render_template_preserves_content() {
    // Tera only auto-escapes for .html templates by default
    // Our JSON API body templates don't need HTML escaping
    let handler = mock_handler_with_creds(vec![]);
    let result = handler.render_template(
        r#"{"content": "{{ content }}"}"#,
        &serde_json::json!({"content": "test value"}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("test value"));
}

#[test]
fn test_render_template_raw_filter() {
    let handler = mock_handler_with_creds(vec![]);
    let result = handler.render_template(
        r#"{{ content | safe }}"#,
        &serde_json::json!({"content": "<b>bold</b>"}),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("<b>bold</b>"));
}

#[test]
fn test_render_template_rejects_macro_directive() {
    let handler = mock_handler_with_creds(vec![]);
    let result = handler.render_template(
        r#"{% macro input(name) %}<input name="{{ name }}">{% endmacro %}"#,
        &serde_json::json!({}),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("forbidden directive"));
}

// ===== PATH VALIDATION TESTS =====

#[test]
fn test_validate_tool_path_with_template_vars() {
    assert!(validate_tool_path("/users/{user_id}").is_ok());
    assert!(validate_tool_path("/api/{version}/items/{id}").is_ok());
    assert!(validate_tool_path("/search/{query}").is_ok());
}

#[test]
fn test_validate_tool_path_with_query_params() {
    assert!(validate_tool_path("/search?q=test&limit=10").is_ok());
    assert!(validate_tool_path("/api?key=value").is_ok());
}

#[test]
fn test_validate_tool_path_allows_double_slash_in_path() {
    let result = validate_tool_path("/api//endpoint");
    assert!(result.is_ok());
}

// ===== SERVER INFO TESTS =====

#[test]
fn test_server_info_capabilities() {
    let handler = mock_handler_with_creds(vec![]);
    let info = handler.get_info();
    assert!(info.instructions.is_some());
    assert!(info.instructions.unwrap().contains("REST-to-MCP"));
}

// ===== MOCK UTILITY TESTS =====

#[test]
fn test_mock_api_config_creates_valid_config() {
    let config = mock_api_config("test_api");
    assert_eq!(config.name, "test_api");
    assert_eq!(config.version, "1.0");
    assert_eq!(config.base_url, "https://api.example.com");
    assert!(config.description.is_some());
    assert_eq!(config.auth.auth_type, AuthType::Bearer);
    assert_eq!(config.auth.credential, "test_api");
}

#[test]
fn test_mock_tool_entry_creates_valid_entry() {
    let entry = mock_tool_entry("myapi", "mytool", "POST", "/endpoint");
    assert_eq!(entry.full_name, "myapi_mytool");
    assert_eq!(entry.api_config.name, "myapi");
    assert_eq!(entry.tool_def.name, "mytool");
    assert_eq!(entry.tool_def.method, "POST");
    assert_eq!(entry.tool_def.path, "/endpoint");
}

#[test]
fn test_mock_handler_with_creds_stores_credentials() {
    let handler = mock_handler_with_creds(vec![("api1", "secret1"), ("api2", "secret2")]);
    assert_eq!(
        handler.credentials.get("api1"),
        Some(&"secret1".to_string())
    );
    assert_eq!(
        handler.credentials.get("api2"),
        Some(&"secret2".to_string())
    );
    assert_eq!(handler.credentials.get("api3"), None);
}
