//! Network integration tests for spn.
//!
//! These tests require network connectivity and are ignored by default.
//! Run with: cargo test --test integration_network -- --ignored

use std::time::Duration;

/// Test that we can connect to GitHub to fetch registry data.
///
/// This verifies the registry URL is accessible and returns valid data.
#[tokio::test]
#[ignore = "requires network - run with: cargo test --test integration_network -- --ignored"]
async fn test_github_registry_connectivity() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let index_url = "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/index";

    // Test that we can reach the registry
    let response = client.head(index_url).send().await;

    match response {
        Ok(resp) => {
            // 2xx or 404 (repo exists, path might not) are acceptable
            let status = resp.status().as_u16();
            assert!(
                (200..300).contains(&status) || status == 404,
                "Unexpected status code: {}",
                status
            );
        }
        Err(e) => {
            // Connection failures are test failures (network issue)
            panic!("Failed to connect to registry: {}", e);
        }
    }
}

/// Test fetching a real package from the registry.
///
/// This test attempts to fetch index data for a known package.
/// It passes if either:
/// - The package is found and has valid data
/// - The package is not found (404 is acceptable for unknown packages)
#[tokio::test]
#[ignore = "requires network - run with: cargo test --test integration_network -- --ignored"]
async fn test_fetch_package_index() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("Failed to create HTTP client");

    // Try to fetch the index file for @workflows scope
    // The sparse index uses a specific path structure: index/@scope/name
    let index_url = "https://raw.githubusercontent.com/supernovae-st/supernovae-registry/main/index/@workflows/dev/code-review";

    let response = client.get(index_url).send().await;

    match response {
        Ok(resp) => {
            let status = resp.status().as_u16();
            if status == 200 {
                // If found, verify it's valid JSON lines (NDJSON)
                let text = resp.text().await.expect("Failed to read response body");
                assert!(
                    !text.is_empty(),
                    "Index file should not be empty if it exists"
                );
                // Each line should be valid JSON (NDJSON format)
                for line in text.lines() {
                    if !line.trim().is_empty() {
                        assert!(
                            serde_json::from_str::<serde_json::Value>(line).is_ok(),
                            "Each line should be valid JSON: {}",
                            line
                        );
                    }
                }
            } else if status == 404 {
                // Package not in registry yet - that's OK
                eprintln!("Note: Package not found in registry (404)");
            } else {
                panic!("Unexpected status code: {}", status);
            }
        }
        Err(e) => {
            panic!("Failed to fetch package index: {}", e);
        }
    }
}

/// Test HTTP timeout behavior.
///
/// Connects to a non-routable IP address (RFC 5737 TEST-NET-1) to verify
/// that timeout configuration works correctly.
#[tokio::test]
#[ignore = "requires network - run with: cargo test --test integration_network -- --ignored"]
async fn test_http_timeout() {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(500))
        .timeout(Duration::from_secs(1))
        .build()
        .expect("Failed to create HTTP client");

    // 192.0.2.1 is from TEST-NET-1 (RFC 5737) - guaranteed non-routable
    let url = "http://192.0.2.1:12345/should-timeout";

    let start = std::time::Instant::now();
    let response = client.get(url).send().await;
    let elapsed = start.elapsed();

    // Should fail with timeout or connection error
    assert!(response.is_err(), "Should fail to connect to non-routable IP");

    // Should complete within reasonable time (timeout + overhead)
    assert!(
        elapsed < Duration::from_secs(3),
        "Timeout should trigger within configured time, took {:?}",
        elapsed
    );
}

/// Test DNS resolution failure handling.
///
/// Attempts to connect to a domain that doesn't exist.
#[tokio::test]
#[ignore = "requires network - run with: cargo test --test integration_network -- --ignored"]
async fn test_dns_failure_handling() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    // This domain should not exist
    let url = "http://this-domain-definitely-does-not-exist-12345.invalid/test";

    let response = client.get(url).send().await;

    // Should fail with DNS or connection error
    assert!(
        response.is_err(),
        "Should fail to resolve non-existent domain"
    );

    let err = response.unwrap_err();
    let err_str = err.to_string().to_lowercase();

    // Error should indicate DNS or connection failure
    assert!(
        err_str.contains("dns")
            || err_str.contains("resolve")
            || err_str.contains("connect")
            || err_str.contains("name"),
        "Error should indicate DNS/connection failure: {}",
        err_str
    );
}
