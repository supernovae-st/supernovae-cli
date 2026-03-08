//! Interactive TUI browser command.
//!
//! Launches a terminal interface for exploring spn resources:
//! - Models (Ollama)
//! - Providers (API keys)
//! - MCP Servers
//! - Skills

use crate::error::Result;
use crate::tui::ExploreApp;

/// Run the explore TUI.
pub async fn run() -> Result<()> {
    let mut app = ExploreApp::new().await;
    app.run().await?;
    Ok(())
}
