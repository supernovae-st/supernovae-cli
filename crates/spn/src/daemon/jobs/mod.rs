//! Background job scheduler for Nika workflows.
//!
//! Manages asynchronous workflow execution with:
//! - Job queue and prioritization
//! - Status tracking
//! - Cancellation support
//! - Persistent job history

mod scheduler;
mod store;
mod types;

pub use scheduler::JobScheduler;
pub use store::JobStore;
pub use types::{Job, JobId, JobState, JobStatus};
