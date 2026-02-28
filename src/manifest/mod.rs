//! Manifest parsing and lockfile generation for SuperNovae packages.
//!
//! This module provides:
//! - `spn_yaml`: Parser for spn.yaml manifest files
//! - `lockfile`: Generator for spn.lock lockfiles (TOML format)

pub mod lockfile;
pub mod spn_yaml;

// Re-export main types for convenience
pub use lockfile::{
    DEFAULT_REGISTRY, LOCKFILE_VERSION, LockfileError, ResolvedPackage, SpnLockfile,
};
pub use spn_yaml::{Dependency, DetailedDependency, ManifestError, SpnManifest, VersionOp};
