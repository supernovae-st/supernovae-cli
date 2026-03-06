//! Package registry types for the SuperNovae ecosystem.
//!
//! These types are used by spn-client for package resolution.

use std::collections::HashMap;

/// Type of package in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PackageType {
    /// Workflow package (nika workflows)
    #[default]
    Workflow,
    /// Skill package (Claude Code skills)
    Skill,
    /// Agent package
    Agent,
    /// Prompt package
    Prompt,
    /// Job package
    Job,
    /// Schema package
    Schema,
    /// MCP server package
    Mcp,
    /// Model package (ollama/huggingface)
    Model,
    /// Data package
    Data,
}

/// Source of a package - where to fetch the actual content.
///
/// The registry contains metadata; sources define where to get the content.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "lowercase"))]
pub enum Source {
    /// Tarball from our CDN or GitHub releases
    Tarball {
        /// URL to download the tarball
        url: String,
        /// SHA256 checksum for verification
        checksum: String,
    },
    /// NPM package (for MCP servers)
    Npm {
        /// NPM package name (e.g., "@modelcontextprotocol/server-filesystem")
        package: String,
        /// Version constraint (e.g., "^1.0.0")
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        version: Option<String>,
    },
    /// PyPI package (for Python MCP servers)
    PyPi {
        /// PyPI package name
        package: String,
        /// Version constraint
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        version: Option<String>,
    },
    /// Pre-built binary (platform-specific)
    Binary {
        /// Platform → URL mapping (e.g., "darwin-arm64" → "https://...")
        platforms: HashMap<String, String>,
    },
    /// Ollama model
    Ollama {
        /// Model name (e.g., "deepseek-coder:6.7b")
        model: String,
    },
    /// HuggingFace model
    HuggingFace {
        /// Repository ID (e.g., "deepseek-ai/deepseek-coder-6.7b")
        repo: String,
        /// Quantization type (e.g., "Q4_K_M")
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        quantization: Option<String>,
    },
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PackageManifest {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,
    /// Package type
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub package_type: PackageType,
    /// Source - where to fetch the actual content
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub source: Option<Source>,
    /// Authors
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Vec::is_empty", default)
    )]
    pub authors: Vec<String>,
    /// License
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub license: Option<String>,
    /// Repository URL
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub repository: Option<String>,
    /// Keywords for search
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Vec::is_empty", default)
    )]
    pub keywords: Vec<String>,
    /// Dependencies
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Vec::is_empty", default)
    )]
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
        assert!(manifest.source.is_none());
    }

    #[test]
    fn test_package_type_variants() {
        assert_eq!(PackageType::default(), PackageType::Workflow);

        // Ensure all variants exist
        let types = [
            PackageType::Workflow,
            PackageType::Skill,
            PackageType::Agent,
            PackageType::Prompt,
            PackageType::Job,
            PackageType::Schema,
            PackageType::Mcp,
            PackageType::Model,
            PackageType::Data,
        ];
        assert_eq!(types.len(), 9);
    }

    #[test]
    fn test_source_npm() {
        let source = Source::Npm {
            package: "@modelcontextprotocol/server-filesystem".to_string(),
            version: Some("^1.0.0".to_string()),
        };

        if let Source::Npm { package, version } = source {
            assert_eq!(package, "@modelcontextprotocol/server-filesystem");
            assert_eq!(version, Some("^1.0.0".to_string()));
        } else {
            panic!("Expected Npm source");
        }
    }

    #[test]
    fn test_source_ollama() {
        let source = Source::Ollama {
            model: "deepseek-coder:6.7b".to_string(),
        };

        if let Source::Ollama { model } = source {
            assert_eq!(model, "deepseek-coder:6.7b");
        } else {
            panic!("Expected Ollama source");
        }
    }

    #[test]
    fn test_source_huggingface() {
        let source = Source::HuggingFace {
            repo: "deepseek-ai/deepseek-coder-6.7b".to_string(),
            quantization: Some("Q4_K_M".to_string()),
        };

        if let Source::HuggingFace { repo, quantization } = source {
            assert_eq!(repo, "deepseek-ai/deepseek-coder-6.7b");
            assert_eq!(quantization, Some("Q4_K_M".to_string()));
        } else {
            panic!("Expected HuggingFace source");
        }
    }

    #[test]
    fn test_source_tarball() {
        let source = Source::Tarball {
            url: "https://cdn.supernovae.studio/packages/workflow-1.0.0.tar.gz".to_string(),
            checksum: "sha256:abc123".to_string(),
        };

        if let Source::Tarball { url, checksum } = source {
            assert!(url.ends_with(".tar.gz"));
            assert!(checksum.starts_with("sha256:"));
        } else {
            panic!("Expected Tarball source");
        }
    }

    #[test]
    fn test_source_binary() {
        let mut platforms = HashMap::new();
        platforms.insert(
            "darwin-arm64".to_string(),
            "https://github.com/org/repo/releases/download/v1.0.0/bin-darwin-arm64".to_string(),
        );
        platforms.insert(
            "linux-x86_64".to_string(),
            "https://github.com/org/repo/releases/download/v1.0.0/bin-linux-x86_64".to_string(),
        );

        let source = Source::Binary { platforms };

        if let Source::Binary { platforms } = source {
            assert_eq!(platforms.len(), 2);
            assert!(platforms.contains_key("darwin-arm64"));
            assert!(platforms.contains_key("linux-x86_64"));
        } else {
            panic!("Expected Binary source");
        }
    }

    #[test]
    fn test_manifest_with_source() {
        let mut manifest = PackageManifest::new("@mcp/neo4j", "1.0.0");
        manifest.package_type = PackageType::Mcp;
        manifest.source = Some(Source::Npm {
            package: "@neo4j/mcp-server-neo4j".to_string(),
            version: Some("^1.0.0".to_string()),
        });

        assert_eq!(manifest.package_type, PackageType::Mcp);
        assert!(manifest.source.is_some());

        if let Some(Source::Npm { package, .. }) = &manifest.source {
            assert_eq!(package, "@neo4j/mcp-server-neo4j");
        }
    }
}
