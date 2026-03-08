//! MCP Protocol support for spn daemon.
//!
//! Implements JSON-RPC 2.0 over stdio for MCP compatibility.
//! This allows Claude Code to use spn daemon as an MCP server.

mod protocol;
mod server;
mod tools;

pub use server::McpServer;
