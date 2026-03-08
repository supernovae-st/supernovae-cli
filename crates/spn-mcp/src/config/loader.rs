//! Configuration loading from ~/.spn/apis/

use std::fs;
use std::path::PathBuf;

use crate::error::{Error, Result};

use super::schema::ApiConfig;

/// Get the APIs configuration directory (~/.spn/apis/).
pub fn apis_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        ))
    })?;

    Ok(home.join(".spn").join("apis"))
}

/// Load a specific API configuration by name.
pub fn load_api(name: &str) -> Result<ApiConfig> {
    let dir = apis_dir()?;
    let path = dir.join(format!("{}.yaml", name));

    if !path.exists() {
        // Try .yml extension
        let yml_path = dir.join(format!("{}.yml", name));
        if yml_path.exists() {
            return load_from_path(&yml_path);
        }
        return Err(Error::ConfigNotFound(name.into()));
    }

    load_from_path(&path)
}

/// Load all API configurations from ~/.spn/apis/.
pub fn load_all_apis() -> Result<Vec<ApiConfig>> {
    let dir = apis_dir()?;

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut configs = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process .yaml and .yml files
        if let Some(ext) = path.extension() {
            if ext == "yaml" || ext == "yml" {
                match load_from_path(&path) {
                    Ok(config) => configs.push(config),
                    Err(e) => {
                        tracing::warn!("Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    // Sort by name for consistent ordering
    configs.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(configs)
}

/// Load configuration from a specific path.
fn load_from_path(path: &PathBuf) -> Result<ApiConfig> {
    let content = fs::read_to_string(path)?;
    let config: ApiConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config(dir: &TempDir, name: &str, content: &str) {
        let path = dir.path().join(format!("{}.yaml", name));
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_apis_dir() {
        let dir = apis_dir().unwrap();
        assert!(dir.ends_with(".spn/apis"));
    }

    #[test]
    fn test_load_from_path() {
        let dir = TempDir::new().unwrap();
        create_test_config(
            &dir,
            "test",
            r#"
name: test
base_url: https://example.com
auth:
  type: bearer
  credential: test
tools:
  - name: ping
    path: /ping
"#,
        );

        let path = dir.path().join("test.yaml");
        let config = load_from_path(&path).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.tools.len(), 1);
    }
}
