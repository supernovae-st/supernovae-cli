# Plan 2: Model Management (spn v0.10)

**Date:** 2026-03-05
**Version:** v1.0
**Status:** DRAFT
**Depends On:** Plan 1 (Secrets Architecture)
**Timeline:** 3-4 days

---

## Executive Summary

Ce plan ajoute la gestion complète des modèles locaux (Ollama) à spn, permettant une UX unifiée où `spn model add llama3.2` télécharge, configure et lance automatiquement un modèle utilisable par nika.

---

## 1. User Stories

### US-1: Découverte de modèles
```
En tant que développeur,
Je veux chercher des modèles disponibles,
Pour trouver celui qui convient à mon use case.

$ spn model search code
┌────────────────────────────────────────────────────────────────┐
│  MODELS MATCHING "code"                                        │
├──────────────────┬──────────┬──────────┬───────────────────────┤
│  Name            │  Size    │  Quant   │  Description          │
├──────────────────┼──────────┼──────────┼───────────────────────┤
│  codellama       │  7.2GB   │  Q4_K_M  │  Code generation      │
│  codellama:13b   │  13.4GB  │  Q4_K_M  │  Code gen (larger)    │
│  deepseek-coder  │  6.7GB   │  Q4_K_M  │  DeepSeek for code    │
│  starcoder2      │  8.1GB   │  Q4_K_M  │  StarCoder 2          │
└──────────────────┴──────────┴──────────┴───────────────────────┘
```

### US-2: Installation one-liner
```
En tant que développeur,
Je veux installer un modèle en une commande,
Pour l'utiliser immédiatement dans mes workflows.

$ spn model add llama3.2
✓ Ollama detected (v0.1.32)
⬇ Pulling llama3.2...
  ████████████████████████░░░░░░ 78% (3.7GB/4.7GB) 2m remaining
✓ Model llama3.2 installed (4.7GB)
✓ Added to ~/.spn/config.yaml
✓ Auto-starting model server...
✓ Ready at http://localhost:11434

  Use in nika:
    providers:
      default: ollama/llama3.2
```

### US-3: Gestion du lifecycle
```
En tant que développeur,
Je veux démarrer/arrêter mes modèles,
Pour gérer mes ressources (RAM/GPU).

$ spn model list
┌────────────────────────────────────────────────────────────────┐
│  INSTALLED MODELS                                              │
├──────────────────┬──────────┬──────────┬───────────┬──────────┤
│  Name            │  Size    │  Status  │  Memory   │  Default │
├──────────────────┼──────────┼──────────┼───────────┼──────────┤
│  llama3.2        │  4.7GB   │  ● running│  5.2GB   │  ✓       │
│  codellama       │  7.2GB   │  ○ stopped│  -       │          │
│  mistral         │  4.1GB   │  ○ stopped│  -       │          │
└──────────────────┴──────────┴──────────┴───────────┴──────────┘

$ spn model stop llama3.2
✓ Stopped llama3.2 (freed 5.2GB RAM)

$ spn model start codellama
✓ Starting codellama...
✓ Ready at http://localhost:11434
```

### US-4: Nika integration seamless
```
En tant que développeur,
Je veux utiliser mes modèles dans nika sans config,
Pour un workflow fluide.

# workflow.nika.yaml
providers:
  default: ollama/llama3.2  # Automatiquement résolu via spn daemon

tasks:
  - id: generate
    infer: "Explain this code"
    # Utilise llama3.2 sur localhost:11434
```

---

## 2. Architecture

### 2.1 Nouveau Crate: spn-ollama

```
crates/spn-ollama/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── client.rs       # Ollama HTTP API client
│   ├── models.rs       # Model metadata types
│   ├── pull.rs         # Download with progress
│   ├── serve.rs        # Process management
│   └── error.rs
```

```toml
# Cargo.toml
[package]
name = "spn-ollama"
version = "0.1.0"
description = "Ollama integration for SuperNovae"

[dependencies]
spn-core = { path = "../spn-core" }
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1.36", features = ["process", "io-util"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
futures-util = "0.3"
indicatif = "0.17"  # Progress bars
thiserror = "2"
tracing = "0.1"
```

### 2.2 Ollama HTTP API Client

```rust
// src/client.rs

use reqwest::Client;
use serde::{Deserialize, Serialize};

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new() -> Self {
        Self::with_url(DEFAULT_OLLAMA_URL)
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: url.into(),
        }
    }

    /// List installed models
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let resp = self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        let data: ListModelsResponse = resp.json().await?;
        Ok(data.models)
    }

    /// Pull a model (streaming progress)
    pub async fn pull_model(
        &self,
        name: &str,
        progress: impl Fn(PullProgress),
    ) -> Result<(), OllamaError> {
        let resp = self.client
            .post(format!("{}/api/pull", self.base_url))
            .json(&PullRequest { name: name.to_string(), stream: true })
            .send()
            .await?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if let Ok(progress_data) = serde_json::from_slice::<PullProgress>(&chunk) {
                progress(progress_data);
            }
        }
        Ok(())
    }

    /// Check if model exists locally
    pub async fn has_model(&self, name: &str) -> Result<bool, OllamaError> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == name || m.name.starts_with(&format!("{}:", name))))
    }

    /// Delete a model
    pub async fn delete_model(&self, name: &str) -> Result<(), OllamaError> {
        self.client
            .delete(format!("{}/api/delete", self.base_url))
            .json(&DeleteRequest { name: name.to_string() })
            .send()
            .await?;
        Ok(())
    }

    /// Generate (for testing model works)
    pub async fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaError> {
        let resp = self.client
            .post(format!("{}/api/generate", self.base_url))
            .json(&GenerateRequest {
                model: model.to_string(),
                prompt: prompt.to_string(),
                stream: false,
            })
            .send()
            .await?;

        let data: GenerateResponse = resp.json().await?;
        Ok(data.response)
    }

    /// Health check
    pub async fn is_running(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }
}

// Types
#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PullProgress {
    pub status: String,
    pub total: Option<u64>,
    pub completed: Option<u64>,
}

impl PullProgress {
    pub fn percent(&self) -> Option<f64> {
        match (self.completed, self.total) {
            (Some(c), Some(t)) if t > 0 => Some((c as f64 / t as f64) * 100.0),
            _ => None,
        }
    }
}
```

### 2.3 Process Management

```rust
// src/serve.rs

use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, Duration};

pub struct OllamaServer {
    process: Option<Child>,
    port: u16,
}

impl OllamaServer {
    /// Start Ollama serve process
    pub fn start() -> Result<Self, OllamaError> {
        Self::start_on_port(11434)
    }

    pub fn start_on_port(port: u16) -> Result<Self, OllamaError> {
        // Check if already running
        if Self::is_port_in_use(port) {
            return Ok(Self { process: None, port });
        }

        let process = Command::new("ollama")
            .arg("serve")
            .env("OLLAMA_HOST", format!("0.0.0.0:{}", port))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let mut server = Self {
            process: Some(process),
            port,
        };

        // Wait for ready
        server.wait_ready(Duration::from_secs(30))?;

        Ok(server)
    }

    /// Wait for server to be ready
    async fn wait_ready(&self, timeout: Duration) -> Result<(), OllamaError> {
        let client = OllamaClient::with_url(format!("http://localhost:{}", self.port));
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if client.is_running().await {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }

        Err(OllamaError::Timeout("Server failed to start".into()))
    }

    /// Stop the server
    pub fn stop(&mut self) -> Result<(), OllamaError> {
        if let Some(ref mut process) = self.process {
            process.kill()?;
            process.wait()?;
            self.process = None;
        }
        Ok(())
    }

    /// Check if a port is in use
    fn is_port_in_use(port: u16) -> bool {
        std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
    }
}

impl Drop for OllamaServer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
```

### 2.4 Daemon Model Manager

```rust
// crates/spn/src/daemon/models.rs

use std::collections::HashMap;
use parking_lot::RwLock;
use spn_ollama::{OllamaClient, OllamaServer, ModelInfo};

pub struct ModelManager {
    /// Ollama client
    client: OllamaClient,

    /// Ollama server process (if we started it)
    server: RwLock<Option<OllamaServer>>,

    /// Cached model list
    models: RwLock<Vec<ModelInfo>>,

    /// Default model name
    default_model: RwLock<Option<String>>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            client: OllamaClient::new(),
            server: RwLock::new(None),
            models: RwLock::new(Vec::new()),
            default_model: RwLock::new(None),
        }
    }

    /// Initialize: check Ollama, start if needed
    pub async fn init(&self) -> Result<(), DaemonError> {
        // Check if Ollama is installed
        if !Self::is_ollama_installed() {
            return Err(DaemonError::OllamaNotInstalled);
        }

        // Start server if not running
        if !self.client.is_running().await {
            let server = OllamaServer::start()?;
            *self.server.write() = Some(server);
        }

        // Refresh model list
        self.refresh_models().await?;

        Ok(())
    }

    /// List installed models
    pub async fn list(&self) -> Result<Vec<ModelInfo>, DaemonError> {
        self.refresh_models().await?;
        Ok(self.models.read().clone())
    }

    /// Pull a new model
    pub async fn pull(
        &self,
        name: &str,
        progress_callback: impl Fn(f64, String),
    ) -> Result<(), DaemonError> {
        self.client.pull_model(name, |p| {
            if let Some(pct) = p.percent() {
                progress_callback(pct, p.status);
            }
        }).await?;

        self.refresh_models().await?;
        Ok(())
    }

    /// Remove a model
    pub async fn remove(&self, name: &str) -> Result<(), DaemonError> {
        self.client.delete_model(name).await?;
        self.refresh_models().await?;
        Ok(())
    }

    /// Get model URL (for nika to use)
    pub fn get_model_url(&self, name: &str) -> Option<String> {
        let models = self.models.read();
        if models.iter().any(|m| m.name == name || m.name.starts_with(&format!("{}:", name))) {
            Some(format!("http://localhost:11434"))
        } else {
            None
        }
    }

    /// Set default model
    pub fn set_default(&self, name: &str) {
        *self.default_model.write() = Some(name.to_string());
    }

    /// Get default model
    pub fn get_default(&self) -> Option<String> {
        self.default_model.read().clone()
    }

    /// Refresh model list from Ollama
    async fn refresh_models(&self) -> Result<(), DaemonError> {
        let models = self.client.list_models().await?;
        *self.models.write() = models;
        Ok(())
    }

    /// Check if Ollama is installed
    fn is_ollama_installed() -> bool {
        which::which("ollama").is_ok()
    }

    /// Shutdown
    pub fn shutdown(&self) {
        if let Some(mut server) = self.server.write().take() {
            let _ = server.stop();
        }
    }
}
```

### 2.5 IPC Protocol Extension

```rust
// crates/spn-client/src/protocol.rs

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    // Existing
    Ping,
    GetSecret { provider: String },
    HasSecret { provider: String },
    ListProviders,

    // NEW: Model management
    ModelList,
    ModelPull { name: String },
    ModelRemove { name: String },
    ModelStatus { name: String },
    ModelGetUrl { name: String },
    ModelSetDefault { name: String },
    ModelGetDefault,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    // Existing
    Pong { version: String },
    Secret { value: String },
    Exists { exists: bool },
    Providers { providers: Vec<String> },
    Error { message: String },

    // NEW: Model responses
    Models { models: Vec<ModelInfo> },
    ModelUrl { url: Option<String> },
    ModelDefault { name: Option<String> },
    Progress { percent: f64, status: String },
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub size_human: String,
    pub status: ModelStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ModelStatus {
    Running,
    Stopped,
    Pulling,
}
```

### 2.6 spn-client Model API

```rust
// crates/spn-client/src/models.rs

impl SpnClient {
    /// List installed models
    pub async fn model_list(&mut self) -> Result<Vec<ModelInfo>, Error> {
        self.send(Request::ModelList).await?;
        match self.recv().await? {
            Response::Models { models } => Ok(models),
            Response::Error { message } => Err(Error::Server(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Pull a model (returns immediately, progress via callback)
    pub async fn model_pull(
        &mut self,
        name: &str,
        mut progress: impl FnMut(f64, String),
    ) -> Result<(), Error> {
        self.send(Request::ModelPull { name: name.to_string() }).await?;

        loop {
            match self.recv().await? {
                Response::Progress { percent, status } => {
                    progress(percent, status);
                }
                Response::Ok => return Ok(()),
                Response::Error { message } => return Err(Error::Server(message)),
                _ => {}
            }
        }
    }

    /// Remove a model
    pub async fn model_remove(&mut self, name: &str) -> Result<(), Error> {
        self.send(Request::ModelRemove { name: name.to_string() }).await?;
        match self.recv().await? {
            Response::Ok => Ok(()),
            Response::Error { message } => Err(Error::Server(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Get URL for a model
    pub async fn model_url(&mut self, name: &str) -> Result<Option<String>, Error> {
        self.send(Request::ModelGetUrl { name: name.to_string() }).await?;
        match self.recv().await? {
            Response::ModelUrl { url } => Ok(url),
            Response::Error { message } => Err(Error::Server(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Set default model
    pub async fn model_set_default(&mut self, name: &str) -> Result<(), Error> {
        self.send(Request::ModelSetDefault { name: name.to_string() }).await?;
        match self.recv().await? {
            Response::Ok => Ok(()),
            Response::Error { message } => Err(Error::Server(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    /// Get default model
    pub async fn model_default(&mut self) -> Result<Option<String>, Error> {
        self.send(Request::ModelGetDefault).await?;
        match self.recv().await? {
            Response::ModelDefault { name } => Ok(name),
            Response::Error { message } => Err(Error::Server(message)),
            _ => Err(Error::UnexpectedResponse),
        }
    }
}
```

---

## 3. CLI Commands

### 3.1 Command Structure

```rust
// crates/spn/src/commands/model.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct ModelCommand {
    #[command(subcommand)]
    command: ModelSubcommand,
}

#[derive(Subcommand)]
pub enum ModelSubcommand {
    /// Search for models on Ollama registry
    Search {
        /// Search query
        query: String,
    },

    /// Show model details
    Info {
        /// Model name
        name: String,
    },

    /// Install a model
    Add {
        /// Model name (e.g., llama3.2, codellama:13b)
        name: String,

        /// Don't auto-start after install
        #[arg(long)]
        no_start: bool,
    },

    /// Remove a model
    Remove {
        /// Model name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// List installed models
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Start model serving
    Start {
        /// Model name (or "all")
        name: Option<String>,
    },

    /// Stop model serving
    Stop {
        /// Model name (or "all")
        name: Option<String>,
    },

    /// Show model status
    Status,

    /// Set default model
    Default {
        /// Model name
        name: String,
    },

    /// Run a quick test prompt
    Test {
        /// Model name
        name: String,

        /// Test prompt
        #[arg(short, long, default_value = "Say hello in one word")]
        prompt: String,
    },
}
```

### 3.2 Implementation: model add

```rust
// crates/spn/src/commands/model.rs

pub async fn handle_add(name: &str, no_start: bool) -> Result<()> {
    let mut client = SpnClient::connect().await
        .context("Failed to connect to daemon. Run: spn daemon start")?;

    // Check if Ollama is installed
    if !which::which("ollama").is_ok() {
        println!("{}", "Ollama not found!".red());
        println!("Install with: {}", "brew install ollama".cyan());
        return Ok(());
    }

    // Progress bar
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} {msg}\n{wide_bar:.cyan/blue} {percent}%")?
            .progress_chars("█▓░")
    );
    pb.set_message(format!("Pulling {}...", name));

    // Pull model
    client.model_pull(name, |percent, status| {
        pb.set_position(percent as u64);
        pb.set_message(status);
    }).await?;

    pb.finish_with_message(format!("✓ Model {} installed", name.green()));

    // Auto-start unless --no-start
    if !no_start {
        println!("Starting model server...");
        client.model_start(name).await?;

        let url = client.model_url(name).await?.unwrap_or_default();
        println!("✓ Ready at {}", url.cyan());
    }

    // Show usage hint
    println!();
    println!("Use in nika workflow:");
    println!("  {}", "providers:".dimmed());
    println!("    {}: {}", "default".dimmed(), format!("ollama/{}", name).green());

    Ok(())
}
```

### 3.3 Implementation: model list

```rust
pub async fn handle_list(json: bool) -> Result<()> {
    let mut client = SpnClient::connect().await?;
    let models = client.model_list().await?;
    let default = client.model_default().await?.unwrap_or_default();

    if json {
        println!("{}", serde_json::to_string_pretty(&models)?);
        return Ok(());
    }

    if models.is_empty() {
        println!("No models installed.");
        println!("Install one with: {}", "spn model add llama3.2".cyan());
        return Ok(());
    }

    // Table header
    println!("┌{:─<60}┐", "");
    println!("│ {:^58} │", "INSTALLED MODELS");
    println!("├{:─<18}┬{:─<10}┬{:─<10}┬{:─<10}┬{:─<8}┤", "", "", "", "", "");
    println!("│ {:^16} │ {:^8} │ {:^8} │ {:^8} │ {:^6} │",
        "Name", "Size", "Status", "Memory", "Default");
    println!("├{:─<18}┼{:─<10}┼{:─<10}┼{:─<10}┼{:─<8}┤", "", "", "", "", "");

    for model in &models {
        let is_default = model.name == default;
        let status_icon = match model.status {
            ModelStatus::Running => "● running".green(),
            ModelStatus::Stopped => "○ stopped".dimmed(),
            ModelStatus::Pulling => "◐ pulling".yellow(),
        };
        let default_mark = if is_default { "✓" } else { "" };

        println!("│ {:16} │ {:>8} │ {:^8} │ {:>8} │ {:^6} │",
            model.name,
            model.size_human,
            status_icon,
            "-", // TODO: memory usage
            default_mark,
        );
    }

    println!("└{:─<18}┴{:─<10}┴{:─<10}┴{:─<10}┴{:─<8}┘", "", "", "", "", "");

    Ok(())
}
```

---

## 4. Nika Integration

### 4.1 Provider Resolution

```rust
// nika/tools/nika/src/runtime/providers.rs

use spn_client::SpnClient;

pub async fn resolve_provider(spec: &str) -> Result<ProviderConfig, Error> {
    // Parse provider spec: "ollama/llama3.2" or "anthropic"
    if spec.starts_with("ollama/") {
        let model_name = spec.strip_prefix("ollama/").unwrap();
        resolve_ollama_provider(model_name).await
    } else {
        resolve_cloud_provider(spec).await
    }
}

async fn resolve_ollama_provider(model: &str) -> Result<ProviderConfig, Error> {
    let mut client = SpnClient::connect_with_fallback().await?;

    // Get model URL from daemon
    let url = client.model_url(model).await?
        .ok_or_else(|| Error::ModelNotFound(model.to_string()))?;

    Ok(ProviderConfig {
        provider_type: ProviderType::Ollama,
        model: model.to_string(),
        base_url: url,
        api_key: None, // Ollama doesn't need API key
    })
}

async fn resolve_cloud_provider(provider: &str) -> Result<ProviderConfig, Error> {
    let mut client = SpnClient::connect_with_fallback().await?;

    // Get API key from daemon
    let api_key = client.get_secret(provider).await?;

    Ok(ProviderConfig {
        provider_type: ProviderType::from_name(provider),
        model: get_default_model(provider),
        base_url: get_api_url(provider),
        api_key: Some(api_key),
    })
}
```

### 4.2 rig-core Integration

```rust
// nika/tools/nika/src/runtime/llm.rs

use rig_core::providers::{anthropic, openai, ollama};

pub fn create_agent(config: &ProviderConfig) -> Result<impl Agent, Error> {
    match config.provider_type {
        ProviderType::Ollama => {
            let client = ollama::Client::new(&config.base_url);
            let agent = client
                .agent(&config.model)
                .build();
            Ok(agent)
        }
        ProviderType::Anthropic => {
            let api_key = config.api_key.as_ref().unwrap();
            let client = anthropic::Client::new(api_key.expose_secret());
            let agent = client
                .agent(&config.model)
                .build();
            Ok(agent)
        }
        // ... other providers
    }
}
```

### 4.3 Workflow Example

```yaml
# workflow.nika.yaml

name: hybrid-workflow
description: Uses both local and cloud models

providers:
  local: ollama/llama3.2      # Local model via spn
  cloud: anthropic            # Cloud API via spn secrets

tasks:
  # Fast local processing
  - id: extract
    infer: "Extract key points from this text"
    provider: local
    input: $document

  # Complex reasoning with cloud
  - id: analyze
    infer: "Provide deep analysis of these points"
    provider: cloud
    input: $extract.output

  # Back to local for formatting
  - id: format
    infer: "Format this analysis as markdown"
    provider: local
    input: $analyze.output
```

---

## 5. Configuration

### 5.1 Config Schema

```yaml
# ~/.spn/config.yaml

version: 2

models:
  # Default model for nika workflows
  default: llama3.2

  # Auto-start these models when daemon starts
  auto_start:
    - llama3.2

  # Ollama settings
  ollama:
    host: "127.0.0.1"
    port: 11434
    # GPU settings (optional)
    gpu_layers: -1  # -1 = auto

  # Installed models (auto-populated)
  installed:
    llama3.2:
      size: 4.7GB
      added: "2026-03-05T10:30:00Z"
    codellama:
      size: 7.2GB
      added: "2026-03-05T11:00:00Z"
```

### 5.2 State File

```yaml
# ~/.spn/state.yaml (runtime state, not user-edited)

models:
  ollama_pid: 12345
  running:
    - name: llama3.2
      loaded_at: "2026-03-05T10:35:00Z"
      memory_mb: 5200
```

---

## 6. Testing

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_name() {
        assert_eq!(parse_model_name("llama3.2"), ("llama3.2", None));
        assert_eq!(parse_model_name("llama3.2:7b"), ("llama3.2", Some("7b")));
        assert_eq!(parse_model_name("codellama:13b-instruct"), ("codellama", Some("13b-instruct")));
    }

    #[test]
    fn test_size_human() {
        assert_eq!(size_human(1024 * 1024 * 1024 * 4), "4.0GB");
        assert_eq!(size_human(1024 * 1024 * 500), "500MB");
    }
}
```

### 6.2 Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires Ollama installed
async fn test_model_lifecycle() {
    let client = OllamaClient::new();

    // Skip if Ollama not running
    if !client.is_running().await {
        return;
    }

    // Pull tiny model for testing
    client.pull_model("tinyllama", |_| {}).await.unwrap();

    // Verify it's listed
    let models = client.list_models().await.unwrap();
    assert!(models.iter().any(|m| m.name.contains("tinyllama")));

    // Test generation
    let response = client.generate("tinyllama", "Say hi").await.unwrap();
    assert!(!response.is_empty());

    // Cleanup
    client.delete_model("tinyllama").await.unwrap();
}
```

### 6.3 E2E Test

```bash
#!/bin/bash
# test-model-e2e.sh

set -e

echo "=== E2E Test: Model Management ==="

# Ensure daemon is running
spn daemon start

# Pull a small model
echo "Pulling tinyllama..."
spn model add tinyllama

# Check it's listed
spn model list | grep -q "tinyllama"

# Test via nika workflow
cat > /tmp/test-model.nika.yaml << 'EOF'
name: test-local-model
providers:
  default: ollama/tinyllama
tasks:
  - id: test
    infer: "Say hello"
EOF

nika run /tmp/test-model.nika.yaml

# Cleanup
spn model remove tinyllama --yes

echo "=== E2E Test PASSED ==="
```

---

## 7. Implementation Phases

### Phase 2A: spn-ollama Crate (Day 1)

```
Tasks:
├── [2A.1] Create crates/spn-ollama/
├── [2A.2] Implement OllamaClient (HTTP API)
├── [2A.3] Implement OllamaServer (process management)
├── [2A.4] Add progress streaming for pull
├── [2A.5] Write unit tests
└── [2A.6] Verify with manual Ollama testing
```

### Phase 2B: Daemon Integration (Day 2)

```
Tasks:
├── [2B.1] Add ModelManager to daemon
├── [2B.2] Extend IPC protocol
├── [2B.3] Add model commands to spn-client
├── [2B.4] Update daemon startup (auto-start models)
└── [2B.5] Test IPC flow
```

### Phase 2C: CLI Commands (Day 2)

```
Tasks:
├── [2C.1] Add model subcommand to CLI
├── [2C.2] Implement: add, remove, list, status
├── [2C.3] Implement: start, stop, default
├── [2C.4] Add progress bars and nice output
└── [2C.5] Test all commands
```

### Phase 2D: Nika Integration (Day 3)

```
Tasks:
├── [2D.1] Add ollama provider resolution
├── [2D.2] Update rig-core integration
├── [2D.3] Test workflows with local models
├── [2D.4] Update documentation
└── [2D.5] E2E test suite
```

---

## 8. Success Criteria

- [ ] `spn model add llama3.2` downloads and auto-starts
- [ ] `spn model list` shows status (running/stopped)
- [ ] `spn model start/stop` works
- [ ] nika can use `ollama/llama3.2` provider
- [ ] Daemon auto-starts configured models
- [ ] Works without daemon (direct Ollama fallback)
- [ ] Progress bar during download
- [ ] All tests pass

---

## 9. Future Enhancements (v0.11+)

- **GPU allocation**: `spn model start llama3.2 --gpu 0`
- **Multiple instances**: Run same model on different ports
- **Model aliases**: `spn model alias fast llama3.2`
- **Model recommendations**: `spn model suggest --task coding`
- **Quantization options**: `spn model add llama3.2 --quant q4_k_m`
- **GGUF support**: Direct GGUF file loading
