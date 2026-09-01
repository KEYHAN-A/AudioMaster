//! Album-oriented mastering orchestration.
//!
//! Album mode preserves bounded source loudness relationships instead of
//! forcing every song to the exact same integrated loudness. Each track still
//! passes through the normal rendering and delivered-file verification path.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::analysis;
use crate::config::Config;
use crate::pipeline::{self, MasteringJob};
use crate::types::{AiProvider, AudioAnalysis, AudioFormat, Backend, MasteringResult, Preset};

#[derive(Debug, Clone)]
pub struct AlbumJob {
    pub input_paths: Vec<PathBuf>,
    pub output_directory: PathBuf,
    pub reference_path: Option<PathBuf>,
    pub backend: Backend,
    pub ai_provider: Option<AiProvider>,
    pub bit_depth: Option<u16>,
    pub format: Option<AudioFormat>,
    pub target_lufs: Option<f64>,
    pub no_limiter: bool,
    pub preset: Option<Preset>,
    /// Maximum source loudness offset retained on either side of the median.
    pub max_relative_offset_lu: f64,
    /// Engineer-authored per-track adjustment applied after continuity targets.
    pub track_offsets_lu: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct AlbumTrackResult {
    pub input_path: PathBuf,
    pub assigned_target_lufs: f64,
    pub result: MasteringResult,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct AlbumResult {
    pub album_target_lufs: f64,
    pub source_median_lufs: f64,
    pub tracks: Vec<AlbumTrackResult>,
    pub delivered_loudness_spread_lu: f64,
    pub report_path: PathBuf,
}

pub async fn run(job: &AlbumJob, config: &Config) -> Result<AlbumResult> {
    anyhow::ensure!(
        job.input_paths.len() >= 2,
        "Album mode requires at least two tracks"
    );
    anyhow::ensure!(
        job.max_relative_offset_lu.is_finite() && (0.0..=3.0).contains(&job.max_relative_offset_lu),
        "Album relative loudness offset must be between 0 and 3 LU"
    );

    std::fs::create_dir_all(&job.output_directory).with_context(|| {
        format!(
            "Creating album output directory {}",
            job.output_directory.display()
        )
    })?;

    let mut analyses = Vec::with_capacity(job.input_paths.len());
    for input in &job.input_paths {
        pipeline::validate_input(input)?;
        analyses.push(analysis::analyze_file(input).await?);
    }

    let source_median_lufs = median_loudness(&analyses);
    let album_target_lufs = job
        .target_lufs
        .or_else(|| job.preset.map(|preset| preset.target_lufs()))
        .unwrap_or(config.general.target_lufs);
    let assigned_targets = derive_track_targets(
        &analyses,
        source_median_lufs,
        album_target_lufs,
        job.max_relative_offset_lu,
    )
    .into_iter()
    .enumerate()
    .map(|(index, target)| {
        let adjustment = job.track_offsets_lu.get(index).copied().unwrap_or(0.0);
        anyhow::ensure!(
            adjustment.is_finite() && (-3.0..=3.0).contains(&adjustment),
            "Album track {} adjustment must be between -3 and 3 LU",
            index + 1
        );
        Ok((target + adjustment).clamp(-24.0, -5.0))
    })
    .collect::<Result<Vec<_>>>()?;

    let format = job.format.unwrap_or(config.general.default_format);
    let mut output_names = HashSet::new();
    let mut destinations = Vec::with_capacity(job.input_paths.len());
    for input_path in &job.input_paths {
        let output_path = output_path_for(input_path, &job.output_directory, format);
        let collision_key = output_path.to_string_lossy().to_lowercase();
        anyhow::ensure!(
            output_names.insert(collision_key),
            "Multiple album tracks resolve to the same output name: {}",
            output_path.display()
        );
        anyhow::ensure!(
            !output_path.exists(),
            "Album output already exists: {}. Choose a new directory or move the existing master.",
            output_path.display()
        );
        destinations.push(output_path);
    }
    let report_path = job.output_directory.join("audiomaster-album-report.json");
    anyhow::ensure!(
        !report_path.exists(),
        "Album verification report already exists: {}",
        report_path.display()
    );

    let staging_directory = album_staging_directory(&job.output_directory);
    std::fs::create_dir(&staging_directory).with_context(|| {
        format!(
            "Creating album staging directory {}",
            staging_directory.display()
        )
    })?;
    let staging_guard = TemporaryAlbumDirectory(staging_directory.clone());
    let mut tracks = Vec::with_capacity(job.input_paths.len());

    for ((input_path, destination), assigned_target_lufs) in job
        .input_paths
        .iter()
        .zip(&destinations)
        .zip(assigned_targets)
    {
        let staged_output = staging_directory.join(
            destination
                .file_name()
                .context("Album output does not have a filename")?,
        );

        let track_job = MasteringJob {
            input_path: input_path.clone(),
            output_path: Some(staged_output),
            reference_path: job.reference_path.clone(),
            backend: job.backend,
            ai_provider: job.ai_provider,
            lmstudio_model: None,
            bit_depth: job.bit_depth,
            format: Some(format),
            target_lufs: Some(assigned_target_lufs),
            no_limiter: job.no_limiter,
            preset: job.preset,
            dry_run: false,
        };

        // The analysis cache makes the pipeline's pre-analysis lookup cheap and
        // ensures all normal single-track safety checks remain authoritative.
        let result = pipeline::run(&track_job, config).await?;
        tracks.push(AlbumTrackResult {
            input_path: input_path.clone(),
            assigned_target_lufs,
            result,
            sha256: String::new(),
        });
    }

    for track in &mut tracks {
        track.sha256 = sha256_file(&track.result.output_path)?;
    }
    let delivered: Vec<f64> = tracks
        .iter()
        .filter_map(|track| {
            track
                .result
                .post_analysis
                .as_ref()
                .map(|value| value.lufs_integrated)
        })
        .collect();
    let delivered_loudness_spread_lu = loudness_spread(&delivered);

    let report = serde_json::json!({
        "schema_version": 1,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "album_target_lufs": album_target_lufs,
        "source_median_lufs": source_median_lufs,
        "delivered_loudness_spread_lu": delivered_loudness_spread_lu,
        "tracks": tracks.iter().enumerate().map(|(index, track)| serde_json::json!({
            "order": index + 1,
            "input_path": track.input_path,
            "output_path": destinations[index],
            "assigned_target_lufs": track.assigned_target_lufs,
            "delivered_lufs": track.result.post_analysis.as_ref().map(|analysis| analysis.lufs_integrated),
            "true_peak_dbtp": track.result.post_analysis.as_ref().map(|analysis| analysis.true_peak_db),
            "sha256": track.sha256,
            "warnings": track.result.warnings,
        })).collect::<Vec<_>>()
    });
    let staged_report = staging_directory.join("audiomaster-album-report.json");
    std::fs::write(&staged_report, serde_json::to_vec_pretty(&report)?)?;

    // Publish only after every track and its report have rendered and verified.
    let mut published: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (track, destination) in tracks.iter_mut().zip(&destinations) {
        let staged = track.result.output_path.clone();
        if let Err(error) = std::fs::rename(&staged, destination) {
            rollback_album_publication(&published);
            return Err(error)
                .with_context(|| format!("Publishing album track {}", destination.display()));
        }
        published.push((destination.clone(), staged));
        track.result.output_path = destination.clone();
        if let Some(post) = &mut track.result.post_analysis {
            post.metadata.path = destination.clone();
        }
    }
    if let Err(error) = std::fs::rename(&staged_report, &report_path) {
        rollback_album_publication(&published);
        return Err(error).context("Publishing album verification report");
    }
    drop(staging_guard);

    Ok(AlbumResult {
        album_target_lufs,
        source_median_lufs,
        tracks,
        delivered_loudness_spread_lu,
        report_path,
    })
}

fn rollback_album_publication(published: &[(PathBuf, PathBuf)]) {
    for (published_path, original_staged) in published.iter().rev() {
        let _ = std::fs::rename(published_path, original_staged);
    }
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn median_loudness(analyses: &[AudioAnalysis]) -> f64 {
    let mut values: Vec<f64> = analyses.iter().map(|value| value.lufs_integrated).collect();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn derive_track_targets(
    analyses: &[AudioAnalysis],
    source_median_lufs: f64,
    album_target_lufs: f64,
    max_relative_offset_lu: f64,
) -> Vec<f64> {
    analyses
        .iter()
        .map(|analysis| {
            album_target_lufs
                + (analysis.lufs_integrated - source_median_lufs)
                    .clamp(-max_relative_offset_lu, max_relative_offset_lu)
        })
        .collect()
}

fn loudness_spread(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    maximum - minimum
}

pub fn output_path_for(input: &Path, output_directory: &Path, format: AudioFormat) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("track");
    output_directory.join(format!("{stem}_mastered.{format}"))
}

fn album_staging_directory(output_directory: &Path) -> PathBuf {
    static NEXT_ALBUM_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT_ALBUM_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    output_directory.join(format!(".audiomaster-album-{}-{id}", std::process::id()))
}

struct TemporaryAlbumDirectory(PathBuf);

impl Drop for TemporaryAlbumDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudioMetadata, FrequencyBands};

    fn analysis(lufs: f64) -> AudioAnalysis {
        AudioAnalysis {
            schema_version: 2,
            metadata: AudioMetadata {
                path: PathBuf::new(),
                sample_rate: 48_000,
                channels: 2,
                duration_secs: 1.0,
                bit_depth: Some(24),
                format: "WAV".into(),
            },
            lufs_integrated: lufs,
            lufs_short_term_max: lufs,
            lufs_momentary_max: lufs,
            loudness_range_lu: 0.0,
            rms_db: lufs,
            peak_db: -1.0,
            true_peak_db: -1.0,
            dynamic_range_db: 0.0,
            crest_factor_db: 0.0,
            peak_to_loudness_ratio: 0.0,
            stereo_width: 0.0,
            stereo_correlation: 1.0,
            dc_offset: 0.0,
            clipped_samples: 0,
            frequency_bands: FrequencyBands {
                sub_bass: 0.0,
                bass: 0.0,
                low_mid: 0.0,
                mid: 0.0,
                upper_mid: 0.0,
                presence: 0.0,
                brilliance: 0.0,
            },
        }
    }

    #[test]
    fn album_targets_preserve_only_bounded_relative_dynamics() {
        let analyses = vec![analysis(-24.0), analysis(-18.0), analysis(-16.0)];
        let targets = derive_track_targets(&analyses, -18.0, -14.0, 1.5);
        assert_eq!(targets, vec![-15.5, -14.0, -12.5]);
    }

    #[test]
    fn output_paths_are_deterministic() {
        assert_eq!(
            output_path_for(
                Path::new("/music/01 Intro.wav"),
                Path::new("/masters"),
                AudioFormat::Flac
            ),
            PathBuf::from("/masters/01 Intro_mastered.flac")
        );
    }
}
