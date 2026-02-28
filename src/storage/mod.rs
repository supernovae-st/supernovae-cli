//! Local storage for installed SuperNovae packages.
//!
//! This module manages:
//! - Package installation to ~/.spn/packages/
//! - State tracking via state.json
//! - Cache management

pub mod local;

// Re-export main types
pub use local::{InstalledPackage, LocalStorage, StorageError, StorageState};
