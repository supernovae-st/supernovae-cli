//! Request handler for daemon commands.

use secrecy::ExposeSecret;
use spn_client::{Request, Response};
use std::sync::Arc;
use tracing::{debug, warn};

use super::{ModelManager, SecretManager};

/// Handles incoming daemon requests.
pub struct RequestHandler {
    /// Secret manager
    secrets: Arc<SecretManager>,

    /// Model manager
    models: Arc<ModelManager>,

    /// Daemon version
    version: String,
}

impl RequestHandler {
    /// Create a new request handler.
    pub fn new(secrets: Arc<SecretManager>, models: Arc<ModelManager>) -> Self {
        Self {
            secrets,
            models,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Handle a request and return a response.
    pub async fn handle(&self, request: Request) -> Response {
        debug!("Handling request: {:?}", request);

        match request {
            Request::Ping => self.handle_ping(),
            Request::GetSecret { provider } => self.handle_get_secret(&provider).await,
            Request::HasSecret { provider } => self.handle_has_secret(&provider).await,
            Request::ListProviders => self.handle_list_providers().await,

            // Model commands
            Request::ModelList => self.handle_model_list().await,
            Request::ModelPull { name } => self.handle_model_pull(&name).await,
            Request::ModelLoad { name, config } => self.handle_model_load(&name, config).await,
            Request::ModelUnload { name } => self.handle_model_unload(&name).await,
            Request::ModelStatus => self.handle_model_status().await,
            Request::ModelDelete { name } => self.handle_model_delete(&name).await,
        }
    }

    fn handle_ping(&self) -> Response {
        Response::Pong {
            version: self.version.clone(),
        }
    }

    async fn handle_get_secret(&self, provider: &str) -> Response {
        match self.secrets.get_cached(provider).await {
            Some(secret) => Response::Secret {
                value: secret.expose_secret().to_string(),
            },
            None => {
                warn!("Secret not found for provider: {}", provider);
                Response::Error {
                    message: format!("Secret not found for provider: {}", provider),
                }
            }
        }
    }

    async fn handle_has_secret(&self, provider: &str) -> Response {
        let exists = self.secrets.has_cached(provider).await;
        Response::Exists { exists }
    }

    async fn handle_list_providers(&self) -> Response {
        let providers = self.secrets.list_cached().await;
        Response::Providers { providers }
    }

    // ==================== Model Handlers ====================

    async fn handle_model_list(&self) -> Response {
        match self.models.list_models().await {
            Ok(models) => Response::Models { models },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_pull(&self, name: &str) -> Response {
        match self.models.pull(name).await {
            Ok(()) => Response::Success { success: true },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_load(
        &self,
        name: &str,
        config: Option<spn_client::LoadConfig>,
    ) -> Response {
        match self.models.load(name, config).await {
            Ok(()) => Response::Success { success: true },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_unload(&self, name: &str) -> Response {
        match self.models.unload(name).await {
            Ok(()) => Response::Success { success: true },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_status(&self) -> Response {
        match self.models.running_models().await {
            Ok(running) => Response::RunningModels { running },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_model_delete(&self, name: &str) -> Response {
        match self.models.delete(name).await {
            Ok(()) => Response::Success { success: true },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_handler() -> RequestHandler {
        let secrets = Arc::new(SecretManager::new());
        let models = Arc::new(ModelManager::new());
        RequestHandler::new(secrets, models)
    }

    fn create_handler_with_secrets() -> (RequestHandler, Arc<SecretManager>) {
        let secrets = Arc::new(SecretManager::new());
        let models = Arc::new(ModelManager::new());
        let handler = RequestHandler::new(Arc::clone(&secrets), models);
        (handler, secrets)
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let handler = create_handler();

        let response = handler.handle(Request::Ping).await;

        match response {
            Response::Pong { version } => {
                assert!(!version.is_empty());
            }
            _ => panic!("Expected Pong response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_secret_found() {
        let (handler, secrets) = create_handler_with_secrets();
        secrets.set_cached("test", "secret-value").await.unwrap();

        let response = handler
            .handle(Request::GetSecret {
                provider: "test".to_string(),
            })
            .await;

        match response {
            Response::Secret { value } => {
                assert_eq!(value, "secret-value");
            }
            _ => panic!("Expected Secret response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_secret_not_found() {
        let handler = create_handler();

        let response = handler
            .handle(Request::GetSecret {
                provider: "nonexistent".to_string(),
            })
            .await;

        match response {
            Response::Error { message } => {
                assert!(message.contains("nonexistent"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_has_secret() {
        let (handler, secrets) = create_handler_with_secrets();
        secrets.set_cached("test", "value").await.unwrap();

        // Existing secret
        let response = handler
            .handle(Request::HasSecret {
                provider: "test".to_string(),
            })
            .await;
        assert!(matches!(response, Response::Exists { exists: true }));

        // Non-existing secret
        let response = handler
            .handle(Request::HasSecret {
                provider: "nonexistent".to_string(),
            })
            .await;
        assert!(matches!(response, Response::Exists { exists: false }));
    }

    #[tokio::test]
    async fn test_handle_list_providers() {
        let (handler, secrets) = create_handler_with_secrets();
        secrets.set_cached("anthropic", "key1").await.unwrap();
        secrets.set_cached("openai", "key2").await.unwrap();

        let response = handler.handle(Request::ListProviders).await;

        match response {
            Response::Providers { providers } => {
                assert_eq!(providers.len(), 2);
                assert!(providers.contains(&"anthropic".to_string()));
                assert!(providers.contains(&"openai".to_string()));
            }
            _ => panic!("Expected Providers response"),
        }
    }
}
