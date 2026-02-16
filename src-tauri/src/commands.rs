use mastering_core::analysis;
use mastering_core::analysis::decode::decode_audio;
use mastering_core::backends::MasteringEngine;
use mastering_core::config::Config;
use mastering_core::pipeline::{self, MasteringJob};
use mastering_core::types::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

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
pub struct BackendDiagnostic {
    pub name: String,
    pub available: bool,
    pub description: String,
    pub error: Option<String>,
    pub python_path: String,
    pub scripts_dir: String,
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

#[derive(Serialize)]
pub struct BatchResult {
    pub path: String,
    pub success: bool,
    pub result: Option<MasterResult>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn analyze_file(path: String) -> Result<AnalysisResult, String> {
    let path = PathBuf::from(&path);
    let result = analysis::analyze_file(&path)
        .await
        .map_err(|e| format!("Analysis failed: {e}"))?;
    Ok(result.into())
}

#[tauri::command]
pub async fn get_waveform_data(
    path: String,
    num_points: usize,
) -> Result<Vec<[f32; 2]>, String> {
    let path = PathBuf::from(&path);
    let num_points = if num_points == 0 { 1000 } else { num_points };

    tokio::task::spawn_blocking(move || {
        let decoded = decode_audio(&path).map_err(|e| format!("Decode failed: {e}"))?;

        let mono: Vec<f32> = if decoded.channels == 1 {
            decoded.samples.clone()
        } else {
            decoded
                .samples
                .chunks(decoded.channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / decoded.channels as f32)
                .collect()
        };

        let total = mono.len();
        let bucket_size = (total / num_points).max(1);
        let mut peaks: Vec<[f32; 2]> = Vec::with_capacity(num_points);

        for i in 0..num_points {
            let start = i * bucket_size;
            let end = ((i + 1) * bucket_size).min(total);
            if start >= total {
                break;
            }
            let slice = &mono[start..end];
            let min = slice.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            peaks.push([min, max]);
        }

        Ok(peaks)
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

fn build_job(request: &MasterRequest) -> Result<(MasteringJob, Config), String> {
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
        output_path: request.output_path.as_ref().map(PathBuf::from),
        reference_path: request.reference_path.as_ref().map(PathBuf::from),
        backend,
        ai_provider,
        bit_depth: request.bit_depth,
        format,
        target_lufs: request.target_lufs,
        no_limiter: request.no_limiter,
        preset,
        dry_run: false,
    };

    Ok((job, config))
}

#[tauri::command]
pub async fn master_file(request: MasterRequest) -> Result<MasterResult, String> {
    let (job, config) = build_job(&request)?;

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
pub async fn master_batch(requests: Vec<MasterRequest>) -> Vec<BatchResult> {
    let mut results = Vec::with_capacity(requests.len());

    for request in &requests {
        let path = request.input_path.clone();
        match build_job(request) {
            Ok((job, config)) => match pipeline::run(&job, &config).await {
                Ok(r) => {
                    results.push(BatchResult {
                        path,
                        success: true,
                        result: Some(MasterResult {
                            output_path: r.output_path.to_string_lossy().to_string(),
                            backend_used: r.backend_used,
                            pre_analysis: r.pre_analysis.map(|a| a.into()),
                            post_analysis: r.post_analysis.map(|a| a.into()),
                        }),
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(BatchResult {
                        path,
                        success: false,
                        result: None,
                        error: Some(format!("{e}")),
                    });
                }
            },
            Err(e) => {
                results.push(BatchResult {
                    path,
                    success: false,
                    result: None,
                    error: Some(e),
                });
            }
        }
    }

    results
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
        (Backend::Matchering, "Reference-based mastering"),
        (Backend::Ai, "AI-assisted mastering"),
        (Backend::LocalMl, "Local ML models"),
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
pub async fn diagnose_backends() -> Result<Vec<BackendDiagnostic>, String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let scripts_dir = Config::python_scripts_dir();
    let scripts_dir_str = scripts_dir.display().to_string();

    let backends = vec![
        (Backend::Matchering, "Reference-based mastering (Matchering)", &config.backends.matchering.python_path),
        (Backend::Ai, "AI-assisted mastering (LLM + DSP)", &config.backends.matchering.python_path),
        (Backend::LocalMl, "Local ML models (DeepAFx-ST)", &config.backends.local_ml.python_path),
    ];

    let mut results = Vec::new();
    for (backend, description, python_path) in backends {
        let engine = MasteringEngine::from_config(backend, &config);
        let (available, error) = match engine.check_available().await {
            Ok(true) => (true, None),
            Ok(false) => (false, Some("Backend check returned false. Python dependencies may be missing.".to_string())),
            Err(e) => (false, Some(format!("{e}"))),
        };
        results.push(BackendDiagnostic {
            name: backend.to_string(),
            available,
            description: description.to_string(),
            error,
            python_path: python_path.clone(),
            scripts_dir: scripts_dir_str.clone(),
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
