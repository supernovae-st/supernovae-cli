//! Configuration scope types.

use std::fmt;
use std::path::PathBuf;

/// Configuration scope level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScopeType {
    /// Global user config (~/.spn/config.toml).
    Global,
    /// Team/project config (./mcp.yaml, ./spn.yaml).
    Team,
    /// Local overrides (./.spn/local.yaml).
    Local,
}

impl ScopeType {
    /// Get display name for the scope.
    pub fn display_name(&self) -> &'static str {
        match self {
            ScopeType::Global => "Global",
            ScopeType::Team => "Team",
            ScopeType::Local => "Local",
        }
    }

    /// Get emoji indicator for the scope.
    pub fn emoji(&self) -> &'static str {
        match self {
            ScopeType::Global => "🌍",
            ScopeType::Team => "👥",
            ScopeType::Local => "💻",
        }
    }

    /// Parse scope from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "global" | "g" | "user" => Some(ScopeType::Global),
            "team" | "t" | "project" => Some(ScopeType::Team),
            "local" | "l" => Some(ScopeType::Local),
            _ => None,
        }
    }
}

impl fmt::Display for ScopeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.emoji(), self.display_name())
    }
}

/// Configuration scope with path information.
#[derive(Debug, Clone)]
pub struct ConfigScope {
    /// Scope type.
    pub scope_type: ScopeType,
    /// Path to configuration file.
    pub path: PathBuf,
    /// Whether the file exists.
    pub exists: bool,
}

impl ConfigScope {
    /// Create a new config scope.
    pub fn new(scope_type: ScopeType, path: PathBuf) -> Self {
        let exists = path.exists();
        Self {
            scope_type,
            path,
            exists,
        }
    }

    /// Get display name.
    pub fn display_name(&self) -> String {
        format!(
            "{} ({}){}",
            self.scope_type,
            self.path.display(),
            if !self.exists { " [not found]" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_type_ordering() {
        assert!(ScopeType::Global < ScopeType::Team);
        assert!(ScopeType::Team < ScopeType::Local);
    }

    #[test]
    fn test_scope_type_from_str() {
        assert_eq!(ScopeType::from_str("global"), Some(ScopeType::Global));
        assert_eq!(ScopeType::from_str("g"), Some(ScopeType::Global));
        assert_eq!(ScopeType::from_str("team"), Some(ScopeType::Team));
        assert_eq!(ScopeType::from_str("local"), Some(ScopeType::Local));
        assert_eq!(ScopeType::from_str("invalid"), None);
    }

    #[test]
    fn test_scope_display() {
        assert_eq!(ScopeType::Global.to_string(), "🌍 Global");
        assert_eq!(ScopeType::Team.to_string(), "👥 Team");
        assert_eq!(ScopeType::Local.to_string(), "💻 Local");
    }
}
