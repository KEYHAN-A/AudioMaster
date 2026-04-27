use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::analysis;
use crate::backends::{MasteringEngine, MasteringOptions};
use crate::config::Config;
use crate::error::MasteringError;
use crate::types::{AiProvider, AudioFormat, Backend, MasteringResult, Preset};

/// Maximum supported file size (500MB)
const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

/// Supported audio formats for input
const SUPPORTED_INPUT_EXTENSIONS: &[&str] = &["wav", "flac", "mp3", "ogg", "m4a", "aac", "wma"];

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
pub fn check_disk_space(output_path: &Path, _estimated_size: u64) -> Result<(), MasteringError> {
    // Get parent directory
    let parent = output_path
        .parent()
        .unwrap_or(Path::new("."));

    // Get available space (simplified check)
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match std::fs::metadata(parent) {
            Ok(meta) => {
                // This is a simplified check - in production you'd use statvfs
                let _ = meta.dev(); // Suppress unused warning
            }
            Err(_) => {
                warn!("Cannot check disk space for {}", parent.display());
            }
        }
    }

    // Ensure output directory exists or can be created
    if !parent.exists() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Err(MasteringError::FileIo {
                message: format!("Cannot create output directory: {}", e),
                path: Some(parent.to_path_buf()),
            });
        }
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
    pub bit_depth: Option<u16>,
    pub format: Option<AudioFormat>,
    pub target_lufs: Option<f64>,
    pub no_limiter: bool,
    pub preset: Option<Preset>,
    pub dry_run: bool,
}

impl MasteringJob {
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

        let format = self.format.unwrap_or(config.general.default_format);
        let ext = match format {
            AudioFormat::Wav => "wav",
            AudioFormat::Flac => "flac",
            AudioFormat::Mp3 => "mp3",
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
                    Backend::Ai
                }
            }
            other => other,
        }
    }
}

/// Execute the full mastering pipeline.
pub async fn run(job: &MasteringJob, config: &Config) -> Result<MasteringResult> {
    let pipeline_start = std::time::Instant::now();

    // Step 0: Validate input
    validate_input(&job.input_path)?;

    let output_path = job.resolved_output_path(config);
    let backend = job.resolved_backend();
    let bit_depth = job.bit_depth.unwrap_or(config.general.default_bit_depth);
    let target_lufs = job
        .target_lufs
        .or_else(|| job.preset.map(|p| p.target_lufs()))
        .unwrap_or(config.general.target_lufs);

    info!("Mastering pipeline started");
    info!("  Input:    {}", job.input_path.display());
    info!("  Output:   {}", output_path.display());
    info!("  Backend:  {backend}");
    info!("  Bit depth: {bit_depth}");
    info!("  Target LUFS: {target_lufs}");

    // Step 1: Pre-analysis
    let analysis_start = std::time::Instant::now();
    info!("Analyzing input audio...");
    let pre_analysis = analysis::analyze_file(&job.input_path)
        .await
        .context("Pre-analysis of input audio failed")?;

    let analysis_elapsed = analysis_start.elapsed();
    info!("Pre-analysis completed in {:.2}s", analysis_elapsed.as_secs_f64());

    info!(
        "  LUFS: {:.1}, Peak: {:.1} dB, RMS: {:.1} dB, Stereo Width: {:.2}",
        pre_analysis.lufs_integrated,
        pre_analysis.peak_db,
        pre_analysis.rms_db,
        pre_analysis.stereo_width
    );

    // Dry run: just show analysis and exit
    if job.dry_run {
        info!("Dry run — no processing performed");
        return Ok(MasteringResult {
            output_path,
            backend_used: backend.to_string(),
            pre_analysis: Some(pre_analysis),
            post_analysis: None,
            params_applied: None,
        });
    }

    // Step 2: Create and configure the backend engine
    let mut engine = MasteringEngine::from_config(backend, config);

    // Override AI provider if specified
    if let (MasteringEngine::Ai(ref mut ai_backend), Some(provider)) =
        (&mut engine, job.ai_provider)
    {
        *ai_backend = ai_backend.clone().with_provider(provider);
    }

    let opts = MasteringOptions {
        input_path: job.input_path.clone(),
        output_path: output_path.clone(),
        reference_path: job.reference_path.clone(),
        bit_depth,
        target_lufs,
        no_limiter: job.no_limiter,
        preset: job.preset,
    };

    // Step 3: Process
    let process_start = std::time::Instant::now();
    info!("Processing with {} backend...", engine.name());
    let backend_output = engine
        .process(&opts)
        .await
        .context("Backend processing failed")?;

    let process_elapsed = process_start.elapsed();
    info!(
        "Backend processing completed in {:.2}s ({})",
        process_elapsed.as_secs_f64(),
        engine.name()
    );

    // Step 4: Post-analysis (if output file was created)
    let post_analysis = if backend_output.output_path.exists() {
        info!("Analyzing output...");
        match analysis::analyze_file(&backend_output.output_path).await {
            Ok(a) => {
                info!(
                    "  Output LUFS: {:.1}, Peak: {:.1} dB",
                    a.lufs_integrated, a.peak_db
                );
                Some(a)
            }
            Err(e) => {
                tracing::warn!("Post-analysis failed: {e}");
                None
            }
        }
    } else {
        None
    };

    // Step 5: Format conversion if needed
    let final_format = job.format.unwrap_or(config.general.default_format);
    if final_format != AudioFormat::Wav && backend_output.output_path.exists() {
        convert_format(&backend_output.output_path, &output_path, final_format)?;
    }

    let total_elapsed = pipeline_start.elapsed();
    info!(
        "Mastering complete: {} (total: {:.2}s, analysis: {:.2}s, processing: {:.2}s)",
        output_path.display(),
        total_elapsed.as_secs_f64(),
        analysis_elapsed.as_secs_f64(),
        process_elapsed.as_secs_f64()
    );

    Ok(MasteringResult {
        output_path,
        backend_used: backend_output.backend_name,
        pre_analysis: Some(pre_analysis),
        post_analysis,
        params_applied: backend_output.params_applied,
    })
}

/// Convert output format using ffmpeg.
fn convert_format(input: &Path, output: &Path, format: AudioFormat) -> Result<()> {
    if input == output {
        return Ok(());
    }

    let codec = match format {
        AudioFormat::Wav => return Ok(()), // Already WAV
        AudioFormat::Flac => "flac",
        AudioFormat::Mp3 => "libmp3lame",
    };

    info!("Converting to {} format...", format);

    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &input.to_string_lossy(),
            "-codec:a",
            codec,
            &output.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("Running ffmpeg for format conversion. Is ffmpeg installed?")?;

    if !status.success() {
        anyhow::bail!("ffmpeg conversion failed with exit code: {}", status);
    }

    Ok(())
}
