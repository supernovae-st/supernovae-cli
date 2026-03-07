//! Unix socket server for the daemon.
//!
//! TODO(v0.14): Integrate model backend features

#![allow(dead_code)]

use spn_client::Request;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::signal;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use super::{
    handler::RequestHandler,
    model_manager::ModelManager,
    paths,
    secrets::SecretManager,
    socket::{verify_peer_credentials, SocketUtils},
    DaemonError,
};

/// Daemon server configuration.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Socket path
    pub socket_path: PathBuf,
    /// PID file path
    pub pid_file: PathBuf,
    /// Whether to preload secrets on start
    pub preload_secrets: bool,
}

impl DaemonConfig {
    /// Create a new daemon configuration with default paths.
    ///
    /// Returns an error if HOME directory is not available.
    pub fn new() -> Result<Self, DaemonError> {
        Ok(Self {
            socket_path: paths::socket().map_err(|e| DaemonError::ConfigError(e.to_string()))?,
            pid_file: paths::pid_file().map_err(|e| DaemonError::ConfigError(e.to_string()))?,
            preload_secrets: true,
        })
    }
}

/// The daemon server.
pub struct DaemonServer {
    config: DaemonConfig,
    secrets: Arc<SecretManager>,
    models: Arc<ModelManager>,
    handler: Arc<RequestHandler>,
    shutdown_tx: broadcast::Sender<()>,
    /// PID file handle - held to maintain flock until shutdown
    pid_file_lock: Option<File>,
}

impl DaemonServer {
    /// Create a new daemon server.
    pub fn new(config: DaemonConfig) -> Self {
        let secrets = Arc::new(SecretManager::new());
        let models = Arc::new(ModelManager::new());
        let handler = Arc::new(RequestHandler::new(
            Arc::clone(&secrets),
            Arc::clone(&models),
        ));
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config,
            secrets,
            models,
            handler,
            shutdown_tx,
            pid_file_lock: None,
        }
    }

    /// Run the daemon server.
    pub async fn run(&mut self) -> Result<(), DaemonError> {
        // Ensure the .spn directory exists
        self.ensure_spn_dir()?;

        // Create PID file (with flock) and store handle to maintain lock
        self.pid_file_lock = Some(self.create_pid_file()?);

        // Clean up any stale socket
        SocketUtils::cleanup_stale_socket(&self.config.socket_path)?;

        // Preload secrets if configured
        if self.config.preload_secrets {
            if let Err(e) = self.secrets.preload_all().await {
                warn!("Failed to preload some secrets: {}", e);
            }
        }

        // Bind to socket
        let listener = self.bind_socket()?;

        info!("Daemon listening on {:?}", self.config.socket_path);

        // Run the accept loop with graceful shutdown
        self.accept_loop(listener).await?;

        // Cleanup (pid_file_lock is dropped automatically, releasing flock)
        self.cleanup();

        Ok(())
    }

    /// Ensure the .spn directory exists.
    fn ensure_spn_dir(&self) -> Result<(), DaemonError> {
        let dir = paths::spn_dir().map_err(|e| DaemonError::ConfigError(e.to_string()))?;
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .map_err(|source| DaemonError::CreateDirFailed { path: dir, source })?;
        }
        Ok(())
    }

    /// Create PID file with exclusive lock.
    ///
    /// Returns the File handle which MUST be held to maintain the flock.
    /// Dropping the File releases the lock.
    fn create_pid_file(&self) -> Result<File, DaemonError> {
        let pid = std::process::id();
        let path = &self.config.pid_file;

        // Check if another daemon is running
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(old_pid) = content.trim().parse::<u32>() {
                    // Check if process is still running
                    if is_process_running(old_pid) {
                        return Err(DaemonError::AlreadyRunning {
                            pid_file: path.clone(),
                        });
                    }
                }
            }
            // Stale PID file, remove it
            fs::remove_file(path).ok();
        }

        // Create new PID file
        let mut file = File::create(path).map_err(|source| DaemonError::PidFileFailed {
            path: path.clone(),
            source,
        })?;

        // Try to acquire exclusive lock
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = file.as_raw_fd();
            let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if result != 0 {
                return Err(DaemonError::LockFailed {
                    path: path.clone(),
                    source: std::io::Error::last_os_error(),
                });
            }
        }

        // Write PID
        writeln!(file, "{}", pid).map_err(|source| DaemonError::PidFileFailed {
            path: path.clone(),
            source,
        })?;

        debug!("Created PID file: {:?} (PID: {})", path, pid);

        // Return the file handle - caller MUST hold it to maintain the lock
        Ok(file)
    }

    /// Bind to the Unix socket.
    ///
    /// Uses umask to ensure the socket is created with restrictive permissions
    /// from the start, preventing any race condition between bind and chmod.
    fn bind_socket(&self) -> Result<UnixListener, DaemonError> {
        let path = &self.config.socket_path;

        // Set restrictive umask BEFORE bind to prevent race condition.
        // umask(0o077) means new files get permissions & ~0o077 = 0o700 max.
        // For sockets, this results in 0o600 (rw-------).
        let old_umask = unsafe { libc::umask(0o077) };

        // Bind (socket is created with restricted permissions immediately)
        let bind_result = UnixListener::bind(path);

        // Restore original umask regardless of bind result
        unsafe { libc::umask(old_umask) };

        let listener = bind_result.map_err(|source| DaemonError::BindFailed {
            path: path.clone(),
            source,
        })?;

        // Double-check permissions are correct (defense in depth)
        SocketUtils::set_socket_permissions(path)?;

        Ok(listener)
    }

    /// Accept loop with graceful shutdown.
    ///
    /// Tracks all spawned connection handlers and drains them on shutdown.
    #[cfg(unix)]
    async fn accept_loop(&self, listener: UnixListener) -> Result<(), DaemonError> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut tasks: JoinSet<()> = JoinSet::new();

        // Set up SIGTERM handler
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .map_err(DaemonError::IoError)?;

        loop {
            tokio::select! {
                // Accept new connection
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let handler = Arc::clone(&self.handler);
                            tasks.spawn(async move {
                                if let Err(e) = handle_connection(stream, handler).await {
                                    error!("Connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }

                // Clean up completed tasks
                Some(result) = tasks.join_next() => {
                    if let Err(e) = result {
                        error!("Task panicked: {:?}", e);
                    }
                }

                // SIGINT (Ctrl+C)
                _ = signal::ctrl_c() => {
                    info!("Received SIGINT, shutting down...");
                    let _ = self.shutdown_tx.send(());
                    break;
                }

                // SIGTERM
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down...");
                    let _ = self.shutdown_tx.send(());
                    break;
                }

                // Broadcast shutdown
                _ = shutdown_rx.recv() => {
                    info!("Shutdown requested");
                    break;
                }
            }
        }

        // Graceful shutdown: wait for all in-flight connections to complete
        Self::drain_tasks(&mut tasks).await;

        Ok(())
    }

    /// Drain all remaining tasks on shutdown with timeout.
    ///
    /// Waits up to 5 seconds for active connections to complete gracefully.
    /// After timeout, remaining tasks are aborted.
    async fn drain_tasks(tasks: &mut JoinSet<()>) {
        use std::time::Duration;

        if tasks.is_empty() {
            return;
        }

        let count = tasks.len();
        info!("Waiting for {} active connections to complete...", count);

        // Give connections 5 seconds to finish gracefully
        let drain_timeout = Duration::from_secs(5);
        let deadline = tokio::time::Instant::now() + drain_timeout;

        // Pin the sleep future once, avoid recreating on each loop iteration
        let sleep = tokio::time::sleep_until(deadline);
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                result = tasks.join_next() => {
                    match result {
                        Some(Ok(())) => {}
                        Some(Err(e)) => {
                            error!("Task panicked during shutdown: {:?}", e);
                        }
                        None => {
                            info!("All connections completed");
                            return;
                        }
                    }
                }
                _ = &mut sleep => {
                    let remaining = tasks.len();
                    if remaining > 0 {
                        warn!(
                            "Drain timeout reached, aborting {} remaining connections",
                            remaining
                        );
                        tasks.abort_all();
                        // Wait for aborted tasks to finish
                        while tasks.join_next().await.is_some() {}
                    }
                    return;
                }
            }
        }
    }

    /// Clean up resources on shutdown.
    fn cleanup(&self) {
        // Remove socket file
        if self.config.socket_path.exists() {
            if let Err(e) = fs::remove_file(&self.config.socket_path) {
                warn!("Failed to remove socket file: {}", e);
            }
        }

        // Remove PID file
        if self.config.pid_file.exists() {
            if let Err(e) = fs::remove_file(&self.config.pid_file) {
                warn!("Failed to remove PID file: {}", e);
            }
        }

        info!("Daemon stopped");
    }

    /// Send shutdown signal.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Handle a single connection.
async fn handle_connection(
    stream: UnixStream,
    handler: Arc<RequestHandler>,
) -> Result<(), DaemonError> {
    // Verify peer credentials using spawn_blocking for the getsockopt syscall
    let std_stream = stream.into_std()?;
    let std_stream = tokio::task::spawn_blocking(move || {
        verify_peer_credentials(&std_stream)?;
        Ok::<_, DaemonError>(std_stream)
    })
    .await
    .map_err(|e| DaemonError::IoError(std::io::Error::other(format!("task join failed: {e}"))))??;

    // Convert back to async
    let mut stream = UnixStream::from_std(std_stream)?;

    loop {
        // Read message length
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // Client disconnected
                debug!("Client disconnected");
                break;
            }
            Err(e) => return Err(DaemonError::IoError(e)),
        }

        let msg_len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check message length (max 1MB)
        if msg_len > 1_048_576 {
            warn!("Message too large: {} bytes", msg_len);
            break;
        }

        // Read message
        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await?;

        // Parse request
        let request: Request = serde_json::from_slice(&msg_buf)?;

        // Handle request
        let response = handler.handle(request).await;

        // Serialize response
        let response_json = serde_json::to_vec(&response)?;

        // Send response
        let response_len = response_json.len() as u32;
        stream.write_all(&response_len.to_be_bytes()).await?;
        stream.write_all(&response_json).await?;
    }

    Ok(())
}

/// Check if a process is running.
fn is_process_running(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_daemon_config_new() {
        // This test only passes when HOME is set (which it should be in test environments)
        if let Ok(config) = DaemonConfig::new() {
            assert!(config.socket_path.to_string_lossy().contains("daemon.sock"));
            assert!(config.pid_file.to_string_lossy().contains("daemon.pid"));
            assert!(config.preload_secrets);
        }
        // If HOME is not set, the test passes (config creation correctly fails)
    }

    #[test]
    fn test_is_process_running() {
        // Current process should be running
        let pid = std::process::id();
        assert!(is_process_running(pid));

        // Non-existent process (high PID)
        assert!(!is_process_running(999999999));
    }

    #[tokio::test]
    async fn test_daemon_server_new() {
        let dir = tempdir().unwrap();
        let config = DaemonConfig {
            socket_path: dir.path().join("test.sock"),
            pid_file: dir.path().join("test.pid"),
            preload_secrets: false,
        };

        let _server = DaemonServer::new(config);
        // Just verify it can be created
    }
}
