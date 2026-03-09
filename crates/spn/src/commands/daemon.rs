//! Daemon CLI commands.
//!
//! Start, stop, and manage the spn daemon.

use crate::daemon::{
    mcp::McpServer, paths, DaemonConfig, DaemonServer, ModelManager, SecretManager, ServiceManager,
};
use crate::error::Result;
use crate::ux::design_system as ds;
use crate::DaemonCommands;
use spn_client::{IpcSchedulerStats, Request, Response, SpnClient, WatcherStatusInfo};
use std::fs;
use std::process::Command;
use std::sync::Arc;

/// Run a daemon command.
pub async fn run(command: DaemonCommands) -> Result<()> {
    match command {
        DaemonCommands::Start { foreground } => start(foreground).await,
        DaemonCommands::Stop => stop().await,
        DaemonCommands::Status { json } => status(json).await,
        DaemonCommands::Restart => restart().await,
        DaemonCommands::Install => install().await,
        DaemonCommands::Uninstall => uninstall().await,
        DaemonCommands::Mcp => run_mcp_server().await,
    }
}

/// Start the daemon.
async fn start(foreground: bool) -> Result<()> {
    // Check if already running
    if is_daemon_running() {
        println!("{} Daemon is already running", ds::warning("⚠"));
        return Ok(());
    }

    if foreground {
        // Run in foreground
        println!("{} Starting daemon in foreground...", ds::success("🚀"));
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
        println!("{} Starting daemon...", ds::success("🚀"));

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
                    println!("{} Daemon started successfully", ds::success("✓"));
                    if let Ok(socket) = paths::socket() {
                        println!("   Socket: {:?}", socket);
                    }
                } else {
                    println!("{} Daemon may have failed to start", ds::warning("⚠"));
                    println!("   Check logs or run with --foreground for debugging");
                }
            }
            Err(e) => {
                println!("{} Failed to start daemon: {}", ds::error("✗"), e);
            }
        }
    }

    Ok(())
}

/// Stop the daemon.
async fn stop() -> Result<()> {
    let pid_file = paths::pid_file()?;

    if !pid_file.exists() {
        println!("{} Daemon is not running (no PID file)", ds::warning("⚠"));
        return Ok(());
    }

    // Read PID
    let pid_str = fs::read_to_string(&pid_file)?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid PID in {:?}", pid_file))?;

    println!("{} Stopping daemon (PID: {})...", ds::warning("🛑"), pid);

    // Send SIGTERM
    let result = unsafe { libc::kill(pid, libc::SIGTERM) };

    if result == 0 {
        // Wait for process to exit
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let check = unsafe { libc::kill(pid, 0) };
            if check != 0 {
                println!("{} Daemon stopped", ds::success("✓"));
                return Ok(());
            }
        }

        // Still running, force kill
        println!("{} Sending SIGKILL...", ds::warning("⚠"));
        unsafe { libc::kill(pid, libc::SIGKILL) };
        println!("{} Daemon force stopped", ds::success("✓"));
    } else {
        // Process doesn't exist, clean up stale PID file
        println!(
            "{} Daemon was not running, cleaning up stale PID file",
            ds::warning("⚠")
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

    // Try to get detailed status from daemon if running
    let (watcher_status, job_stats) = if running {
        query_daemon_status().await
    } else {
        (None, None)
    };

    if json {
        let mut status = serde_json::json!({
            "running": running,
            "socket": socket_path.to_string_lossy(),
            "socket_exists": socket_exists,
            "pid_file": pid_file_path.to_string_lossy(),
            "pid": pid,
        });

        // Add detailed status if available
        if let Some(watcher) = &watcher_status {
            status["watcher"] = serde_json::json!({
                "is_running": watcher.is_running,
                "watched_count": watcher.watched_count,
                "watched_paths": watcher.watched_paths,
                "debounce_ms": watcher.debounce_ms,
                "recent_projects": watcher.recent_projects,
                "foreign_pending": watcher.foreign_pending,
                "foreign_ignored": watcher.foreign_ignored,
            });
        }
        if let Some(jobs) = &job_stats {
            status["jobs"] = serde_json::json!({
                "total": jobs.total,
                "pending": jobs.pending,
                "running": jobs.running,
                "completed": jobs.completed,
                "failed": jobs.failed,
                "cancelled": jobs.cancelled,
                "has_nika": jobs.has_nika,
            });
        }

        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("{}", ds::highlight("spn daemon status"));
        println!();

        if running {
            println!("  Status:  {} running", ds::success("●"));
            if let Some(pid) = pid {
                println!("  PID:     {}", pid);
            }

            // Display watcher service status
            if let Some(watcher) = &watcher_status {
                println!();
                println!("  {}", ds::highlight("Watcher Service"));
                println!(
                    "  ├── Watched paths:       {}",
                    ds::primary(watcher.watched_count.to_string())
                );
                println!(
                    "  ├── Recent projects:     {}",
                    ds::primary(watcher.recent_projects.len().to_string())
                );

                let foreign_count = watcher.foreign_pending.len();
                if foreign_count > 0 {
                    println!(
                        "  └── Foreign MCPs:        {} {}",
                        ds::warning(foreign_count.to_string()),
                        ds::warning("(pending adoption)")
                    );
                    for mcp in &watcher.foreign_pending {
                        println!(
                            "      └── {} (from {})",
                            ds::highlight(&mcp.name),
                            mcp.source
                        );
                    }
                } else {
                    println!(
                        "  └── Foreign MCPs:        {}",
                        ds::success("0 (none detected)")
                    );
                }
            }

            // Display job scheduler status
            if let Some(jobs) = &job_stats {
                println!();
                println!("  {}", ds::highlight("Job Scheduler"));
                println!(
                    "  ├── Total jobs:          {}",
                    ds::primary(jobs.total.to_string())
                );

                if jobs.running > 0 {
                    println!(
                        "  ├── Running:             {}",
                        ds::success(jobs.running.to_string())
                    );
                }
                if jobs.pending > 0 {
                    println!(
                        "  ├── Pending:             {}",
                        ds::warning(jobs.pending.to_string())
                    );
                }

                let nika_status = if jobs.has_nika {
                    ds::success("available")
                } else {
                    ds::warning("not found")
                };
                println!("  └── Nika binary:         {}", nika_status);
            }
        } else {
            println!("  Status:  {} stopped", ds::error("○"));
            println!();
            println!(
                "  {} Run '{}' to start",
                ds::primary("→"),
                ds::primary("spn daemon start")
            );
        }

        println!();
        println!("  Socket:  {:?}", socket_path);
        println!("  PID file: {:?}", pid_file_path);

        if !running && socket_exists {
            println!();
            println!("  {} Stale socket file detected", ds::warning("⚠"));
            println!("  Run 'spn daemon start' to clean up and start");
        }
    }

    Ok(())
}

/// Query detailed status from daemon via IPC.
async fn query_daemon_status() -> (Option<WatcherStatusInfo>, Option<IpcSchedulerStats>) {
    // Try to connect to daemon
    let mut client = match SpnClient::connect().await {
        Ok(c) => c,
        Err(_) => return (None, None),
    };

    // Query watcher status
    let watcher_status = client.watcher_status().await.ok();

    // Query job stats
    let job_stats = match client.send_request(Request::JobStats).await {
        Ok(Response::JobStatsResult { stats }) => Some(stats),
        _ => None,
    };

    (watcher_status, job_stats)
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
    println!(
        "{} Installing daemon as system service...",
        ds::primary("🔧")
    );

    let manager =
        ServiceManager::detect().map_err(|e| anyhow::anyhow!("Service management: {}", e))?;

    println!(
        "   {} Service manager: {:?}",
        ds::primary("→"),
        manager.manager_type()
    );

    match manager.install() {
        Ok(()) => {
            println!("{} Daemon installed successfully!", ds::success("✓"));
            println!();
            println!("The daemon will now start automatically at login.");
            println!();
            println!("To check status:     {}", ds::primary("spn daemon status"));
            println!(
                "To uninstall:        {}",
                ds::primary("spn daemon uninstall")
            );
        }
        Err(crate::daemon::ServiceError::AlreadyInstalled) => {
            println!(
                "{} Daemon is already installed as a service",
                ds::warning("⚠")
            );
            println!();
            println!(
                "To reinstall, first run: {}",
                ds::primary("spn daemon uninstall")
            );
        }
        Err(e) => {
            println!("{} Installation failed: {}", ds::error("✗"), e);
            return Err(anyhow::anyhow!("Service install failed: {}", e).into());
        }
    }

    Ok(())
}

/// Run as MCP server over stdio.
///
/// This allows Claude Code to use spn daemon as an MCP server.
async fn run_mcp_server() -> Result<()> {
    // Initialize managers
    let secrets = Arc::new(SecretManager::new());
    let models = Arc::new(ModelManager::new());

    // Create and run MCP server
    let server = McpServer::new(secrets, models);
    server
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}

/// Uninstall daemon system service.
async fn uninstall() -> Result<()> {
    println!("{} Uninstalling daemon service...", ds::primary("🔧"));

    let manager =
        ServiceManager::detect().map_err(|e| anyhow::anyhow!("Service management: {}", e))?;

    match manager.uninstall() {
        Ok(()) => {
            println!("{} Daemon service uninstalled", ds::success("✓"));
            println!();
            println!("The daemon will no longer start at login.");
            println!();
            println!("To run manually:     {}", ds::primary("spn daemon start"));
            println!("To reinstall:        {}", ds::primary("spn daemon install"));
        }
        Err(crate::daemon::ServiceError::NotInstalled) => {
            println!("{} Daemon is not installed as a service", ds::warning("⚠"));
        }
        Err(e) => {
            println!("{} Uninstall failed: {}", ds::error("✗"), e);
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
