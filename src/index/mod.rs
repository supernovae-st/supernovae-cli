//! Sparse index client for SuperNovae registry.
//!
//! This module provides:
//! - `types`: Index entry types (NDJSON format)
//! - `client`: HTTP/local client for fetching index entries
//! - `downloader`: Tarball download with integrity verification

pub mod client;
pub mod downloader;
pub mod types;

// Re-export types used by other modules
pub use client::IndexClient;
pub use downloader::{DownloadedPackage, Downloader};
