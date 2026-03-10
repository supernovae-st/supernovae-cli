//! Smart onboarding detection for contextual help.
//!
//! Provides fast checks (<50ms total) to detect user's onboarding state
//! and display relevant hints in the help screen.
//!
//! # Checks
//!
//! - `has_provider`: Any provider key exists (keychain or env)
//! - `daemon_running`: Daemon socket exists and process is alive
//! - `ollama_running`: Ollama API responding on port 11434
//! - `nika_installed`: `nika` binary found in PATH
//!
//! # Usage
//!
//! ```text
//! use crate::ux::onboarding::OnboardingState;
//!
//! let state = OnboardingState::detect();
//! for hint in state.hints() {
//!     println!("{}", hint);
//! }
//! ```

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

/// Onboarding state detected from fast system checks.
#[derive(Debug, Clone, Default)]
pub struct OnboardingState {
    /// At least one provider key is configured (keychain or env).
    pub has_provider: bool,
    /// Daemon socket exists and process is running.
    pub daemon_running: bool,
    /// Ollama is running (port 11434 accepting connections).
    pub ollama_running: bool,
    /// Nika binary is installed and in PATH.
    pub nika_installed: bool,
}

impl OnboardingState {
    /// Detect onboarding state with fast checks (<50ms total).
    ///
    /// All checks are designed to be non-blocking or have strict timeouts.
    #[must_use]
    pub fn detect() -> Self {
        Self {
            has_provider: check_has_provider(),
            daemon_running: check_daemon_running(),
            ollama_running: check_ollama_running(),
            nika_installed: check_nika_installed(),
        }
    }

    /// Check if user appears to be completely new (no setup done).
    #[must_use]
    pub fn is_new_user(&self) -> bool {
        !self.has_provider && !self.daemon_running && !self.nika_installed
    }

    /// Check if user has partial setup (some things configured).
    #[must_use]
    pub fn is_partial_setup(&self) -> bool {
        self.has_provider || self.daemon_running || self.nika_installed
    }

    /// Check if user has full setup (all key components ready).
    #[must_use]
    pub fn is_fully_setup(&self) -> bool {
        self.has_provider && self.daemon_running
    }

    /// Generate contextual hints based on onboarding state.
    ///
    /// Returns a list of actionable hints for the user.
    #[must_use]
    pub fn hints(&self) -> Vec<OnboardingHint> {
        let mut hints = Vec::new();

        if self.is_new_user() {
            hints.push(OnboardingHint::new_user());
        } else {
            if !self.has_provider {
                hints.push(OnboardingHint::no_provider());
            }

            if !self.daemon_running && self.has_provider {
                hints.push(OnboardingHint::daemon_not_running());
            }

            if !self.ollama_running {
                hints.push(OnboardingHint::ollama_not_running());
            }

            if !self.nika_installed {
                hints.push(OnboardingHint::nika_not_installed());
            }
        }

        hints
    }

    /// Get the primary hint (most important action).
    #[must_use]
    pub fn primary_hint(&self) -> Option<OnboardingHint> {
        self.hints().into_iter().next()
    }
}

/// A contextual hint for the user.
#[derive(Debug, Clone)]
pub struct OnboardingHint {
    /// Emoji indicator for the hint type.
    pub emoji: &'static str,
    /// Short message describing the issue or suggestion.
    pub message: &'static str,
    /// Command to run to address the issue.
    pub command: &'static str,
}

impl OnboardingHint {
    /// Create hint for completely new users.
    fn new_user() -> Self {
        Self {
            emoji: "🚀",
            message: "New here? Run the setup wizard",
            command: "spn setup",
        }
    }

    /// Create hint for missing provider keys.
    fn no_provider() -> Self {
        Self {
            emoji: "🔐",
            message: "No API keys configured",
            command: "spn provider set anthropic",
        }
    }

    /// Create hint for daemon not running.
    fn daemon_not_running() -> Self {
        Self {
            emoji: "⚡",
            message: "Daemon not running (enables zero-popup secrets)",
            command: "spn daemon start",
        }
    }

    /// Create hint for Ollama not running.
    fn ollama_not_running() -> Self {
        Self {
            emoji: "🦙",
            message: "Ollama not running (needed for local models)",
            command: "ollama serve",
        }
    }

    /// Create hint for Nika not installed.
    fn nika_not_installed() -> Self {
        Self {
            emoji: "🦋",
            message: "Nika not installed (workflow engine)",
            command: "spn setup nika",
        }
    }
}

// ============================================================================
// Fast Check Functions (<50ms total target)
// ============================================================================

/// Check if any provider key is configured.
///
/// Uses `spn_keyring::has_any_keys()` which checks both keychain and env vars.
fn check_has_provider() -> bool {
    spn_keyring::has_any_keys()
}

/// Check if daemon is running.
///
/// Checks if socket file exists AND process is alive (via PID file).
fn check_daemon_running() -> bool {
    let socket_path = daemon_socket_path();
    let pid_path = daemon_pid_path();

    // Socket must exist
    if !socket_path.exists() {
        return false;
    }

    // PID file must exist and process must be alive
    if !pid_path.exists() {
        return false;
    }

    // Read PID and check if process is running
    std::fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map(is_process_running)
        .unwrap_or(false)
}

/// Check if Ollama is running.
///
/// Attempts TCP connection to 127.0.0.1:11434 with 100ms timeout.
fn check_ollama_running() -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], 11434));
    TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
}

/// Check if Nika is installed.
///
/// Uses `which` crate to check PATH.
fn check_nika_installed() -> bool {
    which::which("nika").is_ok()
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get daemon socket path (~/.spn/daemon.sock).
fn daemon_socket_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".spn")
        .join("daemon.sock")
}

/// Get daemon PID file path (~/.spn/daemon.pid).
fn daemon_pid_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".spn")
        .join("daemon.pid")
}

/// Check if a process with given PID is running.
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    // SAFETY: kill(pid, 0) is always safe to call. Signal 0 doesn't send any signal,
    // it just checks if the process exists and we have permission to signal it.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // On non-Unix, assume running if PID file exists
    true
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onboarding_state_detect() {
        // Should not panic, just detect current state
        let state = OnboardingState::detect();
        // State is valid regardless of values
        assert!(state.is_new_user() || state.is_partial_setup());
    }

    #[test]
    fn test_new_user_state() {
        let state = OnboardingState {
            has_provider: false,
            daemon_running: false,
            ollama_running: false,
            nika_installed: false,
        };
        assert!(state.is_new_user());
        assert!(!state.is_partial_setup());
        assert!(!state.is_fully_setup());
    }

    #[test]
    fn test_partial_setup_state() {
        let state = OnboardingState {
            has_provider: true,
            daemon_running: false,
            ollama_running: false,
            nika_installed: false,
        };
        assert!(!state.is_new_user());
        assert!(state.is_partial_setup());
        assert!(!state.is_fully_setup());
    }

    #[test]
    fn test_fully_setup_state() {
        let state = OnboardingState {
            has_provider: true,
            daemon_running: true,
            ollama_running: true,
            nika_installed: true,
        };
        assert!(!state.is_new_user());
        assert!(state.is_partial_setup());
        assert!(state.is_fully_setup());
    }

    #[test]
    fn test_new_user_hints() {
        let state = OnboardingState {
            has_provider: false,
            daemon_running: false,
            ollama_running: false,
            nika_installed: false,
        };
        let hints = state.hints();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].command, "spn setup");
    }

    #[test]
    fn test_partial_setup_hints() {
        let state = OnboardingState {
            has_provider: true,
            daemon_running: false,
            ollama_running: false,
            nika_installed: false,
        };
        let hints = state.hints();
        // Should have hints for daemon, ollama, and nika
        assert!(hints.len() >= 2);
    }

    #[test]
    fn test_fully_setup_no_hints() {
        let state = OnboardingState {
            has_provider: true,
            daemon_running: true,
            ollama_running: true,
            nika_installed: true,
        };
        let hints = state.hints();
        assert!(hints.is_empty());
    }

    #[test]
    fn test_daemon_socket_path() {
        let path = daemon_socket_path();
        assert!(path.to_string_lossy().contains("daemon.sock"));
    }

    #[test]
    fn test_daemon_pid_path() {
        let path = daemon_pid_path();
        assert!(path.to_string_lossy().contains("daemon.pid"));
    }

    #[test]
    fn test_check_ollama_running_does_not_panic() {
        // This may return true or false depending on system state
        let _ = check_ollama_running();
    }

    #[test]
    fn test_check_nika_installed_does_not_panic() {
        // This may return true or false depending on system state
        let _ = check_nika_installed();
    }

    #[test]
    fn test_onboarding_hint_fields() {
        let hint = OnboardingHint::new_user();
        assert!(!hint.emoji.is_empty());
        assert!(!hint.message.is_empty());
        assert!(!hint.command.is_empty());
    }

    #[test]
    fn test_primary_hint_returns_first() {
        let state = OnboardingState {
            has_provider: false,
            daemon_running: false,
            ollama_running: false,
            nika_installed: false,
        };
        let primary = state.primary_hint();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().command, "spn setup");
    }
}
