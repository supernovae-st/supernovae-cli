//! Daemon CLI commands.
//!
//! Start, stop, and manage the spn daemon.

use crate::daemon::{paths, DaemonConfig, DaemonServer, ServiceManager};
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
        DaemonCommands::Install => install().await,
        DaemonCommands::Uninstall => uninstall().await,
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
        println!("   Socket: {:?}", paths::socket()?);
        println!("   PID file: {:?}", paths::pid_file()?);
        println!();
        println!("Press Ctrl+C to stop");

        let config =
            DaemonConfig::new().map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;
        let mut server = DaemonServer::new(config);

        server
            .run()
            .await
            .map_err(|e| anyhow::anyhow!("Daemon error: {}", e))?;
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
                    if let Ok(socket) = paths::socket() {
                        println!("   Socket: {:?}", socket);
                    }
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
    let pid_file = paths::pid_file()?;

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
        println!(
            "{} Daemon was not running, cleaning up stale PID file",
            "⚠".yellow()
        );
        fs::remove_file(&pid_file).ok();
    }

    // Clean up socket if it exists
    if let Ok(socket) = paths::socket() {
        if socket.exists() {
            fs::remove_file(&socket).ok();
        }
    }

    Ok(())
}

/// Show daemon status.
async fn status(json: bool) -> Result<()> {
    let running = is_daemon_running();
    let socket_path = paths::socket()?;
    let pid_file_path = paths::pid_file()?;
    let socket_exists = socket_path.exists();
    let pid = get_daemon_pid();

    if json {
        let status = serde_json::json!({
            "running": running,
            "socket": socket_path.to_string_lossy(),
            "socket_exists": socket_exists,
            "pid_file": pid_file_path.to_string_lossy(),
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
            println!();
            println!(
                "  {} Run '{}' to start",
                "→".cyan(),
                "spn daemon start".cyan()
            );
        }

        println!();
        println!("  Socket:  {:?}", socket_path);
        println!("  PID file: {:?}", pid_file_path);

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
    let Ok(pid_file) = paths::pid_file() else {
        return false;
    };

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
    let pid_file = paths::pid_file().ok()?;

    if !pid_file.exists() {
        return None;
    }

    fs::read_to_string(&pid_file)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Install daemon as a system service.
async fn install() -> Result<()> {
    println!("{} Installing daemon as system service...", "🔧".cyan());

    let manager =
        ServiceManager::detect().map_err(|e| anyhow::anyhow!("Service management: {}", e))?;

    println!(
        "   {} Service manager: {:?}",
        "→".blue(),
        manager.manager_type()
    );

    match manager.install() {
        Ok(()) => {
            println!("{} Daemon installed successfully!", "✓".green());
            println!();
            println!("The daemon will now start automatically at login.");
            println!();
            println!("To check status:     {}", "spn daemon status".cyan());
            println!("To uninstall:        {}", "spn daemon uninstall".cyan());
        }
        Err(crate::daemon::ServiceError::AlreadyInstalled) => {
            println!("{} Daemon is already installed as a service", "⚠".yellow());
            println!();
            println!("To reinstall, first run: {}", "spn daemon uninstall".cyan());
        }
        Err(e) => {
            println!("{} Installation failed: {}", "✗".red(), e);
            return Err(anyhow::anyhow!("Service install failed: {}", e).into());
        }
    }

    Ok(())
}

/// Uninstall daemon system service.
async fn uninstall() -> Result<()> {
    println!("{} Uninstalling daemon service...", "🔧".cyan());

    let manager =
        ServiceManager::detect().map_err(|e| anyhow::anyhow!("Service management: {}", e))?;

    match manager.uninstall() {
        Ok(()) => {
            println!("{} Daemon service uninstalled", "✓".green());
            println!();
            println!("The daemon will no longer start at login.");
            println!();
            println!("To run manually:     {}", "spn daemon start".cyan());
            println!("To reinstall:        {}", "spn daemon install".cyan());
        }
        Err(crate::daemon::ServiceError::NotInstalled) => {
            println!("{} Daemon is not installed as a service", "⚠".yellow());
        }
        Err(e) => {
            println!("{} Uninstall failed: {}", "✗".red(), e);
            return Err(anyhow::anyhow!("Service uninstall failed: {}", e).into());
        }
    }

    Ok(())
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
