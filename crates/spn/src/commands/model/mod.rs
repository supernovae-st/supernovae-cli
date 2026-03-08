//! Model CLI commands.
//!
//! Manage local LLM models via the spn daemon + Ollama.
//! Search and discover models from the SuperNovae registry.

mod handler;
pub mod run;

// Re-export the main entry point
pub use handler::run as execute;
