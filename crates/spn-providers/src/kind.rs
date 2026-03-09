//! Backend kind enumeration.
//!
//! Identifies the type of backend (local or cloud provider).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Backend kind enumeration.
///
/// Identifies whether a backend is local (Ollama, llama.cpp) or
/// a cloud provider (Anthropic, OpenAI, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    // =========================================================================
    // Local Backends
    // =========================================================================
    /// Ollama local model server.
    Ollama,
    /// llama.cpp backend (planned).
    LlamaCpp,

    // =========================================================================
    // Cloud Backends
    // =========================================================================
    /// Anthropic (Claude models).
    Anthropic,
    /// OpenAI (GPT models).
    OpenAI,
    /// Mistral AI.
    Mistral,
    /// Groq (fast inference).
    Groq,
    /// DeepSeek.
    DeepSeek,
    /// Google Gemini.
    Gemini,

    // =========================================================================
    // Multimodal Backends (Phase B)
    // =========================================================================
    /// Candle (Stable Diffusion, Whisper).
    Candle,
    /// mistral.rs (vision models).
    MistralRs,
}

impl BackendKind {
    /// Check if this is a local backend.
    #[must_use]
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            Self::Ollama | Self::LlamaCpp | Self::Candle | Self::MistralRs
        )
    }

    /// Check if this is a cloud backend.
    #[must_use]
    pub fn is_cloud(&self) -> bool {
        !self.is_local()
    }

    /// Get the backend identifier string.
    #[must_use]
    pub fn id(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::LlamaCpp => "llama-cpp",
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Mistral => "mistral",
            Self::Groq => "groq",
            Self::DeepSeek => "deepseek",
            Self::Gemini => "gemini",
            Self::Candle => "candle",
            Self::MistralRs => "mistral-rs",
        }
    }

    /// Get the human-readable name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::LlamaCpp => "llama.cpp",
            Self::Anthropic => "Anthropic",
            Self::OpenAI => "OpenAI",
            Self::Mistral => "Mistral AI",
            Self::Groq => "Groq",
            Self::DeepSeek => "DeepSeek",
            Self::Gemini => "Google Gemini",
            Self::Candle => "Candle",
            Self::MistralRs => "mistral.rs",
        }
    }

    /// Get the environment variable for the API key (cloud backends only).
    #[must_use]
    pub fn env_var(&self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::OpenAI => Some("OPENAI_API_KEY"),
            Self::Mistral => Some("MISTRAL_API_KEY"),
            Self::Groq => Some("GROQ_API_KEY"),
            Self::DeepSeek => Some("DEEPSEEK_API_KEY"),
            Self::Gemini => Some("GEMINI_API_KEY"),
            _ => None,
        }
    }

    /// Get the default API endpoint for cloud backends.
    #[must_use]
    pub fn default_endpoint(&self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("https://api.anthropic.com"),
            Self::OpenAI => Some("https://api.openai.com"),
            Self::Mistral => Some("https://api.mistral.ai"),
            Self::Groq => Some("https://api.groq.com"),
            Self::DeepSeek => Some("https://api.deepseek.com"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com"),
            Self::Ollama => Some("http://localhost:11434"),
            _ => None,
        }
    }

    /// List all backend kinds.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Ollama,
            Self::LlamaCpp,
            Self::Anthropic,
            Self::OpenAI,
            Self::Mistral,
            Self::Groq,
            Self::DeepSeek,
            Self::Gemini,
            Self::Candle,
            Self::MistralRs,
        ]
    }

    /// List all cloud backend kinds.
    #[must_use]
    pub fn cloud_backends() -> &'static [Self] {
        &[
            Self::Anthropic,
            Self::OpenAI,
            Self::Mistral,
            Self::Groq,
            Self::DeepSeek,
            Self::Gemini,
        ]
    }

    /// List all local backend kinds.
    #[must_use]
    pub fn local_backends() -> &'static [Self] {
        &[Self::Ollama, Self::LlamaCpp, Self::Candle, Self::MistralRs]
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl FromStr for BackendKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ollama" => Ok(Self::Ollama),
            "llama-cpp" | "llamacpp" | "llama_cpp" => Ok(Self::LlamaCpp),
            "anthropic" | "claude" => Ok(Self::Anthropic),
            "openai" | "gpt" => Ok(Self::OpenAI),
            "mistral" => Ok(Self::Mistral),
            "groq" => Ok(Self::Groq),
            "deepseek" => Ok(Self::DeepSeek),
            "gemini" | "google" => Ok(Self::Gemini),
            "candle" => Ok(Self::Candle),
            "mistral-rs" | "mistralrs" => Ok(Self::MistralRs),
            _ => Err(format!("Unknown backend: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_kind_is_local() {
        assert!(BackendKind::Ollama.is_local());
        assert!(BackendKind::LlamaCpp.is_local());
        assert!(BackendKind::Candle.is_local());
        assert!(BackendKind::MistralRs.is_local());

        assert!(!BackendKind::Anthropic.is_local());
        assert!(!BackendKind::OpenAI.is_local());
    }

    #[test]
    fn test_backend_kind_is_cloud() {
        assert!(BackendKind::Anthropic.is_cloud());
        assert!(BackendKind::OpenAI.is_cloud());
        assert!(BackendKind::Mistral.is_cloud());
        assert!(BackendKind::Groq.is_cloud());
        assert!(BackendKind::DeepSeek.is_cloud());
        assert!(BackendKind::Gemini.is_cloud());

        assert!(!BackendKind::Ollama.is_cloud());
    }

    #[test]
    fn test_backend_kind_id() {
        assert_eq!(BackendKind::Ollama.id(), "ollama");
        assert_eq!(BackendKind::Anthropic.id(), "anthropic");
        assert_eq!(BackendKind::MistralRs.id(), "mistral-rs");
    }

    #[test]
    fn test_backend_kind_from_str() {
        assert_eq!(BackendKind::from_str("ollama").unwrap(), BackendKind::Ollama);
        assert_eq!(
            BackendKind::from_str("anthropic").unwrap(),
            BackendKind::Anthropic
        );
        assert_eq!(BackendKind::from_str("claude").unwrap(), BackendKind::Anthropic);
        assert_eq!(BackendKind::from_str("gpt").unwrap(), BackendKind::OpenAI);
        assert!(BackendKind::from_str("unknown").is_err());
    }

    #[test]
    fn test_backend_kind_env_var() {
        assert_eq!(
            BackendKind::Anthropic.env_var(),
            Some("ANTHROPIC_API_KEY")
        );
        assert_eq!(BackendKind::OpenAI.env_var(), Some("OPENAI_API_KEY"));
        assert_eq!(BackendKind::Ollama.env_var(), None);
    }

    #[test]
    fn test_backend_kind_all() {
        let all = BackendKind::all();
        assert!(all.contains(&BackendKind::Ollama));
        assert!(all.contains(&BackendKind::Anthropic));
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_backend_kind_display() {
        assert_eq!(BackendKind::Anthropic.to_string(), "anthropic");
        assert_eq!(BackendKind::Ollama.to_string(), "ollama");
    }
}
