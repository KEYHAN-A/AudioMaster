use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::analysis;
use crate::backends::{MasteringEngine, MasteringOptions};
use crate::config::Config;
use crate::types::{AiProvider, AudioFormat, Backend, MasteringResult, Preset};

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
    info!("Analyzing input audio...");
    let pre_analysis = analysis::analyze_file(&job.input_path)
        .await
        .context("Pre-analysis of input audio failed")?;

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
    info!("Processing with {} backend...", engine.name());
    let backend_output = engine
        .process(&opts)
        .await
        .context("Backend processing failed")?;

    info!("{}", backend_output.message);

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

    info!("Mastering complete: {}", output_path.display());

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
