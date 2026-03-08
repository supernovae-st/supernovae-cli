//! Dynamic stats collection for help screen.
//!
//! Provides fast (<100ms) stats collection for:
//! - Configured LLM/MCP providers (keychain checks)
//! - MCP servers (mcp.yaml parsing)
//! - Ollama models (if running)
//!
//! All checks are synchronous and use timeouts to ensure fast execution.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use spn_keyring::SpnKeyring;

use crate::mcp::{config_manager, McpScope};

/// Stats collected for the help screen display.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    /// Number of LLM providers with keys configured.
    pub llm_providers: usize,
    /// Number of MCP providers with keys configured.
    pub mcp_providers: usize,
    /// Number of MCP servers configured.
    pub mcp_servers: usize,
    /// Number of Ollama models (None if ollama not running or check skipped).
    pub ollama_models: Option<usize>,
    /// Whether ollama is running.
    pub ollama_running: bool,
    /// Total time taken to collect stats.
    pub collection_time: Duration,
}

impl Stats {
    /// Collect all stats with a timeout budget.
    ///
    /// Ensures collection completes in under 100ms by skipping slow checks.
    pub fn collect() -> Self {
        let start = Instant::now();
        let mut stats = Stats::default();

        // Check keychain providers (fast, <20ms typically)
        stats.collect_provider_stats();

        // Check MCP servers from config (fast, <10ms)
        stats.collect_mcp_stats();

        // Check Ollama (fast if running, skip if would be slow)
        // Only do this if we have time left in our budget
        if start.elapsed() < Duration::from_millis(80) {
            stats.collect_ollama_stats();
        }

        stats.collection_time = start.elapsed();
        stats
    }

    /// Collect provider stats from keychain.
    fn collect_provider_stats(&mut self) {
        // Use SpnKeyring::list() which checks all known providers
        let configured = SpnKeyring::list();

        // Categorize by provider type
        for provider_id in &configured {
            // LLM providers
            if matches!(
                provider_id.as_str(),
                "anthropic" | "openai" | "mistral" | "groq" | "deepseek" | "gemini" | "ollama"
            ) {
                self.llm_providers += 1;
            }
            // MCP providers
            else if matches!(
                provider_id.as_str(),
                "neo4j" | "github" | "slack" | "perplexity" | "firecrawl" | "supadata"
            ) {
                self.mcp_providers += 1;
            }
        }
    }

    /// Collect MCP server stats from mcp.yaml.
    fn collect_mcp_stats(&mut self) {
        let manager = config_manager();

        // Try to list servers from global scope
        if let Ok(servers) = manager.list_servers(McpScope::Global) {
            self.mcp_servers = servers.len();
        }

        // Also check project scope if available
        if let Ok(project_servers) = manager.list_servers(McpScope::Project) {
            // Project servers may override global, so just add unique ones
            // For simplicity, we just show the total from resolved config
            if !project_servers.is_empty() {
                // Use resolved config for accurate count
                if let Ok(all_servers) = manager.list_all_servers() {
                    self.mcp_servers = all_servers.len();
                }
            }
        }
    }

    /// Collect Ollama stats (quick check only).
    ///
    /// Uses a simple HTTP request over TCP with a short timeout to check
    /// if Ollama is running and count models.
    fn collect_ollama_stats(&mut self) {
        // Parse Ollama host from env or use default
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "127.0.0.1:11434".to_string());

        // Strip protocol if present
        let addr = host
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        // Try to connect with a short timeout (30ms)
        let stream = TcpStream::connect_timeout(
            &addr.parse().unwrap_or_else(|_| "127.0.0.1:11434".parse().unwrap()),
            Duration::from_millis(30),
        );

        let Ok(mut stream) = stream else {
            self.ollama_running = false;
            self.ollama_models = None;
            return;
        };

        // Set read/write timeouts
        let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(20)));

        // Send a simple HTTP GET request for /api/tags
        let request = format!(
            "GET /api/tags HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            addr
        );

        if stream.write_all(request.as_bytes()).is_err() {
            self.ollama_running = false;
            return;
        }

        // Read response (limit to 16KB to avoid slow reads)
        let mut buffer = vec![0u8; 16384];
        let bytes_read = match stream.read(&mut buffer) {
            Ok(n) => n,
            Err(_) => {
                self.ollama_running = false;
                return;
            }
        };

        let response = String::from_utf8_lossy(&buffer[..bytes_read]);

        // Check if we got a successful HTTP response
        if !response.starts_with("HTTP/1.1 200") && !response.starts_with("HTTP/1.0 200") {
            self.ollama_running = false;
            return;
        }

        self.ollama_running = true;

        // Try to extract model count from JSON body
        // Find the body (after \r\n\r\n)
        if let Some(body_start) = response.find("\r\n\r\n") {
            let body = &response[body_start + 4..];

            // Try to parse as JSON and count models
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                    self.ollama_models = Some(models.len());
                    return;
                }
            }
        }

        // Ollama is running but couldn't count models
        self.ollama_models = None;
    }

    /// Get total configured providers count.
    pub fn total_providers(&self) -> usize {
        self.llm_providers + self.mcp_providers
    }

    /// Format provider stats for display.
    pub fn providers_display(&self) -> String {
        if self.total_providers() == 0 {
            "run spn provider set".to_string()
        } else {
            format!("{} configured", self.total_providers())
        }
    }

    /// Format MCP servers stats for display.
    pub fn mcp_display(&self) -> String {
        if self.mcp_servers == 0 {
            "none configured".to_string()
        } else {
            format!("{} server{}", self.mcp_servers, if self.mcp_servers == 1 { "" } else { "s" })
        }
    }

    /// Format Ollama stats for display.
    pub fn ollama_display(&self) -> String {
        if !self.ollama_running {
            "not running".to_string()
        } else {
            match self.ollama_models {
                Some(0) => "running, no models".to_string(),
                Some(n) => format!("{} model{}", n, if n == 1 { "" } else { "s" }),
                None => "running".to_string(),
            }
        }
    }
}

/// Get the global MCP config path for display.
pub fn global_mcp_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".spn")
        .join("mcp.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_collect_does_not_panic() {
        // Stats collection should never panic, even if services are unavailable
        let stats = Stats::collect();
        // Just verify it completes
        assert!(stats.collection_time < Duration::from_secs(5));
    }

    #[test]
    fn test_stats_display_functions() {
        let stats = Stats::default();
        assert_eq!(stats.providers_display(), "run spn provider set");
        assert_eq!(stats.mcp_display(), "none configured");
        assert_eq!(stats.ollama_display(), "not running");
    }

    #[test]
    fn test_stats_with_providers() {
        let mut stats = Stats::default();
        stats.llm_providers = 2;
        stats.mcp_providers = 1;
        assert_eq!(stats.total_providers(), 3);
        assert_eq!(stats.providers_display(), "3 configured");
    }

    #[test]
    fn test_stats_with_mcp_servers() {
        let mut stats = Stats::default();
        stats.mcp_servers = 1;
        assert_eq!(stats.mcp_display(), "1 server");

        stats.mcp_servers = 3;
        assert_eq!(stats.mcp_display(), "3 servers");
    }

    #[test]
    fn test_stats_with_ollama() {
        let mut stats = Stats::default();

        // Not running
        assert_eq!(stats.ollama_display(), "not running");

        // Running but no models
        stats.ollama_running = true;
        stats.ollama_models = Some(0);
        assert_eq!(stats.ollama_display(), "running, no models");

        // Running with models
        stats.ollama_models = Some(1);
        assert_eq!(stats.ollama_display(), "1 model");

        stats.ollama_models = Some(5);
        assert_eq!(stats.ollama_display(), "5 models");
    }

    #[test]
    fn test_global_mcp_path() {
        let path = global_mcp_path();
        assert!(path.to_string_lossy().contains("mcp.yaml"));
    }
}
