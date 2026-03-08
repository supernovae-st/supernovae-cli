//! Model run command implementation.
//!
//! Runs inference on a local model via Ollama.

use crate::error::{Result, SpnError};
use crate::ux::design_system as ds;
use spn_client::SpnClient;
use std::io::{self, Read};

/// Arguments for the model run command.
#[derive(Clone, Debug)]
pub struct RunArgs {
    /// Model name (e.g., llama3.2, mistral:7b)
    pub model: String,
    /// Prompt text (use - for stdin, @file for file input)
    pub prompt: String,
    /// Stream output tokens as they arrive
    pub stream: bool,
    /// Temperature (0.0 - 2.0)
    pub temperature: f32,
    /// System prompt
    pub system: Option<String>,
    /// Output as JSON
    pub json: bool,
}

impl Default for RunArgs {
    fn default() -> Self {
        Self {
            model: String::new(),
            prompt: String::new(),
            stream: false,
            temperature: 0.7,
            system: None,
            json: false,
        }
    }
}

/// Resolve prompt from various input sources.
///
/// - `-` reads from stdin
/// - `@path` reads from file
/// - Otherwise uses the prompt as-is
pub fn resolve_prompt(prompt: &str) -> Result<String> {
    if prompt == "-" {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| SpnError::CommandFailed(format!("Failed to read stdin: {}", e)))?;
        Ok(buffer.trim().to_string())
    } else if let Some(path) = prompt.strip_prefix('@') {
        // Read from file
        std::fs::read_to_string(path)
            .map(|s| s.trim().to_string())
            .map_err(|e| SpnError::CommandFailed(format!("Failed to read file '{}': {}", path, e)))
    } else {
        // Use prompt as-is
        Ok(prompt.to_string())
    }
}

/// Build the full prompt with optional prefix.
#[allow(dead_code)] // For future use with prompt prefixes
pub fn build_full_prompt(prefix: Option<&str>, content: &str) -> String {
    match prefix {
        Some(p) if !p.is_empty() => format!("{} {}", p.trim(), content),
        _ => content.to_string(),
    }
}

/// Run inference on a model.
pub async fn run(args: RunArgs) -> Result<()> {
    // Resolve prompt from stdin, file, or direct
    let prompt = resolve_prompt(&args.prompt)?;

    if prompt.is_empty() {
        return Err(SpnError::CommandFailed(
            "Prompt cannot be empty".to_string(),
        ));
    }

    // Connect to daemon
    let mut client = SpnClient::connect().await.map_err(|_| {
        SpnError::CommandFailed(format!(
            "Daemon is not running\n\nStart the daemon with: {} spn daemon start",
            ds::primary("->")
        ))
    })?;

    // Send model run request
    let request = spn_client::Request::ModelRun {
        model: args.model.clone(),
        prompt: prompt.clone(),
        system: args.system.clone(),
        temperature: Some(args.temperature),
        stream: args.stream,
    };

    let response = client
        .send_request(request)
        .await
        .map_err(|e| SpnError::CommandFailed(format!("Daemon request failed: {}", e)))?;

    match response {
        spn_client::Response::ModelRunResult { content, stats } => {
            if args.json {
                let output = serde_json::json!({
                    "model": args.model,
                    "prompt": prompt,
                    "response": content,
                    "stats": stats,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                // Print response directly
                print!("{}", content);
                if !content.ends_with('\n') {
                    println!();
                }

                // Print stats if verbose
                if let Some(stats) = stats {
                    if let Some(tps) = stats.get("tokens_per_second") {
                        eprintln!(
                            "\n{} {:.1} tokens/sec",
                            ds::muted("Speed:"),
                            tps.as_f64().unwrap_or(0.0)
                        );
                    }
                }
            }
        }

        spn_client::Response::Error { message } => {
            return Err(SpnError::CommandFailed(message));
        }

        _ => {
            return Err(SpnError::CommandFailed(
                "Unexpected response from daemon".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // =========================================================================
    // resolve_prompt tests
    // =========================================================================

    #[test]
    fn test_resolve_prompt_direct() {
        let result = resolve_prompt("Hello world").unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_resolve_prompt_empty() {
        let result = resolve_prompt("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_resolve_prompt_file_input() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Content from file").unwrap();

        let path = format!("@{}", file.path().display());
        let result = resolve_prompt(&path).unwrap();
        assert_eq!(result, "Content from file");
    }

    #[test]
    fn test_resolve_prompt_file_not_found() {
        let result = resolve_prompt("@/nonexistent/path/file.txt");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read file"));
    }

    #[test]
    fn test_resolve_prompt_file_with_whitespace() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "  Content with whitespace  ").unwrap();
        writeln!(file, "  ").unwrap();

        let path = format!("@{}", file.path().display());
        let result = resolve_prompt(&path).unwrap();
        assert_eq!(result, "Content with whitespace");
    }

    // =========================================================================
    // build_full_prompt tests
    // =========================================================================

    #[test]
    fn test_build_full_prompt_no_prefix() {
        let result = build_full_prompt(None, "content");
        assert_eq!(result, "content");
    }

    #[test]
    fn test_build_full_prompt_with_prefix() {
        let result = build_full_prompt(Some("Review:"), "the code");
        assert_eq!(result, "Review: the code");
    }

    #[test]
    fn test_build_full_prompt_empty_prefix() {
        let result = build_full_prompt(Some(""), "content");
        assert_eq!(result, "content");
    }

    #[test]
    fn test_build_full_prompt_prefix_with_whitespace() {
        let result = build_full_prompt(Some("  Analyze:  "), "data");
        assert_eq!(result, "Analyze: data");
    }

    // =========================================================================
    // RunArgs tests
    // =========================================================================

    #[test]
    fn test_run_args_default() {
        let args = RunArgs::default();
        assert!(args.model.is_empty());
        assert!(args.prompt.is_empty());
        assert!(!args.stream);
        assert!((args.temperature - 0.7).abs() < f32::EPSILON);
        assert!(args.system.is_none());
        assert!(!args.json);
    }

    #[test]
    fn test_run_args_with_values() {
        let args = RunArgs {
            model: "llama3.2".to_string(),
            prompt: "Hello".to_string(),
            stream: true,
            temperature: 0.5,
            system: Some("You are helpful".to_string()),
            json: true,
        };

        assert_eq!(args.model, "llama3.2");
        assert_eq!(args.prompt, "Hello");
        assert!(args.stream);
        assert!((args.temperature - 0.5).abs() < f32::EPSILON);
        assert_eq!(args.system.as_deref(), Some("You are helpful"));
        assert!(args.json);
    }

    // =========================================================================
    // Integration tests (require daemon - marked as ignored)
    // =========================================================================

    #[tokio::test]
    #[ignore = "requires running daemon"]
    async fn test_run_basic_prompt() {
        let args = RunArgs {
            model: "llama3.2".to_string(),
            prompt: "Say hello".to_string(),
            ..Default::default()
        };

        let result = run(args).await;
        // Should succeed if daemon and model are available
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Daemon"));
    }

    #[tokio::test]
    #[ignore = "requires running daemon"]
    async fn test_run_empty_prompt_fails() {
        let args = RunArgs {
            model: "llama3.2".to_string(),
            prompt: String::new(),
            ..Default::default()
        };

        let result = run(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[tokio::test]
    #[ignore = "requires running daemon"]
    async fn test_run_with_system_prompt() {
        let args = RunArgs {
            model: "llama3.2".to_string(),
            prompt: "What are you?".to_string(),
            system: Some("You are a helpful pirate.".to_string()),
            ..Default::default()
        };

        let result = run(args).await;
        // Should succeed if daemon and model are available
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Daemon"));
    }
}
