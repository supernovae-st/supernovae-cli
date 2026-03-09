# Master Plan Phase C: Hardware-Aware Model Discovery (llmfit-core)

**Version:** 0.18.0
**Status:** Draft
**Author:** Claude + Thibaut
**Date:** 2026-03-09
**Depends on:** Phase A (v0.16.0), Phase B (v0.17.0)

---

## Executive Summary

Phase C adds **hardware-aware model recommendations** using llmfit-core library.
This enables intelligent model selection based on actual hardware capabilities,
not just user preferences.

**IMPORTANT:** This phase uses llmfit as a **library only** (llmfit-core crate).
No TUI will be integrated — all output is CLI/JSON/programmatic.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PHASE C SCOPE                                                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ✅ IN SCOPE                              ❌ OUT OF SCOPE                       │
│  ─────────────────────────────────────    ─────────────────────────────────     │
│  • llmfit-core library integration        • llmfit TUI                          │
│  • Hardware profiling (GPU/CPU/RAM)       • Interactive model browser           │
│  • Model scoring (Quality/Speed/Fit)      • Model training                      │
│  • spn model explore CLI command          • Fine-tuning                         │
│  • spn model recommend CLI command        • Cloud benchmarking                  │
│  • JSON/table output formats              • Model comparison charts             │
│  • Orchestrator scoring integration       • Real-time performance monitoring    │
│  • Hardware-aware intent resolution                                             │
│  • Local benchmark cache                                                        │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Research Summary

### llmfit

**Stats:** 13.6k ⭐ | Hardware-aware model recommendations

**Core Capabilities:**
- `SystemSpecs::detect()` - Hardware profiling
- Model scoring with 4 dimensions: Quality, Speed, Fit, Context
- Local/remote model discovery
- Benchmark database

**Architecture:**
```
llmfit (monorepo)
├── crates/
│   ├── llmfit-core/     ← Library crate (what we use)
│   │   ├── hardware.rs  ← SystemSpecs detection
│   │   ├── scoring.rs   ← ModelScore calculation
│   │   ├── registry.rs  ← Model metadata database
│   │   └── recommend.rs ← Recommendation engine
│   │
│   ├── llmfit-tui/      ← TUI (NOT USED)
│   └── llmfit-cli/      ← CLI binary (NOT USED)
```

**Key APIs:**
```rust
// Hardware detection
let specs = SystemSpecs::detect()?;
println!("GPU: {:?}", specs.gpus);
println!("RAM: {}GB", specs.total_ram_gb);
println!("VRAM: {}GB", specs.total_vram_gb);

// Model scoring
let scorer = ModelScorer::new(&specs);
let score = scorer.score_model(&model_info)?;
println!("Quality: {}/100", score.quality);
println!("Speed: {}/100", score.speed);
println!("Fit: {}/100", score.hardware_fit);
println!("Context: {}/100", score.context_window_score);

// Recommendations
let engine = RecommendationEngine::new(&specs);
let recommendations = engine.recommend(
    ModelIntent::CodeGeneration,
    &constraints,
)?;
```

**Scoring Dimensions:**

| Dimension | Description | Formula |
|-----------|-------------|---------|
| **Quality** | Model capability (benchmarks) | ELO rating normalized 0-100 |
| **Speed** | Inference speed on hardware | tokens/sec × VRAM fit |
| **Fit** | Hardware compatibility | VRAM required vs available |
| **Context** | Context window size | log2(context_len) normalized |

---

## Architecture

### Target State

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PHASE C ARCHITECTURE                                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  User Request                                                                   │
│  "I need a model for code generation, prioritize speed"                         │
│         │                                                                       │
│         ▼                                                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐    │
│  │  ModelOrchestrator (updated)                                           │    │
│  │  ├── resolve_intent_with_hardware(CodeGeneration, prefer_speed)       │    │
│  │  │                                                                     │    │
│  │  │   ┌─────────────────────────────────────────────────────────────┐  │    │
│  │  │   │  RecommendationEngine (llmfit-core)                         │  │    │
│  │  │   │  ├── SystemSpecs::detect() → GPU, RAM, VRAM                 │  │    │
│  │  │   │  ├── ModelScorer::score_model() → Quality/Speed/Fit/Context │  │    │
│  │  │   │  └── rank_models() → sorted recommendations                  │  │    │
│  │  │   └─────────────────────────────────────────────────────────────┘  │    │
│  │  │                                                                     │    │
│  │  └── Returns: @models/codellama:7b (best fit for hardware)            │    │
│  └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                 │
│  CLI Commands:                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────┐    │
│  │  spn model explore                                                     │    │
│  │  ├── Lists all available models with scores                           │    │
│  │  ├── Filters by capability, size, family                              │    │
│  │  └── Output: table, json, yaml                                         │    │
│  │                                                                         │    │
│  │  spn model recommend --intent code-generation --prefer speed          │    │
│  │  ├── Hardware-aware recommendation                                     │    │
│  │  └── Shows top 5 models with reasoning                                 │    │
│  │                                                                         │    │
│  │  spn model benchmark <model>                                           │    │
│  │  ├── Run local benchmark                                               │    │
│  │  └── Store results in ~/.spn/benchmarks/                               │    │
│  └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Task 1: llmfit-core Integration

**Option A: Direct dependency (if MIT/Apache licensed)**

```toml
# crates/spn-backends/Cargo.toml
[dependencies]
llmfit-core = { version = "0.6", optional = true }

[features]
llmfit = ["dep:llmfit-core"]
```

**Option B: Fork and extract (if license incompatible)**

Create `spn-recommend` crate with ported logic:

```
crates/spn-recommend/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── hardware.rs    # SystemSpecs (from llmfit)
    ├── scoring.rs     # ModelScore (from llmfit)
    ├── registry.rs    # Model metadata
    └── recommend.rs   # RecommendationEngine
```

For this plan, we assume **Option A** (direct dependency).

### Task 2: Hardware Profiling Enhancement

```rust
// crates/spn-backends/src/hardware.rs (Phase C additions)

use llmfit_core::SystemSpecs as LlmfitSpecs;

/// Enhanced hardware specs with llmfit integration
#[derive(Debug, Clone)]
pub struct SystemProfile {
    /// Raw hardware specs
    pub specs: HardwareSpecs,

    /// llmfit-compatible profile
    pub llmfit_specs: LlmfitSpecs,

    /// Benchmark results (cached)
    pub benchmarks: Option<BenchmarkResults>,
}

impl SystemProfile {
    /// Detect hardware using both our detection and llmfit's
    pub fn detect() -> Self {
        let specs = HardwareSpecs::detect();

        // llmfit detection (more comprehensive GPU info)
        let llmfit_specs = LlmfitSpecs::detect().unwrap_or_default();

        Self {
            specs,
            llmfit_specs,
            benchmarks: None,
        }
    }

    /// Load cached benchmarks
    pub fn with_benchmarks(mut self) -> Self {
        let cache_path = dirs::data_dir()
            .map(|d| d.join("spn").join("benchmarks.json"))
            .unwrap_or_default();

        if cache_path.exists() {
            if let Ok(data) = std::fs::read_to_string(&cache_path) {
                self.benchmarks = serde_json::from_str(&data).ok();
            }
        }

        self
    }

    /// Effective VRAM (considers unified memory on Mac)
    pub fn effective_vram_gb(&self) -> f64 {
        if self.llmfit_specs.is_apple_silicon() {
            // Unified memory: can use ~75% of RAM for models
            self.specs.ram_gb * 0.75
        } else {
            self.llmfit_specs.total_vram_gb()
        }
    }

    /// Can run this model?
    pub fn can_run(&self, model: &ModelInfo, quant: QuantizationLevel) -> bool {
        let required_vram = model.estimated_vram_gb(quant);
        self.effective_vram_gb() >= required_vram * 1.1  // 10% headroom
    }

    /// Optimal quantization for model
    pub fn optimal_quantization(&self, model: &ModelInfo) -> QuantizationLevel {
        let params_b = model.parameter_size.unwrap_or(7_000_000_000) as f64 / 1e9;
        self.specs.recommend_quantization(params_b)
    }
}

/// Cached benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub models: Vec<ModelBenchmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBenchmark {
    pub model_name: String,
    pub backend: String,
    pub tokens_per_second: f64,
    pub time_to_first_token_ms: u64,
    pub memory_used_gb: f64,
}
```

### Task 3: Model Scoring System

```rust
// crates/spn-backends/src/scoring.rs

use llmfit_core::{ModelScorer as LlmfitScorer, ModelScore as LlmfitScore};
use crate::hardware::SystemProfile;
use spn_core::ModelInfo;

/// Scoring weights for different use cases
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub quality: f64,
    pub speed: f64,
    pub fit: f64,
    pub context: f64,
}

impl ScoringWeights {
    pub fn balanced() -> Self {
        Self { quality: 0.3, speed: 0.25, fit: 0.25, context: 0.2 }
    }

    pub fn quality_first() -> Self {
        Self { quality: 0.5, speed: 0.15, fit: 0.2, context: 0.15 }
    }

    pub fn speed_first() -> Self {
        Self { quality: 0.2, speed: 0.45, fit: 0.2, context: 0.15 }
    }

    pub fn fit_first() -> Self {
        Self { quality: 0.2, speed: 0.2, fit: 0.45, context: 0.15 }
    }

    pub fn from_preference(prefer: &str) -> Self {
        match prefer {
            "quality" => Self::quality_first(),
            "speed" => Self::speed_first(),
            "fit" | "hardware" => Self::fit_first(),
            _ => Self::balanced(),
        }
    }
}

/// Comprehensive model score
#[derive(Debug, Clone)]
pub struct ModelScore {
    /// Raw quality score (0-100) - benchmark performance
    pub quality: f64,

    /// Speed score (0-100) - tokens/sec on this hardware
    pub speed: f64,

    /// Hardware fit (0-100) - VRAM compatibility
    pub fit: f64,

    /// Context window score (0-100) - context length
    pub context: f64,

    /// Overall weighted score
    pub overall: f64,

    /// Reasoning for the score
    pub reasoning: Vec<String>,
}

impl ModelScore {
    pub fn compute(
        model: &ModelInfo,
        profile: &SystemProfile,
        weights: &ScoringWeights,
        benchmarks: Option<&ModelBenchmark>,
    ) -> Self {
        let mut reasoning = Vec::new();

        // Quality: from external benchmarks or estimates
        let quality = Self::compute_quality(model, &mut reasoning);

        // Speed: from local benchmarks or estimates
        let speed = Self::compute_speed(model, profile, benchmarks, &mut reasoning);

        // Fit: hardware compatibility
        let fit = Self::compute_fit(model, profile, &mut reasoning);

        // Context: context window size
        let context = Self::compute_context(model, &mut reasoning);

        // Overall weighted score
        let overall = quality * weights.quality
            + speed * weights.speed
            + fit * weights.fit
            + context * weights.context;

        Self { quality, speed, fit, context, overall, reasoning }
    }

    fn compute_quality(model: &ModelInfo, reasoning: &mut Vec<String>) -> f64 {
        // Use llmfit's benchmark database if available
        // Otherwise estimate from parameter count and family
        let params_b = model.parameter_size.unwrap_or(7_000_000_000) as f64 / 1e9;

        let base_score = match model.family.as_deref() {
            Some("claude") => 95.0,
            Some("gpt") => 92.0,
            Some("llama") => 75.0 + (params_b.log2() * 3.0).min(20.0),
            Some("mistral") => 78.0 + (params_b.log2() * 3.0).min(17.0),
            Some("phi") => 70.0 + (params_b.log2() * 4.0).min(20.0),
            _ => 60.0 + (params_b.log2() * 2.0).min(25.0),
        };

        reasoning.push(format!(
            "Quality: {:.0} ({}B params, {} family)",
            base_score, params_b, model.family.as_deref().unwrap_or("unknown")
        ));

        base_score.min(100.0)
    }

    fn compute_speed(
        model: &ModelInfo,
        profile: &SystemProfile,
        benchmark: Option<&ModelBenchmark>,
        reasoning: &mut Vec<String>,
    ) -> f64 {
        if let Some(bench) = benchmark {
            // Real benchmark data
            let score = (bench.tokens_per_second / 100.0 * 100.0).min(100.0);
            reasoning.push(format!(
                "Speed: {:.0} (measured {:.1} tok/s)",
                score, bench.tokens_per_second
            ));
            return score;
        }

        // Estimate based on model size vs VRAM
        let params_b = model.parameter_size.unwrap_or(7_000_000_000) as f64 / 1e9;
        let vram = profile.effective_vram_gb();

        // Rough heuristic: larger model relative to VRAM = slower
        let ratio = params_b * 2.0 / vram;  // FP16 needs ~2GB per 1B params
        let score = if ratio < 0.5 {
            95.0  // Plenty of room
        } else if ratio < 0.8 {
            80.0  // Comfortable
        } else if ratio < 1.0 {
            60.0  // Tight fit
        } else if ratio < 1.5 {
            40.0  // Needs quantization
        } else {
            20.0  // Will struggle
        };

        reasoning.push(format!(
            "Speed: {:.0} (estimated, {}B params / {:.1}GB VRAM)",
            score, params_b, vram
        ));

        score
    }

    fn compute_fit(
        model: &ModelInfo,
        profile: &SystemProfile,
        reasoning: &mut Vec<String>,
    ) -> f64 {
        let params_b = model.parameter_size.unwrap_or(7_000_000_000) as f64 / 1e9;
        let vram = profile.effective_vram_gb();

        // VRAM needed at different quantization levels
        let vram_fp16 = params_b * 2.0;
        let vram_q8 = params_b * 1.0;
        let vram_q4 = params_b * 0.5;

        let score = if vram >= vram_fp16 * 1.2 {
            100.0  // Can run FP16 with headroom
        } else if vram >= vram_fp16 {
            90.0   // Can run FP16
        } else if vram >= vram_q8 * 1.2 {
            80.0   // Can run Q8 with headroom
        } else if vram >= vram_q8 {
            70.0   // Can run Q8
        } else if vram >= vram_q4 * 1.2 {
            60.0   // Can run Q4 with headroom
        } else if vram >= vram_q4 {
            50.0   // Can run Q4
        } else {
            20.0   // Too large for hardware
        };

        let quant = if vram >= vram_fp16 {
            "FP16"
        } else if vram >= vram_q8 {
            "Q8"
        } else if vram >= vram_q4 {
            "Q4"
        } else {
            "Q2 (may fail)"
        };

        reasoning.push(format!(
            "Fit: {:.0} ({}B needs {:.1}GB, have {:.1}GB → {})",
            score, params_b, vram_q4, vram, quant
        ));

        score
    }

    fn compute_context(model: &ModelInfo, reasoning: &mut Vec<String>) -> f64 {
        let context_len = model.context_length.unwrap_or(4096);

        // Logarithmic scale: 4k=50, 8k=60, 32k=75, 128k=87, 200k=92
        let score = (context_len as f64).log2() * 10.0 - 20.0;
        let score = score.max(20.0).min(100.0);

        reasoning.push(format!(
            "Context: {:.0} ({}k tokens)",
            score, context_len / 1000
        ));

        score
    }
}

/// Model scorer with hardware profile
pub struct ModelScorer {
    profile: SystemProfile,
    weights: ScoringWeights,
}

impl ModelScorer {
    pub fn new(profile: SystemProfile, weights: ScoringWeights) -> Self {
        Self { profile, weights }
    }

    pub fn score(&self, model: &ModelInfo) -> ModelScore {
        let benchmark = self.profile.benchmarks.as_ref()
            .and_then(|b| b.models.iter().find(|m| m.model_name == model.name));

        ModelScore::compute(model, &self.profile, &self.weights, benchmark)
    }

    pub fn score_all(&self, models: &[ModelInfo]) -> Vec<(ModelInfo, ModelScore)> {
        let mut scored: Vec<_> = models.iter()
            .map(|m| (m.clone(), self.score(m)))
            .collect();

        // Sort by overall score descending
        scored.sort_by(|a, b| b.1.overall.partial_cmp(&a.1.overall).unwrap());

        scored
    }
}
```

### Task 4: Recommendation Engine

```rust
// crates/spn-backends/src/recommend.rs

use crate::{
    hardware::SystemProfile,
    scoring::{ModelScore, ModelScorer, ScoringWeights},
    registry::BackendRegistry,
    orchestrator::ModelIntent,
    model_ref::ModelRef,
};
use spn_core::ModelInfo;

/// Recommendation with reasoning
#[derive(Debug)]
pub struct ModelRecommendation {
    pub model_ref: ModelRef,
    pub model_info: ModelInfo,
    pub score: ModelScore,
    pub rank: usize,
    pub summary: String,
}

/// Recommendation engine using llmfit-core logic
pub struct RecommendationEngine {
    profile: SystemProfile,
    registry: BackendRegistry,
}

impl RecommendationEngine {
    pub fn new(profile: SystemProfile, registry: BackendRegistry) -> Self {
        Self { profile, registry }
    }

    /// Recommend models for an intent
    pub async fn recommend(
        &self,
        intent: ModelIntent,
        constraints: &ModelConstraints,
        limit: usize,
    ) -> Vec<ModelRecommendation> {
        // 1. Get candidate models based on intent
        let candidates = self.get_candidates(intent, constraints).await;

        // 2. Determine scoring weights based on preferences
        let weights = self.intent_to_weights(intent, constraints);

        // 3. Score all candidates
        let scorer = ModelScorer::new(self.profile.clone(), weights);
        let scored = scorer.score_all(&candidates);

        // 4. Filter by hardware fit
        let runnable: Vec<_> = scored.into_iter()
            .filter(|(model, score)| score.fit >= 30.0)  // Minimum fit threshold
            .take(limit)
            .collect();

        // 5. Build recommendations
        runnable.into_iter()
            .enumerate()
            .map(|(idx, (model, score))| {
                let summary = self.generate_summary(&model, &score, idx + 1);
                ModelRecommendation {
                    model_ref: ModelRef::from_model_info(&model),
                    model_info: model,
                    score,
                    rank: idx + 1,
                    summary,
                }
            })
            .collect()
    }

    /// Get candidate models for intent
    async fn get_candidates(
        &self,
        intent: ModelIntent,
        constraints: &ModelConstraints,
    ) -> Vec<ModelInfo> {
        let mut candidates = Vec::new();

        // Collect from all backends
        for kind in self.registry.available() {
            // Skip cloud backends if local_only
            if constraints.local_only && kind.is_cloud() {
                continue;
            }

            if let Some(backend) = self.registry.get(kind) {
                if let Ok(models) = backend.list_models().await {
                    // Filter by intent capability
                    for model in models {
                        if self.model_fits_intent(&model, intent) {
                            candidates.push(model);
                        }
                    }
                }
            }
        }

        candidates
    }

    /// Check if model fits intent
    fn model_fits_intent(&self, model: &ModelInfo, intent: ModelIntent) -> bool {
        let modality = model.modality.as_deref().unwrap_or("text");
        let family = model.family.as_deref().unwrap_or("");

        match intent {
            ModelIntent::CodeGeneration => {
                family.contains("code") || family.contains("starcoder")
                    || model.name.contains("code")
            }
            ModelIntent::ImageGeneration => modality == "text-to-image",
            ModelIntent::ImageAnalysis => modality == "vision",
            ModelIntent::SpeechToText => modality == "speech-to-text",
            ModelIntent::DeepReasoning => {
                model.parameter_size.unwrap_or(0) > 30_000_000_000
                    || family == "claude" || family == "gpt"
            }
            ModelIntent::FastGeneration => {
                model.parameter_size.unwrap_or(0) < 15_000_000_000
                    || model.name.contains("turbo") || model.name.contains("flash")
            }
            ModelIntent::CreativeWriting => modality == "text",
            ModelIntent::Translation => modality == "text",
            _ => true,
        }
    }

    /// Determine weights based on intent + constraints
    fn intent_to_weights(&self, intent: ModelIntent, constraints: &ModelConstraints) -> ScoringWeights {
        // Base weights for intent
        let mut weights = match intent {
            ModelIntent::DeepReasoning => ScoringWeights::quality_first(),
            ModelIntent::FastGeneration => ScoringWeights::speed_first(),
            ModelIntent::CodeGeneration => ScoringWeights {
                quality: 0.35, speed: 0.25, fit: 0.25, context: 0.15
            },
            _ => ScoringWeights::balanced(),
        };

        // Adjust for constraints
        if constraints.prefer_speed {
            weights.speed += 0.15;
            weights.quality -= 0.1;
            weights.context -= 0.05;
        }
        if constraints.prefer_quality {
            weights.quality += 0.15;
            weights.speed -= 0.1;
            weights.fit -= 0.05;
        }
        if constraints.local_only {
            weights.fit += 0.1;  // Hardware fit matters more for local
        }

        // Normalize
        let sum = weights.quality + weights.speed + weights.fit + weights.context;
        weights.quality /= sum;
        weights.speed /= sum;
        weights.fit /= sum;
        weights.context /= sum;

        weights
    }

    /// Generate human-readable summary
    fn generate_summary(&self, model: &ModelInfo, score: &ModelScore, rank: usize) -> String {
        let params = model.parameter_size
            .map(|p| format!("{}B", p / 1_000_000_000))
            .unwrap_or_else(|| "?B".to_string());

        let top_reason = score.reasoning.first()
            .cloned()
            .unwrap_or_default();

        format!(
            "#{} {} ({}) - Overall: {:.0}/100\n   {}",
            rank, model.name, params, score.overall, top_reason
        )
    }
}

/// Constraints for model selection (extended)
#[derive(Debug, Clone, Default)]
pub struct ModelConstraints {
    pub local_only: bool,
    pub free_only: bool,
    pub prefer_speed: bool,
    pub prefer_quality: bool,
    pub max_params_b: Option<f64>,
    pub min_context_k: Option<u32>,
    pub allowed_backends: Option<Vec<BackendKind>>,
}
```

### Task 5: CLI Commands

```rust
// crates/spn/src/commands/model.rs (Phase C additions)

use crate::ux::{ds, tables};
use spn_backends::{
    hardware::SystemProfile,
    scoring::{ModelScorer, ScoringWeights},
    recommend::{RecommendationEngine, ModelConstraints},
    orchestrator::ModelIntent,
};

/// Explore available models with hardware-aware scoring
#[derive(Debug, Parser)]
pub struct ExploreCmd {
    /// Filter by family (llama, mistral, phi, etc.)
    #[arg(short, long)]
    pub family: Option<String>,

    /// Filter by modality (text, vision, image, audio)
    #[arg(short, long)]
    pub modality: Option<String>,

    /// Filter by backend (ollama, candle, mistral-rs)
    #[arg(short, long)]
    pub backend: Option<String>,

    /// Maximum parameters in billions
    #[arg(long)]
    pub max_params: Option<f64>,

    /// Scoring preference (quality, speed, fit, balanced)
    #[arg(long, default_value = "balanced")]
    pub prefer: String,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub output: OutputFormat,

    /// Number of results
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

impl ExploreCmd {
    pub async fn run(&self, orchestrator: &ModelOrchestrator) -> Result<()> {
        // Detect hardware
        let profile = SystemProfile::detect().with_benchmarks();

        println!("{}", ds::info_line(&format!(
            "Hardware: {} GPU(s), {:.1}GB effective VRAM",
            profile.specs.gpus.len(),
            profile.effective_vram_gb()
        )));

        // Collect all models
        let mut all_models = Vec::new();
        for (kind, backend) in orchestrator.list_backends().await? {
            if let Some(ref filter) = self.backend {
                if kind.id() != filter {
                    continue;
                }
            }

            if let Ok(models) = backend.list_models().await {
                for model in models {
                    // Apply filters
                    if let Some(ref family) = self.family {
                        if model.family.as_deref() != Some(family.as_str()) {
                            continue;
                        }
                    }
                    if let Some(ref modality) = self.modality {
                        if model.modality.as_deref() != Some(modality.as_str()) {
                            continue;
                        }
                    }
                    if let Some(max) = self.max_params {
                        if let Some(params) = model.parameter_size {
                            if (params as f64 / 1e9) > max {
                                continue;
                            }
                        }
                    }
                    all_models.push((kind, model));
                }
            }
        }

        // Score models
        let weights = ScoringWeights::from_preference(&self.prefer);
        let scorer = ModelScorer::new(profile, weights);

        let mut scored: Vec<_> = all_models.iter()
            .map(|(kind, model)| {
                let score = scorer.score(model);
                (kind, model, score)
            })
            .collect();

        scored.sort_by(|a, b| b.2.overall.partial_cmp(&a.2.overall).unwrap());
        scored.truncate(self.limit);

        // Output
        match self.output {
            OutputFormat::Table => self.print_table(&scored),
            OutputFormat::Json => self.print_json(&scored)?,
            OutputFormat::Yaml => self.print_yaml(&scored)?,
        }

        Ok(())
    }

    fn print_table(&self, models: &[(&BackendKind, &ModelInfo, ModelScore)]) {
        println!();
        println!("{}", tables::header(&[
            ("Rank", 5),
            ("Model", 25),
            ("Backend", 10),
            ("Params", 8),
            ("Quality", 8),
            ("Speed", 8),
            ("Fit", 8),
            ("Overall", 8),
        ]));

        for (idx, (kind, model, score)) in models.iter().enumerate() {
            let params = model.parameter_size
                .map(|p| format!("{:.1}B", p as f64 / 1e9))
                .unwrap_or_else(|| "-".to_string());

            let row = tables::row(&[
                (format!("#{}", idx + 1), 5),
                (model.name.clone(), 25),
                (kind.id().to_string(), 10),
                (params, 8),
                (format!("{:.0}", score.quality), 8),
                (format!("{:.0}", score.speed), 8),
                (format!("{:.0}", score.fit), 8),
                (format!("{:.0}", score.overall), 8),
            ]);

            let color = if score.overall >= 80.0 {
                console::Style::new().green()
            } else if score.overall >= 60.0 {
                console::Style::new().yellow()
            } else {
                console::Style::new().red()
            };

            println!("{}", color.apply_to(row));
        }

        println!();
        println!("{}", ds::info_line("Scores: Quality=capability, Speed=inference, Fit=hardware, Overall=weighted"));
    }

    fn print_json(&self, models: &[(&BackendKind, &ModelInfo, ModelScore)]) -> Result<()> {
        let output: Vec<_> = models.iter()
            .enumerate()
            .map(|(idx, (kind, model, score))| {
                serde_json::json!({
                    "rank": idx + 1,
                    "name": model.name,
                    "backend": kind.id(),
                    "family": model.family,
                    "parameters": model.parameter_size,
                    "scores": {
                        "quality": score.quality,
                        "speed": score.speed,
                        "fit": score.fit,
                        "context": score.context,
                        "overall": score.overall,
                    },
                    "reasoning": score.reasoning,
                })
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

    fn print_yaml(&self, models: &[(&BackendKind, &ModelInfo, ModelScore)]) -> Result<()> {
        let output: Vec<_> = models.iter()
            .enumerate()
            .map(|(idx, (kind, model, score))| {
                serde_json::json!({
                    "rank": idx + 1,
                    "name": model.name,
                    "backend": kind.id(),
                    "scores": {
                        "overall": score.overall,
                        "quality": score.quality,
                        "speed": score.speed,
                        "fit": score.fit,
                    },
                })
            })
            .collect();

        println!("{}", serde_yaml::to_string(&output)?);
        Ok(())
    }
}

/// Get hardware-aware model recommendations
#[derive(Debug, Parser)]
pub struct RecommendCmd {
    /// What you want to do
    #[arg(value_enum)]
    pub intent: IntentArg,

    /// Prefer quality over speed
    #[arg(long)]
    pub prefer_quality: bool,

    /// Prefer speed over quality
    #[arg(long)]
    pub prefer_speed: bool,

    /// Local models only (no cloud APIs)
    #[arg(long)]
    pub local_only: bool,

    /// Free models only
    #[arg(long)]
    pub free_only: bool,

    /// Number of recommendations
    #[arg(short, long, default_value = "5")]
    pub limit: usize,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum IntentArg {
    FastGeneration,
    DeepReasoning,
    CreativeWriting,
    CodeGeneration,
    Translation,
    ImageGeneration,
    ImageAnalysis,
    SpeechToText,
}

impl From<IntentArg> for ModelIntent {
    fn from(arg: IntentArg) -> Self {
        match arg {
            IntentArg::FastGeneration => ModelIntent::FastGeneration,
            IntentArg::DeepReasoning => ModelIntent::DeepReasoning,
            IntentArg::CreativeWriting => ModelIntent::CreativeWriting,
            IntentArg::CodeGeneration => ModelIntent::CodeGeneration,
            IntentArg::Translation => ModelIntent::Translation,
            IntentArg::ImageGeneration => ModelIntent::ImageGeneration,
            IntentArg::ImageAnalysis => ModelIntent::ImageAnalysis,
            IntentArg::SpeechToText => ModelIntent::SpeechToText,
        }
    }
}

impl RecommendCmd {
    pub async fn run(&self, orchestrator: &ModelOrchestrator) -> Result<()> {
        let profile = SystemProfile::detect().with_benchmarks();

        println!("{}", ds::info_line(&format!(
            "Finding best models for {:?} on your hardware...",
            self.intent
        )));

        let constraints = ModelConstraints {
            local_only: self.local_only,
            free_only: self.free_only,
            prefer_speed: self.prefer_speed,
            prefer_quality: self.prefer_quality,
            ..Default::default()
        };

        let engine = RecommendationEngine::new(profile, orchestrator.registry().clone());
        let recommendations = engine.recommend(
            self.intent.into(),
            &constraints,
            self.limit,
        ).await;

        if recommendations.is_empty() {
            println!("{}", ds::warn_line("No models found matching your criteria"));
            return Ok(());
        }

        match self.output {
            OutputFormat::Table => {
                println!();
                println!("{}", ds::header(&format!(
                    "Top {} models for {:?}",
                    recommendations.len(),
                    self.intent
                )));
                println!();

                for rec in &recommendations {
                    // Rank and model name
                    let params = rec.model_info.parameter_size
                        .map(|p| format!("{}B", p / 1_000_000_000))
                        .unwrap_or_else(|| "?".to_string());

                    println!("  {} {} ({}) - Score: {:.0}/100",
                        ds::bullet(rec.rank),
                        console::style(&rec.model_info.name).bold(),
                        params,
                        rec.score.overall
                    );

                    // Score breakdown
                    println!("     Quality: {:.0}  Speed: {:.0}  Fit: {:.0}  Context: {:.0}",
                        rec.score.quality,
                        rec.score.speed,
                        rec.score.fit,
                        rec.score.context
                    );

                    // Top reasoning
                    for reason in rec.score.reasoning.iter().take(2) {
                        println!("     {}", console::style(reason).dim());
                    }

                    println!();
                }

                // Usage hint
                if let Some(top) = recommendations.first() {
                    println!("{}", ds::success_line(&format!(
                        "To use: spn model load {}",
                        top.model_info.name
                    )));
                }
            }
            OutputFormat::Json => {
                let output: Vec<_> = recommendations.iter()
                    .map(|r| {
                        serde_json::json!({
                            "rank": r.rank,
                            "model": r.model_info.name,
                            "model_ref": r.model_ref.to_string(),
                            "scores": {
                                "overall": r.score.overall,
                                "quality": r.score.quality,
                                "speed": r.score.speed,
                                "fit": r.score.fit,
                                "context": r.score.context,
                            },
                            "reasoning": r.score.reasoning,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            OutputFormat::Yaml => {
                let output: Vec<_> = recommendations.iter()
                    .map(|r| {
                        serde_json::json!({
                            "rank": r.rank,
                            "model": r.model_info.name,
                            "overall_score": r.score.overall,
                        })
                    })
                    .collect();
                println!("{}", serde_yaml::to_string(&output)?);
            }
        }

        Ok(())
    }
}

/// Run local benchmark on a model
#[derive(Debug, Parser)]
pub struct BenchmarkCmd {
    /// Model to benchmark
    pub model: String,

    /// Number of prompt tokens to test
    #[arg(long, default_value = "100")]
    pub prompt_tokens: usize,

    /// Number of completion tokens to generate
    #[arg(long, default_value = "100")]
    pub completion_tokens: usize,

    /// Number of iterations
    #[arg(long, default_value = "3")]
    pub iterations: usize,

    /// Save results to cache
    #[arg(long)]
    pub save: bool,
}

impl BenchmarkCmd {
    pub async fn run(&self, orchestrator: &ModelOrchestrator) -> Result<()> {
        let model_ref = ModelRef::parse(&format!("@models/{}", self.model));
        let (backend, model_name) = orchestrator.get_backend(&model_ref).await?;

        println!("{}", ds::info_line(&format!(
            "Benchmarking {} ({} iterations)...",
            self.model, self.iterations
        )));

        // Generate test prompt
        let prompt = "The quick brown fox ".repeat(self.prompt_tokens / 5);
        let messages = vec![ChatMessage::user(prompt)];
        let options = ChatOptions {
            max_tokens: Some(self.completion_tokens as u32),
            ..Default::default()
        };

        let mut results = Vec::new();

        for i in 1..=self.iterations {
            print!("  Iteration {}/{}... ", i, self.iterations);
            std::io::stdout().flush()?;

            let start = std::time::Instant::now();
            let response = backend.chat(model_name.clone(), messages.clone(), Some(options.clone())).await?;
            let elapsed = start.elapsed();

            let tokens = response.eval_count.unwrap_or(self.completion_tokens as u32) as f64;
            let tps = tokens / elapsed.as_secs_f64();

            println!("{:.1} tok/s", tps);
            results.push((tps, elapsed.as_millis() as u64));
        }

        // Summarize
        let avg_tps: f64 = results.iter().map(|r| r.0).sum::<f64>() / results.len() as f64;
        let avg_ttft = results.iter().map(|r| r.1).sum::<u64>() / results.len() as u64;

        println!();
        println!("{}", ds::success_line(&format!(
            "Average: {:.1} tok/s, {:.0}ms TTFT",
            avg_tps, avg_ttft
        )));

        // Save to cache if requested
        if self.save {
            let benchmark = ModelBenchmark {
                model_name: self.model.clone(),
                backend: backend.id().to_string(),
                tokens_per_second: avg_tps,
                time_to_first_token_ms: avg_ttft,
                memory_used_gb: 0.0,  // TODO: measure
            };

            self.save_benchmark(&benchmark)?;
            println!("{}", ds::info_line("Results saved to ~/.spn/benchmarks.json"));
        }

        Ok(())
    }

    fn save_benchmark(&self, benchmark: &ModelBenchmark) -> Result<()> {
        let cache_path = dirs::data_dir()
            .ok_or_else(|| anyhow!("Could not find data directory"))?
            .join("spn")
            .join("benchmarks.json");

        std::fs::create_dir_all(cache_path.parent().unwrap())?;

        let mut results = if cache_path.exists() {
            serde_json::from_str::<BenchmarkResults>(&std::fs::read_to_string(&cache_path)?)?
        } else {
            BenchmarkResults {
                timestamp: chrono::Utc::now(),
                models: Vec::new(),
            }
        };

        // Update or add
        if let Some(existing) = results.models.iter_mut().find(|m| m.model_name == benchmark.model_name) {
            *existing = benchmark.clone();
        } else {
            results.models.push(benchmark.clone());
        }

        results.timestamp = chrono::Utc::now();
        std::fs::write(&cache_path, serde_json::to_string_pretty(&results)?)?;

        Ok(())
    }
}
```

### Task 6: Orchestrator Integration

```rust
// crates/spn-backends/src/orchestrator.rs (Phase C additions)

impl ModelOrchestrator {
    /// Resolve intent with hardware-aware scoring (Phase C)
    pub async fn resolve_intent_with_hardware(
        &self,
        intent: ModelIntent,
        constraints: &ModelConstraints,
    ) -> BackendResult<ModelRef> {
        // Use recommendation engine
        let profile = SystemProfile::detect().with_benchmarks();
        let engine = RecommendationEngine::new(profile, self.registry.read().await.clone());

        let recommendations = engine.recommend(intent, constraints, 1).await;

        if let Some(top) = recommendations.first() {
            Ok(top.model_ref.clone())
        } else {
            // Fallback to simple resolution
            self.resolve_intent(intent, constraints).await
        }
    }

    /// Get all backends
    pub async fn list_backends(&self) -> BackendResult<Vec<(BackendKind, Arc<dyn DynModelBackend>)>> {
        let registry = self.registry.read().await;
        let mut result = Vec::new();

        for kind in registry.available() {
            if let Some(backend) = registry.get(kind) {
                result.push((kind, backend));
            }
        }

        Ok(result)
    }

    /// Get registry reference (for recommendation engine)
    pub fn registry(&self) -> &RwLock<BackendRegistry> {
        &self.registry
    }
}
```

### Task 7: MCP Tools

```rust
// crates/spn-mcp/src/tools/recommend.rs

/// MCP tool: spn_model_recommend
pub struct ModelRecommendTool {
    orchestrator: Arc<ModelOrchestrator>,
}

impl Tool for ModelRecommendTool {
    fn name(&self) -> &str { "spn_model_recommend" }

    fn description(&self) -> &str {
        "Get hardware-aware model recommendations for a task"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "intent": {
                    "type": "string",
                    "enum": [
                        "fast-generation",
                        "deep-reasoning",
                        "creative-writing",
                        "code-generation",
                        "translation",
                        "image-generation",
                        "image-analysis",
                        "speech-to-text"
                    ],
                    "description": "What you want the model to do"
                },
                "prefer_speed": {
                    "type": "boolean",
                    "description": "Prioritize inference speed"
                },
                "prefer_quality": {
                    "type": "boolean",
                    "description": "Prioritize output quality"
                },
                "local_only": {
                    "type": "boolean",
                    "description": "Only recommend local models (no cloud APIs)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of recommendations",
                    "default": 3
                }
            },
            "required": ["intent"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let intent_str = params["intent"].as_str().unwrap();
        let intent = match intent_str {
            "fast-generation" => ModelIntent::FastGeneration,
            "deep-reasoning" => ModelIntent::DeepReasoning,
            "creative-writing" => ModelIntent::CreativeWriting,
            "code-generation" => ModelIntent::CodeGeneration,
            "translation" => ModelIntent::Translation,
            "image-generation" => ModelIntent::ImageGeneration,
            "image-analysis" => ModelIntent::ImageAnalysis,
            "speech-to-text" => ModelIntent::SpeechToText,
            _ => return Err("Invalid intent".into()),
        };

        let constraints = ModelConstraints {
            prefer_speed: params.get("prefer_speed").and_then(|v| v.as_bool()).unwrap_or(false),
            prefer_quality: params.get("prefer_quality").and_then(|v| v.as_bool()).unwrap_or(false),
            local_only: params.get("local_only").and_then(|v| v.as_bool()).unwrap_or(false),
            ..Default::default()
        };

        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let profile = SystemProfile::detect();
        let registry = self.orchestrator.registry().read().await;
        let engine = RecommendationEngine::new(profile, registry.clone());

        let recommendations = engine.recommend(intent, &constraints, limit).await;

        Ok(serde_json::json!({
            "hardware": {
                "gpus": profile.specs.gpus.iter().map(|g| g.name.clone()).collect::<Vec<_>>(),
                "vram_gb": profile.effective_vram_gb(),
            },
            "recommendations": recommendations.iter().map(|r| {
                serde_json::json!({
                    "rank": r.rank,
                    "model": r.model_info.name,
                    "model_ref": r.model_ref.to_string(),
                    "overall_score": r.score.overall,
                    "scores": {
                        "quality": r.score.quality,
                        "speed": r.score.speed,
                        "fit": r.score.fit,
                    },
                    "reasoning": r.score.reasoning,
                })
            }).collect::<Vec<_>>()
        }))
    }
}

/// MCP tool: spn_hardware_profile
pub struct HardwareProfileTool;

impl Tool for HardwareProfileTool {
    fn name(&self) -> &str { "spn_hardware_profile" }

    fn description(&self) -> &str {
        "Get the hardware profile of the current machine for model selection"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        let profile = SystemProfile::detect();

        Ok(serde_json::json!({
            "cpu_cores": profile.specs.cpu_cores,
            "ram_gb": profile.specs.ram_gb,
            "gpus": profile.specs.gpus.iter().map(|g| {
                serde_json::json!({
                    "name": g.name,
                    "vram_gb": g.vram_gb,
                    "device_type": format!("{:?}", g.device_type),
                })
            }).collect::<Vec<_>>(),
            "optimal_device": format!("{:?}", profile.specs.optimal_device),
            "effective_vram_gb": profile.effective_vram_gb(),
            "apple_silicon": profile.llmfit_specs.is_apple_silicon(),
        }))
    }
}
```

---

## File Changes Summary

| File | Action | LOC |
|------|--------|-----|
| `crates/spn-backends/src/hardware.rs` | Update | +100 |
| `crates/spn-backends/src/scoring.rs` | Create | ~300 |
| `crates/spn-backends/src/recommend.rs` | Create | ~250 |
| `crates/spn-backends/src/orchestrator.rs` | Update | +50 |
| `crates/spn/src/commands/model.rs` | Update | +400 |
| `crates/spn-mcp/src/tools/recommend.rs` | Create | ~150 |
| `crates/spn-backends/Cargo.toml` | Update | +10 |

**Total:** ~1,260 LOC

---

## Cargo.toml Updates

```toml
# crates/spn-backends/Cargo.toml (Phase C additions)

[features]
# Phase C: Hardware-aware recommendations
llmfit = ["dep:llmfit-core"]
recommend = ["llmfit"]

[dependencies]
# Phase C: llmfit-core (library only)
llmfit-core = { version = "0.6", optional = true }

# Hardware detection (already added in Phase B)
num_cpus = "1.16"

# Scoring/benchmarks
chrono = { version = "0.4", features = ["serde"] }
```

---

## CLI Command Summary

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PHASE C CLI COMMANDS                                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  spn model explore                                                              │
│  ├── Lists all models with hardware-aware scoring                               │
│  ├── --family llama             Filter by model family                          │
│  ├── --modality vision          Filter by capability                            │
│  ├── --max-params 13            Max parameters in billions                      │
│  ├── --prefer speed             Scoring preference                              │
│  └── --output json|yaml|table   Output format                                   │
│                                                                                 │
│  spn model recommend <intent>                                                   │
│  ├── code-generation, deep-reasoning, fast-generation, etc.                    │
│  ├── --prefer-speed             Prioritize inference speed                      │
│  ├── --prefer-quality           Prioritize output quality                       │
│  ├── --local-only               No cloud APIs                                   │
│  └── --limit 5                  Number of recommendations                       │
│                                                                                 │
│  spn model benchmark <model>                                                    │
│  ├── --prompt-tokens 100        Test prompt size                                │
│  ├── --completion-tokens 100    Tokens to generate                              │
│  ├── --iterations 3             Number of runs                                  │
│  └── --save                     Save to benchmark cache                         │
│                                                                                 │
│  spn model hardware                                                             │
│  └── Show detected hardware profile                                             │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Example Output

### `spn model explore --prefer speed --limit 5`

```
Hardware: 1 GPU(s), 24.0GB effective VRAM

  Rank  Model                     Backend     Params    Quality  Speed   Fit     Overall
  #1    claude-sonnet             anthropic   175B      95       90      100     93
  #2    llama3.2:8b               ollama      8.0B      78       95      100     88
  #3    phi-3-mini                ollama      3.8B      72       98      100     85
  #4    codestral                 mistral     22B       85       75      90      82
  #5    llama3.2:70b              ollama      70B       88       60      85      78

Scores: Quality=capability, Speed=inference, Fit=hardware, Overall=weighted
```

### `spn model recommend code-generation --local-only`

```
Finding best models for CodeGeneration on your hardware...

Top 5 models for CodeGeneration

  1. codellama:7b (7B) - Score: 82/100
     Quality: 75  Speed: 90  Fit: 95  Context: 70
     Quality: 75 (7B params, codellama family)
     Speed: 90 (estimated, 7B params / 24.0GB VRAM)

  2. starcoder2:7b (7B) - Score: 80/100
     Quality: 78  Speed: 88  Fit: 95  Context: 65
     Quality: 78 (7B params, starcoder family)

  3. deepseek-coder:6.7b (6.7B) - Score: 79/100
     Quality: 80  Speed: 85  Fit: 95  Context: 60
     Quality: 80 (6.7B params, deepseek family)

To use: spn model load codellama:7b
```

### `spn model benchmark llama3.2:8b --iterations 5 --save`

```
Benchmarking llama3.2:8b (5 iterations)...
  Iteration 1/5... 45.2 tok/s
  Iteration 2/5... 47.1 tok/s
  Iteration 3/5... 46.8 tok/s
  Iteration 4/5... 46.5 tok/s
  Iteration 5/5... 46.9 tok/s

Average: 46.5 tok/s, 150ms TTFT
Results saved to ~/.spn/benchmarks.json
```

---

## Verification Checklist

- [ ] `cargo build --workspace --features recommend` passes
- [ ] `spn model explore` lists models with scores
- [ ] `spn model recommend code-generation` gives sensible results
- [ ] `spn model benchmark llama3.2:8b` measures actual performance
- [ ] Hardware detection works on macOS (Metal) and Linux (CUDA)
- [ ] JSON/YAML output is valid and parseable
- [ ] MCP tools `spn_model_recommend` and `spn_hardware_profile` work
- [ ] Scores are consistent and repeatable
- [ ] Benchmark cache persists across sessions

---

## Commit Strategy

```bash
# Commit 1: Hardware profiling enhancements
feat(backends): enhance SystemProfile with llmfit integration

# Commit 2: Scoring system
feat(backends): add ModelScore and ModelScorer

# Commit 3: Recommendation engine
feat(backends): add RecommendationEngine for hardware-aware selection

# Commit 4: CLI commands
feat(cli): add model explore, recommend, benchmark commands

# Commit 5: MCP tools
feat(mcp): add model recommendation MCP tools

# Commit 6: Orchestrator integration
feat(backends): integrate recommendations into ModelOrchestrator
```

---

## Dependencies

**New crates (Option A - direct dependency):**
- `llmfit-core` (0.6.x) - Hardware profiling and model scoring

**If forking (Option B):**
- Port ~500 LOC from llmfit-core to `spn-recommend` crate
- No external dependency

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| llmfit-core breaking changes | Pin version, consider forking |
| Inaccurate scoring | Use local benchmarks to calibrate |
| Missing model metadata | Maintain internal registry |
| Hardware detection failure | Graceful fallback to estimates |

---

## Success Criteria

1. **Accuracy:** Top recommendation matches user hardware 90%+ of time
2. **Performance:** `spn model explore` completes in <1s
3. **Usability:** CLI output is clear and actionable
4. **Integration:** Orchestrator uses hardware-aware selection
5. **Extensibility:** Easy to add new scoring dimensions

---

## Comparison: TUI vs CLI Approach

| Aspect | llmfit TUI | spn CLI (our approach) |
|--------|------------|------------------------|
| **Interactivity** | Full TUI with navigation | Commands + flags |
| **Output formats** | Visual only | Table, JSON, YAML |
| **Automation** | Difficult | Easy (scripting, MCP) |
| **Dependencies** | ratatui, crossterm | None (console for colors) |
| **Binary size** | +5MB | +0.5MB |
| **Learning curve** | Learn TUI controls | Learn CLI flags |
| **Integration** | Standalone tool | Part of spn ecosystem |

**Our approach prioritizes:** Automation, integration, scriptability over interactive browsing.
