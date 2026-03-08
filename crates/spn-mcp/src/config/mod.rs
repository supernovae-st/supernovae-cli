//! Configuration module for spn-mcp.
//!
//! Handles loading and validation of API wrapper configurations from YAML files.

mod loader;
mod schema;

pub use loader::{apis_dir, load_all_apis, load_api};
pub use schema::{ApiConfig, ApiKeyLocation, AuthConfig, AuthType, RateLimitConfig, ToolDef};
// These are only used in tests across the crate
#[allow(unused_imports)]
pub use schema::{ParamDef, ParamType};

use crate::error::{Error, Result};

/// Validate an API configuration.
pub fn validate(config: &ApiConfig) -> Result<()> {
    // Check name is valid identifier
    if config.name.is_empty() {
        return Err(Error::ConfigValidation("name is required".into()));
    }

    if !config
        .name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::ConfigValidation(
            "name must be alphanumeric with underscores or hyphens".into(),
        ));
    }

    // Check base_url is valid
    if config.base_url.is_empty() {
        return Err(Error::ConfigValidation("base_url is required".into()));
    }

    if !config.base_url.starts_with("http://") && !config.base_url.starts_with("https://") {
        return Err(Error::ConfigValidation(
            "base_url must start with http:// or https://".into(),
        ));
    }

    // Check auth credential is specified
    if config.auth.credential.is_empty() {
        return Err(Error::ConfigValidation(
            "auth.credential is required".into(),
        ));
    }

    // Check tools
    if config.tools.is_empty() {
        return Err(Error::ConfigValidation(
            "at least one tool is required".into(),
        ));
    }

    for tool in &config.tools {
        validate_tool(tool)?;
    }

    Ok(())
}

fn validate_tool(tool: &ToolDef) -> Result<()> {
    if tool.name.is_empty() {
        return Err(Error::ConfigValidation("tool.name is required".into()));
    }

    if !tool.name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(Error::ConfigValidation(format!(
            "tool.name '{}' must be alphanumeric with underscores",
            tool.name
        )));
    }

    if tool.path.is_empty() {
        return Err(Error::ConfigValidation(format!(
            "tool '{}': path is required",
            tool.name
        )));
    }

    if !tool.path.starts_with('/') {
        return Err(Error::ConfigValidation(format!(
            "tool '{}': path must start with /",
            tool.name
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let config = ApiConfig {
            name: "test_api".into(),
            version: "1.0".into(),
            base_url: "https://api.example.com".into(),
            description: Some("Test API".into()),
            auth: AuthConfig {
                auth_type: AuthType::Bearer,
                credential: "test".into(),
                location: None,
                key_name: None,
            },
            rate_limit: None,
            headers: None,
            tools: vec![ToolDef {
                name: "get_data".into(),
                description: Some("Get data".into()),
                method: "GET".into(),
                path: "/data".into(),
                body_template: None,
                params: vec![],
                response: None,
            }],
        };

        assert!(validate(&config).is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let config = ApiConfig {
            name: "".into(),
            version: "1.0".into(),
            base_url: "https://api.example.com".into(),
            description: None,
            auth: AuthConfig {
                auth_type: AuthType::Bearer,
                credential: "test".into(),
                location: None,
                key_name: None,
            },
            rate_limit: None,
            headers: None,
            tools: vec![],
        };

        let err = validate(&config).unwrap_err();
        assert!(err.to_string().contains("name is required"));
    }

    #[test]
    fn test_validate_invalid_base_url() {
        let config = ApiConfig {
            name: "test".into(),
            version: "1.0".into(),
            base_url: "not-a-url".into(),
            description: None,
            auth: AuthConfig {
                auth_type: AuthType::Bearer,
                credential: "test".into(),
                location: None,
                key_name: None,
            },
            rate_limit: None,
            headers: None,
            tools: vec![],
        };

        let err = validate(&config).unwrap_err();
        assert!(err.to_string().contains("http"));
    }
}
