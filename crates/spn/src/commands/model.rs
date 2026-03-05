//! Model CLI commands.
//!
//! Manage local LLM models via the spn daemon + Ollama.

use crate::error::Result;
use crate::ModelCommands;
use colored::Colorize;
use dialoguer::Confirm;
use spn_client::{LoadConfig, Request, Response, SpnClient};

pub async fn run(command: ModelCommands) -> Result<()> {
    match command {
        ModelCommands::List { json, running } => list(json, running).await,
        ModelCommands::Pull { name } => pull(&name).await,
        ModelCommands::Load { name, keep_alive } => load(&name, keep_alive).await,
        ModelCommands::Unload { name } => unload(&name).await,
        ModelCommands::Delete { name, yes } => delete(&name, yes).await,
        ModelCommands::Status { json } => status(json).await,
    }
}

// ============================================================================
// List Models
// ============================================================================

async fn list(json: bool, running_only: bool) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    let request = if running_only {
        Request::ModelStatus
    } else {
        Request::ModelList
    };

    let response = client
        .send_request(request)
        .await
        .map_err(|e| anyhow::anyhow!("Daemon request failed: {}", e))?;

    match response {
        Response::Models { models } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&models)?);
                return Ok(());
            }

            if models.is_empty() {
                println!("{}", "No models installed.".yellow());
                println!();
                println!("Get started:");
                println!("  {} spn model pull llama3.2", "•".cyan());
                println!("  {} spn model pull mistral:7b", "•".cyan());
                return Ok(());
            }

            println!("{}", "Installed Models".bold());
            println!();

            // Header
            println!(
                "  {:<30} {:>10} {:>10}",
                "NAME".dimmed(),
                "SIZE".dimmed(),
                "QUANT".dimmed()
            );
            println!("  {}", "-".repeat(52));

            // Models
            for model in &models {
                let size = format_size(model.size);
                let quant = model.quantization.as_deref().unwrap_or("-");
                println!("  {:<30} {:>10} {:>10}", model.name, size, quant);
            }

            println!();
            println!("  {} model(s) installed", models.len());
        }

        Response::RunningModels { running } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&running)?);
                return Ok(());
            }

            if running.is_empty() {
                println!("{}", "No models currently loaded.".yellow());
                return Ok(());
            }

            println!("{}", "Running Models".bold());
            println!();

            for model in &running {
                let vram = model
                    .vram_used
                    .map(|v| format!("{:.1} GB VRAM", v as f64 / 1_073_741_824.0))
                    .unwrap_or_else(|| "-".to_string());

                println!("  {} {} ({})", "*".green(), model.name, vram);
            }
        }

        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }

        _ => {
            eprintln!("{}", "Unexpected response from daemon".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Pull Model
// ============================================================================

async fn pull(name: &str) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    println!("{} Pulling model: {}", "->".cyan(), name.bold());
    println!("   This may take a while...");

    let response = client
        .send_request(Request::ModelPull {
            name: name.to_string(),
        })
        .await
        .map_err(|e| anyhow::anyhow!("Daemon request failed: {}", e))?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' pulled successfully", "*".green(), name);
        }
        Response::Success { success: false } => {
            eprintln!("{} Pull failed", "x".red());
            std::process::exit(1);
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Load Model
// ============================================================================

async fn load(name: &str, keep_alive: bool) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    println!("{} Loading model: {}", "->".cyan(), name.bold());

    let config = if keep_alive {
        Some(LoadConfig {
            gpu_ids: vec![],
            gpu_layers: -1,
            context_size: None,
            keep_alive: true,
        })
    } else {
        None
    };

    let response = client
        .send_request(Request::ModelLoad {
            name: name.to_string(),
            config,
        })
        .await
        .map_err(|e| anyhow::anyhow!("Daemon request failed: {}", e))?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' loaded", "*".green(), name);
            if keep_alive {
                println!("   Model will stay loaded until manually unloaded");
            }
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Unload Model
// ============================================================================

async fn unload(name: &str) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    println!("{} Unloading model: {}", "->".cyan(), name.bold());

    let response = client
        .send_request(Request::ModelUnload {
            name: name.to_string(),
        })
        .await
        .map_err(|e| anyhow::anyhow!("Daemon request failed: {}", e))?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' unloaded", "*".green(), name);
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Delete Model
// ============================================================================

async fn delete(name: &str, skip_confirm: bool) -> Result<()> {
    if !skip_confirm {
        let confirm = Confirm::new()
            .with_prompt(format!("Delete model '{}'?", name))
            .default(false)
            .interact()
            .map_err(|e| anyhow::anyhow!("Failed to get confirmation: {}", e))?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let mut client = connect_to_daemon().await?;

    println!("{} Deleting model: {}", "->".cyan(), name.bold());

    let response = client
        .send_request(Request::ModelDelete {
            name: name.to_string(),
        })
        .await
        .map_err(|e| anyhow::anyhow!("Daemon request failed: {}", e))?;

    match response {
        Response::Success { success: true } => {
            println!("{} Model '{}' deleted", "*".green(), name);
        }
        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }
        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Status
// ============================================================================

async fn status(json: bool) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    let response = client
        .send_request(Request::ModelStatus)
        .await
        .map_err(|e| anyhow::anyhow!("Daemon request failed: {}", e))?;

    match response {
        Response::RunningModels { running } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&running)?);
                return Ok(());
            }

            println!("{}", "Model Status".bold());
            println!();

            if running.is_empty() {
                println!("  {} No models loaded", "o".dimmed());
                println!();
                println!(
                    "  Load a model with: {} spn model load llama3.2",
                    "->".cyan()
                );
            } else {
                println!(
                    "  {:<30} {:>12}",
                    "MODEL".dimmed(),
                    "VRAM".dimmed()
                );
                println!("  {}", "-".repeat(44));

                let mut total_vram: u64 = 0;

                for model in &running {
                    let vram = model.vram_used.unwrap_or(0);
                    total_vram += vram;

                    let vram_str = if vram > 0 {
                        format!("{:.1} GB", vram as f64 / 1_073_741_824.0)
                    } else {
                        "-".to_string()
                    };

                    println!(
                        "  {} {:<28} {:>12}",
                        "*".green(),
                        model.name,
                        vram_str
                    );
                }

                if total_vram > 0 {
                    println!("  {}", "-".repeat(44));
                    println!(
                        "  {:<30} {:>12}",
                        "Total VRAM",
                        format!("{:.1} GB", total_vram as f64 / 1_073_741_824.0)
                    );
                }
            }
        }

        Response::Error { message } => {
            eprintln!("{} {}", "Error:".red(), message);
            std::process::exit(1);
        }

        _ => {
            eprintln!("{}", "Unexpected response".red());
            std::process::exit(1);
        }
    }

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

async fn connect_to_daemon() -> Result<SpnClient> {
    match SpnClient::connect().await {
        Ok(client) => Ok(client),
        Err(_) => {
            eprintln!("{} Daemon is not running", "x".red());
            eprintln!();
            eprintln!("Start the daemon with: {} spn daemon start", "->".cyan());
            std::process::exit(1);
        }
    }
}

fn format_size(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
        assert_eq!(format_size(4_500_000_000), "4.2 GB");
        assert_eq!(format_size(500_000_000), "477 MB");
        assert_eq!(format_size(1000), "1000 B");
    }

    #[test]
    fn test_format_size_zero() {
        assert_eq!(format_size(0), "0 B");
    }
}
