//! CLI command implementations.
//!
//! Many command handlers are marked `async` for API uniformity even when they
//! don't currently contain await points. This allows future async operations
//! without breaking changes.
#![allow(clippy::unused_async)]

pub mod add;
pub mod completion;
pub mod config;
pub mod daemon;
pub mod doctor;
pub mod explore;
pub mod help;
pub mod suggest;
pub mod info;
pub mod init;
pub mod jobs;
pub mod install;
pub mod list;
pub mod mcp;
pub mod model;
pub mod nk;
pub mod nv;
pub mod outdated;
pub mod provider;
pub mod publish;
pub mod remove;
pub mod schema;
pub mod search;
pub mod secrets;
pub mod setup;
pub mod skill;
pub mod status;
pub mod sync;
pub mod update;
pub mod version;
