//! spn.yaml manifest parser.
//!
//! The manifest file defines project dependencies for the SuperNovae package ecosystem.
//!
//! # Example
//!
//! ```yaml
//! name: my-project
//! version: 1.0.0
//!
//! dependencies:
//!   "@nika/seo-audit": "^0.1"
//!   "@workflows/dev-productivity/code-review": "1.0.0"
//!
//! dev-dependencies:
//!   "@community/mock-mcp": "^0.1"
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when working with manifests.
#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Failed to read manifest file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse manifest YAML: {0}")]
    ParseError(#[from] serde_yaml::Error),

    #[error("Invalid version constraint: {0}")]
    InvalidVersionConstraint(String),

    #[error("Manifest not found at: {0}")]
    NotFound(String),
}

/// A dependency entry with version constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Dependency {
    /// Simple version string: "@nika/seo-audit": "^0.1"
    Simple(String),
    /// Detailed dependency with features
    Detailed(DetailedDependency),
}

/// Detailed dependency specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetailedDependency {
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub optional: bool,
}

impl Dependency {
    /// Get the version constraint string.
    pub fn version(&self) -> &str {
        match self {
            Dependency::Simple(v) => v,
            Dependency::Detailed(d) => &d.version,
        }
    }

    /// Parse the version constraint and return (operator, version).
    /// Supports: ^, ~, >=, >, <=, <, = (or exact match)
    pub fn parse_constraint(&self) -> Result<(VersionOp, semver::Version), ManifestError> {
        let version_str = self.version();

        let (op, version_part) = if let Some(v) = version_str.strip_prefix('^') {
            (VersionOp::Caret, v)
        } else if let Some(v) = version_str.strip_prefix('~') {
            (VersionOp::Tilde, v)
        } else if let Some(v) = version_str.strip_prefix(">=") {
            (VersionOp::Gte, v)
        } else if let Some(v) = version_str.strip_prefix('>') {
            (VersionOp::Gt, v)
        } else if let Some(v) = version_str.strip_prefix("<=") {
            (VersionOp::Lte, v)
        } else if let Some(v) = version_str.strip_prefix('<') {
            (VersionOp::Lt, v)
        } else if let Some(v) = version_str.strip_prefix('=') {
            (VersionOp::Exact, v)
        } else {
            (VersionOp::Exact, version_str)
        };

        // Handle partial versions like "0.1" -> "0.1.0"
        let normalized = normalize_version(version_part);
        let version = semver::Version::parse(&normalized).map_err(|e| {
            ManifestError::InvalidVersionConstraint(format!("{}: {}", version_str, e))
        })?;

        Ok((op, version))
    }
}

/// Version comparison operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VersionOp {
    /// ^ - Compatible with (same major for 1.x, same major.minor for 0.x)
    Caret,
    /// ~ - Approximately (same major.minor)
    Tilde,
    /// >= - Greater than or equal
    Gte,
    /// > - Greater than
    Gt,
    /// <= - Less than or equal
    Lte,
    /// < - Less than
    Lt,
    /// = or exact match
    Exact,
}

impl VersionOp {
    /// Check if a version satisfies this constraint.
    pub fn satisfies(&self, constraint: &semver::Version, candidate: &semver::Version) -> bool {
        match self {
            VersionOp::Exact => candidate == constraint,
            VersionOp::Gte => candidate >= constraint,
            VersionOp::Gt => candidate > constraint,
            VersionOp::Lte => candidate <= constraint,
            VersionOp::Lt => candidate < constraint,
            VersionOp::Tilde => {
                candidate.major == constraint.major
                    && candidate.minor == constraint.minor
                    && candidate.patch >= constraint.patch
            }
            VersionOp::Caret => {
                if constraint.major == 0 {
                    // For 0.x, ^0.1.2 means >=0.1.2 and <0.2.0
                    candidate.major == 0
                        && candidate.minor == constraint.minor
                        && candidate.patch >= constraint.patch
                } else {
                    // For 1.x+, ^1.2.3 means >=1.2.3 and <2.0.0
                    candidate.major == constraint.major && candidate >= constraint
                }
            }
        }
    }
}

/// Normalize partial version strings to full semver.
fn normalize_version(version: &str) -> String {
    let parts: Vec<&str> = version.trim().split('.').collect();
    match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => version.to_string(),
    }
}

/// The spn.yaml manifest structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpnManifest {
    /// Project name.
    pub name: String,

    /// Project version (semver).
    pub version: String,

    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,

    /// Optional authors list.
    #[serde(default)]
    pub authors: Vec<String>,

    /// Optional license.
    #[serde(default)]
    pub license: Option<String>,

    /// Optional repository URL.
    #[serde(default)]
    pub repository: Option<String>,

    /// Runtime dependencies.
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,

    /// Development dependencies.
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, Dependency>,
}

impl SpnManifest {
    /// Load manifest from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ManifestError::NotFound(path.display().to_string()));
        }
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// Parse manifest from a YAML string.
    pub fn from_str(content: &str) -> Result<Self, ManifestError> {
        let manifest: SpnManifest = serde_yaml::from_str(content)?;
        Ok(manifest)
    }

    /// Find the manifest file in the current directory or .spn/ subdirectory.
    pub fn find_in_dir<P: AsRef<Path>>(dir: P) -> Result<Self, ManifestError> {
        let dir = dir.as_ref();

        // Check .spn/spn.yaml first
        let spn_dir_path = dir.join(".spn").join("spn.yaml");
        if spn_dir_path.exists() {
            return Self::from_file(&spn_dir_path);
        }

        // Check spn.yaml in root
        let root_path = dir.join("spn.yaml");
        if root_path.exists() {
            return Self::from_file(&root_path);
        }

        Err(ManifestError::NotFound(format!(
            "No spn.yaml found in {} or .spn/",
            dir.display()
        )))
    }

    /// Serialize manifest to YAML string.
    pub fn to_yaml(&self) -> Result<String, ManifestError> {
        let yaml = serde_yaml::to_string(self)?;
        Ok(yaml)
    }

    /// Write manifest to a file.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ManifestError> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// Add a dependency to the manifest.
    pub fn add_dependency(&mut self, name: &str, version: &str) {
        self.dependencies
            .insert(name.to_string(), Dependency::Simple(version.to_string()));
    }

    /// Remove a dependency from the manifest.
    pub fn remove_dependency(&mut self, name: &str) -> Option<Dependency> {
        self.dependencies.remove(name)
    }

    /// Get all dependencies (runtime only).
    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &Dependency)> {
        self.dependencies.iter()
    }

    /// Get all dependencies including dev-dependencies.
    pub fn all_dependencies_with_dev(&self) -> impl Iterator<Item = (&String, &Dependency)> {
        self.dependencies.iter().chain(self.dev_dependencies.iter())
    }
}

impl Default for SpnManifest {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_manifest() {
        let yaml = r#"
name: my-project
version: 1.0.0

dependencies:
  "@nika/seo-audit": "^0.1"
  "@workflows/code-review": "1.0.0"
"#;
        let manifest = SpnManifest::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "my-project");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.dependencies.len(), 2);
    }

    #[test]
    fn test_parse_detailed_dependency() {
        let yaml = r#"
name: test
version: 1.0.0

dependencies:
  "@nika/seo-audit":
    version: "^0.1"
    features: ["mcp"]
"#;
        let manifest = SpnManifest::from_str(yaml).unwrap();
        let dep = manifest.dependencies.get("@nika/seo-audit").unwrap();
        match dep {
            Dependency::Detailed(d) => {
                assert_eq!(d.version, "^0.1");
                assert_eq!(d.features, vec!["mcp"]);
            }
            _ => panic!("Expected detailed dependency"),
        }
    }

    #[test]
    fn test_version_constraint_parsing() {
        let dep = Dependency::Simple("^0.1".to_string());
        let (op, version) = dep.parse_constraint().unwrap();
        assert_eq!(op, VersionOp::Caret);
        assert_eq!(version, semver::Version::new(0, 1, 0));

        let dep = Dependency::Simple("~1.2.3".to_string());
        let (op, version) = dep.parse_constraint().unwrap();
        assert_eq!(op, VersionOp::Tilde);
        assert_eq!(version, semver::Version::new(1, 2, 3));

        let dep = Dependency::Simple(">=2.0".to_string());
        let (op, version) = dep.parse_constraint().unwrap();
        assert_eq!(op, VersionOp::Gte);
        assert_eq!(version, semver::Version::new(2, 0, 0));
    }

    #[test]
    fn test_caret_satisfies() {
        // ^0.1 should match 0.1.0, 0.1.5, but not 0.2.0
        let constraint = semver::Version::new(0, 1, 0);
        assert!(VersionOp::Caret.satisfies(&constraint, &semver::Version::new(0, 1, 0)));
        assert!(VersionOp::Caret.satisfies(&constraint, &semver::Version::new(0, 1, 5)));
        assert!(!VersionOp::Caret.satisfies(&constraint, &semver::Version::new(0, 2, 0)));

        // ^1.2 should match 1.2.0, 1.3.0, but not 2.0.0
        let constraint = semver::Version::new(1, 2, 0);
        assert!(VersionOp::Caret.satisfies(&constraint, &semver::Version::new(1, 2, 0)));
        assert!(VersionOp::Caret.satisfies(&constraint, &semver::Version::new(1, 3, 0)));
        assert!(!VersionOp::Caret.satisfies(&constraint, &semver::Version::new(2, 0, 0)));
    }

    #[test]
    fn test_add_remove_dependency() {
        let mut manifest = SpnManifest::default();
        manifest.name = "test".to_string();

        manifest.add_dependency("@nika/seo-audit", "^0.1");
        assert_eq!(manifest.dependencies.len(), 1);

        manifest.remove_dependency("@nika/seo-audit");
        assert_eq!(manifest.dependencies.len(), 0);
    }

    #[test]
    fn test_serialize_to_yaml() {
        let mut manifest = SpnManifest::default();
        manifest.name = "test-project".to_string();
        manifest.version = "1.0.0".to_string();
        manifest.add_dependency("@nika/seo-audit", "^0.1");

        let yaml = manifest.to_yaml().unwrap();
        assert!(yaml.contains("name: test-project"));
        assert!(yaml.contains("version: 1.0.0"));
        assert!(yaml.contains("@nika/seo-audit"));
    }
}
