//! Integration tests for spn-ollama.
//!
//! These tests require a running Ollama instance.
//! Tests are skipped if Ollama is not available.
//!
//! For inference tests, set `OLLAMA_TEST_MODEL` env var (default: tinyllama).

use spn_core::ChatMessage;
use spn_ollama::{ClientConfig, OllamaBackend, OllamaClient, DEFAULT_ENDPOINT};
use std::time::Duration;

/// Default model for inference tests (small and fast).
const DEFAULT_TEST_MODEL: &str = "tinyllama";

/// Skip test if Ollama is not running.
async fn skip_if_ollama_unavailable() -> Option<OllamaClient> {
    let client = OllamaClient::new();
    if client.is_running().await {
        Some(client)
    } else {
        eprintln!("Skipping test: Ollama not running at {DEFAULT_ENDPOINT}");
        None
    }
}

/// Get test model from env or use default.
fn get_test_model() -> String {
    std::env::var("OLLAMA_TEST_MODEL").unwrap_or_else(|_| DEFAULT_TEST_MODEL.to_string())
}

/// Check if the test model is available.
async fn skip_if_model_unavailable(client: &OllamaClient) -> bool {
    let model = get_test_model();
    match client.model_info(&model).await {
        Ok(_) => true,
        Err(_) => {
            eprintln!(
                "Skipping test: model '{}' not installed. Pull with: ollama pull {}",
                model, model
            );
            false
        }
    }
}

#[tokio::test]
async fn test_is_running() {
    let client = OllamaClient::new();
    // This test always runs - it just checks the connection
    let _ = client.is_running().await;
}

#[tokio::test]
async fn test_list_models() {
    let Some(client) = skip_if_ollama_unavailable().await else {
        return;
    };

    let models = client.list_models().await;
    assert!(models.is_ok(), "list_models should succeed: {:?}", models);
}

#[tokio::test]
async fn test_running_models() {
    let Some(client) = skip_if_ollama_unavailable().await else {
        return;
    };

    let models = client.running_models().await;
    assert!(
        models.is_ok(),
        "running_models should succeed: {:?}",
        models
    );
}

#[tokio::test]
async fn test_model_info_not_found() {
    let Some(client) = skip_if_ollama_unavailable().await else {
        return;
    };

    let result = client.model_info("nonexistent-model-xyz-12345").await;
    assert!(result.is_err(), "nonexistent model should return error");

    if let Err(e) = result {
        let error_str = e.to_string();
        // Should be ModelNotFound or contain relevant error
        assert!(
            error_str.contains("not found") || error_str.contains("404"),
            "Expected not found error, got: {}",
            error_str
        );
    }
}

#[tokio::test]
async fn test_custom_config() {
    let config = ClientConfig::new()
        .with_connect_timeout(Duration::from_secs(10))
        .with_request_timeout(Duration::from_secs(60))
        .with_model_timeout(Duration::from_secs(600))
        .with_max_retries(5)
        .with_retry_delay(Duration::from_millis(100));

    let client = OllamaClient::with_config("http://localhost:11434", config);
    assert_eq!(client.endpoint(), "http://localhost:11434");
    assert_eq!(client.config().connect_timeout, Duration::from_secs(10));
    assert_eq!(client.config().request_timeout, Duration::from_secs(60));
    assert_eq!(client.config().model_timeout, Duration::from_secs(600));
    assert_eq!(client.config().max_retries, 5);
}

#[tokio::test]
async fn test_no_retries_config() {
    let config = ClientConfig::new().no_retries();
    assert_eq!(config.max_retries, 0);
}

#[tokio::test]
async fn test_custom_endpoint() {
    let client = OllamaClient::with_endpoint("http://custom:8080");
    assert_eq!(client.endpoint(), "http://custom:8080");
}

// ============================================================================
// Inference Tests (require model)
// ============================================================================

#[tokio::test]
async fn test_chat_simple() {
    let Some(client) = skip_if_ollama_unavailable().await else {
        return;
    };

    if !skip_if_model_unavailable(&client).await {
        return;
    }

    let model = get_test_model();
    let messages = vec![ChatMessage::user("Say hello in exactly 3 words.")];

    let result = client.chat(&model, &messages, None).await;
    assert!(result.is_ok(), "chat should succeed: {:?}", result);

    let response = result.unwrap();
    assert!(response.done, "response should be complete");
    assert!(
        !response.message.content.is_empty(),
        "response should have content"
    );
}

#[tokio::test]
async fn test_chat_stream() {
    let Some(client) = skip_if_ollama_unavailable().await else {
        return;
    };

    if !skip_if_model_unavailable(&client).await {
        return;
    }

    let model = get_test_model();
    let messages = vec![ChatMessage::user("Count from 1 to 3.")];

    let mut tokens = Vec::new();
    let result = client
        .chat_stream(&model, &messages, None, |token| {
            tokens.push(token.to_string());
        })
        .await;

    assert!(result.is_ok(), "chat_stream should succeed: {:?}", result);
    assert!(!tokens.is_empty(), "should receive tokens");

    let response = result.unwrap();
    assert!(response.done, "response should be complete");
}

#[tokio::test]
async fn test_embed_simple() {
    let Some(client) = skip_if_ollama_unavailable().await else {
        return;
    };

    if !skip_if_model_unavailable(&client).await {
        return;
    }

    let model = get_test_model();
    let result = client.embed(&model, "Hello world").await;

    // Note: Not all models support embeddings, so we accept both success and error
    match result {
        Ok(response) => {
            assert!(
                !response.embedding.is_empty(),
                "embedding should have values"
            );
        }
        Err(e) => {
            // Some models don't support embeddings - that's OK
            eprintln!("Note: embed not supported by {}: {}", model, e);
        }
    }
}

// ============================================================================
// Backend Tests
// ============================================================================

#[tokio::test]
async fn test_backend_with_config() {
    let config = ClientConfig::new()
        .with_model_timeout(Duration::from_secs(600))
        .with_connect_timeout(Duration::from_secs(10));

    let backend = OllamaBackend::with_config("http://localhost:11434", config);
    assert_eq!(backend.client().endpoint(), "http://localhost:11434");
    assert_eq!(
        backend.client().config().model_timeout,
        Duration::from_secs(600)
    );
}
