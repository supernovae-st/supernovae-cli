//! # spn-client
//!
//! Client library for communicating with the spn daemon.
//!
//! This crate provides a simple interface for applications (like Nika) to securely
//! retrieve secrets from the spn daemon without directly accessing the OS keychain.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use spn_client::{SpnClient, ExposeSecret};
//!
//! # async fn example() -> Result<(), spn_client::Error> {
//! // Connect to the daemon
//! let mut client = SpnClient::connect().await?;
//!
//! // Get a secret
//! let api_key = client.get_secret("anthropic").await?;
//! println!("Got key: {}", api_key.expose_secret());
//!
//! // Check if a secret exists
//! if client.has_secret("openai").await? {
//!     println!("OpenAI key available");
//! }
//!
//! // List all providers
//! let providers = client.list_providers().await?;
//! println!("Available providers: {:?}", providers);
//! # Ok(())
//! # }
//! ```
//!
//! ## Fallback Mode
//!
//! If the daemon is not running, the client can fall back to reading from
//! environment variables:
//!
//! ```rust,no_run
//! use spn_client::SpnClient;
//!
//! # async fn example() -> Result<(), spn_client::Error> {
//! let mut client = SpnClient::connect_with_fallback().await?;
//! // Works even if daemon is not running
//! # Ok(())
//! # }
//! ```

mod error;
mod protocol;

pub use error::Error;
pub use protocol::{Request, Response};
pub use secrecy::{ExposeSecret, SecretString};

use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::{debug, warn};

/// Default socket path for the spn daemon.
pub fn default_socket_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".spn").join("daemon.sock"))
        .unwrap_or_else(|| PathBuf::from("/tmp/spn-daemon.sock"))
}

/// Check if the daemon socket exists.
pub fn daemon_socket_exists() -> bool {
    default_socket_path().exists()
}

/// Client for communicating with the spn daemon.
///
/// The client uses Unix socket IPC to communicate with the daemon,
/// which handles all keychain access to avoid repeated auth prompts.
#[derive(Debug)]
pub struct SpnClient {
    stream: Option<UnixStream>,
    fallback_mode: bool,
}

impl SpnClient {
    /// Connect to the spn daemon.
    ///
    /// Returns an error if the daemon is not running.
    pub async fn connect() -> Result<Self, Error> {
        Self::connect_to(&default_socket_path()).await
    }

    /// Connect to the daemon at a specific socket path.
    pub async fn connect_to(socket_path: &PathBuf) -> Result<Self, Error> {
        debug!("Connecting to spn daemon at {:?}", socket_path);

        let stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| Error::ConnectionFailed {
                path: socket_path.clone(),
                source: e,
            })?;

        // Verify connection with ping
        let mut client = Self {
            stream: Some(stream),
            fallback_mode: false,
        };

        client.ping().await?;
        debug!("Connected to spn daemon");

        Ok(client)
    }

    /// Connect to the daemon, falling back to env vars if daemon is unavailable.
    ///
    /// This is the recommended way to connect in applications that should
    /// work even without the daemon running.
    pub async fn connect_with_fallback() -> Result<Self, Error> {
        match Self::connect().await {
            Ok(client) => Ok(client),
            Err(e) => {
                warn!("spn daemon not running, using env var fallback: {}", e);
                Ok(Self {
                    stream: None,
                    fallback_mode: true,
                })
            }
        }
    }

    /// Check if the client is in fallback mode (daemon not connected).
    pub fn is_fallback_mode(&self) -> bool {
        self.fallback_mode
    }

    /// Ping the daemon to verify the connection.
    pub async fn ping(&mut self) -> Result<String, Error> {
        let response = self.send_request(Request::Ping).await?;
        match response {
            Response::Pong { version } => Ok(version),
            Response::Error { message } => Err(Error::DaemonError(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Get a secret for the given provider.
    ///
    /// In fallback mode, attempts to read from the environment variable
    /// associated with the provider (e.g., ANTHROPIC_API_KEY).
    pub async fn get_secret(&mut self, provider: &str) -> Result<SecretString, Error> {
        if self.fallback_mode {
            return self.get_secret_from_env(provider);
        }

        let response = self
            .send_request(Request::GetSecret {
                provider: provider.to_string(),
            })
            .await?;

        match response {
            Response::Secret { value } => Ok(SecretString::from(value)),
            Response::Error { message } => Err(Error::SecretNotFound {
                provider: provider.to_string(),
                details: message,
            }),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Check if a secret exists for the given provider.
    pub async fn has_secret(&mut self, provider: &str) -> Result<bool, Error> {
        if self.fallback_mode {
            return Ok(self.get_secret_from_env(provider).is_ok());
        }

        let response = self
            .send_request(Request::HasSecret {
                provider: provider.to_string(),
            })
            .await?;

        match response {
            Response::Exists { exists } => Ok(exists),
            Response::Error { message } => Err(Error::DaemonError(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// List all available providers.
    pub async fn list_providers(&mut self) -> Result<Vec<String>, Error> {
        if self.fallback_mode {
            return Ok(self.list_env_providers());
        }

        let response = self.send_request(Request::ListProviders).await?;

        match response {
            Response::Providers { providers } => Ok(providers),
            Response::Error { message } => Err(Error::DaemonError(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Send a request to the daemon and receive a response.
    async fn send_request(&mut self, request: Request) -> Result<Response, Error> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(Error::NotConnected)?;

        // Serialize request
        let request_json = serde_json::to_vec(&request).map_err(Error::SerializationError)?;

        // Send length-prefixed message
        let len = request_json.len() as u32;
        stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(Error::IoError)?;
        stream
            .write_all(&request_json)
            .await
            .map_err(Error::IoError)?;

        // Read response length
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(Error::IoError)?;
        let response_len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check response length (max 1MB)
        if response_len > 1_048_576 {
            return Err(Error::ResponseTooLarge(response_len));
        }

        // Read response
        let mut response_buf = vec![0u8; response_len];
        stream
            .read_exact(&mut response_buf)
            .await
            .map_err(Error::IoError)?;

        // Deserialize
        let response: Response =
            serde_json::from_slice(&response_buf).map_err(Error::DeserializationError)?;

        Ok(response)
    }

    // Fallback helpers

    fn get_secret_from_env(&self, provider: &str) -> Result<SecretString, Error> {
        let env_var = provider_to_env_var(provider);
        std::env::var(&env_var)
            .map(SecretString::from)
            .map_err(|_| Error::SecretNotFound {
                provider: provider.to_string(),
                details: format!("Environment variable {} not set", env_var),
            })
    }

    fn list_env_providers(&self) -> Vec<String> {
        KNOWN_PROVIDERS
            .iter()
            .filter(|p| std::env::var(provider_to_env_var(p)).is_ok())
            .map(|s| s.to_string())
            .collect()
    }
}

/// Known provider names.
const KNOWN_PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    "mistral",
    "groq",
    "deepseek",
    "gemini",
    "ollama",
    "neo4j",
    "github",
    "slack",
    "perplexity",
    "firecrawl",
    "supadata",
];

/// Convert a provider name to its environment variable.
fn provider_to_env_var(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "anthropic" => "ANTHROPIC_API_KEY".to_string(),
        "openai" => "OPENAI_API_KEY".to_string(),
        "mistral" => "MISTRAL_API_KEY".to_string(),
        "groq" => "GROQ_API_KEY".to_string(),
        "deepseek" => "DEEPSEEK_API_KEY".to_string(),
        "gemini" => "GEMINI_API_KEY".to_string(),
        "ollama" => "OLLAMA_HOST".to_string(),
        "neo4j" => "NEO4J_PASSWORD".to_string(),
        "github" => "GITHUB_TOKEN".to_string(),
        "slack" => "SLACK_TOKEN".to_string(),
        "perplexity" => "PERPLEXITY_API_KEY".to_string(),
        "firecrawl" => "FIRECRAWL_API_KEY".to_string(),
        "supadata" => "SUPADATA_API_KEY".to_string(),
        other => format!("{}_API_KEY", other.to_uppercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_to_env_var() {
        assert_eq!(provider_to_env_var("anthropic"), "ANTHROPIC_API_KEY");
        assert_eq!(provider_to_env_var("openai"), "OPENAI_API_KEY");
        assert_eq!(provider_to_env_var("neo4j"), "NEO4J_PASSWORD");
        assert_eq!(provider_to_env_var("github"), "GITHUB_TOKEN");
        assert_eq!(provider_to_env_var("unknown"), "UNKNOWN_API_KEY");
    }

    #[test]
    fn test_default_socket_path() {
        let path = default_socket_path();
        assert!(path.to_string_lossy().contains(".spn"));
        assert!(path.to_string_lossy().contains("daemon.sock"));
    }

    #[test]
    fn test_daemon_socket_exists() {
        // Should return false since daemon isn't running in tests
        assert!(!daemon_socket_exists());
    }
}
