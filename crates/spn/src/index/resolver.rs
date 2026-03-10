//! Dependency resolver for SuperNovae packages.
//!
//! Uses a directed graph (petgraph) to resolve transitive dependencies
//! and topological sort to determine installation order.
//!
//! TODO(v0.16): Integrate with `spn add` and `spn install` for dependency resolution
//!
//! # Example
//!
//! ```text
//! use spn::index::{IndexClient, DependencyResolver};
//!
//! let client = IndexClient::new();
//! let mut resolver = DependencyResolver::new(client);
//!
//! // Resolve all dependencies for a package
//! let packages = resolver.resolve("@workflows/seo-audit", None).await?;
//!
//! // Packages are in installation order (dependencies first)
//! for pkg in packages {
//!     println!("Install: {}@{}", pkg.name, pkg.version);
//! }
//! ```

#![allow(dead_code)]

use std::collections::HashMap;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use semver::{Version, VersionReq};
use thiserror::Error;

use super::client::{IndexClient, IndexError};
use super::types::IndexEntry;

/// Errors that can occur during dependency resolution.
#[derive(Debug, Error)]
pub enum ResolverError {
    /// Package not found in registry.
    #[error("Package not found: {0}")]
    PackageNotFound(String),

    /// No version satisfies the requirement.
    #[error("No version of {package} satisfies requirement {requirement}")]
    NoSatisfyingVersion {
        package: String,
        requirement: String,
    },

    /// Cyclic dependency detected.
    #[error("Cyclic dependency detected: {cycle}")]
    CyclicDependency { cycle: String },

    /// Version conflict between requirements.
    #[error("Version conflict for {package}: {v1} vs {v2}")]
    VersionConflict {
        package: String,
        v1: String,
        v2: String,
    },

    /// Invalid version string.
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    /// Invalid version requirement.
    #[error("Invalid version requirement: {0}")]
    InvalidRequirement(String),

    /// Index error.
    #[error("Index error: {0}")]
    IndexError(#[from] IndexError),
}

/// A resolved package with its version and metadata.
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Package name.
    pub name: String,
    /// Resolved version.
    pub version: String,
    /// Full index entry.
    pub entry: IndexEntry,
    /// Whether this is a direct dependency (vs transitive).
    pub is_direct: bool,
}

/// Dependency resolver using directed graph and topological sort.
pub struct DependencyResolver {
    client: IndexClient,
    graph: DiGraph<ResolvedPackage, ()>,
    resolved: HashMap<String, NodeIndex>,
}

impl DependencyResolver {
    /// Create a new resolver with the given index client.
    pub fn new(client: IndexClient) -> Self {
        Self {
            client,
            graph: DiGraph::new(),
            resolved: HashMap::new(),
        }
    }

    /// Resolve all dependencies for a package.
    ///
    /// Returns packages in installation order (dependencies before dependents).
    ///
    /// # Arguments
    ///
    /// * `name` - Package name (e.g., "@workflows/seo-audit")
    /// * `version` - Optional specific version or requirement (e.g., "1.0.0" or "^1.0")
    pub async fn resolve(
        &mut self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Vec<ResolvedPackage>, ResolverError> {
        // Reset state for new resolution
        self.graph.clear();
        self.resolved.clear();

        // Resolve the root package and all dependencies
        self.resolve_recursive(name, version, true).await?;

        // Topological sort to get installation order
        let sorted = toposort(&self.graph, None).map_err(|cycle| {
            let node = &self.graph[cycle.node_id()];
            ResolverError::CyclicDependency {
                cycle: format!("{}@{}", node.name, node.version),
            }
        })?;

        // Return packages in order
        Ok(sorted
            .into_iter()
            .map(|idx| self.graph[idx].clone())
            .collect())
    }

    /// Recursively resolve a package and its dependencies.
    ///
    /// Uses `Box::pin` for recursive async calls to avoid infinite-sized futures.
    fn resolve_recursive<'a>(
        &'a mut self,
        name: &'a str,
        version_req: Option<&'a str>,
        is_direct: bool,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<NodeIndex, ResolverError>> + Send + 'a>,
    > {
        Box::pin(async move {
            // Check if already resolved
            if let Some(&idx) = self.resolved.get(name) {
                // Verify version compatibility
                let resolved = &self.graph[idx];
                if let Some(req) = version_req {
                    if !version_matches(&resolved.version, req)? {
                        return Err(ResolverError::VersionConflict {
                            package: name.to_string(),
                            v1: resolved.version.clone(),
                            v2: req.to_string(),
                        });
                    }
                }
                return Ok(idx);
            }

            // Fetch package from index
            let entry = self.fetch_best_version(name, version_req).await?;

            // Create resolved package
            let resolved_pkg = ResolvedPackage {
                name: name.to_string(),
                version: entry.version.clone(),
                entry: entry.clone(),
                is_direct,
            };

            // Add to graph
            let idx = self.graph.add_node(resolved_pkg);
            self.resolved.insert(name.to_string(), idx);

            // Resolve dependencies
            for dep in &entry.deps {
                // Skip optional and dev dependencies
                if dep.optional || dep.dev {
                    continue;
                }

                // Resolve dependency recursively
                let dep_idx = self
                    .resolve_recursive(&dep.name, Some(&dep.req), false)
                    .await?;

                // Add edge: dependency -> dependent
                // This ensures deps come before dependents in topo sort
                self.graph.add_edge(dep_idx, idx, ());
            }

            Ok(idx)
        })
    }

    /// Fetch the best version that satisfies the requirement.
    async fn fetch_best_version(
        &self,
        name: &str,
        version_req: Option<&str>,
    ) -> Result<IndexEntry, ResolverError> {
        let entries = self
            .client
            .fetch_package(name)
            .await
            .map_err(|_| ResolverError::PackageNotFound(name.to_string()))?;

        match version_req {
            Some(req) => {
                find_best_match(req, &entries).ok_or_else(|| ResolverError::NoSatisfyingVersion {
                    package: name.to_string(),
                    requirement: req.to_string(),
                })
            }
            None => {
                // Get latest non-yanked version
                entries
                    .iter()
                    .filter(|e| !e.yanked)
                    .max_by(|a, b| {
                        let va = Version::parse(&a.version).ok();
                        let vb = Version::parse(&b.version).ok();
                        va.cmp(&vb)
                    })
                    .cloned()
                    .ok_or_else(|| ResolverError::PackageNotFound(name.to_string()))
            }
        }
    }

    /// Get resolution statistics.
    pub fn stats(&self) -> ResolverStats {
        let direct = self.graph.node_weights().filter(|p| p.is_direct).count();
        let transitive = self.graph.node_count() - direct;

        ResolverStats {
            total: self.graph.node_count(),
            direct,
            transitive,
        }
    }
}

/// Statistics about a dependency resolution.
#[derive(Debug, Clone)]
pub struct ResolverStats {
    /// Total number of packages.
    pub total: usize,
    /// Direct dependencies.
    pub direct: usize,
    /// Transitive dependencies.
    pub transitive: usize,
}

/// Find the best version that satisfies a requirement.
///
/// Supports semver requirements like `^1.0`, `~1.0.0`, `>=1.0,<2.0`, `=1.0.0`.
pub fn find_best_match(requirement: &str, available: &[IndexEntry]) -> Option<IndexEntry> {
    // Try to parse as exact version first
    if let Ok(exact) = Version::parse(requirement) {
        return available
            .iter()
            .find(|e| !e.yanked && e.version == exact.to_string())
            .cloned();
    }

    // Parse as version requirement
    let req = match VersionReq::parse(requirement) {
        Ok(req) => req,
        Err(e) => {
            tracing::debug!("Invalid version requirement '{}': {}", requirement, e);
            return None;
        }
    };

    available
        .iter()
        .filter(|e| !e.yanked)
        .filter(|e| {
            Version::parse(&e.version)
                .map(|v| req.matches(&v))
                .unwrap_or(false)
        })
        .max_by(|a, b| {
            let va = Version::parse(&a.version).ok();
            let vb = Version::parse(&b.version).ok();
            va.cmp(&vb)
        })
        .cloned()
}

/// Check if a version matches a requirement.
fn version_matches(version: &str, requirement: &str) -> Result<bool, ResolverError> {
    let v =
        Version::parse(version).map_err(|_| ResolverError::InvalidVersion(version.to_string()))?;

    // Try exact match first
    if let Ok(exact) = Version::parse(requirement) {
        return Ok(v == exact);
    }

    // Try as requirement
    let req = VersionReq::parse(requirement)
        .map_err(|_| ResolverError::InvalidRequirement(requirement.to_string()))?;

    Ok(req.matches(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, version: &str) -> IndexEntry {
        IndexEntry::new(name, version, &format!("sha256:{}", version))
    }

    #[test]
    fn test_find_best_match_exact() {
        let entries = vec![
            make_entry("pkg", "1.0.0"),
            make_entry("pkg", "1.1.0"),
            make_entry("pkg", "2.0.0"),
        ];

        let best = find_best_match("1.0.0", &entries).unwrap();
        assert_eq!(best.version, "1.0.0");
    }

    #[test]
    fn test_find_best_match_caret() {
        let entries = vec![
            make_entry("pkg", "1.0.0"),
            make_entry("pkg", "1.1.0"),
            make_entry("pkg", "1.2.5"),
            make_entry("pkg", "2.0.0"),
        ];

        // ^1.0 should match highest 1.x
        let best = find_best_match("^1.0", &entries).unwrap();
        assert_eq!(best.version, "1.2.5");
    }

    #[test]
    fn test_find_best_match_tilde() {
        let entries = vec![
            make_entry("pkg", "1.0.0"),
            make_entry("pkg", "1.0.5"),
            make_entry("pkg", "1.1.0"),
        ];

        // ~1.0.0 should match highest 1.0.x
        let best = find_best_match("~1.0.0", &entries).unwrap();
        assert_eq!(best.version, "1.0.5");
    }

    #[test]
    fn test_find_best_match_range() {
        let entries = vec![
            make_entry("pkg", "1.0.0"),
            make_entry("pkg", "1.5.0"),
            make_entry("pkg", "2.0.0"),
            make_entry("pkg", "2.5.0"),
        ];

        // >=1.0,<2.0 should match highest in range
        let best = find_best_match(">=1.0.0, <2.0.0", &entries).unwrap();
        assert_eq!(best.version, "1.5.0");
    }

    #[test]
    fn test_find_best_match_skips_yanked() {
        let mut entries = vec![make_entry("pkg", "1.0.0"), make_entry("pkg", "1.1.0")];
        entries[1].yanked = true;

        let best = find_best_match("^1.0", &entries).unwrap();
        assert_eq!(best.version, "1.0.0"); // 1.1.0 is yanked
    }

    #[test]
    fn test_find_best_match_no_match() {
        let entries = vec![make_entry("pkg", "1.0.0"), make_entry("pkg", "1.1.0")];

        let best = find_best_match("^2.0", &entries);
        assert!(best.is_none());
    }

    #[test]
    fn test_version_matches() {
        assert!(version_matches("1.0.0", "^1.0").unwrap());
        assert!(version_matches("1.5.0", "^1.0").unwrap());
        assert!(!version_matches("2.0.0", "^1.0").unwrap());

        assert!(version_matches("1.0.5", "~1.0.0").unwrap());
        assert!(!version_matches("1.1.0", "~1.0.0").unwrap());

        assert!(version_matches("1.0.0", "1.0.0").unwrap());
        assert!(!version_matches("1.0.1", "1.0.0").unwrap());
    }

    #[test]
    fn test_resolver_stats() {
        let client = IndexClient::new();
        let resolver = DependencyResolver::new(client);
        let stats = resolver.stats();

        assert_eq!(stats.total, 0);
        assert_eq!(stats.direct, 0);
        assert_eq!(stats.transitive, 0);
    }

    #[test]
    fn test_resolver_error_display() {
        let err = ResolverError::PackageNotFound("@test/pkg".to_string());
        assert_eq!(err.to_string(), "Package not found: @test/pkg");

        let err = ResolverError::NoSatisfyingVersion {
            package: "@test/pkg".to_string(),
            requirement: "^2.0".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "No version of @test/pkg satisfies requirement ^2.0"
        );

        let err = ResolverError::CyclicDependency {
            cycle: "a@1.0.0 → b@1.0.0 → a@1.0.0".to_string(),
        };
        assert!(err.to_string().contains("Cyclic dependency detected"));

        let err = ResolverError::VersionConflict {
            package: "@test/pkg".to_string(),
            v1: "1.0.0".to_string(),
            v2: "2.0.0".to_string(),
        };
        assert!(err.to_string().contains("Version conflict"));
        assert!(err.to_string().contains("1.0.0"));
        assert!(err.to_string().contains("2.0.0"));
    }

    #[test]
    fn test_resolved_package_fields() {
        let entry = make_entry("@test/pkg", "1.0.0");
        let pkg = ResolvedPackage {
            name: "@test/pkg".to_string(),
            version: "1.0.0".to_string(),
            entry: entry.clone(),
            is_direct: true,
        };

        assert_eq!(pkg.name, "@test/pkg");
        assert_eq!(pkg.version, "1.0.0");
        assert!(pkg.is_direct);
        assert_eq!(pkg.entry.cksum, entry.cksum);
    }

    #[test]
    fn test_version_matches_invalid_version() {
        let result = version_matches("invalid", "^1.0");
        assert!(result.is_err());
        assert!(matches!(result, Err(ResolverError::InvalidVersion(_))));
    }

    #[test]
    fn test_version_matches_invalid_requirement() {
        let result = version_matches("1.0.0", "not a version req");
        assert!(result.is_err());
        assert!(matches!(result, Err(ResolverError::InvalidRequirement(_))));
    }

    #[test]
    fn test_find_best_match_prefers_latest() {
        let entries = vec![
            make_entry("pkg", "1.0.0"),
            make_entry("pkg", "1.2.0"),
            make_entry("pkg", "1.1.0"),
        ];

        // Should return 1.2.0 (highest matching), not 1.1.0 (last in list)
        let best = find_best_match("^1.0", &entries).unwrap();
        assert_eq!(best.version, "1.2.0");
    }

    #[test]
    fn test_find_best_match_equal_operator() {
        let entries = vec![
            make_entry("pkg", "1.0.0"),
            make_entry("pkg", "1.1.0"),
            make_entry("pkg", "1.2.0"),
        ];

        // =1.1.0 should only match exactly 1.1.0
        let best = find_best_match("=1.1.0", &entries).unwrap();
        assert_eq!(best.version, "1.1.0");
    }

    #[test]
    fn test_find_best_match_all_yanked() {
        let mut entries = vec![make_entry("pkg", "1.0.0"), make_entry("pkg", "1.1.0")];
        entries[0].yanked = true;
        entries[1].yanked = true;

        let best = find_best_match("^1.0", &entries);
        assert!(best.is_none());
    }
}
