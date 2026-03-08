//! Agent delegation system.
//!
//! Enables spawning and managing sub-agents for task delegation.
//! Each agent has a specialized role and can operate independently.

#![allow(dead_code)]

mod manager;
mod types;

pub use manager::AgentManager;
pub use types::{Agent, AgentConfig, AgentId, AgentRole, AgentState, AgentStatus, DelegatedTask};
