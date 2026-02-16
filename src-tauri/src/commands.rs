use mastering_core::analysis;
use mastering_core::backends::MasteringEngine;
use mastering_core::config::Config;
use mastering_core::pipeline::{self, MasteringJob};
use mastering_core::types::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize)]
pub struct AnalysisResult {
    pub metadata: AudioMetadata,
    pub lufs_integrated: f64,
    pub lufs_short_term_max: f64,
    pub rms_db: f64,
    pub peak_db: f64,
    pub true_peak_db: f64,
    pub dynamic_range_db: f64,
    pub stereo_width: f64,
    pub frequency_bands: FrequencyBands,
}

impl From<AudioAnalysis> for AnalysisResult {
    fn from(a: AudioAnalysis) -> Self {
        Self {
            metadata: a.metadata,
            lufs_integrated: a.lufs_integrated,
            lufs_short_term_max: a.lufs_short_term_max,
            rms_db: a.rms_db,
            peak_db: a.peak_db,
            true_peak_db: a.true_peak_db,
            dynamic_range_db: a.dynamic_range_db,
            stereo_width: a.stereo_width,
            frequency_bands: a.frequency_bands,
        }
    }
}

#[derive(Serialize)]
pub struct MasterResult {
    pub output_path: String,
    pub backend_used: String,
    pub pre_analysis: Option<AnalysisResult>,
    pub post_analysis: Option<AnalysisResult>,
}

#[derive(Serialize)]
pub struct BackendStatus {
    pub name: String,
    pub available: bool,
    pub description: String,
}

#[derive(Serialize)]
pub struct PresetInfo {
    pub name: String,
    pub target_lufs: f64,
    pub description: String,
}

#[derive(Deserialize)]
pub struct MasterRequest {
    pub input_path: String,
    pub output_path: Option<String>,
    pub reference_path: Option<String>,
    pub backend: Option<String>,
    pub ai_provider: Option<String>,
    pub bit_depth: Option<u16>,
    pub format: Option<String>,
    pub target_lufs: Option<f64>,
    pub preset: Option<String>,
    pub no_limiter: bool,
}

#[tauri::command]
pub async fn analyze_file(path: String) -> Result<AnalysisResult, String> {
    let path = PathBuf::from(&path);
    let result = analysis::analyze_file(&path)
        .await
        .map_err(|e| format!("Analysis failed: {e}"))?;
    Ok(result.into())
}

#[tauri::command]
pub async fn master_file(request: MasterRequest) -> Result<MasterResult, String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;

    let backend: Backend = request
        .backend
        .as_deref()
        .unwrap_or("auto")
        .parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let ai_provider: Option<AiProvider> = request
        .ai_provider
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let format: Option<AudioFormat> = request
        .format
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let preset: Option<Preset> = request
        .preset
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let job = MasteringJob {
        input_path: PathBuf::from(&request.input_path),
        output_path: request.output_path.map(PathBuf::from),
        reference_path: request.reference_path.map(PathBuf::from),
        backend,
        ai_provider,
        bit_depth: request.bit_depth,
        format,
        target_lufs: request.target_lufs,
        no_limiter: request.no_limiter,
        preset,
        dry_run: false,
    };

    let result = pipeline::run(&job, &config)
        .await
        .map_err(|e| format!("Mastering failed: {e}"))?;

    Ok(MasterResult {
        output_path: result.output_path.to_string_lossy().to_string(),
        backend_used: result.backend_used,
        pre_analysis: result.pre_analysis.map(|a| a.into()),
        post_analysis: result.post_analysis.map(|a| a.into()),
    })
}

#[tauri::command]
pub fn get_config() -> Result<serde_json::Value, String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    serde_json::to_value(&config).map_err(|e| format!("Serialize error: {e}"))
}

#[tauri::command]
pub fn save_config(config_json: serde_json::Value) -> Result<(), String> {
    let config: Config =
        serde_json::from_value(config_json).map_err(|e| format!("Invalid config: {e}"))?;
    config.save().map_err(|e| format!("Save error: {e}"))
}

#[tauri::command]
pub async fn check_backends() -> Result<Vec<BackendStatus>, String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;

    let backends = vec![
        (
            Backend::Matchering,
            "Reference-based mastering (matches EQ, loudness, stereo width)",
        ),
        (
            Backend::Ai,
            "AI-assisted mastering (LLM suggests DSP parameters)",
        ),
        (
            Backend::LocalMl,
            "Local ML models (DeepAFx-ST, HuggingFace)",
        ),
    ];

    let mut results = Vec::new();
    for (backend, description) in backends {
        let engine = MasteringEngine::from_config(backend, &config);
        let available = engine.check_available().await.unwrap_or(false);
        results.push(BackendStatus {
            name: backend.to_string(),
            available,
            description: description.to_string(),
        });
    }

    Ok(results)
}

#[tauri::command]
pub fn get_presets() -> Vec<PresetInfo> {
    vec![
        PresetInfo {
            name: "streaming".into(),
            target_lufs: Preset::Streaming.target_lufs(),
            description: Preset::Streaming.description().into(),
        },
        PresetInfo {
            name: "cd".into(),
            target_lufs: Preset::Cd.target_lufs(),
            description: Preset::Cd.description().into(),
        },
        PresetInfo {
            name: "vinyl".into(),
            target_lufs: Preset::Vinyl.target_lufs(),
            description: Preset::Vinyl.description().into(),
        },
        PresetInfo {
            name: "loud".into(),
            target_lufs: Preset::Loud.target_lufs(),
            description: Preset::Loud.description().into(),
        },
    ]
}
