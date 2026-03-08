//! Backup system types for SuperNovae ecosystem.
//!
//! This module provides type definitions for the unified backup system.
//! Implementation lives in spn-cli, types are here for shared use.

mod manifest;
mod types;

pub use manifest::{
    BackupContents, BackupManifest, ComponentVersions, NikaContents, NovaNetContents, SpnContents,
};
pub use types::{BackupError, BackupInfo, RestoreInfo};
