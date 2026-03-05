//! Package registry types for the SuperNovae ecosystem.
//!
//! These types are used by spn-client for package resolution.

/// Type of package in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PackageType {
    /// Workflow package (nika workflows)
    #[default]
    Workflow,
    /// Skill package (Claude Code skills)
    Skill,
    /// Agent package
    Agent,
    /// MCP server package
    Mcp,
    /// Data package
    Data,
}

/// Reference to a package in the registry.
///
/// # Format
///
/// - `@scope/name` - Latest version
/// - `@scope/name@1.2.3` - Specific version
/// - `@scope/name@^1.0` - Version range
///
/// # Example
///
/// ```
/// use spn_core::PackageRef;
///
/// let pkg = PackageRef::parse("@workflows/code-review@1.0.0").unwrap();
/// assert_eq!(pkg.scope, Some("workflows".to_string()));
/// assert_eq!(pkg.name, "code-review");
/// assert_eq!(pkg.version, Some("1.0.0".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    /// Package scope (e.g., "workflows" from "@workflows/name")
    pub scope: Option<String>,
    /// Package name
    pub name: String,
    /// Version constraint
    pub version: Option<String>,
}

impl PackageRef {
    /// Parse a package reference string.
    ///
    /// Supports formats:
    /// - `name`
    /// - `name@version`
    /// - `@scope/name`
    /// - `@scope/name@version`
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        // Check if scoped (@scope/name@version)
        if let Some(without_at) = input.strip_prefix('@') {
            let (scope, rest) = without_at.split_once('/')?;

            // Check for version
            if let Some((name, version)) = rest.split_once('@') {
                Some(PackageRef {
                    scope: Some(scope.to_string()),
                    name: name.to_string(),
                    version: Some(version.to_string()),
                })
            } else {
                Some(PackageRef {
                    scope: Some(scope.to_string()),
                    name: rest.to_string(),
                    version: None,
                })
            }
        } else {
            // name or name@version
            if let Some((name, version)) = input.split_once('@') {
                Some(PackageRef {
                    scope: None,
                    name: name.to_string(),
                    version: Some(version.to_string()),
                })
            } else {
                Some(PackageRef {
                    scope: None,
                    name: input.to_string(),
                    version: None,
                })
            }
        }
    }

    /// Get the full package name (with scope if present).
    pub fn full_name(&self) -> String {
        match &self.scope {
            Some(scope) => format!("@{}/{}", scope, self.name),
            None => self.name.clone(),
        }
    }

    /// Get the full package reference string.
    pub fn to_string_with_version(&self) -> String {
        match &self.version {
            Some(v) => format!("{}@{}", self.full_name(), v),
            None => self.full_name(),
        }
    }
}

/// Package manifest (spn.yaml content).
#[derive(Debug, Clone, Default)]
pub struct PackageManifest {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Package type
    pub package_type: PackageType,
    /// Authors
    pub authors: Vec<String>,
    /// License
    pub license: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Keywords for search
    pub keywords: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<PackageRef>,
}

impl PackageManifest {
    /// Create a new package manifest.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            ..Default::default()
        }
    }

    /// Get the package reference for this manifest.
    pub fn as_ref(&self) -> PackageRef {
        PackageRef {
            scope: None, // TODO: Extract from name if scoped
            name: self.name.clone(),
            version: Some(self.version.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_name() {
        let pkg = PackageRef::parse("code-review").unwrap();
        assert_eq!(pkg.scope, None);
        assert_eq!(pkg.name, "code-review");
        assert_eq!(pkg.version, None);
    }

    #[test]
    fn test_parse_name_with_version() {
        let pkg = PackageRef::parse("code-review@1.0.0").unwrap();
        assert_eq!(pkg.scope, None);
        assert_eq!(pkg.name, "code-review");
        assert_eq!(pkg.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_scoped() {
        let pkg = PackageRef::parse("@workflows/code-review").unwrap();
        assert_eq!(pkg.scope, Some("workflows".to_string()));
        assert_eq!(pkg.name, "code-review");
        assert_eq!(pkg.version, None);
    }

    #[test]
    fn test_parse_scoped_with_version() {
        let pkg = PackageRef::parse("@workflows/code-review@1.2.3").unwrap();
        assert_eq!(pkg.scope, Some("workflows".to_string()));
        assert_eq!(pkg.name, "code-review");
        assert_eq!(pkg.version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_empty() {
        assert!(PackageRef::parse("").is_none());
        assert!(PackageRef::parse("  ").is_none());
    }

    #[test]
    fn test_full_name() {
        let scoped = PackageRef::parse("@workflows/code-review").unwrap();
        assert_eq!(scoped.full_name(), "@workflows/code-review");

        let unscoped = PackageRef::parse("code-review").unwrap();
        assert_eq!(unscoped.full_name(), "code-review");
    }

    #[test]
    fn test_to_string_with_version() {
        let pkg = PackageRef::parse("@workflows/code-review@1.0.0").unwrap();
        assert_eq!(pkg.to_string_with_version(), "@workflows/code-review@1.0.0");

        let pkg = PackageRef::parse("@workflows/code-review").unwrap();
        assert_eq!(pkg.to_string_with_version(), "@workflows/code-review");
    }

    #[test]
    fn test_package_manifest() {
        let manifest = PackageManifest::new("my-workflow", "1.0.0");
        assert_eq!(manifest.name, "my-workflow");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.package_type, PackageType::Workflow);
    }
}
