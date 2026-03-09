# Master Plan Phase B: Multimodal Backends (Candle + mistral.rs)

**Version:** 0.17.0
**Status:** Draft
**Author:** Claude + Thibaut
**Date:** 2026-03-09
**Depends on:** Phase A (v0.16.0)

---

## Executive Summary

Phase B adds **Rust-native multimodal capabilities** via Candle (HuggingFace) and mistral.rs.
This enables image generation, vision analysis, and speech-to-text without Python dependencies.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PHASE B SCOPE                                                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ✅ IN SCOPE                              ❌ OUT OF SCOPE                       │
│  ─────────────────────────────────────    ─────────────────────────────────     │
│  • CandleBackend implementation           • llmfit integration (Phase C)        │
│  • MistralRsBackend implementation        • Hardware-aware scoring              │
│  • Stable Diffusion (image gen)           • Model explorer TUI                  │
│  • Whisper (speech-to-text)               • Cloud image APIs (DALL-E)           │
│  • Vision models (Llama 3.2 Vision)       • Text-to-speech                      │
│  • Phi-3.5 Vision                         • Video generation                    │
│  • Hardware detection (CUDA/Metal)                                              │
│  • Quantized model loading                                                      │
│  • @models/ aliases for multimodal                                              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Research Summary

### Candle (HuggingFace)

**Stats:** 19.6k ⭐ | Pure Rust ML framework | CUDA + Metal support

**Capabilities:**
- Stable Diffusion 1.5/2.1/XL/Turbo
- Whisper (transcription)
- Quantized models (GGML, GGUF, EXL2, AWQ, HQQ, GPTQ)
- LLaMA, Mistral, Phi, Gemma inference
- CUDA 12.x + Metal acceleration

**Key APIs:**
```rust
// Tensor operations
let tensor = Tensor::new(&[1.0f32, 2.0, 3.0], &Device::Cuda(0))?;

// Model loading
let vb = VarBuilder::from_safetensors(path, dtype, &device)?;

// Stable Diffusion
let sd = StableDiffusion::new(vb, clip_vb, unet_vb, vae_vb)?;
let images = sd.generate(&prompt, guidance_scale, steps)?;

// Whisper
let whisper = Whisper::new(vb)?;
let text = whisper.transcribe(&audio_samples)?;
```

### mistral.rs

**Stats:** 6.7k ⭐ | Rust LLM inference | Vision models focus

**Capabilities:**
- Llama 3.2 Vision (11B, 90B)
- Phi-3.5 Vision (4.2B)
- Pixtral 12B
- ISQ (in-situ quantization)
- PagedAttention, Flash Attention
- HTTP server + library modes

**Key APIs:**
```rust
// Library integration
let model = MistralRs::new(
    MistralRsBuilder::new()
        .with_model(ModelKind::VisionPlain("llama-3.2-vision:11b"))
        .with_device_map(DeviceMapMetadata::auto())
        .build()
)?;

// Vision inference
let request = Request::new_chat(
    vec![
        Content::Text("What's in this image?".into()),
        Content::Image(ImageSource::Path("photo.jpg".into())),
    ],
    SamplingParams::default(),
);
let response = model.send(request).await?;

// Hardware detection
let specs = HardwareSpecs::detect()?;
println!("GPU: {:?}, VRAM: {}GB", specs.gpus, specs.total_vram_gb);
```

---

## Architecture

### Target State

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PHASE B ARCHITECTURE                                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐    │
│  │  ModelOrchestrator (from Phase A)                                      │    │
│  │  ├── resolve_intent(ImageGeneration) → CandleBackend + SD-Turbo       │    │
│  │  ├── resolve_intent(SpeechToText) → CandleBackend + Whisper           │    │
│  │  ├── resolve_intent(ImageAnalysis) → MistralRsBackend + Llama-Vision  │    │
│  │  └── route_request(...)                                               │    │
│  └────────────────────────────────────────────────────────────────────────┘    │
│                                       │                                         │
│           ┌───────────────────────────┼───────────────────────────┐             │
│           ▼                           ▼                           ▼             │
│  ┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐   │
│  │  Phase A        │         │  CandleBackend  │         │ MistralRsBackend│   │
│  │  Backends       │         │  (multimodal)   │         │  (vision)       │   │
│  └─────────────────┘         └─────────────────┘         └─────────────────┘   │
│                                      │                           │              │
│                                      ▼                           ▼              │
│                              ┌─────────────────┐         ┌─────────────────┐   │
│                              │ Stable Diffusion│         │ Llama 3.2 Vision│   │
│                              │ Whisper         │         │ Phi-3.5 Vision  │   │
│                              │ LLMs (fallback) │         │ Pixtral 12B     │   │
│                              └─────────────────┘         └─────────────────┘   │
│                                                                                 │
│  Hardware Layer:                                                                │
│  ┌────────────────────────────────────────────────────────────────────────┐    │
│  │  HardwareDetector                                                      │    │
│  │  ├── detect_cuda() → CUDA 12.x available, GPU info                    │    │
│  │  ├── detect_metal() → Apple Silicon, unified memory                   │    │
│  │  └── detect_cpu() → Core count, AVX/AVX2 support                      │    │
│  └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Task 1: Hardware Detection Module

**Purpose:** Detect GPU capabilities to choose optimal device/quantization.

```rust
// crates/spn-backends/src/hardware.rs

use std::process::Command;

/// Detected hardware capabilities
#[derive(Debug, Clone)]
pub struct HardwareSpecs {
    /// CPU cores
    pub cpu_cores: usize,

    /// Total RAM in GB
    pub ram_gb: f64,

    /// Available GPUs
    pub gpus: Vec<GpuDevice>,

    /// Optimal device for ML
    pub optimal_device: DeviceType,
}

#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub name: String,
    pub vram_gb: f64,
    pub compute_capability: Option<(u32, u32)>,  // CUDA
    pub device_type: DeviceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Cpu,
    Cuda(usize),   // GPU index
    Metal,
}

impl HardwareSpecs {
    /// Auto-detect hardware
    pub fn detect() -> Self {
        let cpu_cores = num_cpus::get();
        let ram_gb = Self::detect_ram();
        let gpus = Self::detect_gpus();

        let optimal_device = if gpus.iter().any(|g| g.device_type == DeviceType::Metal) {
            DeviceType::Metal
        } else if let Some(cuda_gpu) = gpus.iter().find(|g| matches!(g.device_type, DeviceType::Cuda(_))) {
            cuda_gpu.device_type
        } else {
            DeviceType::Cpu
        };

        Self { cpu_cores, ram_gb, gpus, optimal_device }
    }

    fn detect_ram() -> f64 {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
                .ok();
            if let Some(out) = output {
                if let Ok(s) = String::from_utf8(out.stdout) {
                    if let Ok(bytes) = s.trim().parse::<u64>() {
                        return bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    }
                }
            }
            16.0  // Default
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Ok(kb) = parts.get(1).unwrap_or(&"0").parse::<u64>() {
                            return kb as f64 / (1024.0 * 1024.0);
                        }
                    }
                }
            }
            16.0
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        16.0
    }

    fn detect_gpus() -> Vec<GpuDevice> {
        let mut gpus = Vec::new();

        // Metal (macOS)
        #[cfg(target_os = "macos")]
        {
            // Check for Apple Silicon
            let output = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok();
            if let Some(out) = output {
                if let Ok(brand) = String::from_utf8(out.stdout) {
                    if brand.contains("Apple") {
                        // Get unified memory
                        let ram = Self::detect_ram();
                        gpus.push(GpuDevice {
                            name: brand.trim().to_string(),
                            vram_gb: ram,  // Unified memory
                            compute_capability: None,
                            device_type: DeviceType::Metal,
                        });
                    }
                }
            }
        }

        // CUDA (nvidia-smi)
        if let Ok(output) = Command::new("nvidia-smi")
            .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
            .output()
        {
            if output.status.success() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    for (idx, line) in stdout.lines().enumerate() {
                        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 2 {
                            let vram_mb: f64 = parts[1].parse().unwrap_or(0.0);
                            gpus.push(GpuDevice {
                                name: parts[0].to_string(),
                                vram_gb: vram_mb / 1024.0,
                                compute_capability: None,  // Could parse from nvidia-smi
                                device_type: DeviceType::Cuda(idx),
                            });
                        }
                    }
                }
            }
        }

        gpus
    }

    /// Recommend quantization based on available VRAM
    pub fn recommend_quantization(&self, model_params_b: f64) -> QuantizationLevel {
        let available_vram = self.gpus.iter().map(|g| g.vram_gb).sum::<f64>();

        // Rule of thumb: FP16 needs ~2 bytes per param, Q4 needs ~0.5 bytes
        let fp16_size_gb = model_params_b * 2.0;
        let q8_size_gb = model_params_b * 1.0;
        let q4_size_gb = model_params_b * 0.5;

        if available_vram >= fp16_size_gb * 1.2 {
            QuantizationLevel::None  // FP16
        } else if available_vram >= q8_size_gb * 1.2 {
            QuantizationLevel::Q8_0
        } else if available_vram >= q4_size_gb * 1.2 {
            QuantizationLevel::Q4_K_M
        } else {
            QuantizationLevel::Q4_K_S  // Smallest
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationLevel {
    None,      // FP16/BF16
    Q8_0,      // 8-bit
    Q6_K,      // 6-bit
    Q5_K_M,    // 5-bit medium
    Q4_K_M,    // 4-bit medium
    Q4_K_S,    // 4-bit small
    Q3_K_M,    // 3-bit medium
    Q2_K,      // 2-bit
}

impl QuantizationLevel {
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Q8_0 => "-q8_0",
            Self::Q6_K => "-q6_k",
            Self::Q5_K_M => "-q5_k_m",
            Self::Q4_K_M => "-q4_k_m",
            Self::Q4_K_S => "-q4_k_s",
            Self::Q3_K_M => "-q3_k_m",
            Self::Q2_K => "-q2_k",
        }
    }
}
```

### Task 2: Candle Backend

```rust
// crates/spn-backends/src/local/candle.rs

use candle_core::{Device, Tensor, DType};
use candle_transformers::models::stable_diffusion;
use candle_transformers::models::whisper;
use crate::traits::{ModelBackend, MultimodalBackend};
use crate::hardware::{HardwareSpecs, DeviceType};
use spn_core::{BackendError, BackendResult, ModelInfo};
use std::path::PathBuf;

const HF_HUB_CACHE: &str = ".cache/huggingface/hub";

/// Candle-based backend for Stable Diffusion and Whisper
pub struct CandleBackend {
    device: Device,
    specs: HardwareSpecs,
    cache_dir: PathBuf,
}

impl CandleBackend {
    pub fn new() -> BackendResult<Self> {
        let specs = HardwareSpecs::detect();
        let device = Self::create_device(&specs)?;
        let cache_dir = dirs::home_dir()
            .map(|h| h.join(HF_HUB_CACHE))
            .unwrap_or_else(|| PathBuf::from(HF_HUB_CACHE));

        Ok(Self { device, specs, cache_dir })
    }

    fn create_device(specs: &HardwareSpecs) -> BackendResult<Device> {
        match specs.optimal_device {
            DeviceType::Metal => {
                #[cfg(feature = "metal")]
                { Ok(Device::new_metal(0)?) }

                #[cfg(not(feature = "metal"))]
                { Ok(Device::Cpu) }
            }
            DeviceType::Cuda(idx) => {
                #[cfg(feature = "cuda")]
                { Ok(Device::new_cuda(idx)?) }

                #[cfg(not(feature = "cuda"))]
                { Ok(Device::Cpu) }
            }
            DeviceType::Cpu => Ok(Device::Cpu),
        }
    }

    /// Check if a model is downloaded
    fn model_downloaded(&self, model_id: &str) -> bool {
        let model_path = self.cache_dir.join(format!("models--{}", model_id.replace('/', "--")));
        model_path.exists()
    }
}

impl ModelBackend for CandleBackend {
    fn id(&self) -> &'static str { "candle" }
    fn name(&self) -> &'static str { "Candle (HuggingFace)" }

    async fn is_running(&self) -> bool {
        // Candle is a library, always "running"
        true
    }

    async fn start(&self) -> BackendResult<()> { Ok(()) }
    async fn stop(&self) -> BackendResult<()> { Ok(()) }

    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        Ok(vec![
            // Stable Diffusion models
            ModelInfo::new("sd-turbo", "stabilityai/sd-turbo")
                .with_family("stable-diffusion")
                .with_modality("text-to-image")
                .with_description("Fast image generation (1-4 steps)"),
            ModelInfo::new("sd-xl", "stabilityai/stable-diffusion-xl-base-1.0")
                .with_family("stable-diffusion")
                .with_modality("text-to-image")
                .with_description("High-quality 1024x1024 images"),
            ModelInfo::new("sd-2.1", "stabilityai/stable-diffusion-2-1")
                .with_family("stable-diffusion")
                .with_modality("text-to-image")
                .with_description("Standard SD 2.1 model"),

            // Whisper models
            ModelInfo::new("whisper-tiny", "openai/whisper-tiny")
                .with_family("whisper")
                .with_modality("speech-to-text")
                .with_parameters(39_000_000),
            ModelInfo::new("whisper-base", "openai/whisper-base")
                .with_family("whisper")
                .with_modality("speech-to-text")
                .with_parameters(74_000_000),
            ModelInfo::new("whisper-small", "openai/whisper-small")
                .with_family("whisper")
                .with_modality("speech-to-text")
                .with_parameters(244_000_000),
            ModelInfo::new("whisper-medium", "openai/whisper-medium")
                .with_family("whisper")
                .with_modality("speech-to-text")
                .with_parameters(769_000_000),
            ModelInfo::new("whisper-large", "openai/whisper-large-v3")
                .with_family("whisper")
                .with_modality("speech-to-text")
                .with_parameters(1_550_000_000),
        ])
    }

    async fn model_info(&self, name: &str) -> BackendResult<ModelInfo> {
        self.list_models().await?
            .into_iter()
            .find(|m| m.name == name)
            .ok_or_else(|| BackendError::ModelNotFound(name.to_string()))
    }

    async fn pull(&self, name: &str, progress: Option<ProgressCallback>) -> BackendResult<()> {
        use hf_hub::{api::sync::Api, Repo, RepoType};

        let model = self.model_info(name).await?;
        let repo_id = model.digest.as_ref().ok_or_else(||
            BackendError::InvalidModel("No HuggingFace repo ID".into())
        )?;

        let api = Api::new()?;
        let repo = api.repo(Repo::new(repo_id.clone(), RepoType::Model));

        // Download model files
        let files = match model.family.as_deref() {
            Some("stable-diffusion") => vec![
                "model_index.json",
                "unet/config.json",
                "unet/diffusion_pytorch_model.safetensors",
                "vae/config.json",
                "vae/diffusion_pytorch_model.safetensors",
                "text_encoder/config.json",
                "text_encoder/model.safetensors",
                "tokenizer/tokenizer.json",
            ],
            Some("whisper") => vec![
                "config.json",
                "model.safetensors",
                "tokenizer.json",
            ],
            _ => vec!["model.safetensors"],
        };

        for (idx, file) in files.iter().enumerate() {
            if let Some(ref cb) = progress {
                cb(PullProgress {
                    status: format!("Downloading {}", file),
                    completed: idx as u64,
                    total: files.len() as u64,
                    digest: Some(file.to_string()),
                });
            }
            repo.get(file)?;
        }

        Ok(())
    }

    async fn delete(&self, name: &str) -> BackendResult<()> {
        let model = self.model_info(name).await?;
        let repo_id = model.digest.as_ref().ok_or_else(||
            BackendError::InvalidModel("No HuggingFace repo ID".into())
        )?;

        let model_path = self.cache_dir.join(format!("models--{}", repo_id.replace('/', "--")));
        if model_path.exists() {
            std::fs::remove_dir_all(&model_path)?;
        }

        Ok(())
    }

    // Text chat not supported (use Phase A backends)
    async fn chat(
        &self,
        _model: &str,
        _messages: &[ChatMessage],
        _options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse> {
        Err(BackendError::NotSupported(
            "Candle backend is for multimodal only. Use Ollama or cloud backends for chat.".into()
        ))
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            chat: false,
            image_generation: true,
            image_analysis: false,
            speech_to_text: true,
            text_to_speech: false,
            embeddings: false,
            streaming: false,
        }
    }
}

/// Multimodal trait implementation
impl MultimodalBackend for CandleBackend {
    /// Generate image from text prompt
    async fn generate_image(
        &self,
        model: &str,
        prompt: &str,
        options: &ImageGenOptions,
    ) -> BackendResult<GeneratedImage> {
        use candle_transformers::models::stable_diffusion as sd;

        let model_info = self.model_info(model).await?;
        let repo_id = model_info.digest.as_ref().unwrap();

        // Load SD pipeline
        let api = hf_hub::api::sync::Api::new()?;
        let repo = api.model(repo_id.clone());

        // Load components
        let tokenizer_path = repo.get("tokenizer/tokenizer.json")?;
        let clip_weights = repo.get("text_encoder/model.safetensors")?;
        let unet_weights = repo.get("unet/diffusion_pytorch_model.safetensors")?;
        let vae_weights = repo.get("vae/diffusion_pytorch_model.safetensors")?;

        let tokenizer = sd::tokenizer::Tokenizer::new(&tokenizer_path)?;
        let text_model = sd::build_clip_transformer(&clip_weights, &self.device)?;
        let unet = sd::build_unet(&unet_weights, &self.device)?;
        let vae = sd::build_vae(&vae_weights, &self.device)?;

        // Generate
        let width = options.width.unwrap_or(512);
        let height = options.height.unwrap_or(512);
        let steps = options.steps.unwrap_or(20);
        let guidance_scale = options.guidance_scale.unwrap_or(7.5);

        let tokens = tokenizer.encode(prompt)?;
        let text_embeddings = text_model.forward(&tokens, &self.device)?;

        // Scheduler (DDIM or DPM)
        let scheduler = sd::ddim::DDIMScheduler::new(steps)?;

        // Initial noise
        let latents = Tensor::randn(0f32, 1f32, (1, 4, height / 8, width / 8), &self.device)?;

        // Denoising loop
        let mut latents = latents;
        for t in scheduler.timesteps() {
            let noise_pred = unet.forward(&latents, t, &text_embeddings)?;
            latents = scheduler.step(&noise_pred, t, &latents)?;
        }

        // Decode
        let image = vae.decode(&latents)?;

        // Convert to PNG bytes
        let image_data = Self::tensor_to_png(&image, width, height)?;

        Ok(GeneratedImage {
            data: image_data,
            format: ImageFormat::Png,
            width,
            height,
            model: model.to_string(),
            seed: options.seed,
        })
    }

    /// Transcribe audio to text
    async fn transcribe(
        &self,
        model: &str,
        audio: &AudioInput,
        options: &TranscribeOptions,
    ) -> BackendResult<TranscriptionResult> {
        use candle_transformers::models::whisper;

        let model_info = self.model_info(model).await?;
        let repo_id = model_info.digest.as_ref().unwrap();

        // Load Whisper model
        let api = hf_hub::api::sync::Api::new()?;
        let repo = api.model(repo_id.clone());

        let config_path = repo.get("config.json")?;
        let weights_path = repo.get("model.safetensors")?;
        let tokenizer_path = repo.get("tokenizer.json")?;

        let config: whisper::Config = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;
        let vb = candle_nn::VarBuilder::from_safetensors(&[weights_path], DType::F32, &self.device)?;
        let model = whisper::model::Whisper::load(&vb, config)?;
        let tokenizer = whisper::tokenizer::Tokenizer::new(&tokenizer_path)?;

        // Load and preprocess audio
        let pcm = match audio {
            AudioInput::Path(path) => Self::load_audio_file(path)?,
            AudioInput::Bytes(bytes) => Self::decode_audio_bytes(bytes)?,
        };

        // Transcribe
        let mel = whisper::audio::pcm_to_mel(&config, &pcm, &self.device)?;
        let segments = whisper::decode(&model, &tokenizer, &mel, options.language.as_deref())?;

        let text = segments.iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let words = if options.word_timestamps {
            Some(segments.into_iter().flat_map(|s| s.words).collect())
        } else {
            None
        };

        Ok(TranscriptionResult {
            text,
            language: options.language.clone(),
            words,
            duration_secs: pcm.len() as f64 / 16000.0,
            model: model.to_string(),
        })
    }
}

impl CandleBackend {
    fn tensor_to_png(tensor: &Tensor, width: u32, height: u32) -> BackendResult<Vec<u8>> {
        use image::{ImageBuffer, Rgb};

        let data = tensor.to_vec3::<f32>()?;
        let mut img = ImageBuffer::new(width, height);

        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r = ((data[0][y as usize][x as usize] + 1.0) * 127.5) as u8;
            let g = ((data[1][y as usize][x as usize] + 1.0) * 127.5) as u8;
            let b = ((data[2][y as usize][x as usize] + 1.0) * 127.5) as u8;
            *pixel = Rgb([r, g, b]);
        }

        let mut bytes = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)?;
        Ok(bytes)
    }

    fn load_audio_file(path: &std::path::Path) -> BackendResult<Vec<f32>> {
        use hound::WavReader;

        let reader = WavReader::open(path)?;
        let spec = reader.spec();

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.into_samples::<f32>().filter_map(Result::ok).collect(),
            hound::SampleFormat::Int => {
                let max = (1 << (spec.bits_per_sample - 1)) as f32;
                reader.into_samples::<i32>()
                    .filter_map(Result::ok)
                    .map(|s| s as f32 / max)
                    .collect()
            }
        };

        // Resample to 16kHz if needed
        if spec.sample_rate != 16000 {
            // Use rubato or similar for resampling
            todo!("Implement resampling")
        }

        Ok(samples)
    }

    fn decode_audio_bytes(bytes: &[u8]) -> BackendResult<Vec<f32>> {
        // Decode WAV/MP3/etc bytes to PCM
        todo!("Implement audio decoding")
    }
}
```

### Task 3: mistral.rs Backend

```rust
// crates/spn-backends/src/local/mistral_rs.rs

use mistralrs::{
    MistralRs, MistralRsBuilder, Model, ModelKind,
    Request, RequestMessage, Response, SamplingParams,
    DeviceMapMetadata, GGMLDType, NormalLoaderType,
};
use crate::traits::{ModelBackend, VisionBackend};
use crate::hardware::HardwareSpecs;
use spn_core::{BackendError, BackendResult, ModelInfo, ChatMessage, ChatResponse};
use std::sync::Arc;
use tokio::sync::RwLock;

/// mistral.rs backend for vision models
pub struct MistralRsBackend {
    specs: HardwareSpecs,
    loaded_model: RwLock<Option<Arc<MistralRs>>>,
    current_model_name: RwLock<Option<String>>,
}

impl MistralRsBackend {
    pub fn new() -> BackendResult<Self> {
        let specs = HardwareSpecs::detect();
        Ok(Self {
            specs,
            loaded_model: RwLock::new(None),
            current_model_name: RwLock::new(None),
        })
    }

    /// Load a vision model
    async fn load_model(&self, name: &str) -> BackendResult<Arc<MistralRs>> {
        // Check if already loaded
        {
            let current = self.current_model_name.read().await;
            let model = self.loaded_model.read().await;
            if current.as_deref() == Some(name) && model.is_some() {
                return Ok(Arc::clone(model.as_ref().unwrap()));
            }
        }

        // Map name to model config
        let model_kind = match name {
            "llama-vision:11b" => ModelKind::VisionPlain {
                model_id: "meta-llama/Llama-3.2-11B-Vision-Instruct".to_string(),
                arch: NormalLoaderType::LlamaVision,
            },
            "llama-vision:90b" => ModelKind::VisionPlain {
                model_id: "meta-llama/Llama-3.2-90B-Vision-Instruct".to_string(),
                arch: NormalLoaderType::LlamaVision,
            },
            "phi-vision" => ModelKind::VisionPlain {
                model_id: "microsoft/Phi-3.5-vision-instruct".to_string(),
                arch: NormalLoaderType::Phi3Vision,
            },
            "pixtral" => ModelKind::VisionPlain {
                model_id: "mistralai/Pixtral-12B-2409".to_string(),
                arch: NormalLoaderType::Pixtral,
            },
            _ => return Err(BackendError::ModelNotFound(name.to_string())),
        };

        // Choose quantization based on hardware
        let quant = self.specs.recommend_quantization(11.0);  // Assume 11B params
        let dtype = match quant {
            QuantizationLevel::None => None,
            QuantizationLevel::Q8_0 => Some(GGMLDType::Q8_0),
            QuantizationLevel::Q4_K_M => Some(GGMLDType::Q4_K_M),
            _ => Some(GGMLDType::Q4_K_S),
        };

        // Build model
        let builder = MistralRsBuilder::new()
            .with_model(model_kind)
            .with_device_map(DeviceMapMetadata::auto())
            .with_isq(dtype);

        let model = Arc::new(builder.build().await?);

        // Update state
        *self.loaded_model.write().await = Some(Arc::clone(&model));
        *self.current_model_name.write().await = Some(name.to_string());

        Ok(model)
    }
}

impl ModelBackend for MistralRsBackend {
    fn id(&self) -> &'static str { "mistral-rs" }
    fn name(&self) -> &'static str { "mistral.rs (Vision)" }

    async fn is_running(&self) -> bool { true }
    async fn start(&self) -> BackendResult<()> { Ok(()) }
    async fn stop(&self) -> BackendResult<()> {
        *self.loaded_model.write().await = None;
        *self.current_model_name.write().await = None;
        Ok(())
    }

    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo::new("llama-vision:11b", "meta-llama/Llama-3.2-11B-Vision-Instruct")
                .with_family("llama")
                .with_modality("vision")
                .with_parameters(11_000_000_000)
                .with_description("Llama 3.2 Vision 11B - image understanding"),
            ModelInfo::new("llama-vision:90b", "meta-llama/Llama-3.2-90B-Vision-Instruct")
                .with_family("llama")
                .with_modality("vision")
                .with_parameters(90_000_000_000)
                .with_description("Llama 3.2 Vision 90B - advanced image understanding"),
            ModelInfo::new("phi-vision", "microsoft/Phi-3.5-vision-instruct")
                .with_family("phi")
                .with_modality("vision")
                .with_parameters(4_200_000_000)
                .with_description("Phi-3.5 Vision - lightweight multimodal"),
            ModelInfo::new("pixtral", "mistralai/Pixtral-12B-2409")
                .with_family("pixtral")
                .with_modality("vision")
                .with_parameters(12_000_000_000)
                .with_description("Pixtral 12B - Mistral's vision model"),
        ])
    }

    async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse> {
        let model = self.load_model(model).await?;

        // Convert messages
        let msgs: Vec<RequestMessage> = messages.iter().map(|m| {
            RequestMessage {
                role: match m.role {
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                    ChatRole::System => "system".to_string(),
                },
                content: m.content.clone(),
                images: m.images.clone(),  // Vision messages can have images
            }
        }).collect();

        let sampling = SamplingParams {
            temperature: options.and_then(|o| o.temperature).map(|t| t as f64),
            max_tokens: options.and_then(|o| o.max_tokens).map(|t| t as usize),
            ..Default::default()
        };

        let request = Request::new_chat(msgs, sampling);
        let response = model.send(request).await?;

        match response {
            Response::Done(completion) => {
                Ok(ChatResponse {
                    message: ChatMessage::assistant(completion.choices[0].message.content.clone()),
                    done: true,
                    eval_count: completion.usage.completion_tokens.map(|n| n as u32),
                    prompt_eval_count: completion.usage.prompt_tokens.map(|n| n as u32),
                    total_duration: None,
                    load_duration: None,
                    prompt_eval_duration: None,
                    eval_duration: None,
                })
            }
            _ => Err(BackendError::Api("Unexpected response type".into())),
        }
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            chat: true,
            image_generation: false,
            image_analysis: true,
            speech_to_text: false,
            text_to_speech: false,
            embeddings: false,
            streaming: true,
        }
    }
}

/// Vision-specific trait
impl VisionBackend for MistralRsBackend {
    /// Analyze an image with a prompt
    async fn analyze_image(
        &self,
        model: &str,
        image: &ImageInput,
        prompt: &str,
        options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse> {
        let model = self.load_model(model).await?;

        // Load image as base64
        let image_data = match image {
            ImageInput::Path(path) => {
                let bytes = std::fs::read(path)?;
                base64::encode(&bytes)
            }
            ImageInput::Url(url) => url.to_string(),
            ImageInput::Base64(b64) => b64.clone(),
        };

        let messages = vec![
            RequestMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                images: Some(vec![image_data]),
            }
        ];

        let sampling = SamplingParams {
            temperature: options.and_then(|o| o.temperature).map(|t| t as f64),
            max_tokens: options.and_then(|o| o.max_tokens).map(|t| t as usize),
            ..Default::default()
        };

        let request = Request::new_chat(messages, sampling);
        let response = model.send(request).await?;

        match response {
            Response::Done(completion) => {
                Ok(ChatResponse {
                    message: ChatMessage::assistant(completion.choices[0].message.content.clone()),
                    done: true,
                    eval_count: completion.usage.completion_tokens.map(|n| n as u32),
                    prompt_eval_count: completion.usage.prompt_tokens.map(|n| n as u32),
                    ..Default::default()
                })
            }
            _ => Err(BackendError::Api("Unexpected response type".into())),
        }
    }
}
```

### Task 4: Multimodal Traits

```rust
// crates/spn-backends/src/traits.rs (additions)

use spn_core::{BackendResult, ChatResponse, ChatOptions};
use std::path::Path;

/// Image generation options
#[derive(Debug, Clone, Default)]
pub struct ImageGenOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub steps: Option<u32>,
    pub guidance_scale: Option<f32>,
    pub negative_prompt: Option<String>,
    pub seed: Option<u64>,
}

/// Generated image result
#[derive(Debug)]
pub struct GeneratedImage {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub model: String,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

/// Audio input for transcription
#[derive(Debug)]
pub enum AudioInput {
    Path(std::path::PathBuf),
    Bytes(Vec<u8>),
}

/// Transcription options
#[derive(Debug, Clone, Default)]
pub struct TranscribeOptions {
    pub language: Option<String>,
    pub word_timestamps: bool,
    pub task: TranscribeTask,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TranscribeTask {
    #[default]
    Transcribe,
    Translate,
}

/// Transcription result
#[derive(Debug)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub words: Option<Vec<WordTimestamp>>,
    pub duration_secs: f64,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

/// Image input for analysis
#[derive(Debug)]
pub enum ImageInput {
    Path(std::path::PathBuf),
    Url(String),
    Base64(String),
}

/// Backend capabilities for multimodal
#[derive(Debug, Clone, Default)]
pub struct BackendCapabilities {
    pub chat: bool,
    pub image_generation: bool,
    pub image_analysis: bool,
    pub speech_to_text: bool,
    pub text_to_speech: bool,
    pub embeddings: bool,
    pub streaming: bool,
}

/// Trait for backends that support image generation
#[async_trait::async_trait]
pub trait MultimodalBackend: ModelBackend {
    /// Generate image from text prompt
    async fn generate_image(
        &self,
        model: &str,
        prompt: &str,
        options: &ImageGenOptions,
    ) -> BackendResult<GeneratedImage>;

    /// Transcribe audio to text
    async fn transcribe(
        &self,
        model: &str,
        audio: &AudioInput,
        options: &TranscribeOptions,
    ) -> BackendResult<TranscriptionResult>;
}

/// Trait for backends that support image analysis
#[async_trait::async_trait]
pub trait VisionBackend: ModelBackend {
    /// Analyze an image with a prompt
    async fn analyze_image(
        &self,
        model: &str,
        image: &ImageInput,
        prompt: &str,
        options: Option<&ChatOptions>,
    ) -> BackendResult<ChatResponse>;
}
```

### Task 5: Update BackendKind and Registry

```rust
// crates/spn-backends/src/registry.rs (additions)

/// Backend identifier (updated with Phase B)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    // Phase A: Local
    Ollama,

    // Phase A: Cloud
    Anthropic,
    OpenAI,
    Mistral,
    Groq,
    DeepSeek,
    Gemini,

    // Phase B: Multimodal
    Candle,
    MistralRs,
}

impl BackendKind {
    pub fn is_multimodal(&self) -> bool {
        matches!(self, Self::Candle | Self::MistralRs)
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Ollama => "llama3.2:8b",
            Self::Anthropic => "claude-sonnet",
            Self::OpenAI => "gpt-4o",
            Self::Candle => "sd-turbo",
            Self::MistralRs => "llama-vision:11b",
            _ => "",
        }
    }
}
```

### Task 6: Model Alias Updates

```rust
// crates/spn-backends/src/model_ref.rs (additions)

impl ModelAlias {
    pub fn resolve(&self) -> (BackendKind, String) {
        let full_name = self.variant.as_ref()
            .map(|v| format!("{}:{}", self.name, v))
            .unwrap_or_else(|| self.name.clone());

        match self.name.as_str() {
            // Phase B: Candle (image/audio)
            "sd-turbo" | "sd-xl" | "sd-2.1" | "stable-diffusion" => {
                (BackendKind::Candle, self.name.clone())
            }
            "whisper" | "whisper-tiny" | "whisper-base" | "whisper-small"
            | "whisper-medium" | "whisper-large" => {
                (BackendKind::Candle, self.name.clone())
            }

            // Phase B: mistral.rs (vision)
            "llama-vision" => {
                (BackendKind::MistralRs, full_name)
            }
            "phi-vision" | "pixtral" => {
                (BackendKind::MistralRs, self.name.clone())
            }

            // ... existing Phase A mappings ...
            _ => self.resolve_phase_a()  // Delegate to Phase A logic
        }
    }
}
```

### Task 7: CLI Commands

```rust
// crates/spn/src/commands/model.rs (additions)

/// Generate image from text prompt
#[derive(Debug, Parser)]
pub struct GenerateImageCmd {
    /// Text prompt describing the image
    pub prompt: String,

    /// Model to use (default: sd-turbo)
    #[arg(short, long, default_value = "sd-turbo")]
    pub model: String,

    /// Output file path
    #[arg(short, long, default_value = "output.png")]
    pub output: PathBuf,

    /// Image width
    #[arg(long, default_value = "512")]
    pub width: u32,

    /// Image height
    #[arg(long, default_value = "512")]
    pub height: u32,

    /// Number of diffusion steps
    #[arg(long, default_value = "20")]
    pub steps: u32,

    /// Guidance scale
    #[arg(long, default_value = "7.5")]
    pub guidance: f32,

    /// Random seed
    #[arg(long)]
    pub seed: Option<u64>,
}

impl GenerateImageCmd {
    pub async fn run(&self, orchestrator: &ModelOrchestrator) -> Result<()> {
        println!("{}", ds::info_line(&format!(
            "Generating image with {} ({} steps, {}x{})",
            self.model, self.steps, self.width, self.height
        )));

        let model_ref = ModelRef::parse(&format!("@models/{}", self.model));
        let (backend, model_name) = orchestrator.get_backend(&model_ref).await?;

        // Check if backend supports image generation
        let multimodal = backend.as_multimodal()
            .ok_or_else(|| anyhow!("Backend does not support image generation"))?;

        let options = ImageGenOptions {
            width: Some(self.width),
            height: Some(self.height),
            steps: Some(self.steps),
            guidance_scale: Some(self.guidance),
            seed: self.seed,
            ..Default::default()
        };

        let image = multimodal.generate_image(&model_name, &self.prompt, &options).await?;

        std::fs::write(&self.output, &image.data)?;

        println!("{}", ds::success_line(&format!(
            "Image saved to {} ({}x{})",
            self.output.display(), image.width, image.height
        )));

        Ok(())
    }
}

/// Transcribe audio file
#[derive(Debug, Parser)]
pub struct TranscribeCmd {
    /// Audio file path
    pub input: PathBuf,

    /// Model to use (default: whisper-small)
    #[arg(short, long, default_value = "whisper-small")]
    pub model: String,

    /// Language code (auto-detect if not specified)
    #[arg(short, long)]
    pub language: Option<String>,

    /// Include word timestamps
    #[arg(long)]
    pub timestamps: bool,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: TranscribeFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TranscribeFormat {
    Text,
    Json,
    Srt,
    Vtt,
}

impl TranscribeCmd {
    pub async fn run(&self, orchestrator: &ModelOrchestrator) -> Result<()> {
        println!("{}", ds::info_line(&format!(
            "Transcribing {} with {}",
            self.input.display(), self.model
        )));

        let model_ref = ModelRef::parse(&format!("@models/{}", self.model));
        let (backend, model_name) = orchestrator.get_backend(&model_ref).await?;

        let multimodal = backend.as_multimodal()
            .ok_or_else(|| anyhow!("Backend does not support transcription"))?;

        let audio = AudioInput::Path(self.input.clone());
        let options = TranscribeOptions {
            language: self.language.clone(),
            word_timestamps: self.timestamps,
            ..Default::default()
        };

        let result = multimodal.transcribe(&model_name, &audio, &options).await?;

        match self.format {
            TranscribeFormat::Text => println!("{}", result.text),
            TranscribeFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            TranscribeFormat::Srt => {
                Self::print_srt(&result);
            }
            TranscribeFormat::Vtt => {
                Self::print_vtt(&result);
            }
        }

        Ok(())
    }

    fn print_srt(result: &TranscriptionResult) {
        if let Some(words) = &result.words {
            for (i, segment) in Self::group_words(words, 10).iter().enumerate() {
                println!("{}", i + 1);
                println!("{} --> {}",
                    Self::format_srt_time(segment.0),
                    Self::format_srt_time(segment.1)
                );
                println!("{}\n", segment.2);
            }
        } else {
            println!("1");
            println!("00:00:00,000 --> {:08}", Self::format_srt_time(result.duration_secs));
            println!("{}", result.text);
        }
    }
}

/// Analyze image with vision model
#[derive(Debug, Parser)]
pub struct AnalyzeCmd {
    /// Image file path or URL
    pub input: String,

    /// Question or prompt about the image
    pub prompt: String,

    /// Model to use (default: llama-vision:11b)
    #[arg(short, long, default_value = "llama-vision:11b")]
    pub model: String,
}

impl AnalyzeCmd {
    pub async fn run(&self, orchestrator: &ModelOrchestrator) -> Result<()> {
        println!("{}", ds::info_line(&format!(
            "Analyzing image with {}",
            self.model
        )));

        let model_ref = ModelRef::parse(&format!("@models/{}", self.model));
        let (backend, model_name) = orchestrator.get_backend(&model_ref).await?;

        let vision = backend.as_vision()
            .ok_or_else(|| anyhow!("Backend does not support image analysis"))?;

        let image = if self.input.starts_with("http") {
            ImageInput::Url(self.input.clone())
        } else {
            ImageInput::Path(PathBuf::from(&self.input))
        };

        let response = vision.analyze_image(&model_name, &image, &self.prompt, None).await?;

        println!("{}", response.message.content);

        Ok(())
    }
}
```

### Task 8: MCP Tools for Multimodal

```rust
// crates/spn-mcp/src/tools/multimodal.rs

/// MCP tool: spn_image_generate
pub struct ImageGenerateTool {
    orchestrator: Arc<ModelOrchestrator>,
}

impl Tool for ImageGenerateTool {
    fn name(&self) -> &str { "spn_image_generate" }

    fn description(&self) -> &str {
        "Generate images from text prompts using Stable Diffusion"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Text description of the image to generate"
                },
                "model": {
                    "type": "string",
                    "enum": ["sd-turbo", "sd-xl", "sd-2.1"],
                    "default": "sd-turbo"
                },
                "width": { "type": "integer", "default": 512 },
                "height": { "type": "integer", "default": 512 },
                "steps": { "type": "integer", "default": 20 },
                "guidance_scale": { "type": "number", "default": 7.5 }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let prompt = params["prompt"].as_str().unwrap();
        let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("sd-turbo");

        let options = ImageGenOptions {
            width: params.get("width").and_then(|v| v.as_u64()).map(|n| n as u32),
            height: params.get("height").and_then(|v| v.as_u64()).map(|n| n as u32),
            steps: params.get("steps").and_then(|v| v.as_u64()).map(|n| n as u32),
            guidance_scale: params.get("guidance_scale").and_then(|v| v.as_f64()).map(|n| n as f32),
            ..Default::default()
        };

        let model_ref = ModelRef::parse(&format!("@models/{}", model));
        let (backend, model_name) = self.orchestrator.get_backend(&model_ref).await?;

        let multimodal = backend.as_multimodal()
            .ok_or("Backend does not support image generation")?;

        let image = multimodal.generate_image(&model_name, prompt, &options).await?;

        // Return base64 image
        Ok(serde_json::json!({
            "image_base64": base64::encode(&image.data),
            "format": "png",
            "width": image.width,
            "height": image.height,
            "model": image.model
        }))
    }
}

/// MCP tool: spn_transcribe
pub struct TranscribeTool {
    orchestrator: Arc<ModelOrchestrator>,
}

impl Tool for TranscribeTool {
    fn name(&self) -> &str { "spn_transcribe" }

    fn description(&self) -> &str {
        "Transcribe audio files to text using Whisper"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "audio_path": {
                    "type": "string",
                    "description": "Path to audio file (WAV, MP3, etc.)"
                },
                "model": {
                    "type": "string",
                    "enum": ["whisper-tiny", "whisper-base", "whisper-small", "whisper-medium", "whisper-large"],
                    "default": "whisper-small"
                },
                "language": {
                    "type": "string",
                    "description": "Language code (e.g., 'en', 'fr'). Auto-detect if not specified."
                },
                "word_timestamps": {
                    "type": "boolean",
                    "default": false
                }
            },
            "required": ["audio_path"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let audio_path = params["audio_path"].as_str().unwrap();
        let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("whisper-small");

        let options = TranscribeOptions {
            language: params.get("language").and_then(|v| v.as_str()).map(String::from),
            word_timestamps: params.get("word_timestamps").and_then(|v| v.as_bool()).unwrap_or(false),
            ..Default::default()
        };

        let model_ref = ModelRef::parse(&format!("@models/{}", model));
        let (backend, model_name) = self.orchestrator.get_backend(&model_ref).await?;

        let multimodal = backend.as_multimodal()
            .ok_or("Backend does not support transcription")?;

        let audio = AudioInput::Path(PathBuf::from(audio_path));
        let result = multimodal.transcribe(&model_name, &audio, &options).await?;

        Ok(serde_json::json!({
            "text": result.text,
            "language": result.language,
            "duration_secs": result.duration_secs,
            "words": result.words
        }))
    }
}

/// MCP tool: spn_image_analyze
pub struct ImageAnalyzeTool {
    orchestrator: Arc<ModelOrchestrator>,
}

impl Tool for ImageAnalyzeTool {
    fn name(&self) -> &str { "spn_image_analyze" }

    fn description(&self) -> &str {
        "Analyze images using vision models (Llama Vision, Phi Vision, Pixtral)"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "image": {
                    "type": "string",
                    "description": "Image path, URL, or base64 data"
                },
                "prompt": {
                    "type": "string",
                    "description": "Question or analysis prompt"
                },
                "model": {
                    "type": "string",
                    "enum": ["llama-vision:11b", "llama-vision:90b", "phi-vision", "pixtral"],
                    "default": "llama-vision:11b"
                }
            },
            "required": ["image", "prompt"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let image_str = params["image"].as_str().unwrap();
        let prompt = params["prompt"].as_str().unwrap();
        let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("llama-vision:11b");

        let image = if image_str.starts_with("http") {
            ImageInput::Url(image_str.to_string())
        } else if image_str.starts_with("data:image") || image_str.len() > 1000 {
            ImageInput::Base64(image_str.to_string())
        } else {
            ImageInput::Path(PathBuf::from(image_str))
        };

        let model_ref = ModelRef::parse(&format!("@models/{}", model));
        let (backend, model_name) = self.orchestrator.get_backend(&model_ref).await?;

        let vision = backend.as_vision()
            .ok_or("Backend does not support image analysis")?;

        let response = vision.analyze_image(&model_name, &image, prompt, None).await?;

        Ok(serde_json::json!({
            "analysis": response.message.content,
            "model": model
        }))
    }
}
```

---

## File Changes Summary

| File | Action | LOC |
|------|--------|-----|
| `crates/spn-backends/src/hardware.rs` | Create | ~200 |
| `crates/spn-backends/src/local/candle.rs` | Create | ~500 |
| `crates/spn-backends/src/local/mistral_rs.rs` | Create | ~350 |
| `crates/spn-backends/src/traits.rs` | Update | +150 |
| `crates/spn-backends/src/registry.rs` | Update | +50 |
| `crates/spn-backends/src/model_ref.rs` | Update | +30 |
| `crates/spn-backends/Cargo.toml` | Update | +30 |
| `crates/spn/src/commands/model.rs` | Update | +300 |
| `crates/spn-mcp/src/tools/multimodal.rs` | Create | ~300 |

**Total:** ~1,910 LOC

---

## Cargo.toml Updates

```toml
# crates/spn-backends/Cargo.toml (Phase B additions)

[features]
default = ["ollama", "cloud-anthropic", "cloud-openai"]

# Phase B: Multimodal
candle = ["dep:candle-core", "dep:candle-nn", "dep:candle-transformers", "dep:hf-hub"]
candle-cuda = ["candle", "candle-core/cuda"]
candle-metal = ["candle", "candle-core/metal"]
mistral-rs = ["dep:mistralrs"]
multimodal = ["candle", "mistral-rs"]

[dependencies]
# Phase B: Candle
candle-core = { version = "0.8", optional = true }
candle-nn = { version = "0.8", optional = true }
candle-transformers = { version = "0.8", optional = true }
hf-hub = { version = "0.3", features = ["tokio"], optional = true }

# Phase B: mistral.rs
mistralrs = { version = "0.4", optional = true }

# Audio processing
hound = { version = "3.5", optional = true }  # WAV
rubato = { version = "0.14", optional = true }  # Resampling

# Image processing
image = { version = "0.25", optional = true }
base64 = "0.21"

# Hardware detection
num_cpus = "1.16"
```

---

## Verification Checklist

- [ ] `cargo build --workspace --features multimodal` passes
- [ ] Hardware detection works on macOS (Metal) and Linux (CUDA)
- [ ] `spn model list --backend candle` shows SD and Whisper models
- [ ] `spn model pull sd-turbo` downloads from HuggingFace
- [ ] `spn model generate "a cat" --output cat.png` creates image
- [ ] `spn model transcribe audio.wav` outputs text
- [ ] `spn model analyze photo.jpg "What's in this image?"` works
- [ ] MCP tools `spn_image_generate`, `spn_transcribe`, `spn_image_analyze` work
- [ ] Nika workflow with `intent: image-generation` routes correctly

---

## Commit Strategy

```bash
# Commit 1: Hardware detection
feat(backends): add HardwareSpecs detection for GPU/VRAM

# Commit 2: Candle backend
feat(backends): add CandleBackend for Stable Diffusion and Whisper

# Commit 3: mistral.rs backend
feat(backends): add MistralRsBackend for vision models

# Commit 4: Multimodal traits
feat(backends): add MultimodalBackend and VisionBackend traits

# Commit 5: CLI commands
feat(cli): add model generate, transcribe, analyze commands

# Commit 6: MCP tools
feat(mcp): add multimodal MCP tools
```

---

## Dependencies

**New crates:**
- `candle-core` (0.8.x) - Tensor operations
- `candle-nn` (0.8.x) - Neural network layers
- `candle-transformers` (0.8.x) - Pre-built model architectures
- `hf-hub` (0.3.x) - HuggingFace model downloads
- `mistralrs` (0.4.x) - Vision model inference
- `hound` (3.5.x) - WAV audio loading
- `rubato` (0.14.x) - Audio resampling
- `image` (0.25.x) - Image encoding/decoding

**Size impact:**
- Candle (CUDA): ~50MB additional binary size
- Candle (Metal): ~30MB additional binary size
- mistral.rs: ~40MB additional binary size

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Large binary size | Feature flags, separate binaries |
| CUDA version mismatch | Document supported CUDA versions |
| Model download size | Progress callbacks, resume support |
| Memory exhaustion | Hardware-aware quantization |
| HuggingFace rate limits | Local caching, retry logic |

---

## Success Criteria

1. **Image Generation:** SD-Turbo generates 512x512 in <5s on M1
2. **Transcription:** Whisper-small transcribes 1min audio in <30s
3. **Vision:** Llama-Vision:11B analyzes image in <10s
4. **Integration:** Nika workflows can use multimodal models
5. **CLI UX:** Commands feel natural (`spn model generate`, `spn model transcribe`)
