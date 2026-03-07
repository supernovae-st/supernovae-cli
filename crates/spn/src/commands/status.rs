//! Status command implementation.
//!
//! Shows the complete state of the SuperNovae ecosystem:
//! - Local models (Ollama)
//! - Credentials (LLM + MCP unified)
//! - MCP servers
//! - Daemon status

use crate::error::Result;
use crate::status::{render, SystemStatus};

/// Run the status command.
pub async fn run(json: bool) -> Result<()> {
    let status = SystemStatus::collect().await;

    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        render::render(&status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_status_runs() {
        // Just verify it doesn't panic
        let result = run(false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_status_json() {
        // Verify JSON output works
        let result = run(true).await;
        assert!(result.is_ok());
    }
}
