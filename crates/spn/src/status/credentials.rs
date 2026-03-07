//! Unified credentials collector.
//!
//! Collects status of all credentials (LLM providers + MCP services)
//! and reports their source (keychain, env, .env, local).

use serde::Serialize;
use spn_client::KNOWN_PROVIDERS;

/// Credential status.
#[derive(Debug, Clone, Serialize)]
pub struct CredentialStatus {
    /// Provider/service name.
    pub name: String,
    /// Type: LLM or MCP.
    pub credential_type: CredentialType,
    /// Current status.
    pub status: Status,
    /// Where the credential is stored.
    pub source: Option<Source>,
    /// Associated endpoint.
    pub endpoint: Option<String>,
}

/// Type of credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialType {
    /// LLM provider (for inference).
    Llm,
    /// MCP service (for MCP servers).
    Mcp,
}

/// Credential status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Credential is configured and ready.
    Ready,
    /// Local provider (no key needed).
    Local,
    /// Not configured.
    NotSet,
}

/// Source of the credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// OS Keychain.
    Keychain,
    /// Environment variable.
    Env,
    /// .env file.
    DotEnv,
    /// Local (no key needed).
    Local,
}

impl Source {
    /// Icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            Source::Keychain => "🔐",
            Source::Env => "📦",
            Source::DotEnv => "📄",
            Source::Local => "🦙",
        }
    }

    /// Short label.
    pub fn label(&self) -> &'static str {
        match self {
            Source::Keychain => "keychain",
            Source::Env => "env",
            Source::DotEnv => ".env",
            Source::Local => "local",
        }
    }
}

/// Known endpoints for providers.
fn get_endpoint(name: &str) -> Option<&'static str> {
    match name {
        "anthropic" => Some("api.anthropic.com"),
        "openai" => Some("api.openai.com"),
        "mistral" => Some("api.mistral.ai"),
        "groq" => Some("api.groq.com"),
        "deepseek" => Some("api.deepseek.com"),
        "gemini" => Some("generativelanguage.googleapis.com"),
        "ollama" => Some("localhost:11434"),
        "neo4j" => Some("bolt://localhost:7687"),
        "github" => Some("api.github.com"),
        "firecrawl" => Some("api.firecrawl.dev"),
        "perplexity" => Some("api.perplexity.ai"),
        "slack" => Some("api.slack.com"),
        "supadata" => Some("api.supadata.dev"),
        _ => None,
    }
}

/// Collect all credential statuses.
pub async fn collect() -> Vec<CredentialStatus> {
    let mut credentials = Vec::new();

    // Load .env file once
    let _ = dotenvy::dotenv();

    for provider in KNOWN_PROVIDERS {
        let credential_type = match provider.category {
            spn_client::ProviderCategory::Llm => CredentialType::Llm,
            spn_client::ProviderCategory::Mcp => CredentialType::Mcp,
            spn_client::ProviderCategory::Local => {
                // Local providers don't need credentials
                credentials.push(CredentialStatus {
                    name: provider.id.to_string(),
                    credential_type: CredentialType::Llm,
                    status: Status::Local,
                    source: Some(Source::Local),
                    endpoint: get_endpoint(provider.id).map(String::from),
                });
                continue;
            }
        };

        // Check keychain first
        #[cfg(feature = "os-keychain")]
        let keychain_result = {
            use spn_keyring::SpnKeyring;
            SpnKeyring::get(provider.id).ok()
        };

        #[cfg(not(feature = "os-keychain"))]
        let keychain_result: Option<String> = None;

        if keychain_result.is_some() {
            credentials.push(CredentialStatus {
                name: provider.id.to_string(),
                credential_type,
                status: Status::Ready,
                source: Some(Source::Keychain),
                endpoint: get_endpoint(provider.id).map(String::from),
            });
            continue;
        }

        // Check environment variable
        if std::env::var(provider.env_var).is_ok() {
            // Try to determine if it came from .env or actual env
            // We'll assume if dotenvy loaded it, it's from .env
            // This is a simplification - in practice we'd need to track this
            credentials.push(CredentialStatus {
                name: provider.id.to_string(),
                credential_type,
                status: Status::Ready,
                source: Some(Source::Env),
                endpoint: get_endpoint(provider.id).map(String::from),
            });
            continue;
        }

        // Not configured
        credentials.push(CredentialStatus {
            name: provider.id.to_string(),
            credential_type,
            status: Status::NotSet,
            source: None,
            endpoint: None,
        });
    }

    credentials
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_collect_returns_all_providers() {
        let credentials = collect().await;
        assert!(!credentials.is_empty());
        // Should have both LLM and MCP types
        assert!(credentials.iter().any(|c| c.credential_type == CredentialType::Llm));
        assert!(credentials.iter().any(|c| c.credential_type == CredentialType::Mcp));
    }

    #[test]
    fn test_source_icons() {
        assert_eq!(Source::Keychain.icon(), "🔐");
        assert_eq!(Source::Env.icon(), "📦");
        assert_eq!(Source::DotEnv.icon(), "📄");
        assert_eq!(Source::Local.icon(), "🦙");
    }
}
