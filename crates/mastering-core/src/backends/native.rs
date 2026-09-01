use anyhow::Result;

use super::{BackendOutput, MasteringOptions};
use crate::dsp;
use crate::types::{
    CompressionParams, EqBand, EqBandType, LimiterParams, MasteringParams, StereoParams,
};

#[derive(Debug, Clone, Default)]
pub struct NativeBackend;

impl NativeBackend {
    pub fn new() -> Self {
        Self
    }

    pub async fn process(&self, options: &MasteringOptions) -> Result<BackendOutput> {
        let analysis = match &options.pre_analysis {
            Some(analysis) => analysis.clone(),
            None => crate::analysis::analyze_file(&options.input_path).await?,
        };
        let proposed =
            create_mastering_plan(&analysis, options.reference_analysis.as_ref(), options);
        let validated = dsp::validate_params(
            proposed,
            analysis.metadata.sample_rate,
            options.target_lufs,
            options.no_limiter,
        )?;
        let params = validated.params;
        let render_params = params.clone();
        let input = options.input_path.clone();
        let output = options.output_path.clone();
        let bit_depth = options.bit_depth;
        let control = options.control.clone();
        let mut warnings = validated.warnings;
        warnings.extend(
            tokio::task::spawn_blocking(move || {
                dsp::render_wav_with_control(&input, &output, &render_params, bit_depth, &control)
            })
            .await??,
        );

        Ok(BackendOutput {
            output_path: options.output_path.clone(),
            params_applied: Some(params),
            backend_name: if options.reference_analysis.is_some() {
                "native/reference".into()
            } else {
                "native".into()
            },
            message: if warnings.is_empty() {
                "Mastered with the deterministic native engine".into()
            } else {
                format!(
                    "Mastered with the native engine ({} warnings)",
                    warnings.len()
                )
            },
            warnings,
        })
    }

    pub async fn check_available(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Conservative, content-adaptive starting plan. The bounds intentionally
/// favor transparency; expert controls and optional advisors may refine it.
fn create_mastering_plan(
    analysis: &crate::types::AudioAnalysis,
    reference: Option<&crate::types::AudioAnalysis>,
    options: &MasteringOptions,
) -> MasteringParams {
    let bands = &analysis.frequency_bands;
    let mut low_delta = ((bands.bass - bands.mid) * -0.12).clamp(-1.5, 1.5);
    let mut air_delta = ((bands.brilliance - bands.mid) * -0.08).clamp(-1.25, 1.25);
    let mud_delta = if bands.low_mid > bands.mid + 3.0 {
        -1.0
    } else {
        0.0
    };
    let mut low_mid_delta = mud_delta;
    let mut mid_delta = 0.0;
    let mut upper_mid_delta = 0.0;
    let mut presence_delta = 0.0;

    let crest = analysis.crest_factor_db;
    let mut ratio = if crest > 16.0 {
        2.2
    } else if crest > 11.0 {
        1.7
    } else {
        1.25
    };
    let threshold = (analysis.rms_db + 4.0).clamp(-30.0, -8.0);
    let attack_ms = if crest > 14.0 { 25.0 } else { 12.0 };
    let release_ms = if analysis.loudness_range_lu > 12.0 {
        180.0
    } else {
        110.0
    };
    let mut width = if analysis.stereo_correlation < 0.0 {
        0.85
    } else if analysis.stereo_width > 1.35 {
        0.9
    } else {
        1.0
    };

    if let Some(reference) = reference {
        let reference_bands = &reference.frequency_bands;
        let source_low_shape = bands.bass - bands.mid;
        let reference_low_shape = reference_bands.bass - reference_bands.mid;
        low_delta = (low_delta + (reference_low_shape - source_low_shape) * 0.15).clamp(-2.0, 2.0);

        let source_air_shape = bands.brilliance - bands.mid;
        let reference_air_shape = reference_bands.brilliance - reference_bands.mid;
        air_delta =
            (air_delta + (reference_air_shape - source_air_shape) * 0.12).clamp(-1.75, 1.75);
        low_mid_delta =
            (low_mid_delta + (reference_bands.low_mid - bands.low_mid) * 0.10).clamp(-2.0, 2.0);
        mid_delta = ((reference_bands.mid - bands.mid) * 0.08).clamp(-1.5, 1.5);
        upper_mid_delta = ((reference_bands.upper_mid - bands.upper_mid) * 0.08).clamp(-1.5, 1.5);
        presence_delta = ((reference_bands.presence - bands.presence) * 0.07).clamp(-1.25, 1.25);

        let lra_difference = analysis.loudness_range_lu - reference.loudness_range_lu;
        ratio = (ratio + lra_difference.max(0.0) * 0.04).clamp(1.1, 2.8);
        let plr_difference = analysis.peak_to_loudness_ratio - reference.peak_to_loudness_ratio;
        if plr_difference > 2.0 {
            ratio = (ratio + plr_difference * 0.03).min(2.8);
        }
        if analysis.stereo_correlation >= 0.0 && reference.stereo_correlation >= 0.0 {
            let reference_width = reference.stereo_width.clamp(0.75, 1.25);
            width = (width * 0.75 + reference_width * 0.25).clamp(0.75, 1.25);
        }
    }

    MasteringParams {
        eq: vec![
            EqBand {
                frequency: 25.0,
                gain_db: 0.0,
                q: 0.707,
                band_type: EqBandType::HighPass,
            },
            EqBand {
                frequency: 100.0,
                gain_db: low_delta,
                q: 0.707,
                band_type: EqBandType::LowShelf,
            },
            EqBand {
                frequency: 350.0,
                gain_db: low_mid_delta,
                q: 0.9,
                band_type: EqBandType::Peak,
            },
            EqBand {
                frequency: 1_200.0,
                gain_db: mid_delta,
                q: 0.8,
                band_type: EqBandType::Peak,
            },
            EqBand {
                frequency: 3_000.0,
                gain_db: upper_mid_delta,
                q: 0.9,
                band_type: EqBandType::Peak,
            },
            EqBand {
                frequency: 5_200.0,
                gain_db: presence_delta,
                q: 1.0,
                band_type: EqBandType::Peak,
            },
            EqBand {
                frequency: 10_000.0,
                gain_db: air_delta,
                q: 0.707,
                band_type: EqBandType::HighShelf,
            },
        ],
        compression: CompressionParams {
            threshold_db: threshold,
            ratio,
            attack_ms,
            release_ms,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        },
        limiter: LimiterParams {
            enabled: !options.no_limiter,
            ceiling_db: if matches!(
                options.delivery_format,
                crate::types::AudioFormat::Mp3 | crate::types::AudioFormat::Aac
            ) {
                -1.2
            } else {
                -1.0
            },
            release_ms: 80.0,
        },
        stereo: StereoParams {
            width,
            balance: 0.0,
        },
        target_lufs: options.target_lufs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudioAnalysis, AudioMetadata, FrequencyBands};
    use std::path::PathBuf;

    fn analysis() -> AudioAnalysis {
        AudioAnalysis {
            schema_version: 2,
            metadata: AudioMetadata {
                path: PathBuf::from("test.wav"),
                sample_rate: 48_000,
                channels: 2,
                duration_secs: 60.0,
                bit_depth: Some(24),
                format: "WAV".into(),
            },
            lufs_integrated: -18.0,
            lufs_short_term_max: -14.0,
            lufs_momentary_max: -12.0,
            loudness_range_lu: 10.0,
            rms_db: -20.0,
            peak_db: -3.0,
            true_peak_db: -2.8,
            dynamic_range_db: 12.0,
            crest_factor_db: 17.0,
            peak_to_loudness_ratio: 15.2,
            stereo_width: 0.8,
            stereo_correlation: 0.5,
            dc_offset: 0.0,
            clipped_samples: 0,
            frequency_bands: FrequencyBands {
                sub_bass: -30.0,
                bass: -18.0,
                low_mid: -15.0,
                mid: -20.0,
                upper_mid: -23.0,
                presence: -25.0,
                brilliance: -28.0,
            },
        }
    }

    #[test]
    fn native_plan_is_conservative_and_honors_limiter_policy() {
        let options = MasteringOptions {
            input_path: PathBuf::new(),
            output_path: PathBuf::new(),
            reference_path: None,
            bit_depth: 24,
            delivery_format: crate::types::AudioFormat::Wav,
            target_lufs: -14.0,
            no_limiter: true,
            preset: None,
            pre_analysis: None,
            reference_analysis: None,
            control: crate::control::ProcessingControl::default(),
        };
        let plan = create_mastering_plan(&analysis(), None, &options);
        assert!(!plan.limiter.enabled);
        assert!(plan.eq.iter().all(|band| band.gain_db.abs() <= 1.5));
        assert!((1.0..=2.5).contains(&plan.compression.ratio));
    }

    #[test]
    fn native_reference_plan_moves_toward_reference_without_copying_it() {
        let source = analysis();
        let mut reference = analysis();
        reference.frequency_bands.bass += 8.0;
        reference.stereo_width = 1.2;
        let options = MasteringOptions {
            input_path: PathBuf::new(),
            output_path: PathBuf::new(),
            reference_path: Some(PathBuf::from("reference.wav")),
            bit_depth: 24,
            delivery_format: crate::types::AudioFormat::Wav,
            target_lufs: -14.0,
            no_limiter: false,
            preset: None,
            pre_analysis: None,
            reference_analysis: Some(reference.clone()),
            control: crate::control::ProcessingControl::default(),
        };
        let without_reference = create_mastering_plan(&source, None, &options);
        let with_reference = create_mastering_plan(&source, Some(&reference), &options);
        assert!(with_reference.eq[1].gain_db > without_reference.eq[1].gain_db);
        assert!(with_reference.stereo.width > without_reference.stereo.width);
        assert!(with_reference
            .eq
            .iter()
            .all(|band| band.gain_db.abs() <= 2.0));
    }
}
