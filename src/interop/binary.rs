//! Binary discovery and execution.
//!
//! Handles finding and running nika/novanet binaries.

use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

use thiserror::Error;

/// Binary types that can be proxied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryType {
    /// Nika workflow engine.
    Nika,
    /// NovaNet knowledge graph CLI.
    NovaNet,
}

impl BinaryType {
    /// Get the binary name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Nika => "nika",
            Self::NovaNet => "novanet",
        }
    }

    /// Get the display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Nika => "Nika",
            Self::NovaNet => "NovaNet",
        }
    }
}

/// Errors that can occur when running binaries.
#[derive(Error, Debug)]
pub enum BinaryError {
    #[error("Binary not found: {0}. Install with: brew install supernovae-st/tap/{0}")]
    NotFound(String),

    #[error("Failed to execute {binary}: {message}")]
    ExecutionFailed { binary: String, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for binary operations.
pub type Result<T> = std::result::Result<T, BinaryError>;

/// Binary runner for executing external commands.
pub struct BinaryRunner {
    /// Type of binary to run.
    binary_type: BinaryType,

    /// Path to the binary (if found).
    binary_path: Option<PathBuf>,
}

impl BinaryRunner {
    /// Create a new binary runner.
    pub fn new(binary_type: BinaryType) -> Self {
        let binary_path = Self::find_binary(binary_type);
        Self {
            binary_type,
            binary_path,
        }
    }

    /// Find the binary in PATH or ~/.spn/bin/.
    fn find_binary(binary_type: BinaryType) -> Option<PathBuf> {
        let name = binary_type.name();

        // Check PATH first
        if let Ok(path) = which::which(name) {
            return Some(path);
        }

        // Check ~/.spn/bin/
        if let Some(home) = dirs::home_dir() {
            let spn_bin = home.join(".spn").join("bin").join(name);
            if spn_bin.exists() {
                return Some(spn_bin);
            }
        }

        None
    }

    /// Check if the binary is available.
    pub fn is_available(&self) -> bool {
        self.binary_path.is_some()
    }

    /// Get the binary path.
    pub fn path(&self) -> Option<&PathBuf> {
        self.binary_path.as_ref()
    }

    /// Run the binary with the given arguments.
    pub fn run(&self, args: &[&str]) -> Result<ExitStatus> {
        let path = self
            .binary_path
            .as_ref()
            .ok_or_else(|| BinaryError::NotFound(self.binary_type.name().to_string()))?;

        let status = Command::new(path)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Ok(status)
    }

    /// Run the binary with output capture.
    pub fn run_capture(&self, args: &[&str]) -> Result<std::process::Output> {
        let path = self
            .binary_path
            .as_ref()
            .ok_or_else(|| BinaryError::NotFound(self.binary_type.name().to_string()))?;

        let output = Command::new(path).args(args).output()?;

        Ok(output)
    }

    /// Check if the binary version matches requirements.
    pub fn check_version(&self, min_version: &str) -> Result<bool> {
        let output = self.run_capture(&["--version"])?;

        if !output.status.success() {
            return Ok(false);
        }

        let version_str = String::from_utf8_lossy(&output.stdout);

        // Parse version from output (e.g., "nika 0.8.0" or "novanet 0.14.0")
        if let Some(version) = version_str.split_whitespace().nth(1) {
            if let (Ok(current), Ok(required)) = (
                semver::Version::parse(version),
                semver::Version::parse(min_version),
            ) {
                return Ok(current >= required);
            }
        }

        Ok(false)
    }
}

/// Run nika with arguments.
pub fn run_nika(args: &[&str]) -> Result<ExitStatus> {
    BinaryRunner::new(BinaryType::Nika).run(args)
}

/// Run novanet with arguments.
pub fn run_novanet(args: &[&str]) -> Result<ExitStatus> {
    BinaryRunner::new(BinaryType::NovaNet).run(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_type_name() {
        assert_eq!(BinaryType::Nika.name(), "nika");
        assert_eq!(BinaryType::NovaNet.name(), "novanet");
    }

    #[test]
    fn test_binary_type_display_name() {
        assert_eq!(BinaryType::Nika.display_name(), "Nika");
        assert_eq!(BinaryType::NovaNet.display_name(), "NovaNet");
    }

    #[test]
    fn test_runner_creation() {
        let runner = BinaryRunner::new(BinaryType::Nika);
        // Binary may or may not be available depending on environment
        assert_eq!(runner.binary_type, BinaryType::Nika);
    }
}
