//! skills.sh integration.
//!
//! Proxies skill installation from https://skills.sh
//!
//! TODO(v0.14): Integrate with `spn skill` commands

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Stdio};

use thiserror::Error;

/// skills.sh API base URL.
pub const SKILLS_API: &str = "https://skills.sh/api";

/// skills.sh registry base URL.
pub const SKILLS_REGISTRY: &str = "https://skills.sh";

/// Errors that can occur with skills.sh operations.
#[derive(Error, Debug)]
pub enum SkillsError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Installation failed: {0}")]
    InstallFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for skills operations.
pub type Result<T> = std::result::Result<T, SkillsError>;

/// Skill metadata from skills.sh.
#[derive(Debug, Clone)]
pub struct SkillInfo {
    /// Skill name (e.g., "brainstorming").
    pub name: String,

    /// Full path (e.g., "superpowers/brainstorming").
    pub path: String,

    /// Description.
    pub description: Option<String>,

    /// Author.
    pub author: Option<String>,

    /// Install count.
    pub installs: u64,
}

/// Skills.sh client for proxying skill operations.
pub struct SkillsClient {
    /// Target directory for skills (~/.claude/skills/).
    target_dir: PathBuf,
}

impl SkillsClient {
    /// Create a new skills client.
    pub fn new() -> Self {
        let target_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join("skills");

        Self { target_dir }
    }

    /// Create a client with a custom target directory.
    pub fn with_target_dir(target_dir: PathBuf) -> Self {
        Self { target_dir }
    }

    /// Get the target directory.
    pub fn target_dir(&self) -> &PathBuf {
        &self.target_dir
    }

    /// Install a skill from skills.sh.
    ///
    /// Uses curl to download the SKILL.md file.
    pub fn install(&self, name: &str) -> Result<PathBuf> {
        // Ensure target directory exists
        std::fs::create_dir_all(&self.target_dir)?;

        // Normalize skill name (remove leading @ or /)
        let skill_path = name.trim_start_matches('@').trim_start_matches('/');

        // Determine the skill file path
        let skill_file = self
            .target_dir
            .join(format!("{}.md", skill_path.replace('/', "-")));

        // Download using curl (skills.sh uses raw markdown)
        let url = format!("{}/{}/SKILL.md", SKILLS_REGISTRY, skill_path);

        let status = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(&skill_file)
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(SkillsError::NotFound(name.to_string()));
        }

        Ok(skill_file)
    }

    /// Remove an installed skill.
    pub fn remove(&self, name: &str) -> Result<()> {
        let skill_path = name.trim_start_matches('@').trim_start_matches('/');
        let skill_file = self
            .target_dir
            .join(format!("{}.md", skill_path.replace('/', "-")));

        if skill_file.exists() {
            std::fs::remove_file(&skill_file)?;
        }

        Ok(())
    }

    /// List installed skills.
    pub fn list_installed(&self) -> Result<Vec<String>> {
        if !self.target_dir.exists() {
            return Ok(vec![]);
        }

        let mut skills = Vec::new();

        for entry in std::fs::read_dir(&self.target_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "md") {
                if let Some(name) = path.file_stem() {
                    skills.push(name.to_string_lossy().to_string());
                }
            }
        }

        skills.sort();
        Ok(skills)
    }

    /// Search skills on skills.sh.
    ///
    /// Uses the skills.sh search page (returns URL for now).
    pub fn search_url(&self, query: &str) -> String {
        format!(
            "{}/search?q={}",
            SKILLS_REGISTRY,
            urlencoding::encode(query)
        )
    }
}

impl Default for SkillsClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Install a skill from skills.sh.
pub fn install_skill(name: &str) -> Result<PathBuf> {
    SkillsClient::new().install(name)
}

/// Remove an installed skill.
pub fn remove_skill(name: &str) -> Result<()> {
    SkillsClient::new().remove(name)
}

/// List installed skills.
pub fn list_skills() -> Result<Vec<String>> {
    SkillsClient::new().list_installed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_skills_client_creation() {
        let client = SkillsClient::new();
        assert!(client.target_dir().to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_custom_target_dir() {
        let temp = TempDir::new().unwrap();
        let client = SkillsClient::with_target_dir(temp.path().to_path_buf());
        assert_eq!(client.target_dir(), temp.path());
    }

    #[test]
    fn test_search_url() {
        let client = SkillsClient::new();
        let url = client.search_url("brainstorming");
        assert!(url.contains("skills.sh"));
        assert!(url.contains("brainstorming"));
    }

    #[test]
    fn test_list_installed_empty() {
        let temp = TempDir::new().unwrap();
        let client = SkillsClient::with_target_dir(temp.path().to_path_buf());
        let skills = client.list_installed().unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_list_installed_with_skills() {
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path();
        std::fs::create_dir_all(skills_dir).unwrap();

        // Create some fake skill files
        std::fs::write(skills_dir.join("brainstorming.md"), "# Brainstorming").unwrap();
        std::fs::write(skills_dir.join("tdd.md"), "# TDD").unwrap();

        let client = SkillsClient::with_target_dir(skills_dir.to_path_buf());
        let skills = client.list_installed().unwrap();

        assert_eq!(skills.len(), 2);
        assert!(skills.contains(&"brainstorming".to_string()));
        assert!(skills.contains(&"tdd".to_string()));
    }

    #[test]
    fn test_remove_skill() {
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path();
        std::fs::create_dir_all(skills_dir).unwrap();

        let skill_file = skills_dir.join("test-skill.md");
        std::fs::write(&skill_file, "# Test").unwrap();
        assert!(skill_file.exists());

        let client = SkillsClient::with_target_dir(skills_dir.to_path_buf());
        client.remove("test-skill").unwrap();

        assert!(!skill_file.exists());
    }
}
