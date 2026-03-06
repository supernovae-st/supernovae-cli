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
mod paths;
mod protocol;

pub use error::Error;
pub use paths::{PathError, SpnPaths};
pub use protocol::{Request, Response};
pub use secrecy::{ExposeSecret, SecretString};

// Re-export all spn-core types for convenience
pub use spn_core::{
    find_provider,
    mask_key,
    provider_to_env_var,
    providers_by_category,
    validate_key_format,
    BackendError,
    GpuInfo,
    LoadConfig,
    McpConfig,
    // MCP
    McpServer,
    McpServerType,
    McpSource,
    ModelInfo,
    PackageManifest,
    // Registry
    PackageRef,
    PackageType,
    // Providers
    Provider,
    ProviderCategory,
    // Backend
    PullProgress,
    RunningModel,
    Source,
    // Validation
    ValidationResult,
    KNOWN_PROVIDERS,
};

use std::path::PathBuf;
use std::time::Duration;
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixStream;
use tracing::debug;
#[cfg(unix)]
use tracing::warn;

/// Default timeout for IPC operations (30 seconds).
pub const DEFAULT_IPC_TIMEOUT: Duration = Duration::from_secs(30);

/// Get socket path for the spn daemon, returning an error if HOME is unavailable.
///
/// Use this function when you need to ensure a secure socket path.
/// Returns an error instead of falling back to `/tmp`.
///
/// This is a convenience wrapper around `SpnPaths::new()?.socket_file()`.
pub fn socket_path() -> Result<PathBuf, Error> {
    SpnPaths::new().map(|p| p.socket_file()).map_err(|_| {
        Error::Configuration("HOME directory not found. Set HOME environment variable.".into())
    })
}

/// Check if the daemon socket exists.
///
/// Returns `false` if HOME directory is unavailable.
pub fn daemon_socket_exists() -> bool {
    socket_path().map(|p| p.exists()).unwrap_or(false)
}

/// Client for communicating with the spn daemon.
///
/// The client uses Unix socket IPC to communicate with the daemon,
/// which handles all keychain access to avoid repeated auth prompts.
///
/// On non-Unix platforms (Windows), the client always operates in fallback mode,
/// reading secrets from environment variables.
#[derive(Debug)]
pub struct SpnClient {
    #[cfg(unix)]
    stream: Option<UnixStream>,
    fallback_mode: bool,
    /// Timeout for IPC operations.
    timeout: Duration,
}

impl SpnClient {
    /// Connect to the spn daemon.
    ///
    /// Returns an error if the daemon is not running.
    ///
    /// This method is only available on Unix platforms.
    #[cfg(unix)]
    pub async fn connect() -> Result<Self, Error> {
        let path = socket_path()?;
        Self::connect_to(&path).await
    }

    /// Connect to the daemon at a specific socket path.
    ///
    /// This method is only available on Unix platforms.
    #[cfg(unix)]
    pub async fn connect_to(socket_path: &PathBuf) -> Result<Self, Error> {
        debug!("Connecting to spn daemon at {:?}", socket_path);

        let stream =
            UnixStream::connect(socket_path)
                .await
                .map_err(|e| Error::ConnectionFailed {
                    path: socket_path.clone(),
                    source: e,
                })?;

        // Verify connection with ping
        let mut client = Self {
            stream: Some(stream),
            fallback_mode: false,
            timeout: DEFAULT_IPC_TIMEOUT,
        };

        client.ping().await?;
        debug!("Connected to spn daemon");

        Ok(client)
    }

    /// Set the timeout for IPC operations.
    ///
    /// The default timeout is 30 seconds.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Get the current timeout for IPC operations.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Connect to the daemon, falling back to env vars if daemon is unavailable.
    ///
    /// This is the recommended way to connect in applications that should
    /// work even without the daemon running.
    ///
    /// On non-Unix platforms (Windows), this always returns a fallback client.
    #[cfg(unix)]
    pub async fn connect_with_fallback() -> Result<Self, Error> {
        match Self::connect().await {
            Ok(client) => Ok(client),
            Err(e) => {
                warn!("spn daemon not running, using env var fallback: {}", e);
                Ok(Self {
                    stream: None,
                    fallback_mode: true,
                    timeout: DEFAULT_IPC_TIMEOUT,
                })
            }
        }
    }

    /// Connect to the daemon, falling back to env vars if daemon is unavailable.
    ///
    /// On non-Unix platforms (Windows), this always returns a fallback client
    /// since Unix sockets are not available.
    #[cfg(not(unix))]
    pub async fn connect_with_fallback() -> Result<Self, Error> {
        debug!("Non-Unix platform: using env var fallback mode");
        Ok(Self {
            fallback_mode: true,
            timeout: DEFAULT_IPC_TIMEOUT,
        })
    }

    /// Check if the client is in fallback mode (daemon not connected).
    pub fn is_fallback_mode(&self) -> bool {
        self.fallback_mode
    }

    /// Ping the daemon to verify the connection.
    ///
    /// This method is only available on Unix platforms.
    #[cfg(unix)]
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
    #[cfg(unix)]
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

    /// Get a secret for the given provider.
    ///
    /// On non-Unix platforms, always reads from environment variables.
    #[cfg(not(unix))]
    pub async fn get_secret(&mut self, provider: &str) -> Result<SecretString, Error> {
        self.get_secret_from_env(provider)
    }

    /// Check if a secret exists for the given provider.
    #[cfg(unix)]
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

    /// Check if a secret exists for the given provider.
    ///
    /// On non-Unix platforms, checks environment variables.
    #[cfg(not(unix))]
    pub async fn has_secret(&mut self, provider: &str) -> Result<bool, Error> {
        Ok(self.get_secret_from_env(provider).is_ok())
    }

    /// List all available providers.
    #[cfg(unix)]
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

    /// List all available providers.
    ///
    /// On non-Unix platforms, lists providers from environment variables.
    #[cfg(not(unix))]
    pub async fn list_providers(&mut self) -> Result<Vec<String>, Error> {
        Ok(self.list_env_providers())
    }

    /// Send a request to the daemon and receive a response.
    ///
    /// This is a low-level method for sending arbitrary requests.
    /// For common operations, use the convenience methods like `get_secret()`.
    ///
    /// The request will time out after the configured timeout (default 30 seconds).
    #[cfg(unix)]
    pub async fn send_request(&mut self, request: Request) -> Result<Response, Error> {
        let timeout_duration = self.timeout;
        let timeout_secs = timeout_duration.as_secs();

        // Wrap the entire operation in a timeout
        tokio::time::timeout(timeout_duration, self.send_request_inner(request))
            .await
            .map_err(|_| Error::Timeout(timeout_secs))?
    }

    /// Inner implementation of send_request without timeout.
    #[cfg(unix)]
    async fn send_request_inner(&mut self, request: Request) -> Result<Response, Error> {
        let stream = self.stream.as_mut().ok_or(Error::NotConnected)?;

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
        let env_var = provider_to_env_var(provider).ok_or_else(|| Error::SecretNotFound {
            provider: provider.to_string(),
            details: format!("Unknown provider: {provider}"),
        })?;
        std::env::var(env_var)
            .map(SecretString::from)
            .map_err(|_| Error::SecretNotFound {
                provider: provider.to_string(),
                details: format!("Environment variable {env_var} not set"),
            })
    }

    fn list_env_providers(&self) -> Vec<String> {
        KNOWN_PROVIDERS
            .iter()
            .filter(|p| std::env::var(p.env_var).is_ok())
            .map(|p| p.id.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_to_env_var() {
        // These now use spn_core::provider_to_env_var which returns Option
        assert_eq!(provider_to_env_var("anthropic"), Some("ANTHROPIC_API_KEY"));
        assert_eq!(provider_to_env_var("openai"), Some("OPENAI_API_KEY"));
        assert_eq!(provider_to_env_var("neo4j"), Some("NEO4J_PASSWORD"));
        assert_eq!(provider_to_env_var("github"), Some("GITHUB_TOKEN"));
        assert_eq!(provider_to_env_var("unknown"), None);
    }

    #[test]
    fn test_socket_path() {
        // socket_path() returns Result, verify it works when HOME is set
        if let Ok(path) = socket_path() {
            assert!(path.to_string_lossy().contains(".spn"));
            assert!(path.to_string_lossy().contains("daemon.sock"));
        }
    }

    #[test]
    fn test_daemon_socket_exists() {
        // Just verify the function runs without panicking
        // The result depends on whether daemon is actually running
        let _exists = daemon_socket_exists();
    }
}
