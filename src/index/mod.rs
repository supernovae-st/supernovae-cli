//! Sparse index client for SuperNovae registry.
//!
//! This module provides:
//! - `types`: Index entry types (NDJSON format)
//! - `client`: HTTP/local client for fetching index entries
//! - `downloader`: Tarball download with integrity verification

pub mod client;
pub mod downloader;
pub mod types;

// Re-export main types for convenience
pub use client::{IndexClient, IndexError, RegistryConfig};
pub use downloader::{DownloadError, DownloadedPackage, Downloader};
pub use types::{IndexDependency, IndexEntry, PackageScope};
