//! Error types for the backends crate.

use crate::BackendKind;
use spn_core::BackendError;
use std::fmt;

/// Error type for backend operations.
#[derive(Debug)]
pub enum BackendsError {
    /// Backend not found in registry.
    BackendNotFound(BackendKind),

    /// Backend not registered.
    BackendNotRegistered(String),

    /// Invalid model alias format.
    InvalidAlias(String),

    /// Model not found for the given alias.
    ModelNotFound(String),

    /// API key missing for cloud backend.
    MissingApiKey(BackendKind),

    /// API request failed.
    ApiError {
        /// The backend that failed.
        backend: BackendKind,
        /// Error message.
        message: String,
        /// HTTP status code if available.
        status: Option<u16>,
    },

    /// Rate limit exceeded.
    RateLimited {
        /// The backend that rate limited.
        backend: BackendKind,
        /// Retry after seconds.
        retry_after: Option<u64>,
    },

    /// Backend error from underlying implementation.
    Backend(BackendError),

    /// IO error.
    Io(std::io::Error),

    /// Configuration error.
    Config(String),
}

impl fmt::Display for BackendsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendNotFound(kind) => {
                write!(f, "Backend not found: {}", kind.name())
            }
            Self::BackendNotRegistered(id) => {
                write!(f, "Backend not registered: {id}")
            }
            Self::InvalidAlias(alias) => {
                write!(f, "Invalid model alias: {alias}")
            }
            Self::ModelNotFound(alias) => {
                write!(f, "Model not found: {alias}")
            }
            Self::MissingApiKey(kind) => {
                write!(
                    f,
                    "Missing API key for {}: set {}",
                    kind.name(),
                    kind.env_var().unwrap_or("API_KEY")
                )
            }
            Self::ApiError {
                backend,
                message,
                status,
            } => {
                if let Some(code) = status {
                    write!(f, "{} API error ({}): {}", backend.name(), code, message)
                } else {
                    write!(f, "{} API error: {}", backend.name(), message)
                }
            }
            Self::RateLimited {
                backend,
                retry_after,
            } => {
                if let Some(secs) = retry_after {
                    write!(
                        f,
                        "{} rate limited, retry after {} seconds",
                        backend.name(),
                        secs
                    )
                } else {
                    write!(f, "{} rate limited", backend.name())
                }
            }
            Self::Backend(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Config(msg) => write!(f, "Configuration error: {msg}"),
        }
    }
}

impl std::error::Error for BackendsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backend(err) => Some(err),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<BackendError> for BackendsError {
    fn from(err: BackendError) -> Self {
        Self::Backend(err)
    }
}

impl From<std::io::Error> for BackendsError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Sanitize API error responses to prevent credential leaks.
///
/// Some APIs echo request headers (including API keys) in error responses.
/// This function removes potentially sensitive data patterns.
#[cfg(any(
    feature = "anthropic",
    feature = "openai",
    feature = "groq",
    feature = "mistral",
    feature = "deepseek",
    feature = "gemini",
    test
))]
#[must_use]
pub fn sanitize_api_error(raw: &str) -> String {
    // Limit length to prevent DoS from huge error bodies
    const MAX_LEN: usize = 500;
    let truncated = if raw.len() > MAX_LEN {
        format!("{}... (truncated)", &raw[..MAX_LEN])
    } else {
        raw.to_string()
    };

    // Simple redaction: look for common API key prefixes
    // We use simple string operations to avoid regex dependency
    let mut result = truncated;

    // Redact sk-ant-xxx patterns (Anthropic)
    if let Some(start) = result.find("sk-ant-") {
        if let Some(end) =
            result[start..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
        {
            result.replace_range(start..start + end, "[REDACTED_API_KEY]");
        } else {
            result.replace_range(start.., "[REDACTED_API_KEY]");
        }
    }

    // Redact sk-xxx patterns (OpenAI)
    if let Some(start) = result.find("sk-") {
        if !result[start..].starts_with("sk-ant-") {
            if let Some(end) =
                result[start..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            {
                result.replace_range(start..start + end, "[REDACTED_API_KEY]");
            } else if result.len() > start + 20 {
                // Only redact if it looks like a key (> 20 chars)
                result.replace_range(start.., "[REDACTED_API_KEY]");
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_not_found_display() {
        let err = BackendsError::BackendNotFound(BackendKind::Anthropic);
        assert!(err.to_string().contains("Anthropic"));
    }

    #[test]
    fn test_missing_api_key_display() {
        let err = BackendsError::MissingApiKey(BackendKind::OpenAI);
        let msg = err.to_string();
        assert!(msg.contains("OpenAI"));
        assert!(msg.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn test_api_error_display() {
        let err = BackendsError::ApiError {
            backend: BackendKind::Groq,
            message: "Invalid request".to_string(),
            status: Some(400),
        };
        let msg = err.to_string();
        assert!(msg.contains("Groq"));
        assert!(msg.contains("400"));
        assert!(msg.contains("Invalid request"));
    }

    #[test]
    fn test_rate_limited_display() {
        let err = BackendsError::RateLimited {
            backend: BackendKind::Anthropic,
            retry_after: Some(30),
        };
        let msg = err.to_string();
        assert!(msg.contains("Anthropic"));
        assert!(msg.contains("30 seconds"));
    }

    #[test]
    fn test_from_backend_error() {
        let backend_err = BackendError::NotRunning;
        let err: BackendsError = backend_err.into();
        assert!(matches!(err, BackendsError::Backend(_)));
    }

    #[cfg(any(
        feature = "anthropic",
        feature = "openai",
        feature = "groq",
        feature = "mistral",
        feature = "deepseek",
        feature = "gemini"
    ))]
    #[test]
    fn test_sanitize_api_error_anthropic_key() {
        let input = "Error: Invalid API key sk-ant-api03-xxxxx123456789";
        let result = sanitize_api_error(input);
        assert!(!result.contains("sk-ant-"));
        assert!(result.contains("[REDACTED_API_KEY]"));
    }

    #[cfg(any(
        feature = "anthropic",
        feature = "openai",
        feature = "groq",
        feature = "mistral",
        feature = "deepseek",
        feature = "gemini"
    ))]
    #[test]
    fn test_sanitize_api_error_truncation() {
        let long_input = "x".repeat(1000);
        let result = sanitize_api_error(&long_input);
        assert!(result.len() < 600);
        assert!(result.contains("(truncated)"));
    }

    #[cfg(any(
        feature = "anthropic",
        feature = "openai",
        feature = "groq",
        feature = "mistral",
        feature = "deepseek",
        feature = "gemini"
    ))]
    #[test]
    fn test_sanitize_api_error_safe_message() {
        let input = "Invalid request: missing required field 'model'";
        let result = sanitize_api_error(input);
        assert_eq!(result, input);
    }
}
