//! MCP server module.
//!
//! Implements the MCP protocol handler with dynamic tool registration.

mod handler;

use crate::config::ApiConfig;
use crate::error::Result;

pub use handler::DynamicHandler;

/// Run the MCP server with the given API configurations.
pub async fn run(configs: Vec<ApiConfig>) -> Result<()> {
    tracing::info!("Starting MCP server...");

    // Build the handler with all configs
    let handler = DynamicHandler::new(configs).await?;

    // Log registered tools
    let tool_names = handler.tool_names();
    for name in &tool_names {
        tracing::debug!("Registered tool: {}", name);
    }
    tracing::info!("Registered {} tools", tool_names.len());

    // Run MCP server on stdio
    handler.run().await
}
