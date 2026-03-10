//! Model CLI commands.
//!
//! Manage local LLM models via HuggingFace storage.
//! Search and discover models from the SuperNovae registry.
//!
//! NOTE: Inference commands (load, unload, run, status) were removed in v0.17.0.
//! Use Nika for model inference with native mistral.rs runtime.

use crate::error::{Result, SpnError};
use crate::interop::model_registry::ModelRegistry;
use crate::prompts;
use crate::ux::design_system as ds;
use crate::ux::progress::transforming_spinner;
use crate::ModelCommands;
use dialoguer::Confirm;
use spn_client::{Request, Response, SpnClient};

pub async fn run(command: ModelCommands) -> Result<()> {
    match command {
        ModelCommands::List { json } => list(json).await,
        ModelCommands::Pull { name } => {
            let name = match name {
                Some(n) => n,
                None => prompts::select_model()?,
            };
            pull(&name).await
        }
        ModelCommands::Remove { name, yes } => {
            let name = match name {
                Some(n) => n,
                None => prompts::select_model()?,
            };
            delete(&name, yes).await
        }
        ModelCommands::Search { query, category } => search(&query, category.as_deref()).await,
        ModelCommands::Info { name, json } => info(&name, json).await,
        ModelCommands::Recommend { use_case } => recommend(use_case.as_deref()).await,
    }
}

// ============================================================================
// List Models
// ============================================================================

async fn list(json: bool) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    let response = client
        .send_request(Request::ModelList)
        .await
        .map_err(|e| anyhow::anyhow!("Daemon request failed: {}", e))?;

    match response {
        Response::Models { models } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&models)?);
                return Ok(());
            }

            if models.is_empty() {
                println!("{}", ds::warning("No models installed."));
                println!();
                println!("Get started:");
                println!("  {} spn model pull llama3.2", ds::primary("•"));
                println!("  {} spn model pull mistral:7b", ds::primary("•"));
                return Ok(());
            }

            println!("{}", ds::highlight("Installed Models"));
            println!();

            // Header
            println!(
                "  {:<30} {:>10} {:>10}",
                ds::muted("NAME"),
                ds::muted("SIZE"),
                ds::muted("QUANT")
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

        Response::Error { message } => {
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

// ============================================================================
// Pull Model
// ============================================================================

async fn pull(name: &str) -> Result<()> {
    let mut client = connect_to_daemon().await?;

    let spinner = transforming_spinner(&format!("Pulling model: {} (this may take a while)", name));

    let response = client
        .send_request(Request::ModelPull {
            name: name.to_string(),
        })
        .await;

    match response {
        Ok(Response::Success { success: true }) => {
            spinner.finish_success(&format!("Model '{}' pulled successfully", name));
        }
        Ok(Response::Success { success: false }) => {
            spinner.finish_error("Pull failed");
            return Err(SpnError::CommandFailed("Pull failed".to_string()));
        }
        Ok(Response::Error { message }) => {
            spinner.finish_error(&format!("Pull failed: {}", message));
            return Err(SpnError::CommandFailed(message));
        }
        Ok(_) => {
            spinner.finish_error("Unexpected response from daemon");
            return Err(SpnError::CommandFailed(
                "Unexpected response from daemon".to_string(),
            ));
        }
        Err(e) => {
            spinner.finish_error(&format!("Daemon request failed: {}", e));
            return Err(SpnError::CommandFailed(format!(
                "Daemon request failed: {}",
                e
            )));
        }
    }

    Ok(())
}

// NOTE: Load/Unload commands removed in v0.17.0 (inference moved to Nika)
// See: ADR-008 - Inference Architecture Refactor

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

    let spinner = transforming_spinner(&format!("Deleting model: {}", name));

    let response = client
        .send_request(Request::ModelDelete {
            name: name.to_string(),
        })
        .await;

    match response {
        Ok(Response::Success { success: true }) => {
            spinner.finish_success(&format!("Model '{}' deleted", name));
        }
        Ok(Response::Error { message }) => {
            spinner.finish_error(&format!("Delete failed: {}", message));
            return Err(SpnError::CommandFailed(message));
        }
        Ok(_) => {
            spinner.finish_error("Unexpected response from daemon");
            return Err(SpnError::CommandFailed(
                "Unexpected response from daemon".to_string(),
            ));
        }
        Err(e) => {
            spinner.finish_error(&format!("Daemon request failed: {}", e));
            return Err(SpnError::CommandFailed(format!(
                "Daemon request failed: {}",
                e
            )));
        }
    }

    Ok(())
}

// NOTE: Status command removed in v0.17.0 (inference moved to Nika)
// Model memory management is now handled by Nika's native runtime

// ============================================================================
// Helpers
// ============================================================================

async fn connect_to_daemon() -> Result<SpnClient> {
    SpnClient::connect().await.map_err(|_| {
        SpnError::CommandFailed(format!(
            "Daemon is not running\n\nStart the daemon with: {} spn daemon start",
            ds::primary("->")
        ))
    })
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

// ============================================================================
// Search Models (from registry)
// ============================================================================

async fn search(query: &str, category: Option<&str>) -> Result<()> {
    let registry = ModelRegistry::new();

    println!(
        "{} Searching for: {}",
        ds::primary("->"),
        ds::highlight(query)
    );
    println!();

    let results = if let Some(cat) = category {
        // Filter by category first, then search
        let models = registry.list_by_category(cat).await;
        let query_lower = query.to_lowercase();
        models
            .into_iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.ollama_model.to_lowercase().contains(&query_lower)
                    || m.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>()
    } else {
        registry.search(query).await
    };

    if results.is_empty() {
        println!("{}", ds::warning("No models found."));
        println!();
        println!("Try:");
        println!("  {} spn model search coding", ds::primary("•"));
        println!("  {} spn model search --category vision", ds::primary("•"));
        return Ok(());
    }

    println!("{}", ds::highlight("Available Models"));
    println!();

    // Header
    println!(
        "  {:<35} {:<12} {}",
        ds::muted("NAME"),
        ds::muted("CATEGORY"),
        ds::muted("DESCRIPTION")
    );
    println!("  {}", "-".repeat(80));

    for model in &results {
        let desc = model
            .description
            .as_ref()
            .map(|d| {
                if d.len() > 40 {
                    format!("{}...", &d[..37])
                } else {
                    d.clone()
                }
            })
            .unwrap_or_else(|| "-".to_string());

        println!(
            "  {:<35} {:<12} {}",
            ds::primary(&model.ollama_model),
            model.category,
            ds::muted(&desc)
        );
    }

    println!();
    println!("  {} model(s) found", results.len());
    println!();
    println!(
        "  Pull a model: {} spn model pull {}",
        ds::primary("->"),
        results
            .first()
            .map(|m| m.ollama_model.as_str())
            .unwrap_or("llama3.2")
    );

    Ok(())
}

// ============================================================================
// Model Info (local first, then registry)
// ============================================================================

async fn info(name: &str, json_output: bool) -> Result<()> {
    // First, check if model is installed locally
    if let Ok(mut client) = SpnClient::connect().await {
        if let Ok(Response::Models { models }) = client.send_request(Request::ModelList).await {
            // Find matching model (handle tag variations like "llama3.2:1b" vs "llama3.2")
            let local_model = models.iter().find(|m| {
                m.name == name
                    || m.name.starts_with(&format!("{}:", name))
                    || name.starts_with(&format!("{}:", m.name.split(':').next().unwrap_or("")))
            });

            if let Some(model) = local_model {
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&model)?);
                    return Ok(());
                }

                println!("{}", ds::highlight("Local Model Information"));
                println!();
                println!("  {} {}", ds::muted("Name:"), ds::primary(&model.name));
                println!("  {} {}", ds::muted("Size:"), format_size(model.size));

                if let Some(ref quant) = model.quantization {
                    println!("  {} {}", ds::muted("Quantization:"), quant);
                }

                if let Some(ref params) = model.parameters {
                    println!("  {} {}", ds::muted("Parameters:"), params);
                }

                if let Some(ref digest) = model.digest {
                    println!(
                        "  {} {}...",
                        ds::muted("Digest:"),
                        &digest[..12.min(digest.len())]
                    );
                }

                println!();
                println!(
                    "  Load model: {} spn model load {}",
                    ds::primary("->"),
                    model.name
                );

                return Ok(());
            }
        }
    }

    // Fallback to registry lookup
    let registry = ModelRegistry::new();

    let model = registry.get(name).await;

    if let Some(model) = model {
        if json_output {
            let json = serde_json::json!({
                "name": model.name,
                "ollama_model": model.ollama_model,
                "description": model.description,
                "category": model.category,
                "variants": model.variants.iter().map(|v| {
                    serde_json::json!({
                        "name": v.name,
                        "ollama": v.ollama,
                        "size": v.size,
                        "vram": v.vram,
                        "best_for": v.best_for
                    })
                }).collect::<Vec<_>>(),
                "benchmarks": model.benchmarks,
                "capabilities": model.capabilities,
                "recommended_for": model.recommended_for
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
            return Ok(());
        }

        println!("{}", ds::highlight("Model Information"));
        println!();
        println!("  {} {}", ds::muted("Name:"), ds::highlight(&model.name));
        println!(
            "  {} {}",
            ds::muted("Ollama:"),
            ds::primary(&model.ollama_model)
        );
        println!("  {} {}", ds::muted("Category:"), model.category);

        if let Some(desc) = &model.description {
            println!("  {} {}", ds::muted("Description:"), desc);
        }

        if !model.capabilities.is_empty() {
            println!();
            println!("  {}", ds::muted("Capabilities:"));
            for cap in &model.capabilities {
                println!("    {} {}", ds::success("•"), cap);
            }
        }

        if !model.recommended_for.is_empty() {
            println!();
            println!("  {}", ds::muted("Recommended for:"));
            for rec in &model.recommended_for {
                println!("    {} {}", ds::primary("→"), rec);
            }
        }

        if !model.variants.is_empty() {
            println!();
            println!("  {}", ds::muted("Variants:"));
            for var in &model.variants {
                println!(
                    "    {} {} (Size: {}, VRAM: {})",
                    ds::primary("•"),
                    ds::highlight(&var.ollama),
                    var.size,
                    var.vram
                );
                if !var.best_for.is_empty() {
                    println!("      Best for: {}", ds::muted(&var.best_for));
                }
            }
        }

        if !model.benchmarks.is_empty() {
            println!();
            println!("  {}", ds::muted("Benchmarks:"));
            for (name, score) in &model.benchmarks {
                println!("    {} {}: {:.1}", ds::primary("•"), name, score);
            }
        }

        println!();
        println!(
            "  Pull this model: {} spn model pull {}",
            ds::primary("->"),
            model.ollama_model
        );
    } else {
        println!(
            "{} Model '{}' not found in registry",
            ds::warning("!"),
            name
        );
        println!();
        println!("Try:");
        println!("  {} spn model search {}", ds::primary("•"), name);
        println!("  {} spn model list", ds::primary("•"));
    }

    Ok(())
}

// ============================================================================
// Recommend Models
// ============================================================================

async fn recommend(use_case: Option<&str>) -> Result<()> {
    let registry = ModelRegistry::new();

    println!("{}", ds::highlight("Model Recommendations"));
    println!();

    let models = registry.recommend(use_case).await;

    if models.is_empty() {
        println!("{}", ds::warning("No recommendations available."));
        return Ok(());
    }

    if let Some(case) = use_case {
        println!("  For use case: {}", ds::primary(case));
        println!();
    } else {
        println!("  {}", ds::muted("Top models by category:"));
        println!();
    }

    for model in &models {
        let desc = model
            .description
            .as_ref()
            .map(|d| {
                if d.len() > 50 {
                    format!("{}...", &d[..47])
                } else {
                    d.clone()
                }
            })
            .unwrap_or_default();

        println!(
            "  {} {} [{}]",
            ds::success("*"),
            ds::highlight(&model.ollama_model),
            ds::primary(&model.category)
        );
        if !desc.is_empty() {
            println!("    {}", ds::muted(&desc));
        }
        println!();
    }

    println!(
        "  Pull a model: {} spn model pull <model>",
        ds::primary("->")
    );
    println!("  More info: {} spn model info <model>", ds::primary("->"));

    Ok(())
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
