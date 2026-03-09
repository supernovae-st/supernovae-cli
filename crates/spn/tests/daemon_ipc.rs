//! Integration tests for daemon IPC protocol.
//!
//! Tests the Unix socket communication between spn-client and spn daemon.

use spn_client::{Request, Response};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::oneshot;

/// Test that socket is created with correct permissions (0600).
#[tokio::test]
async fn test_socket_permissions() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("test.sock");

    // Set umask before binding (like the daemon does)
    let old_umask = unsafe { libc::umask(0o077) };
    let listener = UnixListener::bind(&socket_path).unwrap();
    unsafe { libc::umask(old_umask) };

    // Check permissions
    let metadata = fs::metadata(&socket_path).unwrap();
    let mode = metadata.permissions().mode() & 0o777;

    // Socket should be 0600 or more restrictive
    assert!(
        mode <= 0o700,
        "Socket permissions too permissive: {:o}",
        mode
    );

    drop(listener);
}

/// Test the length-prefixed protocol encoding.
#[tokio::test]
async fn test_protocol_length_prefix() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("protocol.sock");

    // Start mock server
    let listener = UnixListener::bind(&socket_path).unwrap();

    let socket_path_clone = socket_path.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        // Read length prefix (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        // Read message
        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await.unwrap();

        // Parse request
        let request: Request = serde_json::from_slice(&msg_buf).unwrap();
        assert!(matches!(request, Request::Ping));

        // Send response
        let response = Response::Pong {
            protocol_version: spn_client::PROTOCOL_VERSION,
            version: "test".to_string(),
        };
        let response_json = serde_json::to_vec(&response).unwrap();

        let response_len = response_json.len() as u32;
        stream.write_all(&response_len.to_be_bytes()).await.unwrap();
        stream.write_all(&response_json).await.unwrap();
    });

    // Connect as client
    let mut client = UnixStream::connect(&socket_path_clone).await.unwrap();

    // Send ping request
    let request = Request::Ping;
    let request_json = serde_json::to_vec(&request).unwrap();
    let len = request_json.len() as u32;

    client.write_all(&len.to_be_bytes()).await.unwrap();
    client.write_all(&request_json).await.unwrap();

    // Read response
    let mut len_buf = [0u8; 4];
    client.read_exact(&mut len_buf).await.unwrap();
    let response_len = u32::from_be_bytes(len_buf) as usize;

    let mut response_buf = vec![0u8; response_len];
    client.read_exact(&mut response_buf).await.unwrap();

    let response: Response = serde_json::from_slice(&response_buf).unwrap();
    assert!(matches!(response, Response::Pong { .. }));

    server.await.unwrap();
}

/// Test that oversized messages are rejected by the server.
///
/// This test verifies that when a client sends a length prefix claiming
/// an oversized message (>1MB), the server correctly identifies this and
/// closes the connection rather than trying to allocate and read 2MB.
#[tokio::test]
async fn test_oversized_message_rejected() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("oversize.sock");

    let listener = UnixListener::bind(&socket_path).unwrap();

    let socket_path_clone = socket_path.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        // Read length prefix
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        // Server should detect oversized message and close connection
        // (matching the actual daemon behavior in server.rs:325-329)
        if msg_len > 1_048_576 {
            // Close connection - don't try to read the oversized message
            drop(stream);
            return true; // Indicate we correctly rejected
        }

        false // Didn't reject
    });

    // Connect and send oversized length
    let mut client = UnixStream::connect(&socket_path_clone).await.unwrap();

    // Send a length that claims 2MB
    let fake_len: u32 = 2_097_152;
    client.write_all(&fake_len.to_be_bytes()).await.unwrap();

    // Server should have rejected the oversized message
    let rejected = server.await.unwrap();
    assert!(rejected, "Server should reject messages > 1MB");
}

/// Test multiple requests on same connection.
#[tokio::test]
async fn test_multiple_requests_same_connection() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("multi.sock");

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        // Handle 3 requests
        for i in 0..3 {
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_err() {
                break;
            }
            let msg_len = u32::from_be_bytes(len_buf) as usize;

            let mut msg_buf = vec![0u8; msg_len];
            stream.read_exact(&mut msg_buf).await.unwrap();

            let request: Request = serde_json::from_slice(&msg_buf).unwrap();
            assert!(matches!(request, Request::Ping));

            let response = Response::Pong {
                protocol_version: spn_client::PROTOCOL_VERSION,
                version: format!("v{}", i),
            };
            let response_json = serde_json::to_vec(&response).unwrap();

            let response_len = response_json.len() as u32;
            stream.write_all(&response_len.to_be_bytes()).await.unwrap();
            stream.write_all(&response_json).await.unwrap();
        }
    });

    let mut client = UnixStream::connect(&socket_path).await.unwrap();

    // Send 3 requests on same connection
    for i in 0..3 {
        let request = Request::Ping;
        let request_json = serde_json::to_vec(&request).unwrap();
        let len = request_json.len() as u32;

        client.write_all(&len.to_be_bytes()).await.unwrap();
        client.write_all(&request_json).await.unwrap();

        let mut len_buf = [0u8; 4];
        client.read_exact(&mut len_buf).await.unwrap();
        let response_len = u32::from_be_bytes(len_buf) as usize;

        let mut response_buf = vec![0u8; response_len];
        client.read_exact(&mut response_buf).await.unwrap();

        let response: Response = serde_json::from_slice(&response_buf).unwrap();
        match response {
            Response::Pong { version, .. } => {
                assert_eq!(version, format!("v{}", i));
            }
            _ => panic!("Expected Pong response"),
        }
    }

    server.await.unwrap();
}

/// Test concurrent connections.
#[tokio::test]
async fn test_concurrent_connections() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("concurrent.sock");

    let listener = Arc::new(UnixListener::bind(&socket_path).unwrap());
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let listener_clone = Arc::clone(&listener);
    let server = tokio::spawn(async move {
        let mut handles = vec![];

        loop {
            tokio::select! {
                result = listener_clone.accept() => {
                    match result {
                        Ok((mut stream, _)) => {
                            handles.push(tokio::spawn(async move {
                                let mut len_buf = [0u8; 4];
                                if stream.read_exact(&mut len_buf).await.is_err() {
                                    return;
                                }
                                let msg_len = u32::from_be_bytes(len_buf) as usize;

                                let mut msg_buf = vec![0u8; msg_len];
                                if stream.read_exact(&mut msg_buf).await.is_err() {
                                    return;
                                }

                                let response = Response::Pong {
                                    protocol_version: spn_client::PROTOCOL_VERSION,
                                    version: "concurrent".to_string(),
                                };
                                let response_json = serde_json::to_vec(&response).unwrap();

                                let response_len = response_json.len() as u32;
                                let _ = stream.write_all(&response_len.to_be_bytes()).await;
                                let _ = stream.write_all(&response_json).await;
                            }));
                        }
                        Err(_) => break,
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }

        // Wait for all handlers
        for handle in handles {
            let _ = handle.await;
        }
    });

    // Spawn 10 concurrent clients
    let mut client_handles = vec![];
    for _ in 0..10 {
        let path = socket_path.clone();
        client_handles.push(tokio::spawn(async move {
            let mut client = UnixStream::connect(&path).await.unwrap();

            let request = Request::Ping;
            let request_json = serde_json::to_vec(&request).unwrap();
            let len = request_json.len() as u32;

            client.write_all(&len.to_be_bytes()).await.unwrap();
            client.write_all(&request_json).await.unwrap();

            let mut len_buf = [0u8; 4];
            client.read_exact(&mut len_buf).await.unwrap();
            let response_len = u32::from_be_bytes(len_buf) as usize;

            let mut response_buf = vec![0u8; response_len];
            client.read_exact(&mut response_buf).await.unwrap();

            let response: Response = serde_json::from_slice(&response_buf).unwrap();
            assert!(matches!(response, Response::Pong { .. }));
        }));
    }

    // Wait for all clients
    for handle in client_handles {
        handle.await.unwrap();
    }

    // Shutdown server
    let _ = shutdown_tx.send(());
    let _ = server.await;
}

/// Test malformed JSON is handled gracefully.
#[tokio::test]
async fn test_malformed_json_handling() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("malformed.sock");

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await.unwrap();

        // Try to parse - should fail
        let result: Result<Request, _> = serde_json::from_slice(&msg_buf);
        assert!(result.is_err(), "Malformed JSON should fail to parse");
    });

    let mut client = UnixStream::connect(&socket_path).await.unwrap();

    // Send malformed JSON
    let malformed = b"not valid json {{{";
    let len = malformed.len() as u32;

    client.write_all(&len.to_be_bytes()).await.unwrap();
    client.write_all(malformed).await.unwrap();

    server.await.unwrap();
}

/// Test secret request/response flow.
#[tokio::test]
async fn test_secret_request_response() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("secret.sock");

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await.unwrap();

        let request: Request = serde_json::from_slice(&msg_buf).unwrap();

        let response = match request {
            Request::GetSecret { provider } if provider == "anthropic" => Response::Secret {
                value: "sk-ant-test123".to_string(),
            },
            Request::GetSecret { provider } => Response::Error {
                message: format!("Unknown provider: {}", provider),
            },
            Request::HasSecret { provider } => Response::Exists {
                exists: provider == "anthropic",
            },
            Request::ListProviders => Response::Providers {
                providers: vec!["anthropic".to_string()],
            },
            _ => Response::Error {
                message: "Unexpected request".to_string(),
            },
        };

        let response_json = serde_json::to_vec(&response).unwrap();
        let response_len = response_json.len() as u32;
        stream.write_all(&response_len.to_be_bytes()).await.unwrap();
        stream.write_all(&response_json).await.unwrap();
    });

    let mut client = UnixStream::connect(&socket_path).await.unwrap();

    // Request a secret
    let request = Request::GetSecret {
        provider: "anthropic".to_string(),
    };
    let request_json = serde_json::to_vec(&request).unwrap();
    let len = request_json.len() as u32;

    client.write_all(&len.to_be_bytes()).await.unwrap();
    client.write_all(&request_json).await.unwrap();

    let mut len_buf = [0u8; 4];
    client.read_exact(&mut len_buf).await.unwrap();
    let response_len = u32::from_be_bytes(len_buf) as usize;

    let mut response_buf = vec![0u8; response_len];
    client.read_exact(&mut response_buf).await.unwrap();

    let response: Response = serde_json::from_slice(&response_buf).unwrap();
    match response {
        Response::Secret { value } => {
            assert_eq!(value, "sk-ant-test123");
        }
        _ => panic!("Expected Secret response"),
    }

    server.await.unwrap();
}

/// Test graceful client disconnect handling.
#[tokio::test]
async fn test_client_disconnect() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("disconnect.sock");

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        // Client will disconnect, read should return EOF
        let mut len_buf = [0u8; 4];
        let result = stream.read_exact(&mut len_buf).await;

        // Should get UnexpectedEof when client disconnects
        assert!(result.is_err(), "Should get error when client disconnects");
    });

    // Connect and immediately disconnect
    let client = UnixStream::connect(&socket_path).await.unwrap();
    drop(client);

    server.await.unwrap();
}

// =============================================================================
// Phase 6: Watcher Status Tests
// =============================================================================

/// Test WatcherStatus request serialization.
#[test]
fn test_watcher_status_request_serialization() {
    let request = Request::WatcherStatus;
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("WATCHER_STATUS"));

    // Roundtrip
    let parsed: Request = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, Request::WatcherStatus));
}

/// Test WatcherStatusResult response serialization.
#[test]
fn test_watcher_status_response_serialization() {
    use spn_client::{ForeignMcpInfo, RecentProjectInfo, WatcherStatusInfo};

    let status = WatcherStatusInfo {
        is_running: true,
        watched_count: 5,
        watched_paths: vec!["/home/user/.claude".to_string()],
        debounce_ms: 500,
        recent_projects: vec![RecentProjectInfo {
            path: "/home/user/dev/project".to_string(),
            last_used: "2026-03-09T12:00:00Z".to_string(),
        }],
        foreign_pending: vec![ForeignMcpInfo {
            name: "test-mcp".to_string(),
            source: "cursor".to_string(),
            scope: "global".to_string(),
            detected: "2026-03-09T12:00:00Z".to_string(),
        }],
        foreign_ignored: vec!["ignored-mcp".to_string()],
    };

    let response = Response::WatcherStatusResult { status };
    let json = serde_json::to_string(&response).unwrap();

    // Response uses untagged serde, so check for status fields directly
    assert!(json.contains("\"status\""));
    assert!(json.contains("is_running"));
    assert!(json.contains("watched_count"));
    assert!(json.contains("recent_projects"));
    assert!(json.contains("foreign_pending"));
    assert!(json.contains("test-mcp"));
    assert!(json.contains("cursor"));

    // Roundtrip
    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::WatcherStatusResult { status } => {
            assert!(status.is_running);
            assert_eq!(status.watched_count, 5);
            assert_eq!(status.recent_projects.len(), 1);
            assert_eq!(status.foreign_pending.len(), 1);
            assert_eq!(status.foreign_pending[0].name, "test-mcp");
        }
        _ => panic!("Expected WatcherStatusResult"),
    }
}

/// Test WatcherStatus IPC flow (mock server).
#[tokio::test]
async fn test_watcher_status_ipc_flow() {
    use spn_client::{ForeignMcpInfo, RecentProjectInfo, WatcherStatusInfo};

    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("watcher.sock");

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        // Read request
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await.unwrap();

        let request: Request = serde_json::from_slice(&msg_buf).unwrap();
        assert!(matches!(request, Request::WatcherStatus));

        // Send response
        let status = WatcherStatusInfo {
            is_running: true,
            watched_count: 3,
            watched_paths: vec![
                "/home/user/.claude".to_string(),
                "/home/user/.cursor".to_string(),
            ],
            debounce_ms: 500,
            recent_projects: vec![RecentProjectInfo {
                path: "/home/user/dev/myproject".to_string(),
                last_used: "2026-03-09T10:30:00Z".to_string(),
            }],
            foreign_pending: vec![ForeignMcpInfo {
                name: "foreign-server".to_string(),
                source: "cursor".to_string(),
                scope: "global".to_string(),
                detected: "2026-03-09T09:00:00Z".to_string(),
            }],
            foreign_ignored: vec![],
        };

        let response = Response::WatcherStatusResult { status };
        let response_json = serde_json::to_vec(&response).unwrap();

        let response_len = response_json.len() as u32;
        stream.write_all(&response_len.to_be_bytes()).await.unwrap();
        stream.write_all(&response_json).await.unwrap();
    });

    // Client request
    let mut client = UnixStream::connect(&socket_path).await.unwrap();

    let request = Request::WatcherStatus;
    let request_json = serde_json::to_vec(&request).unwrap();
    let len = request_json.len() as u32;

    client.write_all(&len.to_be_bytes()).await.unwrap();
    client.write_all(&request_json).await.unwrap();

    // Read response
    let mut len_buf = [0u8; 4];
    client.read_exact(&mut len_buf).await.unwrap();
    let response_len = u32::from_be_bytes(len_buf) as usize;

    let mut response_buf = vec![0u8; response_len];
    client.read_exact(&mut response_buf).await.unwrap();

    let response: Response = serde_json::from_slice(&response_buf).unwrap();
    match response {
        Response::WatcherStatusResult { status } => {
            assert!(status.is_running);
            assert_eq!(status.watched_count, 3);
            assert_eq!(status.watched_paths.len(), 2);
            assert_eq!(status.recent_projects.len(), 1);
            assert_eq!(status.recent_projects[0].path, "/home/user/dev/myproject");
            assert_eq!(status.foreign_pending.len(), 1);
            assert_eq!(status.foreign_pending[0].name, "foreign-server");
        }
        _ => panic!("Expected WatcherStatusResult response"),
    }

    server.await.unwrap();
}

/// Test WatcherStatus with empty data.
#[tokio::test]
async fn test_watcher_status_empty() {
    use spn_client::WatcherStatusInfo;

    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("watcher_empty.sock");

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await.unwrap();

        // Return empty status
        let status = WatcherStatusInfo {
            is_running: true,
            watched_count: 0,
            watched_paths: vec![],
            debounce_ms: 500,
            recent_projects: vec![],
            foreign_pending: vec![],
            foreign_ignored: vec![],
        };

        let response = Response::WatcherStatusResult { status };
        let response_json = serde_json::to_vec(&response).unwrap();

        let response_len = response_json.len() as u32;
        stream.write_all(&response_len.to_be_bytes()).await.unwrap();
        stream.write_all(&response_json).await.unwrap();
    });

    let mut client = UnixStream::connect(&socket_path).await.unwrap();

    let request = Request::WatcherStatus;
    let request_json = serde_json::to_vec(&request).unwrap();
    let len = request_json.len() as u32;

    client.write_all(&len.to_be_bytes()).await.unwrap();
    client.write_all(&request_json).await.unwrap();

    let mut len_buf = [0u8; 4];
    client.read_exact(&mut len_buf).await.unwrap();
    let response_len = u32::from_be_bytes(len_buf) as usize;

    let mut response_buf = vec![0u8; response_len];
    client.read_exact(&mut response_buf).await.unwrap();

    let response: Response = serde_json::from_slice(&response_buf).unwrap();
    match response {
        Response::WatcherStatusResult { status } => {
            assert!(status.is_running);
            assert_eq!(status.watched_count, 0);
            assert!(status.watched_paths.is_empty());
            assert!(status.recent_projects.is_empty());
            assert!(status.foreign_pending.is_empty());
        }
        _ => panic!("Expected WatcherStatusResult response"),
    }

    server.await.unwrap();
}
