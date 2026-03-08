//! Reasoning trace capture system.
//!
//! Captures and stores LLM reasoning traces for analysis,
//! debugging, and learning from past interactions.

#![allow(dead_code)]

mod store;
mod types;

pub use store::TraceStore;
pub use types::{ReasoningTrace, TraceId, TraceMetadata, TraceStep, TraceStepKind};
