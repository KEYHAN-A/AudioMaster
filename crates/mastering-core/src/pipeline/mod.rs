use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::analysis;
use crate::backends::{MasteringEngine, MasteringOptions};
use crate::config::Config;
use crate::error::MasteringError;
use crate::types::{
    AiProvider, AudioAnalysis, AudioFormat, Backend, MasteringResult, MasteringWarning, Preset,
};

/// Safety ceiling for pathological inputs. Long-form jobs above the in-memory
/// threshold use the bounded streaming graph.
const MAX_FILE_SIZE: u64 = 32 * 1024 * 1024 * 1024;

/// Supported audio formats for input
const SUPPORTED_INPUT_EXTENSIONS: &[&str] = &[
    "wav", "aif", "aiff", "flac", "mp3", "ogg", "m4a", "aac", "wma",
];

/// Validate input file before processing.
pub fn validate_input(path: &Path) -> Result<(), MasteringError> {
    // Check file exists
    if !path.exists() {
        return Err(MasteringError::FileIo {
            message: "Input file does not exist".to_string(),
            path: Some(path.to_path_buf()),
        });
    }

    // Check it's a file (not a directory)
    if !path.is_file() {
        return Err(MasteringError::ValidationError {
            message: "Path is not a file".to_string(),
            field: Some("input_path".to_string()),
        });
    }

    // Check file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !SUPPORTED_INPUT_EXTENSIONS.contains(&ext.as_str()) {
        return Err(MasteringError::ValidationError {
            message: format!(
                "Unsupported file format: {}. Supported: {}",
                ext,
                SUPPORTED_INPUT_EXTENSIONS.join(", ")
            ),
            field: Some("input_path".to_string()),
        });
    }

    // Check file size
    let metadata = path.metadata().map_err(|e| MasteringError::FileIo {
        message: format!("Cannot read file metadata: {}", e),
        path: Some(path.to_path_buf()),
    })?;

    let file_size = metadata.len();
    if file_size == 0 {
        return Err(MasteringError::ValidationError {
            message: "Input file is empty".to_string(),
            field: Some("input_path".to_string()),
        });
    }

    if file_size > MAX_FILE_SIZE {
        return Err(MasteringError::ValidationError {
            message: format!(
                "Input file too large ({} MB). Maximum size is {} MB.",
                file_size / (1024 * 1024),
                MAX_FILE_SIZE / (1024 * 1024)
            ),
            field: Some("input_path".to_string()),
        });
    }

    Ok(())
}

/// Check disk space for output file.
pub fn check_disk_space(output_path: &Path, estimated_size: u64) -> Result<(), MasteringError> {
    let parent = output_path.parent().unwrap_or(Path::new("."));
    if !parent.exists() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Err(MasteringError::FileIo {
                message: format!("Cannot create output directory: {}", e),
                path: Some(parent.to_path_buf()),
            });
        }
    }

    match fs2::available_space(parent) {
        Ok(available) if available < estimated_size => {
            return Err(MasteringError::FileIo {
                message: format!(
                    "Insufficient disk space: need approximately {} MB, only {} MB available",
                    estimated_size / (1024 * 1024),
                    available / (1024 * 1024)
                ),
                path: Some(parent.to_path_buf()),
            });
        }
        Ok(_) => {}
        Err(error) => warn!("Cannot check disk space for {}: {error}", parent.display()),
    }

    Ok(())
}

/// Trait for pre-flight checks that backends can implement.
pub trait PreflightCheck {
    /// Check if the backend is available and properly configured.
    fn check_available(&self) -> Result<(), MasteringError>;

    /// Validate configuration for this backend.
    fn validate_config(&self, config: &Config) -> Result<(), MasteringError>;
}

/// High-level mastering job request.
#[derive(Debug, Clone)]
pub struct MasteringJob {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub reference_path: Option<PathBuf>,
    pub backend: Backend,
    pub ai_provider: Option<AiProvider>,
    pub lmstudio_model: Option<String>,
    pub bit_depth: Option<u16>,
    pub format: Option<AudioFormat>,
    pub target_lufs: Option<f64>,
    pub no_limiter: bool,
    pub preset: Option<Preset>,
    pub dry_run: bool,
}

impl MasteringJob {
    pub fn resolved_format(&self, config: &Config) -> AudioFormat {
        if let Some(format) = self.format {
            return format;
        }
        if let Some(extension) = self
            .output_path
            .as_ref()
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
            .and_then(|extension| extension.parse().ok())
        {
            return extension;
        }
        config.general.default_format
    }

    /// Resolve the output path from input path if not explicitly set.
    pub fn resolved_output_path(&self, config: &Config) -> PathBuf {
        if let Some(ref out) = self.output_path {
            return out.clone();
        }

        let stem = self
            .input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let format = self.resolved_format(config);
        let ext = match format {
            AudioFormat::Wav => "wav",
            AudioFormat::Aiff => "aiff",
            AudioFormat::Flac => "flac",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Aac => "m4a",
        };

        let parent = self.input_path.parent().unwrap_or(Path::new("."));
        parent.join(format!("{stem}_mastered.{ext}"))
    }

    /// Resolve which backend to actually use.
    pub fn resolved_backend(&self) -> Backend {
        match self.backend {
            Backend::Auto => {
                if self.reference_path.is_some() {
                    Backend::Matchering
                } else {
                    Backend::Native
                }
            }
            other => other,
        }
    }
}

/// Execute the full mastering pipeline.
pub async fn run(job: &MasteringJob, config: &Config) -> Result<MasteringResult> {
    run_with_control(job, config, crate::control::ProcessingControl::default()).await
}

/// Execute a mastering pipeline with cooperative cancellation and progress.
pub async fn run_with_control(
    job: &MasteringJob,
    config: &Config,
    control: crate::control::ProcessingControl,
) -> Result<MasteringResult> {
    run_with_control_and_policy(job, config, control, false).await
}

pub async fn run_with_control_and_policy(
    job: &MasteringJob,
    config: &Config,
    control: crate::control::ProcessingControl,
    allow_overwrite: bool,
) -> Result<MasteringResult> {
    let pipeline_start = std::time::Instant::now();

    control.check_cancelled()?;
    control.report("validation", 0.0, 0, None, "Validating source");

    // Step 0: Validate input
    validate_input(&job.input_path)?;
    if let Some(reference) = &job.reference_path {
        validate_input(reference)
            .map_err(anyhow::Error::from)
            .context("Invalid reference track")?;
    }

    let bit_depth = job.bit_depth.unwrap_or(config.general.default_bit_depth);
    anyhow::ensure!(
        matches!(bit_depth, 16 | 24 | 32),
        "Output bit depth must be 16, 24, or 32"
    );
    let target_lufs = job
        .target_lufs
        .or_else(|| job.preset.map(|p| p.target_lufs()))
        .unwrap_or(config.general.target_lufs);
    anyhow::ensure!(
        target_lufs.is_finite() && (-24.0..=-5.0).contains(&target_lufs),
        "Target loudness must be between -24 and -5 LUFS"
    );

    let output_path = job.resolved_output_path(config);
    anyhow::ensure!(
        output_path != job.input_path,
        "Output path must be different from the input path"
    );
    anyhow::ensure!(
        allow_overwrite || !output_path.exists(),
        "Output already exists; explicit overwrite confirmation is required: {}",
        output_path.display()
    );
    let estimated_size = job
        .input_path
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0)
        * 4;
    check_disk_space(&output_path, estimated_size)?;
    let final_format = job.resolved_format(config);
    let output_extension = output_path
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| extension.parse::<AudioFormat>().ok());
    anyhow::ensure!(
        output_extension == Some(final_format),
        "Output filename extension must match the requested {} format",
        final_format
    );
    let intermediate_path = temporary_output_path(&output_path, "wav");
    let intermediate_guard = TemporaryArtifact::new(intermediate_path.clone());
    let backend = job.resolved_backend();

    info!("Mastering pipeline started");
    info!(
        "  Input format: {}",
        job.input_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
    );
    info!("  Output format: {final_format}");
    info!("  Backend:  {backend}");
    info!("  Bit depth: {bit_depth}");
    info!("  Target LUFS: {target_lufs}");

    // Step 1: Pre-analysis
    let analysis_start = std::time::Instant::now();
    info!("Analyzing input audio...");
    control.report("analysis", 0.05, 0, None, "Analyzing source");
    let pre_analysis = analysis::analyze_file(&job.input_path)
        .await
        .context("Pre-analysis of input audio failed")?;

    let analysis_elapsed = analysis_start.elapsed();
    info!(
        "Pre-analysis completed in {:.2}s",
        analysis_elapsed.as_secs_f64()
    );

    info!(
        "  LUFS: {:.1}, Peak: {:.1} dB, RMS: {:.1} dB, Stereo Width: {:.2}",
        pre_analysis.lufs_integrated,
        pre_analysis.peak_db,
        pre_analysis.rms_db,
        pre_analysis.stereo_width
    );
    let reference_analysis = if let Some(reference_path) = &job.reference_path {
        control.check_cancelled()?;
        control.report("analysis", 0.15, 0, None, "Analyzing reference");
        Some(
            analysis::analyze_file(reference_path)
                .await
                .context("Analysis of reference audio failed")?,
        )
    } else {
        None
    };

    // Dry run: just show analysis and exit
    if job.dry_run {
        info!("Dry run — no processing performed");
        return Ok(MasteringResult {
            output_path,
            backend_used: backend.to_string(),
            pre_analysis: Some(pre_analysis),
            post_analysis: None,
            params_applied: None,
            warnings: Vec::new(),
        });
    }

    // Step 2: Create and configure the backend engine
    let mut config = config.clone();
    if let Some(ref model) = job.lmstudio_model {
        config.ai.lmstudio.model = model.clone();
    }
    let opts = MasteringOptions {
        input_path: job.input_path.clone(),
        output_path: intermediate_path.clone(),
        reference_path: job.reference_path.clone(),
        bit_depth,
        delivery_format: final_format,
        target_lufs,
        no_limiter: job.no_limiter,
        preset: job.preset,
        pre_analysis: Some(pre_analysis.clone()),
        reference_analysis,
        control: control.clone(),
    };

    // Step 3: Process
    control.check_cancelled()?;
    control.report("render", 0.25, 0, None, "Rendering master");
    let process_start = std::time::Instant::now();
    let backend_output = if job.backend == Backend::Auto {
        process_auto(job, &config, &opts).await?
    } else {
        let engine = configured_engine(backend, &config, job.ai_provider);
        info!("Processing with {} backend...", engine.name());
        engine
            .process(&opts)
            .await
            .context("Backend processing failed")?
    };

    let process_elapsed = process_start.elapsed();
    info!(
        "Backend processing completed in {:.2}s ({})",
        process_elapsed.as_secs_f64(),
        backend_output.backend_name
    );

    anyhow::ensure!(
        backend_output.output_path.exists(),
        "Backend completed without creating an output file"
    );

    // Step 4: Convert from the lossless intermediate. The candidate remains
    // hidden until it passes delivery verification.
    control.check_cancelled()?;
    control.report(
        "encode",
        0.82,
        0,
        None,
        "Encoding delivery and preserving metadata",
    );
    let converted_path = temporary_output_path(&output_path, &final_format.to_string());
    let converted_guard = TemporaryArtifact::new(converted_path.clone());
    convert_format(
        &backend_output.output_path,
        &job.input_path,
        &converted_path,
        final_format,
        bit_depth,
    )
    .await?;
    let delivery_candidate = converted_path;

    // Step 5: Verify the exact candidate that will be delivered, including
    // lossy codec overshoot. Verification failure never replaces a good file.
    info!("Analyzing output...");
    control.check_cancelled()?;
    control.report("verification", 0.9, 0, None, "Verifying delivered audio");
    let mut post_analysis = analysis::analyze_file(&delivery_candidate)
        .await
        .context("Verification analysis of mastered output failed")?;
    let mut warnings: Vec<MasteringWarning> = backend_output
        .warnings
        .iter()
        .map(|message| MasteringWarning {
            code: "ENGINE_ADJUSTMENT".into(),
            message: message.clone(),
        })
        .collect();
    verify_delivery(
        &post_analysis,
        target_lufs,
        backend_output.params_applied.as_ref(),
        &mut warnings,
    )?;
    info!(
        "  Output LUFS: {:.1}, True Peak: {:.1} dBTP",
        post_analysis.lufs_integrated, post_analysis.true_peak_db
    );

    publish_output(&delivery_candidate, &output_path, allow_overwrite)?;
    control.report("complete", 1.0, 0, None, "Mastering complete");
    post_analysis.metadata.path = output_path.clone();
    converted_guard.disarm();
    intermediate_guard.disarm();

    let total_elapsed = pipeline_start.elapsed();
    info!(
        "Mastering complete (total: {:.2}s, analysis: {:.2}s, processing: {:.2}s)",
        total_elapsed.as_secs_f64(),
        analysis_elapsed.as_secs_f64(),
        process_elapsed.as_secs_f64()
    );

    Ok(MasteringResult {
        output_path,
        backend_used: backend_output.backend_name,
        pre_analysis: Some(pre_analysis),
        post_analysis: Some(post_analysis),
        params_applied: backend_output.params_applied,
        warnings,
    })
}

fn verify_delivery(
    analysis: &AudioAnalysis,
    target_lufs: f64,
    params: Option<&crate::types::MasteringParams>,
    warnings: &mut Vec<MasteringWarning>,
) -> Result<()> {
    anyhow::ensure!(
        analysis.lufs_integrated.is_finite() && analysis.true_peak_db.is_finite(),
        "Delivered-file verification produced non-finite loudness measurements"
    );

    if (analysis.lufs_integrated - target_lufs).abs() > 1.0 {
        warnings.push(MasteringWarning {
            code: "LOUDNESS_TARGET_MISS".into(),
            message: format!(
                "Delivered loudness is {:.1} LUFS; requested target was {:.1} LUFS",
                analysis.lufs_integrated, target_lufs
            ),
        });
    }

    if let Some(params) = params {
        if params.limiter.enabled {
            anyhow::ensure!(
                analysis.true_peak_db <= params.limiter.ceiling_db + 0.15,
                "Delivered true peak ({:.2} dBTP) exceeds limiter ceiling ({:.2} dBTP)",
                analysis.true_peak_db,
                params.limiter.ceiling_db
            );
            anyhow::ensure!(
                analysis.clipped_samples == 0,
                "Delivered file contains {} clipped samples",
                analysis.clipped_samples
            );
        } else if analysis.clipped_samples > 0 {
            warnings.push(MasteringWarning {
                code: "CLIPPED_SAMPLES".into(),
                message: format!(
                    "Delivered file contains {} clipped samples because limiting is disabled",
                    analysis.clipped_samples
                ),
            });
        }
    }

    if analysis.dc_offset > 0.005 {
        warnings.push(MasteringWarning {
            code: "DC_OFFSET".into(),
            message: format!(
                "Delivered audio has {:.4} maximum DC offset",
                analysis.dc_offset
            ),
        });
    }
    if analysis.metadata.channels == 2 && analysis.stereo_correlation < -0.1 {
        warnings.push(MasteringWarning {
            code: "PHASE_RISK".into(),
            message: format!(
                "Delivered stereo correlation is {:.2}; check mono compatibility",
                analysis.stereo_correlation
            ),
        });
    }

    Ok(())
}

fn configured_engine(
    backend: Backend,
    config: &Config,
    provider: Option<AiProvider>,
) -> MasteringEngine {
    let mut engine = MasteringEngine::from_config(backend, config);
    if let (MasteringEngine::Ai(ai_backend), Some(provider)) = (&mut engine, provider) {
        *ai_backend = ai_backend.clone().with_provider(provider);
    }
    engine
}

async fn process_auto(
    job: &MasteringJob,
    config: &Config,
    options: &MasteringOptions,
) -> Result<crate::backends::BackendOutput> {
    let mut candidates = Vec::new();
    if job.reference_path.is_some() && !job.no_limiter {
        candidates.push(Backend::Matchering);
    }
    candidates.push(Backend::Native);

    let mut failures = Vec::new();
    for backend in candidates {
        let engine = configured_engine(backend, config, job.ai_provider);
        match engine.check_available().await {
            Ok(true) => {}
            Ok(false) => {
                failures.push(format!("{} is unavailable", engine.name()));
                continue;
            }
            Err(error) => {
                failures.push(format!("{} preflight failed: {error}", engine.name()));
                continue;
            }
        }

        info!("Auto mode trying {} backend", engine.name());
        match engine.process(options).await {
            Ok(output) => return Ok(output),
            Err(error) => {
                warn!("Auto backend {} failed: {error}", engine.name());
                failures.push(format!("{}: {error}", engine.name()));
                let _ = std::fs::remove_file(&options.output_path);
            }
        }
    }

    anyhow::bail!(
        "No mastering backend completed successfully. {}",
        failures.join("; ")
    )
}

/// Convert output format using ffmpeg.
async fn convert_format(
    input: &Path,
    metadata_source: &Path,
    output: &Path,
    format: AudioFormat,
    bit_depth: u16,
) -> Result<()> {
    let (codec, extra_args): (&str, Vec<&str>) = match format {
        AudioFormat::Wav => {
            let codec = match bit_depth {
                16 => "pcm_s16le",
                24 => "pcm_s24le",
                _ => "pcm_f32le",
            };
            (codec, Vec::new())
        }
        AudioFormat::Aiff => {
            let codec = match bit_depth {
                16 => "pcm_s16be",
                24 => "pcm_s24be",
                _ => "pcm_f32be",
            };
            (codec, Vec::new())
        }
        AudioFormat::Flac => {
            let sample_format = if bit_depth <= 16 { "s16" } else { "s32" };
            ("flac", vec!["-sample_fmt", sample_format])
        }
        AudioFormat::Mp3 => ("libmp3lame", vec!["-b:a", "320k"]),
        AudioFormat::Aac => ("aac", vec!["-b:a", "320k"]),
    };

    info!("Converting to {} format...", format);

    let status = tokio::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input)
        .arg("-i")
        .arg(metadata_source)
        .arg("-map")
        .arg("0:a:0")
        .arg("-map_metadata")
        .arg("1")
        .arg("-codec:a")
        .arg(codec)
        .args(extra_args)
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .context("Running ffmpeg for format conversion. Is ffmpeg installed?")?;

    if !status.success() {
        anyhow::bail!("ffmpeg conversion failed with exit code: {}", status);
    }

    Ok(())
}

fn temporary_output_path(final_path: &Path, extension: &str) -> PathBuf {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = final_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("master");
    final_path.with_file_name(format!(
        ".{name}.audiomaster-{}-{id}.{extension}",
        std::process::id()
    ))
}

fn publish_output(temporary_path: &Path, final_path: &Path, allow_overwrite: bool) -> Result<()> {
    if final_path.exists() {
        anyhow::ensure!(allow_overwrite, "Output overwrite was not authorized");
        std::fs::remove_file(final_path)
            .with_context(|| format!("Replacing existing output: {}", final_path.display()))?;
    }
    std::fs::rename(temporary_path, final_path).with_context(|| {
        format!(
            "Publishing mastered output from {} to {}",
            temporary_path.display(),
            final_path.display()
        )
    })?;
    Ok(())
}

struct TemporaryArtifact {
    path: PathBuf,
    armed: std::sync::atomic::AtomicBool,
}

impl TemporaryArtifact {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            armed: std::sync::atomic::AtomicBool::new(true),
        }
    }

    fn disarm(&self) {
        self.armed
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Drop for TemporaryArtifact {
    fn drop(&mut self) {
        if self.armed.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
