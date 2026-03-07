//! Daemon status collector.
//!
//! Checks if the spn daemon is running and collects status info.

use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

/// Daemon status.
#[derive(Debug, Clone, Serialize)]
pub struct DaemonStatus {
    /// Whether daemon is running.
    pub running: bool,
    /// Process ID (if running).
    pub pid: Option<u32>,
    /// Socket path.
    pub socket_path: PathBuf,
    /// Uptime (if running).
    pub uptime: Option<Duration>,
}

/// Get the daemon socket path.
fn socket_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".spn")
        .join("daemon.sock")
}

/// Get the daemon PID file path.
fn pid_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".spn")
        .join("daemon.pid")
}

/// Collect daemon status.
pub async fn collect() -> DaemonStatus {
    let socket = socket_path();
    let pid_file = pid_path();

    // Check if socket exists
    let socket_exists = socket.exists();

    // Read PID from file
    let pid = if pid_file.exists() {
        std::fs::read_to_string(&pid_file)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
    } else {
        None
    };

    // Check if PID is actually running
    let pid_running = pid.map(is_process_running).unwrap_or(false);

    let running = socket_exists && pid_running;

    // Try to get uptime by checking PID file modification time
    let uptime = if running {
        pid_file
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|modified| {
                std::time::SystemTime::now()
                    .duration_since(modified)
                    .ok()
            })
    } else {
        None
    };

    DaemonStatus {
        running,
        pid: if running { pid } else { None },
        socket_path: socket,
        uptime,
    }
}

/// Check if a process with given PID is running.
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    use std::process::Command;
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // On non-Unix, just assume it's running if PID file exists
    true
}

impl DaemonStatus {
    /// Format uptime as human-readable string.
    pub fn uptime_display(&self) -> String {
        match self.uptime {
            Some(d) => {
                let secs = d.as_secs();
                if secs < 60 {
                    format!("{}s", secs)
                } else if secs < 3600 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else {
                    let hours = secs / 3600;
                    let mins = (secs % 3600) / 60;
                    format!("{}h {}m", hours, mins)
                }
            }
            None => "──".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_collect_returns_status() {
        let status = collect().await;
        // Socket path should always be set
        assert!(status.socket_path.to_string_lossy().contains("daemon.sock"));
    }

    #[test]
    fn test_uptime_display() {
        let status = DaemonStatus {
            running: true,
            pid: Some(123),
            socket_path: PathBuf::from("/tmp/test.sock"),
            uptime: Some(Duration::from_secs(3725)),
        };
        assert_eq!(status.uptime_display(), "1h 2m");
    }
}
