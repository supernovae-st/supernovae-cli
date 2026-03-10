//! Provider definitions for LLM and MCP services.
//!
//! This module is the **single source of truth** for all provider metadata
//! across the SuperNovae ecosystem.

/// Category of provider service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProviderCategory {
    /// LLM inference providers (Anthropic, OpenAI, etc.)
    Llm,
    /// MCP service providers (Neo4j, GitHub, etc.)
    Mcp,
    /// Local model runners (Ollama)
    Local,
}

/// Provider metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Provider {
    /// Unique identifier (e.g., "anthropic", "openai")
    pub id: &'static str,
    /// Human-readable name (e.g., "Anthropic Claude")
    pub name: &'static str,
    /// Environment variable name (e.g., "ANTHROPIC_API_KEY")
    pub env_var: &'static str,
    /// Expected key prefix for validation (e.g., "sk-ant-")
    pub key_prefix: Option<&'static str>,
    /// Provider category
    pub category: ProviderCategory,
    /// Whether this provider requires an API key
    pub requires_key: bool,
    /// Description of the provider
    pub description: &'static str,
}

/// All known providers in the SuperNovae ecosystem.
///
/// This constant is the **single source of truth** for provider definitions.
/// It replaces the duplicated PROVIDERS arrays in nika and spn.
pub static KNOWN_PROVIDERS: &[Provider] = &[
    // ==================== LLM Providers ====================
    Provider {
        id: "anthropic",
        name: "Anthropic Claude",
        env_var: "ANTHROPIC_API_KEY",
        key_prefix: Some("sk-ant-"),
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Claude models (Opus, Sonnet, Haiku)",
    },
    Provider {
        id: "openai",
        name: "OpenAI GPT",
        env_var: "OPENAI_API_KEY",
        key_prefix: Some("sk-"),
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "GPT-4, GPT-3.5, and other OpenAI models",
    },
    Provider {
        id: "mistral",
        name: "Mistral AI",
        env_var: "MISTRAL_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Mistral and Mixtral models",
    },
    Provider {
        id: "groq",
        name: "Groq",
        env_var: "GROQ_API_KEY",
        key_prefix: Some("gsk_"),
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Ultra-fast inference with Groq LPU",
    },
    Provider {
        id: "deepseek",
        name: "DeepSeek",
        env_var: "DEEPSEEK_API_KEY",
        key_prefix: Some("sk-"),
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "DeepSeek Coder and Chat models",
    },
    Provider {
        id: "gemini",
        name: "Google Gemini",
        env_var: "GEMINI_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Gemini Pro and Ultra models",
    },
    Provider {
        id: "ollama",
        name: "Ollama",
        env_var: "OLLAMA_API_BASE_URL",
        key_prefix: None,
        category: ProviderCategory::Local,
        requires_key: false,
        description: "Local model runner (llama, mistral, etc.) [DEPRECATED: use native]",
    },
    Provider {
        id: "native",
        name: "Native (mistral.rs)",
        env_var: "NATIVE_MODEL_PATH",
        key_prefix: None,
        category: ProviderCategory::Local,
        requires_key: false,
        description: "Local inference via mistral.rs (in-process)",
    },
    // ==================== Cloud LLM Providers (rig-core unsupported) ====================
    Provider {
        id: "cohere",
        name: "Cohere",
        env_var: "COHERE_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Cohere Command and Embed models",
    },
    Provider {
        id: "together",
        name: "Together AI",
        env_var: "TOGETHER_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Together AI inference platform",
    },
    Provider {
        id: "fireworks",
        name: "Fireworks AI",
        env_var: "FIREWORKS_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Fireworks AI fast inference",
    },
    Provider {
        id: "cerebras",
        name: "Cerebras",
        env_var: "CEREBRAS_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Llm,
        requires_key: true,
        description: "Cerebras ultra-fast inference",
    },
    // NOTE: Replicate and Anyscale NOT included:
    // - Replicate: NOT a chat API (async prediction polling)
    // - Anyscale: SERVICE SUNSET June 2024
    // ==================== MCP Service Providers ====================
    Provider {
        id: "neo4j",
        name: "Neo4j Graph Database",
        env_var: "NEO4J_PASSWORD",
        key_prefix: None,
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "Graph database for knowledge storage",
    },
    Provider {
        id: "github",
        name: "GitHub API",
        env_var: "GITHUB_TOKEN",
        key_prefix: Some("ghp_"),
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "GitHub API access",
    },
    Provider {
        id: "slack",
        name: "Slack API",
        env_var: "SLACK_BOT_TOKEN",
        key_prefix: Some("xoxb-"),
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "Slack workspace integration",
    },
    Provider {
        id: "perplexity",
        name: "Perplexity AI",
        env_var: "PERPLEXITY_API_KEY",
        key_prefix: Some("pplx-"),
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "AI-powered web search",
    },
    Provider {
        id: "firecrawl",
        name: "Firecrawl",
        env_var: "FIRECRAWL_API_KEY",
        key_prefix: Some("fc-"),
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "Web scraping and crawling",
    },
    Provider {
        id: "supadata",
        name: "Supadata API",
        env_var: "SUPADATA_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "Video transcription and web scraping",
    },
    Provider {
        id: "dataforseo",
        name: "DataForSEO",
        env_var: "DATAFORSEO_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "SEO data, keyword research, SERP analysis",
    },
    Provider {
        id: "ahrefs",
        name: "Ahrefs API",
        env_var: "AHREFS_API_KEY",
        key_prefix: None,
        category: ProviderCategory::Mcp,
        requires_key: true,
        description: "Backlink analysis and SEO metrics",
    },
];

/// Find a provider by ID (case-insensitive).
///
/// # Example
///
/// ```
/// use spn_core::find_provider;
///
/// let provider = find_provider("anthropic").unwrap();
/// assert_eq!(provider.env_var, "ANTHROPIC_API_KEY");
///
/// let provider = find_provider("OPENAI").unwrap();
/// assert_eq!(provider.id, "openai");
/// ```
#[must_use]
pub fn find_provider(id: &str) -> Option<&'static Provider> {
    KNOWN_PROVIDERS
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(id))
}

/// Get the environment variable name for a provider.
///
/// # Example
///
/// ```
/// use spn_core::provider_to_env_var;
///
/// assert_eq!(provider_to_env_var("anthropic"), Some("ANTHROPIC_API_KEY"));
/// assert_eq!(provider_to_env_var("unknown"), None);
/// ```
pub fn provider_to_env_var(id: &str) -> Option<&'static str> {
    find_provider(id).map(|p| p.env_var)
}

/// Get all providers in a specific category.
///
/// # Example
///
/// ```
/// use spn_core::{providers_by_category, ProviderCategory};
///
/// let llm_providers: Vec<_> = providers_by_category(ProviderCategory::Llm).collect();
/// assert!(llm_providers.iter().any(|p| p.id == "anthropic"));
/// ```
pub fn providers_by_category(
    category: ProviderCategory,
) -> impl Iterator<Item = &'static Provider> {
    KNOWN_PROVIDERS
        .iter()
        .filter(move |p| p.category == category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_provider() {
        assert!(find_provider("anthropic").is_some());
        assert!(find_provider("ANTHROPIC").is_some());
        assert!(find_provider("unknown").is_none());
    }

    #[test]
    fn test_provider_to_env_var() {
        assert_eq!(provider_to_env_var("anthropic"), Some("ANTHROPIC_API_KEY"));
        assert_eq!(provider_to_env_var("github"), Some("GITHUB_TOKEN"));
        assert_eq!(provider_to_env_var("unknown"), None);
    }

    #[test]
    fn test_providers_by_category() {
        let llm: Vec<_> = providers_by_category(ProviderCategory::Llm).collect();
        // 10 LLM providers: anthropic, openai, mistral, groq, deepseek, gemini,
        //                   cohere, together, fireworks, cerebras
        assert!(
            llm.len() >= 10,
            "Expected at least 10 LLM providers, got {}",
            llm.len()
        );
        assert!(llm.iter().all(|p| p.category == ProviderCategory::Llm));

        let mcp: Vec<_> = providers_by_category(ProviderCategory::Mcp).collect();
        assert!(mcp.len() >= 5);
        assert!(mcp.iter().all(|p| p.category == ProviderCategory::Mcp));

        let local: Vec<_> = providers_by_category(ProviderCategory::Local).collect();
        // 2 Local providers: ollama, native
        assert!(
            local.len() >= 2,
            "Expected at least 2 Local providers, got {}",
            local.len()
        );
        assert!(local.iter().all(|p| p.category == ProviderCategory::Local));
    }

    #[test]
    fn test_all_providers_have_env_var() {
        for provider in KNOWN_PROVIDERS {
            assert!(
                !provider.env_var.is_empty(),
                "Provider {} missing env_var",
                provider.id
            );
        }
    }

    #[test]
    fn test_provider_count() {
        // Ensure we have at least 20 providers (12 LLM + 8 MCP)
        // LLM: anthropic, openai, mistral, groq, deepseek, gemini, ollama, native,
        //      cohere, together, fireworks, cerebras
        // MCP: neo4j, github, slack, perplexity, firecrawl, supadata, dataforseo, ahrefs
        assert!(
            KNOWN_PROVIDERS.len() >= 20,
            "Expected at least 20 providers, got {}",
            KNOWN_PROVIDERS.len()
        );
    }

    #[test]
    fn test_new_cloud_providers() {
        // Verify the new cloud providers exist
        assert!(find_provider("cohere").is_some());
        assert!(find_provider("together").is_some());
        assert!(find_provider("fireworks").is_some());
        assert!(find_provider("cerebras").is_some());
        assert!(find_provider("native").is_some());
    }
}
