//! Integration tests for spn-ollama.
//!
//! These tests require a running Ollama instance.
//! Tests are skipped if Ollama is not available.

use spn_ollama::{ClientConfig, OllamaClient, DEFAULT_ENDPOINT};
use std::time::Duration;

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
