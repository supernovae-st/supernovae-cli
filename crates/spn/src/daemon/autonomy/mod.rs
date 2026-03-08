//! Autonomous orchestration system.
//!
//! Coordinates agents, jobs, memory, traces, and proactive suggestions
//! to enable autonomous operation with human oversight.

#![allow(dead_code)]
#![allow(unused_imports)]

mod orchestrator;
mod policy;
mod types;

pub use orchestrator::{AutonomyOrchestrator, OrchestratorConfig, OrchestratorStats};
pub use policy::{ApprovalLevel, AutonomyLevel, AutonomyPolicy, PolicyViolation};
pub use types::{
    AutonomousTask, Decision, DecisionOutcome, OrchestratorState, TaskResult, TaskSource,
    TaskStatus,
};
