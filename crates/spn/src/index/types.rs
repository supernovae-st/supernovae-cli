//! Sparse index types for SuperNovae registry.
//!
//! The sparse index uses a Cargo-style NDJSON format for package metadata.
//!
//! TODO(v0.14): Integrate advanced package type methods

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single version entry in the sparse index.
///
/// Each line in the index file is one of these entries (NDJSON format).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexEntry {
    /// Full package name (e.g., "@workflows/dev-productivity/code-review").
    pub name: String,

    /// Version string (semver).
    #[serde(rename = "vers")]
    pub version: String,

    /// Direct dependencies (name → version constraint).
    #[serde(default)]
    pub deps: Vec<IndexDependency>,

    /// SHA256 checksum of the tarball.
    pub cksum: String,

    /// Optional features map.
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,

    /// Whether this version is yanked.
    #[serde(default)]
    pub yanked: bool,

    /// Optional links (for build scripts, rarely used).
    #[serde(default)]
    pub links: Option<String>,
}

/// A dependency entry in the index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexDependency {
    /// Dependency name.
    pub name: String,

    /// Version requirement (e.g., "^0.1", ">=1.0").
    pub req: String,

    /// Optional features to enable.
    #[serde(default)]
    pub features: Vec<String>,

    /// Whether this is an optional dependency.
    #[serde(default)]
    pub optional: bool,

    /// Whether this is a dev dependency.
    #[serde(default)]
    pub dev: bool,

    /// Default features enabled.
    #[serde(default = "default_true")]
    pub default_features: bool,

    /// Registry URL (for dependencies from other registries).
    #[serde(default)]
    pub registry: Option<String>,
}

fn default_true() -> bool {
    true
}

impl IndexEntry {
    /// Create a simple index entry without dependencies.
    pub fn new(name: &str, version: &str, checksum: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            deps: Vec::new(),
            cksum: checksum.to_string(),
            features: HashMap::new(),
            yanked: false,
            links: None,
        }
    }

    /// Parse the version as semver.
    pub fn semver(&self) -> Result<semver::Version, semver::Error> {
        semver::Version::parse(&self.version)
    }

    /// Check if this entry is usable (not yanked).
    pub fn is_available(&self) -> bool {
        !self.yanked
    }
}

impl IndexDependency {
    /// Create a simple dependency.
    pub fn new(name: &str, req: &str) -> Self {
        Self {
            name: name.to_string(),
            req: req.to_string(),
            features: Vec::new(),
            optional: false,
            dev: false,
            default_features: true,
            registry: None,
        }
    }
}

/// Package scope extracted from the name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageScope {
    /// Scope without @ (e.g., "workflows", "nika", "community").
    pub scope: String,

    /// Package path after scope (e.g., "dev-productivity/code-review").
    pub path: String,
}

impl PackageScope {
    /// Parse a package name into scope and path.
    ///
    /// # Examples
    /// ```
    /// # use spn::index::types::PackageScope;
    /// let pkg = PackageScope::parse("@workflows/dev-productivity/code-review").unwrap();
    /// assert_eq!(pkg.scope, "workflows");
    /// assert_eq!(pkg.path, "dev-productivity/code-review");
    /// ```
    pub fn parse(name: &str) -> Option<Self> {
        if !name.starts_with('@') {
            return None;
        }

        let without_at = &name[1..];
        let mut parts = without_at.splitn(2, '/');

        let scope = parts.next()?.to_string();
        let path = parts.next()?.to_string();

        Some(Self { scope, path })
    }

    /// Get the index path for this package.
    ///
    /// Maps to: `index/@{scope_prefix}/{path}`
    /// Where scope_prefix is: w (workflows), n (nika), c (community)
    pub fn index_path(&self) -> String {
        let prefix = match self.scope.as_str() {
            "workflows" => "w",
            "nika" => "n",
            "community" => "c",
            other => other,
        };

        format!("@{}/{}", prefix, self.path)
    }

    /// Full package name with @.
    pub fn full_name(&self) -> String {
        format!("@{}/{}", self.scope, self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_index_entry() {
        let json = r#"{"name":"@workflows/data/json-transformer","vers":"1.0.0","deps":[],"cksum":"sha256:abc123","features":{},"yanked":false}"#;
        let entry: IndexEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.name, "@workflows/data/json-transformer");
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.cksum, "sha256:abc123");
        assert!(!entry.yanked);
    }

    #[test]
    fn test_parse_entry_with_deps() {
        let json = r#"{"name":"@nika/seo-audit","vers":"0.1.0","deps":[{"name":"@community/mock-mcp","req":"^0.1"}],"cksum":"sha256:def456","features":{},"yanked":false}"#;
        let entry: IndexEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.deps.len(), 1);
        assert_eq!(entry.deps[0].name, "@community/mock-mcp");
        assert_eq!(entry.deps[0].req, "^0.1");
    }

    #[test]
    fn test_package_scope_parse() {
        let scope = PackageScope::parse("@workflows/dev-productivity/code-review").unwrap();
        assert_eq!(scope.scope, "workflows");
        assert_eq!(scope.path, "dev-productivity/code-review");
    }

    #[test]
    fn test_package_scope_index_path() {
        let scope = PackageScope::parse("@workflows/data/json-transformer").unwrap();
        assert_eq!(scope.index_path(), "@w/data/json-transformer");

        let scope = PackageScope::parse("@nika/seo-audit").unwrap();
        assert_eq!(scope.index_path(), "@n/seo-audit");
    }

    #[test]
    fn test_invalid_package_name() {
        assert!(PackageScope::parse("no-at-sign").is_none());
        assert!(PackageScope::parse("@").is_none());
    }

    #[test]
    fn test_index_entry_yanked() {
        let mut entry = IndexEntry::new("@test/pkg", "1.0.0", "sha256:test");
        assert!(entry.is_available());

        entry.yanked = true;
        assert!(!entry.is_available());
    }
}
