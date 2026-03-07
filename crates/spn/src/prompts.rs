//! Interactive prompts for v0.14.0 "The Delight Release".
//!
//! Provides guided selection when required arguments are missing,
//! turning errors into helpful interactions.
//!
//! Note: These prompts will be integrated into commands in v0.14.1+
//! when we update provider/mcp/model commands to use interactive fallback.

#![allow(dead_code)] // Functions prepared for upcoming command integration

use crate::ux;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Password};

/// Result type for prompt operations
pub type PromptResult<T> = std::result::Result<T, dialoguer::Error>;

// ============================================================================
// PROVIDER PROMPTS
// ============================================================================

/// Provider option with description
struct ProviderOption {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    key_prefix: &'static str,
}

const LLM_PROVIDERS: &[ProviderOption] = &[
    ProviderOption {
        id: "anthropic",
        name: "Anthropic",
        description: "Claude API",
        key_prefix: "sk-ant-",
    },
    ProviderOption {
        id: "openai",
        name: "OpenAI",
        description: "GPT-4, etc.",
        key_prefix: "sk-",
    },
    ProviderOption {
        id: "mistral",
        name: "Mistral",
        description: "Mistral AI",
        key_prefix: "",
    },
    ProviderOption {
        id: "groq",
        name: "Groq",
        description: "Fast inference",
        key_prefix: "gsk_",
    },
    ProviderOption {
        id: "deepseek",
        name: "DeepSeek",
        description: "DeepSeek AI",
        key_prefix: "sk-",
    },
    ProviderOption {
        id: "gemini",
        name: "Gemini",
        description: "Google Gemini",
        key_prefix: "",
    },
];

const MCP_PROVIDERS: &[ProviderOption] = &[
    ProviderOption {
        id: "neo4j",
        name: "Neo4j",
        description: "Graph database",
        key_prefix: "",
    },
    ProviderOption {
        id: "github",
        name: "GitHub",
        description: "GitHub API",
        key_prefix: "ghp_",
    },
    ProviderOption {
        id: "perplexity",
        name: "Perplexity",
        description: "AI search",
        key_prefix: "pplx-",
    },
    ProviderOption {
        id: "firecrawl",
        name: "Firecrawl",
        description: "Web scraping",
        key_prefix: "fc-",
    },
    ProviderOption {
        id: "supadata",
        name: "Supadata",
        description: "Transcripts & crawling",
        key_prefix: "sd_",
    },
];

/// Prompt user to select a provider when not specified
pub fn select_provider() -> PromptResult<String> {
    println!();
    println!("  {}", console::style("LLM Providers").bold());

    let mut items: Vec<String> = LLM_PROVIDERS
        .iter()
        .map(|p| format!("{:<12} {}", p.id, console::style(p.description).dim()))
        .collect();

    items.push(String::new()); // Separator
    items.push(console::style("MCP Secrets").bold().to_string());

    for p in MCP_PROVIDERS {
        items.push(format!(
            "{:<12} {}",
            p.id,
            console::style(p.description).dim()
        ));
    }

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Which provider would you like to configure?")
        .items(&items)
        .default(0)
        .interact()?;

    // Map selection back to provider id
    let provider_id = if selection < LLM_PROVIDERS.len() {
        LLM_PROVIDERS[selection].id
    } else {
        let mcp_index = selection - LLM_PROVIDERS.len() - 2; // -2 for separator + header
        if mcp_index < MCP_PROVIDERS.len() {
            MCP_PROVIDERS[mcp_index].id
        } else {
            LLM_PROVIDERS[0].id // Fallback
        }
    };

    Ok(provider_id.to_string())
}

/// Prompt for API key input (secure, hidden)
pub fn prompt_api_key(provider: &str) -> PromptResult<String> {
    let hint = LLM_PROVIDERS
        .iter()
        .chain(MCP_PROVIDERS.iter())
        .find(|p| p.id == provider)
        .map(|p| {
            if p.key_prefix.is_empty() {
                String::new()
            } else {
                format!(" (starts with {}...)", p.key_prefix)
            }
        })
        .unwrap_or_default();

    Password::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter {} API key{}", provider, hint))
        .interact()
}

// ============================================================================
// MCP SERVER PROMPTS
// ============================================================================

/// MCP server option with description
struct McpOption {
    id: &'static str,
    description: &'static str,
    recommended: bool,
}

const MCP_SERVERS: &[McpOption] = &[
    McpOption {
        id: "neo4j",
        description: "Graph database (recommended for NovaNet)",
        recommended: true,
    },
    McpOption {
        id: "github",
        description: "GitHub API access",
        recommended: false,
    },
    McpOption {
        id: "perplexity",
        description: "AI-powered search",
        recommended: false,
    },
    McpOption {
        id: "firecrawl",
        description: "Web scraping",
        recommended: false,
    },
    McpOption {
        id: "supadata",
        description: "Transcripts & crawling",
        recommended: false,
    },
    McpOption {
        id: "sequential-thinking",
        description: "Step-by-step reasoning",
        recommended: false,
    },
    McpOption {
        id: "novanet",
        description: "NovaNet knowledge graph (11 tools)",
        recommended: true,
    },
];

/// Prompt user to select an MCP server when not specified
pub fn select_mcp_server() -> PromptResult<String> {
    let items: Vec<String> = MCP_SERVERS
        .iter()
        .map(|s| {
            let rec = if s.recommended { " *" } else { "" };
            format!(
                "{:<20} {}{}",
                s.id,
                console::style(s.description).dim(),
                console::style(rec).cyan()
            )
        })
        .collect();

    println!();
    println!(
        "  {} recommended",
        console::style("*").cyan()
    );

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Which MCP server would you like to add?")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(MCP_SERVERS[selection].id.to_string())
}

// ============================================================================
// MODEL PROMPTS
// ============================================================================

/// Model option with size info
struct ModelOption {
    name: &'static str,
    description: &'static str,
    size: &'static str,
}

const POPULAR_MODELS: &[ModelOption] = &[
    ModelOption {
        name: "llama3.2:1b",
        description: "Fast, efficient",
        size: "1.2 GB",
    },
    ModelOption {
        name: "llama3.2:3b",
        description: "Balanced",
        size: "2.0 GB",
    },
    ModelOption {
        name: "llama3.2:7b",
        description: "High quality",
        size: "4.0 GB",
    },
    ModelOption {
        name: "mistral:7b",
        description: "Instruction-tuned",
        size: "4.1 GB",
    },
    ModelOption {
        name: "codellama:7b",
        description: "Coding specialist",
        size: "3.8 GB",
    },
    ModelOption {
        name: "phi3:mini",
        description: "Lightweight",
        size: "2.2 GB",
    },
];

/// Prompt user to select a model when not specified
pub fn select_model() -> PromptResult<String> {
    let items: Vec<String> = POPULAR_MODELS
        .iter()
        .map(|m| {
            format!(
                "{:<18} {:>8}  {}",
                m.name,
                console::style(m.size).dim(),
                console::style(m.description).dim()
            )
        })
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Which model would you like to pull?")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(POPULAR_MODELS[selection].name.to_string())
}

// ============================================================================
// CONFIRMATION PROMPTS
// ============================================================================

/// Confirm a destructive operation
pub fn confirm_delete(name: &str, details: Option<&str>) -> PromptResult<bool> {
    let prompt = match details {
        Some(d) => format!("Delete {} ({})? This cannot be undone", name, d),
        None => format!("Delete {}? This cannot be undone", name),
    };

    ux::confirm(&prompt, false)
}

/// Confirm overwriting existing data
pub fn confirm_overwrite(name: &str) -> PromptResult<bool> {
    ux::confirm(&format!("{} already exists. Overwrite?", name), false)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_list_not_empty() {
        assert!(!LLM_PROVIDERS.is_empty());
        assert!(!MCP_PROVIDERS.is_empty());
    }

    #[test]
    fn test_mcp_servers_not_empty() {
        assert!(!MCP_SERVERS.is_empty());
    }

    #[test]
    fn test_models_not_empty() {
        assert!(!POPULAR_MODELS.is_empty());
    }
}
