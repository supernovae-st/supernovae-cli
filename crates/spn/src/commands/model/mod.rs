//! Model CLI commands.
//!
//! Manage local LLM models via HuggingFace storage.
//! Search and discover models from the SuperNovae registry.
//!
//! NOTE: Inference commands (load, unload, run, status) were removed in v0.17.0.
//! Use Nika for model inference with native mistral.rs runtime.

mod handler;

// Re-export the main entry point
pub use handler::run as execute;
