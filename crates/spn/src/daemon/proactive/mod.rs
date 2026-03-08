//! Proactive suggestion system.
//!
//! Analyzes context and proactively suggests relevant actions
//! based on current state, history, and patterns.

#![allow(dead_code)]

mod analyzer;
mod triggers;
mod types;

pub use analyzer::SuggestionAnalyzer;
pub use triggers::{ContextTrigger, TriggerCondition};
pub use types::{
    ProactiveSuggestion, SuggestionCategory, SuggestionId, SuggestionPriority, SuggestionSource,
};
