//! Native notifications for foreign MCP detection.
//!
//! Sends desktop notifications when foreign MCPs are detected in client configs.
//! Uses `notify-rust` for cross-platform desktop notifications.

use notify_rust::Notification;
use tracing::{debug, warn};

use super::foreign::ForeignSource;

/// Notification service for MCP-related events.
#[derive(Debug, Clone)]
pub struct NotificationService {
    /// Whether native notifications are enabled.
    enabled: bool,
    /// Whether logging is enabled.
    log_enabled: bool,
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationService {
    /// Create a new notification service with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: true,
            log_enabled: true,
        }
    }

    /// Create a disabled notification service (for testing).
    #[must_use]
    #[allow(dead_code)] // Reserved for tests
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            log_enabled: false,
        }
    }

    /// Enable or disable native notifications.
    #[allow(dead_code)] // Reserved for daemon config API
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Enable or disable logging.
    #[allow(dead_code)] // Reserved for daemon config API
    pub fn set_log_enabled(&mut self, enabled: bool) {
        self.log_enabled = enabled;
    }

    /// Send a notification about a foreign MCP detection.
    pub fn notify_foreign_mcp(&self, name: &str, source: ForeignSource) {
        // Always log to daemon stderr
        if self.log_enabled {
            self.log_foreign_mcp(name, source);
        }

        // Send native notification if enabled
        if self.enabled {
            if let Err(e) = self.send_native_notification(name, source) {
                warn!("Failed to send native notification: {e}");
            }
        }
    }

    /// Send a native desktop notification.
    fn send_native_notification(&self, name: &str, source: ForeignSource) -> Result<(), notify_rust::error::Error> {
        Notification::new()
            .summary("spn: Foreign MCP detected")
            .body(&format!("'{}' found in {}", name, source))
            .appname("spn")
            .timeout(5000) // 5 seconds
            .show()?;

        debug!("Sent native notification for foreign MCP: {name}");
        Ok(())
    }

    /// Log foreign MCP detection to stderr/tracing.
    fn log_foreign_mcp(&self, name: &str, source: ForeignSource) {
        // Use eprintln for daemon stderr visibility
        eprintln!(
            "[WATCH] Foreign MCP detected: {} (source: {})",
            name, source
        );

        // Also log via tracing for structured logs
        tracing::info!(
            mcp_name = %name,
            source = %source,
            "Foreign MCP detected"
        );
    }

    /// Notify that multiple foreign MCPs were detected.
    #[allow(dead_code)] // Reserved for batch notification API
    pub fn notify_foreign_batch(&self, count: usize, sources: &[ForeignSource]) {
        if count == 0 {
            return;
        }

        let source_str = if sources.len() == 1 {
            sources[0].to_string()
        } else {
            format!("{} clients", sources.len())
        };

        if self.log_enabled {
            eprintln!("[WATCH] {} foreign MCP(s) detected in {}", count, source_str);
        }

        if self.enabled {
            let body = if count == 1 {
                format!("1 new MCP found in {}", source_str)
            } else {
                format!("{} new MCPs found in {}", count, source_str)
            };

            let _ = Notification::new()
                .summary("spn: Foreign MCPs detected")
                .body(&body)
                .appname("spn")
                .timeout(5000)
                .show();
        }
    }

    /// Notify about MCP sync completion.
    #[allow(dead_code)] // Reserved for sync notification API
    pub fn notify_sync_complete(&self, synced_count: usize) {
        if !self.log_enabled && !self.enabled {
            return;
        }

        if self.log_enabled {
            eprintln!("[SYNC] {} MCP(s) synced to clients", synced_count);
        }

        // Only send native notification for significant sync events
        if self.enabled && synced_count > 0 {
            let _ = Notification::new()
                .summary("spn: MCPs synced")
                .body(&format!("{} server(s) synced to all clients", synced_count))
                .appname("spn")
                .timeout(3000)
                .show();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_service_does_nothing() {
        let service = NotificationService::disabled();
        // Should not panic or do anything visible
        service.notify_foreign_mcp("test", ForeignSource::Cursor);
    }

    #[test]
    fn test_new_service_is_enabled() {
        let service = NotificationService::new();
        assert!(service.enabled);
        assert!(service.log_enabled);
    }

    #[test]
    fn test_set_enabled() {
        let mut service = NotificationService::new();
        service.set_enabled(false);
        assert!(!service.enabled);
    }
}
