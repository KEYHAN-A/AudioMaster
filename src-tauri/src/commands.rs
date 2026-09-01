use mastering_core::album::{self, AlbumJob};
use mastering_core::analysis;
use mastering_core::analysis::decode::decode_audio;
use mastering_core::backends::MasteringEngine;
use mastering_core::config::Config;
use mastering_core::error::MasteringError;
use mastering_core::pipeline::{self, MasteringJob};
use mastering_core::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::Emitter;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Structured error response for the frontend.
#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
    pub code: String,
    pub can_retry: bool,
    pub can_fallback: bool,
    pub suggested_action: Option<String>,
    pub details: Option<String>,
}

impl From<MasteringError> for ErrorResponse {
    fn from(err: MasteringError) -> Self {
        let (code, can_retry, can_fallback, suggested_action) = match &err {
            MasteringError::NetworkTimeout { can_retry, .. } => (
                "NETWORK_TIMEOUT".to_string(),
                *can_retry,
                true,
                Some(err.user_message()),
            ),
            MasteringError::AudioDecodeFailed { .. } => (
                "AUDIO_DECODE_FAILED".to_string(),
                false,
                false,
                Some(err.user_message()),
            ),
            MasteringError::PythonUnavailable { .. } => (
                "PYTHON_UNAVAILABLE".to_string(),
                false,
                false,
                Some(err.user_message()),
            ),
            MasteringError::ApiQuotaExceeded { .. } => (
                "API_QUOTA_EXCEEDED".to_string(),
                false,
                true,
                Some(err.user_message()),
            ),
            MasteringError::InvalidConfig { .. } => (
                "INVALID_CONFIG".to_string(),
                false,
                false,
                Some(err.user_message()),
            ),
            MasteringError::FileIo { .. } => ("FILE_IO_ERROR".to_string(), false, false, None),
            MasteringError::BackendError { can_fallback, .. } => {
                ("BACKEND_ERROR".to_string(), true, *can_fallback, None)
            }
            MasteringError::ProcessingError { .. } => {
                ("PROCESSING_ERROR".to_string(), true, false, None)
            }
            MasteringError::ValidationError { .. } => {
                ("VALIDATION_ERROR".to_string(), true, false, None)
            }
            MasteringError::Generic { .. } => ("UNKNOWN_ERROR".to_string(), false, false, None),
        };

        ErrorResponse {
            message: err.to_string(),
            code,
            can_retry,
            can_fallback,
            suggested_action,
            details: None,
        }
    }
}

/// Helper specifically for MasteringError.
pub fn mastering_error_to_response(err: MasteringError) -> String {
    serde_json::to_string(&ErrorResponse::from(err)).unwrap_or_else(|_| {
        r#"{"message":"Unknown error","code":"UNKNOWN_ERROR","can_retry":false,"can_fallback":false}"#.to_string()
    })
}

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AnalysisResult {
    pub schema_version: u16,
    pub metadata: AudioMetadata,
    pub lufs_integrated: f64,
    pub lufs_short_term_max: f64,
    pub lufs_momentary_max: f64,
    pub loudness_range_lu: f64,
    pub rms_db: f64,
    pub peak_db: f64,
    pub true_peak_db: f64,
    pub dynamic_range_db: f64,
    pub crest_factor_db: f64,
    pub peak_to_loudness_ratio: f64,
    pub stereo_width: f64,
    pub stereo_correlation: f64,
    pub dc_offset: f64,
    pub clipped_samples: u64,
    pub frequency_bands: FrequencyBands,
}

impl From<AudioAnalysis> for AnalysisResult {
    fn from(a: AudioAnalysis) -> Self {
        Self {
            schema_version: a.schema_version,
            metadata: a.metadata,
            lufs_integrated: a.lufs_integrated,
            lufs_short_term_max: a.lufs_short_term_max,
            lufs_momentary_max: a.lufs_momentary_max,
            loudness_range_lu: a.loudness_range_lu,
            rms_db: a.rms_db,
            peak_db: a.peak_db,
            true_peak_db: a.true_peak_db,
            dynamic_range_db: a.dynamic_range_db,
            crest_factor_db: a.crest_factor_db,
            peak_to_loudness_ratio: a.peak_to_loudness_ratio,
            stereo_width: a.stereo_width,
            stereo_correlation: a.stereo_correlation,
            dc_offset: a.dc_offset,
            clipped_samples: a.clipped_samples,
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
    pub params_applied: Option<MasteringParams>,
    pub warnings: Vec<MasteringWarning>,
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
    pub job_id: Option<String>,
    #[serde(default)]
    pub overwrite: bool,
    pub input_path: String,
    pub output_path: Option<String>,
    pub reference_path: Option<String>,
    pub backend: Option<String>,
    pub ai_provider: Option<String>,
    pub lmstudio_model: Option<String>,
    pub bit_depth: Option<u16>,
    pub format: Option<String>,
    pub target_lufs: Option<f64>,
    pub preset: Option<String>,
    pub no_limiter: bool,
}

#[derive(Clone, Serialize)]
struct MasteringProgressEvent {
    job_id: String,
    progress: mastering_core::control::JobProgress,
}

static ACTIVE_JOBS: OnceLock<Mutex<HashMap<String, mastering_core::control::ProcessingControl>>> =
    OnceLock::new();

fn active_jobs() -> &'static Mutex<HashMap<String, mastering_core::control::ProcessingControl>> {
    ACTIVE_JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Serialize)]
pub struct BatchResult {
    pub path: String,
    pub success: bool,
    pub result: Option<MasterResult>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct PreviewResult {
    pub original_path: String,
    pub mastered_path: String,
    pub duration_seconds: f64,
    pub level_match_gain_db: f64,
}

#[derive(Deserialize)]
pub struct AlbumRequest {
    pub input_paths: Vec<String>,
    pub output_directory: String,
    pub reference_path: Option<String>,
    pub backend: Option<String>,
    pub ai_provider: Option<String>,
    pub bit_depth: Option<u16>,
    pub format: Option<String>,
    pub target_lufs: Option<f64>,
    pub preset: Option<String>,
    pub no_limiter: bool,
    pub max_relative_offset_lu: Option<f64>,
    #[serde(default)]
    pub track_offsets_lu: Vec<f64>,
}

#[derive(Serialize)]
pub struct AlbumTrackMasterResult {
    pub input_path: String,
    pub assigned_target_lufs: f64,
    pub result: MasterResult,
    pub sha256: String,
}

#[derive(Serialize)]
pub struct AlbumMasterResult {
    pub album_target_lufs: f64,
    pub source_median_lufs: f64,
    pub delivered_loudness_spread_lu: f64,
    pub tracks: Vec<AlbumTrackMasterResult>,
    pub report_path: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn analyze_file(path: String) -> Result<AnalysisResult, String> {
    let path = PathBuf::from(&path);

    // Validate input
    if !path.exists() {
        return Err(mastering_error_to_response(MasteringError::FileIo {
            message: "File not found".to_string(),
            path: Some(path.clone()),
        }));
    }

    let result = analysis::analyze_file(&path)
        .await
        .map_err(|e| mastering_error_to_response(e.into()))?;
    Ok(result.into())
}

#[tauri::command]
pub async fn get_waveform_data(path: String, num_points: usize) -> Result<Vec<[f32; 2]>, String> {
    let path = PathBuf::from(&path);
    let num_points = if num_points == 0 { 1000 } else { num_points };

    tokio::task::spawn_blocking(move || {
        let decoded = decode_audio(&path).map_err(|e| {
            mastering_error_to_response(MasteringError::audio_decode_failed(
                path.display().to_string(),
                e.to_string(),
            ))
        })?;

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
    .map_err(|e| {
        mastering_error_to_response(MasteringError::Generic {
            message: format!("Task failed: {e}"),
            source: None,
        })
    })?
}

fn build_job(request: &MasterRequest) -> Result<(MasteringJob, Config), String> {
    let config = load_desktop_config().map_err(|e| {
        mastering_error_to_response(MasteringError::InvalidConfig {
            message: e,
            config_key: Some("credentials".into()),
        })
    })?;

    let backend: Backend = request
        .backend
        .as_deref()
        .unwrap_or("auto")
        .parse()
        .map_err(|e| {
            mastering_error_to_response(MasteringError::InvalidConfig {
                message: format!("Invalid backend: {}", e),
                config_key: Some("backend".to_string()),
            })
        })?;

    let ai_provider: Option<AiProvider> = request
        .ai_provider
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e| {
            mastering_error_to_response(MasteringError::InvalidConfig {
                message: format!("Invalid AI provider: {}", e),
                config_key: Some("ai_provider".to_string()),
            })
        })?;

    let format: Option<AudioFormat> = request
        .format
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e| {
            mastering_error_to_response(MasteringError::InvalidConfig {
                message: format!("Invalid format: {}", e),
                config_key: Some("format".to_string()),
            })
        })?;

    let preset: Option<Preset> = request
        .preset
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e| {
            mastering_error_to_response(MasteringError::InvalidConfig {
                message: format!("Invalid preset: {}", e),
                config_key: Some("preset".to_string()),
            })
        })?;

    let job = MasteringJob {
        input_path: PathBuf::from(&request.input_path),
        output_path: request.output_path.as_ref().map(PathBuf::from),
        reference_path: request.reference_path.as_ref().map(PathBuf::from),
        backend,
        ai_provider,
        lmstudio_model: request.lmstudio_model.clone(),
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
pub async fn master_file(
    app: tauri::AppHandle,
    request: MasterRequest,
) -> Result<MasterResult, String> {
    let (job, mut config) = build_job(&request)?;
    hydrate_keyhan_advisor(&job, &mut config).await?;

    crate::telemetry::set_processing_context(
        &job.backend.to_string(),
        &job.preset
            .map(|preset| preset.to_string())
            .unwrap_or_default(),
        &job.input_path.display().to_string(),
    );

    // Validate input file exists
    if !job.input_path.exists() {
        return Err(mastering_error_to_response(MasteringError::FileIo {
            message: "Input file not found".to_string(),
            path: Some(job.input_path.clone()),
        }));
    }

    let job_id = request
        .job_id
        .clone()
        .unwrap_or_else(|| format!("desktop-{}", std::process::id()));
    let event_job_id = job_id.clone();
    let control = mastering_core::control::ProcessingControl::with_callback(move |progress| {
        let _ = app.emit(
            "mastering-progress",
            MasteringProgressEvent {
                job_id: event_job_id.clone(),
                progress,
            },
        );
    });
    active_jobs()
        .lock()
        .map_err(|_| "Mastering job registry is unavailable".to_string())?
        .insert(job_id.clone(), control.clone());

    let result =
        pipeline::run_with_control_and_policy(&job, &config, control, request.overwrite).await;
    if let Ok(mut jobs) = active_jobs().lock() {
        jobs.remove(&job_id);
    }
    let result = result.map_err(|error| {
        if error.to_string().contains("job was cancelled") {
            serde_json::to_string(&ErrorResponse {
                message: "Mastering cancelled safely".into(),
                code: "JOB_CANCELLED".into(),
                can_retry: true,
                can_fallback: false,
                suggested_action: None,
                details: None,
            })
            .unwrap_or_else(|_| "Mastering cancelled safely".into())
        } else {
            mastering_error_to_response(error.into())
        }
    })?;

    Ok(into_master_result(result))
}

#[tauri::command]
pub fn cancel_mastering(job_id: String) -> Result<bool, String> {
    let jobs = active_jobs()
        .lock()
        .map_err(|_| "Mastering job registry is unavailable".to_string())?;
    if let Some(control) = jobs.get(&job_id) {
        control.cancel();
        return Ok(true);
    }
    Ok(false)
}

#[tauri::command]
pub async fn create_mastering_preview(request: MasterRequest) -> Result<PreviewResult, String> {
    let (mut job, mut config) = build_job(&request)?;
    hydrate_keyhan_advisor(&job, &mut config).await?;
    pipeline::validate_input(&job.input_path).map_err(mastering_error_to_response)?;
    let preview_id = request
        .job_id
        .unwrap_or_else(|| format!("{}-{}", std::process::id(), now_millis()));
    let safe_id: String = preview_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .take(80)
        .collect();
    let preview_dir = std::env::temp_dir().join("audiomaster-previews");
    std::fs::create_dir_all(&preview_dir).map_err(|error| error.to_string())?;
    cleanup_old_previews(&preview_dir);
    let excerpt = preview_dir.join(format!("{safe_id}-source.wav"));
    let mastered = preview_dir.join(format!("{safe_id}-master.wav"));
    let matched = preview_dir.join(format!("{safe_id}-original-matched.wav"));

    let status = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error", "-t", "30"])
        .arg("-i")
        .arg(&job.input_path)
        .args(["-map", "0:a:0", "-c:a", "pcm_f32le"])
        .arg(&excerpt)
        .status()
        .await
        .map_err(|error| format!("Could not create preview excerpt: {error}"))?;
    if !status.success() {
        return Err("FFmpeg could not decode the preview excerpt".into());
    }

    job.input_path = excerpt.clone();
    job.output_path = Some(mastered.clone());
    job.format = Some(AudioFormat::Wav);
    job.bit_depth = Some(32);
    job.dry_run = false;
    let result = pipeline::run(&job, &config)
        .await
        .map_err(|error| mastering_error_to_response(error.into()))?;
    let pre = result
        .pre_analysis
        .as_ref()
        .ok_or_else(|| "Preview input analysis is unavailable".to_string())?;
    let post = result
        .post_analysis
        .as_ref()
        .ok_or_else(|| "Preview output analysis is unavailable".to_string())?;
    let gain_db = (post.lufs_integrated - pre.lufs_integrated).clamp(-30.0, 30.0);
    let volume_filter = format!("volume={gain_db:.4}dB");
    let status = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .arg("-i")
        .arg(&excerpt)
        .args(["-af", &volume_filter, "-c:a", "pcm_f32le"])
        .arg(&matched)
        .status()
        .await
        .map_err(|error| format!("Could not level-match preview: {error}"))?;
    let _ = std::fs::remove_file(&excerpt);
    if !status.success() {
        return Err("FFmpeg could not level-match the preview".into());
    }

    Ok(PreviewResult {
        original_path: matched.to_string_lossy().into_owned(),
        mastered_path: mastered.to_string_lossy().into_owned(),
        duration_seconds: post.metadata.duration_secs,
        level_match_gain_db: gain_db,
    })
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn cleanup_old_previews(directory: &std::path::Path) {
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(24 * 60 * 60))
        .unwrap_or(std::time::UNIX_EPOCH);
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .is_some_and(|modified| modified < cutoff)
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn into_master_result(result: MasteringResult) -> MasterResult {
    MasterResult {
        output_path: result.output_path.to_string_lossy().to_string(),
        backend_used: result.backend_used,
        pre_analysis: result.pre_analysis.map(|a| a.into()),
        post_analysis: result.post_analysis.map(|a| a.into()),
        params_applied: result.params_applied,
        warnings: result.warnings,
    }
}

#[tauri::command]
pub async fn master_album(request: AlbumRequest) -> Result<AlbumMasterResult, String> {
    let mut config = load_desktop_config()?;
    let backend = request
        .backend
        .as_deref()
        .unwrap_or("native")
        .parse::<Backend>()
        .map_err(|error| format!("Invalid album backend: {error}"))?;
    let ai_provider = request
        .ai_provider
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(|error| format!("Invalid AI provider: {error}"))?;
    let format = request
        .format
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(|error| format!("Invalid output format: {error}"))?;
    let preset = request
        .preset
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(|error| format!("Invalid preset: {error}"))?;
    if backend == Backend::Ai
        && ai_provider.unwrap_or(config.ai.default_provider) == AiProvider::KeyhanStudio
        && config.ai.keyhanstudio.api_key.is_empty()
    {
        config.ai.keyhanstudio.api_key = crate::cloud::cloud_access_token().await?;
    }

    let job = AlbumJob {
        input_paths: request.input_paths.into_iter().map(PathBuf::from).collect(),
        output_directory: PathBuf::from(request.output_directory),
        reference_path: request.reference_path.map(PathBuf::from),
        backend,
        ai_provider,
        bit_depth: request.bit_depth,
        format,
        target_lufs: request.target_lufs,
        no_limiter: request.no_limiter,
        preset,
        max_relative_offset_lu: request.max_relative_offset_lu.unwrap_or(1.5),
        track_offsets_lu: request.track_offsets_lu,
    };
    let result = album::run(&job, &config)
        .await
        .map_err(|error| mastering_error_to_response(error.into()))?;

    Ok(AlbumMasterResult {
        album_target_lufs: result.album_target_lufs,
        source_median_lufs: result.source_median_lufs,
        delivered_loudness_spread_lu: result.delivered_loudness_spread_lu,
        report_path: result.report_path.to_string_lossy().into_owned(),
        tracks: result
            .tracks
            .into_iter()
            .map(|track| AlbumTrackMasterResult {
                input_path: track.input_path.to_string_lossy().into_owned(),
                assigned_target_lufs: track.assigned_target_lufs,
                result: into_master_result(track.result),
                sha256: track.sha256,
            })
            .collect(),
    })
}

#[tauri::command]
pub fn export_diagnostic_bundle() -> Result<String, String> {
    crate::telemetry::export_diagnostic_bundle()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("Could not export diagnostics: {error}"))
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
                        result: Some(into_master_result(r)),
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
    let mut config = load_desktop_config()?;
    // Secrets are write-only in the desktop UI and are never sent to the webview.
    config.ai.keyhanstudio.api_key.clear();
    config.ai.openai.api_key.clear();
    config.ai.anthropic.api_key.clear();
    serde_json::to_value(&config).map_err(|e| format!("Serialize error: {e}"))
}

#[tauri::command]
pub fn save_config(config_json: serde_json::Value) -> Result<(), String> {
    let mut config: Config =
        serde_json::from_value(config_json).map_err(|e| format!("Invalid config: {e}"))?;
    crate::cloud::store_provider_secret("keyhanstudio", &config.ai.keyhanstudio.api_key)?;
    crate::cloud::store_provider_secret("openai", &config.ai.openai.api_key)?;
    crate::cloud::store_provider_secret("anthropic", &config.ai.anthropic.api_key)?;
    config.ai.keyhanstudio.api_key.clear();
    config.ai.openai.api_key.clear();
    config.ai.anthropic.api_key.clear();
    config.save().map_err(|e| format!("Save error: {e}"))
}

#[tauri::command]
pub fn clear_provider_credential(provider: String) -> Result<(), String> {
    crate::cloud::delete_provider_secret(&provider)
}

fn load_desktop_config() -> Result<Config, String> {
    let mut config = Config::load().map_err(|error| format!("Config error: {error}"))?;
    let mut migrated = false;
    for (provider, configured) in [
        ("keyhanstudio", &mut config.ai.keyhanstudio.api_key),
        ("openai", &mut config.ai.openai.api_key),
        ("anthropic", &mut config.ai.anthropic.api_key),
    ] {
        if let Some(secret) = crate::cloud::load_provider_secret(provider)? {
            *configured = secret;
        } else if !configured.is_empty() {
            crate::cloud::store_provider_secret(provider, configured)?;
            migrated = true;
        }
    }
    if migrated {
        let mut persisted = config.clone();
        persisted.ai.keyhanstudio.api_key.clear();
        persisted.ai.openai.api_key.clear();
        persisted.ai.anthropic.api_key.clear();
        persisted
            .save()
            .map_err(|error| format!("Could not remove migrated plaintext credentials: {error}"))?;
    }
    Ok(config)
}

async fn hydrate_keyhan_advisor(job: &MasteringJob, config: &mut Config) -> Result<(), String> {
    let provider = job.ai_provider.unwrap_or(config.ai.default_provider);
    if job.backend == Backend::Ai
        && provider == AiProvider::KeyhanStudio
        && config.ai.keyhanstudio.api_key.is_empty()
    {
        config.ai.keyhanstudio.api_key = crate::cloud::cloud_access_token().await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn check_backends() -> Result<Vec<BackendStatus>, String> {
    let config = load_desktop_config()?;

    let backends = vec![
        (Backend::Native, "Deterministic native mastering"),
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
    let config = load_desktop_config()?;
    let scripts_dir = Config::python_scripts_dir();
    let scripts_dir_str = scripts_dir.display().to_string();

    let backends = vec![
        (
            Backend::Native,
            "Deterministic native mastering",
            "built-in",
        ),
        (
            Backend::Matchering,
            "Reference-based mastering (Matchering)",
            &config.backends.matchering.python_path,
        ),
        (
            Backend::Ai,
            "AI-assisted mastering (LLM + DSP)",
            &config.backends.matchering.python_path,
        ),
        (
            Backend::LocalMl,
            "Local ML models (DeepAFx-ST)",
            &config.backends.local_ml.python_path,
        ),
    ];

    let mut results = Vec::new();
    for (backend, description, python_path) in backends {
        let engine = MasteringEngine::from_config(backend, &config);
        let (available, error) = match engine.check_available().await {
            Ok(true) => (true, None),
            Ok(false) => (
                false,
                Some(
                    "Backend check returned false. Python dependencies may be missing.".to_string(),
                ),
            ),
            Err(e) => (false, Some(format!("{e}"))),
        };
        results.push(BackendDiagnostic {
            name: backend.to_string(),
            available,
            description: description.to_string(),
            error,
            python_path: python_path.to_string(),
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

// ---------------------------------------------------------------------------
// LM Studio commands
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct LmStudioStatus {
    pub running: bool,
    pub endpoint: String,
}

#[tauri::command]
pub async fn lmstudio_status(endpoint: Option<String>) -> Result<LmStudioStatus, String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://localhost:1234/v1".to_string());
    let running = mastering_core::backends::ai::AiBackend::lmstudio_status(&endpoint)
        .await
        .unwrap_or(false);
    Ok(LmStudioStatus { running, endpoint })
}

#[derive(Serialize)]
pub struct LmStudioModelInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loaded: Option<bool>,
}

#[tauri::command]
pub async fn lmstudio_models(endpoint: Option<String>) -> Result<Vec<LmStudioModelInfo>, String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://localhost:1234/v1".to_string());
    let models = mastering_core::backends::ai::AiBackend::lmstudio_models(&endpoint)
        .await
        .map_err(|e| format!("Failed to list LM Studio models: {e}"))?;
    Ok(models
        .into_iter()
        .map(|m| LmStudioModelInfo {
            id: m.id,
            display_name: m.display_name,
            size_gb: m.size_gb,
            quant: m.quant,
            architecture: m.architecture,
            loaded: m.loaded,
        })
        .collect())
}

#[tauri::command]
pub async fn lmstudio_load_model(endpoint: Option<String>, model_id: String) -> Result<(), String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://localhost:1234/v1".to_string());
    mastering_core::backends::ai::AiBackend::lmstudio_load_model(&endpoint, &model_id)
        .await
        .map_err(|e| format!("Failed to load model: {e}"))
}

#[tauri::command]
pub async fn lmstudio_unload_model(
    endpoint: Option<String>,
    model_id: String,
) -> Result<(), String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://localhost:1234/v1".to_string());
    mastering_core::backends::ai::AiBackend::lmstudio_unload_model(&endpoint, &model_id)
        .await
        .map_err(|e| format!("Failed to unload model: {e}"))
}

#[tauri::command]
pub async fn lmstudio_loaded_models(
    endpoint: Option<String>,
) -> Result<Vec<LmStudioModelInfo>, String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://localhost:1234/v1".to_string());
    let models = mastering_core::backends::ai::AiBackend::lmstudio_loaded_models(&endpoint)
        .await
        .map_err(|e| format!("Failed to get loaded models: {e}"))?;
    Ok(models
        .into_iter()
        .map(|m| LmStudioModelInfo {
            id: m.id,
            display_name: m.display_name,
            size_gb: m.size_gb,
            quant: m.quant,
            architecture: m.architecture,
            loaded: Some(true),
        })
        .collect())
}

#[derive(Serialize)]
pub struct VramInfo {
    pub gpus: Vec<mastering_core::gpu::GpuInfo>,
    pub detected_vram_mb: Option<u64>,
    pub tier: Option<String>,
    pub recommendations: Vec<mastering_core::gpu::ModelRecommendation>,
}

#[tauri::command]
pub fn detect_vram() -> Result<VramInfo, String> {
    let gpus =
        mastering_core::gpu::detect_vram().map_err(|e| format!("GPU detection failed: {e}"))?;

    let detected_vram_mb = gpus.iter().map(|g| g.vram_total_mb).max();
    let tier = detected_vram_mb.map(|v| {
        let tiers = mastering_core::gpu::get_vram_tiers();
        tiers
            .iter()
            .rfind(|t| v >= t.vram_mb)
            .map(|t| t.tier_name.clone())
            .unwrap_or_else(|| "<4GB".to_string())
    });

    let recommendations = if let Some(vram) = detected_vram_mb {
        mastering_core::gpu::get_recommendations_for_vram(vram)
    } else {
        vec![]
    };

    Ok(VramInfo {
        gpus,
        detected_vram_mb,
        tier,
        recommendations,
    })
}

#[derive(Serialize)]
pub struct LmStudioRecommendation {
    pub recommended: Vec<mastering_core::gpu::ModelRecommendation>,
    pub vram_mb: Option<u64>,
    pub tier: Option<String>,
    pub available_models: Vec<String>,
}

/// Cross-reference VRAM recommendations with models available in LM Studio.
#[tauri::command]
pub async fn lmstudio_recommend_models(
    endpoint: Option<String>,
) -> Result<LmStudioRecommendation, String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://localhost:1234/v1".to_string());

    // Detect GPU VRAM
    let gpus =
        mastering_core::gpu::detect_vram().map_err(|e| format!("GPU detection failed: {e}"))?;
    let vram_mb = gpus.iter().map(|g| g.vram_total_mb).max();

    let tier = vram_mb.map(|v| {
        let tiers = mastering_core::gpu::get_vram_tiers();
        tiers
            .iter()
            .rfind(|t| v >= t.vram_mb)
            .map(|t| t.tier_name.clone())
            .unwrap_or_else(|| "<4GB".to_string())
    });

    // Get models from LM Studio
    let models = mastering_core::backends::ai::AiBackend::lmstudio_models(&endpoint)
        .await
        .map_err(|e| format!("Failed to list LM Studio models: {e}"))?;
    let available_ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();

    // Cross-reference
    let recommended = if let Some(vram) = vram_mb {
        mastering_core::gpu::recommend_from_available(&available_ids, vram)
    } else {
        vec![]
    };

    Ok(LmStudioRecommendation {
        recommended,
        vram_mb,
        tier,
        available_models: available_ids,
    })
}
