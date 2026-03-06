//! Service management for daemon auto-start.
//!
//! Supports launchd (macOS) and systemd (Linux) for running the daemon
//! as a system service that starts automatically at login.
//!
//! # Example
//!
//! ```rust,ignore
//! use spn::daemon::service::ServiceManager;
//!
//! let manager = ServiceManager::detect()?;
//! manager.install()?;   // Install and start
//! manager.status()?;    // Check if running
//! manager.uninstall()?; // Stop and remove
//! ```

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

/// Service management errors.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Platform not supported for service management.
    #[error("Service management not supported on this platform")]
    UnsupportedPlatform,

    /// Service is already installed.
    #[error("Service is already installed")]
    AlreadyInstalled,

    /// Service is not installed.
    #[error("Service is not installed")]
    NotInstalled,

    /// Failed to find the spn binary.
    #[error("Cannot find spn binary: {0}")]
    BinaryNotFound(String),

    /// IO error during service operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Command execution failed.
    #[error("Command failed: {0}")]
    CommandFailed(String),
}

/// Result type for service operations.
pub type Result<T> = std::result::Result<T, ServiceError>;

/// Service status information.
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    /// Whether the service is installed.
    pub installed: bool,
    /// Whether the service is currently running.
    pub running: bool,
    /// Whether the service is enabled (starts at login).
    pub enabled: bool,
    /// Service manager type.
    pub manager: ServiceManagerType,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let installed = if self.installed { "yes" } else { "no" };
        let running = if self.running { "running" } else { "stopped" };
        let enabled = if self.enabled { "enabled" } else { "disabled" };

        write!(
            f,
            "Service: {:?}\nInstalled: {}\nStatus: {}\nAuto-start: {}",
            self.manager, installed, running, enabled
        )
    }
}

/// Type of service manager.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceManagerType {
    /// macOS launchd.
    Launchd,
    /// Linux systemd.
    Systemd,
    /// No supported service manager.
    None,
}

/// Service manager for daemon auto-start.
pub struct ServiceManager {
    manager_type: ServiceManagerType,
}

impl ServiceManager {
    /// Detect the available service manager.
    pub fn detect() -> Result<Self> {
        let manager_type = Self::detect_type();

        if manager_type == ServiceManagerType::None {
            return Err(ServiceError::UnsupportedPlatform);
        }

        Ok(Self { manager_type })
    }

    /// Detect the service manager type for the current platform.
    fn detect_type() -> ServiceManagerType {
        #[cfg(target_os = "macos")]
        {
            ServiceManagerType::Launchd
        }

        #[cfg(target_os = "linux")]
        {
            // Check if systemd is running
            if std::path::Path::new("/run/systemd/system").exists() {
                ServiceManagerType::Systemd
            } else {
                ServiceManagerType::None
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            ServiceManagerType::None
        }
    }

    /// Get the service manager type.
    pub fn manager_type(&self) -> ServiceManagerType {
        self.manager_type
    }

    /// Install the daemon as a system service.
    pub fn install(&self) -> Result<()> {
        match self.manager_type {
            ServiceManagerType::Launchd => self.install_launchd(),
            ServiceManagerType::Systemd => self.install_systemd(),
            ServiceManagerType::None => Err(ServiceError::UnsupportedPlatform),
        }
    }

    /// Uninstall the daemon service.
    pub fn uninstall(&self) -> Result<()> {
        match self.manager_type {
            ServiceManagerType::Launchd => self.uninstall_launchd(),
            ServiceManagerType::Systemd => self.uninstall_systemd(),
            ServiceManagerType::None => Err(ServiceError::UnsupportedPlatform),
        }
    }

    /// Get the current service status.
    pub fn status(&self) -> Result<ServiceStatus> {
        match self.manager_type {
            ServiceManagerType::Launchd => self.status_launchd(),
            ServiceManagerType::Systemd => self.status_systemd(),
            ServiceManagerType::None => Err(ServiceError::UnsupportedPlatform),
        }
    }

    // ========== launchd (macOS) ==========

    fn launchd_plist_path() -> PathBuf {
        let home = dirs::home_dir().expect("HOME not set");
        home.join("Library/LaunchAgents/com.supernovae.spn-daemon.plist")
    }

    fn install_launchd(&self) -> Result<()> {
        let plist_path = Self::launchd_plist_path();

        // Check if already installed
        if plist_path.exists() {
            return Err(ServiceError::AlreadyInstalled);
        }

        // Find spn binary
        let spn_bin = Self::find_spn_binary()?;
        let home = dirs::home_dir().expect("HOME not set");

        // Load template and replace placeholders
        let template = include_str!("../../../../assets/launchd/com.supernovae.spn-daemon.plist");
        let content = template
            .replace("${SPN_BIN}", &spn_bin)
            .replace("${HOME}", &home.display().to_string());

        // Ensure directory exists
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Ensure ~/.spn exists for logs
        let spn_dir = home.join(".spn");
        fs::create_dir_all(&spn_dir)?;

        // Write plist
        fs::write(&plist_path, content)?;

        // Load the service
        let status = Command::new("launchctl")
            .args(["load", "-w", &plist_path.display().to_string()])
            .status()?;

        if !status.success() {
            // Clean up on failure
            let _ = fs::remove_file(&plist_path);
            return Err(ServiceError::CommandFailed(
                "launchctl load failed".to_string(),
            ));
        }

        Ok(())
    }

    fn uninstall_launchd(&self) -> Result<()> {
        let plist_path = Self::launchd_plist_path();

        // Check if installed
        if !plist_path.exists() {
            return Err(ServiceError::NotInstalled);
        }

        // Unload the service
        let _ = Command::new("launchctl")
            .args(["unload", "-w", &plist_path.display().to_string()])
            .status();

        // Remove the plist file
        fs::remove_file(&plist_path)?;

        Ok(())
    }

    fn status_launchd(&self) -> Result<ServiceStatus> {
        let plist_path = Self::launchd_plist_path();
        let installed = plist_path.exists();

        // Check if running via launchctl list
        let output = Command::new("launchctl")
            .args(["list", "com.supernovae.spn-daemon"])
            .output()?;

        let running = output.status.success();

        Ok(ServiceStatus {
            installed,
            running,
            enabled: installed, // launchd: installed = enabled (RunAtLoad)
            manager: ServiceManagerType::Launchd,
        })
    }

    // ========== systemd (Linux) ==========

    fn systemd_unit_path() -> PathBuf {
        let home = dirs::home_dir().expect("HOME not set");
        home.join(".config/systemd/user/spn-daemon.service")
    }

    fn install_systemd(&self) -> Result<()> {
        let unit_path = Self::systemd_unit_path();

        // Check if already installed
        if unit_path.exists() {
            return Err(ServiceError::AlreadyInstalled);
        }

        // Find spn binary
        let spn_bin = Self::find_spn_binary()?;
        let home = dirs::home_dir().expect("HOME not set");

        // Load template and replace placeholders
        let template = include_str!("../../../../assets/systemd/spn-daemon.service");
        let content = template
            .replace("${SPN_BIN}", &spn_bin)
            .replace("%h", &home.display().to_string());

        // Ensure directory exists
        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Ensure ~/.spn exists for logs
        let spn_dir = home.join(".spn");
        fs::create_dir_all(&spn_dir)?;

        // Write unit file
        fs::write(&unit_path, content)?;

        // Reload systemd daemon
        let reload = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()?;

        if !reload.success() {
            let _ = fs::remove_file(&unit_path);
            return Err(ServiceError::CommandFailed(
                "systemctl daemon-reload failed".to_string(),
            ));
        }

        // Enable the service
        let enable = Command::new("systemctl")
            .args(["--user", "enable", "spn-daemon.service"])
            .status()?;

        if !enable.success() {
            let _ = fs::remove_file(&unit_path);
            return Err(ServiceError::CommandFailed(
                "systemctl enable failed".to_string(),
            ));
        }

        // Start the service
        let start = Command::new("systemctl")
            .args(["--user", "start", "spn-daemon.service"])
            .status()?;

        if !start.success() {
            return Err(ServiceError::CommandFailed(
                "systemctl start failed".to_string(),
            ));
        }

        Ok(())
    }

    fn uninstall_systemd(&self) -> Result<()> {
        let unit_path = Self::systemd_unit_path();

        // Check if installed
        if !unit_path.exists() {
            return Err(ServiceError::NotInstalled);
        }

        // Stop the service
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "spn-daemon.service"])
            .status();

        // Disable the service
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "spn-daemon.service"])
            .status();

        // Remove the unit file
        fs::remove_file(&unit_path)?;

        // Reload systemd daemon
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();

        Ok(())
    }

    fn status_systemd(&self) -> Result<ServiceStatus> {
        let unit_path = Self::systemd_unit_path();
        let installed = unit_path.exists();

        // Check if running
        let is_active = Command::new("systemctl")
            .args(["--user", "is-active", "spn-daemon.service"])
            .output()?;
        let running = is_active.status.success();

        // Check if enabled
        let is_enabled = Command::new("systemctl")
            .args(["--user", "is-enabled", "spn-daemon.service"])
            .output()?;
        let enabled = is_enabled.status.success();

        Ok(ServiceStatus {
            installed,
            running,
            enabled,
            manager: ServiceManagerType::Systemd,
        })
    }

    // ========== Utilities ==========

    /// Find the spn binary path.
    fn find_spn_binary() -> Result<String> {
        // First, try which
        if let Ok(output) = Command::new("which").arg("spn").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }

        // Try common locations
        let home = dirs::home_dir().expect("HOME not set");
        let candidates = [
            home.join(".cargo/bin/spn"),
            PathBuf::from("/usr/local/bin/spn"),
            PathBuf::from("/opt/homebrew/bin/spn"),
        ];

        for path in candidates {
            if path.exists() {
                return Ok(path.display().to_string());
            }
        }

        // Try current exe
        if let Ok(exe) = std::env::current_exe() {
            return Ok(exe.display().to_string());
        }

        Err(ServiceError::BinaryNotFound(
            "Cannot find spn binary. Make sure it's in PATH.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_manager_type() {
        let manager_type = ServiceManager::detect_type();

        #[cfg(target_os = "macos")]
        assert_eq!(manager_type, ServiceManagerType::Launchd);

        #[cfg(target_os = "linux")]
        {
            // Either Systemd or None depending on the system
            assert!(
                manager_type == ServiceManagerType::Systemd
                    || manager_type == ServiceManagerType::None
            );
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        assert_eq!(manager_type, ServiceManagerType::None);
    }

    #[test]
    fn test_launchd_plist_path() {
        let path = ServiceManager::launchd_plist_path();
        assert!(path.ends_with("Library/LaunchAgents/com.supernovae.spn-daemon.plist"));
    }

    #[test]
    fn test_systemd_unit_path() {
        let path = ServiceManager::systemd_unit_path();
        assert!(path.ends_with(".config/systemd/user/spn-daemon.service"));
    }

    #[test]
    fn test_service_status_display() {
        let status = ServiceStatus {
            installed: true,
            running: true,
            enabled: true,
            manager: ServiceManagerType::Launchd,
        };

        let display = status.to_string();
        assert!(display.contains("Launchd"));
        assert!(display.contains("yes"));
        assert!(display.contains("running"));
        assert!(display.contains("enabled"));
    }

    #[test]
    fn test_service_error_display() {
        let err = ServiceError::UnsupportedPlatform;
        assert!(err.to_string().contains("not supported"));

        let err = ServiceError::AlreadyInstalled;
        assert!(err.to_string().contains("already installed"));

        let err = ServiceError::NotInstalled;
        assert!(err.to_string().contains("not installed"));
    }
}
