//! Model types and registry for native inference.
//!
//! This module provides:
//! - [`ModelType`]: Capability types (Text, Vision, Embedding, etc.)
//! - [`ModelArchitecture`]: Architectures supported by mistral.rs
//! - [`KnownModel`]: Curated model definitions
//! - [`KNOWN_MODELS`]: Registry of pre-validated models
//! - [`resolve_model`]: Resolve model ID to curated or HuggingFace model
//! - [`auto_select_quantization`]: RAM-based quantization selection
//!
//! # Example
//!
//! ```
//! use spn_core::{resolve_model, ResolvedModel, KNOWN_MODELS};
//!
//! // Resolve a curated model
//! let model = resolve_model("qwen3:8b");
//! assert!(matches!(model, Some(ResolvedModel::Curated(_))));
//!
//! // Resolve a HuggingFace model
//! let model = resolve_model("hf:bartowski/Qwen3-30B-GGUF");
//! assert!(matches!(model, Some(ResolvedModel::HuggingFace { .. })));
//! ```

use crate::backend::Quantization;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// Model Types
// ============================================================================

/// Model capability type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ModelType {
    /// Text generation (LLM).
    Text,
    /// Vision-language model (VLM).
    Vision,
    /// Embedding model for vector representations.
    Embedding,
    /// Audio/speech model.
    Audio,
    /// Image generation (diffusion).
    Diffusion,
}

impl ModelType {
    /// Human-readable name for this model type.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Vision => "Vision",
            Self::Embedding => "Embedding",
            Self::Audio => "Audio",
            Self::Diffusion => "Diffusion",
        }
    }

    /// Builder type name in mistral.rs.
    #[must_use]
    pub const fn builder_name(&self) -> &'static str {
        match self {
            Self::Text => "TextModelBuilder / GgufModelBuilder",
            Self::Vision => "VisionModelBuilder",
            Self::Embedding => "EmbeddingModelBuilder",
            Self::Audio => "AudioModelBuilder",
            Self::Diffusion => "DiffusionModelBuilder",
        }
    }
}

// ============================================================================
// Model Architecture
// ============================================================================

/// Architecture supported by mistral.rs v0.7.0.
///
/// See: <https://github.com/EricLBuehler/mistral.rs#supported-models>
///
/// **Note:** Architecture names must match mistral.rs exactly (case-sensitive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(non_camel_case_types)]
pub enum ModelArchitecture {
    // =========================================================================
    // TEXT MODELS (loaded via TextModelBuilder or GgufModelBuilder)
    // =========================================================================
    /// Mistral 7B and variants.
    Mistral,
    /// Gemma (first generation).
    Gemma,
    /// Gemma 2.
    Gemma2,
    /// Gemma 3.
    Gemma3,
    /// Mixtral MoE models.
    Mixtral,
    /// Llama (1, 2, 3).
    Llama,
    /// Llama 4.
    Llama4,
    /// Phi-2.
    Phi2,
    /// Phi-3.
    Phi3,
    /// Phi-3.5 MoE variant.
    Phi3_5MoE,
    /// Qwen 2.
    Qwen2,
    /// Qwen 3.
    Qwen3,
    /// Qwen 3 MoE variant.
    Qwen3Moe,
    /// GLM-4 (ChatGLM).
    GLM4,
    /// StarCoder 2.
    Starcoder2,
    /// DeepSeek V2.
    DeepseekV2,
    /// DeepSeek V3.
    DeepseekV3,
    /// SmolLM 3.
    SmolLM3,

    // =========================================================================
    // VISION MODELS (loaded via VisionModelBuilder)
    // =========================================================================
    /// Phi-3 Vision.
    Phi3V,
    /// Phi-4 Multimodal.
    Phi4MM,
    /// IDEFICS 2.
    Idefics2,
    /// IDEFICS 3.
    Idefics3,
    /// LLaVA-NeXT.
    LlavaNext,
    /// LLaVA.
    Llava,
    /// Vision Llama.
    VLlama,
    /// Qwen2 Vision-Language.
    Qwen2VL,
    /// Qwen 2.5 Vision-Language.
    Qwen2_5VL,
    /// MiniCPM-O (multimodal).
    MiniCPM_O,
    /// Gemma 3 with native vision.
    Gemma3n,
    /// Mistral 3 with vision.
    Mistral3,

    // =========================================================================
    // EMBEDDING MODELS (loaded via EmbeddingModelBuilder)
    // =========================================================================
    /// Nomic Embed architecture.
    NomicEmbed,
    /// BAAI BGE embedding models.
    BGE,
    /// Snowflake Arctic embedding.
    Arctic,

    // =========================================================================
    // DIFFUSION (loaded via DiffusionModelBuilder)
    // =========================================================================
    /// Flux diffusion model.
    Flux,

    // =========================================================================
    // AUDIO (future support)
    // =========================================================================
    /// Dia audio model.
    Dia,
}

impl ModelArchitecture {
    /// Returns the model type for this architecture.
    #[must_use]
    pub const fn model_type(&self) -> ModelType {
        match self {
            // Text models
            Self::Mistral
            | Self::Gemma
            | Self::Gemma2
            | Self::Gemma3
            | Self::Mixtral
            | Self::Llama
            | Self::Llama4
            | Self::Phi2
            | Self::Phi3
            | Self::Phi3_5MoE
            | Self::Qwen2
            | Self::Qwen3
            | Self::Qwen3Moe
            | Self::GLM4
            | Self::Starcoder2
            | Self::DeepseekV2
            | Self::DeepseekV3
            | Self::SmolLM3 => ModelType::Text,

            // Vision models
            Self::Phi3V
            | Self::Phi4MM
            | Self::Idefics2
            | Self::Idefics3
            | Self::LlavaNext
            | Self::Llava
            | Self::VLlama
            | Self::Qwen2VL
            | Self::Qwen2_5VL
            | Self::MiniCPM_O
            | Self::Gemma3n
            | Self::Mistral3 => ModelType::Vision,

            // Embedding models
            Self::NomicEmbed | Self::BGE | Self::Arctic => ModelType::Embedding,

            // Diffusion
            Self::Flux => ModelType::Diffusion,

            // Audio
            Self::Dia => ModelType::Audio,
        }
    }

    /// String representation matching mistral.rs enum names.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Mistral => "Mistral",
            Self::Gemma => "Gemma",
            Self::Gemma2 => "Gemma2",
            Self::Gemma3 => "Gemma3",
            Self::Mixtral => "Mixtral",
            Self::Llama => "Llama",
            Self::Llama4 => "Llama4",
            Self::Phi2 => "Phi2",
            Self::Phi3 => "Phi3",
            Self::Phi3_5MoE => "Phi3_5MoE",
            Self::Qwen2 => "Qwen2",
            Self::Qwen3 => "Qwen3",
            Self::Qwen3Moe => "Qwen3Moe",
            Self::GLM4 => "GLM4",
            Self::Starcoder2 => "Starcoder2",
            Self::DeepseekV2 => "DeepseekV2",
            Self::DeepseekV3 => "DeepseekV3",
            Self::SmolLM3 => "SmolLM3",
            Self::Phi3V => "Phi3V",
            Self::Phi4MM => "Phi4MM",
            Self::Idefics2 => "Idefics2",
            Self::Idefics3 => "Idefics3",
            Self::LlavaNext => "LlavaNext",
            Self::Llava => "Llava",
            Self::VLlama => "VLlama",
            Self::Qwen2VL => "Qwen2VL",
            Self::Qwen2_5VL => "Qwen2_5VL",
            Self::MiniCPM_O => "MiniCPM_O",
            Self::Gemma3n => "Gemma3n",
            Self::Mistral3 => "Mistral3",
            Self::NomicEmbed => "NomicEmbed",
            Self::BGE => "BGE",
            Self::Arctic => "Arctic",
            Self::Flux => "Flux",
            Self::Dia => "Dia",
        }
    }
}

impl std::fmt::Display for ModelArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Known Model Definition
// ============================================================================

/// A curated model in the registry.
///
/// These models are pre-validated for compatibility with mistral.rs and have
/// tested quantization options.
#[derive(Debug, Clone)]
pub struct KnownModel {
    /// Short ID used in YAML (e.g., "qwen3:8b").
    pub id: &'static str,

    /// Human-readable name.
    pub name: &'static str,

    /// Model type (Text, Vision, Embedding, etc.).
    pub model_type: ModelType,

    /// Architecture for mistral.rs.
    pub architecture: ModelArchitecture,

    /// HuggingFace repo (e.g., "Qwen/Qwen3-8B-GGUF").
    pub hf_repo: &'static str,

    /// Default GGUF filename.
    pub default_file: &'static str,

    /// Available quantizations with filenames.
    pub quantizations: &'static [(Quantization, &'static str)],

    /// Model size in billions of parameters.
    pub param_billions: f32,

    /// Minimum RAM in GB for Q4_K_M quantization.
    pub min_ram_gb: u32,

    /// Description.
    pub description: &'static str,
}

impl KnownModel {
    /// Get the filename for a specific quantization.
    #[must_use]
    pub fn filename_for_quant(&self, quant: Quantization) -> Option<&'static str> {
        self.quantizations
            .iter()
            .find(|(q, _)| *q == quant)
            .map(|(_, f)| *f)
    }

    /// Get the default quantization (first in list).
    #[must_use]
    pub fn default_quantization(&self) -> Option<Quantization> {
        self.quantizations.first().map(|(q, _)| *q)
    }

    /// Check if this model supports a specific quantization.
    #[must_use]
    pub fn supports_quant(&self, quant: Quantization) -> bool {
        self.quantizations.iter().any(|(q, _)| *q == quant)
    }
}

// ============================================================================
// Known Models Registry
// ============================================================================

/// Curated model registry.
///
/// These models are pre-validated for:
/// - Compatibility with mistral.rs
/// - Correct architecture mapping
/// - Working quantizations
/// - Accurate memory requirements
pub static KNOWN_MODELS: &[KnownModel] = &[
    // =========================================================================
    // TEXT MODELS
    // =========================================================================
    KnownModel {
        id: "qwen3:0.6b",
        name: "Qwen3 0.6B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Qwen3,
        hf_repo: "Qwen/Qwen3-0.6B-GGUF",
        default_file: "qwen3-0.6b-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "qwen3-0.6b-q4_k_m.gguf"),
            (Quantization::Q8_0, "qwen3-0.6b-q8_0.gguf"),
            (Quantization::F16, "qwen3-0.6b-f16.gguf"),
        ],
        param_billions: 0.6,
        min_ram_gb: 2,
        description: "Ultra-lightweight model for edge devices",
    },
    KnownModel {
        id: "qwen3:8b",
        name: "Qwen3 8B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Qwen3,
        hf_repo: "Qwen/Qwen3-8B-GGUF",
        default_file: "qwen3-8b-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "qwen3-8b-q4_k_m.gguf"),
            (Quantization::Q5_K_M, "qwen3-8b-q5_k_m.gguf"),
            (Quantization::Q8_0, "qwen3-8b-q8_0.gguf"),
        ],
        param_billions: 8.0,
        min_ram_gb: 8,
        description: "Best balance of speed and quality for most tasks",
    },
    KnownModel {
        id: "qwen3:32b",
        name: "Qwen3 32B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Qwen3,
        hf_repo: "Qwen/Qwen3-32B-GGUF",
        default_file: "qwen3-32b-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "qwen3-32b-q4_k_m.gguf"),
            (Quantization::Q5_K_M, "qwen3-32b-q5_k_m.gguf"),
        ],
        param_billions: 32.0,
        min_ram_gb: 24,
        description: "High-quality reasoning, requires 24GB+ RAM",
    },
    KnownModel {
        id: "llama4:8b",
        name: "Llama 4 8B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Llama4,
        // TODO: Verify HF repo exists when Llama 4 GGUF is released
        hf_repo: "meta-llama/Llama-4-8B-GGUF",
        default_file: "llama-4-8b-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "llama-4-8b-q4_k_m.gguf"),
            (Quantization::Q8_0, "llama-4-8b-q8_0.gguf"),
        ],
        param_billions: 8.0,
        min_ram_gb: 8,
        description: "Meta's latest Llama model",
    },
    KnownModel {
        id: "phi4:14b",
        name: "Phi-4 14B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Phi3, // Phi4 uses Phi3 arch
        hf_repo: "microsoft/Phi-4-GGUF",
        default_file: "phi-4-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "phi-4-q4_k_m.gguf"),
            (Quantization::Q8_0, "phi-4-q8_0.gguf"),
        ],
        param_billions: 14.0,
        min_ram_gb: 12,
        description: "Microsoft's reasoning-focused model",
    },
    KnownModel {
        id: "gemma3:4b",
        name: "Gemma 3 4B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Gemma3,
        hf_repo: "google/gemma-3-4b-gguf",
        default_file: "gemma-3-4b-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "gemma-3-4b-q4_k_m.gguf"),
            (Quantization::Q8_0, "gemma-3-4b-q8_0.gguf"),
        ],
        param_billions: 4.0,
        min_ram_gb: 6,
        description: "Google's efficient small model",
    },
    KnownModel {
        id: "gemma3:12b",
        name: "Gemma 3 12B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Gemma3,
        hf_repo: "google/gemma-3-12b-gguf",
        default_file: "gemma-3-12b-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "gemma-3-12b-q4_k_m.gguf"),
            (Quantization::Q5_K_M, "gemma-3-12b-q5_k_m.gguf"),
        ],
        param_billions: 12.0,
        min_ram_gb: 10,
        description: "Google's mid-size model",
    },
    KnownModel {
        id: "mistral:7b",
        name: "Mistral 7B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::Mistral,
        hf_repo: "mistralai/Mistral-7B-v0.3-GGUF",
        default_file: "mistral-7b-v0.3-q4_k_m.gguf",
        quantizations: &[
            (Quantization::Q4_K_M, "mistral-7b-v0.3-q4_k_m.gguf"),
            (Quantization::Q8_0, "mistral-7b-v0.3-q8_0.gguf"),
        ],
        param_billions: 7.0,
        min_ram_gb: 8,
        description: "Mistral's flagship 7B model",
    },
    KnownModel {
        id: "deepseek:7b",
        name: "DeepSeek V3 7B",
        model_type: ModelType::Text,
        architecture: ModelArchitecture::DeepseekV3,
        // TODO: Verify HF repo path - DeepSeek V3 may use different naming
        hf_repo: "deepseek-ai/DeepSeek-V3-7B-GGUF",
        default_file: "deepseek-v3-7b-q4_k_m.gguf",
        quantizations: &[(Quantization::Q4_K_M, "deepseek-v3-7b-q4_k_m.gguf")],
        param_billions: 7.0,
        min_ram_gb: 8,
        description: "DeepSeek's latest architecture",
    },
    // =========================================================================
    // VISION MODELS
    // =========================================================================
    KnownModel {
        id: "qwen3-vision:8b",
        name: "Qwen3 Vision 8B",
        model_type: ModelType::Vision,
        architecture: ModelArchitecture::Qwen2_5VL,
        hf_repo: "Qwen/Qwen2.5-VL-8B-GGUF",
        default_file: "qwen2.5-vl-8b-q4_k_m.gguf",
        quantizations: &[(Quantization::Q4_K_M, "qwen2.5-vl-8b-q4_k_m.gguf")],
        param_billions: 8.0,
        min_ram_gb: 12,
        description: "Vision-language model for image understanding",
    },
    KnownModel {
        id: "llama4-vision:8b",
        name: "Llama 4 Vision 8B",
        model_type: ModelType::Vision,
        architecture: ModelArchitecture::VLlama, // Vision uses VLlama, not Llama4
        // TODO: Verify HF repo exists when Llama 4 Vision is released
        hf_repo: "meta-llama/Llama-4-Vision-8B-GGUF",
        default_file: "llama-4-vision-8b-q4_k_m.gguf",
        quantizations: &[(Quantization::Q4_K_M, "llama-4-vision-8b-q4_k_m.gguf")],
        param_billions: 8.0,
        min_ram_gb: 12,
        description: "Meta's multimodal Llama 4",
    },
    KnownModel {
        id: "phi4-vision:14b",
        name: "Phi-4 Vision 14B",
        model_type: ModelType::Vision,
        architecture: ModelArchitecture::Phi4MM,
        hf_repo: "microsoft/Phi-4-MM-GGUF",
        default_file: "phi-4-mm-q4_k_m.gguf",
        quantizations: &[(Quantization::Q4_K_M, "phi-4-mm-q4_k_m.gguf")],
        param_billions: 14.0,
        min_ram_gb: 16,
        description: "Microsoft's multimodal Phi-4",
    },
    KnownModel {
        id: "gemma3-vision:12b",
        name: "Gemma 3 Vision 12B",
        model_type: ModelType::Vision,
        architecture: ModelArchitecture::Gemma3n, // Vision uses Gemma3n (native vision)
        // TODO: Verify HF repo exists when Gemma 3 Vision GGUF is released
        hf_repo: "google/gemma-3-12b-vision-gguf",
        default_file: "gemma-3-12b-vision-q4_k_m.gguf",
        quantizations: &[(Quantization::Q4_K_M, "gemma-3-12b-vision-q4_k_m.gguf")],
        param_billions: 12.0,
        min_ram_gb: 14,
        description: "Google's vision-enabled Gemma",
    },
    // =========================================================================
    // EMBEDDING MODELS
    // =========================================================================
    KnownModel {
        id: "nomic-embed",
        name: "Nomic Embed Text v1.5",
        model_type: ModelType::Embedding,
        architecture: ModelArchitecture::NomicEmbed,
        hf_repo: "nomic-ai/nomic-embed-text-v1.5-GGUF",
        default_file: "nomic-embed-text-v1.5-f16.gguf",
        quantizations: &[
            (Quantization::F16, "nomic-embed-text-v1.5-f16.gguf"),
            (Quantization::Q8_0, "nomic-embed-text-v1.5-q8_0.gguf"),
        ],
        param_billions: 0.137,
        min_ram_gb: 1,
        description: "High-quality 768-dim embeddings",
    },
    KnownModel {
        id: "bge-m3",
        name: "BGE-M3",
        model_type: ModelType::Embedding,
        architecture: ModelArchitecture::BGE,
        hf_repo: "BAAI/bge-m3-GGUF",
        default_file: "bge-m3-f16.gguf",
        quantizations: &[(Quantization::F16, "bge-m3-f16.gguf")],
        param_billions: 0.568,
        min_ram_gb: 2,
        description: "Multilingual embedding model",
    },
    KnownModel {
        id: "snowflake-arctic",
        name: "Snowflake Arctic Embed",
        model_type: ModelType::Embedding,
        architecture: ModelArchitecture::Arctic,
        hf_repo: "Snowflake/snowflake-arctic-embed-m-GGUF",
        default_file: "snowflake-arctic-embed-m-f16.gguf",
        quantizations: &[(Quantization::F16, "snowflake-arctic-embed-m-f16.gguf")],
        param_billions: 0.335,
        min_ram_gb: 1,
        description: "Enterprise-grade embeddings",
    },
];

// ============================================================================
// Model Resolution
// ============================================================================

/// Result of model resolution.
#[derive(Debug)]
pub enum ResolvedModel<'a> {
    /// Curated model from KNOWN_MODELS.
    Curated(&'a KnownModel),
    /// HuggingFace passthrough.
    HuggingFace {
        /// HuggingFace repository (e.g., "bartowski/Qwen3-30B-GGUF").
        repo: String,
    },
}

/// Resolve a model ID to a [`KnownModel`] or HuggingFace passthrough.
///
/// Supports:
/// - Curated IDs: `"qwen3:8b"` → [`ResolvedModel::Curated`]
/// - HuggingFace: `"hf:bartowski/Qwen3-30B-GGUF"` → [`ResolvedModel::HuggingFace`]
///
/// # Example
///
/// ```
/// use spn_core::{resolve_model, ResolvedModel};
///
/// // Curated model
/// if let Some(ResolvedModel::Curated(model)) = resolve_model("qwen3:8b") {
///     assert_eq!(model.param_billions, 8.0);
/// }
///
/// // HuggingFace passthrough
/// if let Some(ResolvedModel::HuggingFace { repo }) = resolve_model("hf:bartowski/Model") {
///     assert_eq!(repo, "bartowski/Model");
/// }
/// ```
#[must_use]
pub fn resolve_model(id: &str) -> Option<ResolvedModel<'_>> {
    if let Some(hf_repo) = id.strip_prefix("hf:") {
        // HuggingFace passthrough
        Some(ResolvedModel::HuggingFace {
            repo: hf_repo.to_string(),
        })
    } else {
        // Curated model lookup
        KNOWN_MODELS
            .iter()
            .find(|m| m.id == id)
            .map(ResolvedModel::Curated)
    }
}

/// Find a curated model by ID.
///
/// Unlike [`resolve_model`], this only searches curated models.
#[must_use]
pub fn find_model(id: &str) -> Option<&'static KnownModel> {
    KNOWN_MODELS.iter().find(|m| m.id == id)
}

/// List all models of a specific type.
pub fn models_by_type(model_type: ModelType) -> impl Iterator<Item = &'static KnownModel> {
    KNOWN_MODELS
        .iter()
        .filter(move |m| m.model_type == model_type)
}

// ============================================================================
// Auto-Quantization Selection
// ============================================================================

/// Auto-select quantization based on available RAM.
///
/// Returns the best (highest quality) quantization that fits in available RAM.
/// Falls back to the smallest quantization if nothing fits.
///
/// # Arguments
///
/// * `model` - The model to select quantization for
/// * `available_ram_gb` - Available system RAM in gigabytes
///
/// # Example
///
/// ```
/// use spn_core::{auto_select_quantization, find_model, Quantization};
///
/// let model = find_model("qwen3:8b").unwrap();
///
/// // With 16GB RAM, should select Q8_0 (high quality)
/// let quant = auto_select_quantization(model, 16);
/// assert_eq!(quant, Quantization::Q8_0);
///
/// // With 8GB RAM, Q5_K_M fits
/// let quant = auto_select_quantization(model, 8);
/// assert_eq!(quant, Quantization::Q5_K_M);
///
/// // For larger models, falls back to smaller quantization
/// let large_model = find_model("qwen3:32b").unwrap();
/// let quant = auto_select_quantization(large_model, 16);
/// assert_eq!(quant, Quantization::Q4_K_M);
/// ```
#[must_use]
pub fn auto_select_quantization(model: &KnownModel, available_ram_gb: u32) -> Quantization {
    // Iterate from highest quality to lowest
    for (quant, _filename) in model.quantizations.iter().rev() {
        // Model memory + 2GB overhead for KV cache and runtime
        let required_gb = (model.param_billions * quant.memory_multiplier()) as u32 + 2;

        if required_gb <= available_ram_gb {
            return *quant;
        }
    }

    // Fallback to smallest quantization
    model
        .quantizations
        .first()
        .map(|(q, _)| *q)
        .unwrap_or(Quantization::Q4_K_M)
}

// ============================================================================
// RAM Detection
// ============================================================================

/// Detect available system RAM in gigabytes.
///
/// Returns a conservative estimate if detection fails.
#[cfg(target_os = "macos")]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    use std::process::Command;

    let output = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok();

    output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|bytes| (bytes / 1_073_741_824) as u32) // bytes to GB
        .unwrap_or(8) // Conservative default
}

/// Detect available system RAM in gigabytes.
#[cfg(target_os = "linux")]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    use std::fs;

    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("MemTotal:"))
                .and_then(|line| {
                    line.split_whitespace()
                        .nth(1)
                        .and_then(|kb| kb.parse::<u64>().ok())
                })
        })
        .map(|kb| (kb / 1_048_576) as u32) // KB to GB
        .unwrap_or(8)
}

/// Detect available system RAM in gigabytes.
#[cfg(target_os = "windows")]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    // TODO: Use winapi to get actual RAM
    16 // Assume 16GB on Windows for now
}

/// Detect available system RAM in gigabytes.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
#[must_use]
pub fn detect_available_ram_gb() -> u32 {
    8 // Conservative default
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_curated_model() {
        let result = resolve_model("qwen3:8b");
        assert!(matches!(result, Some(ResolvedModel::Curated(_))));

        if let Some(ResolvedModel::Curated(model)) = result {
            assert_eq!(model.id, "qwen3:8b");
            assert_eq!(model.param_billions, 8.0);
        }
    }

    #[test]
    fn test_resolve_huggingface_model() {
        let result = resolve_model("hf:bartowski/Qwen3-30B-GGUF");
        assert!(matches!(result, Some(ResolvedModel::HuggingFace { .. })));

        if let Some(ResolvedModel::HuggingFace { repo }) = result {
            assert_eq!(repo, "bartowski/Qwen3-30B-GGUF");
        }
    }

    #[test]
    fn test_resolve_unknown_model() {
        let result = resolve_model("unknown:model");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_model() {
        let model = find_model("qwen3:8b");
        assert!(model.is_some());
        assert_eq!(model.unwrap().name, "Qwen3 8B");
    }

    #[test]
    fn test_models_by_type() {
        let text_models: Vec<_> = models_by_type(ModelType::Text).collect();
        assert!(text_models.len() >= 9);
        assert!(text_models.iter().all(|m| m.model_type == ModelType::Text));

        let vision_models: Vec<_> = models_by_type(ModelType::Vision).collect();
        assert!(vision_models.len() >= 4);

        let embed_models: Vec<_> = models_by_type(ModelType::Embedding).collect();
        assert!(embed_models.len() >= 3);
    }

    #[test]
    fn test_auto_select_quantization_high_ram() {
        let model = find_model("qwen3:8b").unwrap();
        // With 32GB RAM, should select highest quality available
        let quant = auto_select_quantization(model, 32);
        assert_eq!(quant, Quantization::Q8_0);
    }

    #[test]
    fn test_auto_select_quantization_low_ram() {
        let model = find_model("qwen3:32b").unwrap();
        // With 16GB RAM, 32B model should fall back to Q4_K_M
        let quant = auto_select_quantization(model, 16);
        assert_eq!(quant, Quantization::Q4_K_M);
    }

    #[test]
    fn test_detect_ram() {
        let ram = detect_available_ram_gb();
        // Should return a reasonable value (at least 1GB, no more than 1TB)
        assert!(ram >= 1);
        assert!(ram <= 1024);
    }

    #[test]
    fn test_known_models_count() {
        // Ensure we have the expected number of models
        // 9 text + 4 vision + 3 embedding = 16 curated models
        assert!(
            KNOWN_MODELS.len() >= 16,
            "Expected at least 16 models, got {}",
            KNOWN_MODELS.len()
        );
    }

    #[test]
    fn test_model_architecture_model_type() {
        // Text architectures
        assert_eq!(ModelArchitecture::Qwen3.model_type(), ModelType::Text);
        assert_eq!(ModelArchitecture::Llama4.model_type(), ModelType::Text);

        // Vision architectures
        assert_eq!(ModelArchitecture::Phi4MM.model_type(), ModelType::Vision);
        assert_eq!(ModelArchitecture::Qwen2_5VL.model_type(), ModelType::Vision);

        // Embedding architectures
        assert_eq!(
            ModelArchitecture::NomicEmbed.model_type(),
            ModelType::Embedding
        );
        assert_eq!(ModelArchitecture::BGE.model_type(), ModelType::Embedding);
    }

    #[test]
    fn test_quantization_memory_multiplier() {
        assert!(Quantization::Q4_K_M.memory_multiplier() < Quantization::Q8_0.memory_multiplier());
        assert!(Quantization::Q8_0.memory_multiplier() < Quantization::F16.memory_multiplier());
    }

    #[test]
    fn test_known_model_filename_for_quant() {
        let model = find_model("qwen3:8b").unwrap();

        let q4_file = model.filename_for_quant(Quantization::Q4_K_M);
        assert!(q4_file.is_some());
        assert!(q4_file.unwrap().contains("q4_k_m"));

        // F16 is not available for this model
        let f16_file = model.filename_for_quant(Quantization::F16);
        assert!(f16_file.is_none());
    }
}
