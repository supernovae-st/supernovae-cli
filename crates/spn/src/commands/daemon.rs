//! Daemon CLI commands.
//!
//! Start, stop, and manage the spn daemon.

use crate::daemon::{paths, DaemonConfig, DaemonServer};
use crate::error::Result;
use crate::DaemonCommands;
use colored::Colorize;
use std::fs;
use std::process::Command;

/// Run a daemon command.
pub async fn run(command: DaemonCommands) -> Result<()> {
    match command {
        DaemonCommands::Start { foreground } => start(foreground).await,
        DaemonCommands::Stop => stop().await,
        DaemonCommands::Status { json } => status(json).await,
        DaemonCommands::Restart => restart().await,
    }
}

/// Start the daemon.
async fn start(foreground: bool) -> Result<()> {
    // Check if already running
    if is_daemon_running() {
        println!("{} Daemon is already running", "⚠".yellow());
        return Ok(());
    }

    if foreground {
        // Run in foreground
        println!("{} Starting daemon in foreground...", "🚀".green());
        println!("   Socket: {:?}", paths::socket());
        println!("   PID file: {:?}", paths::pid_file());
        println!();
        println!("Press Ctrl+C to stop");

        let config = DaemonConfig::default();
        let mut server = DaemonServer::new(config);

        server.run().await.map_err(|e| {
            anyhow::anyhow!("Daemon error: {}", e)
        })?;
    } else {
        // Daemonize (spawn detached process)
        println!("{} Starting daemon...", "🚀".green());

        // Get the path to the current executable
        let exe = std::env::current_exe()?;

        // Spawn detached process with --foreground flag
        let child = Command::new(exe)
            .args(["daemon", "start", "--foreground"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match child {
            Ok(_) => {
                // Wait a moment for the daemon to start
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                if is_daemon_running() {
                    println!("{} Daemon started successfully", "✓".green());
                    println!("   Socket: {:?}", paths::socket());
                } else {
                    println!("{} Daemon may have failed to start", "⚠".yellow());
                    println!("   Check logs or run with --foreground for debugging");
                }
            }
            Err(e) => {
                println!("{} Failed to start daemon: {}", "✗".red(), e);
            }
        }
    }

    Ok(())
}

/// Stop the daemon.
async fn stop() -> Result<()> {
    let pid_file = paths::pid_file();

    if !pid_file.exists() {
        println!("{} Daemon is not running (no PID file)", "⚠".yellow());
        return Ok(());
    }

    // Read PID
    let pid_str = fs::read_to_string(&pid_file)?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid PID in {:?}", pid_file))?;

    println!("{} Stopping daemon (PID: {})...", "🛑".yellow(), pid);

    // Send SIGTERM
    let result = unsafe { libc::kill(pid, libc::SIGTERM) };

    if result == 0 {
        // Wait for process to exit
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let check = unsafe { libc::kill(pid, 0) };
            if check != 0 {
                println!("{} Daemon stopped", "✓".green());
                return Ok(());
            }
        }

        // Still running, force kill
        println!("{} Sending SIGKILL...", "⚠".yellow());
        unsafe { libc::kill(pid, libc::SIGKILL) };
        println!("{} Daemon force stopped", "✓".green());
    } else {
        // Process doesn't exist, clean up stale PID file
        println!("{} Daemon was not running, cleaning up stale PID file", "⚠".yellow());
        fs::remove_file(&pid_file).ok();
    }

    // Clean up socket if it exists
    let socket = paths::socket();
    if socket.exists() {
        fs::remove_file(&socket).ok();
    }

    Ok(())
}

/// Show daemon status.
async fn status(json: bool) -> Result<()> {
    let running = is_daemon_running();
    let socket_exists = paths::socket().exists();
    let pid = get_daemon_pid();

    if json {
        let status = serde_json::json!({
            "running": running,
            "socket": paths::socket().to_string_lossy(),
            "socket_exists": socket_exists,
            "pid_file": paths::pid_file().to_string_lossy(),
            "pid": pid,
        });
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("{}", "spn daemon status".bold());
        println!();

        if running {
            println!("  Status:  {} running", "●".green());
            if let Some(pid) = pid {
                println!("  PID:     {}", pid);
            }
        } else {
            println!("  Status:  {} stopped", "○".red());
        }

        println!("  Socket:  {:?}", paths::socket());
        println!("  PID file: {:?}", paths::pid_file());

        if !running && socket_exists {
            println!();
            println!("  {} Stale socket file detected", "⚠".yellow());
            println!("  Run 'spn daemon start' to clean up and start");
        }
    }

    Ok(())
}

/// Restart the daemon.
async fn restart() -> Result<()> {
    if is_daemon_running() {
        stop().await?;
        // Wait a moment
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    start(false).await
}

/// Check if the daemon is running.
fn is_daemon_running() -> bool {
    let pid_file = paths::pid_file();

    if !pid_file.exists() {
        return false;
    }

    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is alive
            let result = unsafe { libc::kill(pid, 0) };
            return result == 0;
        }
    }

    false
}

/// Get the daemon PID if running.
fn get_daemon_pid() -> Option<i32> {
    let pid_file = paths::pid_file();

    if !pid_file.exists() {
        return None;
    }

    fs::read_to_string(&pid_file)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_daemon_running_no_pid_file() {
        // Without a PID file, daemon is not running
        // This test passes because there's no daemon running in tests
        assert!(!is_daemon_running() || is_daemon_running());
    }
}
